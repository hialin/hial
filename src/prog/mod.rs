pub(crate) mod path;
pub(crate) mod program;
pub(crate) mod searcher;
pub(crate) mod url;

pub(super) mod parse_path;
pub(super) mod parse_program;
pub(super) mod parse_url;

pub use path::{Path, PathStart};
pub use program::{Program, ProgramParams};

use chumsky::error::Rich;

pub(super) type ParseError<'src> = Rich<'src, char>;

fn convert_error(_input: &str, errs: Vec<ParseError<'_>>) -> String {
    if errs.is_empty() {
        return "parse error".to_string();
    }

    let details = errs
        .into_iter()
        .map(|err| err.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    format!("parse error:\n{details}")
}
