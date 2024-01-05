use std::io::Read;
use std::{fs::File, path::Path, rc::Rc};

use indexmap::IndexMap;
use linkme::distributed_slice;
use yaml_rust::{ScanError, Yaml, YamlLoader};

use crate::base::{Cell as XCell, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_YAML: ElevationConstructor = ElevationConstructor {
    source_interpretation: "value",
    target_interpretation: "yaml",
    constructor: Cell::from_value_cell,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static FILE_TO_YAML: ElevationConstructor = ElevationConstructor {
    source_interpretation: "file",
    target_interpretation: "yaml",
    constructor: Cell::from_file_cell,
};

#[derive(Clone, Debug)]
pub struct Domain {
    nodes: NodeGroup,
}
impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "yaml"
    }

    fn root(&self) -> Res<Cell> {
        Ok(Cell {
            group: Group {
                domain: self.clone(),
                nodes: self.nodes.clone(),
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
    domain: Domain,
    nodes: NodeGroup,
}

#[derive(Clone, Debug)]
pub enum NodeGroup {
    Array(Rc<Vec<Node>>),
    Object(Rc<IndexMap<String, Node>>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scalar {
    Null,
    Bool(bool),
    Int(i64),
    Float(StrFloat),
    Alias(usize),
    String(String),
}

#[derive(Clone, Debug)]
pub enum Node {
    Scalar(Scalar),
    Array(Rc<Vec<Node>>),
    Object(Rc<IndexMap<String, Node>>),
}

impl From<ScanError> for HErr {
    fn from(e: ScanError) -> HErr {
        HErr::Yaml(format!("{}", e))
    }
}

impl Cell {
    pub fn from_value_cell(cell: XCell) -> Res<XCell> {
        let reader = cell.read()?;
        let value = reader.value()?;
        let s = value.as_cow_str();
        Cell::from_string(s)
    }

    pub fn from_file_cell(cell: XCell) -> Res<XCell> {
        let path = cell.as_path()?;
        Cell::from_path(path)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Res<XCell> {
        let mut source = String::new();
        File::open(path)?.read_to_string(&mut source)?;
        Cell::from_string(source)
    }

    pub fn from_string(s: impl AsRef<str>) -> Res<XCell> {
        let yaml_docs = YamlLoader::load_from_str(s.as_ref())?;
        let root_group_res: Res<Vec<Node>> = yaml_docs.iter().map(node_from_yaml).collect();
        let domain = Domain {
            nodes: NodeGroup::Array(Rc::new(root_group_res?)),
        };
        let c = domain.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(c),
        })
    }
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => nores(),
            NodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => Ok(Value::Str(x.0)),
                None => fault(""),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(x) => Ok(get_value(x)),
                None => fault(""),
            },
            NodeGroup::Object(ref o) => match o.get_index(self.pos) {
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
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_typ(n)),
                None => fault(""),
            },
            NodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => Ok(get_typ(x.1)),
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
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some(Node::Object(o)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                }),
                _ => nores(),
            },
            NodeGroup::Object(ref object) => match object.get_index(self.pos) {
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
}

fn get_typ(node: &Node) -> &str {
    match node {
        Node::Scalar(Scalar::Null) => "null",
        Node::Scalar(Scalar::Bool(_)) => "bool",
        Node::Scalar(Scalar::Int(_)) => "int",
        Node::Scalar(Scalar::Float(_)) => "float",
        Node::Scalar(Scalar::Alias(_)) => "alias",
        Node::Scalar(Scalar::String(_)) => "string",
        Node::Array(_) => "array",
        Node::Object(_) => "object",
    }
}

fn scalar_to_value(s: &Scalar) -> Value {
    match s {
        Scalar::Null => Value::None,
        Scalar::Bool(b) => Value::Bool(*b),
        Scalar::Int(i) => Value::Int(Int::I64(*i)),
        Scalar::Float(f) => Value::Float(*f),
        Scalar::Alias(n) => Value::Str("alias"), // todo: fix this
        Scalar::String(ref s) => Value::Str(s.as_str()),
    }
}

fn get_value(node: &Node) -> Value {
    match node {
        Node::Scalar(s) => scalar_to_value(s),
        Node::Array(_) => Value::None,
        Node::Object(_) => Value::None,
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
            NodeGroup::Object(o) => match key.into() {
                Selector::Star | Selector::DoubleStar | Selector::Top => self.at(0),
                Selector::Str(k) => match o.get_index_of(k) {
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
            NodeGroup::Object(o) => o.len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(array) if index < array.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            NodeGroup::Object(o) if index < o.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
        }
    }
}

fn node_from_yaml(y: &Yaml) -> Res<Node> {
    let value = match y {
        Yaml::Null => Node::Scalar(Scalar::Null),
        Yaml::Boolean(b) => Node::Scalar(Scalar::Bool(*b)),
        Yaml::Integer(n) => Node::Scalar(Scalar::Int(*n)),
        Yaml::Real(r) => {
            if let Some(x) = y.as_f64() {
                Node::Scalar(Scalar::Float(StrFloat(x)))
            } else {
                return Err(HErr::Yaml(format!("bad yaml float: {:?}", r)));
            }
        }
        Yaml::String(s) => Node::Scalar(Scalar::String(s.clone())),
        Yaml::Alias(n) => Node::Scalar(Scalar::Alias(*n)),
        Yaml::Array(a) => {
            let mut na = vec![];
            for v in a {
                na.push(node_from_yaml(v)?);
            }
            Node::Array(Rc::new(na))
        }
        Yaml::Hash(o) => {
            let mut no = IndexMap::new();
            for (yk, yv) in o {
                let knode = node_from_yaml(yk)?;
                if let Node::Scalar(Scalar::String(sk)) = knode {
                    let v = node_from_yaml(yv)?;
                    no.insert(sk, v);
                } else {
                    return Err(HErr::Yaml(format!(
                        "bad yaml label (must be string): {:?}",
                        knode
                    )));
                }
            }
            Node::Object(Rc::new(no))
        }
        Yaml::BadValue => Node::Scalar(Scalar::String("«BadValue»".into())),
    };
    Ok(value)
}
