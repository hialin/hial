use std::rc::Rc;

use indexmap::IndexMap;
use linkme::distributed_slice;
use mongodb::bson::{Bson, Document};

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
    utils::ownrc::*,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_MONGO: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value"],
    target_interpretations: &["mongo"],
    constructor: Cell::from_cell,
};

// ^mongo  -> server (root)
//   /dbname -> database
//     /collname -> collection
//       /[i] -> document
//         /field -> field (scalar value or nested doc/array)

const DEFAULT_DOC_LIMIT: i64 = 100;

#[derive(Clone, Debug)]
pub(crate) enum NodeData {
    Server {
        client: Rc<mongodb::sync::Client>,
        db_names: OwnRc<Vec<String>>,
    },
    Database {
        client: Rc<mongodb::sync::Client>,
        name: String,
        coll_names: OwnRc<Vec<String>>,
    },
    Collection {
        client: Rc<mongodb::sync::Client>,
        db_name: String,
        coll_name: String,
        docs: OwnRc<Vec<Document>>,
    },
    Document {
        fields: OwnRc<IndexMap<String, Bson>>,
    },
    Array {
        items: OwnRc<Vec<Bson>>,
    },
}

#[derive(Debug)]
pub(crate) enum ReadNodeData {
    Server(ReadRc<Vec<String>>),
    Database(ReadRc<Vec<String>>),
    Collection(ReadRc<Vec<Document>>),
    Document(ReadRc<IndexMap<String, Bson>>),
    Array(ReadRc<Vec<Bson>>),
}

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    data: Rc<NodeData>,
    head: Option<Rc<(Cell, Relation)>>,
}

#[derive(Debug)]
pub(crate) struct CellReader {
    data: ReadNodeData,
    pos: usize,
}

#[derive(Debug)]
pub(crate) struct CellWriter {}

implement_try_from_xell!(Cell, Mongo);

