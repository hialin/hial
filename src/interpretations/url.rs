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
pub struct Domain(Rc<(Url, Option<XCell>)>);

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "url"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell(self.clone()))
    }

    fn origin(&self) -> Res<XCell> {
        match &self.0 .1 {
            Some(c) => Ok(c.clone()),
            None => nores(),
        }
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

#[derive(Clone, Debug)]
pub struct Cell(Domain);

#[derive(Debug)]
pub struct CellReader(Domain);

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.domain().interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                let cell = Domain(Rc::new((Url::parse(value)?, Some(cell)))).root()?;
                Ok(XCell {
                    dyn_cell: DynCell::from(cell),
                })
            }
            _ => nores(),
        }
    }

    pub fn from_str(s: &str) -> Res<XCell> {
        let cell = Domain(Rc::new((Url::parse(s)?, None))).root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(cell),
        })
    }

    pub fn as_url_str(&self) -> &str {
        self.0 .0 .0.as_str()
    }
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(Value::Str(self.0 .0 .0.as_str()))
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn index(&self) -> Res<usize> {
        nores()
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

    fn ty(&self) -> Res<&str> {
        Ok("url")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0.clone()))
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }

    fn head(&self) -> Res<(Self, Relation)> {
        todo!()
    }
}

impl From<ParseError> for HErr {
    fn from(e: ParseError) -> HErr {
        caused(HErrKind::InvalidFormat, "cannot parse url", e)
    }
}
