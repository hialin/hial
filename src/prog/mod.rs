pub(crate) mod path;
pub(crate) mod program;
pub(crate) mod searcher;
pub(crate) mod url;

pub(super) mod parse_path;
pub(super) mod parse_program;
pub(super) mod parse_url;

pub use path::{Path, PathStart};
pub use program::{Program, ProgramParams};

use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::error::Simple;

pub(super) type ParseError = Simple<char>;

fn convert_error(input: &str, errs: Vec<ParseError>) -> String {
    if errs.is_empty() {
        return "parse error".to_string();
    }

    let mut out = Vec::new();
    for err in errs {
        let span = err.span();
        let report_span = ((), span.clone());
        let expected = format_expected(&err);
        let found = err
            .found()
            .map(|c| format!("`{c}`"))
            .unwrap_or_else(|| "end of input".to_string());

        let report = Report::build(ReportKind::Error, report_span)
            .with_message(format!("expected {expected}, found {found}"))
            .with_label(
                Label::new(((), span))
                    .with_message(format!("expected {expected}"))
                    .with_color(Color::Red),
            )
            .finish();

        let _ = report.write(Source::from(input), &mut out);
    }

    format!("parse error:\n{}", String::from_utf8_lossy(&out))
}

fn format_expected(err: &ParseError) -> String {
    let expected = err
        .expected()
        .map(|token| {
            token
                .map(|c| format!("`{c}`"))
                .unwrap_or_else(|| "end of input".to_string())
        })
        .collect::<Vec<_>>();

    if expected.is_empty() {
        "something else".to_string()
    } else {
        expected.join(", ")
    }
}
