use super::{ParseError, convert_error};
use crate::{
    api::*,
    prog::{parse_url::*, path::*},
};
use chumsky::prelude::*;
use std::str::FromStr;

// TODO: this is not ok, remove this function and fix the problems
fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

pub fn parse_path(input: &str) -> Res<Path<'_>> {
    path_items_parser()
        .then_ignore(end())
        .parse(input)
        .map_err(|err| usererr(convert_error(input, err)))
}

pub fn parse_path_with_starter(input: &str) -> Res<(PathStart<'_>, Path<'_>)> {
    path_with_starter_parser()
        .then_ignore(end())
        .parse(input)
        .map_err(|err| usererr(convert_error(input, err)))
}

fn ws() -> impl Parser<char, (), Error = ParseError> + Clone {
    filter(|c: &char| c.is_whitespace()).repeated().ignored()
}

fn identifier_parser() -> impl Parser<char, String, Error = ParseError> + Clone {
    filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
        .repeated()
        .at_least(1)
        .collect::<String>()
        .labelled("identifier")
}

fn number_isize_parser() -> impl Parser<char, isize, Error = ParseError> + Clone {
    one_of("+-")
        .or_not()
        .then(
            filter(|c: &char| c.is_ascii_digit())
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .try_map(|(sign, digits), span| {
            let mut raw = String::new();
            if let Some(sign) = sign {
                raw.push(sign);
            }
            raw.push_str(&digits);
            isize::from_str(raw.as_str()).map_err(|_| Simple::custom(span, "invalid number"))
        })
        .labelled("number")
}

fn quoted_parser(quote: char) -> impl Parser<char, String, Error = ParseError> + Clone {
    let inner = choice((
        just('\\').ignore_then(any()).map(|c| {
            let mut out = String::from("\\");
            out.push(c);
            out
        }),
        filter(move |c: &char| *c != quote && *c != '\\').map(|c| c.to_string()),
    ))
    .repeated()
    .collect::<Vec<String>>()
    .map(|parts| parts.concat());

    just(quote)
        .ignore_then(inner)
        .then_ignore(just(quote))
        .map(move |raw| unescape_string(raw.as_str(), quote))
}

fn string_parser() -> impl Parser<char, String, Error = ParseError> + Clone {
    choice((quoted_parser('\''), quoted_parser('"'))).labelled("string")
}

fn operation_parser<'a>() -> impl Parser<char, &'a str, Error = ParseError> + Clone {
    choice((just("==").to("=="), just("!=").to("!="))).labelled("operation")
}

pub(super) fn value_int_parser() -> impl Parser<char, OwnValue, Error = ParseError> + Clone {
    number_isize_parser()
        .map(|num| OwnValue::from(num as i64))
        .labelled("value_int")
}

fn value_string_parser() -> impl Parser<char, OwnValue, Error = ParseError> + Clone {
    string_parser()
        .map(OwnValue::String)
        .labelled("value_string")
}

fn value_ident_parser() -> impl Parser<char, OwnValue, Error = ParseError> + Clone {
    identifier_parser()
        .map(OwnValue::String)
        .labelled("value_ident")
}

pub(super) fn rvalue_parser() -> impl Parser<char, OwnValue, Error = ParseError> + Clone {
    choice((
        value_string_parser(),
        value_int_parser(),
        value_ident_parser(),
    ))
    .labelled("value")
}

fn relation_parser() -> impl Parser<char, Relation, Error = ParseError> + Clone {
    let rels = [
        Relation::Attr as u8 as char,
        Relation::Sub as u8 as char,
        Relation::Interpretation as u8 as char,
        Relation::Field as u8 as char,
    ];
    one_of(rels)
        .map(|c| Relation::try_from(c).unwrap_or_else(|_| panic!("bad relation")))
        .labelled("relation")
}

fn path_start_parser<'a>() -> impl Parser<char, PathStart<'a>, Error = ParseError> + Clone {
    let path_start_url = url_parser().map(PathStart::Url).labelled("path_start_url");
    let file_prefix = choice((
        just("./").to("./"),
        just("~/").to("~/"),
        just("/").to("/"),
        empty().to(""),
    ));
    let path_start_file = file_prefix
        .then(path_code_points().separated_by(just('/')))
        .map(|(prefix, parts)| {
            let suffix = parts.join("/");
            let full = if suffix.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}{suffix}")
            };
            PathStart::File(full)
        })
        .labelled("path_start_file");
    let path_start_string = string_parser()
        .map(PathStart::String)
        .labelled("path_start_string");
    choice((path_start_url, path_start_file, path_start_string)).labelled("path_start")
}

