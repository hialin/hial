use core::fmt;
use linkme::distributed_slice;
use std::{cell::OnceCell, fmt::Debug, rc::Rc};
use tree_sitter::{Parser, Tree, TreeCursor};

use crate::{base::Cell as XCell, base::*, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_RUST: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["rust", "javascript", "python", "go"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Domain {
    language: String,
    source: String,
    tree: Tree,
}

#[derive(Clone)]
pub(crate) struct Cell {
    domain: Rc<Domain>,
    // since the tree is in a Rc, the treecursor is valid as long as the cell is valid
    cursor: TreeCursor<'static>,
    // cache the position since treesitter does not provide it
    position: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct CellReader {
    cell: Cell,
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
    pub(crate) fn from_cell(cell: XCell, lang: &'static str) -> Res<XCell> {
        match cell.interpretation() {
            "value" => {
                let source = cell.read().value()?.as_cow_str().into_owned();
                Self::make_cell(source, lang.to_owned(), Some(cell))
            }
            "fs" => {
                let r = cell.read();
                let path = r.as_file_path()?;
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

    fn is_leaf(&self) -> bool {
        // a node is leaf if its subtee has no field names in tree-sitter
        let mut c = self.cursor.clone();
        if !c.goto_first_child() {
            return true;
        };
        let mut cursor_stack = vec![c];
        while let Some(mut c) = cursor_stack.pop() {
            loop {
                if c.field_name().is_some() {
                    return false;
                }
                let mut c2 = c.clone();
                if c2.goto_first_child() {
                    cursor_stack.push(c2);
                };

                if !c.goto_next_sibling() {
                    break;
                }
            }
        }
        true
    }
}

fn sitter_from_source(source: String, language: String) -> Res<Cell> {
    let sitter_language = match language.as_str() {
        // "go" => unsafe { tree_sitter_go() },
        "javascript" => unsafe { tree_sitter_javascript() },
        // "python" => unsafe { tree_sitter_python() },
        "rust" => unsafe { tree_sitter_rust() },
        _ => return userres(format!("unsupported language: {}", language)),
    };

    // debug!("sitter language: {}", language);
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
        position: 0,
    })
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        let node = self.cell.cursor.node();
        let typ = if node.kind() == &self.cell.domain.source[node.byte_range()] {
            "sym"
        } else {
            node.kind()
        };
        Ok(typ)
    }

    fn index(&self) -> Res<usize> {
        Ok(self.cell.position)
    }

    fn label(&self) -> Res<Value> {
        self.cell
            .cursor
            .field_name()
            .ok_or_else(noerr)
            .map(Value::Str)
    }

    fn value(&self) -> Res<Value> {
        if self.value.get().is_none() {
            let mut opt_value = None;
            let node = self.cell.cursor.node();
            let src = &self.cell.domain.source[node.byte_range()];
            if !node.is_named()
                || node.child_count() == 0
                || node.kind() == "string_literal"
                || self.cell.is_leaf()
            {
                opt_value = Some(src.to_string());
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

    fn serial(&self) -> Res<String> {
        let node = self.cell.cursor.node();
        let src = &self.cell.domain.source[node.byte_range()];
        Ok(src.to_string())
    }
}

impl CellWriterTrait for Cell {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        // TODO: not clear how to edit the tree because afterwards
        // some cursors/cells will be invalid
        todo!() // TODO: implement this somehow
    }
}

impl CellTrait for Cell {
    type Group = Cell;
    type CellReader = CellReader;
    type CellWriter = Cell;

    fn interpretation(&self) -> &str {
        self.domain.language.as_str()
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            cell: self.clone(),
            value: OnceCell::new(),
        })
    }

    fn write(&self) -> Res<Cell> {
        Ok(self.clone())
    }

    fn sub(&self) -> Res<Cell> {
        if self.is_leaf() {
            return nores();
        }
        Ok(self.clone())
    }

    fn head(&self) -> Res<(Self, Relation)> {
        let mut parent = self.cursor.clone();
        if !parent.goto_parent() {
            return nores();
        }
        let mut position = 0;
        {
            let mut ancestor = parent.clone();
            if ancestor.goto_parent() && ancestor.goto_first_child() {
                loop {
                    if ancestor.node() == parent.node() {
                        break;
                    }
                    position += 1;
                    if !ancestor.goto_next_sibling() {
                        position = 0;
                    }
                }
            }
        }
        Ok((
            Cell {
                domain: self.domain.clone(),
                cursor: parent,
                position,
            },
            Relation::Sub,
        ))
    }
}

impl GroupTrait for Cell {
    type Cell = Cell;
    type CellIterator = std::iter::Once<Res<Cell>>;

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
        // TODO: this is a slow O(n) implementation
        for _ in 0..index {
            if !cursor.goto_next_sibling() {
                return nores();
            }
        }
        // println!("at: {} {}", index, cursor.node().kind());
        Ok(Cell {
            domain: self.domain.clone(),
            cursor,
            position: index,
        })
    }

    fn get_all(&self, key: Value) -> Res<Self::CellIterator> {
        let mut cursor = self.cursor.clone();
        if !cursor.goto_first_child() {
            return nores();
        }
        for i in 0..self.cursor.node().child_count() {
            if key == cursor.field_name().unwrap_or_default() {
                let cell = Cell {
                    domain: self.domain.clone(),
                    cursor,
                    position: i,
                };
                return Ok(std::iter::once(Ok(cell)));
            }
            if !cursor.goto_next_sibling() {
                return nores();
            }
        }
        nores()
    }
}
