#![allow(unused_variables, dead_code)]
#![deny(
	// warnings, // todo uncomment this
    missing_debug_implementations,
    // missing_copy_implementations, // todo uncomment this
    bare_trait_objects,
    // missing_docs
)]

use std::borrow::Borrow;

pub mod base;
// pub mod c_api;
mod interpretations;
pub mod pathlang;
pub mod perftests;
pub mod pprint;
mod utils;

#[cfg(test)]
mod tests;

// pub use base::common::Selector;
// pub use base::error::{HErr, Res};
// pub use base::intra::*;
// pub use base::rust_api;
// pub use base::value::{Int, OwnedValue, Value};

pub static mut VERBOSE: bool = false;

#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => (if unsafe{crate::VERBOSE} { eprintln!("[verbose] {}", format!($($arg)*))});
}

pub fn set_verbose(flag: bool) {
    unsafe { VERBOSE = flag }
}

pub fn verbose_error(e: impl Borrow<crate::base::HErr>) {
    let e = e.borrow();
    if !matches!(e, crate::base::HErr::NotFound(_)) {
        verbose!("Error: {:?}", e)
    }
}

extern "C" {
    fn tree_sitter_rust() -> tree_sitter::Language;
    fn tree_sitter_javascript() -> tree_sitter::Language;
}

pub fn tree_sitter_language(language: &str) -> Option<tree_sitter::Language> {
    match language {
        "rust" => Some(unsafe { tree_sitter_rust() }),
        "javascript" => Some(unsafe { tree_sitter_javascript() }),
        _ => None,
    }
}

pub fn double(x: i32) -> i32 {
    x * 2
}
