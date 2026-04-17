// In `build.rs`, the fields of the following structs are only read in the
// `Debug` implementation, as intended. This triggers the `dead_code` lint.
#![allow(dead_code)]

#[derive(Debug)]
pub struct BlockData<'a> {
    pub first: u32,
    pub last: u32,
    pub name: &'a str,
}

#[derive(Debug)]
pub struct CodepointData<'a> {
    pub name: &'a str,
    pub general_category: &'a str,
    pub canonical_combining_class: &'a str,
}
