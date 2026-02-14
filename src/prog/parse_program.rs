use crate::{
    api::*,
    prog::{parse_path::*, program::*, *},
};
use chumsky::prelude::*;

pub fn parse_program(input: &str) -> Res<Program<'_>> {
    let assignment = path_with_starter_parser()
        .then_ignore(ws())
        .then_ignore(just('='))
        .then_ignore(ws())
        .then(rvalue_parser())
        .map(|((start, path), value)| Statement::Assignment(start, path, value));

    let path_stmt = path_with_starter_parser().map(|(start, path)| Statement::Path(start, path));

    let program = choice((assignment, path_stmt))
        .labelled("statement")
        .separated_by(ws().ignore_then(just(';')).then_ignore(ws()))
        .allow_trailing()
        .map(Program)
        .labelled("program")
        .then_ignore(end());

    program
        .parse(input)
        .map_err(|err| usererr(convert_error(input, err)))
}

fn ws() -> impl Parser<char, (), Error = ParseError> + Clone {
    filter(|c: &char| c.is_whitespace()).repeated().ignored()
}
