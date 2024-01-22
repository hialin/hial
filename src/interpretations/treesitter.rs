use std::{ops::Range, path::Path, rc::Rc};

use linkme::distributed_slice;
use tree_sitter::{Parser, Tree, TreeCursor};

use crate::{
    base::{Cell as XCell, *},
    *,
};

use self::utils::ownrc::{OwnRc, ReadRc, WriteRc};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_RUST: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["rust", "javascript"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub struct Domain(Rc<DomainData>);

#[derive(Clone, Debug)]
pub struct DomainData {
    language: String,
    source: String,
    tree: Tree,
    origin: Option<XCell>,
}

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        self.0.language.as_str()
    }

    fn root(&self) -> Res<Self::Cell> {
        let cnode = node_to_cnode(self.0.tree.walk(), &self.0.source);

        let group = Group {
            domain: self.clone(),
            nodes: OwnRc::new(vec![cnode]),
        };
        Ok(Cell { group, pos: 0 })
    }

    fn origin(&self) -> Res<XCell> {
        self.0.origin.clone().ok_or(noerr())
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Domain,
    // since the tree is in a Rc, the treecursor is valid on self's lifetime
    nodes: OwnRc<Vec<CNode>>,
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Debug)]
pub struct CellReader {
    nodes: ReadRc<Vec<CNode>>,
    pos: usize,
}

#[derive(Debug)]
pub struct CellWriter {
    nodes: WriteRc<Vec<CNode>>,
    pos: usize,
}
impl CellWriterTrait for CellWriter {}

#[derive(Clone, Debug)]
pub struct CNode {
    typ: &'static str,
    value: Option<String>,
    subs: OwnRc<Vec<CNode>>,
    src: Range<usize>,
    head: Option<OwnRc<Vec<CNode>>>,
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

    pub fn get_underlying_string(cell: &Cell) -> Res<&str> {
        let reader = cell
            .group
            .nodes
            .read()
            .ok_or_else(|| lockerr("cannot read nodes"))?;
        let cnode = guard_some!(reader.get(cell.pos), {
            return fault("bad pos in rust cell");
        });
        Ok(&cell.group.domain.0.source[cnode.src.clone()])
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

fn node_to_cnode(mut cursor: TreeCursor, source: &str) -> CNode {
    let node = cursor.node();
    let src = &source[node.byte_range()];

    // node.kind() is treesiter's structural type, we prefer a more semantic type
    let typ = cursor.field_name().unwrap_or_else(|| {
        if node.kind() == src {
            "literal"
        } else {
            node.kind()
        }
    });

    let mut value = None;
    if !node.is_named() || node.child_count() == 0 || typ == "string_literal" {
        value = Some(src.to_string());
    }

    let subs_rc = OwnRc::new(vec![]);
    let mut subs = subs_rc.write().unwrap();
    if cursor.goto_first_child() && typ != "string_literal" {
        subs.push(node_to_cnode(cursor.clone(), source));
        while cursor.goto_next_sibling() {
            subs.push(node_to_cnode(cursor.clone(), source));
        }
    }

    for cn in subs.iter_mut() {
        cn.head = Some(subs_rc.clone());
    }

    reshape_subs(&mut value, typ, &mut subs, source);

    CNode {
        typ,
        value,
        subs: subs_rc,
        src: node.start_byte()..node.end_byte(),
        head: None,
    }
}

fn reshape_subs(value: &mut Option<String>, typ: &str, subs: &mut Vec<CNode>, source: &str) {
    if subs.len() > 1 {
        let first = &subs[0];
        let last = &subs[subs.len() - 1];
        let first_src = &source[first.src.clone()];
        let last_src = &source[last.src.clone()];
        if (first_src, last_src) == ("(", ")")
            || (first_src, last_src) == ("[", "]")
            || (first_src, last_src) == ("{", "}")
            || (first_src, last_src) == ("<", ">")
        {
            *value = Some(format!(
                "{}{}{}",
                first_src,
                value.as_deref().unwrap_or(""),
                last_src
            ));
            subs.remove(0);
            subs.remove(subs.len() - 1);
        } else if is_empty(value)
            && first.subs.read().unwrap().is_empty()
            && !is_empty(&first.value)
            && typ.starts_with(first.value.as_deref().unwrap_or(""))
        {
            *value = subs[0].value.clone();
            subs.remove(0);
        }

        let mut new_subs = vec![];
        for s in subs.drain(..) {
            if s.value.as_deref() == Some(",")
                && (value.as_deref() == Some("()")
                    || value.as_deref() == Some("[]")
                    || value.as_deref() == Some("{}")
                    || value.as_deref() == Some("<>"))
            {
            } else {
                new_subs.push(s);
            }
        }
        *subs = new_subs;
    }

    if subs.len() == 1 && typ == "visibility_modifier" && is_empty(value) {
        *value = subs.remove(0).value;
    }
    if subs.len() > 1
        && subs[subs.len() - 1].typ.is_empty()
        && subs[subs.len() - 1].value.as_deref() == Some(";")
    {
        subs.remove(subs.len() - 1);
    }
}

fn is_empty(v: &Option<String>) -> bool {
    v.as_ref().map_or(true, |v| v.is_empty())
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        nores()
        // if let Some(label) = self.group.nodes[self.pos].name {
        //     Ok(Value::Str(label))
        // } else {
        //     nores()
        // }
    }

    fn value(&self) -> Res<Value> {
        let cnode = &self.nodes[self.pos];
        cnode
            .value
            .as_ref()
            .map_or_else(nores, |v| Ok(Value::Str(v)))
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

    fn ty(&self) -> Res<&str> {
        let r = self
            .group
            .nodes
            .read()
            .ok_or_else(|| lockerr("cannot read nodes"))?;
        Ok(r[self.pos].typ)
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            nodes: self
                .group
                .nodes
                .read()
                .ok_or_else(|| lockerr("cannot read nodes"))?,
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            nodes: self
                .group
                .nodes
                .write()
                .ok_or_else(|| lockerr("cannot write nodes"))?,
            pos: self.pos,
        })
    }

    fn sub(&self) -> Res<Group> {
        let r = self
            .group
            .nodes
            .read()
            .ok_or_else(|| lockerr("cannot read nodes"))?;
        let cnode = r
            .get(self.pos)
            .ok_or_else(|| faulterr("bad pos in rust cell"))?;
        let mut group = self.group.clone();
        group.nodes = cnode.subs.clone();
        Ok(group)
    }

    fn head(&self) -> Res<(Self, Relation)> {
        let r = self
            .group
            .nodes
            .read()
            .ok_or_else(|| lockerr("cannot read nodes"))?;
        let cnode = r
            .get(self.pos)
            .ok_or_else(|| faulterr("bad pos in rust cell"))?;
        if let Some(head) = &cnode.head {
            Ok((
                Cell {
                    group: Group {
                        domain: self.group.domain.clone(),
                        nodes: head.clone(),
                    },
                    pos: 0,
                },
                Relation::Sub,
            ))
        } else {
            nores()
        }
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
        self.nodes
            .read()
            .ok_or_else(|| lockerr("cannot read nodes"))
            .map(|v| v.len())
    }

    fn at(&self, index: usize) -> Res<Cell> {
        if index < self.len()? {
            Ok(Cell {
                group: self.clone(),
                pos: index,
            })
        } else {
            nores()
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        nores()
        // let key = key.into();
        // for (i, node) in self.nodes.iter().enumerate() {
        //     if let Some(name) = node.name {
        //         if key == name {
        //             return self.at(i);
        //         }
        //     }
        // }
        // nores()
    }
}
