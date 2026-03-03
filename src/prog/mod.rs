pub(crate) mod path;
pub(crate) mod program;
pub(crate) mod searcher;
pub(crate) mod url;

pub(super) mod parse_path;
pub(super) mod parse_program;
pub(super) mod parse_url;

pub use path::{Path, PathStart};
pub use program::{ExecutionContext, Program, ProgramParams};

use chumsky::error::Rich;
use chumsky::span::SimpleSpan;
use std::ops::Range;

use ariadne::{CharSet, Config, Label, Report, ReportKind, sources};

pub(super) type ParseError<'src> = Rich<'src, char>;

fn normalize_span(span: &SimpleSpan<usize>, input: &str) -> Range<usize> {
    let input_len = input.chars().count();
    let mut start = span.start.min(input_len);
    let mut end = span.end.min(input_len);

    if start == end && input_len > 0 {
        if end < input_len {
            end += 1;
        } else {
            start = start.saturating_sub(1);
        }
    }

    start..end
}

fn convert_error(input: &str, errs: Vec<ParseError<'_>>, source_name: &'static str) -> String {
    if errs.is_empty() {
        return format!("parse error in {source_name}");
    }

    let config = Config::default()
        .with_color(true)
        .with_char_set(CharSet::Unicode);
    let mut output = Vec::new();

    for err in errs {
        let span = normalize_span(err.span(), input);
        let message = err.to_string();

        let report = Report::build(ReportKind::Error, (source_name, span.clone()))
            .with_config(config)
            .with_message("parse error")
            .with_label(Label::new((source_name, span)).with_message(message))
            .finish();

        if report
            .write(sources([(source_name, input)]), &mut output)
            .is_err()
        {
            return format!("parse error in {source_name}");
        }
    }

    String::from_utf8_lossy(&output).trim_end().to_string()
}
