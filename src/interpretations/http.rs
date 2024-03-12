use indexmap::IndexMap;
use linkme::distributed_slice;
use reqwest::{blocking::Client, Error as ReqwestError};

use crate::{
    api::{interpretation::*, *},
    utils::ownrc::*,
    warning,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static URL_TO_HTTP: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value"],
    target_interpretations: &["http"],
    constructor: Cell::from_cell,
};

// ^http .value -> bytes
//       @status
//          code
//          reason
//       @headers/...

#[derive(Debug)]
pub(crate) struct Response {
    status: i16,
    reason: String,
    headers: IndexMap<String, Vec<String>>,
    body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Debug)]
pub(crate) struct CellReader {
    kind: GroupKind,
    response: ReadRc<Response>,
    pos: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum GroupKind {
    Root,
    Attr,
    Status,
    Headers,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    kind: GroupKind,
    response: OwnRc<Response>,
}

#[derive(Debug)]
pub(crate) struct CellWriter {}

impl Cell {
    pub(crate) fn from_cell(cell: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        let reader = cell.read().err()?;
        let value = reader.value()?;
        let value_cow = value.as_cow_str();
        let url = value_cow.as_ref();

        let response = Client::builder()
            .user_agent("hial")
            .build()?
            .get(url)
            .send()?;

        let mut headers = IndexMap::<String, Vec<String>>::new();
        for (k, v) in response.headers().iter() {
            let valueheader = v.to_str().map_or(String::from("<blob>"), |x| x.to_string());
            if let Some(header) = headers.get_mut(k.as_str()) {
                header.push(valueheader);
            } else {
                headers.insert(k.as_str().to_string(), vec![valueheader]);
            }
        }
        let status = response.status().as_u16() as i16;
        let reason = response
            .status()
            .canonical_reason()
            .unwrap_or("")
            .to_string();
        if status >= 400 {
            warning!("Error: http call failed: {} = {} {}", url, status, reason);
        }
        let response = OwnRc::new(Response {
            status,
            reason,
            headers,
            body: response.bytes()?.as_ref().to_vec(),
        });

        let http_cell = Cell {
            group: Group {
                kind: GroupKind::Root,
                response,
            },
            pos: 0,
        };
        Ok(new_xell(DynCell::from(http_cell), Some(cell)))
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "http"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            kind: self.group.kind,
            response: self
                .group
                .response
                .read()
                .ok_or_else(|| lockerr("cannot read cell"))?,
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }

    fn sub(&self) -> Res<Group> {
        if self.group.kind == GroupKind::Attr {
            let mut group = self.group.clone();
            if self.pos == 0 {
                group.kind = GroupKind::Status;
            } else {
                group.kind = GroupKind::Headers;
            }
            Ok(group)
        } else {
            nores()
        }
    }

    fn attr(&self) -> Res<Group> {
        if self.group.kind == GroupKind::Root && self.pos == 0 {
            let mut group = self.group.clone();
            group.kind = GroupKind::Attr;
            Ok(group)
        } else {
            nores()
        }
    }

