use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::num::ParseIntError;
use std::path::Path;
use std::str::FromStr;

const UNICODE_VERSION: &str = "17.0.0";
const UTR25_REVISION: &str = "15";

#[derive(Hash, Copy, Clone, Eq, PartialEq)]
struct Codepoint(u32);

impl FromStr for Codepoint {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(u32::from_str_radix(s.trim(), 0x10)?))
    }
}

impl Debug for Codepoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:04X}", self.0)
    }
}

#[derive(Copy, Clone)]
struct CodepointRange {
    first: Codepoint,
    last: Codepoint,
}

impl FromStr for CodepointRange {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.split_once("..") {
            None => {
                let cp = s.parse()?;
                Ok(Self {
                    first: cp,
                    last: cp,
                })
            }
            Some((first, last)) => Ok(Self {
                first: first.parse()?,
                last: last.parse()?,
            }),
        }
    }
}

impl Debug for CodepointRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.first == self.last {
            write!(f, "{:?}", self.first)
        } else {
            write!(f, "{:?}..={:?}", self.first, self.last)
        }
    }
}

/// Returns non-empty lines of a Unicode data file.
///
/// Each line consists of multiple entries, separated by semicolons. Comments
/// and whitespace are properly stripped.
fn get_unicode_data_file(url: &str) -> Vec<Vec<String>> {
    ureq::get(url)
        .call()
        .unwrap()
        .body_mut()
        .read_to_string()
        .unwrap()
        .lines()
        .map(|line| {
            line.split_once('#')
                .map(|(content, _)| content)
                .unwrap_or(line)
        })
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split(';')
                .map(|entry| entry.trim().to_owned())
                .collect()
        })
        .collect()
}

/// Returns the URL of a UCD file.
fn ucd(file: &str) -> String {
    format!("https://www.unicode.org/Public/{UNICODE_VERSION}/ucd/{file}")
}

fn build_block_data(buf: &mut String) {
    buf.push_str("fn block_data(cp: u32) -> Option<(u32, u32, &'static str)> {\n");
    buf.push_str("    match cp {\n");
    for line in get_unicode_data_file(&ucd("Blocks.txt")) {
        let [range, name] = line.as_array().unwrap();
        let range = CodepointRange::from_str(range).unwrap();
        let properties = (range.first, range.last, name);
        buf.push_str(&format!("        {range:?} => Some({properties:?}),\n"));
    }
    buf.push_str("        _ => None,\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");
}

fn build_character_data(buf: &mut String) {
    // We have to split this into multiple functions because a single function
    // would have too many local constants.
    // plugin panicked: tried to allocate too many function local constant values
    let mut batches = Vec::new();
    for batch in get_unicode_data_file(&ucd("UnicodeData.txt")).chunks(10000) {
        let batch_name = format!("character_data_{}", batches.len());
        buf.push_str("#[inline(never)]\n");
        buf.push_str(&format!(
            "fn {batch_name}(cp: u32) -> Option<(&'static str, &'static str, &'static str)> {{\n"
        ));
        batches.push(batch_name);
        buf.push_str("    match cp {\n");
        for line in batch {
            let [
                cp,
                name,
                general_category,
                canonical_combining_class,
                _bidi_class,
                // https://unicode.org/reports/tr44/#Decomposition_Type
                _decomposition,
                // https://unicode.org/reports/tr44/#Numeric_Type
                _,
                _,
                _numeric_value,
                _bidi_mirrored,
                _unicode_1_name,
                _iso_comment,
                _simple_uppercase_mapping,
                _simple_lowercase_mapping,
                // Note that if empty, this should fall back to Simple_Uppercase_Mapping.
                _simple_titlecase_mapping,
            ] = line.as_array().unwrap();
            let cp = Codepoint::from_str(cp).unwrap();
            let properties = (name, general_category, canonical_combining_class);
            buf.push_str(&format!("        {cp:?} => Some({properties:?}),\n",));
        }
        buf.push_str("        _ => None,\n");
        buf.push_str("    }\n");
        buf.push_str("}\n");
    }

    buf.push_str(
        "fn character_data(cp: u32) -> Option<(&'static str, &'static str, &'static str)> {\n",
    );
    buf.push_str("    None\n");
    for batch_name in batches {
        buf.push_str(&format!("        .or_else(|| {batch_name}(cp))\n",));
    }
    buf.push_str("}\n");
}

fn build_math_data(buf: &mut String) {
    let math_class_file_url = format!(
        "https://www.unicode.org/Public/math/revision-{UTR25_REVISION}/MathClass-{UTR25_REVISION}.txt",
    );
    buf.push_str("fn math_data(cp: u32) -> Option<&'static str> {\n");
    buf.push_str("    match cp {\n");
    for line in get_unicode_data_file(&math_class_file_url) {
        let [cp, math_class] = line.as_array().unwrap();
        let range = CodepointRange::from_str(cp).unwrap();
        buf.push_str(&format!("        {range:?} => Some({math_class:?}),\n",));
    }
    buf.push_str("        _ => None,\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");
}

fn build_alias_data(buf: &mut String) {
    let mut corrections = HashMap::<_, Vec<_>>::new();
    let mut controls = HashMap::<_, Vec<_>>::new();
    let mut alternates = HashMap::<_, Vec<_>>::new();
    let mut figments = HashMap::<_, Vec<_>>::new();
    let mut abbreviations = HashMap::<_, Vec<_>>::new();
    for line in get_unicode_data_file(&ucd("NameAliases.txt")) {
        let [cp, alias, alias_type] = line.as_array().unwrap();
        let cp = Codepoint::from_str(cp).unwrap();
        match alias_type.as_ref() {
            "correction" => corrections.entry(cp).or_default().push(alias.clone()),
            "control" => controls.entry(cp).or_default().push(alias.clone()),
            "alternate" => alternates.entry(cp).or_default().push(alias.clone()),
            "figment" => figments.entry(cp).or_default().push(alias.clone()),
            "abbreviation" => abbreviations.entry(cp).or_default().push(alias.clone()),
            _ => panic!("unexpected alias type"),
        }
    }

    let mut keys = corrections
        .keys()
        .chain(controls.keys())
        .chain(alternates.keys())
        .chain(figments.keys())
        .chain(abbreviations.keys())
        .collect::<Vec<_>>();
    keys.sort_by_key(|cp| cp.0);
    keys.dedup();

    let return_type = "&'static [&'static str], ".repeat(5);
    buf.push_str(&format!("fn alias_data(cp: u32) -> ({return_type}) {{\n"));
    buf.push_str("    match cp {\n");
    for cp in keys {
        let empty = Vec::new();
        let properties = format!(
            "(&{:?}, &{:?}, &{:?}, &{:?}, &{:?})",
            corrections.get(cp).unwrap_or(&empty),
            controls.get(cp).unwrap_or(&empty),
            alternates.get(cp).unwrap_or(&empty),
            figments.get(cp).unwrap_or(&empty),
            abbreviations.get(cp).unwrap_or(&empty),
        );
        buf.push_str(&format!("        {cp:?} => {properties},\n"));
    }
    buf.push_str("        _ => (&[], &[], &[], &[], &[]),\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let mut buf = String::new();
    build_block_data(&mut buf);
    build_character_data(&mut buf);
    build_math_data(&mut buf);
    build_alias_data(&mut buf);

    let out = std::env::var_os("OUT_DIR").unwrap();
    let dest = Path::new(&out).join("out.rs");
    std::fs::write(&dest, buf).unwrap();
}
