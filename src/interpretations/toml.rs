use std::{cell::OnceCell, rc::Rc};

use indexmap::IndexMap;
use linkme::distributed_slice;
use nom::AsBytes;
use {toml, toml::Value as TomlValue};

use crate::{
    api::{interpretation::*, Cell as XCell, *},
    utils::ownrc::{OwnRc, ReadRc, WriteRc},
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_TOML: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["toml"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Debug)]
pub(crate) struct CellReader {
    nodes: ReadNodeGroup,
    pos: usize,
    value: OnceCell<String>,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    nodes: WriteNodeGroup,
    pos: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    nodes: NodeGroup,
    head: Option<Rc<(Cell, Relation)>>,
}

#[derive(Clone, Debug)]
pub(crate) enum NodeGroup {
    Array(OwnRc<Vec<Node>>),
    Table(OwnRc<IndexMap<String, Node>>),
}

#[derive(Debug)]
pub(crate) enum ReadNodeGroup {
    Array(ReadRc<Vec<Node>>),
    Table(ReadRc<IndexMap<String, Node>>),
}

#[derive(Debug)]
pub(crate) enum WriteNodeGroup {
    Array(WriteRc<Vec<Node>>),
    Table(WriteRc<IndexMap<String, Node>>),
}

#[derive(Clone, Debug)]
pub(crate) enum Node {
    Scalar(TomlValue),
    Array(OwnRc<Vec<Node>>),
    Table(OwnRc<IndexMap<String, Node>>),
}

impl From<toml::de::Error> for HErr {
    fn from(e: toml::de::Error) -> HErr {
        caused(HErrKind::InvalidFormat, "bad toml", e)
    }
}

