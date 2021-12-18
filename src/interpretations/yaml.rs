use crate::{base::*, utils::vecmap::*};
use std::io::Read;
use std::{fs::File, path::Path, rc::Rc};
use yaml_rust::{ScanError, Yaml, YamlLoader};

#[derive(Clone, Debug)]
pub struct Domain {
    preroot: NodeGroup,
}
impl InDomain for Domain {
    type Cell = Cell;
    type Group = Group;

    fn interpretation(&self) -> &str {
        "yaml"
    }

    fn root(&self) -> Res<Cell> {
        Ok(Cell {
            group: Group {
                domain: self.clone(),
                nodes: self.preroot.clone(),
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
pub struct ValueRef {
    group: Group,
    pos: usize,
    pub is_label: bool,
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Domain,
    nodes: NodeGroup,
}

#[derive(Clone, Debug)]
pub enum NodeGroup {
    Array(Rc<Vec<Node>>),
    Object(Rc<VecMap<String, Node>>),
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
    Object(Rc<VecMap<String, Node>>),
}

impl From<ScanError> for HErr {
    fn from(e: ScanError) -> HErr {
        HErr::Yaml(format!("{}", e))
    }
}

pub fn from_path(path: &Path) -> Res<Cell> {
    let mut source = String::new();
    File::open(path)?.read_to_string(&mut source)?;
    from_string(&source)
}

pub fn from_string(source: &str) -> Res<Cell> {
    let yaml_docs = YamlLoader::load_from_str(source)?;
    let root_group_res: Res<Vec<Node>> = yaml_docs.iter().map(node_from_yaml).collect();
    let domain = Rc::new(Domain {
        preroot: NodeGroup::Array(Rc::new(root_group_res?)),
    });
    domain.root()
}

impl InValueRef for ValueRef {
    fn get(&self) -> Res<Value> {
        if self.is_label {
            match self.group.nodes {
                NodeGroup::Array(ref a) => NotFound::NoLabel.into(),
                NodeGroup::Object(ref o) => match o.at(self.pos) {
                    Some(x) => Ok(Value::Str(x.0)),
                    None => HErr::internal("").into(),
                },
            }
        } else {
            match self.group.nodes {
                NodeGroup::Array(ref a) => match a.get(self.pos) {
                    Some(x) => Ok(get_value(x)),
                    None => HErr::internal("").into(),
                },
                NodeGroup::Object(ref o) => match o.at(self.pos) {
                    Some(x) => Ok(get_value(&x.1)),
                    None => HErr::internal("").into(),
                },
            }
        }
    }
}

impl InCell for Cell {
    type Domain = Domain;
    type ValueRef = ValueRef;

    fn domain(&self) -> &Domain {
        &self.group.domain
    }

    fn typ(&self) -> Res<&str> {
        match self.group.nodes {
            NodeGroup::Array(ref a) => match a.get(self.pos) {
                Some(n) => Ok(get_typ(n)),
                None => HErr::internal("").into(),
            },
            NodeGroup::Object(ref o) => match o.at(self.pos) {
                Some(x) => Ok(get_typ(&x.1)),
                None => HErr::internal("").into(),
            },
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<ValueRef> {
        Ok(ValueRef {
            group: self.group.clone(),
            pos: self.pos,
            is_label: true,
        })
    }

    fn value(&self) -> Res<ValueRef> {
        Ok(ValueRef {
            group: self.group.clone(),
            pos: self.pos,
            is_label: false,
        })
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
                _ => NotFound::NoGroup(format!("/")).into(),
            },
            NodeGroup::Object(ref object) => match object.at(self.pos) {
                Some((_, Node::Array(a))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Array(a.clone()),
                }),
                Some((_, Node::Object(o))) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Object(o.clone()),
                }),
                _ => NotFound::NoGroup(format!("/")).into(),
            },
        }
    }

    fn attr(&self) -> Res<Group> {
        NotFound::NoGroup(format!("@")).into()
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

impl InGroup for Group {
    type Domain = Domain;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(a) => NotFound::NoLabel.into(),
            NodeGroup::Object(o) => match key.into() {
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
        match &self.nodes {
            NodeGroup::Array(array) => array.len(),
            NodeGroup::Object(o) => o.len(),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match &self.nodes {
            NodeGroup::Array(array) if index < array.len() => Ok(Cell {
                group: self.clone(),
                pos: index as usize,
            }),
            NodeGroup::Object(o) if index < o.len() => Ok(Cell {
                group: self.clone(),
                pos: index as usize,
            }),
            _ => NotFound::NoResult(format!("{}", index)).into(),
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
            let mut no = VecMap::new();
            for (yk, yv) in o {
                let knode = node_from_yaml(yk)?;
                if let Node::Scalar(Scalar::String(sk)) = knode {
                    let v = node_from_yaml(yv)?;
                    no.put(sk, v);
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
