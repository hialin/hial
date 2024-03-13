use std::io::Read;
use std::{fs::File, rc::Rc};

use indexmap::IndexMap;
use linkme::distributed_slice;
use yaml_rust::{ScanError, Yaml, YamlLoader};

use crate::implement_try_from_xell;
use crate::{
    api::{interpretation::*, *},
    utils::ownrc::{OwnRc, ReadRc, WriteRc},
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_YAML: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["yaml"],
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
    Object(OwnRc<IndexMap<Yaml, Node>>),
}

#[derive(Debug)]
pub(crate) enum ReadNodeGroup {
    Array(ReadRc<Vec<Node>>),
    Object(ReadRc<IndexMap<Yaml, Node>>),
}

#[derive(Debug)]
pub(crate) enum WriteNodeGroup {
    Array(WriteRc<Vec<Node>>),
    Object(WriteRc<IndexMap<Yaml, Node>>),
}

#[derive(Clone, Debug)]
pub(crate) enum Node {
    Scalar(Yaml),
    Array(OwnRc<Vec<Node>>),
    Object(OwnRc<IndexMap<Yaml, Node>>),
}

implement_try_from_xell!(Cell, Yaml);

impl From<ScanError> for HErr {
    fn from(e: ScanError) -> HErr {
        caused(HErrKind::InvalidFormat, "yaml parse error", e)
    }
}

impl Cell {
    pub(crate) fn from_cell(cell: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        match cell.interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Cell::make_cell(value, Some(cell))
            }
            "fs" => {
                let mut source = String::new();
                let r = cell.read();
                let path = r.as_file_path()?;
                File::open(path)
                    .map_err(|e| caused(HErrKind::IO, format!("cannot open file: {:?}", path), e))?
                    .read_to_string(&mut source)
                    .map_err(|e| {
                        caused(HErrKind::IO, format!("cannot read file: {:?}", path), e)
                    })?;
                Cell::make_cell(source, Some(cell))
            }
            _ => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Cell::make_cell(value, Some(cell)).map_err(|e| {
                    if e.kind == HErrKind::InvalidFormat {
                        noerr()
                    } else {
                        e
                    }
                })
            }
        }
    }

    fn make_cell(s: impl AsRef<str>, origin: Option<Xell>) -> Res<Xell> {
        let yaml_docs = YamlLoader::load_from_str(s.as_ref())?;
        let root_group_res: Res<Vec<Node>> = yaml_docs.iter().map(node_from_yaml).collect();
        let yaml_cell = Cell {
            group: Group {
                nodes: NodeGroup::Array(OwnRc::new(root_group_res?)),
                head: None,
            },
            pos: 0,
        };
        Ok(Xell::new_from(DynCell::from(yaml_cell), origin))
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_ty(n)),
                None => fault(""),
            },
            ReadNodeGroup::Object(ref o) => match o.get_index(self.pos) {
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
            ReadNodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => yaml_to_value(x.0),
                None => fault(""),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => to_value(x),
                None => fault(""),
            },
            ReadNodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => to_value(x.1),
                None => fault(""),
            },
        }
    }

    fn serial(&self) -> Res<String> {
        let yaml = match self.nodes {
            ReadNodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => node_to_yaml(x)?,
                None => fault("")?,
            },
            ReadNodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => node_to_yaml(x.1)?,
                None => fault("")?,
            },
        };
        let mut s = String::new();
        let mut emitter = yaml_rust::emitter::YamlEmitter::new(&mut s);
        emitter.compact(true);
        emitter
            .dump(&yaml)
            .map_err(|e| caused(HErrKind::InvalidFormat, "cannot serialize yaml node", e))?;
        if s.starts_with("---\n") {
            s = s[4..].to_string();
        }
        Ok(s)
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.nodes {
            WriteNodeGroup::Array(ref mut a) => match a.get_mut(self.pos) {
                Some(x) => *x = Node::Scalar(ownvalue_to_yaml(value)?),
                None => fault("")?,
            },
            WriteNodeGroup::Object(ref mut o) => match o.get_index_mut(self.pos) {
                Some(x) => *x.1 = Node::Scalar(ownvalue_to_yaml(value)?),
                None => fault("")?,
            },
        };
        Ok(())
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "yaml"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => {
                    ReadNodeGroup::Array(a.read().ok_or_else(|| lockerr("cannot read group"))?)
                }
                NodeGroup::Object(ref o) => {
                    ReadNodeGroup::Object(o.read().ok_or_else(|| lockerr("cannot read group"))?)
                }
            },
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            nodes: match self.group.nodes {
                NodeGroup::Array(ref a) => {
                    WriteNodeGroup::Array(a.write().ok_or_else(|| lockerr("cannot write group"))?)
                }
                NodeGroup::Object(ref o) => {
                    WriteNodeGroup::Object(o.write().ok_or_else(|| lockerr("cannot write group"))?)
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
                }),
                Some(Node::Object(o)) => Ok(Group {
                    nodes: NodeGroup::Object(o.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
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
                }),
                Some((_, Node::Object(o))) => Ok(Group {
                    nodes: NodeGroup::Object(o.clone()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                _ => nores(),
            },
        }
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match &self.group.head {
            Some(h) => Ok((h.0.clone(), h.1)),
            None => nores(),
        }
    }
}

