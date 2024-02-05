use std::{fs::File, rc::Rc};

use indexmap::IndexMap;
use linkme::distributed_slice;
use serde::Serialize;
use serde_json::{ser::PrettyFormatter, Serializer, Value as SValue};

use crate::{
    base::{Cell as XCell, *},
    guard_some,
    utils::{
        indentation::{detect_file_indentation, detect_indentation},
        ownrc::*,
    },
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_JSON: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs", "http"],
    target_interpretations: &["json"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    nodes: NodeGroup,
    head: Option<Rc<(Cell, Relation)>>,
    indent: Rc<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum NodeGroup {
    Array(OwnRc<Vec<Node>>),
    Object(OwnRc<IndexMap<String, Node>>),
}

#[derive(Clone, Debug)]
pub(crate) enum Node {
    Scalar(SValue),
    Array(OwnRc<Vec<Node>>),
    Object(OwnRc<IndexMap<String, Node>>),
}

#[derive(Debug)]
pub(crate) struct CellReader {
    nodes: ReadNodeGroup,
    pos: usize,
    indent: Rc<String>,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    nodes: WriteNodeGroup,
    pos: usize,
}

#[derive(Debug)]
pub(crate) enum ReadNodeGroup {
    Array(ReadRc<Vec<Node>>),
    Object(ReadRc<IndexMap<String, Node>>),
}

#[derive(Debug)]
pub(crate) enum WriteNodeGroup {
    Array(WriteRc<Vec<Node>>),
    Object(WriteRc<IndexMap<String, Node>>),
}

impl Cell {
    pub(crate) fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        let (serde_value, indent) = match cell.interpretation() {
            "value" => {
                let s = cell.read().value()?.to_string();
                let indent = detect_indentation(&s);
                (serde_json::from_str(s.as_ref())?, indent)
            }
            "fs" => {
                let path = cell.as_file_path()?;
                let indent = detect_file_indentation(path);
                (
                    serde_json::from_reader(
                        File::open(path)
                            .map_err(|e| caused(HErrKind::IO, "cannot read json", e))?,
                    )?,
                    indent,
                )
            }
            "http" => {
                let s = cell.read().value()?.to_string();
                let indent = detect_indentation(&s);
                (serde_json::from_str(s.as_ref())?, indent)
            }
            _ => return nores(),
        };
        Self::from_serde_value(serde_value, Some(cell), indent)
    }

    fn from_serde_value(json: SValue, origin: Option<XCell>, indent: String) -> Res<XCell> {
        let nodes = OwnRc::new(vec![serde_to_node(json)]);
        let json_cell = Cell {
            group: Group {
                nodes: NodeGroup::Array(nodes),
                head: None,
                indent: Rc::new(indent),
            },
            pos: 0,
        };
        Ok(new_cell(DynCell::from(json_cell), origin))
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "json"
    }

    fn ty(&self) -> Res<&str> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a
                .read()
                .ok_or_else(|| lockerr("cannot read cell"))?
                .get(self.pos)
            {
                Some(n) => Ok(get_ty(n)),
                None => fault(format!("bad index {}", self.pos)),
            },
            NodeGroup::Object(ref o) => match o
                .read()
                .ok_or_else(|| lockerr("cannot read cell"))?
                .get_index(self.pos)
            {
                Some(x) => Ok(get_ty(x.1)),
                None => fault(format!("bad index {}", self.pos)),
            },
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => {
                    ReadNodeGroup::Array(a.read().ok_or_else(|| lockerr("cannot read cell"))?)
                }
                NodeGroup::Object(ref o) => {
                    ReadNodeGroup::Object(o.read().ok_or_else(|| lockerr("cannot read cell"))?)
                }
            },
            pos: self.pos,
            indent: Rc::clone(&self.group.indent),
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => {
                    WriteNodeGroup::Array(a.write().ok_or_else(|| lockerr("cannot write cell"))?)
                }
                NodeGroup::Object(ref o) => {
                    WriteNodeGroup::Object(o.write().ok_or_else(|| lockerr("cannot write cell"))?)
                }
            },
            pos: self.pos,
        })
    }

    fn sub(&self) -> Res<Group> {
        match self.group.nodes {
            NodeGroup::Array(ref array) => match &array
                .read()
                .ok_or_else(|| lockerr("cannot read cell"))?
                .get(self.pos)
            {
                Some(Node::Array(a)) => Ok(Group {
                    nodes: NodeGroup::Array(a.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                    indent: Rc::clone(&self.group.indent),
                }),
                Some(Node::Object(o)) => Ok(Group {
                    nodes: NodeGroup::Object(o.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                    indent: Rc::clone(&self.group.indent),
                }),
                _ => nores(),
            },
            NodeGroup::Object(ref object) => match object
                .read()
                .ok_or_else(|| lockerr("cannot read cell"))?
                .get_index(self.pos)
            {
                Some((_, Node::Array(a))) => Ok(Group {
                    nodes: NodeGroup::Array(a.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                    indent: Rc::clone(&self.group.indent),
                }),
                Some((_, Node::Object(o))) => Ok(Group {
                    nodes: NodeGroup::Object(o.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                    indent: Rc::clone(&self.group.indent),
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
        if let ReadNodeGroup::Object(ref o) = self.nodes {
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
                Node::Scalar(sv) => Ok(serde_to_value(sv)),
                _ => nores(),
            }
        }

        match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => get_value(x),
                None => fault(""),
            },
            ReadNodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => get_value(x.1),
                None => fault(""),
            },
        }
    }

    fn serial(&self) -> Res<String> {
        let serde_value = match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(node) => node_to_serde(node)?,
                None => return fault(format!("bad index {}", self.pos)),
            },
            ReadNodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => node_to_serde(x.1)?,
                None => return fault(format!("bad index {}", self.pos)),
            },
        };

        let mut buf = Vec::new();
        if !self.indent.is_empty() {
            let formatter = PrettyFormatter::with_indent(self.indent.as_bytes());
            serde_value
                .serialize(&mut Serializer::with_formatter(&mut buf, formatter))
                .map_err(|e| caused(HErrKind::IO, "cannot serialize json", e))?;
        } else {
            serde_value
                .serialize(&mut Serializer::new(&mut buf))
                .map_err(|e| caused(HErrKind::IO, "cannot serialize json", e))?;
        };
        String::from_utf8(buf)
            .map_err(|e| caused(HErrKind::InvalidFormat, "bad json serialization", e))
    }
}

