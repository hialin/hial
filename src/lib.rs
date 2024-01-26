// #![feature(test)]
#![allow(unused_variables, dead_code)]
#![deny(
// warnings, // todo uncomment this
missing_debug_implementations,
// missing_copy_implementations, // todo uncomment this
bare_trait_objects,
// missing_docs
)]

pub mod base;
// pub mod c_api;
mod interpretations;
pub mod pathlang;
pub mod perftests;
pub mod pprint;
pub mod utils;

#[cfg(test)]
mod tests;

extern "C" {
    // fn tree_sitter_go() -> tree_sitter::Language;
    fn tree_sitter_javascript() -> tree_sitter::Language;
    // fn tree_sitter_python() -> tree_sitter::Language;
    fn tree_sitter_rust() -> tree_sitter::Language;
}
