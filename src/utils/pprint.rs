use std::{
    fmt::{Error, Write},
    io::Read,
};

use crate::api::*;

const SPACE_TO_SEPARATOR: usize = 32;
const SEPARATORS: &[&str] = &["│ ", "╞ ", "┝ ", "├ "];
const INDENT: usize = 4;
const ANSI_DIM_ON: &str = "\x1b[2m";
const ANSI_DIM_OFF: &str = "\x1b[22m";

pub fn pprint(cell: &Xell, depth: usize, breadth: usize) {
    let mut buffer = String::new();
    if let Err(e) = _pprint(cell, "", depth, breadth, 0, &mut buffer) {
        eprintln!("pprint error: {:?}", e);
    }
}

fn _pprint(
    cell: &Xell,
    prefix: &str,
    depth: usize,
    breadth: usize,
    indent: usize,
    buffer: &mut String,
) -> Result<(), Error> {
    if indent > depth || depth == usize::MAX {
        return Ok(());
    }
    print_cell(cell, prefix, indent, buffer)?;
    pprint_group("@", cell.attr(), depth, breadth, indent, buffer)?;
    pprint_group("", cell.sub(), depth, breadth, indent, buffer)?;
    Ok(())
}

fn pprint_group(
    prefix: &str,
    group: Group,
    depth: usize,
    breadth: usize,
    indent: usize,
    buffer: &mut String,
) -> Result<(), Error> {
    const SHOW_ELLIPSES: bool = false;
    if depth > 0 && indent + 1 == depth {
        if SHOW_ELLIPSES {
            make_indent(indent + 1, buffer)?;
            println!("{}{}…", buffer, prefix);
            buffer.clear();
        }
    } else {
        for (i, x) in group.into_iter().enumerate() {
            match x.err() {
                Ok(x) => _pprint(&x, prefix, depth, breadth, indent + 1, buffer)?,
                Err(err) => eprintln!("error: {:?}", err),
            }
            if breadth > 0 && i + 1 >= breadth {
                if SHOW_ELLIPSES {
                    make_indent(indent + 1, buffer)?;
                    println!("{}{}…", buffer, prefix);
                    buffer.clear();
                }
                break;
            }
        }
    }
    Ok(())
}

fn make_indent(indent: usize, buffer: &mut String) -> Result<(), Error> {
    while buffer.len() < SPACE_TO_SEPARATOR {
        buffer.push(' ');
    }

    let mut visual_correction = 0;
    if indent > 0 {
        let typeseparator: &'static str = SEPARATORS.get(indent).unwrap_or(&SEPARATORS[0]);
        buffer.push_str(typeseparator);
        visual_correction = 2; // unicode separator has 3 bytes for 1 char
    }

    let width = buffer.len() - visual_correction + INDENT * indent;
    while buffer.len() < width {
        buffer.push(' ');
    }

    Ok(())
}

fn print_cell(cell: &Xell, prefix: &str, indent: usize, buffer: &mut String) -> Result<(), Error> {
    let mut typ = String::new();
    write!(buffer, "{} ", cell.interpretation(),)?;

    let mut empty = true;
    let reader = match cell.read().err() {
        Ok(reader) => Some(reader),
        Err(err) => {
            if err.kind != HErrKind::None {
                empty = false;
                write!(buffer, "⚠cannot read: {:?}⚠", err)?;
            }
            None
        }
    };

    if let Some(reader) = reader {
        write!(
            buffer,
            "{}",
            reader.ty().unwrap_or_else(|e| {
                typ = format!("⚠{:?}⚠", e);
                &typ
            })
        )?;
        make_indent(indent, buffer)?;
        write!(buffer, "{}", prefix)?;

        let key = reader.label();
        let value = reader.value();
        match key {
            Ok(k) => {
                if Some(&k) != value.as_ref().ok() {
                    empty = false;
                    write!(buffer, "{}: ", k)
                } else {
                    write!(buffer, "")
                }
            }
            Err(err) => {
                if err.kind == HErrKind::None {
                    write!(buffer, "")
                } else {
                    empty = false;
                    write!(buffer, "⚠{:?}⚠ ", err)
                }
            }
        }?;
        match value {
            Ok(v) => {
                // write!(buffer, "empty={} ", v.is_empty())?;
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
                                    print_bytes(buffer, bytes)
                                }
                                Err(err) => {
                                    empty = false;
                                    write!(buffer, "⚠cannot read bytes: {:?}", err)
                                }
                            }
                        }
                        Err(err) => {
                            if err.kind == HErrKind::None {
                                write!(buffer, "")
                            } else {
                                empty = false;
                                write!(buffer, "⚠{:?}⚠", err)
                            }
                        }
                    }
                } else {
                    print_value(buffer, indent, v)
                }
            }
            Err(err) => {
                if err.kind == HErrKind::None {
                    write!(buffer, "")
                } else {
                    empty = false;
                    write!(buffer, "⚠{:?}⚠", err)
                }
            }
        }?;
    }

    if empty {
        write!(buffer, "•")?;
    }

    println!("{}", buffer);
    buffer.clear();
    Ok(())
}

fn print_value(buffer: &mut String, indent: usize, v: Value) -> Result<(), Error> {
    match v {
        Value::None => write!(buffer, "None"),
        Value::Bool(x) => write!(buffer, "{}", x),
        Value::Int(x) => write!(buffer, "{}", x),
        Value::Float(x) => write!(buffer, "{}", x),
        Value::Str(x) => print_string(buffer, indent, x),
        Value::Bytes => write!(buffer, "⟨bytes⟩"),
    }
}

fn print_bytes(buffer: &mut String, bytes: &[u8]) -> Result<(), Error> {
    write!(buffer, "{}⟨", ANSI_DIM_ON)?;
    write_bytes(buffer, bytes)?;
    write!(buffer, "⟩{}", ANSI_DIM_OFF)
}

fn print_string(buffer: &mut String, indent: usize, s: &str) -> Result<(), Error> {
    if !s.contains('\n') {
        return write!(buffer, "{}", s);
    }

    let mut pre = String::new();
    make_indent(indent, &mut pre)?;
    pre.push_str("❝ ");

    for (i, l) in s.split('\n').enumerate() {
        if i == 0 {
            writeln!(buffer, "❝ {}", l)?;
        } else {
            writeln!(buffer, "{}{}", pre, l)?;
        }
    }
    if buffer.ends_with("\n\n") {
        buffer.pop(); // remove last '\n'
    }
    if buffer.ends_with('\n') {
        buffer.pop();
    }
    Ok(())
}
