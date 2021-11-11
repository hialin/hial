use crate::{
    base::{common::*, in_api::*},
    tree_sitter_language, *,
};
use std::{ops::Range, path::Path, rc::Rc};
use tree_sitter::{Parser, Tree, TreeCursor};

#[derive(Clone, Debug)]
pub struct Domain {
    language: &'static str,
    source: String,
    tree: Tree,
}

impl InDomain for Domain {
    type Cell = Cell;
    type Group = Group;

    fn root(self: &Rc<Self>) -> Res<Self::Cell> {
        let cnode = node_to_cnode(self.tree.walk(), &self.source);

        let group = Group {
            domain: self.clone(),
            nodes: Rc::new(vec![cnode]),
        };
        Ok(Cell { group, pos: 0 })
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Rc<Domain>,
    // since the tree is in a Rc, the treecursor is valid on self's lifetime
    nodes: Rc<Vec<CNode>>,
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub struct CNode {
    typ: &'static str,
    name: Option<&'static str>,
    value: String,
    subs: Rc<Vec<CNode>>,
    src: Range<usize>,
}

pub fn from_path(path: &Path, language: &'static str) -> Res<Cell> {
    let source = std::fs::read_to_string(&path)?;
    sitter_from_source(source, language)
}

pub fn from_string(source: String, language: &'static str) -> Res<Cell> {
    sitter_from_source(source, language)
}

pub fn get_underlying_string(cell: &Cell) -> Res<&str> {
    let cnode = guard_some!(cell.group.nodes.get(cell.pos), {
        return HErr::internal("bad pos in rust cell").into();
    });
    Ok(&cell.group.domain.source[cnode.src.clone()])
}

fn sitter_from_source(source: String, language: &'static str) -> Res<Cell> {
    let sitter_language = guard_some!(tree_sitter_language(language), {
        return Err(HErr::Sitter(format!("unsupported language: {}", language)));
    });

    // println!("sitter language: {}", language);
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
        return Err(HErr::Sitter(format!("cannot get parse tree")));
    });
    let domain = Rc::new(Domain {
        language,
        source,
        tree,
    });
    domain.root()
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

impl InCell for Cell {
    type Domain = Domain;

    fn domain(&self) -> &Rc<Self::Domain> {
        &self.group.domain
    }

    fn typ(&self) -> Res<&str> {
        Ok(self.group.nodes[self.pos].typ)
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<&str> {
        if let Some(label) = self.group.nodes[self.pos].name {
            Ok(label)
        } else {
            NotFound::NoLabel().into()
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

    fn sub(&self) -> Res<Group> {
        let cnode = &self.group.nodes[self.pos];
        let mut group = self.group.clone();
        group.nodes = cnode.subs.clone();
        Ok(group)
    }

    fn attr(&self) -> Res<Group> {
        NotFound::NoGroup(format!("@")).into()
    }
}

impl InGroup for Group {
    type Domain = Domain;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: false,
        }
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }

    fn at(&self, index: usize) -> Res<Cell> {
        if index < self.nodes.len() {
            Ok(Cell {
                group: self.clone(),
                pos: index,
            })
        } else {
            NotFound::NoResult(format!("{}", index)).into()
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
        NotFound::NoResult(format!("")).into()
    }
}

impl Cell {
    pub fn language(&self) -> &'static str {
        self.group.domain.language
    }
}
