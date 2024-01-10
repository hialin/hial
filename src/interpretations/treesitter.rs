use std::{ops::Range, path::Path, rc::Rc};

use linkme::distributed_slice;
use tree_sitter::{Parser, Tree, TreeCursor};

use crate::{
    base::{Cell as XCell, *},
    tree_sitter_language, *,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_RUST: ElevationConstructor = ElevationConstructor {
    source_interpretation: "value",
    target_interpretation: "rust",
    constructor: Cell::from_value_cell_rust,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static FILE_TO_RUST: ElevationConstructor = ElevationConstructor {
    source_interpretation: "file",
    target_interpretation: "rust",
    constructor: Cell::from_file_cell_rust,
};

#[derive(Clone, Debug)]
pub struct Domain(Rc<DomainData>);

#[derive(Clone, Debug)]
pub struct DomainData {
    language: &'static str,
    source: String,
    tree: Tree,
}

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        self.0.language
    }

    fn root(&self) -> Res<Self::Cell> {
        let cnode = node_to_cnode(self.0.tree.walk(), &self.0.source);

        let group = Group {
            domain: self.clone(),
            nodes: Rc::new(vec![cnode]),
        };
        Ok(Cell { group, pos: 0 })
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Domain,
    // since the tree is in a Rc, the treecursor is valid on self's lifetime
    nodes: Rc<Vec<CNode>>,
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Debug)]
pub struct CellReader {
    group: Group,
    pos: usize,
}

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

#[derive(Clone, Debug)]
pub struct CNode {
    typ: &'static str,
    name: Option<&'static str>,
    value: String,
    subs: Rc<Vec<CNode>>,
    src: Range<usize>,
}

impl Cell {
    pub fn from_value_cell_rust(cell: XCell) -> Res<XCell> {
        Cell::from_value_cell(cell, "rust")
    }
    pub fn from_file_cell_rust(cell: XCell) -> Res<XCell> {
        Cell::from_file_cell(cell, "rust")
    }

    pub fn from_value_cell(cell: XCell, language: &'static str) -> Res<XCell> {
        let reader = cell.read();
        let value = reader.value()?;
        let source = value.as_cow_str();
        let domain = sitter_from_source(source.into_owned(), language)?;
        Ok(XCell {
            dyn_cell: DynCell::from(domain.root()?),
        })
    }

    pub fn from_file_cell(cell: XCell, language: &'static str) -> Res<XCell> {
        let path = cell.as_path()?;
        Cell::from_path(path, language)
    }

    pub fn from_path(path: &Path, language: &'static str) -> Res<XCell> {
        let source = std::fs::read_to_string(path)?;
        Cell::from_string(source, language)
    }

    pub fn from_string(source: String, language: &'static str) -> Res<XCell> {
        let domain = sitter_from_source(source, language)?;
        Ok(XCell {
            dyn_cell: DynCell::from(domain.root()?),
        })
    }

    pub fn get_underlying_string(cell: &Cell) -> Res<&str> {
        let cnode = guard_some!(cell.group.nodes.get(cell.pos), {
            return fault("bad pos in rust cell");
        });
        Ok(&cell.group.domain.0.source[cnode.src.clone()])
    }
}

fn sitter_from_source(source: String, language: &'static str) -> Res<Domain> {
    let sitter_language = guard_some!(tree_sitter_language(language), {
        return Err(HErr::Sitter(format!("unsupported language: {}", language)));
    });

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
        return Err(HErr::Sitter(format!("{}", err)));
    });

    let tree = guard_some!(parser.parse(&source, None), {
        return Err(HErr::Sitter("cannot get parse tree".to_string()));
    });
    Ok(Domain(Rc::new(DomainData {
        language,
        source,
        tree,
    })))
}

fn node_to_cnode(mut cursor: TreeCursor, source: &str) -> CNode {
    let node = cursor.node();
    let name = cursor.field_name();
    let typ = if node.is_named() { node.kind() } else { "" };
    let src = &source[node.byte_range()];

    let mut value = String::new();
    if !node.is_named() || node.child_count() == 0 {
        value = src.to_string();
    }
    if typ == "string_literal" {
        value = src.to_string();
    }

    let mut subs = vec![];
    if cursor.goto_first_child() && typ != "string_literal" {
        subs.push(node_to_cnode(cursor.clone(), source));
        while cursor.goto_next_sibling() {
            subs.push(node_to_cnode(cursor.clone(), source));
        }
    }

    // reshape_subs(&mut value, typ, &mut subs, source);

    CNode {
        typ,
        name,
        value,
        subs: Rc::new(subs),
        src: node.start_byte()..node.end_byte(),
    }
}

fn reshape_subs(value: &mut String, typ: &str, subs: &mut Vec<CNode>, source: &str) {
    if subs.len() > 1 {
        let first = &subs[0];
        let last = &subs[subs.len() - 1];
        let s1 = &source[first.src.clone()];
        let s2 = &source[last.src.clone()];
        if (s1, s2) == ("(", ")")
            || (s1, s2) == ("[", "]")
            || (s1, s2) == ("{", "}")
            || (s1, s2) == ("<", ">")
        {
            *value = format!("{}{}{}", s1, value, s2);
            subs.remove(0);
            subs.remove(subs.len() - 1);
        } else if value.is_empty()
            && !first.value.is_empty()
            && first.subs.is_empty()
            && typ.starts_with(&first.value)
        {
            *value = subs[0].value.clone();
            subs.remove(0);
        }

        let mut new_subs = vec![];
        for s in subs.drain(..) {
            if s.value == "," && (value == "()" || value == "[]" || value == "{}" || value == "<>")
            {
            } else {
                new_subs.push(s);
            }
        }
        *subs = new_subs;
    }

    if subs.len() == 1 && typ == "visibility_modifier" && value.is_empty() {
        *value = subs.remove(0).value;
    }
    if subs.len() > 1 && subs[subs.len() - 1].typ.is_empty() && subs[subs.len() - 1].value == ";" {
        subs.remove(subs.len() - 1);
    }
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        if let Some(label) = self.group.nodes[self.pos].name {
            Ok(Value::Str(label))
        } else {
            nores()
        }
    }

    fn value(&self) -> Res<Value> {
        let cnode = &self.group.nodes[self.pos];
        if cnode.value.is_empty() {
            Ok(Value::None)
        } else {
            Ok(Value::Str(&cnode.value))
        }
    }
}

impl CellTrait for Cell {
    type Domain = Domain;
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn domain(&self) -> Domain {
        self.group.domain.clone()
    }

    fn typ(&self) -> Res<&str> {
        Ok(self.group.nodes[self.pos].typ)
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            group: self.group.clone(),
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }

    fn sub(&self) -> Res<Group> {
        let cnode = &self.group.nodes[self.pos];
        let mut group = self.group.clone();
        group.nodes = cnode.subs.clone();
        Ok(group)
    }
}

impl GroupTrait for Group {
    type Cell = Cell;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: false,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(self.nodes.len())
    }

    fn at(&self, index: usize) -> Res<Cell> {
        if index < self.nodes.len() {
            Ok(Cell {
                group: self.clone(),
                pos: index,
            })
        } else {
            nores()
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        for (i, node) in self.nodes.iter().enumerate() {
            if let Some(name) = node.name {
                if key == name {
                    return self.at(i);
                }
            }
        }
        nores()
    }
}