impl Group {
    fn new(data: NodeData, head: Option<Rc<(Cell, Relation)>>) -> Self {
        Group {
            data: Rc::new(data),
            head,
        }
    }
}

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, _params: &ElevateParams) -> Res<Xell> {
        let reader = origin.read().err()?;
        let value = reader.value()?;
        let conn_str = value.as_cow_str();

        let client = mongodb::sync::Client::with_uri_str(conn_str.as_ref())
            .map_err(|e| caused(HErrKind::Net, "mongo: cannot connect", e))?;

        let db_names = client
            .list_database_names()
            .with_options(mongodb::options::ListDatabasesOptions::default())
            .run()
            .map_err(|e| caused(HErrKind::Net, "mongo: cannot list databases", e))?;

        let group = Group::new(
            NodeData::Server {
                client: Rc::new(client),
                db_names: OwnRc::new(db_names),
            },
            None,
        );
        let cell = Cell { group, pos: 0 };
        Ok(Xell::new_from(DynCell::from(cell), Some(origin)))
    }

    fn make_reader(&self) -> Res<CellReader> {
        let data = match self.group.data.as_ref() {
            NodeData::Server { db_names, .. } => {
                ReadNodeData::Server(db_names.read().ok_or_else(|| lockerr("cannot read"))?)
            }
            NodeData::Database { coll_names, .. } => {
                ReadNodeData::Database(coll_names.read().ok_or_else(|| lockerr("cannot read"))?)
            }
            NodeData::Collection { docs, .. } => {
                ReadNodeData::Collection(docs.read().ok_or_else(|| lockerr("cannot read"))?)
            }
            NodeData::Document { fields } => {
                ReadNodeData::Document(fields.read().ok_or_else(|| lockerr("cannot read"))?)
            }
            NodeData::Array { items } => {
                ReadNodeData::Array(items.read().ok_or_else(|| lockerr("cannot read"))?)
            }
        };
        Ok(CellReader {
            data,
            pos: self.pos,
        })
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "mongo"
    }

    fn read(&self) -> Res<Self::CellReader> {
        self.make_reader()
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }

    fn sub(&self) -> Res<Group> {
        let head = Some(Rc::new((self.clone(), Relation::Sub)));
        match self.group.data.as_ref() {
            NodeData::Server {
                client, db_names, ..
            } => {
                let name = {
                    let r = db_names
                        .read()
                        .ok_or_else(|| lockerr("cannot read db_names"))?;
                    r.get(self.pos).ok_or_else(|| faulterr("bad pos"))?.clone()
                };
                let coll_names = client
                    .database(&name)
                    .list_collection_names()
                    .run()
                    .map_err(|e| caused(HErrKind::Net, "mongo: cannot list collections", e))?;
                Ok(Group::new(
                    NodeData::Database {
                        client: client.clone(),
                        name,
                        coll_names: OwnRc::new(coll_names),
                    },
                    head,
                ))
            }
            NodeData::Database {
                client,
                name,
                coll_names,
                ..
            } => {
                let coll_name = {
                    let r = coll_names
                        .read()
                        .ok_or_else(|| lockerr("cannot read coll_names"))?;
                    r.get(self.pos).ok_or_else(|| faulterr("bad pos"))?.clone()
                };
                let db = client.database(name);
                let coll = db.collection::<Document>(&coll_name);
                let docs: Vec<Document> = {
                    let mut cursor = coll.find(Document::new()).limit(DEFAULT_DOC_LIMIT).run()?;
                    let mut v = Vec::new();
                    while let Some(Ok(doc)) = cursor.next() {
                        v.push(doc);
                    }
                    v
                };
                Ok(Group::new(
                    NodeData::Collection {
                        client: client.clone(),
                        db_name: name.clone(),
                        coll_name,
                        docs: OwnRc::new(docs),
                    },
                    head,
                ))
            }
            NodeData::Collection { docs, .. } => {
                let fields = {
                    let r = docs.read().ok_or_else(|| lockerr("cannot read docs"))?;
                    let doc = r.get(self.pos).ok_or_else(|| faulterr("bad pos"))?;
                    let map: IndexMap<String, Bson> =
                        doc.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    OwnRc::new(map)
                };
                Ok(Group::new(NodeData::Document { fields }, head))
            }
            NodeData::Document { fields } => {
                let r = fields.read().ok_or_else(|| lockerr("cannot read fields"))?;
                let (_, bson) = r.get_index(self.pos).ok_or_else(|| faulterr("bad pos"))?;
                bson_to_sub_group(bson, self)
            }
            NodeData::Array { items } => {
                let r = items.read().ok_or_else(|| lockerr("cannot read array"))?;
                let bson = r.get(self.pos).ok_or_else(|| faulterr("bad pos"))?;
                bson_to_sub_group(bson, self)
            }
        }
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.group.head {
            Some(ref head) => Ok((head.0.clone(), head.1)),
            None => nores(),
        }
    }
}

