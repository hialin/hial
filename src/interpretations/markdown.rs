use comrak::{
    Arena, Options,
    nodes::{AstNode, NodeValue},
    parse_document,
};
use linkme::distributed_slice;
use std::io::Read;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
    utils::ownrc::{OwnRc, ReadRc},
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_MARKDOWN: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs", "http"],
    target_interpretations: &["markdown"],
    constructor: Cell::from_cell,
};

#[derive(Debug)]
struct Data {
    source: String,
    roots: Vec<usize>,
    entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
struct Entry {
    parent: Option<usize>,
    kind: EntryKind,
    children: Vec<usize>,
}

#[derive(Clone, Debug)]
enum EntryKind {
    Preamble,
    Section { label: String },
    Paragraph { value: String },
    Code { value: String },
}

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    data: OwnRc<Data>,
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
    Root,
    Entry(usize),
}

#[derive(Debug)]
pub(crate) struct CellReader {
    data: ReadRc<Data>,
    kind: Kind,
}

#[derive(Debug)]
pub(crate) struct CellWriter;

#[derive(Clone, Debug)]
pub(crate) struct Group {
    data: OwnRc<Data>,
    kind: GroupKind,
}

pub(crate) type CellIterator = std::vec::IntoIter<Res<Cell>>;

#[derive(Clone, Debug)]
enum GroupKind {
    Root,
    Children(usize),
}

#[derive(Clone, Debug)]
struct HeadingFrame {
    level: u8,
    entry_id: Option<usize>,
}

implement_try_from_xell!(Cell, Markdown);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, _: &ElevateParams) -> Res<Xell> {
        let reader = origin.read().err()?;
        let source = match reader.value()? {
            Value::Bytes => {
                let mut bytes = Vec::new();
                reader
                    .value_read()?
                    .read_to_end(&mut bytes)
                    .map_err(|e| caused(HErrKind::IO, "cannot read markdown bytes", e))?;
                String::from_utf8(bytes).map_err(|e| {
                    caused(
                        HErrKind::InvalidFormat,
                        "markdown interpretation requires utf-8 input",
                        e,
                    )
                })?
            }
            value => value.as_cow_str().into_owned(),
        };

        let markdown_cell = Cell {
            data: OwnRc::new(Data::from_source(&source)),
            kind: Kind::Root,
        };
        Ok(Xell::new_from(DynCell::from(markdown_cell), Some(origin)))
    }
}

impl Data {
    fn from_source(source: &str) -> Self {
        let arena = Arena::new();
        let root = parse_document(&arena, source, &Options::default());

        let mut data = Self {
            source: source.to_string(),
            roots: Vec::new(),
            entries: Vec::new(),
        };
        let mut headings: Vec<HeadingFrame> = Vec::new();
        let mut preamble_id = None;

        let mut child = root.first_child();
        while let Some(node) = child {
            if let Some((level, label)) = heading_from_node(node) {
                while headings.last().is_some_and(|parent| parent.level >= level) {
                    headings.pop();
                }

                let entry_id = if label.is_empty() {
                    None
                } else {
                    Some(data.push_section(nearest_section_parent(&headings), label))
                };
                headings.push(HeadingFrame { level, entry_id });
            } else if let Some(kind) = block_kind(node) {
                let parent = nearest_section_parent(&headings);
                let entry_id = match parent {
                    Some(parent_id) => data.push_child(parent_id, kind),
                    None => {
                        let preamble_id = *preamble_id.get_or_insert_with(|| data.push_preamble());
                        data.push_child(preamble_id, kind)
                    }
                };
                debug_assert!(data.entries.get(entry_id).is_some());
            }
            child = node.next_sibling();
        }

        data
    }

    fn push_root(&mut self, kind: EntryKind) -> usize {
        let entry_id = self.entries.len();
        self.entries.push(Entry {
            parent: None,
            kind,
            children: Vec::new(),
        });
        self.roots.push(entry_id);
        entry_id
    }

    fn push_child(&mut self, parent_id: usize, kind: EntryKind) -> usize {
        let entry_id = self.entries.len();
        self.entries.push(Entry {
            parent: Some(parent_id),
            kind,
            children: Vec::new(),
        });
        self.entries[parent_id].children.push(entry_id);
        entry_id
    }

    fn push_preamble(&mut self) -> usize {
        self.push_root(EntryKind::Preamble)
    }

