use std::rc::Rc;

use indexmap::IndexMap;
use linkme::distributed_slice;
use {toml, toml::Value as TomlValue};

use crate::base::{Cell as XCell, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_TOML: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["toml"],
    constructor: Cell::from_cell,
};

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
pub struct Group {
    nodes: NodeGroup,
    head: Option<Rc<(Cell, Relation)>>,
}

#[derive(Clone, Debug)]
pub enum NodeGroup {
    Array(Rc<Vec<Node>>),
    Table(Rc<IndexMap<String, Node>>),
}
#[derive(Clone, Debug)]
pub enum Node {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Datetime(String),
    Array(Rc<Vec<Node>>),
    Table(Rc<IndexMap<String, Node>>),
}

impl From<toml::de::Error> for HErr {
    fn from(e: toml::de::Error) -> HErr {
        caused(HErrKind::InvalidFormat, "bad toml", e)
    }
}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Self::make_cell(value, Some(cell))
            }
            "fs" => {
                let path = cell.as_file_path()?;
                Self::make_cell(
                    &std::fs::read_to_string(path)
                        .map_err(|e| caused(HErrKind::IO, "cannot read file", e))?,
                    Some(cell),
                )
            }
            _ => nores(),
        }
    }

    fn make_cell(source: &str, origin: Option<XCell>) -> Res<XCell> {
        let toml: TomlValue = toml::from_str(source)?;
        let root_node = node_from_toml(toml);
        let preroot = Rc::new(vec![root_node]);
        let toml_cell = Cell {
            group: Group {
                nodes: NodeGroup::Array(preroot),
                head: None,
            },
            pos: 0,
        };
        Ok(new_cell(DynCell::from(toml_cell), origin))
    }
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => nores(),
            NodeGroup::Table(ref t) => match t.get_index(self.pos) {
                Some(x) => Ok(Value::Str(x.0)),
                None => fault(""),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => get_value(x),
                None => fault(""),
            },
            NodeGroup::Table(ref t) => match t.get_index(self.pos) {
                Some(x) => get_value(x.1),
                None => fault(""),
            },
        }
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "toml"
    }

    fn ty(&self) -> Res<&str> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_ty(n)),
                None => fault(""),
            },
            NodeGroup::Table(ref t) => match t.get_index(self.pos) {
                Some(x) => Ok(get_ty(x.1)),
                None => fault(""),
            },
        }
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
        match self.group.nodes {
            NodeGroup::Array(ref array) => match &array.get(self.pos) {
                Some(Node::Array(a)) => Ok(Group {
                    nodes: NodeGroup::Array(a.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                Some(Node::Table(o)) => Ok(Group {
                    nodes: NodeGroup::Table(o.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                _ => nores(),
            },
            NodeGroup::Table(ref table) => match table.get_index(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group {
                    nodes: NodeGroup::Array(a.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                Some((_, Node::Table(o))) => Ok(Group {
                    nodes: NodeGroup::Table(o.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                _ => nores(),
            },
        }
    }

    fn attr(&self) -> Res<Group> {
        nores()
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match &self.group.head {
            Some(h) => Ok((h.0.clone(), h.1)),
            None => nores(),
        }
    }
}

fn get_ty(node: &Node) -> &str {
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

fn get_value(node: &Node) -> Res<Value> {
    match node {
        Node::Bool(b) => Ok(Value::Bool(*b)),
        Node::I64(i) => Ok(Value::Int(Int::I64(*i))),
        Node::F64(f) => Ok(Value::Float(StrFloat(*f))),
        Node::String(ref s) => Ok(Value::Str(s.as_str())),
        Node::Datetime(ref d) => Ok(Value::Str(d.as_str())),
        _ => nores(),
    }
}

impl GroupTrait for Group {
    type Cell = Cell;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(a) => nores(),
            NodeGroup::Table(t) => match key.into() {
                Selector::Star | Selector::DoubleStar | Selector::Top => self.at(0),
                Selector::Str(k) => match t.get_index_of(k) {
                    Some(pos) => Ok(Cell {
                        group: self.clone(),
                        pos,
                    }),
                    _ => nores(),
                },
            },
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(match &self.nodes {
            NodeGroup::Array(array) => array.len(),
            NodeGroup::Table(t) => t.len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(array) if index < array.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            NodeGroup::Table(t) if index < t.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
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
            let mut nt = IndexMap::new();
            for (k, v) in t {
                nt.insert(k, node_from_toml(v));
            }
            Node::Table(Rc::new(nt))
        }
    }
}
