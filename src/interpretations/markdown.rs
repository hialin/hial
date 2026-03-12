use comrak::{
    Arena, Options, parse_document,
    nodes::{AstNode, NodeValue},
};
use linkme::distributed_slice;
use std::io::Read;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
    utils::{
        ownrc::{OwnRc, ReadRc},
        ownrcutils::read,
    },
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
    Preamble {
        body: String,
    },
    Section {
        label: String,
        level: u8,
        body: String,
    },
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
    LevelAttr(usize),
}

#[derive(Debug)]
pub(crate) struct CellReader {
    data: ReadRc<Data>,
    kind: Kind,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    kind: Kind,
}

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
    Attrs(usize),
}

#[derive(Clone, Debug)]
struct HeadingInfo {
    label: String,
    level: u8,
    start_line: usize,
    end_line: usize,
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
        let headings = collect_headings(source);
        let line_starts = line_starts(source);
        let line_count = line_starts.len().saturating_sub(1);

        let mut roots = Vec::new();
        let mut entries = Vec::<Entry>::new();
        let mut stack: Vec<(usize, u8)> = Vec::new();

        let first_heading_start = headings.first().map(|h| h.start_line).unwrap_or(line_count + 1);
        let preamble = trim_newline_edges(slice_line_range(
            source,
            &line_starts,
            1,
            first_heading_start,
        ));
        if !preamble.is_empty() {
            roots.push(entries.len());
            entries.push(Entry {
                parent: None,
                kind: EntryKind::Preamble { body: preamble },
                children: vec![],
            });
        }

        for (idx, heading) in headings.iter().enumerate() {
            while let Some((_, parent_level)) = stack.last() {
                if *parent_level >= heading.level {
                    stack.pop();
                } else {
                    break;
                }
            }

            let next_heading_start = headings
                .get(idx + 1)
                .map(|h| h.start_line)
                .unwrap_or(line_count + 1);
            let body = trim_newline_edges(slice_line_range(
                source,
                &line_starts,
                heading.end_line + 1,
                next_heading_start,
            ));

            let entry_id = entries.len();
            let parent = stack.last().map(|(entry_id, _)| *entry_id);
            entries.push(Entry {
                parent,
                kind: EntryKind::Section {
                    label: heading.label.clone(),
                    level: heading.level,
                    body,
                },
                children: vec![],
            });

            if let Some(parent_id) = parent {
                entries[parent_id].children.push(entry_id);
            } else {
                roots.push(entry_id);
            }

            stack.push((entry_id, heading.level));
        }

