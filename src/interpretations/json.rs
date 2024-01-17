use std::{fs::File, path::Path};

use indexmap::IndexMap;
use linkme::distributed_slice;
use serde_json::Value as SerdeValue;

use crate::guard_some;
use crate::utils::ownrc::UseRc;
use crate::{
    base::{Cell as XCell, *},
    utils::ownrc::*,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_JSON: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "file", "http"],
    target_interpretations: &["json"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Domain,
    nodes: NodeGroup,
    head: Option<Box<(Cell, Relation)>>,
}

#[derive(Clone, Debug)]
pub struct Domain(OwnRc<DomainData>);

#[derive(Clone, Debug)]
pub struct DomainData {
    nodes: OwnRc<Vec<Node>>,
    write_policy: WritePolicy,
    origin: Option<XCell>,
}

#[derive(Clone, Debug)]
pub enum NodeGroup {
    Array(OwnRc<Vec<Node>>),
    Object(OwnRc<IndexMap<String, Node>>),
}

#[derive(Clone, Debug)]
pub enum Node {
    Scalar(OwnValue),
    Array(OwnRc<Vec<Node>>),
    Object(OwnRc<IndexMap<String, Node>>),
}

#[derive(Debug)]
pub struct CellReader {
    nodes: UrcNodeGroup,
    pos: usize,
}

#[derive(Debug)]
pub struct CellWriter {
    nodes: UrcNodeGroup,
    pos: usize,
}

