#[path = "src/shared.rs"]
mod shared;

use std::collections::HashMap;
use std::path::Path;

use crate::shared::{BlockData, CodepointData};

const UNICODE_VERSION: &str = "17.0.0";
const UTR25_REVISION: &str = "15";

fn parse_codepoint(s: &str) -> u32 {
    u32::from_str_radix(s.trim(), 0x10).unwrap()
}

fn parse_codepoint_range(s: &str) -> (u32, u32) {
    match s.split_once("..") {
        None => {
            let cp = parse_codepoint(s);
            (cp, cp)
        }
        Some((first, last)) => (parse_codepoint(first), parse_codepoint(last)),
    }
}

/// Returns the string content of a remote file.
fn read_remote_file(url: &str) -> String {
    ureq::get(url)
        .call()
        .unwrap()
        .body_mut()
        .read_to_string()
        .unwrap()
}

/// Returns non-empty lines of a Unicode data file.
///
/// Each line consists of multiple entries, separated by semicolons. Comments
/// and whitespace are properly stripped.
fn get_unicode_data_file(url: &str) -> Vec<Vec<String>> {
    read_remote_file(url)
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
    buf.push_str("pub fn block_data(cp: u32) -> Option<BlockData<'static>> {\n");
    buf.push_str("    match cp {\n");
    for line in get_unicode_data_file(&ucd("Blocks.txt")) {
        let [range, name] = line.as_array().unwrap();
        let (first, last) = parse_codepoint_range(range);
        let data = BlockData { first, last, name };
        buf.push_str(&format!(
            "        0x{first:04X}..=0x{last:04X} => Some({data:?}),\n"
        ));
    }
    buf.push_str("        _ => None,\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");
}

fn build_codepoint_data(buf: &mut String) {
    // We have to split this into multiple functions because a single function
    // would have too many local constants.
    // plugin panicked: tried to allocate too many function local constant values
    let mut batches = Vec::new();
    for batch in get_unicode_data_file(&ucd("UnicodeData.txt")).chunks(10000) {
        let batch_name = format!("codepoint_data_{}", batches.len());
        buf.push_str("#[inline(never)]\n");
        buf.push_str(&format!(
            "fn {batch_name}(cp: u32) -> Option<CodepointData<'static>> {{\n"
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
            let cp = parse_codepoint(cp);
            let data = CodepointData {
                name,
                general_category,
                canonical_combining_class,
            };
            buf.push_str(&format!("        0x{cp:04X} => Some({data:?}),\n",));
        }
        buf.push_str("        _ => None,\n");
        buf.push_str("    }\n");
        buf.push_str("}\n");
    }

    buf.push_str("pub fn codepoint_data(cp: u32) -> Option<CodepointData<'static>> {\n");
    buf.push_str("    None\n");
    for batch_name in batches {
        buf.push_str(&format!("        .or_else(|| {batch_name}(cp))\n",));
    }
    buf.push_str("}\n");
}

/// Converts a Unicode class name from the data files to the name used in Typst.
///
/// Note that Typst does not assign a name to all Unicode math classes. For the
/// ones that don't have a name in Typst, we use the most typst-y name.
///
/// - [Unicode names](https://www.unicode.org/reports/tr25/tr25-16.html#mathematical_classification_0)
/// - [Typst names](https://typst.app/docs/reference/math/class/#parameters-class)
fn typst_math_class(c: &str) -> &'static str {
    match c {
        "N" => "normal",
        "A" => "alphabetic",
        "B" => "binary",
        "C" => "closing",
        "D" => "diacritic",
        "F" => "fence",
        "G" => "glyph-part",
        "O" => "opening",
        "L" => "large",
        "P" => "punctuation",
        "R" => "relation",
        "S" => "space",
        "U" => "unary",
        "V" => "vary",
        "X" => "special",
        _ => unreachable!("illegal math class"),
    }
}

