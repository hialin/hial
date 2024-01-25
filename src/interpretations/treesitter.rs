use core::fmt;
use std::{cell::OnceCell, fmt::Debug, rc::Rc};

use linkme::distributed_slice;
use tree_sitter::{Parser, Tree, TreeCursor};

use crate::{
    base::Cell as XCell, base::*, debug, guard_ok, guard_some, tree_sitter_javascript,
    tree_sitter_rust,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_RUST: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["rust", "javascript"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub struct Domain {
    language: String,
    source: String,
    tree: Tree,
}

pub unsafe fn change_lifetime<'old, 'new: 'old, T: 'new>(data: &'old T) -> &'new T {
    &*(data as *const _)
}

#[derive(Clone)]
pub struct Cell {
    domain: Rc<Domain>,
    // since the tree is in a Rc, the treecursor is valid as long as the cell is valid
    cursor: TreeCursor<'static>,
    value: OnceCell<Option<String>>,
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cell({:?}, {:?})",
            self.interpretation(),
            self.cursor.node().kind()
        )
    }
}

impl Cell {
    pub fn from_cell(cell: XCell, lang: &'static str) -> Res<XCell> {
        match cell.interpretation() {
            "value" => {
                let source = cell.read().value()?.as_cow_str().into_owned();
                Self::make_cell(source, lang.to_owned(), Some(cell))
            }
            "fs" => {
                let path = cell.as_file_path()?;
                let source = std::fs::read_to_string(path)
                    .map_err(|e| caused(HErrKind::IO, "cannot read file", e))?;
                Self::make_cell(source, lang.to_owned(), Some(cell))
            }
            _ => nores(),
        }
    }

    fn make_cell(source: String, language: String, origin: Option<XCell>) -> Res<XCell> {
        let ts_cell = sitter_from_source(source, language)?;
        Ok(new_cell(DynCell::from(ts_cell), origin))
    }
}

fn sitter_from_source(source: String, language: String) -> Res<Cell> {
    let sitter_language = match language.as_str() {
        "rust" => unsafe { tree_sitter_rust() },
        "javascript" => unsafe { tree_sitter_javascript() },
        _ => return userr(format!("unsupported language: {}", language)),
    };

    debug!("sitter language: {}", language);

    // println!("node kinds:");
    // let mut nk = vec![];
    // for i in 0..sitter_language.node_kind_count() {
    //     nk.push(sitter_language.node_kind_for_id(i as u16).unwrap_or(""));
    // }
    // nk.sort();
    // for k in &nk {
    //     println!("\t{}", k);
    // }
    // println!("fields");
    // for i in 0..sitter_language.field_count() {
    //     println!(
    //         "\t{}",
    //         sitter_language.field_name_for_id(i as u16).unwrap_or("")
    //     );
    // }

    let mut parser = Parser::new();
    guard_ok!(parser.set_language(sitter_language), err => {
        return Err(caused(HErrKind::Internal, format!("cannot set language {}", language), err));
    });

    let tree = guard_some!(parser.parse(&source, None), {
        return fault("cannot get parse tree");
    });
    let domain = Rc::new(Domain {
        language,
        source,
        tree,
    });
    Ok(Cell {
        domain: domain.clone(),
        // TODO: find alternative for this unsafe change of lifetime to 'static
        // this should be safe as long as the tree is in Rc and not modified
        cursor: unsafe { std::mem::transmute(domain.tree.walk()) },
        value: OnceCell::new(),
    })
}

impl CellReaderTrait for Cell {
    fn index(&self) -> Res<usize> {
        let mut n = self.cursor.node();
        let mut i = 0;
        while let Some(p) = n.prev_sibling() {
            n = p;
            i += 1;
        }
        Ok(i)
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn value(&self) -> Res<Value> {
        if self.value.get().is_none() {
            let mut opt_value = None;
            let node = self.cursor.node();
            if !node.is_named() || node.child_count() == 0 || node.kind() == "string_literal" {
                opt_value = Some(self.domain.source[node.byte_range()].to_string());
            }
            self.value
                .set(opt_value)
                .map_err(|e| faulterr("cannot set value"))?;
        }
        // println!("value: {:?}", self.value.get());
        if let Some(value) = self.value.get().unwrap().as_ref() {
            Ok(Value::Str(value))
        } else {
            nores()
        }
    }
}

impl CellWriterTrait for Cell {}

impl CellTrait for Cell {
    type Group = Cell;
    type CellReader = Cell;
    type CellWriter = Cell;

    fn interpretation(&self) -> &str {
        self.domain.language.as_str()
    }

    fn ty(&self) -> Res<&str> {
        let node = self.cursor.node();
        let typ = self.cursor.field_name().unwrap_or_else(|| {
            if node.kind() == &self.domain.source[node.byte_range()] {
                "symbol"
            } else {
                node.kind()
            }
        });
        Ok(typ)
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(self.clone())
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(self.clone())
    }

    fn sub(&self) -> Res<Cell> {
        Ok(self.clone())
    }

    fn head(&self) -> Res<(Self, Relation)> {
        let mut cursor = self.cursor.clone();
        if !cursor.goto_parent() {
            return nores();
        }
        Ok((
            Cell {
                domain: self.domain.clone(),
                cursor,
                value: OnceCell::new(),
            },
            Relation::Sub,
        ))
    }
}

impl GroupTrait for Cell {
    type Cell = Cell;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: false,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(self.cursor.node().child_count())
    }

    fn at(&self, index: usize) -> Res<Cell> {
        let mut cursor = self.cursor.clone();
        if !cursor.goto_first_child() {
            return nores();
        }
        for _ in 0..index {
            if !cursor.goto_next_sibling() {
                return nores();
            }
        }
        // println!("at: {} {}", index, cursor.node().kind());
        Ok(Cell {
            domain: self.domain.clone(),
            cursor,
            value: OnceCell::new(),
        })
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        if let Some(child_node) = self.cursor.node().child_by_field_name(key.to_string()) {
            Ok(Cell {
                domain: self.domain.clone(),
                cursor: child_node.walk(),
                value: OnceCell::new(),
            })
        } else {
            nores()
        }
    }
}
