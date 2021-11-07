use crate::base::common::*;
use crate::base::interpretation_api::*;
use crate::utils::vecmap::*;
use std::{path::Path, rc::Rc};
use toml;
use toml::Value as TomlValue;

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
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

#[derive(Clone, Debug)]
pub enum Group {
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
    let root_group = Rc::new(vec![root_node]);
    Ok(Cell {
        group: Group::Array(root_group),
        pos: 0,
    })
}

impl InterpretationCell for Cell {
    type Group = Group;

    fn typ(&self) -> Res<&str> {
        match self.group {
            Group::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_typ(n)),
                None => HErr::internal("").into(),
            },
            Group::Table(ref t) => match t.at(self.pos) {
                Some(x) => Ok(get_typ(&x.1)),
                None => HErr::internal("").into(),
            },
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<&str> {
        match self.group {
            Group::Array(ref a) => NotFound::NoLabel().into(),
            Group::Table(ref t) => match t.at(self.pos) {
                Some(x) => Ok(x.0),
                None => HErr::internal("").into(),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self.group {
            Group::Array(ref a) => match a.get(self.pos) {
                Some(x) => Ok(get_value(x)),
                None => HErr::internal("").into(),
            },
            Group::Table(ref t) => match t.at(self.pos) {
                Some(x) => Ok(get_value(&x.1)),
                None => HErr::internal("").into(),
            },
        }
    }

    fn sub(&self) -> Res<Group> {
        match self.group {
            Group::Array(ref array) => match &array.get(self.pos) {
                Some(Node::Array(a)) => Ok(Group::Array(a.clone())),
                Some(Node::Table(o)) => Ok(Group::Table(o.clone())),
                _ => HErr::internal("").into(),
            },
            Group::Table(ref table) => match table.at(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group::Array(a.clone())),
                Some((_, Node::Table(o))) => Ok(Group::Table(o.clone())),
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

impl InterpretationGroup for Group {
    type Cell = Cell;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        match self {
            Group::Array(a) => NotFound::NoLabel().into(),
            Group::Table(t) => match key.into() {
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
        match self {
            Group::Array(array) => array.len(),
            Group::Table(t) => t.len(),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match self {
            Group::Array(array) if index < array.len() => Ok(Cell {
                group: self.clone(),
                pos: index as usize,
            }),
            Group::Table(t) if index < t.len() => Ok(Cell {
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
