use crate::pathlang::parseurl::*;
use crate::pathlang::path::*;
use crate::{base::common::*, guard_ok};
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag},
    character::complete::{digit1, none_of, one_of},
    combinator::{all_consuming, not, opt, recognize},
    error::{context, VerboseError},
    multi::{many0, many0_count, many1},
    sequence::{delimited, terminated, tuple},
    IResult,
};
use std::str::FromStr;

pub type NomRes<T, U> = IResult<T, U, VerboseError<T>>;

impl<'a> Path<'a> {
    pub fn parse(input: &str) -> Res<Path> {
        let path_res = all_consuming(path_items)(input);
        let path =
            guard_ok!(path_res, err => { return Err(HErr::BadPath(convert_error(input, err)))});
        Ok(path.1)
    }

    pub fn parse_with_starter(input: &str) -> Res<(CellRepresentation, Path)> {
        let path_res = all_consuming(path_with_starter)(input);
        let path =
            guard_ok!(path_res, err => { return Err(HErr::BadPath(convert_error(input, err)))});
        Ok(path.1)
    }
}

fn convert_error(input: &str, err: nom::Err<VerboseError<&str>>) -> String {
    match err {
        nom::Err::Incomplete(needed) => {
            format!("path parsing failed, more input needed {:?}", needed)
        }
        nom::Err::Error(e) => nom::error::convert_error(input, e),
        nom::Err::Failure(e) => nom::error::convert_error(input, e),
    }
}

fn path_with_starter(input: &str) -> NomRes<&str, (CellRepresentation, Path)> {
    context("path", tuple((cell_representation, path_items)))(input).map(|(next_input, res)| {
        let (start, path) = res;
        (next_input, (start, path))
    })
}

fn cell_representation(input: &str) -> NomRes<&str, CellRepresentation> {
    context(
        "cell_representation",
        alt((
            cell_representation_url,
            cell_representation_file,
            cell_representation_string,
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res))
}

fn cell_representation_url(input: &str) -> NomRes<&str, CellRepresentation> {
    context("cell_representation_url", url)(input)
        .map(|(next_input, res)| (next_input, CellRepresentation::Url(res)))
}

fn cell_representation_file(input: &str) -> NomRes<&str, CellRepresentation> {
    context(
        "cell_representation_file",
        tuple((
            alt((tag("/"), tag("."))),
            many0(terminated(path_code_points, tag("/"))),
            opt(path_code_points),
        )),
    )(input)
    .map(|(next_input, res)| {
        let mut len: usize = res.0.len();
        len += res.1.into_iter().map(|s| s.len() + 1).sum::<usize>();
        if let Some(last) = res.2 {
            len += last.len();
        }
        (next_input, CellRepresentation::File(&input[0..len]))
    })
}

fn cell_representation_string(input: &str) -> NomRes<&str, CellRepresentation> {
    context("cell_representation_string", string)(input)
        .map(|(next_input, res)| (next_input, CellRepresentation::String(res)))
}

fn value(input: &str) -> NomRes<&str, Value> {
    context("value", alt((value_string, value_uint)))(input)
        .map(|(next_input, res)| (next_input, res))
}

fn value_string(input: &str) -> NomRes<&str, Value> {
    context("value_string", parse_quoted_single)(input)
        .map(|(next_input, res)| (next_input, Value::Str(res)))
}

fn value_uint(input: &str) -> NomRes<&str, Value> {
    context("value_uint", digit1)(input)
        .and_then(|(next_input, res)| match res.parse::<u64>() {
            Ok(n) => Ok((next_input, n)),
            Err(_) => Err(nom::Err::Error(VerboseError { errors: vec![] })),
        })
        .map(|(next_input, num)| (next_input, Value::Int(Int::U64(num))))
}

fn parse_quoted_single(input: &str) -> NomRes<&str, &str> {
    let esc = escaped(none_of("\\\'"), '\\', tag("'"));
    let esc_or_empty = alt((esc, tag("")));
    delimited(tag("'"), esc_or_empty, tag("'"))(input)
}

fn string(input: &str) -> NomRes<&str, &str> {
    context(
        "string",
        alt((
            delimited(tag("'"), many0_count(not(tag("'"))), tag("'")),
            delimited(tag("\""), many0_count(not(tag("\""))), tag("\"")),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, &input[0..res]))
}

fn path_items(input: &str) -> NomRes<&str, Path> {
    context("path_items", many0(path_item))(input).map(|(next_input, res)| {
        let path_items = res.iter().map(|p| p.to_owned()).collect();
        (next_input, Path(path_items))
    })
}

fn path_item(input: &str) -> NomRes<&str, PathItem> {
    context(
        "path_item",
        tuple((
            path_item_start,
            opt(path_item_selector),
            opt(path_item_index),
            many0(filter),
        )),
    )(input)
    .and_then(|(next_input, res)| {
        if res.1.is_none() && res.2.is_none() {
            Err(nom::Err::Error(VerboseError { errors: vec![] }))
        } else {
            let pi = PathItem {
                relation: Relation::try_from(res.0).unwrap_or_else(|_| panic!("bad relation")),
                selector: res.1,
                index: res.2,
                filters: res.3,
            };
            Ok((next_input, pi))
        }
    })
}

fn path_item_selector(input: &str) -> NomRes<&str, Selector> {
    context("path_item_selector", path_code_points)(input)
        .map(|(next_input, res)| (next_input, Selector::from(res)))
}

fn path_item_index(input: &str) -> NomRes<&str, usize> {
    context(
        "path_item_index",
        delimited(tag("["), number_usize, tag("]")),
    )(input)
    .map(|(next_input, res)| (next_input, res))
}

fn filter(input: &str) -> NomRes<&str, Filter> {
    context("filter", delimited(tag("["), expression, tag("]")))(input)
        .map(|(next_input, res)| (next_input, Filter { expr: res }))
}

fn expression(input: &str) -> NomRes<&str, Expression> {
    context(
        "expression",
        tuple((path_items, opt(accessor), operation, value)),
    )(input)
    .map(|(next_input, res)| {
        (
            next_input,
            Expression {
                left_path: res.0,
                left_accessor: res.1,
                op: res.2,
                right: res.3,
            },
        )
    })
}

fn expression_left(input: &str) -> NomRes<&str, Path> {
    context("expression_left", path_items)(input).map(|(next_input, res)| (next_input, res))
}

fn path_item_start(input: &str) -> NomRes<&str, char> {
    context("path_item_start", alt((tag("/"), tag("@"), tag("^"))))(input)
        .map(|(next_input, res)| (next_input, res.chars().next().unwrap()))
}

fn number_usize(input: &str) -> NomRes<&str, usize> {
    context("number", recognize(many1(one_of("0123456789"))))(input).map(|(next_input, res)| {
        let n = usize::from_str(res).unwrap_or_else(|_| panic!("parse error, logic error"));
        (next_input, n)
    })
}

fn operation(input: &str) -> NomRes<&str, &str> {
    context("operation", alt((tag("=="), tag("!="))))(input)
        .map(|(next_input, res)| (next_input, res))
}

fn accessor(input: &str) -> NomRes<&str, &str> {
    context(
        "accessor",
        tuple((
            tag("."),
            alt((tag("value"), tag("type"), tag("index"), tag("label"))),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res.1))
}