fn bson_to_sub_group(bson: &Bson, parent: &Cell) -> Res<Group> {
    let head = Some(Rc::new((parent.clone(), Relation::Sub)));
    match bson {
        Bson::Document(d) => {
            let map: IndexMap<String, Bson> =
                d.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            Ok(Group::new(
                NodeData::Document {
                    fields: OwnRc::new(map),
                },
                head,
            ))
        }
        Bson::Array(a) => Ok(Group::new(
            NodeData::Array {
                items: OwnRc::new(a.clone()),
            },
            head,
        )),
        _ => nores(),
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

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match &self.data {
            ReadNodeData::Server(_) => Ok("server"),
            ReadNodeData::Database(_) => Ok("database"),
            ReadNodeData::Collection(_) => Ok("collection"),
            ReadNodeData::Document(fields) => {
                let (_, bson) = fields
                    .get_index(self.pos)
                    .ok_or_else(|| faulterr("bad pos"))?;
                Ok(bson_ty(bson))
            }
            ReadNodeData::Array(items) => {
                let bson = items.get(self.pos).ok_or_else(|| faulterr("bad pos"))?;
                Ok(bson_ty(bson))
            }
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos)
    }

    fn label(&self) -> Res<Value<'_>> {
        match &self.data {
            ReadNodeData::Server(db_names) => {
                let name = db_names.get(self.pos).ok_or_else(|| faulterr("bad pos"))?;
                Ok(Value::Str(name))
            }
            ReadNodeData::Database(coll_names) => {
                let name = coll_names
                    .get(self.pos)
                    .ok_or_else(|| faulterr("bad pos"))?;
                Ok(Value::Str(name))
            }
            ReadNodeData::Collection(_) => nores(),
            ReadNodeData::Document(fields) => {
                let (k, _) = fields
                    .get_index(self.pos)
                    .ok_or_else(|| faulterr("bad pos"))?;
                Ok(Value::Str(k))
            }
            ReadNodeData::Array(_) => nores(),
        }
    }

    fn value(&self) -> Res<Value<'_>> {
        match &self.data {
            ReadNodeData::Server(_) | ReadNodeData::Database(_) | ReadNodeData::Collection(_) => {
                nores()
            }
            ReadNodeData::Document(fields) => {
                let (_, bson) = fields
                    .get_index(self.pos)
                    .ok_or_else(|| faulterr("bad pos"))?;
                bson_to_value(bson)
            }
            ReadNodeData::Array(items) => {
                let bson = items.get(self.pos).ok_or_else(|| faulterr("bad pos"))?;
                bson_to_value(bson)
            }
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, _value: OwnValue) -> Res<()> {
        fault("mongo: read-only")
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Once<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: !matches!(self.data.as_ref(), NodeData::Array { .. }),
        }
    }

    fn len(&self) -> Res<usize> {
        match self.data.as_ref() {
            NodeData::Server { db_names, .. } => {
                Ok(db_names.read().ok_or_else(|| lockerr("cannot read"))?.len())
            }
            NodeData::Database { coll_names, .. } => Ok(coll_names
                .read()
                .ok_or_else(|| lockerr("cannot read"))?
                .len()),
            NodeData::Collection { docs, .. } => {
                Ok(docs.read().ok_or_else(|| lockerr("cannot read"))?.len())
            }
            NodeData::Document { fields } => {
                Ok(fields.read().ok_or_else(|| lockerr("cannot read"))?.len())
            }
            NodeData::Array { items } => {
                Ok(items.read().ok_or_else(|| lockerr("cannot read"))?.len())
            }
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        if index < self.len()? {
            Ok(Cell {
                group: self.clone(),
                pos: index,
            })
        } else {
            nores()
        }
    }

    fn get_all(&self, key: Value) -> Res<Self::CellIterator> {
        let cell = match self.data.as_ref() {
            NodeData::Server { db_names, .. } => {
                let Value::Str(name) = key else {
                    return nores();
                };
                let r = db_names.read().ok_or_else(|| lockerr("cannot read"))?;
                match r.iter().position(|n| n == name) {
                    Some(pos) => Ok(Cell {
                        group: self.clone(),
                        pos,
                    }),
                    None => nores(),
                }
            }
            NodeData::Database { coll_names, .. } => {
                let Value::Str(name) = key else {
                    return nores();
                };
                let r = coll_names.read().ok_or_else(|| lockerr("cannot read"))?;
                match r.iter().position(|n| n == name) {
                    Some(pos) => Ok(Cell {
                        group: self.clone(),
                        pos,
                    }),
                    None => nores(),
                }
            }
            NodeData::Collection { .. } => nores(),
            NodeData::Document { fields } => {
                let Value::Str(k) = key else { return nores() };
                let r = fields.read().ok_or_else(|| lockerr("cannot read"))?;
                match r.get_index_of(k) {
                    Some(pos) => Ok(Cell {
                        group: self.clone(),
                        pos,
                    }),
                    None => nores(),
                }
            }
            NodeData::Array { .. } => nores(),
        };
        Ok(std::iter::once(cell))
    }
}

impl From<mongodb::error::Error> for HErr {
    fn from(e: mongodb::error::Error) -> HErr {
        caused(HErrKind::Net, "mongo error", e)
    }
}
