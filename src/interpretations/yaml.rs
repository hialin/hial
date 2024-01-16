use std::io::Read;
use std::{fs::File, path::Path, rc::Rc};

use indexmap::IndexMap;
use linkme::distributed_slice;
use yaml_rust::{ScanError, Yaml, YamlLoader};

use crate::base::{Cell as XCell, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_YAML: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "file"],
    target_interpretations: &["yaml"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub struct Domain(Rc<DomainData>);

#[derive(Clone, Debug)]
pub struct DomainData {
    nodes: NodeGroup,
    origin: Option<XCell>,
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
                nodes: self.0.nodes.clone(),
                head: None,
            },
            pos: 0,
        })
    }

    fn origin(&self) -> Res<XCell> {
        match &self.0.origin {
            Some(c) => Ok(c.clone()),
            None => nores(),
        }
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
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
    head: Option<Box<(Cell, Relation)>>,
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
        caused(HErrKind::InvalidFormat, "yaml parse error", e)
    }
}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.domain().interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Cell::make_cell(value, Some(cell))
            }
            "file" => {
                let mut source = String::new();
                let path = cell.as_file_path()?;
                File::open(path)
                    .map_err(|e| {
                        caused(
                            HErrKind::IO,
                            format!("cannot open file: {}", path.to_string_lossy()),
                            e,
                        )
                    })?
                    .read_to_string(&mut source)
                    .map_err(|e| {
                        caused(
                            HErrKind::IO,
                            format!("cannot read file: {}", path.to_string_lossy()),
                            e,
                        )
                    })?;
                Cell::make_cell(source, Some(cell))
            }
            _ => fault(""),
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> Res<XCell> {
        let mut source = String::new();
        let path = path.as_ref();
        File::open(path)
            .map_err(|e| {
                caused(
                    HErrKind::IO,
                    format!("cannot open file: {}", path.to_string_lossy()),
                    e,
                )
            })?
            .read_to_string(&mut source)
            .map_err(|e| {
                caused(
                    HErrKind::IO,
                    format!("cannot read file: {}", path.to_string_lossy()),
                    e,
                )
            })?;
        Cell::make_cell(source, None)
    }

    pub fn from_str(source: impl AsRef<str>) -> Res<XCell> {
        Cell::make_cell(source, None)
    }

    fn make_cell(s: impl AsRef<str>, origin: Option<XCell>) -> Res<XCell> {
        let yaml_docs = YamlLoader::load_from_str(s.as_ref())?;
        let root_group_res: Res<Vec<Node>> = yaml_docs.iter().map(node_from_yaml).collect();
        let domain = Domain(Rc::new(DomainData {
            origin,
            nodes: NodeGroup::Array(Rc::new(root_group_res?)),
        }));
        Ok(XCell {
            dyn_cell: DynCell::from(domain.root()?),
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
                Some(x) => get_value(x),
                None => fault(""),
            },
            NodeGroup::Object(ref o) => match o.get_index(self.pos) {
                Some(x) => get_value(x.1),
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

    fn domain(&self) -> Domain {
        self.group.domain.clone()
    }

    fn ty(&self) -> Res<&str> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_ty(n)),
                None => fault(""),
            },
            NodeGroup::Object(ref o) => match o.get_index(self.pos) {
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
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                    head: Some(Box::new((self.clone(), Relation::Sub))),
                }),
                Some(Node::Object(o)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                    head: Some(Box::new((self.clone(), Relation::Sub))),
                }),
                _ => nores(),
            },
            NodeGroup::Object(ref object) => match object.get_index(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                    head: Some(Box::new((self.clone(), Relation::Sub))),
                }),
                Some((_, Node::Object(o))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                    head: Some(Box::new((self.clone(), Relation::Sub))),
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

fn get_ty(node: &Node) -> &str {
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

fn get_value(node: &Node) -> Res<Value> {
    match node {
        Node::Scalar(s) => Ok(scalar_to_value(s)),
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
                return fault("failed assertion, assumed all yaml reals can output f64");
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
                    return fault(format!(
                        "unsupported yaml label (expected string): {:?}",
                        knode
                    ));
                }
            }
            Node::Object(Rc::new(no))
        }
        Yaml::BadValue => Node::Scalar(Scalar::String("«BadValue»".into())),
    };
    Ok(value)
}