    fn push_section(&mut self, parent_id: Option<usize>, label: String) -> usize {
        match parent_id {
            Some(parent_id) => self.push_child(parent_id, EntryKind::Section { label }),
            None => self.push_root(EntryKind::Section { label }),
        }
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match self.kind {
            Kind::Root => Ok("document"),
            Kind::Entry(entry_id) => match &self.data.entries.get(entry_id).ok_or_else(noerr)?.kind
            {
                EntryKind::Preamble => Ok("preamble"),
                EntryKind::Section { .. } => Ok("title"),
                EntryKind::Paragraph { .. } => Ok("text"),
                EntryKind::Code { .. } => Ok("code"),
            },
        }
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            Kind::Root => Ok(0),
            Kind::Entry(entry_id) => Ok(entry_id),
        }
    }

    fn label(&self) -> Res<Value<'_>> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Entry(entry_id) => match &self.data.entries.get(entry_id).ok_or_else(noerr)?.kind
            {
                EntryKind::Preamble => Ok(Value::Str("preamble")),
                EntryKind::Section { label } => Ok(Value::Str(label.as_str())),
                EntryKind::Paragraph { .. } | EntryKind::Code { .. } => nores(),
            },
        }
    }

    fn value(&self) -> Res<Value<'_>> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Entry(entry_id) => match &self.data.entries.get(entry_id).ok_or_else(noerr)?.kind
            {
                EntryKind::Paragraph { value } | EntryKind::Code { value } => {
                    Ok(Value::Str(value.as_str()))
                }
                EntryKind::Preamble | EntryKind::Section { .. } => nores(),
            },
        }
    }

    fn serial(&self) -> Res<String> {
        match self.kind {
            Kind::Root => Ok(self.data.source.clone()),
            Kind::Entry(_) => nores(),
        }
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, _: OwnValue) -> Res<()> {
        nores()
    }

    fn set_label(&mut self, _: OwnValue) -> Res<()> {
        nores()
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "markdown"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            data: self
                .data
                .read()
                .ok_or_else(|| lockerr("cannot read markdown"))?,
            kind: self.kind.clone(),
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter)
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Entry(entry_id) => {
                let entry = self
                    .data
                    .read()
                    .ok_or_else(|| lockerr("cannot read markdown"))?
                    .entries
                    .get(entry_id)
                    .ok_or_else(noerr)?
                    .clone();
                match entry.parent {
                    Some(parent_id) => Ok((
                        Cell {
                            data: self.data.clone(),
                            kind: Kind::Entry(parent_id),
                        },
                        Relation::Sub,
                    )),
                    None => Ok((
                        Cell {
                            data: self.data.clone(),
                            kind: Kind::Root,
                        },
                        Relation::Sub,
                    )),
                }
            }
        }
    }

    fn sub(&self) -> Res<Self::Group> {
        match self.kind {
            Kind::Root => Ok(Group {
                data: self.data.clone(),
                kind: GroupKind::Root,
            }),
            Kind::Entry(entry_id) => Ok(Group {
                data: self.data.clone(),
                kind: GroupKind::Children(entry_id),
            }),
        }
    }

    fn attr(&self) -> Res<Self::Group> {
        nores()
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
        match self.kind {
            GroupKind::Root => Ok(self
                .data
                .read()
                .ok_or_else(|| lockerr("cannot read markdown"))?
                .roots
                .len()),
            GroupKind::Children(entry_id) => Ok(self
                .data
                .read()
                .ok_or_else(|| lockerr("cannot read markdown"))?
                .entries
                .get(entry_id)
                .ok_or_else(noerr)?
                .children
                .len()),
        }
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        let data = self
            .data
            .read()
            .ok_or_else(|| lockerr("cannot read markdown"))?;
        let entry_id = match self.kind {
            GroupKind::Root => *data.roots.get(index).ok_or_else(noerr)?,
            GroupKind::Children(parent_id) => *data
                .entries
                .get(parent_id)
                .ok_or_else(noerr)?
                .children
                .get(index)
                .ok_or_else(noerr)?,
        };
        Ok(Cell {
            data: self.data.clone(),
            kind: Kind::Entry(entry_id),
        })
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        let Value::Str(label) = label else {
            return nores();
        };

        let data = self
            .data
            .read()
            .ok_or_else(|| lockerr("cannot read markdown"))?;
        let ids: Vec<usize> = match self.kind {
            GroupKind::Root => data
                .roots
                .iter()
                .copied()
                .filter(|entry_id| entry_matches_label(&data.entries[*entry_id], label))
                .collect(),
            GroupKind::Children(parent_id) => data
                .entries
                .get(parent_id)
                .ok_or_else(noerr)?
                .children
                .iter()
                .copied()
                .filter(|entry_id| entry_matches_label(&data.entries[*entry_id], label))
                .collect(),
        };

        Ok(ids
            .into_iter()
            .map(|entry_id| {
                Ok(Cell {
                    data: self.data.clone(),
                    kind: Kind::Entry(entry_id),
                })
            })
            .collect::<Vec<_>>()
            .into_iter())
    }
}

fn nearest_section_parent(headings: &[HeadingFrame]) -> Option<usize> {
    headings.iter().rev().find_map(|frame| frame.entry_id)
}

fn block_kind(node: &AstNode<'_>) -> Option<EntryKind> {
    let value = node.data.borrow().value.clone();
    match value {
        NodeValue::Paragraph => {
            let value = collect_text(node).trim().to_string();
            if value.is_empty() {
                None
            } else {
                Some(EntryKind::Paragraph { value })
            }
        }
        NodeValue::CodeBlock(code) => {
            let value = trim_newline_edges(code.literal);
            if value.is_empty() {
                None
            } else {
                Some(EntryKind::Code { value })
            }
        }
        _ => None,
    }
}

fn entry_matches_label(entry: &Entry, label: &str) -> bool {
    match &entry.kind {
        EntryKind::Preamble => label == "preamble",
        EntryKind::Section { label: entry_label } => entry_label == label,
        EntryKind::Paragraph { .. } | EntryKind::Code { .. } => false,
    }
}

fn heading_from_node(node: &AstNode<'_>) -> Option<(u8, String)> {
    let data = node.data.borrow();
    let NodeValue::Heading(heading) = &data.value else {
        return None;
    };
    Some((heading.level, collect_text(node).trim().to_string()))
}

fn collect_text(node: &AstNode<'_>) -> String {
    let mut text = String::new();
    collect_text_into(node, &mut text);
    text
}

fn collect_text_into(node: &AstNode<'_>, text: &mut String) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(t) => text.push_str(t),
        NodeValue::Code(code) => text.push_str(&code.literal),
        NodeValue::LineBreak | NodeValue::SoftBreak => text.push(' '),
        _ => {}
    }
    drop(data);

    let mut child = node.first_child();
    while let Some(next) = child {
        collect_text_into(next, text);
        child = next.next_sibling();
    }
}

fn trim_newline_edges(s: String) -> String {
    s.trim_matches(|c| c == '\n' || c == '\r').to_string()
}
