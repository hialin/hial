use std::{fs::File, path::Path};

use indexmap::IndexMap;
use linkme::distributed_slice;
use serde_json::Value as SerdeValue;

use crate::guard_some;
use crate::utils::orc::Urc;
use crate::{
    base::{Cell as XCell, *},
    utils::orc::*,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_JSON: ElevationConstructor = ElevationConstructor {
    source_interpretation: "value",
    target_interpretation: "json",
    constructor: Cell::from_value_cell,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static FILE_TO_JSON: ElevationConstructor = ElevationConstructor {
    source_interpretation: "file",
    target_interpretation: "json",
    constructor: Cell::from_file_cell,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static HTTP_TO_JSON: ElevationConstructor = ElevationConstructor {
    source_interpretation: "http",
    target_interpretation: "json",
    constructor: Cell::from_http_cell,
};

#[derive(Clone, Debug)]
pub struct Domain {
    nodes: Orc<Vec<Node>>,
    write_policy: WritePolicy,
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
                nodes: NodeGroup::Array(self.nodes.clone()),
            },
            pos: 0,
        })
    }
}

#[derive(Clone, Debug)]
pub enum Node {
    Scalar(OwnValue),
    Array(Orc<Vec<Node>>),
    Object(Orc<IndexMap<String, Node>>),
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
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

#[derive(Clone, Debug)]
pub struct Group {
    domain: Domain,
    nodes: NodeGroup,
}

#[derive(Clone, Debug)]
pub enum NodeGroup {
    Array(Orc<Vec<Node>>),
    Object(Orc<IndexMap<String, Node>>),
}

#[derive(Debug)]
pub enum UrcNodeGroup {
    Array(Urc<Vec<Node>>),
    Object(Urc<IndexMap<String, Node>>),
}

impl From<serde_json::Error> for HErr {
    fn from(e: serde_json::Error) -> HErr {
        HErr::Json(format!("{}", e))
    }
}

fn from_json_value(json: SerdeValue) -> Res<Domain> {
    let root_node = node_from_json(json);
    let nodes = Orc::new(vec![root_node]);
    Ok(Domain {
        nodes,
        write_policy: WritePolicy::ReadOnly,
    })
}

fn owned_value_to_node(v: OwnValue) -> Res<Node> {
    Ok(Node::Scalar(v))
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
        fn get_value(node: &Node) -> Value {
            match node {
                Node::Scalar(v) => v.as_value(),
                Node::Array(_) => Value::None,
                Node::Object(_) => Value::None,
            }
        }

        match self.nodes {
            UrcNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => Ok(get_value(x)),
                None => fault(""),
            },
            UrcNodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => Ok(get_value(x.1)),
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
                    .ok_or_else(|| HErr::Json("bad pos".into()))?;
                *node = Node::Scalar(value);
            }
        };
        Ok(())
    }
}

impl Cell {
    pub fn from_value_cell(cell: XCell) -> Res<XCell> {
        let s = cell.read()?.value()?.to_string();
        Cell::from_string(s.as_str())
    }

    pub fn from_file_cell(cell: XCell) -> Res<XCell> {
        let path = cell.as_path()?;
        Cell::from_path(path)
    }

    pub fn from_http_cell(cell: XCell) -> Res<XCell> {
        let s = cell.read()?.value()?.to_string();
        Cell::from_string(s.as_str())
    }

    pub fn from_string(s: impl AsRef<str>) -> Res<XCell> {
        let json: SerdeValue = serde_json::from_str(s.as_ref())?;
        let jcell = from_json_value(json)?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(jcell),
        })
    }

    pub fn from_path(path: impl AsRef<Path>) -> Res<XCell> {
        let file = File::open(path)?;
        let json: SerdeValue = serde_json::from_reader(file)?;
        let jcell = from_json_value(json)?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(jcell),
        })
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
            NodeGroup::Object(ref o) => match o.urc().get_index(self.pos) {
                Some(x) => Ok(get_typ(x.1)),
                None => fault(format!("bad index {}", self.pos)),
            },
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => UrcNodeGroup::Array(a.urc()),
                NodeGroup::Object(ref o) => UrcNodeGroup::Object(o.urc()),
            },
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => UrcNodeGroup::Array(a.urc()),
                NodeGroup::Object(ref o) => UrcNodeGroup::Object(o.urc()),
            },
            pos: self.pos,
        })
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
            NodeGroup::Object(ref object) => match object.urc().get_index(self.pos) {
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

    // TODO: remove this after implementing writer
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

    // TODO: remove this after implementing writer
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
                Selector::Str(k) => match o.urc().get_index_of(k) {
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
            NodeGroup::Array(array) => array.urc().len(),
            NodeGroup::Object(o) => o.urc().len(),
        })
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
            Node::Array(Orc::new(na))
        }
        SerdeValue::Object(o) => {
            let mut no = IndexMap::new();
            for (k, v) in o {
                no.insert(k, node_from_json(v));
            }
            Node::Object(Orc::new(no))
        }
    }
}
