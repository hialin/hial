use std::rc::Rc;

use linkme::distributed_slice;
use reqwest::Url;
use url::ParseError;

use crate::base::{Cell as XCell, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_URL: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value"],
    target_interpretations: &["url"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub struct Data {
    url: Url,
}

#[derive(Clone, Debug)]
pub struct Cell(Rc<Data>);

#[derive(Debug)]
pub struct CellReader(Rc<Data>);

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                let url_cell = Cell(Rc::new(Data {
                    url: Url::parse(value)?,
                }));
                Ok(new_cell(DynCell::from(url_cell), Some(cell)))
            }
            _ => nores(),
        }
    }

    pub fn from_str(s: &str) -> Res<XCell> {
        let url_cell = Cell(Rc::new(Data {
            url: Url::parse(s)?,
        }));
        Ok(new_cell(DynCell::from(url_cell), None))
    }
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(Value::Str(self.0.url.as_str()))
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn index(&self) -> Res<usize> {
        nores()
    }
}

impl CellTrait for Cell {
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "url"
    }

    fn ty(&self) -> Res<&str> {
        Ok("url")
    }

    fn read(&self) -> Res<CellReader> {
        Ok(CellReader(self.0.clone()))
    }

    fn write(&self) -> Res<CellWriter> {
        Ok(CellWriter {})
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }
}

impl From<ParseError> for HErr {
    fn from(e: ParseError) -> HErr {
        caused(HErrKind::InvalidFormat, "cannot parse url", e)
    }
}