#[derive(Debug)]
pub enum UrcNodeGroup {
    Array(UseRc<Vec<Node>>),
    Object(UseRc<IndexMap<String, Node>>),
}

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "json"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell {
            group: Group {
                domain: self.clone(),
                nodes: NodeGroup::Array(self.0.tap().nodes.clone()),
                head: None,
            },
            pos: 0,
        })
    }

    fn origin(&self) -> Res<XCell> {
        match &self.0.tap().origin {
            Some(c) => Ok(c.clone()),
            None => nores(),
        }
    }
}
impl SaveTrait for Domain {
    fn save(&self, target: SaveTarget) -> Res<()> {
        let s = self.root()?.serialize()?;
        match target {
            SaveTarget::Origin => match self.0.tap().origin {
                Some(ref origin) => origin.write().set_value(OwnValue::String(s))?,
                None => return userr("no origin, cannot save"),
            },
            SaveTarget::Cell(ref cell) => cell.write().set_value(OwnValue::String(s))?,
        };
        Ok(())
    }
    // TODO: add rest of implementation
}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        let serde_value = match cell.domain().interpretation() {
            "value" => {
                let s = cell.read().value()?.to_string();
                serde_json::from_str(s.as_ref())?
            }
            "file" => {
                let path = cell.as_file_path()?;
                serde_json::from_reader(
                    File::open(path).map_err(|e| caused(HErrKind::IO, "cannot read json", e))?,
                )?
            }
            "http" => {
                let s = cell.read().value()?.to_string();
                serde_json::from_str(s.as_ref())?
            }
            _ => return nores(),
        };
        Self::from_serde_value(serde_value, Some(cell))
    }

    pub fn from_string(s: impl AsRef<str>) -> Res<XCell> {
        let json: SerdeValue = serde_json::from_str(s.as_ref())?;
        Self::from_serde_value(json, None)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Res<XCell> {
        let file = File::open(path).map_err(|e| caused(HErrKind::IO, "cannot read json", e))?;
        let json: SerdeValue = serde_json::from_reader(file)?;
        Self::from_serde_value(json, None)
    }

    pub fn from_serde_value(json: SerdeValue, origin: Option<XCell>) -> Res<XCell> {
        let nodes = OwnRc::new(vec![node_from_json(json)]);
        let domain = Domain(OwnRc::new(DomainData {
            nodes,
            write_policy: WritePolicy::ReadOnly,
            origin,
        }));
        Ok(XCell {
            dyn_cell: DynCell::from(domain.root()?),
        })
    }

    pub fn serialize(&self) -> Res<String> {
        let serde_value = match self.group.nodes {
            NodeGroup::Array(ref a) => match a.tap().get(self.pos) {
                Some(node) => node_to_serde(node),
                None => return fault(format!("bad index {}", self.pos)),
            },
            NodeGroup::Object(ref o) => match o.tap().get_index(self.pos) {
                Some(x) => node_to_serde(x.1),
                None => return fault(format!("bad index {}", self.pos)),
            },
        };
        Ok(serde_json::to_string_pretty(&serde_value)?)
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
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.tap().get(self.pos) {
                Some(n) => Ok(get_ty(n)),
                None => fault(format!("bad index {}", self.pos)),
            },
            NodeGroup::Object(ref o) => match o.tap().get_index(self.pos) {
                Some(x) => Ok(get_ty(x.1)),
                None => fault(format!("bad index {}", self.pos)),
            },
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => UrcNodeGroup::Array(a.tap()),
                NodeGroup::Object(ref o) => UrcNodeGroup::Object(o.tap()),
            },
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => UrcNodeGroup::Array(a.tap()),
                NodeGroup::Object(ref o) => UrcNodeGroup::Object(o.tap()),
            },
            pos: self.pos,
        })
    }

    fn sub(&self) -> Res<Group> {
        match self.group.nodes {
            NodeGroup::Array(ref array) => match &array.tap().get(self.pos) {
                Some(Node::Array(a)) => Ok(Group {
                    head: Some(Box::new((self.clone(), Relation::Sub))),
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some(Node::Object(o)) => Ok(Group {
                    head: Some(Box::new((self.clone(), Relation::Sub))),
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                }),
                _ => nores(),
            },
            NodeGroup::Object(ref object) => match object.tap().get_index(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group {
                    head: Some(Box::new((self.clone(), Relation::Sub))),
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some((_, Node::Object(o))) => Ok(Group {
                    head: Some(Box::new((self.clone(), Relation::Sub))),
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                }),
                _ => nores(),
            },
        }
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.group.head {
            Some(ref head) => Ok((head.0.clone(), head.1)),
            None => nores(),
        }
    }
}

impl From<serde_json::Error> for HErr {
    fn from(e: serde_json::Error) -> HErr {
        caused(HErrKind::InvalidFormat, "cannot read json", e)
    }
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        if let UrcNodeGroup::Object(ref o) = self.nodes {
            if let Some(x) = o.get_index(self.pos) {
                return Ok(Value::Str(x.0));
            } else {
                return fault("bad pos");
            }
        }
        nores()
    }

    fn value(&self) -> Res<Value> {
        fn get_value(node: &Node) -> Res<Value> {
            match node {
                Node::Scalar(v) => Ok(v.as_value()),
                _ => nores(),
            }
        }

        match self.nodes {
            UrcNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => get_value(x),
                None => fault(""),
            },
            UrcNodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => get_value(x.1),
                None => fault(""),
            },
        }
    }
}

impl CellWriterTrait for CellWriter {
    // TODO: support write policies
    // if self.domain.write_policy == WritePolicy::ReadOnly {
    //     return Err(HErr::ReadOnly);
    // }
    fn set_label(&mut self, label: OwnValue) -> Res<()> {
        match self.nodes {
            UrcNodeGroup::Array(_) => {
                return userr("cannot set label on array object");
            }
            UrcNodeGroup::Object(ref mut o) => {
                let (_, v) = guard_some!(o.swap_remove_index(self.pos), {
                    return fault("bad pos");
                });
                let (new_index, _) = o.insert_full(label.to_string(), v);
                o.swap_indices(self.pos, new_index)
            }
        };
        Ok(())
    }
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.nodes {
            UrcNodeGroup::Array(ref mut a) => {
                a[self.pos] = Node::Scalar(value);
            }
            UrcNodeGroup::Object(ref mut o) => {
                let (_, node) = o
                    .get_index_mut(self.pos)
                    .ok_or_else(|| faulterr("bad pos"))?;
                *node = Node::Scalar(value);
            }
        };
        Ok(())
    }

    // fn delete(&mut self) -> Res<()> {
    //     match self.group.nodes {
    //         NodeGroup::Array(ref mut a) => {
    //             let mut urca = a.urc();
    //             let v = guard_some!(urca.get_mut(), {
    //                 return Err(HErr::ExclusivityRequired {
    //                     path: "".into(),
    //                     op: "delete",
    //                 });
    //             });
    //             v.remove(self.pos);
    //         }
    //         NodeGroup::Object(ref mut o) => {
    //             let mut urco = o.urc();
    //             let v = guard_some!(urco.get_mut(), {
    //                 return Err(HErr::ExclusivityRequired {
    //                     path: "".into(),
    //                     op: "delete",
    //                 });
    //             });
    //             v.remove(self.pos);
    //         }
    //     };
    //     Ok(())
    // }
}

