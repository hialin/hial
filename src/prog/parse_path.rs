use super::{convert_error, NomRes};
use crate::{
    api::*,
    guard_ok,
    prog::{parse_url::*, path::*},
};
use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, take_till},
    character::complete::{anychar, digit1, none_of, one_of, space0},
    combinator::{all_consuming, opt, recognize},
    error::{context, VerboseError, VerboseErrorKind},
    multi::{many0, many1, separated_list0, separated_list1},
    sequence::{delimited, tuple},
};
use std::str::{from_utf8, FromStr};

pub fn parse_path(input: &str) -> Res<Path> {
    let path_res = all_consuming(path_items)(input);
    let path = guard_ok!(path_res, err => { return userres(convert_error(input, err))});
    Ok(path.1)
}

pub fn parse_path_with_starter(input: &str) -> Res<(PathStart, Path)> {
    let path_res = all_consuming(path_with_starter)(input);
    let path = guard_ok!(path_res, err => { return userres(convert_error(input, err))});
    Ok(path.1)
}

pub fn path_with_starter(input: &str) -> NomRes<&str, (PathStart, Path)> {
    context("path", tuple((path_start, space0, path_items)))(input)
        .map(|(next_input, res)| (next_input, (res.0, res.2)))
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
            alt((tag("/"), tag("."), tag("~"))),
            separated_list0(tag("/"), path_code_points),
        )),
    )(input)
    .map(|(next_input, res)| {
        let mut len: usize = res.0.len();
        len += res.1.into_iter().map(|s| s.len() + 1).sum::<usize>();
        (next_input, PathStart::File(input[0..len].to_string()))
    })
}

fn path_start_string(input: &str) -> NomRes<&str, PathStart> {
    context("path_start_string", string)(input)
        .map(|(next_input, res)| (next_input, PathStart::String(res)))
}

fn path_items(input: &str) -> NomRes<&str, Path> {
    context("path_items", many0(path_item))(input).map(|(next_input, res)| {
        let path_items = res.iter().map(|p| p.to_owned()).collect();
        (next_input, Path(path_items))
    })
}

fn path_item(input: &str) -> NomRes<&str, PathItem> {
    context("path_item", alt((elevation_path_item, normal_path_item)))(input)
}

fn elevation_path_item(input: &str) -> NomRes<&str, PathItem> {
    context(
        "elevation path item",
        tuple((
            space0,
            tag("^"),
            space0,
            path_item_selector,
            space0,
            many0(interpretation_param),
        )),
    )(input)
    .map(|(next_input, res)| {
        (
            next_input,
            PathItem::Elevation(ElevationPathItem {
                interpretation: res.3,
                params: res.5,
            }),
        )
    })
}