fn get_ty(node: &Node) -> &'static str {
    match node {
        Node::Scalar(Yaml::Null) => "null",
        Node::Scalar(Yaml::Alias(_)) => "alias",
        Node::Scalar(Yaml::BadValue) => "badvalue",
        Node::Scalar(Yaml::Boolean(_)) => "bool",
        Node::Scalar(Yaml::Integer(_)) => "int",
        Node::Scalar(Yaml::Real(_)) => "float",
        Node::Scalar(Yaml::String(_)) => "string",
        Node::Array(_) => "array",
        Node::Object(_) => "object",
        _ => "",
    }
}

fn yaml_to_value(s: &Yaml) -> Res<Value> {
    Ok(match s {
        Yaml::Boolean(b) => Value::Bool(*b),
        Yaml::Integer(i) => Value::Int(Int::I64(*i)),
        Yaml::Real(r) => {
            let f = r
                .parse()
                .map_err(|e| caused(HErrKind::InvalidFormat, "", e))?;
            Value::Float(StrFloat(f))
        }
        Yaml::Alias(n) => Value::Str("alias"),
        Yaml::String(ref s) => Value::Str(s.as_str()),
        Yaml::BadValue => Value::Str("badvalue"),
        _ => Value::None,
    })
}

fn ownvalue_to_yaml(v: OwnValue) -> Res<Yaml> {
    Ok(match v {
        OwnValue::Bool(b) => Yaml::Boolean(b),
        OwnValue::Int(i) => match i {
            Int::I64(i) => Yaml::Integer(i),
            Int::U64(i) => Yaml::Integer(i as i64),
            Int::I32(i) => Yaml::Integer(i as i64),
            Int::U32(i) => Yaml::Integer(i as i64),
        },
        OwnValue::Float(StrFloat(f)) => Yaml::Real(f.to_string()),
        OwnValue::String(s) => Yaml::String(s),
        OwnValue::None => Yaml::Null,
        OwnValue::Bytes(b) => Yaml::String(String::from_utf8_lossy(&b).to_string()),
    })
}

fn to_value(node: &Node) -> Res<Value> {
    match node {
        Node::Scalar(y) => yaml_to_value(y),
        _ => nores(),
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
            NodeGroup::Object(o) => o.read().ok_or_else(|| lockerr("cannot read group"))?.len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        let len = self.len()?;
        match &self.nodes {
            NodeGroup::Array(array) if index < len => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            NodeGroup::Object(o) if index < len => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
        }
    }

    fn get_all(&self, key: Value) -> Res<Self::CellIterator> {
        let cell = match &self.nodes {
            NodeGroup::Array(a) => nores(),
            NodeGroup::Object(o) => match key {
                Value::Str(k) => {
                    let o = o.read().ok_or_else(|| lockerr("cannot read group"))?;
                    match o.get_index_of(&Yaml::String(k.to_string())) {
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

fn node_from_yaml(y: &Yaml) -> Res<Node> {
    let value = match y {
        Yaml::Array(a) => {
            let mut na = vec![];
            for v in a {
                na.push(node_from_yaml(v)?);
            }
            Node::Array(OwnRc::new(na))
        }
        Yaml::Hash(o) => {
            let mut no = IndexMap::new();
            for (yk, yv) in o {
                no.insert(yk.clone(), node_from_yaml(yv)?);
            }
            Node::Object(OwnRc::new(no))
        }
        _ => Node::Scalar(y.clone()),
    };
    Ok(value)
}

fn node_to_yaml(node: &Node) -> Res<Yaml> {
    Ok(match node {
        Node::Scalar(y) => y.clone(),
        Node::Array(a) => {
            let mut na = yaml_rust::yaml::Array::new();
            for v in a.read().ok_or_else(|| lockerr("cannot read group"))?.iter() {
                na.push(node_to_yaml(v)?);
            }
            Yaml::Array(na)
        }
        Node::Object(o) => {
            let mut no = yaml_rust::yaml::Hash::new();
            for (yk, yv) in o.read().ok_or_else(|| lockerr("cannot read group"))?.iter() {
                no.insert(yk.clone(), node_to_yaml(yv)?);
            }
            Yaml::Hash(no)
        }
    })
}