    fn head(&self) -> Res<(Self, Relation)> {
        let mut cell = Cell {
            group: Group {
                kind: GroupKind::Root,
                response: self.group.response.clone(),
            },
            pos: 0,
        };
        let mut rel = Relation::Sub;
        match self.group.kind {
            GroupKind::Root => {
                return nores();
            }
            GroupKind::Attr => {
                cell.group.kind = GroupKind::Root;
                rel = Relation::Attr;
            }
            GroupKind::Status => {
                cell.group.kind = GroupKind::Attr;
            }
            GroupKind::Headers => {
                cell.group.kind = GroupKind::Attr;
                cell.pos = 1;
            }
        }
        Ok((cell, rel))
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match (&self.kind, self.pos) {
            (GroupKind::Root, _) => Ok("body"),
            (GroupKind::Attr, 0) => Ok(""),
            (GroupKind::Attr, 1) => Ok(""),
            (GroupKind::Status, 0) => Ok("int"),
            (GroupKind::Status, 1) => Ok("string"),
            (GroupKind::Headers, _) => Ok("header"),
            _ => Ok(""),
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value> {
        match (&self.kind, self.pos) {
            (GroupKind::Root, _) => nores(),
            (GroupKind::Attr, 0) => Ok(Value::Str("status")),
            (GroupKind::Attr, 1) => Ok(Value::Str("headers")),
            (GroupKind::Status, 0) => Ok(Value::Str("code")),
            (GroupKind::Status, 1) => Ok(Value::Str("reason")),
            (GroupKind::Headers, _) => {
                if let Some((k, _)) = self.response.headers.get_index(self.pos) {
                    return Ok(Value::Str(k));
                }
                fault(format!("bad pos in headers: {}", self.pos))
            }
            _ => fault(format!("bad kind/pos: {:?}/{}", self.kind, self.pos)),
        }
    }

    fn value(&self) -> Res<Value> {
        match (&self.kind, self.pos) {
            (GroupKind::Root, 0) => Ok(Value::Bytes(&self.response.body)),
            (GroupKind::Attr, 0) => nores(),
            (GroupKind::Attr, 1) => nores(),
            (GroupKind::Status, 0) => Ok(Value::Int(Int::I32(self.response.status as i32))),
            (GroupKind::Status, 1) => Ok(Value::Str(&self.response.reason)),
            (GroupKind::Headers, _) => {
                let header_values = if let Some(hv) = self.response.headers.get_index(self.pos) {
                    hv.1
                } else {
                    return fault(format!("bad pos in headers: {}", self.pos));
                };
                Ok(Value::Str(header_values[0].as_str()))
            }
            _ => fault(format!("bad kind/pos: {:?}/{}", self.kind, self.pos)),
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        fault("set_value not yet implemented for http")
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Once<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: self.kind != GroupKind::Headers,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(match self.kind {
            GroupKind::Root => 0,
            GroupKind::Attr => 2,
            GroupKind::Status => 2,
            GroupKind::Headers => self
                .response
                .read()
                .ok_or_else(|| lockerr("cannot read group"))?
                .headers
                .len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match (self.kind, index) {
            (GroupKind::Attr, i) if i < 2 => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            (GroupKind::Status, i) if i < 2 => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            (GroupKind::Headers, i)
                if i < self
                    .response
                    .read()
                    .ok_or_else(|| lockerr("cannot read group"))?
                    .headers
                    .len() =>
            {
                Ok(Cell {
                    group: self.clone(),
                    pos: index,
                })
            }
            _ => nores(),
        }
    }

    fn get_all(&self, key: Value) -> Res<Self::CellIterator> {
        let cell = match (self.kind, key) {
            (GroupKind::Attr, sel) if sel == "status" => Ok(Cell {
                group: self.clone(),
                pos: 0,
            }),
            (GroupKind::Attr, sel) if sel == "headers" => Ok(Cell {
                group: self.clone(),
                pos: 1,
            }),
            (GroupKind::Status, sel) if sel == "code" => Ok(Cell {
                group: self.clone(),
                pos: 0,
            }),
            (GroupKind::Status, sel) if sel == "reason" => Ok(Cell {
                group: self.clone(),
                pos: 1,
            }),
            (GroupKind::Headers, Value::Str(key)) => {
                if let Some((i, _, _)) = self
                    .response
                    .read()
                    .ok_or_else(|| lockerr("cannot read group"))?
                    .headers
                    .get_full(key)
                {
                    Ok(Cell {
                        group: self.clone(),
                        pos: i,
                    })
                } else {
                    nores()
                }
            }
            _ => nores(),
        };
        Ok(std::iter::once(cell))
    }
}

impl From<ReqwestError> for HErr {
    fn from(e: ReqwestError) -> HErr {
        caused(HErrKind::Net, "http request error", e)
    }
}
