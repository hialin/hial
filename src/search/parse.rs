use crate::{api::*, guard_ok, search::parseurl::*, search::path::*};
use nom::character::complete::space0;
use nom::error::VerboseErrorKind;
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag},
    character::complete::{digit1, none_of, one_of},
    combinator::{all_consuming, opt, recognize},
    error::{context, VerboseError},
    multi::{many0, many1},
    sequence::{delimited, terminated, tuple},
    IResult,
};
use std::str::{from_utf8, FromStr};

pub type NomRes<T, U> = IResult<T, U, VerboseError<T>>;

impl<'a> Path<'a> {
    pub fn parse(input: &str) -> Res<Path> {
        let path_res = all_consuming(path_items)(input);
        let path = guard_ok!(path_res, err => { return userres(convert_error(input, err))});
        Ok(path.1)
    }

    pub fn parse_with_starter(input: &str) -> Res<(PathStart, Path)> {
        let path_res = all_consuming(path_with_starter)(input);
        let path = guard_ok!(path_res, err => { return userres(convert_error(input, err))});
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

fn path_with_starter(input: &str) -> NomRes<&str, (PathStart, Path)> {
    context("path", tuple((path_start, path_items)))(input).map(|(next_input, res)| {
        let (start, path) = res;
        (next_input, (start, path))
    })
}

fn path_start(input: &str) -> NomRes<&str, PathStart> {
    context(
        "path_start",
        alt((path_start_url, path_start_file, path_start_string)),
    )(input)
}

fn path_start_url(input: &str) -> NomRes<&str, PathStart> {
    context("path_start_url", url)(input).map(|(next_input, res)| (next_input, PathStart::Url(res)))
}

fn path_start_file(input: &str) -> NomRes<&str, PathStart> {
    context(
        "path_start_file",
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
        (next_input, PathStart::File(&input[0..len]))
    })
}

fn path_start_string(input: &str) -> NomRes<&str, PathStart> {
    context("path_start_string", string)(input)
        .map(|(next_input, res)| (next_input, PathStart::String(res)))
}

fn value(input: &str) -> NomRes<&str, Value> {
    context("value", alt((value_string, value_uint)))(input)
}

fn value_string(input: &str) -> NomRes<&str, Value> {
    context("value_string", string)(input).map(|(next_input, res)| (next_input, Value::Str(res)))
}

fn value_uint(input: &str) -> NomRes<&str, Value> {
    context("value_uint", digit1)(input)
        .and_then(|(next_input, res)| match res.parse::<u64>() {
            Ok(n) => Ok((next_input, n)),
            Err(_) => Err(nom::Err::Error(VerboseError { errors: vec![] })),
        })
        .map(|(next_input, num)| (next_input, Value::Int(Int::U64(num))))
}

fn string(input: &str) -> NomRes<&str, &str> {
    context("string", alt((parse_quoted_single, parse_quoted_double)))(input)
}

fn parse_quoted_single(input: &str) -> NomRes<&str, &str> {
    let esc = escaped(none_of("\\\'"), '\\', tag("'"));
    let esc_or_empty = alt((esc, tag("")));
    delimited(tag("'"), esc_or_empty, tag("'"))(input)
}

fn parse_quoted_double(input: &str) -> NomRes<&str, &str> {
    let esc = escaped(none_of("\\\""), '\\', tag("\""));
    let esc_or_empty = alt((esc, tag("")));
    delimited(tag("\""), esc_or_empty, tag("\""))(input)
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
            space0,
            path_item_start,
            space0,
            opt(path_item_selector),
            space0,
            opt(path_item_index),
            space0,
            many0(filter),
            space0,
        )),
    )(input)
    .and_then(|(next_input, res)| {
        if res.3.is_none() && res.5.is_none() {
            Err(nom::Err::Error(VerboseError { errors: vec![] }))
        } else {
            let pi = PathItem {
                relation: Relation::try_from(res.1).unwrap_or_else(|_| panic!("bad relation")),
                selector: res.3,
                index: res.5,
                filters: res.7,
            };
            if pi.relation == Relation::Field && !pi.filters.is_empty() {
                Err(nom::Err::Error(VerboseError {
                    errors: vec![(
                        next_input,
                        VerboseErrorKind::Context("field relation cannot have filters"),
                    )],
                }))
            } else {
                Ok((next_input, pi))
            }
        }
    })
}

fn path_item_selector(input: &str) -> NomRes<&str, Selector> {
    context("path_item_selector", path_code_points)(input)
        .map(|(next_input, res)| (next_input, Selector::from(res)))
}

fn path_item_index(input: &str) -> NomRes<&str, isize> {
    context(
        "path_item_index",
        delimited(tag("["), number_isize, tag("]")),
    )(input)
}

fn filter(input: &str) -> NomRes<&str, Filter> {
    context("filter", delimited(tag("["), expression, tag("]")))(input)
        .map(|(next_input, res)| (next_input, Filter { expr: res }))
}

fn expression(input: &str) -> NomRes<&str, Expression> {
    context(
        "expression",
        tuple((path_items, opt(tuple((operation, value))))),
    )(input)
    .map(|(next_input, res)| {
        (
            next_input,
            Expression {
                left: res.0,
                op: res.1.map(|x| x.0),
                right: res.1.map(|x| x.1),
            },
        )
    })
}

fn expression_left(input: &str) -> NomRes<&str, Path> {
    context("expression_left", path_items)(input)
}

fn path_item_start(input: &str) -> NomRes<&str, char> {
    context(
        "path_item_start",
        alt((
            tag(from_utf8(&[Relation::Attr as u8]).unwrap()),
            tag(from_utf8(&[Relation::Sub as u8]).unwrap()),
            tag(from_utf8(&[Relation::Interpretation as u8]).unwrap()),
            tag(from_utf8(&[Relation::Field as u8]).unwrap()),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res.chars().next().unwrap()))
}

fn number_usize(input: &str) -> NomRes<&str, usize> {
    context("positive number", recognize(many1(one_of("0123456789"))))(input).map(
        |(next_input, res)| {
            let n = usize::from_str(res).unwrap_or_else(|_| panic!("parse error, logic error"));
            (next_input, n)
        },
    )
}

fn number_isize(input: &str) -> NomRes<&str, isize> {
    context(
        "number",
        recognize(tuple((opt(one_of("+-")), many1(one_of("0123456789"))))),
    )(input)
    .map(|(next_input, res)| {
        let n = isize::from_str(res).unwrap_or_else(|_| panic!("parse error, logic error"));
        (next_input, n)
    })
}

fn operation(input: &str) -> NomRes<&str, &str> {
    context("operation", alt((tag("=="), tag("!="))))(input)
}