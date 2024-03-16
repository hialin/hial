use linkme::distributed_slice;
use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, Event},
    Error as XmlError, Reader,
};
use std::{fs::File, io::BufRead, rc::Rc};

use crate::{
    api::{interpretation::*, *},
    debug, guard_variant, implement_try_from_xell,
    utils::{
        indentation::{detect_file_indentation, detect_indentation},
        ownrc::{OwnRc, ReadRc, WriteRc},
        ownrcutils::read,
    },
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_XML: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["xml"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct CellIterator {
    group: Group,
    next_pos: usize,
    next_back_pos: usize,
    key: OwnValue,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    nodes: NodeGroup,
    head: Option<Rc<(Cell, Relation)>>,
    indent: Rc<String>,
}

#[derive(Clone, Debug)]
enum NodeGroup {
    Node(OwnRc<Vec<Node>>),
    Attr(OwnRc<Vec<Attribute>>),
}

#[derive(Debug)]
pub(crate) enum CellReader {
    Node {
        nodes: ReadRc<Vec<Node>>,
        pos: usize,
        indent: Rc<String>,
    },
    Attr {
        nodes: ReadRc<Vec<Attribute>>,
        pos: usize,
        indent: Rc<String>,
    },
}

#[derive(Debug)]
pub(crate) enum CellWriter {
    Node {
        nodes: WriteRc<Vec<Node>>,
        pos: usize,
    },
    Attr {
        nodes: WriteRc<Vec<Attribute>>,
        pos: usize,
    },
}

#[derive(Debug)]
pub(crate) enum Node {
    Document(OwnRc<Vec<Node>>),
    Decl(OwnRc<Vec<Attribute>>), // version, encoding, standalone
    DocType(String),
    PI(String),
    Element((String, OwnRc<Vec<Attribute>>, String, OwnRc<Vec<Node>>)),
    Text(String),
    Comment(String),
    CData(Vec<u8>),
    Error(String),
}

#[derive(Debug)]
pub(crate) enum Attribute {
    Attribute(String, String),
    Error(String),
}

implement_try_from_xell!(Cell, Xml);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        match origin.interpretation() {
            "fs" => {
                let r = origin.read();
                let path = r.as_file_path()?;
                let file = File::open(path).map_err(|e| {
                    caused(
                        HErrKind::InvalidFormat,
                        format!("cannot open file {:?}", path),
                        e,
                    )
                })?;
                let indent = detect_file_indentation(path);
                let mut reader = Reader::from_file(path).map_err(HErr::from)?;
                let root = xml_to_node(&mut reader)?;
                Self::from_root_node(root, Some(origin), indent)
            }
            _ => {
                let r = origin.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let indent = detect_indentation(cow.as_ref());
                let mut reader = Reader::from_str(cow.as_ref());
                let root = xml_to_node(&mut reader)?;
                Self::from_root_node(root, Some(origin), indent)
            }
        }
    }

    fn from_root_node(root: Node, origin: Option<Xell>, indent: String) -> Res<Xell> {
        let xml_cell = Cell {
            group: Group {
                nodes: NodeGroup::Node(OwnRc::new(vec![root])),
                head: None,
                indent: Rc::new(indent),
            },
            pos: 0,
        };
        Ok(Xell::new_from(DynCell::from(xml_cell), origin))
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

    fn last(stack: &mut [Vec<Node>]) -> Res<&mut Vec<Node>> {
        stack
            .last_mut()
            .ok_or_else(|| faulterr("no element in stack"))
    }

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
                last(&mut stack)?.push(Node::Decl(OwnRc::new(attrs)));
            }
            Ok(Event::DocType(ref e)) => {
                counts.count_doc_type += 1;
                let doctype = e.unescape()?;
                last(&mut stack)?.push(Node::DocType(doctype.into()));
            }
            Ok(Event::PI(ref e)) => {
                counts.count_pi += 1;
                let text = decoder.decode(e)?;
                last(&mut stack)?.push(Node::PI(text.into()));
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
                last(&mut stack)?.push(Node::Text(text.into()));
            }
            Ok(Event::Comment(e)) => {
                counts.count_comment += 1;
                let text = decoder.decode(e.as_ref())?;
                last(&mut stack)?.push(Node::Comment(text.into()));
            }
            Ok(Event::CData(e)) => {
                counts.count_cdata += 1;
                let mut u = e.to_vec();
                u.shrink_to_fit();
                last(&mut stack)?.push(Node::CData(u));
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
                let element = Node::Element((name.into(), OwnRc::new(a), text, OwnRc::new(v)));
                last(&mut stack)?.push(element);
            }
            Ok(Event::Empty(ref _e)) => {
                // should not happed because we use reader.expand_empty_elements(true);
            }
            Err(err) => {
                let text = format!("{}", err);
                last(&mut stack)?.push(Node::Error(text));
            }

            Ok(Event::Eof) => break,
            // _ => (), // There are several other `Event`s we do not consider here
        }

        // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
        buf.clear();
    }
    let v = stack.pop().ok_or_else(|| faulterr("no element in stack"))?;
    let document = Node::Document(OwnRc::new(v));

    debug!("xml stats: {:?}", counts);
    Ok(document)
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        Ok(match &self {
            CellReader::Node { nodes, pos, .. } => match &nodes[*pos] {
                Node::Document(_) => "document",
                Node::Decl(_) => "decl",
                Node::DocType(_) => "doctype",
                Node::PI(_) => "PI",
                Node::Element(_) => "element",
                Node::Text(_) => "text",
                Node::Comment(_) => "comment",
                Node::CData(_) => "cdata",
                Node::Error(_) => "error",
            },
            CellReader::Attr { nodes, pos, .. } => match &nodes[*pos] {
                Attribute::Attribute(_, _) => "attribute",
                Attribute::Error(_) => "error",
            },
        })
    }

    fn index(&self) -> Res<usize> {
        match *self {
            CellReader::Node { pos, .. } => Ok(pos),
            CellReader::Attr { pos, .. } => Ok(pos),
        }
    }

    fn label(&self) -> Res<Value> {
        match self {
            CellReader::Node { nodes, pos, .. } => match &nodes[*pos] {
                Node::Element((x, _, _, _)) => Ok(Value::Str(x.as_str())),
                Node::Decl(_) => Ok(Value::Str("xml")),
                Node::DocType(x) => Ok(Value::Str("DOCTYPE")),
                x => nores(),
            },
            CellReader::Attr { nodes, pos, .. } => match &nodes[*pos] {
                Attribute::Attribute(k, _) => Ok(Value::Str(k.as_str())),
                _ => nores(),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self {
            CellReader::Node { nodes, pos, .. } => match &nodes[*pos] {
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
            CellReader::Attr { nodes, pos, .. } => match &nodes[*pos] {
                Attribute::Attribute(_, x) => Ok(Value::Str(x)),
                Attribute::Error(x) => Ok(Value::Str(x)),
            },
        }
    }

    fn serial(&self) -> Res<String> {
        use quick_xml::{
            events::{BytesCData, BytesText},
            writer::Writer,
        };
        use std::io::Cursor;

        fn write_err(e: impl std::error::Error + 'static) -> HErr {
            caused(HErrKind::InvalidFormat, "xml serialization error", e)
        }

        fn serialize_node(writer: &mut Writer<Cursor<Vec<u8>>>, node: &Node) -> Res<()> {
            match node {
                Node::CData(data) => {
                    let s = String::from_utf8(data.clone()).map_err(write_err)?;
                    writer
                        .write_event(Event::CData(BytesCData::new(s)))
                        .map_err(write_err)?
                }
                Node::Comment(s) => writer
                    .write_event(Event::Comment(BytesText::new(s)))
                    .map_err(write_err)?,
                Node::Decl(attrs) => {
                    let attrs = read(attrs)?;
                    let version = attrs.iter().find_map(|attr| match attr {
                        Attribute::Attribute(k, v) if k == "version" => Some(v.as_str()),
                        _ => None,
                    });
                    let encoding = attrs.iter().find_map(|attr| match attr {
                        Attribute::Attribute(k, v) if k == "encoding" => Some(v.as_str()),
                        _ => None,
                    });
                    let standalone = attrs.iter().find_map(|attr| match attr {
                        Attribute::Attribute(k, v) if k == "standalone" => Some(v.as_str()),
                        _ => None,
                    });
                    if let Some(version) = version {
                        writer
                            .write_event(Event::Decl(BytesDecl::new(version, encoding, standalone)))
                            .map_err(write_err)?;
                    }
                }
                Node::DocType(s) => writer
                    .write_event(Event::DocType(BytesText::new(s.as_str())))
                    .map_err(write_err)?,
                Node::Document(nodes) => {
                    for n in read(nodes)?.iter() {
                        serialize_node(writer, n)?;
                    }
                }
                Node::Element((name, attrs, text, nodes)) => {
                    let mut bs = BytesStart::new(name);
                    {
                        for a in read(attrs)?.iter() {
                            match a {
                                Attribute::Attribute(k, v) => {
                                    bs.push_attribute((k.as_str(), v.as_str()));
                                }
                                Attribute::Error(e) => bs.push_attribute(("ERROR", e.as_str())),
                            }
                        }
                    }

                    if text.is_empty() && read(nodes)?.is_empty() {
                        writer.write_event(Event::Empty(bs)).map_err(write_err)?;
                        return Ok(());
                    }

                    writer.write_event(Event::Start(bs)).map_err(write_err)?;
                    if !text.is_empty() {
                        writer
                            .write_event(Event::Text(BytesText::new(text.as_str())))
                            .map_err(write_err)?;
                    }
                    {
                        for n in read(nodes)?.iter() {
                            serialize_node(writer, n)?;
                        }
                    }
                    writer
                        .write_event(Event::End(BytesEnd::new(name.as_str())))
                        .map_err(write_err)?;
                }
                Node::Error(s) => writer
                    .write_event(Event::Comment(BytesText::new(&format!("ERROR: {}", s))))
                    .map_err(write_err)?,
                Node::PI(s) => writer
                    .write_event(Event::PI(BytesText::new(s.as_str())))
                    .map_err(write_err)?,
                Node::Text(s) => writer
                    .write_event(Event::Text(BytesText::new(s)))
                    .map_err(write_err)?,
            }
            Ok(())
        }

        match self {
            CellReader::Node { nodes, pos, indent } => {
                let mut writer = if indent.is_empty() {
                    Writer::new(Cursor::new(Vec::new()))
                } else {
                    let first = indent.chars().next().unwrap();
                    Writer::new_with_indent(Cursor::new(Vec::new()), first as u8, indent.len())
                };
                serialize_node(&mut writer, &nodes[*pos])?;
                let ser = writer.into_inner().into_inner();
                String::from_utf8(ser).map_err(|e| {
                    caused(
                        HErrKind::InvalidFormat,
                        "invalid utf8 in xml serialization",
                        e,
                    )
                })
            }
            CellReader::Attr { nodes, pos, .. } => match &nodes[*pos] {
                Attribute::Attribute(k, v) => Ok(format!("{}=\"{}\"", k, v)),
                Attribute::Error(x) => Err(deformed(x)),
            },
        }
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self {
            CellWriter::Node { nodes, pos } => match &mut nodes[*pos] {
                Node::Document(_) => return userres("cannot set value of document"),
                Node::Decl(x) => return userres("cannot set value of decl"),
                Node::DocType(x) => *x = value.as_value().as_cow_str().to_string(),
                Node::PI(x) => *x = value.as_value().as_cow_str().to_string(),
                Node::Element((_, _, x, _)) => *x = value.as_value().as_cow_str().to_string(),

                Node::Text(x) => *x = value.as_value().as_cow_str().to_string(),
                Node::Comment(x) => *x = value.as_value().as_cow_str().to_string(),
                Node::CData(x) => {
                    if let OwnValue::Bytes(b) = value {
                        *x = b;
                    } else {
                        *x = value.as_value().as_cow_str().to_string().into_bytes();
                    }
                }
                Node::Error(x) => return userres("cannot set value of error node"),
            },
            CellWriter::Attr { nodes, pos } => match &mut nodes[*pos] {
                Attribute::Attribute(_, x) => *x = value.as_value().as_cow_str().to_string(),
                Attribute::Error(x) => return userres("cannot set value of error attribute"),
            },
        }
        Ok(())
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "xml"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(match &self.group.nodes {
            NodeGroup::Node(n) => CellReader::Node {
                nodes: n.read().ok_or_else(|| lockerr("cannot read nodes"))?,
                pos: self.pos,
                indent: Rc::clone(&self.group.indent),
            },
            NodeGroup::Attr(a) => CellReader::Attr {
                nodes: a.read().ok_or_else(|| lockerr("cannot read nodes"))?,
                pos: self.pos,
                indent: Rc::clone(&self.group.indent),
            },
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(match &self.group.nodes {
            NodeGroup::Node(n) => CellWriter::Node {
                nodes: n.write().ok_or_else(|| lockerr("cannot write nodes"))?,
                pos: self.pos,
            },
            NodeGroup::Attr(a) => CellWriter::Attr {
                nodes: a.write().ok_or_else(|| lockerr("cannot write nodes"))?,
                pos: self.pos,
            },
        })
    }

    fn sub(&self) -> Res<Group> {
        match &self.group.nodes {
            NodeGroup::Node(n) => {
                let nodes = n.read().ok_or_else(|| lockerr("cannot read nodes"))?;
                match &nodes[self.pos] {
                    Node::Document(x) => Ok(Group {
                        nodes: NodeGroup::Node(x.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                        indent: Rc::clone(&self.group.indent),
                    }),
                    Node::Element((_, _, _, x)) => Ok(Group {
                        nodes: NodeGroup::Node(x.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                        indent: Rc::clone(&self.group.indent),
                    }),
                    _ => nores(),
                }
            }
            _ => nores(),
        }
    }

    fn attr(&self) -> Res<Group> {
        match &self.group.nodes {
            NodeGroup::Node(n) => {
                let nodes = n.read().ok_or_else(|| lockerr("cannot read nodes"))?;
                match &nodes[self.pos] {
                    Node::Decl(x) => Ok(Group {
                        nodes: NodeGroup::Attr(x.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                        indent: Rc::clone(&self.group.indent),
                    }),
                    Node::Element((_, x, _, _)) => Ok(Group {
                        nodes: NodeGroup::Attr(x.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                        indent: Rc::clone(&self.group.indent),
                    }),
                    _ => nores(),
                }
            }
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

impl Iterator for CellIterator {
    type Item = Res<Cell>;

    fn next(&mut self) -> Option<Self::Item> {
        fn inner(this: &mut CellIterator) -> Res<Cell> {
            loop {
                let cell = this.group.at(this.next_pos)?;
                this.next_pos += 1;
                let reader = cell.read()?;
                if Some(this.key.as_value()) == reader.label().ok() {
                    return Ok(cell);
                }
            }
        }
        match inner(self) {
            Ok(cell) => Some(Ok(cell)),
            Err(e) => {
                if e.kind == HErrKind::None {
                    None
                } else {
                    Some(Err(e))
                }
            }
        }
    }
}
impl DoubleEndedIterator for CellIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        fn inner(this: &mut CellIterator) -> Res<Cell> {
            loop {
                if this.next_back_pos == 0 {
                    return nores();
                }
                this.next_back_pos -= 1;
                let cell = this.group.at(this.next_back_pos)?;
                let reader = cell.read()?;
                if Some(this.key.as_value()) == reader.label().ok() {
                    return Ok(cell);
                }
            }
        }
        match inner(self) {
            Ok(cell) => Some(Ok(cell)),
            Err(e) => {
                if e.kind == HErrKind::None {
                    None
                } else {
                    Some(Err(e))
                }
            }
        }
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = CellIterator;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: false,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(match &self.nodes {
            NodeGroup::Node(group) => group
                .read()
                .ok_or_else(|| lockerr("cannot read nodes"))?
                .len(),
            NodeGroup::Attr(group) => group
                .read()
                .ok_or_else(|| lockerr("cannot read nodes"))?
                .len(),
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

    fn get_all(&self, key: Value) -> Res<CellIterator> {
        Ok(CellIterator {
            group: self.clone(),
            next_pos: 0,
            next_back_pos: self.len()?,
            key: key.to_owned_value(),
        })
    }
}

impl From<XmlError> for HErr {
    fn from(e: XmlError) -> HErr {
        caused(HErrKind::InvalidFormat, "xml error", e)
    }
}
