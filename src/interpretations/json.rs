use crate::base::common::*;
use crate::base::interpretation_api::*;
use crate::utils::vecmap::*;
use serde_json::Value as SerdeValue;
use std::{fs::File, path::Path, rc::Rc};

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub enum Node {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Array(Rc<Vec<Node>>),
    Object(Rc<VecMap<String, Node>>),
}

#[derive(Clone, Debug)]
pub enum Group {
    Array(Rc<Vec<Node>>),
    Object(Rc<VecMap<String, Node>>),
}

impl From<serde_json::Error> for HErr {
    fn from(e: serde_json::Error) -> HErr {
        HErr::Json(format!("{}", e))
    }
}

pub fn from_path(path: &Path) -> Res<Cell> {
    let file = File::open(path)?;
    let json: SerdeValue = serde_json::from_reader(file)?;
    from_json_value(json)
}

pub fn from_string(source: &str) -> Res<Cell> {
    let json: SerdeValue = serde_json::from_str(source)?;
    from_json_value(json)
}

fn from_json_value(json: SerdeValue) -> Res<Cell> {
    let root_node = node_from_json(json);
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
                None => HErr::internal(format!("bad index {}", self.pos)).into(),
            },
            Group::Object(ref o) => match o.at(self.pos) {
                Some(x) => Ok(get_typ(&x.1)),
                None => HErr::internal(format!("bad index {}", self.pos)).into(),
            },
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<&str> {
        match self.group {
            Group::Array(ref a) => NotFound::NoLabel().into(),
            Group::Object(ref o) => match o.at(self.pos) {
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
            Group::Object(ref o) => match o.at(self.pos) {
                Some(x) => Ok(get_value(&x.1)),
                None => HErr::internal("").into(),
            },
        }
    }

    fn sub(&self) -> Res<Group> {
        match self.group {
            Group::Array(ref array) => match &array.get(self.pos) {
                Some(Node::Array(a)) => Ok(Group::Array(a.clone())),
                Some(Node::Object(o)) => Ok(Group::Object(o.clone())),
                _ => NotFound::NoGroup("".into()).into(),
            },
            Group::Object(ref object) => match object.at(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group::Array(a.clone())),
                Some((_, Node::Object(o))) => Ok(Group::Object(o.clone())),
                _ => NotFound::NoGroup("".into()).into(),
            },
        }
    }

    fn attr(&self) -> Res<Group> {
        NotFound::NoGroup(format!("")).into()
    }
}

fn get_typ(node: &Node) -> &str {
    match node {
        Node::Null => "null",
        Node::Bool(_) => "bool",
        Node::I64(_) => "int",
        Node::U64(_) => "uint",
        Node::F64(_) => "float",
        Node::String(_) => "string",
        Node::Array(_) => "array",
        Node::Object(_) => "object",
    }
}

fn get_value(node: &Node) -> Value {
    match node {
        Node::Null => Value::None,
        Node::Bool(b) => Value::Bool(*b),
        Node::I64(i) => Value::Int(Int::I64(*i)),
        Node::U64(u) => Value::Int(Int::U64(*u)),
        Node::F64(f) => Value::Float(StrFloat(*f)),
        Node::String(ref s) => Value::Str(s.as_str()),
        Node::Array(_) => Value::None,
        Node::Object(_) => Value::None,
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
            Group::Object(o) => match key.into() {
                Selector::Star | Selector::DoubleStar | Selector::Top => self.at(0),
                Selector::Str(k) => match o.get(k) {
                    Some((pos, _, _)) => Ok(Cell {
                        group: self.clone(),
                        pos,
                    }),
                    _ => NotFound::NoResult(format!("")).into(),
                },
            },
        }
    }

    fn len(&self) -> usize {
        match self {
            Group::Array(array) => array.len(),
            Group::Object(o) => o.len(),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match self {
            Group::Array(array) if index < array.len() => Ok(Cell {
                group: self.clone(),
                pos: index as usize,
            }),
            Group::Object(o) if index < o.len() => Ok(Cell {
                group: self.clone(),
                pos: index as usize,
            }),
            _ => NotFound::NoResult(format!("{}", index)).into(),
        }
    }
}

fn node_from_json(sv: SerdeValue) -> Node {
    match sv {
        SerdeValue::Null => Node::Null,
        SerdeValue::Bool(b) => Node::Bool(b),
        SerdeValue::Number(n) => {
            if n.is_i64() {
                Node::I64(n.as_i64().unwrap())
            } else if n.is_u64() {
                Node::U64(n.as_u64().unwrap())
            } else {
                Node::F64(n.as_f64().unwrap())
            }
        }
        SerdeValue::String(s) => Node::String(s),
        SerdeValue::Array(a) => {
            let mut na = vec![];
            for v in a {
                na.push(node_from_json(v));
            }
            Node::Array(Rc::new(na))
        }
        SerdeValue::Object(o) => {
            let mut no = VecMap::new();
            for (k, v) in o {
                no.put(k, node_from_json(v));
            }
            Node::Object(Rc::new(no))
        }
    }
}
