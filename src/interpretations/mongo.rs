use std::rc::Rc;

use linkme::distributed_slice;
use mongodb::{
    bson::{Bson, Document},
    sync::{Client, Collection, Database},
};

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_MONGO: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value"],
    target_interpretations: &["mongo"],
    constructor: Cell::from_cell,
};

/*
Mongo xell specification:
The main data structures are Cell and Group.
The Group is an enumeration: it can be a server, list of databases, list of collections or list of documents variant.
The Cell is usually a pointer to a group and a position in the group.
*/

// ^mongo  -> server (root)
//   /dbname -> database
//     /collname -> collection
//       /[i] -> document
//         /field -> field (scalar value or nested doc/array)

const DEFAULT_DOC_LIMIT: i64 = 100;

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub(crate) enum Group {
    Server {
        client: Client,
    },
    Databases {
        client: Client,
        names: Rc<Vec<String>>,
        head: Option<Rc<(Cell, Relation)>>,
    },
    Collections {
        db: Database,
        names: Rc<Vec<String>>,
        head: Option<Rc<(Cell, Relation)>>,
    },
    Documents {
        docs: Rc<Vec<Document>>,
        head: Option<Rc<(Cell, Relation)>>,
    },
    Fields {
        fields: Rc<Vec<(String, Bson)>>,
        head: Option<Rc<(Cell, Relation)>>,
    },
    Array {
        values: Rc<Vec<Bson>>,
        head: Option<Rc<(Cell, Relation)>>,
    },
}

pub(crate) type CellReader = Cell;
pub(crate) type CellWriter = Cell;

