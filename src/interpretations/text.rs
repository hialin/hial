use linkme::distributed_slice;
use std::io::Read;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
    utils::{
        ownrc::{OwnRc, ReadRc, WriteRc},
        ownrcutils::read,
    },
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_TEXT: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs", "http"],
    target_interpretations: &["text"],
    constructor: Cell::from_cell,
};

#[derive(Debug)]
struct Data {
    lines: Vec<String>,
    newline: String,
    trailing_newline: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    data: OwnRc<Data>,
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
    Root,
    Line(usize),
}

#[derive(Debug)]
pub(crate) struct CellReader {
    data: ReadRc<Data>,
    kind: Kind,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    data: WriteRc<Data>,
    kind: Kind,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    data: OwnRc<Data>,
}

implement_try_from_xell!(Cell, Text);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, _: &ElevateParams) -> Res<Xell> {
        let reader = origin.read().err()?;
        let text = match reader.value()? {
            Value::Bytes => {
                let mut bytes = Vec::new();
                reader
                    .value_read()?
                    .read_to_end(&mut bytes)
                    .map_err(|e| caused(HErrKind::IO, "cannot read text bytes", e))?;
                String::from_utf8(bytes).map_err(|e| {
                    caused(
                        HErrKind::InvalidFormat,
                        "text interpretation requires utf-8 input",
                        e,
                    )
                })?
            }
            value => value.as_cow_str().into_owned(),
        };

        let text_cell = Cell {
            data: OwnRc::new(Data::from_text(&text)),
            kind: Kind::Root,
        };
        Ok(Xell::new_from(DynCell::from(text_cell), Some(origin)))
    }
}

impl Data {
    fn from_text(text: &str) -> Self {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" }.to_string();
        let trailing_newline = text.ends_with("\n");
        let normalized = text.replace("\r\n", "\n");
        let lines = if normalized.is_empty() {
            vec![]
        } else {
            let mut lines = normalized
                .split('\n')
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            if trailing_newline {
                lines.pop();
            }
            lines
        };

        Self {
            lines,
            newline,
            trailing_newline,
        }
    }

    fn serialize(&self) -> String {
        let mut serial = self.lines.join(self.newline.as_str());
        if self.trailing_newline && !self.lines.is_empty() {
            serial.push_str(self.newline.as_str());
        }
        serial
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match self.kind {
            Kind::Root => Ok("document"),
            Kind::Line(_) => Ok("line"),
        }
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            Kind::Root => Ok(0),
            Kind::Line(i) => Ok(i),
        }
    }

    fn label(&self) -> Res<Value<'_>> {
        nores()
    }

    fn value(&self) -> Res<Value<'_>> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Line(i) => self
                .data
                .lines
                .get(i)
                .map(|line| Value::from(line.as_str()))
                .ok_or_else(noerr),
        }
    }

    fn serial(&self) -> Res<String> {
        match self.kind {
            Kind::Root => Ok(self.data.serialize()),
            Kind::Line(_) => nores(),
        }
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Line(i) => {
                let Some(line) = self.data.lines.get_mut(i) else {
                    return nores();
                };
                *line = value.as_cow_str().into_owned();
                Ok(())
            }
        }
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "text"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            data: self
                .data
                .read()
                .ok_or_else(|| lockerr("cannot read text"))?,
            kind: self.kind.clone(),
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            data: self
                .data
                .write()
                .ok_or_else(|| lockerr("cannot write text"))?,
            kind: self.kind.clone(),
        })
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Line(_) => Ok((
                Cell {
                    data: self.data.clone(),
                    kind: Kind::Root,
                },
                Relation::Sub,
            )),
        }
    }

    fn sub(&self) -> Res<Self::Group> {
        match self.kind {
            Kind::Root => Ok(Group {
                data: self.data.clone(),
            }),
            Kind::Line(_) => nores(),
        }
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Empty<Res<Self::Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(read(&self.data)?.lines.len())
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        if index >= read(&self.data)?.lines.len() {
            return nores();
        }
        Ok(Cell {
            data: self.data.clone(),
            kind: Kind::Line(index),
        })
    }

    fn get_all(&self, _: Value<'_>) -> Res<Self::CellIterator> {
        nores()
    }
}