fn get_ty(node: &Node) -> &'static str {
    match node {
        Node::Scalar(OwnValue::None) => "null",
        Node::Scalar(OwnValue::Bool(_)) => "bool",
        Node::Scalar(OwnValue::Int(_)) => "number",
        Node::Scalar(OwnValue::Float(_)) => "number",
        Node::Scalar(OwnValue::String(_)) => "string",
        Node::Scalar(OwnValue::Bytes(_)) => "string",
        Node::Array(_) => "array",
        Node::Object(_) => "object",
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

    fn get<'s, S: Into<Selector<'s>>>(&self, key: S) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(a) => nores(),
            NodeGroup::Object(o) => match key.into() {
                Selector::Star | Selector::DoubleStar | Selector::Top => self.at(0),
                Selector::Str(k) => match o.tap().get_index_of(k) {
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
            NodeGroup::Array(array) => array.tap().len(),
            NodeGroup::Object(o) => o.tap().len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(ref array) if index < array.tap().len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            NodeGroup::Object(ref o) if index < o.tap().len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
        }
    }
}

fn node_from_json(sv: SerdeValue) -> Node {
    match sv {
        SerdeValue::Null => Node::Scalar(OwnValue::None),
        SerdeValue::Bool(b) => Node::Scalar(OwnValue::Bool(b)),
        SerdeValue::Number(n) => {
            if n.is_i64() {
                Node::Scalar(Value::from(n.as_i64().unwrap()).to_owned_value())
            } else if n.is_u64() {
                Node::Scalar(Value::from(n.as_u64().unwrap()).to_owned_value())
            } else {
                Node::Scalar(Value::from(n.as_f64().unwrap()).to_owned_value())
            }
        }
        SerdeValue::String(s) => Node::Scalar(OwnValue::String(s)),
        SerdeValue::Array(a) => {
            let mut na = vec![];
            for v in a {
                na.push(node_from_json(v));
            }
            Node::Array(OwnRc::new(na))
        }
        SerdeValue::Object(o) => {
            let mut no = IndexMap::new();
            for (k, v) in o {
                no.insert(k, node_from_json(v));
            }
            Node::Object(OwnRc::new(no))
        }
    }
}

fn node_to_serde(node: &Node) -> SerdeValue {
    match node {
        Node::Scalar(OwnValue::None) => SerdeValue::Null,
        Node::Scalar(OwnValue::Bool(b)) => SerdeValue::Bool(*b),
        Node::Scalar(OwnValue::Int(Int::I32(i))) => SerdeValue::Number((*i).into()),
        Node::Scalar(OwnValue::Int(Int::U32(i))) => SerdeValue::Number((*i).into()),
        Node::Scalar(OwnValue::Int(Int::I64(i))) => SerdeValue::Number((*i).into()),
        Node::Scalar(OwnValue::Int(Int::U64(i))) => SerdeValue::Number((*i).into()),
        Node::Scalar(OwnValue::Float(StrFloat(f))) => SerdeValue::Number(
            serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0)),
        ),
        Node::Scalar(OwnValue::String(s)) => SerdeValue::String(s.clone()),
        Node::Scalar(OwnValue::Bytes(b)) => SerdeValue::String(String::from_utf8_lossy(b).into()),
        Node::Array(a) => {
            let mut na = vec![];
            for v in &*a.tap() {
                na.push(node_to_serde(v));
            }
            SerdeValue::Array(na)
        }
        Node::Object(o) => {
            let mut no = serde_json::map::Map::new();
            for (k, v) in o.tap().as_slice() {
                no.insert(k.clone(), node_to_serde(v));
            }
            SerdeValue::Object(no)
        }
    }
}