implement_try_from_xell!(Cell, Mongo);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, _params: &ElevateParams) -> Res<Xell> {
        let reader = origin.read().err()?;
        let value = reader.value()?;
        let conn_str = value.as_cow_str();

        let client = mongodb::sync::Client::with_uri_str(conn_str.as_ref())
            .map_err(|e| caused(HErrKind::Net, "mongo: cannot connect", e))?;

        let group = Group::Server { client };
        let cell = Cell { group, pos: 0 };
        Ok(Xell::new_from(DynCell::from(cell), Some(origin)))
    }

    fn group(&self) -> &Group {
        &self.group
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn group_head(&self) -> Res<(Self, Relation)> {
        match self.group() {
            Group::Server { .. } => nores(),
            Group::Databases { head, .. }
            | Group::Collections { head, .. }
            | Group::Documents { head, .. }
            | Group::Fields { head, .. }
            | Group::Array { head, .. } => match head {
                Some(head) => Ok((head.0.clone(), head.1)),
                None => nores(),
            },
        }
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = Cell;
    type CellWriter = Cell;

    fn interpretation(&self) -> &str {
        "mongo"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(self.clone())
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(self.clone())
    }

    fn sub(&self) -> Res<Self::Group> {
        match self.group() {
            Group::Server { client } => {
                let names = client.list_database_names().run()?;
                Ok(Group::Databases {
                    client: client.clone(),
                    names: Rc::new(names),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                })
            }
            Group::Databases { client, names, .. } => {
                let name = names
                    .get(self.pos())
                    .ok_or_else(|| faulterr("database index out of bounds"))?;
                let db = client.database(name);
                let names = db.list_collection_names().run()?;
                Ok(Group::Collections {
                    db,
                    names: Rc::new(names),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                })
            }
            Group::Collections { db, names, .. } => {
                let name = names
                    .get(self.pos())
                    .ok_or_else(|| faulterr("collection index out of bounds"))?;
                let coll: Collection<Document> = db.collection(name);
                let mut docs = vec![];
                let cursor = coll.find(Document::new()).limit(DEFAULT_DOC_LIMIT).run()?;
                for doc in cursor {
                    docs.push(doc?);
                }
                Ok(Group::Documents {
                    docs: Rc::new(docs),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                })
            }
            Group::Documents { docs, .. } => {
                let doc = docs
                    .get(self.pos())
                    .ok_or_else(|| faulterr("document index out of bounds"))?;
                Ok(Group::Fields {
                    fields: Rc::new(doc.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                    head: Some(Rc::new((self.clone(), Relation::Sub))),
                })
            }
            Group::Fields { fields, .. } => {
                let value = &fields
                    .get(self.pos())
                    .ok_or_else(|| faulterr("field index out of bounds"))?
                    .1;

                match value {
                    Bson::Document(doc) => Ok(Group::Fields {
                        fields: Rc::new(doc.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    Bson::Array(values) => Ok(Group::Array {
                        values: Rc::new(values.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    _ => nores(),
                }
            }
            Group::Array { values, .. } => {
                let value = values
                    .get(self.pos())
                    .ok_or_else(|| faulterr("array index out of bounds"))?;
                match value {
                    Bson::Document(doc) => Ok(Group::Fields {
                        fields: Rc::new(doc.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    Bson::Array(values) => Ok(Group::Array {
                        values: Rc::new(values.clone()),
                        head: Some(Rc::new((self.clone(), Relation::Sub))),
                    }),
                    _ => nores(),
                }
            }
        }
    }

    fn head(&self) -> Res<(Self, Relation)> {
        self.group_head()
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Once<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(match self {
            Group::Server { .. } => 1,
            Group::Databases { names, .. } => names.len(),
            Group::Collections { names, .. } => names.len(),
            Group::Documents { docs, .. } => docs.len(),
            Group::Fields { fields, .. } => fields.len(),
            Group::Array { values, .. } => values.len(),
        })
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match self {
            Group::Server { .. } if index == 0 => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            Group::Databases { names, .. } if index < names.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            Group::Collections { names, .. } if index < names.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            Group::Documents { docs, .. } if index < docs.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            Group::Fields { fields, .. } if index < fields.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            Group::Array { values, .. } if index < values.len() => Ok(Cell {
                group: self.clone(),
                pos: index,
            }),
            _ => nores(),
        }
    }

    fn get_all(&self, key: Value) -> Res<Self::CellIterator> {
        let cell = match self {
            Group::Databases { names, .. } => match key {
                Value::Str(name) => {
                    if let Some(pos) = names.iter().position(|x| x == name) {
                        Ok(Cell {
                            group: self.clone(),
                            pos,
                        })
                    } else {
                        nores()
                    }
                }
                _ => nores(),
            },
            Group::Collections { names, .. } => match key {
                Value::Str(name) => {
                    if let Some(pos) = names.iter().position(|x| x == name) {
                        Ok(Cell {
                            group: self.clone(),
                            pos,
                        })
                    } else {
                        nores()
                    }
                }
                _ => nores(),
            },
            Group::Fields { fields, .. } => match key {
                Value::Str(name) => {
                    if let Some(pos) = fields.iter().position(|(k, _)| k == name) {
                        Ok(Cell {
                            group: self.clone(),
                            pos,
                        })
                    } else {
                        nores()
                    }
                }
                _ => nores(),
            },
            Group::Array { values, .. } => match key {
                Value::Int(i) => {
                    let pos = i.as_i128();
                    if pos < 0 || pos > usize::MAX as i128 {
                        return nores();
                    }
                    let pos = pos as usize;
                    if pos < values.len() {
                        Ok(Cell {
                            group: self.clone(),
                            pos,
                        })
                    } else {
                        nores()
                    }
                }
                _ => nores(),
            },
            _ => nores(),
        };
        Ok(std::iter::once(cell))
    }
}

fn bson_to_value(bson: &Bson) -> Res<Value<'_>> {
    match bson {
        Bson::Null => Ok(Value::None),
        Bson::Boolean(b) => Ok(Value::Bool(*b)),
        Bson::Int32(i) => Ok(Value::from(*i)),
        Bson::Int64(i) => Ok(Value::from(*i)),
        Bson::Double(f) => Ok(Value::from(*f)),
        Bson::String(s) => Ok(Value::Str(s)),
        Bson::ObjectId(oid) => Ok(Value::Str(oid.to_hex().leak())),
        Bson::Document(_) | Bson::Array(_) => nores(),
        _ => nores(),
    }
}

fn bson_ty(bson: &Bson) -> &'static str {
    match bson {
        Bson::Null => "null",
        Bson::Boolean(_) => "bool",
        Bson::Int32(_) | Bson::Int64(_) => "number",
        Bson::Double(_) => "number",
        Bson::String(_) => "string",
        Bson::ObjectId(_) => "objectid",
        Bson::Document(_) => "document",
        Bson::Array(_) => "array",
        Bson::DateTime(_) => "datetime",
        Bson::Binary(_) => "binary",
        Bson::RegularExpression(_) => "regex",
        Bson::Timestamp(_) => "timestamp",
        _ => "unknown",
    }
}

impl CellReaderTrait for Cell {
    fn ty(&self) -> Res<&str> {
        match self.group() {
            Group::Server { .. } => Ok("server"),
            Group::Databases { .. } => Ok("database"),
            Group::Collections { .. } => Ok("collection"),
            Group::Documents { .. } => Ok("document"),
            Group::Fields { fields, .. } => Ok(bson_ty(
                &fields
                    .get(self.pos())
                    .ok_or_else(|| faulterr("field index out of bounds"))?
                    .1,
            )),
            Group::Array { values, .. } => Ok(bson_ty(
                values
                    .get(self.pos())
                    .ok_or_else(|| faulterr("array index out of bounds"))?,
            )),
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos())
    }

    fn label(&self) -> Res<Value<'_>> {
        match self.group() {
            Group::Server { .. } => nores(),
            Group::Databases { names, .. } => Ok(Value::Str(
                names
                    .get(self.pos())
                    .ok_or_else(|| faulterr("database index out of bounds"))?,
            )),
            Group::Collections { names, .. } => {
                Ok(Value::Str(names.get(self.pos()).ok_or_else(|| {
                    faulterr("collection index out of bounds")
                })?))
            }
            Group::Documents { .. } => nores(),
            Group::Fields { fields, .. } => Ok(Value::Str(
                fields
                    .get(self.pos())
                    .ok_or_else(|| faulterr("field index out of bounds"))?
                    .0
                    .as_str(),
            )),
            Group::Array { .. } => nores(),
        }
    }

    fn value(&self) -> Res<Value<'_>> {
        match self.group() {
            Group::Fields { fields, .. } => bson_to_value(
                &fields
                    .get(self.pos())
                    .ok_or_else(|| faulterr("field index out of bounds"))?
                    .1,
            ),
            Group::Array { values, .. } => bson_to_value(
                values
                    .get(self.pos())
                    .ok_or_else(|| faulterr("array index out of bounds"))?,
            ),
            _ => nores(),
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for Cell {
    fn set_value(&mut self, _value: OwnValue) -> Res<()> {
        fault("mongo: read-only")
    }
}
impl From<mongodb::error::Error> for HErr {
    fn from(e: mongodb::error::Error) -> HErr {
        caused(HErrKind::Net, "mongo error", e)
    }
}
