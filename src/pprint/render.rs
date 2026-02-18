use std::fmt::{Error, Write};

use crate::config::ColorPalette;

const SPACE_TO_SEPARATOR: usize = 32;
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_INTERPRETATION_DARK: &str = "2";
const COLOR_TYPE_STRING_DARK: &str = "38;5;114";
const COLOR_TYPE_NUMBER_DARK: &str = "38;5;81";
const COLOR_TYPE_BOOL_DARK: &str = "38;5;221";
const COLOR_TREE_DARK: &str = "2";
const COLOR_EDGE_PREFIX_DARK: &str = "34";
const COLOR_KEY_DARK: &str = "39";
const COLOR_ERROR_DARK: &str = "31";

const COLOR_INTERPRETATION_LIGHT: &str = "2";
const COLOR_TYPE_STRING_LIGHT: &str = "38;5;22";
const COLOR_TYPE_NUMBER_LIGHT: &str = "38;5;88";
const COLOR_TYPE_BOOL_LIGHT: &str = "38;5;130";
const COLOR_TREE_LIGHT: &str = "2";
const COLOR_EDGE_PREFIX_LIGHT: &str = "24";
const COLOR_KEY_LIGHT: &str = "39";
const COLOR_ERROR_LIGHT: &str = "160";

pub(crate) struct TreePrefix {
    pub(crate) ancestors_have_next: Vec<bool>,
    pub(crate) has_parent: bool,
    pub(crate) is_last: bool,
}

pub(crate) struct LineData {
    pub(crate) interpretation: String,
    pub(crate) value_type: String,
    pub(crate) edge_prefix: String,
    pub(crate) key: Option<String>,
    pub(crate) key_error: Option<String>,
    pub(crate) value: Option<LineValue>,
    pub(crate) value_error: Option<String>,
    pub(crate) read_error: Option<String>,
    pub(crate) empty: bool,
}

pub(crate) enum LineValue {
    Inline(String),
    Bytes(String),
    Multiline(Vec<String>),
}

struct PPrintTheme {
    branch: &'static str,
    last_branch: &'static str,
    vertical: &'static str,
    space: &'static str,
    multiline_prefix: &'static str,
    empty_marker: &'static str,
    bytes_open: &'static str,
    bytes_close: &'static str,
}

pub(crate) fn render_line(
    line_data: &LineData,
    tree_prefix: &TreePrefix,
    color_palette: ColorPalette,
) -> Result<String, Error> {
    let mut line = String::new();
    let theme = PPrintTheme::unicode();
    let palette = palette_colors(color_palette);
    let value_color = type_color_code(&line_data.value_type, &palette);
    let key_color = if is_none_value(line_data.value.as_ref()) {
        value_color.as_str()
    } else {
        palette.key
    };
    let interpretation = colorize(palette.interpretation, &line_data.interpretation);
    let value_type = colorize(&value_color, &line_data.value_type);
    let tree = colorize(palette.tree, &build_tree_prefix(tree_prefix, &theme));
    let edge_prefix = if line_data.edge_prefix == "@" {
        colorize(palette.tree, &line_data.edge_prefix)
    } else {
        colorize(palette.edge_prefix, &line_data.edge_prefix)
    };

    write!(line, "{} {}", interpretation, value_type)?;
    let width = line_data.interpretation.len() + 1 + line_data.value_type.len();
    if width < SPACE_TO_SEPARATOR {
        write!(line, "{:width$}", "", width = SPACE_TO_SEPARATOR - width)?;
    }
    write!(line, "{}{}", tree, edge_prefix)?;
    let continuation_prefix = line.clone();

    if let Some(err) = &line_data.read_error {
        write!(line, "{}", colorize(palette.error, err))?;
    }
    if let Some(err) = &line_data.key_error {
        write!(line, "{} ", colorize(palette.error, err))?;
    }
    if let Some(key) = &line_data.key {
        write!(line, "{}: ", colorize(key_color, key))?;
    }
    if let Some(err) = &line_data.value_error {
        write!(line, "{}", colorize(palette.error, err))?;
    }
    if let Some(value) = &line_data.value {
        match value {
            LineValue::Inline(v) => {
                write!(line, "{}", colorize(&value_color, v))?;
            }
            LineValue::Bytes(v) => {
                write!(
                    line,
                    "{}{}{}",
                    colorize(palette.tree, theme.bytes_open),
                    colorize(palette.tree, v),
                    colorize(palette.tree, theme.bytes_close)
                )?;
            }
            LineValue::Multiline(lines) => {
                if let Some(first) = lines.first() {
                    write!(
                        line,
                        "{}{}",
                        colorize(&value_color, theme.multiline_prefix),
                        colorize(&value_color, first),
                    )?;
                }
                for value_line in lines.iter().skip(1) {
                    write!(
                        line,
                        "\n{}{}{}",
                        continuation_prefix,
                        colorize(&value_color, theme.multiline_prefix),
                        colorize(&value_color, value_line),
                    )?;
                }
            }
        }
    }
    if line_data.empty {
        write!(line, "{}", colorize(palette.tree, theme.empty_marker))?;
    }
    Ok(line)
}

