use crate::{
    api::*,
    guard_ok,
    prog::{parse_path::*, program::*, *},
};
use nom::{
    branch::alt, bytes::complete::tag, character::complete::space0, combinator::all_consuming,
    error::context, multi::separated_list0, sequence::tuple,
};

pub fn parse_program(input: &str) -> Res<Program> {
    let statements_res = all_consuming(program)(input);
    let statements = guard_ok!(statements_res, err => {
        return userres(convert_error(input, err))
    });
    Ok(statements.1)
}

fn program(input: &str) -> NomRes<&str, Program> {
    context(
        "program",
        separated_list0(tuple((space0, tag(";"), space0)), statement),
    )(input)
    .map(|(next_input, res)| {
        let statements = res.iter().map(|p| p.to_owned()).collect();
        (next_input, Program(statements))
    })
}

fn statement(input: &str) -> NomRes<&str, Statement> {
    context("statement", alt((statement_assignment, statement_path)))(input)
}

fn statement_path(input: &str) -> NomRes<&str, Statement> {
    context("path", path_with_starter)(input)
        .map(|(next_input, res)| (next_input, Statement::Path(res.0, res.1)))
}

fn statement_assignment(input: &str) -> NomRes<&str, Statement> {
    context(
        "assignment",
        tuple((path_with_starter, space0, tag("="), space0, rvalue)),
    )(input)
    .map(|(next_input, res)| (next_input, Statement::Assignment(res.0 .0, res.0 .1, res.4)))
}