pub(super) fn path_with_starter_parser<'a>()
-> impl Parser<char, (PathStart<'a>, Path<'a>), Error = ParseError> + Clone {
    path_start_parser()
        .then_ignore(ws())
        .then(path_items_parser())
        .labelled("path")
}

pub(super) fn path_items_parser<'a>() -> impl Parser<char, Path<'a>, Error = ParseError> + Clone {
    recursive(|path_items| {
        let path_item_selector = path_code_points()
            .map(|s| Selector::from(leak_str(s)))
            .labelled("path_item_selector");
        let path_item_index = just('[')
            .ignore_then(number_isize_parser())
            .then_ignore(just(']'))
            .labelled("path_item_index");
        let type_expression = just(':')
            .ignore_then(identifier_parser())
            .map(|ty| Expression::Type { ty })
            .labelled("type expression");
        let ternary_expression = path_item_parser(path_items.clone())
            .repeated()
            .map(Path)
            .then(
                ws().ignore_then(operation_parser().then_ignore(ws()).then(rvalue_parser()))
                    .or_not(),
            )
            .try_map(|(left, op_right), span| {
                if left.0.is_empty() && op_right.is_none() {
                    return Err(Simple::custom(span, "empty ternary expression"));
                }
                Ok(Expression::Ternary { left, op_right })
            })
            .labelled("ternary expression");
        let expression = ws()
            .ignore_then(
                choice((type_expression, ternary_expression))
                    .separated_by(just('|'))
                    .at_least(1),
            )
            .map(|expressions: Vec<Expression<'_>>| {
                if expressions.len() == 1 {
                    expressions.into_iter().next().unwrap()
                } else {
                    Expression::Or { expressions }
                }
            })
            .labelled("expression");
        let filter = just('[')
            .ignore_then(expression)
            .then_ignore(just(']'))
            .map(|expr| Filter { expr })
            .labelled("filter");

        let interpretation_param_longform = identifier_parser()
            .then_ignore(ws())
            .then_ignore(just('='))
            .then_ignore(ws())
            .then(rvalue_parser())
            .map(|(name, value)| InterpretationParam {
                name: Some(name),
                value,
            })
            .labelled("interpretation parameter longform");
        let interpretation_param_shortform =
            choice((rvalue_parser(), identifier_parser().map(OwnValue::String)))
                .map(|value| InterpretationParam { name: None, value })
                .labelled("interpretation parameter shortform");
        let interpretation_param = just('[')
            .ignore_then(ws())
            .ignore_then(choice((
                interpretation_param_longform,
                interpretation_param_shortform,
            )))
            .then_ignore(ws())
            .then_ignore(just(']'))
            .labelled("interpretation parameter");

        let elevation_path_item = ws()
            .ignore_then(just('^'))
            .ignore_then(ws())
            .ignore_then(path_item_selector.clone())
            .then_ignore(ws())
            .then(interpretation_param.repeated())
            .map(|(interpretation, params)| {
                PathItem::Elevation(ElevationPathItem {
                    interpretation,
                    params,
                })
            })
            .labelled("elevation path item");

        let normal_path_item = ws()
            .ignore_then(relation_parser())
            .then_ignore(ws())
            .then(path_item_selector.or_not())
            .then_ignore(ws())
            .then(path_item_index.or_not())
            .then_ignore(ws())
            .then(filter.repeated())
            .then_ignore(ws())
            .try_map(|(((relation, selector), index), filters), span| {
                if selector.is_none() && index.is_none() {
                    return Err(Simple::custom(
                        span,
                        "normal path item requires selector or index",
                    ));
                }
                if relation == Relation::Field && !filters.is_empty() {
                    return Err(Simple::custom(span, "field relation cannot have filters"));
                }
                Ok(PathItem::Normal(NormalPathItem {
                    relation,
                    selector,
                    index,
                    filters,
                }))
            })
            .labelled("normal path item");

        choice((elevation_path_item, normal_path_item))
            .repeated()
            .map(Path)
            .labelled("path_items")
    })
}