impl CellWriterTrait for CellWriter {
    fn set_label(&mut self, label: OwnValue) -> Res<()> {
        match self.nodes {
            WriteNodeGroup::Array(_) => {
                return userres("cannot set label on array object");
            }
            WriteNodeGroup::Object(ref mut o) => {
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
            WriteNodeGroup::Array(ref mut a) => {
                a[self.pos] = Node::Scalar(ownvalue_to_serde(value));
            }
            WriteNodeGroup::Object(ref mut o) => {
                let (_, node) = o
                    .get_index_mut(self.pos)
                    .ok_or_else(|| faulterr("bad pos"))?;
                *node = Node::Scalar(ownvalue_to_serde(value));
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
        Node::Scalar(SValue::Null) => "null",
        Node::Scalar(SValue::Bool(_)) => "bool",
        Node::Scalar(SValue::Number(_)) => "number",
        Node::Scalar(SValue::String(_)) => "string",
        Node::Scalar(_) => "??", // should not happen
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
                Selector::Str(k) => match o
                    .read()
                    .ok_or_else(|| lockerr("cannot read group"))?
                    .get_index_of(k)
                {
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
            NodeGroup::Array(array) => array
                .read()
                .ok_or_else(|| lockerr("cannot read group"))?
                .len(),
            NodeGroup::Object(o) => o.read().ok_or_else(|| lockerr("cannot read group"))?.len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(ref array)
                if index
                    < array
                        .read()
                        .ok_or_else(|| lockerr("cannot read group"))?
                        .len() =>
            {
                Ok(Cell {
                    group: self.clone(),
                    pos: index,
                })
            }
            NodeGroup::Object(ref o)
                if index < o.read().ok_or_else(|| lockerr("cannot read group"))?.len() =>
            {
                Ok(Cell {
                    group: self.clone(),
                    pos: index,
                })
            }
            _ => nores(),
        }
    }
}

fn serde_to_value(sv: &SValue) -> Value<'_> {
    match sv {
        SValue::Bool(b) => Value::Bool(*b),
        SValue::Number(n) => {
            if n.is_i64() {
                Value::from(n.as_i64().unwrap())
            } else if n.is_u64() {
                Value::from(n.as_u64().unwrap())
            } else {
                Value::from(n.as_f64().unwrap())
            }
        }
        SValue::String(s) => Value::Str(s),
        _ => Value::None,
    }
}

fn ownvalue_to_serde(v: OwnValue) -> SValue {
    match v {
        OwnValue::None => SValue::Null,
        OwnValue::Bool(b) => SValue::Bool(b),
        OwnValue::Int(Int::I32(i)) => SValue::Number(i.into()),
        OwnValue::Int(Int::U32(i)) => SValue::Number(i.into()),
        OwnValue::Int(Int::I64(i)) => SValue::Number(i.into()),
        OwnValue::Int(Int::U64(i)) => SValue::Number(i.into()),
        OwnValue::Float(StrFloat(f)) => {
            SValue::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))
        }
        OwnValue::String(s) => SValue::String(s),
        OwnValue::Bytes(b) => SValue::String(String::from_utf8_lossy(&b).into()),
    }
}

fn serde_to_node(sv: SValue) -> Node {
    match sv {
        SValue::Array(a) => {
            let mut na = vec![];
            for v in a {
                na.push(serde_to_node(v));
            }
            Node::Array(OwnRc::new(na))
        }
        SValue::Object(o) => {
            let mut no = IndexMap::new();
            for (k, v) in o {
                no.insert(k, serde_to_node(v));
            }
            Node::Object(OwnRc::new(no))
        }
        _ => Node::Scalar(sv),
    }
}

fn node_to_serde(node: &Node) -> Res<SValue> {
    Ok(match node {
        Node::Scalar(sv) => sv.clone(),
        Node::Array(a) => {
            let mut na = vec![];
            for v in &*a.write().ok_or_else(|| lockerr("cannot write nodes"))? {
                na.push(node_to_serde(v)?);
            }
            SValue::Array(na)
        }
        Node::Object(o) => {
            let mut no = serde_json::map::Map::new();
            for (k, v) in o
                .write()
                .ok_or_else(|| lockerr("cannot write nodes"))?
                .as_slice()
            {
                no.insert(k.clone(), node_to_serde(v)?);
            }
            SValue::Object(no)
        }
    })
}
