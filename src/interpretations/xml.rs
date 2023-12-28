use std::io::BufRead;
use std::path::Path;
use std::rc::Rc;

use quick_xml::{events::Event, Error as XmlError, Reader};

// use crate::vecmap::*;
use crate::{base::*, debug, guard_some};

#[derive(Clone, Debug)]
pub struct Domain {
    preroot: NodeList,
}

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "xml"
    }

    fn root(&self) -> Res<Cell> {
        Ok(Cell {
            group: Group {
                domain: self.clone(),
                nodes: NodeGroup::Node(self.preroot.clone()),
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
    Node(NodeList),
    Attr(AttrList),
}

#[derive(Clone, Debug)]
pub struct NodeList(Rc<Vec<Node>>);

#[derive(Clone, Debug)]
pub struct AttrList(Rc<Vec<Attribute>>);

#[derive(Debug)]
enum Node {
    Document(Rc<Vec<Node>>),
    Decl(Rc<Vec<Attribute>>),
    // version, encoding, standalone
    DocType(String),
    PI(String),
    Element((String, Rc<Vec<Attribute>>, Rc<Vec<Node>>)),
    Text(String),
    Comment(String),
    CData(Vec<u8>),
    Error(String),
}

#[derive(Debug)]
enum Attribute {
    Attribute(String, String),
    Error(String),
}

pub fn from_path(path: &Path) -> Res<Domain> {
    let mut reader = Reader::from_file(path).map_err(HErr::from)?;
    let root = xml_to_node(&mut reader)?;
    Ok(Domain {
        preroot: NodeList(Rc::new(vec![root])),
    })
}

pub fn from_string(string: &str) -> Res<Domain> {
    let mut reader = Reader::from_str(string);
    let root = xml_to_node(&mut reader)?;
    Ok(Domain {
        preroot: NodeList(Rc::new(vec![root])),
    })
}

fn xml_to_node<B: BufRead>(reader: &mut Reader<B>) -> Res<Node> {
    reader.trim_text(true);
    reader.expand_empty_elements(true);

    #[derive(Debug, Default)]
    struct Counts {
        count_document: usize,
        count_decl: usize,
        count_doc_type: usize,
        count_pi: usize,
        count_element: usize,
        count_text: usize,
        count_comment: usize,
        count_cdata: usize,
        count_attributes: usize,
    }
    let mut counts = Counts::default();

    let mut stack: Vec<Vec<Node>> = vec![vec![]];
    let mut attribute_stack: Vec<Vec<Attribute>> = vec![vec![]];
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Decl(ref e)) => {
                counts.count_decl += 1;
                let mut attrs = vec![];
                let rawversion = e.version()?;
                let version = reader.decode(rawversion.as_ref())?;
                attrs.push(Attribute::Attribute("version".into(), version.into()));
                if let Some(encoding) = e.encoding() {
                    let rawencoding = encoding?;
                    let encoding = reader.decode(rawencoding.as_ref())?;
                    attrs.push(Attribute::Attribute("encoding".into(), encoding.into()));
                }
                if let Some(standalone) = e.standalone() {
                    let rawstandalone = standalone?;
                    let standalone = reader.decode(rawstandalone.as_ref())?;
                    attrs.push(Attribute::Attribute("standalone".into(), standalone.into()));
                }
                let v = guard_some!(stack.last_mut(), {
                    return Err(HErr::Xml(format!("no element in stack")));
                });
                v.push(Node::Decl(Rc::new(attrs)));
            }
            Ok(Event::DocType(ref e)) => {
                counts.count_doc_type += 1;
                let rawdoctype = e.unescaped()?;
                let doctype = reader.decode(rawdoctype.as_ref())?;
                let v = guard_some!(stack.last_mut(), {
                    return Err(HErr::Xml(format!("no element in stack")));
                });
                v.push(Node::DocType(doctype.into()));
            }
            Ok(Event::PI(ref e)) => {
                counts.count_pi += 1;
                let text = reader.decode(e.as_ref())?;
                let v = guard_some!(stack.last_mut(), {
                    return Err(HErr::Xml(format!("no element in stack")));
                });
                v.push(Node::PI(text.into()));
            }
            Ok(Event::Start(ref e)) => {
                let mut attrs = vec![];
                for resa in e.attributes().with_checks(false) {
                    match resa {
                        Ok(a) => {
                            let key = reader.decode(a.key)?;
                            let value = reader.decode(&a.value)?;
                            attrs.push(Attribute::Attribute(key.into(), value.into()))
                        }
                        Err(err) => attrs.push(Attribute::Error(format!("{}", err))),
                    }
                }
                attrs.shrink_to_fit();
                attribute_stack.push(attrs);
                stack.push(vec![]);
            }
            Ok(Event::Text(e)) => {
                counts.count_text += 1;
                let text = reader.decode(e.as_ref())?;
                // let text = e.unescape_and_decode(&reader).map_err(generr)?;
                let v = &mut stack
                    .last_mut()
                    .ok_or(HErr::Xml(format!("no element in stack")))?;
                v.push(Node::Text(text.into()));
            }
            Ok(Event::Comment(e)) => {
                counts.count_comment += 1;
                // let text = e.unescape_and_decode(&reader).map_err(generr)?;
                let text = reader.decode(e.as_ref())?;
                let v = &mut stack
                    .last_mut()
                    .ok_or(HErr::Xml(format!("no element in stack")))?;
                v.push(Node::Comment(text.into()));
            }
            Ok(Event::CData(e)) => {
                counts.count_cdata += 1;
                let v = &mut stack
                    .last_mut()
                    .ok_or(HErr::Xml(format!("no element in stack")))?;
                let mut u = e.to_vec();
                u.shrink_to_fit();
                v.push(Node::CData(u));
            }
            Ok(Event::End(ref e)) => {
                counts.count_element += 1;
                let mut v = stack
                    .pop()
                    .ok_or(HErr::Xml(format!("no element in stack")))?;
                v.shrink_to_fit();
                let mut a = attribute_stack
                    .pop()
                    .ok_or(HErr::Xml(format!("no element in stack")))?;
                a.shrink_to_fit();
                counts.count_attributes += a.len();
                let name = reader.decode(e.name())?;
                let x = Node::Element((name.into(), Rc::new(a), Rc::new(v)));
                let v = &mut stack
                    .last_mut()
                    .ok_or(HErr::Xml(format!("no element in stack")))?;
                v.push(x);
            }
            Ok(Event::Empty(ref _e)) => {
                // should not happed because we use reader.expand_empty_elements(true);
            }
            Err(err) => {
                let text = format!("{}", err);
                let v = &mut stack
                    .last_mut()
                    .ok_or(HErr::Xml(format!("no element in stack")))?;
                v.push(Node::Error(text));
            }

            Ok(Event::Eof) => break,
            // _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    let v = stack
        .pop()
        .ok_or(HErr::Xml(format!("no element in stack")))?;
    let document = Node::Document(Rc::new(v));

    debug!("xml stats: {:?}", counts);
    Ok(document)
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        match &self.group.nodes {
            NodeGroup::Node(group) => match &group.0[self.pos] {
                Node::Element((x, _, _)) => Ok(Value::Str(x.as_str())),
                Node::Decl(_) => Ok(Value::Str("xml")),
                Node::DocType(x) => Ok(Value::Str("DOCTYPE")),
                x => nores(),
            },
            NodeGroup::Attr(group) => match &group.0[self.pos] {
                Attribute::Attribute(k, _) => Ok(Value::Str(k.as_str())),
                _ => nores(),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match &self.group.nodes {
            NodeGroup::Node(group) => match &group.0[self.pos] {
                Node::Document(_) => Ok(Value::None),
                Node::Decl(_) => Ok(Value::Str("xml")),
                Node::DocType(x) => Ok(Value::Str(x.trim())),
                Node::PI(x) => Ok(Value::Str(x)),
                Node::Element((x, _, _)) => Ok(Value::Str(x)),
                Node::Text(x) => Ok(Value::Str(x)),
                Node::Comment(x) => Ok(Value::Str(x)),
                Node::CData(x) => Ok(Value::Bytes(x.as_slice())),
                Node::Error(x) => Ok(Value::Str(x)),
            },
            NodeGroup::Attr(group) => match &group.0[self.pos] {
                Attribute::Attribute(_, x) => Ok(Value::Str(x)),
                Attribute::Error(x) => Ok(Value::Str(x)),
            },
        }
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn typ(&self) -> Res<&str> {
        match &self.group.nodes {
            NodeGroup::Node(group) => match &group.0[self.pos] {
                Node::Document(_) => Ok("document"),
                Node::Decl(_) => Ok("decl"),
                Node::DocType(_) => Ok("doctype"),
                Node::PI(_) => Ok("PI"),
                Node::Element(_) => Ok("element"),
                Node::Text(_) => Ok("text"),
                Node::Comment(_) => Ok("comment"),
                Node::CData(_) => Ok("cdata"),
                Node::Error(_) => Ok("error"),
            },
            NodeGroup::Attr(group) => Ok("attribute"),
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
        match &self.group.nodes {
            NodeGroup::Node(group) => match &group.0[self.pos] {
                Node::Document(x) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Node(NodeList(x.clone())),
                }),
                Node::Element((_, _, x)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Node(NodeList(x.clone())),
                }),
                _ => nores(),
            },
            _ => nores(),
        }
    }

    fn attr(&self) -> Res<Group> {
        match &self.group.nodes {
            NodeGroup::Node(group) => match &group.0[self.pos] {
                Node::Element((_, x, _)) => Ok(Group {
                    domain: self.group.domain.clone(),
                    nodes: NodeGroup::Attr(AttrList(x.clone())),
                }),
                _ => nores(),
            },
            _ => nores(),
        }
    }
}

impl GroupTrait for Group {
    type Cell = Cell;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: false,
        }
    }

    fn len(&self) -> usize {
        match &self.nodes {
            NodeGroup::Node(group) => group.0.len(),
            NodeGroup::Attr(group) => group.0.len(),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        if index >= self.len() {
            return nores();
        }
        Ok(Cell {
            group: self.clone(),
            pos: index,
        })
    }

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, key: S) -> Res<Cell> {
        let key = key.into();
        for i in 0..self.len() {
            if let Ok(cell) = self.at(i) {
                if let Ok(reader) = cell.read() {
                    if let Ok(k) = reader.label() {
                        if key == k {
                            return Ok(cell);
                        }
                    }
                }
            }
        }
        nores()
    }
}

impl From<XmlError> for HErr {
    fn from(e: XmlError) -> HErr {
        HErr::Xml(format!("{}", e))
    }
}
