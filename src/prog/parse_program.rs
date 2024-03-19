use crate::{
    api::*,
    guard_ok,
    prog::{parse_path::*, program::*, *},
};
use nom::{
    character::complete::space0, combinator::all_consuming, error::context, multi::many0,
    sequence::terminated,
};

pub fn parse_program(input: &str) -> Res<Program> {
    let statements_res = all_consuming(program)(input);
    let statements = guard_ok!(statements_res, err => {
        return userres(convert_error(input, err))
    });
    Ok(statements.1)
}

fn program(input: &str) -> NomRes<&str, Program> {
    context("program", many0(statement))(input).map(|(next_input, res)| {
        let statements = res.iter().map(|p| p.to_owned()).collect();
        (next_input, Program(statements))
    })
}

fn statement(input: &str) -> NomRes<&str, Statement> {
    context("statement", terminated(path_with_starter, space0))(input)
        .map(|(next_input, res)| (next_input, Statement::PathWithStart(res.0, res.1)))
}