fn normal_path_item(input: &str) -> NomRes<&str, PathItem> {
    context(
        "normal path item",
        tuple((
            space0,
            relation,
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
            let npi = NormalPathItem {
                relation: Relation::try_from(res.1).unwrap_or_else(|_| panic!("bad relation")),
                selector: res.3,
                index: res.5,
                filters: res.7,
            };
            if npi.relation == Relation::Field && !npi.filters.is_empty() {
                Err(nom::Err::Error(VerboseError {
                    errors: vec![(
                        next_input,
                        VerboseErrorKind::Context("field relation cannot have filters"),
                    )],
                }))
            } else {
                Ok((next_input, PathItem::Normal(npi)))
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
        tuple((
            space0,
            separated_list1(tag("|"), alt((type_expression, ternary_expression))),
        )),
    )(input)
    .map(|(next_input, res)| {
        (
            next_input,
            if res.1.len() == 1 {
                res.1.into_iter().next().unwrap()
            } else {
                Expression::Or { expressions: res.1 }
            },
        )
    })
}

fn ternary_expression(input: &str) -> NomRes<&str, Expression> {
    context(
        "ternary expression",
        tuple((path_items, space0, opt(tuple((operation, space0, rvalue))))),
    )(input)
    .map(|(next_input, (left, _, opts))| {
        (
            next_input,
            Expression::Ternary {
                left,
                op_right: opts.map(|(op, _, right)| (op, right)),
            },
        )
    })
}

fn type_expression(input: &str) -> NomRes<&str, Expression> {
    context("type expression", tuple((tag(":"), identifier)))(input)
        .map(|(next_input, res)| (next_input, Expression::Type { ty: res.1 }))
}

fn interpretation_param(input: &str) -> NomRes<&str, InterpretationParam> {
    context(
        "interpretation parameter",
        delimited(
            tag("["),
            tuple((
                space0,
                alt((string, identifier)),
                space0,
                opt(tuple((tag("="), space0, rvalue))),
            )),
            tag("]"),
        ),
    )(input)
    .map(|(next_input, res)| {
        (
            next_input,
            InterpretationParam {
                name: res.1,
                value: res.3.map(|x| x.2),
            },
        )
    })
}

fn relation(input: &str) -> NomRes<&str, char> {
    context(
        "relation",
        alt((
            tag(from_utf8(&[Relation::Attr as u8]).unwrap()),
            tag(from_utf8(&[Relation::Sub as u8]).unwrap()),
            tag(from_utf8(&[Relation::Interpretation as u8]).unwrap()),
            tag(from_utf8(&[Relation::Field as u8]).unwrap()),
        )),
    )(input)
    .map(|(next_input, res)| (next_input, res.chars().next().unwrap()))
}

pub(super) fn rvalue(input: &str) -> NomRes<&str, OwnValue> {
    context("value", alt((value_string, value_uint, value_ident)))(input)
}

fn value_ident(input: &str) -> NomRes<&str, OwnValue> {
    context("value ident", identifier)(input)
        .map(|(next_input, res)| (next_input, OwnValue::String(res.to_string())))
}

fn value_string(input: &str) -> NomRes<&str, OwnValue> {
    context("value_string", string)(input)
        .map(|(next_input, res)| (next_input, OwnValue::String(res)))
}

pub(super) fn value_uint(input: &str) -> NomRes<&str, OwnValue> {
    context("value_uint", digit1)(input)
        .and_then(|(next_input, res)| match res.parse::<u64>() {
            Ok(n) => Ok((next_input, n)),
            Err(_) => Err(nom::Err::Error(VerboseError { errors: vec![] })),
        })
        .map(|(next_input, num)| (next_input, OwnValue::Int(Int::U64(num))))
}

fn identifier(input: &str) -> NomRes<&str, String> {
    fn accept(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    context("identifier", take_till(|c| !accept(c)))(input)
        .map(|(next_input, res)| (next_input, res.to_string()))
}

fn string(input: &str) -> NomRes<&str, String> {
    context("string", alt((parse_quoted_single, parse_quoted_double)))(input)
}

#[cfg(test)]
#[test]
fn test_parse_string() {
    assert_eq!(string(r#""(\w+)""#).unwrap().1, r#"(\w+)"#);
    assert_eq!(string(r#"'(\w+)'"#).unwrap().1, r#"(\w+)"#);
    assert_eq!(string(r#""(\\w+)""#).unwrap().1, r#"(\w+)"#);
    assert_eq!(
        string(r#""(\w+.\w+)@(\w+)""#).unwrap().1,
        r#"(\w+.\w+)@(\w+)"#
    );
}

fn unescape_string(s: &str, special: char) -> String {
    let mut r = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some(&c) if c == special || c == '\\' => {
                    r.push(c);
                    chars.next();
                }
                _ => r.push('\\'),
            }
        } else {
            r.push(ch);
        }
    }
    r
}

fn parse_quoted_single(input: &str) -> NomRes<&str, String> {
    let esc = escaped(none_of("\'\\"), '\\', anychar);
    let esc_or_empty = alt((esc, tag("")));
    delimited(tag("'"), esc_or_empty, tag("'"))(input)
        .map(|(next_input, res)| (next_input, unescape_string(res, '\'')))
}

fn parse_quoted_double(input: &str) -> NomRes<&str, String> {
    let esc = escaped(none_of("\"\\"), '\\', anychar);
    let esc_or_empty = alt((esc, tag("")));
    delimited(tag("\""), esc_or_empty, tag("\""))(input)
        .map(|(next_input, res)| (next_input, unescape_string(res, '"')))
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