impl PPrintTheme {
    fn unicode() -> Self {
        Self {
            branch: "├── ",
            last_branch: "└── ",
            vertical: "│   ",
            space: "    ",
            multiline_prefix: "❝ ",
            empty_marker: "•",
            bytes_open: "⟨",
            bytes_close: "⟩",
        }
    }
}

fn build_tree_prefix(tree_prefix: &TreePrefix, theme: &PPrintTheme) -> String {
    let mut prefix = String::new();
    for has_next in &tree_prefix.ancestors_have_next {
        if *has_next {
            prefix.push_str(theme.vertical);
        } else {
            prefix.push_str(theme.space);
        }
    }
    if tree_prefix.has_parent {
        if tree_prefix.is_last {
            prefix.push_str(theme.last_branch);
        } else {
            prefix.push_str(theme.branch);
        }
    }
    prefix
}

fn colorize(color_code: &str, text: &str) -> String {
    if color_code.is_empty() || text.is_empty() {
        return text.to_string();
    }
    format!("\x1b[{}m{}{}", color_code, text, COLOR_RESET)
}

#[derive(Default)]
struct PaletteColors {
    interpretation: &'static str,
    type_string: &'static str,
    type_number: &'static str,
    type_bool: &'static str,
    tree: &'static str,
    edge_prefix: &'static str,
    key: &'static str,
    error: &'static str,
    hash_palette: &'static [u8],
}

fn palette_colors(color_palette: ColorPalette) -> PaletteColors {
    match color_palette {
        ColorPalette::None => PaletteColors::default(),
        ColorPalette::Dark => PaletteColors {
            interpretation: COLOR_INTERPRETATION_DARK,
            type_string: COLOR_TYPE_STRING_DARK,
            type_number: COLOR_TYPE_NUMBER_DARK,
            type_bool: COLOR_TYPE_BOOL_DARK,
            tree: COLOR_TREE_DARK,
            edge_prefix: COLOR_EDGE_PREFIX_DARK,
            key: COLOR_KEY_DARK,
            error: COLOR_ERROR_DARK,
            hash_palette: &[75, 79, 86, 110, 117, 141, 149, 159, 177, 186, 207, 216],
        },
        ColorPalette::Light => PaletteColors {
            interpretation: COLOR_INTERPRETATION_LIGHT,
            type_string: COLOR_TYPE_STRING_LIGHT,
            type_number: COLOR_TYPE_NUMBER_LIGHT,
            type_bool: COLOR_TYPE_BOOL_LIGHT,
            tree: COLOR_TREE_LIGHT,
            edge_prefix: COLOR_EDGE_PREFIX_LIGHT,
            key: COLOR_KEY_LIGHT,
            error: COLOR_ERROR_LIGHT,
            hash_palette: &[18, 23, 24, 25, 31, 52, 53, 58, 88, 94, 124, 130],
        },
    }
}

