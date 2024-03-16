use std::ops::Range;

use linkme::distributed_slice;
use regex::Regex;

use crate::{
    api::*,
    implement_try_from_xell,
    utils::{
        ownrc::{OwnRc, ReadRc, WriteRc},
        ownrcutils::read,
    },
};

use self::interpretation::{CellReaderTrait, CellTrait, CellWriterTrait, GroupTrait, LabelType};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_URL: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value"],
    target_interpretations: &["regex"],
    constructor: Cell::from_cell,
};

#[derive(Debug)]
struct Data {
    matches: Vec<Match>,
}

#[derive(Debug)]
struct Match {
    text: String,
    captures: Vec<Capture>,
}

#[derive(Debug)]
enum Capture {
    NotMatched,
    Matched(Range<usize>),
}

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    data: OwnRc<Data>,
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
    Root,
    Match(usize),
    Capture(usize, usize),
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
    kind: Kind,
}

implement_try_from_xell!(Cell, Regex);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        let r = origin.read();
        let v = r.value()?;
        let text = v.as_cow_str().to_string();

        let Some(arg) = params.iter().next() else {
            return userres("regex requires a parameter");
        };

        let re = Regex::new(arg.0.as_cow_str().as_ref())
            .map_err(|e| caused(HErrKind::User, "bad regex", e))?;

        let mut data = Data { matches: vec![] };
        for (i, captures) in re.captures_iter(text.as_str()).enumerate() {
            let Some(first) = captures.get(0) else {
                return fault("regex match should have at least one capture");
            };
            let mut m = Match {
                text: first.as_str().to_string(),
                captures: vec![],
            };
            for capture in captures.iter().skip(1) {
                let c = match capture {
                    None => Capture::NotMatched {},
                    Some(capture) => Capture::Matched(
                        // relative to the start of the whole match
                        capture.start() - first.start()..capture.end() - first.start(),
                    ),
                };
                m.captures.push(c);
            }
            data.matches.push(m);
        }

        let root = Cell {
            data: OwnRc::new(data),
            kind: Kind::Root,
        };
        Ok(Xell::new_from(DynCell::from(root), Some(origin)))
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match self.kind {
            Kind::Root => Ok("root"),
            Kind::Match(i) => Ok("match"),
            Kind::Capture(i, j) => match self.data.matches.get(i).and_then(|m| m.captures.get(j)) {
                Some(Capture::NotMatched) => Ok("unmatched"),
                Some(Capture::Matched { .. }) => Ok("capture"),
                None => nores(),
            },
        }
    }

    fn value(&self) -> Res<Value> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Match(i) => {
                let Some(m) = &self.data.matches.get(i) else {
                    return nores();
                };
                Ok(Value::Str(&m.text))
            }
            Kind::Capture(i, j) => {
                let Some(m) = &self.data.matches.get(i) else {
                    return nores();
                };
                match &m.captures.get(j) {
                    Some(Capture::NotMatched) => nores(),
                    Some(Capture::Matched(range)) => Ok(Value::Str(&m.text[range.clone()])),
                    None => nores(),
                }
            }
        }
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            Kind::Root => Ok(0),
            Kind::Match(i) => Ok(i),
            Kind::Capture(_, j) => Ok(j),
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        userres("cannot set value of a regex cell")
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "regex"
    }

    fn read(&self) -> Res<CellReader> {
        Ok(CellReader {
            data: self
                .data
                .read()
                .ok_or_else(|| lockerr("cannot read matches"))?,
            kind: self.kind.clone(),
        })
    }

    fn write(&self) -> Res<CellWriter> {
        userres("cannot write a regex cell")
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.kind {
            Kind::Root => nores(),
            Kind::Match(_) => Ok((
                Cell {
                    data: self.data.clone(),
                    kind: Kind::Root,
                },
                Relation::Sub,
            )),
            Kind::Capture(i, _) => Ok((
                Cell {
                    data: self.data.clone(),
                    kind: Kind::Match(i),
                },
                Relation::Sub,
            )),
        }
    }

    fn sub(&self) -> Res<Self::Group> {
        match self.kind {
            Kind::Root => Ok(Group {
                data: self.data.clone(),
                kind: Kind::Root,
            }),
            Kind::Match(i) => Ok(Group {
                data: self.data.clone(),
                kind: Kind::Match(i),
            }),
            Kind::Capture(i, _) => nores(),
        }
    }

    fn attr(&self) -> Res<Self::Group> {
        nores()
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
        match self.kind {
            Kind::Root => Ok(read(&self.data)?.matches.len()),
            Kind::Match(i) => Ok(read(&self.data)?
                .matches
                .get(i)
                .map(|m| m.captures.len())
                .unwrap_or(0)),
            _ => nores(),
        }
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        match self.kind {
            Kind::Root => Ok(Cell {
                data: self.data.clone(),
                kind: Kind::Match(index),
            }),
            Kind::Match(i) => Ok(Cell {
                data: self.data.clone(),
                kind: Kind::Capture(i, index),
            }),
            _ => nores(),
        }
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        nores()
    }
}
