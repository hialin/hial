use core::fmt;
use std::{cell::OnceCell, fmt::Debug, path::Path, rc::Rc};

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

// TODO: rewrite treesitter interpretation as a thin wrapper around a treesitter cursor
// will have to solve the lifetime problem (cursor has a lifetime tied to the tree)
// which is not allowed by the current interpretation traits

#[derive(Clone, Debug)]
pub struct Domain(Rc<DomainData>);

#[derive(Clone, Debug)]
pub struct DomainData {
    language: String,
    source: String,
    tree: Tree,
    origin: Option<XCell>,
}

pub unsafe fn change_lifetime<'old, 'new: 'old, T: 'new>(data: &'old T) -> &'new T {
    &*(data as *const _)
}

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        self.0.language.as_str()
    }

    fn root(&self) -> Res<Self::Cell> {
        let cursor = self.0.tree.walk();
        Ok(Cell {
            domain: self.clone(),
            // TODO: unsafe change of lifetime to 'static
            // should be safe as long as the tree is in Rc and not modified
            cursor: unsafe { std::mem::transmute(cursor) },
            value: OnceCell::new(),
        })
    }

    fn origin(&self) -> Res<XCell> {
        self.0.origin.clone().ok_or(noerr())
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

#[derive(Clone)]
pub struct Cell {
    domain: Domain,
    // since the tree is in a Rc, the treecursor is valid as long as the cell is valid
    cursor: TreeCursor<'static>,
    value: OnceCell<Option<String>>,
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cell({:?}, {:?})",
            self.domain.interpretation(),
            self.cursor.node().kind()
        )
    }
}

impl Cell {
    pub fn from_cell(cell: XCell, lang: &'static str) -> Res<XCell> {
        match cell.domain().interpretation() {
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

    pub fn from_path(path: &Path, language: String) -> Res<XCell> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| caused(HErrKind::IO, "cannot read file", e))?;
        Self::make_cell(source, language, None)
    }

    pub fn from_string(source: String, language: String) -> Res<XCell> {
        Self::make_cell(source, language, None)
    }

    fn make_cell(source: String, language: String, origin: Option<XCell>) -> Res<XCell> {
        let domain = sitter_from_source(source, language, origin)?;
        Ok(XCell {
            dyn_cell: DynCell::from(domain.root()?),
        })
    }
}

fn sitter_from_source(source: String, language: String, origin: Option<XCell>) -> Res<Domain> {
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
    Ok(Domain(Rc::new(DomainData {
        language,
        source,
        tree,
        origin,
    })))
}

impl CellReaderTrait for Cell {
    fn index(&self) -> Res<usize> {
        self.cursor.field_id().map(|i| i as usize).ok_or_else(noerr)
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn value(&self) -> Res<Value> {
        if self.value.get().is_none() {
            let mut opt_value = None;
            let node = self.cursor.node();
            if !node.is_named() || node.child_count() == 0 || node.kind() == "string_literal" {
                opt_value = Some(self.domain().0.source[node.byte_range()].to_string());
            }
            self.value
                .set(opt_value)
                .map_err(|e| faulterr("cannot set value"))?;
        }
        // println!("value: {:?}", self.value.get());
        if let Some(value) = self.value.get().unwrap() {
            Ok(Value::Str(value))
        } else {
            nores()
        }
    }
}

impl CellWriterTrait for Cell {}

impl CellTrait for Cell {
    type Domain = Domain;
    type Group = Cell;
    type CellReader = Cell;
    type CellWriter = Cell;

    fn domain(&self) -> Domain {
        self.domain.clone()
    }

    fn ty(&self) -> Res<&str> {
        let node = self.cursor.node();
        let typ = self.cursor.field_name().unwrap_or_else(|| {
            if node.kind() == &self.domain().0.source[node.byte_range()] {
                "literal"
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
