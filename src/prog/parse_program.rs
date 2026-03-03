use crate::{
    api::*,
    prog::{parse_path::*, program::*, *},
};
use chumsky::prelude::*;

pub fn parse_program(input: &str) -> Res<Program<'_>> {
    let var_bind = just(':')
        .ignore_then(identifier_parser())
        .then_ignore(ws())
        .then_ignore(just(":="))
        .then_ignore(ws())
        .then(path_with_starter_parser())
        .map(|(name, (start, path))| Statement::VarBind(name, start, path));

    let assignment = path_with_starter_parser()
        .then_ignore(ws())
        .then_ignore(just('='))
        .then_ignore(ws())
        .then(rvalue_parser())
        .map(|((start, path), value)| Statement::Assignment(start, path, value));

    let path_stmt = path_with_starter_parser().map(|(start, path)| Statement::Path(start, path));

    let program = choice((var_bind, assignment, path_stmt))
        .labelled("statement")
        .separated_by(ws().ignore_then(just(';')).then_ignore(ws()))
        .allow_trailing()
        .collect::<Vec<_>>()
        .map(Program)
        .labelled("program")
        .then_ignore(end());

    program
        .parse(input)
        .into_result()
        .map_err(|err| inputerr(convert_error(input, err, "<program>")))
}

fn ws<'src>() -> impl Parser<'src, &'src str, (), extra::Err<ParseError<'src>>> + Clone {
    any()
        .filter(|c: &char| c.is_whitespace())
        .repeated()
        .ignored()
}

fn identifier_parser<'src>()
-> impl Parser<'src, &'src str, String, extra::Err<ParseError<'src>>> + Clone {
    any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .labelled("identifier")
}
