use std::io::BufRead;
use std::rc::Rc;

use linkme::distributed_slice;
use quick_xml::{events::Event, Error as XmlError, Reader};

use crate::{
    base::{Cell as XCell, *},
    debug, guard_variant,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_XML: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["xml"],
    constructor: Cell::from_cell,
};

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
    nodes: NodeGroup,
    head: Option<Rc<(Cell, Relation)>>,
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
    Decl(Rc<Vec<Attribute>>), // version, encoding, standalone
    DocType(String),
    PI(String),
    Element((String, Rc<Vec<Attribute>>, String, Rc<Vec<Node>>)),
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

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let mut reader = Reader::from_str(cow.as_ref());
                let root = xml_to_node(&mut reader)?;
                Self::from_root_node(root, Some(cell))
            }
            "fs" => {
                let path = cell.as_file_path()?;
                let mut reader = Reader::from_file(path).map_err(HErr::from)?;
                let root = xml_to_node(&mut reader)?;
                Self::from_root_node(root, Some(cell))
            }
            _ => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let mut reader = Reader::from_str(cow.as_ref());
                let root = xml_to_node(&mut reader)?;
                Self::from_root_node(root, Some(cell)).map_err(|e| {
                    if e.kind == HErrKind::InvalidFormat {
                        noerr()
                    } else {
                        e
                    }
                })
            }
        }
    }

    fn from_root_node(root: Node, origin: Option<XCell>) -> Res<XCell> {
        let xml_cell = Cell {
            group: Group {
                nodes: NodeGroup::Node(NodeList(Rc::new(vec![root]))),
                head: None,
            },
            pos: 0,
        };
        Ok(new_cell(DynCell::from(xml_cell), origin))
    }
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
    let decoder = reader.decoder();

    loop {
        if !decoder.encoding().is_ascii_compatible() {
            return Err(deformed("not ascii compatible"));
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Decl(ref e)) => {
                counts.count_decl += 1;
                let mut attrs = vec![];
                let rawversion = e.version()?;
                let version = decoder.decode(rawversion.as_ref())?;
                attrs.push(Attribute::Attribute("version".into(), version.into()));
                if let Some(encoding) = e.encoding() {
                    let rawencoding = encoding?;
                    let encoding = decoder.decode(rawencoding.as_ref())?;
                    attrs.push(Attribute::Attribute("encoding".into(), encoding.into()));
                }
                if let Some(standalone) = e.standalone() {
                    let rawstandalone = standalone?;
                    let standalone = decoder.decode(rawstandalone.as_ref())?;
                    attrs.push(Attribute::Attribute("standalone".into(), standalone.into()));
                }
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(Node::Decl(Rc::new(attrs)));
            }
            Ok(Event::DocType(ref e)) => {
                counts.count_doc_type += 1;
                let doctype = e.unescape()?;
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(Node::DocType(doctype.into()));
            }
            Ok(Event::PI(ref e)) => {
                counts.count_pi += 1;
                let text = decoder.decode(e.as_ref())?;
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(Node::PI(text.into()));
            }
            Ok(Event::Start(ref e)) => {
                let mut attrs = vec![];
                for resa in e.attributes().with_checks(false) {
                    match resa {
                        Ok(a) => {
                            let key = decoder.decode(a.key.0)?;
                            let value = decoder.decode(&a.value)?;
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
                let text = decoder.decode(e.as_ref())?;
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(Node::Text(text.into()));
            }
            Ok(Event::Comment(e)) => {
                counts.count_comment += 1;
                let text = decoder.decode(e.as_ref())?;
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(Node::Comment(text.into()));
            }
            Ok(Event::CData(e)) => {
                counts.count_cdata += 1;
                let v = stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?;
                let mut u = e.to_vec();
                u.shrink_to_fit();
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(Node::CData(u));
            }
            Ok(Event::End(ref e)) => {
                counts.count_element += 1;
                let mut v = stack.pop().ok_or_else(|| faulterr("no element in stack"))?;
                v.shrink_to_fit();
                let mut a = attribute_stack
                    .pop()
                    .ok_or_else(|| faulterr("no element in attr stack"))?;
                a.shrink_to_fit();
                counts.count_attributes += a.len();
                let name = decoder.decode(e.name().0)?;
                let mut text = String::new();
                if let Some(Node::Text(t)) = v.first() {
                    if !t.trim().is_empty() {
                        // TODO: use a deque to avoid shifting elements on remove
                        text = guard_variant!(v.remove(0), Node::Text).unwrap();
                    }
                }
                let element = Node::Element((name.into(), Rc::new(a), text, Rc::new(v)));
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(element);
            }
            Ok(Event::Empty(ref _e)) => {
                // should not happed because we use reader.expand_empty_elements(true);
            }
            Err(err) => {
                let text = format!("{}", err);
                stack
                    .last_mut()
                    .ok_or_else(|| faulterr("no element in stack"))?
                    .push(Node::Error(text));
            }

            Ok(Event::Eof) => break,
            // _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    let v = stack.pop().ok_or_else(|| faulterr("no element in stack"))?;
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
                Node::Element((x, _, _, _)) => Ok(Value::Str(x.as_str())),
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
                Node::Document(_) => nores(),
                Node::Decl(_) => nores(),
                Node::DocType(x) => Ok(Value::Str(x.trim())),
                Node::PI(x) => Ok(Value::Str(x)),
                Node::Element((_, _, text, _)) => {
                    return Ok(Value::Str(text.as_str()));
                }
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

    fn interpretation(&self) -> &str {
        "xml"
    }

    fn ty(&self) -> Res<&str> {
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
                    nodes: NodeGroup::Node(NodeList(x.clone())),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                Node::Element((_, _, _, x)) => Ok(Group {
                    nodes: NodeGroup::Node(NodeList(x.clone())),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                _ => nores(),
            },
            _ => nores(),
        }
    }

    fn attr(&self) -> Res<Group> {
        match &self.group.nodes {
            NodeGroup::Node(group) => match &group.0[self.pos] {
                Node::Decl(x) => Ok(Group {
                    nodes: NodeGroup::Attr(AttrList(x.clone())),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                Node::Element((_, x, _, _)) => Ok(Group {
                    nodes: NodeGroup::Attr(AttrList(x.clone())),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                }),
                _ => nores(),
            },
            _ => nores(),
        }
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

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: false,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(match &self.nodes {
            NodeGroup::Node(group) => group.0.len(),
            NodeGroup::Attr(group) => group.0.len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        if index >= self.len()? {
            return nores();
        }
        Ok(Cell {
            group: self.clone(),
            pos: index,
        })
    }

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, key: S) -> Res<Cell> {
        let key = key.into();
        for i in 0..self.len()? {
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
        caused(HErrKind::InvalidFormat, "xml error", e)
    }
}
