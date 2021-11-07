#![allow(unused_variables, dead_code)]
#![deny(
	// warnings, // todo uncomment this
    missing_debug_implementations,
    missing_copy_implementations,
    bare_trait_objects,
    // missing_docs
)]

pub use base::common::{HErr, Int, Res, Selector, Value};
pub use base::interpretation_api::*;
pub use base::rust_api;
use std::borrow::Borrow;

pub mod base;
mod interpretations;
mod utils;

pub mod c_api;
pub mod pathlang;
pub mod pprint;

pub mod perftests;
mod tests;

#[macro_export]
macro_rules! guard_ok {
    ($var:expr, $err:ident => $else_block:expr) => {
        match $var {
            Ok(x) => x,
            Err($err) => $else_block,
        }
    };
}

#[macro_export]
macro_rules! guard_some {
    ($var:expr, $else_block:expr) => {
        match $var {
            Some(x) => x,
            None => $else_block,
        }
    };
}

pub static mut VERBOSE: bool = false;

#[macro_export]
macro_rules! verbose {
    ($($arg:tt)*) => (if unsafe{crate::VERBOSE} { eprintln!("[verbose] {}", format!($($arg)*))});
}

pub fn set_verbose(flag: bool) {
    unsafe { VERBOSE = flag }
}

pub fn verbose_error(e: impl Borrow<HErr>) {
    let e = e.borrow();
    if !matches!(e, HErr::NotFound(_)) {
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