fn path_item_parser<'a>(
    path_items: impl Parser<char, Path<'a>, Error = ParseError> + Clone + 'a,
) -> impl Parser<char, PathItem<'a>, Error = ParseError> + Clone {
    let path_item_selector = path_code_points()
        .map(|s| Selector::from(leak_str(s)))
        .labelled("path_item_selector");
    let path_item_index = just('[')
        .ignore_then(number_isize_parser())
        .then_ignore(just(']'))
        .labelled("path_item_index");
    let type_expression = just(':')
        .ignore_then(identifier_parser())
        .map(|ty| Expression::Type { ty })
        .labelled("type expression");
    let ternary_expression = path_items
        .clone()
        .then(
            ws().ignore_then(operation_parser().then_ignore(ws()).then(rvalue_parser()))
                .or_not(),
        )
        .try_map(|(left, op_right), span| {
            if left.0.is_empty() && op_right.is_none() {
                return Err(Simple::custom(span, "empty ternary expression"));
            }
            Ok(Expression::Ternary { left, op_right })
        })
        .labelled("ternary expression");
    let expression = ws()
        .ignore_then(
            choice((type_expression, ternary_expression))
                .separated_by(just('|'))
                .at_least(1),
        )
        .map(|expressions: Vec<Expression<'_>>| {
            if expressions.len() == 1 {
                expressions.into_iter().next().unwrap()
            } else {
                Expression::Or { expressions }
            }
        })
        .labelled("expression");
    let filter = just('[')
        .ignore_then(expression)
        .then_ignore(just(']'))
        .map(|expr| Filter { expr })
        .labelled("filter");
    let interpretation_param_longform = identifier_parser()
        .then_ignore(ws())
        .then_ignore(just('='))
        .then_ignore(ws())
        .then(rvalue_parser())
        .map(|(name, value)| InterpretationParam {
            name: Some(name),
            value,
        })
        .labelled("interpretation parameter longform");
    let interpretation_param_shortform =
        choice((rvalue_parser(), identifier_parser().map(OwnValue::String)))
            .map(|value| InterpretationParam { name: None, value })
            .labelled("interpretation parameter shortform");
    let interpretation_param = just('[')
        .ignore_then(ws())
        .ignore_then(choice((
            interpretation_param_longform,
            interpretation_param_shortform,
        )))
        .then_ignore(ws())
        .then_ignore(just(']'))
        .labelled("interpretation parameter");

    let elevation_path_item = ws()
        .ignore_then(just('^'))
        .ignore_then(ws())
        .ignore_then(path_item_selector.clone())
        .then_ignore(ws())
        .then(interpretation_param.repeated())
        .map(|(interpretation, params)| {
            PathItem::Elevation(ElevationPathItem {
                interpretation,
                params,
            })
        })
        .labelled("elevation path item");
    let normal_path_item = ws()
        .ignore_then(relation_parser())
        .then_ignore(ws())
        .then(path_item_selector.or_not())
        .then_ignore(ws())
        .then(path_item_index.or_not())
        .then_ignore(ws())
        .then(filter.repeated())
        .then_ignore(ws())
        .try_map(|(((relation, selector), index), filters), span| {
            if selector.is_none() && index.is_none() {
                return Err(Simple::custom(
                    span,
                    "normal path item requires selector or index",
                ));
            }
            if relation == Relation::Field && !filters.is_empty() {
                return Err(Simple::custom(span, "field relation cannot have filters"));
            }
            Ok(PathItem::Normal(NormalPathItem {
                relation,
                selector,
                index,
                filters,
            }))
        })
        .labelled("normal path item");

    choice((elevation_path_item, normal_path_item)).labelled("path_item")
}

#[cfg(test)]
#[test]
fn test_parse_string() {
    assert_eq!(
        string_parser().parse(r#""(\w+)""#),
        Ok(r#"(\w+)"#.to_string())
    );
    assert_eq!(
        string_parser().parse(r#"'(\w+)'"#),
        Ok(r#"(\w+)"#.to_string())
    );
    assert_eq!(
        string_parser().parse(r#""(\\w+)""#),
        Ok(r#"(\w+)"#.to_string())
    );
    assert_eq!(
        string_parser().parse(r#""(\w+.\w+)@(\w+)""#),
        Ok(r#"(\w+.\w+)@(\w+)"#.to_string())
    );
}

#[cfg(test)]
#[test]
fn test_parse_file_path_start_variants() {
    assert_eq!(
        path_start_parser().then_ignore(end()).parse("~/x"),
        Ok(PathStart::File("~/x".to_string()))
    );
    assert_eq!(
        path_start_parser().then_ignore(end()).parse("./x"),
        Ok(PathStart::File("./x".to_string()))
    );
    assert_eq!(
        path_start_parser().then_ignore(end()).parse("/x/y"),
        Ok(PathStart::File("/x/y".to_string()))
    );
    assert_eq!(
        path_start_parser().then_ignore(end()).parse("x"),
        Ok(PathStart::File("x".to_string()))
    );
    assert_eq!(
        path_start_parser().then_ignore(end()).parse("x/y"),
        Ok(PathStart::File("x/y".to_string()))
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
