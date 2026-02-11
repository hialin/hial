use crate::{
    api::*,
    prog::{parse_path::*, program::*, *},
};
use chumsky::prelude::*;

pub fn parse_program(input: &str) -> Res<Program<'_>> {
    let parser = program_parser().then_ignore(end());
    parser
        .parse(input)
        .map_err(|err| usererr(convert_error(input, err)))
}

fn program_parser<'a>() -> impl Parser<char, Program<'a>, Error = ParseError> + Clone {
    statement_parser()
        .separated_by(ws().ignore_then(just(';')).then_ignore(ws()))
        .allow_trailing()
        .map(Program)
        .labelled("program")
}

fn statement_parser<'a>() -> impl Parser<char, Statement<'a>, Error = ParseError> + Clone {
    let assignment = path_with_starter_parser()
        .then_ignore(ws())
        .then_ignore(just('='))
        .then_ignore(ws())
        .then(rvalue_parser())
        .map(|((start, path), value)| Statement::Assignment(start, path, value));

    let path_stmt = path_with_starter_parser().map(|(start, path)| Statement::Path(start, path));
    choice((assignment, path_stmt)).labelled("statement")
}

fn ws() -> impl Parser<char, (), Error = ParseError> + Clone {
    filter(|c: &char| c.is_whitespace()).repeated().ignored()
}
