pub(crate) mod path;
pub(crate) mod program;
pub(crate) mod searcher;
pub(crate) mod url;

pub(super) mod parse_path;
pub(super) mod parse_program;
pub(super) mod parse_url;

pub use path::{Path, PathStart};
pub use program::{Program, ProgramParams};

use nom::{error::VerboseError, IResult};
pub type NomRes<T, U> = IResult<T, U, VerboseError<T>>;

fn convert_error(input: &str, err: nom::Err<VerboseError<&str>>) -> String {
    match err {
        nom::Err::Incomplete(needed) => {
            format!("path parsing failed, more input needed {:?}", needed)
        }
        nom::Err::Error(e) => nom::error::convert_error(input, e),
        nom::Err::Failure(e) => nom::error::convert_error(input, e),
    }
}
