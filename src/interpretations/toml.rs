use crate::{base::*, utils::vecmap::*};
use std::{path::Path, rc::Rc};
use toml;
use toml::Value as TomlValue;

#[derive(Clone, Debug)]
pub struct Domain {
    preroot: NodeGroup,
}

impl InDomain for Domain {
    type Cell = Cell;
    type Group = Group;

    fn root(self: &Rc<Self>) -> Res<Self::Cell> {
        Ok(Cell {
            group: Group {
                domain: self.clone(),
                nodes: self.preroot.clone(),
            },
            pos: 0,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Rc<Domain>,
    nodes: NodeGroup,
}

#[derive(Clone, Debug)]
pub enum NodeGroup {
    Array(Rc<Vec<Node>>),
    Table(Rc<VecMap<String, Node>>),
}
#[derive(Clone, Debug)]
pub enum Node {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Datetime(String),
    Array(Rc<Vec<Node>>),
    Table(Rc<VecMap<String, Node>>),
}

impl From<toml::de::Error> for HErr {
    fn from(e: toml::de::Error) -> HErr {
        HErr::Toml(format!("{}", e))
    }
}

pub fn from_path(path: &Path) -> Res<Cell> {
    let source = std::fs::read_to_string(&path)?;
    from_string(&source)
}

pub fn from_string(source: &str) -> Res<Cell> {
    let toml: TomlValue = toml::from_str(source)?;
    let root_node = node_from_toml(toml);
    let preroot = Rc::new(vec![root_node]);
    let domain = Rc::new(Domain {
        preroot: NodeGroup::Array(preroot),
    });
    domain.root()
}

impl InCell for Cell {
    type Domain = Domain;

    fn domain(&self) -> &Rc<Self::Domain> {
        &self.group.domain
    }

    fn typ(&self) -> Res<&str> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_typ(n)),
                None => HErr::internal("").into(),
            },
            NodeGroup::Table(ref t) => match t.at(self.pos) {
                Some(x) => Ok(get_typ(&x.1)),
                None => HErr::internal("").into(),
            },
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<&str> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => NotFound::NoLabel().into(),
            NodeGroup::Table(ref t) => match t.at(self.pos) {
                Some(x) => Ok(x.0),
                None => HErr::internal("").into(),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => Ok(get_value(x)),
                None => HErr::internal("").into(),
            },
            NodeGroup::Table(ref t) => match t.at(self.pos) {
                Some(x) => Ok(get_value(&x.1)),
                None => HErr::internal("").into(),
            },
        }
    }

    fn sub(&self) -> Res<Group> {
        match self.group.nodes {
            NodeGroup::Array(ref array) => match &array.get(self.pos) {
                Some(Node::Array(a)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some(Node::Table(o)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Table(o.clone()),
                }),
                _ => HErr::internal("").into(),
            },
            NodeGroup::Table(ref table) => match table.at(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some((_, Node::Table(o))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Table(o.clone()),
                }),
                _ => HErr::internal("").into(),
            },
        }
    }

    fn attr(&self) -> Res<Group> {
        NotFound::NoGroup(format!("")).into()
    }
}

fn get_typ(node: &Node) -> &str {
    match node {
        Node::Bool(_) => "bool",
        Node::I64(_) => "int",
        Node::F64(_) => "float",
        Node::String(_) => "string",
        Node::Datetime(_) => "datetime",
        Node::Array(_) => "array",
        Node::Table(_) => "table",
    }
}

fn get_value(node: &Node) -> Value {
    match node {
        Node::Bool(b) => Value::Bool(*b),
        Node::I64(i) => Value::Int(Int::I64(*i)),
        Node::F64(f) => Value::Float(StrFloat(*f)),
        Node::String(ref s) => Value::Str(s.as_str()),
        Node::Datetime(ref d) => Value::Str(d.as_str()),
        Node::Array(_) => Value::None,
        Node::Table(_) => Value::None,
    }
}

impl InGroup for Group {
    type Domain = Domain;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(a) => NotFound::NoLabel().into(),
            NodeGroup::Table(t) => match key.into() {
                Selector::Star | Selector::DoubleStar | Selector::Top => self.at(0),
                Selector::Str(k) => match t.get(k) {
                    Some((pos, _, _)) => Ok(Cell {
                        group: self.clone(),
                        pos,
                    }),
                    _ => NotFound::NoResult(format!("{}", k)).into(),
                },
            },
        }
    }

    fn len(&self) -> usize {
        match &self.nodes {
            NodeGroup::Array(array) => array.len(),
            NodeGroup::Table(t) => t.len(),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(array) if index < array.len() => Ok(Cell {
                group: self.clone(),
                pos: index as usize,
            }),
            NodeGroup::Table(t) if index < t.len() => Ok(Cell {
                group: self.clone(),
                pos: index as usize,
            }),
            _ => NotFound::NoResult(format!("{}", index)).into(),
        }
    }
}

fn node_from_toml(tv: TomlValue) -> Node {
    match tv {
        TomlValue::Boolean(b) => Node::Bool(b),
        TomlValue::Integer(n) => Node::I64(n),
        TomlValue::Float(f) => Node::F64(f),
        TomlValue::String(s) => Node::String(s),
        TomlValue::Datetime(d) => Node::String(d.to_string()),
        TomlValue::Array(a) => {
            let mut na = vec![];
            for v in a {
                na.push(node_from_toml(v));
            }
            Node::Array(Rc::new(na))
        }
        TomlValue::Table(t) => {
            let mut nt = VecMap::new();
            for (k, v) in t {
                nt.put(k, node_from_toml(v));
            }
            Node::Table(Rc::new(nt))
        }
    }
}