impl Cell {
    pub(crate) fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Self::make_cell(value, Some(cell))
            }
            "fs" => {
                let r = cell.read();
                let path = r.as_file_path()?;
                Self::make_cell(
                    &std::fs::read_to_string(path).map_err(|e| {
                        caused(HErrKind::IO, format!("cannot read file: {:?}", path), e)
                    })?,
                    Some(cell),
                )
            }
            _ => nores(),
        }
    }

    fn make_cell(source: &str, origin: Option<XCell>) -> Res<XCell> {
        let toml: TomlValue = toml::from_str(source)?;
        let root_node = node_from_toml(toml);
        let preroot = OwnRc::new(vec![root_node]);
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
    fn ty(&self) -> Res<&str> {
        match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_ty(n)),
                None => fault(""),
            },
            ReadNodeGroup::Table(ref t) => match t.get_index(self.pos) {
                Some(x) => Ok(get_ty(x.1)),
                None => fault(""),
            },
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        match self.nodes {
            ReadNodeGroup::Array(ref a) => nores(),
            ReadNodeGroup::Table(ref t) => match t.get_index(self.pos) {
                Some(x) => Ok(Value::Str(x.0)),
                None => fault(""),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => to_value(x, &self.value),
                None => fault(""),
            },
            ReadNodeGroup::Table(ref t) => match t.get_index(self.pos) {
                Some(x) => to_value(x.1, &self.value),
                None => fault(""),
            },
        }
    }

    fn serial(&self) -> Res<String> {
        let tv = match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => node_to_toml(x),
                None => fault(""),
            },
            ReadNodeGroup::Table(ref t) => match t.get_index(self.pos) {
                Some(x) => {
                    let mut table = toml::value::Table::new();
                    table.insert(x.0.clone(), node_to_toml(x.1)?);
                    Ok(TomlValue::Table(table))
                }
                None => fault(""),
            },
        }?;
        toml::to_string_pretty(&tv).map_err(|e| caused(HErrKind::InvalidFormat, "bad toml", e))
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.nodes {
            WriteNodeGroup::Array(ref mut a) => {
                a[self.pos] = Node::Scalar(to_toml(value)?);
            }
            WriteNodeGroup::Table(ref mut o) => {
                let (_, node) = o
                    .get_index_mut(self.pos)
                    .ok_or_else(|| faulterr("bad pos"))?;
                *node = Node::Scalar(to_toml(value)?);
            }
        };
        Ok(())
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "toml"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => {
                    ReadNodeGroup::Array(a.read().ok_or_else(|| lockerr("cannot read group"))?)
                }
                NodeGroup::Table(ref t) => {
                    ReadNodeGroup::Table(t.read().ok_or_else(|| lockerr("cannot read group"))?)
                }
            },
            pos: self.pos,
            value: OnceCell::new(),
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => {
                    WriteNodeGroup::Array(a.write().ok_or_else(|| lockerr("cannot write group"))?)
                }
                NodeGroup::Table(ref t) => {
                    WriteNodeGroup::Table(t.write().ok_or_else(|| lockerr("cannot write group"))?)
                }
            },
            pos: self.pos,
        })
    }

    fn sub(&self) -> Res<Group> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => {
                let a = a.read().ok_or_else(|| lockerr("cannot read group"))?;
                match &a.get(self.pos) {
                    Some(Node::Array(a)) => Ok(Group {
                        nodes: NodeGroup::Array(a.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    Some(Node::Table(o)) => Ok(Group {
                        nodes: NodeGroup::Table(o.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    _ => nores(),
                }
            }
            NodeGroup::Table(ref t) => {
                let t = t.read().ok_or_else(|| lockerr("cannot read group"))?;
                match t.get_index(self.pos) {
                    Some((_, Node::Array(a))) => Ok(Group {
                        nodes: NodeGroup::Array(a.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    Some((_, Node::Table(o))) => Ok(Group {
                        nodes: NodeGroup::Table(o.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    _ => nores(),
                }
            }
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

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Once<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(match &self.nodes {
            NodeGroup::Array(a) => a.read().ok_or_else(|| lockerr("cannot read group"))?.len(),
            NodeGroup::Table(t) => t.read().ok_or_else(|| lockerr("cannot read group"))?.len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        let len = self.len()?;
        match &self.nodes {
            NodeGroup::Array(array) if index < len => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            NodeGroup::Table(t) if index < len => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
        }
    }

    fn get_all(&self, key: Value) -> Res<Self::CellIterator> {
        let cell = match &self.nodes {
            NodeGroup::Array(a) => nores(),
            NodeGroup::Table(t) => match key {
                Value::Str(k) => {
                    let t = t.read().ok_or_else(|| lockerr("cannot read group"))?;
                    match t.get_index_of(k) {
                        Some(pos) => Ok(Cell {
                            group: self.clone(),
                            pos,
                        }),
                        _ => nores(),
                    }
                }
                _ => nores(),
            },
        };
        Ok(std::iter::once(cell))
    }
}

fn get_ty(node: &Node) -> &'static str {
    match node {
        Node::Scalar(TomlValue::Boolean(_)) => "bool",
        Node::Scalar(TomlValue::Datetime(_)) => "datetime",
        Node::Scalar(TomlValue::Float(_)) => "float",
        Node::Scalar(TomlValue::Integer(_)) => "int",
        Node::Scalar(TomlValue::String(_)) => "string",
        Node::Scalar(_) => "", // should not happen
        Node::Array(_) => "array",
        Node::Table(_) => "table",
    }
}

fn to_value<'a>(node: &'a Node, cache: &'a OnceCell<String>) -> Res<Value<'a>> {
    match node {
        Node::Scalar(TomlValue::Boolean(b)) => Ok(Value::Bool(*b)),
        Node::Scalar(TomlValue::Datetime(d)) => Ok(Value::Str(cache.get_or_init(|| d.to_string()))),
        Node::Scalar(TomlValue::Float(f)) => Ok(Value::Float(StrFloat(*f))),
        Node::Scalar(TomlValue::Integer(i)) => Ok(Value::Int(Int::I64(*i))),
        Node::Scalar(TomlValue::String(s)) => Ok(Value::Str(s.as_str())),
        _ => nores(),
    }
}

fn to_toml(value: OwnValue) -> Res<TomlValue> {
    match value {
        OwnValue::None => Ok(TomlValue::String("".to_string())),
        OwnValue::Bool(b) => Ok(TomlValue::Boolean(b)),
        OwnValue::Float(StrFloat(f)) => Ok(TomlValue::Float(f)),
        OwnValue::Int(Int::I64(i)) => Ok(TomlValue::Integer(i)),
        OwnValue::Int(Int::I32(i)) => Ok(TomlValue::Integer(i as i64)),
        OwnValue::Int(Int::U64(i)) => Ok(TomlValue::Integer(i as i64)),
        OwnValue::Int(Int::U32(i)) => Ok(TomlValue::Integer(i as i64)),
        OwnValue::String(s) => Ok(TomlValue::String(s)),
        OwnValue::Bytes(s) => Ok(TomlValue::String(
            String::from_utf8_lossy(s.as_bytes()).into(),
        )),
    }
}

fn node_from_toml(tv: TomlValue) -> Node {
    match tv {
        TomlValue::Array(a) => {
            let mut na = vec![];
            for v in a {
                na.push(node_from_toml(v));
            }
            Node::Array(OwnRc::new(na))
        }
        TomlValue::Table(t) => {
            let mut nt = IndexMap::new();
            for (k, v) in t {
                nt.insert(k, node_from_toml(v));
            }
            Node::Table(OwnRc::new(nt))
        }
        _ => Node::Scalar(tv),
    }
}

fn node_to_toml(node: &Node) -> Res<TomlValue> {
    Ok(match node {
        Node::Scalar(tv) => tv.clone(),
        Node::Array(a) => {
            let mut na = toml::value::Array::new();
            for v in &*a.write().ok_or_else(|| lockerr("cannot write nodes"))? {
                na.push(node_to_toml(v)?);
            }
            TomlValue::Array(na)
        }
        Node::Table(t) => {
            let mut nt = toml::value::Table::new();
            for (k, v) in t
                .write()
                .ok_or_else(|| lockerr("cannot write nodes"))?
                .as_slice()
            {
                nt.insert(k.clone(), node_to_toml(v)?);
            }
            TomlValue::Table(nt)
        }
    })
}