        Self {
            source: source.to_string(),
            roots,
            entries,
        }
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match self.kind {
            Kind::Root => Ok("document"),
            Kind::Entry(entry_id) => match &self.data.entries.get(entry_id).ok_or_else(noerr)?.kind {
                EntryKind::Preamble { .. } => Ok("preamble"),
                EntryKind::Section { .. } => Ok("section"),
            },
            Kind::LevelAttr(_) => Ok("attribute"),
        }
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            Kind::Root => Ok(0),
            Kind::Entry(entry_id) => Ok(entry_id),
            Kind::LevelAttr(_) => Ok(0),
        }
    }

    fn label(&self) -> Res<Value<'_>> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Entry(entry_id) => match &self.data.entries.get(entry_id).ok_or_else(noerr)?.kind {
                EntryKind::Preamble { .. } => Ok(Value::Str("preamble")),
                EntryKind::Section { label, .. } => Ok(Value::Str(label.as_str())),
            },
            Kind::LevelAttr(_) => Ok(Value::Str("level")),
        }
    }

    fn value(&self) -> Res<Value<'_>> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Entry(entry_id) => match &self.data.entries.get(entry_id).ok_or_else(noerr)?.kind {
                EntryKind::Preamble { body } => Ok(Value::Str(body.as_str())),
                EntryKind::Section { body, .. } => Ok(Value::Str(body.as_str())),
            },
            Kind::LevelAttr(entry_id) => match &self.data.entries.get(entry_id).ok_or_else(noerr)?.kind {
                EntryKind::Section { level, .. } => Ok(Value::from(*level as usize)),
                EntryKind::Preamble { .. } => nores(),
            },
        }
    }

    fn serial(&self) -> Res<String> {
        match self.kind {
            Kind::Root => Ok(self.data.source.clone()),
            Kind::Entry(_) | Kind::LevelAttr(_) => nores(),
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
        Ok(CellWriter {
            kind: self.kind.clone(),
        })
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Entry(entry_id) => {
                let entry = read(&self.data)?.entries.get(entry_id).ok_or_else(noerr)?.clone();
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
            Kind::LevelAttr(entry_id) => Ok((
                Cell {
                    data: self.data.clone(),
                    kind: Kind::Entry(entry_id),
                },
                Relation::Attr,
            )),
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
            Kind::LevelAttr(_) => nores(),
        }
    }

    fn attr(&self) -> Res<Self::Group> {
        match self.kind {
            Kind::Entry(entry_id) => {
                let data = read(&self.data)?;
                let entry = data.entries.get(entry_id).ok_or_else(noerr)?;
                if matches!(entry.kind, EntryKind::Section { .. }) {
                    Ok(Group {
                        data: self.data.clone(),
                        kind: GroupKind::Attrs(entry_id),
                    })
                } else {
                    nores()
                }
            }
            Kind::Root | Kind::LevelAttr(_) => nores(),
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
        match self.kind {
            GroupKind::Root => Ok(read(&self.data)?.roots.len()),
            GroupKind::Children(entry_id) => Ok(read(&self.data)?
                .entries
                .get(entry_id)
                .ok_or_else(noerr)?
                .children
                .len()),
            GroupKind::Attrs(_) => Ok(1),
        }
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        match self.kind {
            GroupKind::Root => {
                let entry_id = *read(&self.data)?.roots.get(index).ok_or_else(noerr)?;
                Ok(Cell {
                    data: self.data.clone(),
                    kind: Kind::Entry(entry_id),
                })
            }
            GroupKind::Children(parent_id) => {
                let entry_id = *read(&self.data)?
                    .entries
                    .get(parent_id)
                    .ok_or_else(noerr)?
                    .children
                    .get(index)
                    .ok_or_else(noerr)?;
                Ok(Cell {
                    data: self.data.clone(),
                    kind: Kind::Entry(entry_id),
                })
            }
            GroupKind::Attrs(entry_id) => {
                if index != 0 {
                    return nores();
                }
                Ok(Cell {
                    data: self.data.clone(),
                    kind: Kind::LevelAttr(entry_id),
                })
            }
        }
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        let Value::Str(label) = label else {
            return nores();
        };

        let data = read(&self.data)?;
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
            GroupKind::Attrs(entry_id) => {
                if label == "level" {
                    return Ok(vec![Ok(Cell {
                        data: self.data.clone(),
                        kind: Kind::LevelAttr(entry_id),
                    })]
                    .into_iter());
                }
                vec![]
            }
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

fn entry_matches_label(entry: &Entry, label: &str) -> bool {
    match &entry.kind {
        EntryKind::Preamble { .. } => label == "preamble",
        EntryKind::Section { label: entry_label, .. } => entry_label == label,
    }
}

fn collect_headings(source: &str) -> Vec<HeadingInfo> {
    let arena = Arena::new();
    let root = parse_document(&arena, source, &Options::default());
    let mut headings = Vec::new();

    let mut child = root.first_child();
    while let Some(node) = child {
        if let Some(heading) = heading_from_node(node) {
            headings.push(heading);
        }
        child = node.next_sibling();
    }

    headings
}

fn heading_from_node(node: &AstNode<'_>) -> Option<HeadingInfo> {
    let data = node.data.borrow();
    let NodeValue::Heading(heading) = &data.value else {
        return None;
    };

    let label = collect_text(node).trim().to_string();
    Some(HeadingInfo {
        label,
        level: heading.level,
        start_line: data.sourcepos.start.line,
        end_line: data.sourcepos.end.line,
    })
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

fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in source.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts.push(source.len());
    starts
}

fn slice_line_range(source: &str, starts: &[usize], start_line: usize, end_line_exclusive: usize) -> String {
    if start_line >= end_line_exclusive {
        return String::new();
    }
    let fallback = source.len();
    let start = *starts
        .get(start_line.saturating_sub(1))
        .unwrap_or_else(|| starts.last().unwrap_or(&0));
    let end = *starts
        .get(end_line_exclusive.saturating_sub(1))
        .unwrap_or_else(|| starts.last().unwrap_or(&fallback));
    source[start..end].to_string()
}

fn trim_newline_edges(s: String) -> String {
    s.trim_matches(|c| c == '\n' || c == '\r').to_string()
}
