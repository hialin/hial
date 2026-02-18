use std::{fmt::Error, io::Read};

use crate::api::*;
use crate::config::ColorPalette;
use crate::pprint::render::{LineData, LineValue, TreePrefix, render_line};

struct PPrintOptions {
    depth: usize,
    breadth: usize,
    color_palette: ColorPalette,
}

pub fn pprint(cell: &Xell, depth: usize, breadth: usize, color_palette: ColorPalette) {
    let options = PPrintOptions {
        depth,
        breadth,
        color_palette,
    };
    let root_prefix = cell.path().unwrap_or_default();
    let has_root_prefix = !root_prefix.is_empty();
    if has_root_prefix {
        let root_line_data = LineData {
            interpretation: String::new(),
            value_type: String::new(),
            edge_prefix: root_prefix,
            key: None,
            key_error: None,
            value: None,
            value_error: None,
            read_error: None,
            empty: false,
        };
        let root_tree_prefix = TreePrefix {
            ancestors_have_next: vec![],
            has_parent: false,
            is_last: false,
        };
        match render_line(&root_line_data, &root_tree_prefix, options.color_palette) {
            Ok(line) => println!("{}", line),
            Err(e) => {
                eprintln!("pprint error: {:?}", e);
                return;
            }
        }
    }
    if let Err(e) = _pprint(cell, "", &options, &[], has_root_prefix, true) {
        eprintln!("pprint error: {:?}", e);
    }
}

fn _pprint(
    cell: &Xell,
    prefix: &str,
    options: &PPrintOptions,
    ancestors_have_next: &[bool],
    has_parent: bool,
    is_last: bool,
) -> Result<(), Error> {
    if options.depth != usize::MAX && ancestors_have_next.len() > options.depth {
        return Ok(());
    }
    let line_data = extract_line_data(cell, prefix);
    let tree_prefix = TreePrefix {
        ancestors_have_next: ancestors_have_next.to_vec(),
        has_parent,
        is_last,
    };
    println!(
        "{}",
        render_line(&line_data, &tree_prefix, options.color_palette)?
    );
    pprint_group(cell, options, &tree_prefix)?;
    Ok(())
}

fn pprint_group(
    cell: &Xell,
    options: &PPrintOptions,
    parent_prefix: &TreePrefix,
) -> Result<(), Error> {
    const SHOW_ELLIPSES: bool = false;
    let indent = parent_prefix.ancestors_have_next.len() + usize::from(parent_prefix.has_parent);
    if options.depth > 0 && indent == options.depth {
        if SHOW_ELLIPSES {
            println!("…");
        }
        return Ok(());
    }

    let mut ancestors = parent_prefix.ancestors_have_next.clone();
    if parent_prefix.has_parent {
        ancestors.push(!parent_prefix.is_last);
    }

    let mut children = cell
        .attr()
        .into_iter()
        .map(|x| ("@", x))
        .chain(cell.sub().into_iter().map(|x| ("", x)))
        .peekable();

    let mut shown = 0usize;
    while let Some((prefix, item)) = children.next() {
        shown += 1;
        let reached_breadth = options.breadth > 0 && shown >= options.breadth;
        let is_last = reached_breadth || children.peek().is_none();
        match item.err() {
            Ok(child) => _pprint(&child, prefix, options, &ancestors, true, is_last)?,
            Err(err) => eprintln!("error: {:?}", err),
        }
        if reached_breadth {
            if SHOW_ELLIPSES {
                println!("…");
            }
            break;
        }
    }

    Ok(())
}

fn extract_line_data(cell: &Xell, prefix: &str) -> LineData {
    let mut value_type = String::new();
    let mut key = None;
    let mut key_error = None;
    let mut value = None;
    let mut value_error = None;
    let mut read_error = None;
    let mut empty = true;

    let reader = match cell.read().err() {
        Ok(reader) => Some(reader),
        Err(err) => {
            if err.kind != HErrKind::None {
                empty = false;
                read_error = Some(format!("⚠cannot read: {:?}⚠", err));
            }
            None
        }
    };

    if let Some(reader) = reader {
        value_type = reader
            .ty()
            .map(|ty| ty.to_string())
            .unwrap_or_else(|e| format!("⚠{:?}⚠", e));

        let key_res = reader.label();
        let value_res = reader.value();
        match key_res {
            Ok(k) => {
                if Some(&k) != value_res.as_ref().ok() {
                    empty = false;
                    key = Some(k.to_string());
                }
            }
            Err(err) => {
                if err.kind != HErrKind::None {
                    empty = false;
                    key_error = Some(format!("⚠{:?}⚠", err));
                }
            }
        }

        match value_res {
            Ok(v) => {
                if empty {
                    empty = v.is_empty();
                }
                if v == Value::Bytes {
                    match reader.value_read() {
                        Ok(mut source) => {
                            let mut bytes = [0; DISPLAY_BYTES_VALUE_LEN + 1];
                            match source.read(&mut bytes) {
                                Ok(n) => {
                                    let bytes = &bytes[..n];
                                    value = Some(LineValue::Bytes(format_bytes(bytes)));
                                }
                                Err(err) => {
                                    empty = false;
                                    value_error = Some(format!("⚠cannot read bytes: {:?}⚠", err));
                                }
                            }
                        }
                        Err(err) => {
                            if err.kind != HErrKind::None {
                                empty = false;
                                value_error = Some(format!("⚠{:?}⚠", err));
                            }
                        }
                    }
                } else if let Some(v) = extract_value(v) {
                    value = Some(v);
                }
            }
            Err(err) => {
                if err.kind != HErrKind::None {
                    empty = false;
                    value_error = Some(format!("⚠{:?}⚠", err));
                }
            }
        }
    }

    LineData {
        interpretation: cell.interpretation().to_string(),
        value_type,
        edge_prefix: prefix.to_string(),
        key,
        key_error,
        value,
        value_error,
        read_error,
        empty,
    }
}

fn extract_value(value: Value) -> Option<LineValue> {
    match value {
        Value::None => Some(LineValue::Inline("None".to_string())),
        Value::Bool(v) => Some(LineValue::Inline(v.to_string())),
        Value::Int(v) => Some(LineValue::Inline(v.to_string())),
        Value::Float(v) => Some(LineValue::Inline(v.to_string())),
        Value::Str(s) => {
            if s.contains('\n') {
                Some(LineValue::Multiline(
                    s.split('\n').map(|line| line.to_string()).collect(),
                ))
            } else {
                Some(LineValue::Inline(s.to_string()))
            }
        }
        Value::Bytes => None,
    }
}

fn format_bytes(bytes: &[u8]) -> String {
    let mut value = String::new();
    let _ = write_bytes(&mut value, bytes);
    value
}

mod render;