fn build_math_data(buf: &mut String) {
    let math_class_file_url = format!(
        "https://www.unicode.org/Public/math/revision-{UTR25_REVISION}/MathClass-{UTR25_REVISION}.txt",
    );
    buf.push_str("pub fn math_data(cp: u32) -> Option<&'static str> {\n");
    buf.push_str("    match cp {\n");
    for line in get_unicode_data_file(&math_class_file_url) {
        let [range, math_class] = line.as_array().unwrap();
        let (first, last) = parse_codepoint_range(range);
        let typst_math_class = typst_math_class(math_class);
        buf.push_str(&format!(
            "        0x{first:04X}..=0x{last:04X} => Some({typst_math_class:?}),\n",
        ));
    }
    buf.push_str("        _ => None,\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");
}

/// Provides access to formal aliases.
fn build_alias_data(buf: &mut String) {
    let mut corrections = HashMap::<_, Vec<_>>::new();
    let mut controls = HashMap::<_, Vec<_>>::new();
    let mut alternates = HashMap::<_, Vec<_>>::new();
    let mut figments = HashMap::<_, Vec<_>>::new();
    let mut abbreviations = HashMap::<_, Vec<_>>::new();
    for line in get_unicode_data_file(&ucd("NameAliases.txt")) {
        let [cp, alias, alias_type] = line.as_array().unwrap();
        let cp = parse_codepoint(cp);
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
    keys.sort();
    keys.dedup();

    let return_type = "&'static [&'static str], ".repeat(5);
    buf.push_str(&format!(
        "pub fn alias_data(cp: u32) -> ({return_type}) {{\n"
    ));
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
        buf.push_str(&format!("        0x{cp:04X} => {properties},\n"));
    }
    buf.push_str("        _ => (&[], &[], &[], &[], &[]),\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");
}

/// Provides information from
/// [NamesList.txt](https://www.unicode.org/Public/UCD/latest/ucd/NamesList.html).
///
/// For now, this only reads informative aliases.
fn build_info_data(buf: &mut String) {
    let mut info_aliases = HashMap::<_, Vec<_>>::new();
    let mut last_char = None;
    // Grammar: https://www.unicode.org/Public/UCD/latest/ucd/NamesList.html.
    for line in read_remote_file(&ucd("NamesList.txt"))
        .lines()
        .map(|line| line.split(';').next().unwrap())
        .filter(|line| !line.is_empty())
    {
        if !line.starts_with('\t') {
            if let Some((l, _)) = line.split_once('\t')
                && (4..=6).contains(&l.len())
                && let Ok(cp) = u32::from_str_radix(l, 0x10)
            {
                // NAME_LINE | RESERVED_LINE
                last_char = Some(cp)
            } else {
                last_char = None
            }
        } else if let Some(cp) = last_char
            && let Some(info_alias) = line.strip_prefix("\t= ")
        {
            // ALIAS_LINE
            info_aliases
                .entry(cp)
                .or_default()
                .push(info_alias.to_owned())
        }
    }

    buf.push_str("pub fn info_data(cp: u32) -> &'static [&'static str] {\n");
    buf.push_str("    match cp {\n");
    let mut keys = info_aliases.keys().collect::<Vec<_>>();
    keys.sort();
    for cp in keys {
        let aliases = info_aliases.get(cp).unwrap();
        buf.push_str(&format!("        0x{cp:04X} => &{aliases:?},\n"));
    }
    buf.push_str("        _ => &[],\n");
    buf.push_str("    }\n");
    buf.push_str("}\n");
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let mut buf = String::new();
    build_block_data(&mut buf);
    build_codepoint_data(&mut buf);
    build_math_data(&mut buf);
    build_alias_data(&mut buf);
    build_info_data(&mut buf);

    let out = std::env::var_os("OUT_DIR").unwrap();
    let dest = Path::new(&out).join("out.rs");
    std::fs::write(&dest, buf).unwrap();
}
