use std::fmt::Error;

use crate::base::*;

pub fn pprint(cell: &Cell, depth: usize, breadth: usize) {
    let mut buffer = String::new();
    if let Err(e) = _pprint(cell, "", depth, breadth, 0, &mut buffer) {
        eprintln!("pprint error: {:?}", e);
    }
}

fn _pprint(
    cell: &Cell,
    prefix: &str,
    depth: usize,
    breadth: usize,
    indent: usize,
    buffer: &mut String,
) -> Result<(), Error> {
    if depth > 0 && indent > depth {
        return Ok(());
    }
    print_cell(&cell, prefix, indent, buffer)?;
    if let Ok(attr) = cell.attr() {
        pprint_group("@", attr, depth, breadth, indent, buffer)?;
    }
    if let Ok(subs) = cell.sub() {
        pprint_group("", subs, depth, breadth, indent, buffer)?;
    }
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
            match x {
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
    const SPACE_TO_SEPARATOR: usize = 32;
    const SEPARATORS: &'static [&'static str] = &["│ ", "╞ ", "┝ ", "├ "];
    const INDENT: usize = 4;

    while buffer.len() < SPACE_TO_SEPARATOR {
        buffer.push_str(" ");
    }

    let mut visual_correction = 0;
    if indent > 0 {
        let typeseparator: &'static str = SEPARATORS.get(indent).unwrap_or(&SEPARATORS[0]);
        buffer.push_str(typeseparator);
        visual_correction = 2; // unicode separator has 3 bytes for 1 char
    }

    let width = buffer.len() - visual_correction + INDENT * indent;
    while buffer.len() < width {
        buffer.push_str(" ");
    }

    Ok(())
}

fn print_cell(cell: &Cell, prefix: &str, indent: usize, buffer: &mut String) -> Result<(), Error> {
    use std::fmt::Write;

    let mut typ = String::new();
    write!(
        buffer,
        "{} {}",
        cell.domain().interpretation(),
        cell.typ().unwrap_or_else(|e| {
            typ = format!("⚠{:?}⚠", e);
            &typ
        })
    )?;
    make_indent(indent, buffer)?;
    write!(buffer, "{}", prefix)?;

    let keyref = cell.label();
    let valueref = cell.value();
    let key = keyref.get();
    let value = valueref.get();
    match key {
        Ok(k) => {
            if Some(&k) != value.as_ref().ok() {
                write!(buffer, "{}: ", k)
            } else {
                write!(buffer, "")
            }
        }
        Err(HErr::NotFound(_)) => write!(buffer, ""),
        Err(err) => write!(buffer, "⚠{:?}⚠ ", err),
    }?;
    match value {
        Ok(v) => write!(buffer, "{}", v),
        Err(err) => write!(buffer, "⚠{:?}⚠", err),
    }?;

    match cell.sub() {
        Ok(group) => {
            let kt = group.label_type();
            write!(buffer, "{}", if kt.is_indexed { "" } else { " ∤" })
        }
        Err(HErr::NotFound(_)) => write!(buffer, ""),
        Err(err) => write!(buffer, "⚠{:?}⚠", err),
    }?;

    println!("{}", buffer);
    buffer.clear();
    Ok(())
}
