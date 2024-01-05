use std::rc::Rc;

use linkme::distributed_slice;
use reqwest::Url;
use url::ParseError;

use crate::base::{Cell as XCell, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_URL: ElevationConstructor = ElevationConstructor {
    source_interpretation: "value",
    target_interpretation: "url",
    constructor: Cell::from_value_cell,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(Rc<Url>);

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "url"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell(self.clone()))
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell(Domain);

#[derive(Debug)]
pub struct CellReader(Domain);

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

impl Cell {
    pub fn from_value_cell(cell: XCell) -> Res<XCell> {
        let reader = cell.read()?;
        let value = reader.value()?;
        let s = value.as_cow_str();
        Self::from_str(s.as_ref())
    }

    pub fn from_str(s: &str) -> Res<XCell> {
        let cell = Domain(Rc::new(Url::parse(s)?)).root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(cell),
        })
    }

    pub fn as_url_str(&self) -> &str {
        self.0 .0.as_str()
    }
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(Value::Str(self.0 .0.as_str()))
    }
}

impl CellTrait for Cell {
    type Domain = Domain;
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn domain(&self) -> Domain {
        self.0.clone()
    }

    fn typ(&self) -> Res<&str> {
        Ok("url")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0.clone()))
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }
}

impl From<ParseError> for HErr {
    fn from(e: ParseError) -> HErr {
        HErr::Url(format!("{}", e))
    }
}
