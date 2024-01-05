use indexmap::IndexMap;
use linkme::distributed_slice;
use reqwest::{blocking::Client, Error as ReqwestError};

use crate::{
    base::{Cell as XCell, *},
    utils::ownrc::*,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static URL_TO_HTTP: ElevationConstructor = ElevationConstructor {
    source_interpretation: "url",
    target_interpretation: "http",
    constructor: Cell::from_url_cell,
};

// ^http .value -> bytes
//       @status
//          code
//          reason
//       @headers/...

#[derive(Clone, Debug)]
pub struct Domain(OwnRc<Response>);

#[derive(Debug)]
pub struct Response {
    status: i16,
    reason: String,
    headers: IndexMap<String, Vec<String>>,
    body: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Debug)]
pub struct CellReader {
    kind: GroupKind,
    response: UseRc<Response>,
    pos: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GroupKind {
    Root,
    Attr,
    Status,
    Headers,
}

#[derive(Clone, Debug)]
pub struct Group {
    kind: GroupKind,
    response: Domain,
}

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

impl Cell {
    pub fn from_url_cell(cell: XCell) -> Res<XCell> {
        Cell::from_url_str(cell.as_url_str()?)
    }

    pub fn from_url_str<'a>(url: impl Into<&'a str>) -> Res<XCell> {
        let url_cell = from_url_str(url.into())?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(url_cell),
        })
    }
}

fn from_url_str(url: &str) -> Res<Domain> {
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
        eprintln!("Error: http call failed: {} = {} {}", url, status, reason);
    }
    Ok(Domain(OwnRc::new(Response {
        status,
        reason,
        headers,
        body: response.bytes()?.as_ref().to_vec(),
    })))
}

fn to_string(cell: &Cell) -> Res<String> {
    let ur = &*cell.group.response.0.tap();
    let bytes = &ur.body;
    let string = String::from_utf8(bytes.to_vec());
    match string {
        Ok(s) => Ok(s),
        Err(err) => Err(HErr::Http(format!("not utf8 string: {}", err))),
    }
}

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "http"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell {
            group: Group {
                kind: GroupKind::Root,
                response: self.clone(),
            },
            pos: 0,
        })
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

impl CellTrait for Cell {
    type Domain = Domain;
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn domain(&self) -> Res<Domain> {
        Ok(self.group.response.clone())
    }

    fn typ(&self) -> Res<&str> {
        match (&self.group.kind, self.pos) {
            (GroupKind::Root, _) => Ok("body"),
            (GroupKind::Attr, 0) => Ok(""),
            (GroupKind::Attr, 1) => Ok(""),
            (GroupKind::Status, 0) => Ok("int"),
            (GroupKind::Status, 1) => Ok("string"),
            (GroupKind::Headers, _) => Ok("header"),
            _ => Ok(""),
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            kind: self.group.kind,
            response: self.group.response.0.tap(),
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }

    fn sub(&self) -> Res<Group> {
        if self.group.kind == GroupKind::Attr && self.pos == 0 {
            let mut group = self.group.clone();
            group.kind = GroupKind::Status;
            Ok(group)
        } else if self.group.kind == GroupKind::Attr && self.pos == 1 {
            let mut group = self.group.clone();
            group.kind = GroupKind::Headers;
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
}

impl CellReaderTrait for CellReader {
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
            (GroupKind::Attr, 0) => Ok(Value::None),
            (GroupKind::Attr, 1) => Ok(Value::None),
            (GroupKind::Status, 0) => Ok(Value::Int(Int::I32(self.response.status as i32))),
            (GroupKind::Status, 1) => Ok(if self.response.reason.is_empty() {
                Value::None
            } else {
                Value::Str(&self.response.reason)
            }),
            (GroupKind::Headers, _) => {
                let header_values = if let Some(hv) = self.response.headers.get_index(self.pos) {
                    hv.1
                } else {
                    return Err(HErr::Http("logic error".to_string()));
                };
                Ok(Value::Str(header_values[0].as_str()))
            }
            _ => Err(HErr::Http("logic error".to_string())),
        }
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    // type SelectIterator = std::vec::IntoIter<Res<Cell>>;

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
            GroupKind::Headers => self.response.0.tap().headers.len(),
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
            (GroupKind::Headers, i) if i < self.response.0.tap().headers.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        match (self.kind, key) {
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
            (GroupKind::Headers, sel) => match sel {
                Selector::Star | Selector::DoubleStar | Selector::Top => self.at(0),
                Selector::Str(k) => {
                    if let Some((i, _, _)) = self.response.0.tap().headers.get_full(k) {
                        Ok(Cell {
                            group: self.clone(),
                            pos: i,
                        })
                    } else {
                        nores()
                    }
                }
            },
            _ => nores(),
        }
    }
}

impl From<ReqwestError> for HErr {
    fn from(e: ReqwestError) -> HErr {
        HErr::Http(format!("{}", e))
    }
}