fn type_color_code(value_type: &str, palette: &PaletteColors) -> String {
    let kind = value_type.to_ascii_lowercase();
    if kind.contains("string") || kind.contains("str") {
        return palette.type_string.to_string();
    }
    if kind.contains("number")
        || kind.contains("float")
        || kind.contains("int")
        || kind.contains("u32")
        || kind.contains("u64")
        || kind.contains("i32")
        || kind.contains("i64")
    {
        return palette.type_number.to_string();
    }
    if kind.contains("bool") {
        return palette.type_bool.to_string();
    }
    let hash = stable_hash(&kind);
    if palette.hash_palette.is_empty() {
        "".to_string()
    } else {
        format!(
            "38;5;{}",
            palette.hash_palette[hash as usize % palette.hash_palette.len()]
        )
    }
}

fn stable_hash(s: &str) -> u32 {
    let mut h = 2166136261u32;
    for b in s.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619u32);
    }
    h
}

fn is_none_value(value: Option<&LineValue>) -> bool {
    matches!(value, Some(LineValue::Inline(v)) if v == "None")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tree_prefix_uses_sibling_state() {
        let prefix = TreePrefix {
            ancestors_have_next: vec![true, false],
            has_parent: true,
            is_last: false,
        };
        assert_eq!(
            build_tree_prefix(&prefix, &PPrintTheme::unicode()),
            "│       ├── "
        );
    }

    #[test]
    fn render_line_without_color_has_no_ansi() {
        let prefix = TreePrefix {
            ancestors_have_next: vec![],
            has_parent: true,
            is_last: true,
        };
        let line = LineData {
            interpretation: "mongo".to_string(),
            value_type: "string".to_string(),
            edge_prefix: "@".to_string(),
            key: Some("name".to_string()),
            key_error: None,
            value: Some(LineValue::Inline("value".to_string())),
            value_error: None,
            read_error: None,
            empty: false,
        };
        let rendered = render_line(&line, &prefix, ColorPalette::None).unwrap();
        println!("rendered: {}", rendered);
        assert!(rendered.contains("mongo string"));
        assert!(rendered.contains("└── @name:"));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    fn render_line_uses_type_color_for_type_and_value() {
        let prefix = TreePrefix {
            ancestors_have_next: vec![],
            has_parent: true,
            is_last: true,
        };
        let line = LineData {
            interpretation: "mongo".to_string(),
            value_type: "string".to_string(),
            edge_prefix: "@".to_string(),
            key: Some("name".to_string()),
            key_error: None,
            value: Some(LineValue::Inline("value".to_string())),
            value_error: None,
            read_error: None,
            empty: false,
        };
        let rendered = render_line(&line, &prefix, ColorPalette::Dark).unwrap();
        assert!(rendered.matches("\x1b[38;5;114m").count() >= 2);
    }

    #[test]
    fn render_line_light_palette_uses_light_string_color() {
        let prefix = TreePrefix {
            ancestors_have_next: vec![],
            has_parent: true,
            is_last: true,
        };
        let line = LineData {
            interpretation: "mongo".to_string(),
            value_type: "string".to_string(),
            edge_prefix: "@".to_string(),
            key: Some("name".to_string()),
            key_error: None,
            value: Some(LineValue::Inline("value".to_string())),
            value_error: None,
            read_error: None,
            empty: false,
        };
        let rendered = render_line(&line, &prefix, ColorPalette::Light).unwrap();
        assert!(rendered.matches("\x1b[38;5;22m").count() >= 2);
    }

    #[test]
    fn render_line_none_value_uses_value_color_for_key() {
        let prefix = TreePrefix {
            ancestors_have_next: vec![],
            has_parent: true,
            is_last: true,
        };
        let line = LineData {
            interpretation: "mongo".to_string(),
            value_type: "number".to_string(),
            edge_prefix: "@".to_string(),
            key: Some("count".to_string()),
            key_error: None,
            value: Some(LineValue::Inline("None".to_string())),
            value_error: None,
            read_error: None,
            empty: false,
        };
        let rendered = render_line(&line, &prefix, ColorPalette::Light).unwrap();
        assert!(rendered.contains("\x1b[38;5;88mcount"));
    }
}
