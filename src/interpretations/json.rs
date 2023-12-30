use std::{fs::File, path::Path};

use serde_json::Value as SerdeValue;

use crate::utils::orc::Urc;
use crate::{
    base::*,
    utils::{orc::*, vecmap::*},
};

#[derive(Clone, Debug)]
pub struct Domain {
    preroot: Orc<Vec<Node>>,
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
                nodes: NodeGroup::Array(self.preroot.clone()),
            },
            pos: 0,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Array(Orc<Vec<Node>>),
    Object(Orc<VecMap<String, Node>>),
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Debug)]
pub struct CellReader {
    group: UrcNodeGroup,
    pos: usize,
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Domain,
    nodes: NodeGroup,
}

#[derive(Clone, Debug)]
pub enum NodeGroup {
    Array(Orc<Vec<Node>>),
    Object(Orc<VecMap<String, Node>>),
}

#[derive(Debug)]
pub enum UrcNodeGroup {
    Array(Urc<Vec<Node>>),
    Object(Urc<VecMap<String, Node>>),
}

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

impl From<serde_json::Error> for HErr {
    fn from(e: serde_json::Error) -> HErr {
        HErr::Json(format!("{}", e))
    }
}

pub fn from_path(path: &Path) -> Res<Domain> {
    let file = File::open(path)?;
    let json: SerdeValue = serde_json::from_reader(file)?;
    from_json_value(json)
}

pub fn from_string(source: &str) -> Res<Domain> {
    let json: SerdeValue = serde_json::from_str(source)?;
    from_json_value(json)
}

fn from_json_value(json: SerdeValue) -> Res<Domain> {
    let root_node = node_from_json(json);
    let preroot = Orc::new(vec![root_node]);
    Ok(Domain { preroot })
}

fn owned_value_to_node(v: OwnValue) -> Res<Node> {
    Ok(match v {
        OwnValue::None => Node::Null,
        OwnValue::Bool(b) => Node::Bool(b),
        OwnValue::Int(Int::I64(i)) => Node::I64(i),
        OwnValue::Int(Int::U64(u)) => Node::U64(u),
        OwnValue::Int(Int::I32(i)) => Node::I64(i as i64),
        OwnValue::Int(Int::U32(u)) => Node::U64(u as u64),
        OwnValue::Float(f) => Node::F64(f.0),
        OwnValue::String(s) => Node::String(s),
        OwnValue::Bytes(_) => {
            return HErr::Json("Cannot convert bytes to json field".into()).into()
        }
    })
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        if let UrcNodeGroup::Object(ref o) = self.group {
            if let Some(x) = o.at(self.pos) {
                return Ok(Value::Str(x.0));
            } else {
                return fault("bad pos");
            }
        }
        nores()
    }

    fn value(&self) -> Res<Value> {
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

        match self.group {
            UrcNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => Ok(get_value(x)),
                None => fault(""),
            },
            UrcNodeGroup::Object(ref o) => match o.at(self.pos) {
                Some(x) => Ok(get_value(x.1)),
                None => fault(""),
            },
        }
    }
}

impl CellTrait for Cell {
    type Domain = Domain;
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn domain(&self) -> Res<Domain> {
        Ok(self.group.domain.clone())
    }

    fn typ(&self) -> Res<&str> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.urc().get(self.pos) {
                Some(n) => Ok(get_typ(n)),
                None => fault(format!("bad index {}", self.pos)),
            },
            NodeGroup::Object(ref o) => match o.urc().at(self.pos) {
                Some(x) => Ok(get_typ(x.1)),
                None => fault(format!("bad index {}", self.pos)),
            },
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            group: match self.group.nodes {
                NodeGroup::Array(ref a) => UrcNodeGroup::Array(a.urc()),
                NodeGroup::Object(ref o) => UrcNodeGroup::Object(o.urc()),
            },
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }

    fn sub(&self) -> Res<Group> {
        match self.group.nodes {
            NodeGroup::Array(ref array) => match &array.urc().get(self.pos) {
                Some(Node::Array(a)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some(Node::Object(o)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                }),
                _ => nores(),
            },
            NodeGroup::Object(ref object) => match object.urc().at(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some((_, Node::Object(o))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                }),
                _ => nores(),
            },
        }
    }

    // fn set_value(&mut self, v: OwnValue) -> Res<()> {
    //     match self.group.nodes {
    //         NodeGroup::Array(ref mut ra) => {
    //             let mut urca = ra.urc();
    //             let a = guard_some!(urca.get_mut(), {
    //                 return Err(HErr::ExclusivityRequired {
    //                     path: "".into(),
    //                     op: "set_value",
    //                 });
    //             });
    //             let x = guard_some!(a.get_mut(self.pos), {
    //                 return fault("bad pos");
    //             });
    //             *x = owned_value_to_node(v)?;
    //         }

    //         NodeGroup::Object(ref mut ro) => {
    //             let mut urco = ro.urc();
    //             let o = guard_some!(urco.get_mut(), {
    //                 return Err(HErr::ExclusivityRequired {
    //                     path: "".into(),
    //                     op: "set_value",
    //                 });
    //             });
    //             let x = guard_some!(o.at_mut(self.pos), {
    //                 return fault("bad pos");
    //             });
    //             let nv = owned_value_to_node(v)?;
    //             *x.1 = nv;
    //         }
    //     };

    //     Ok(())
    // }

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

fn get_typ(node: &Node) -> &'static str {
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
                Selector::Str(k) => match o.urc().get(k) {
                    Some((pos, _, _)) => Ok(Cell {
                        group: self.clone(),
                        pos,
                    }),
                    _ => nores(),
                },
            },
        }
    }

    fn len(&self) -> usize {
        match &self.nodes {
            NodeGroup::Array(array) => array.urc().len(),
            NodeGroup::Object(o) => o.urc().len(),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(ref array) if index < array.urc().len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            NodeGroup::Object(ref o) if index < o.urc().len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
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
            Node::Array(Orc::new(na))
        }
        SerdeValue::Object(o) => {
            let mut no = VecMap::new();
            for (k, v) in o {
                no.put(k, node_from_json(v));
            }
            Node::Object(Orc::new(no))
        }
    }
}
