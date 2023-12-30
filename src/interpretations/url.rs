use std::rc::Rc;

use reqwest::Url;
use url::ParseError;

use crate::base::*;

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell(Domain);

#[derive(Debug)]
pub struct CellReader(Domain);

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

pub fn from_string(s: &str) -> Res<Domain> {
    Ok(Domain(Rc::new(Url::parse(s)?)))
}

impl Cell {
    pub fn as_str(&self) -> &str {
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

    fn domain(&self) -> Res<Domain> {
        Ok(self.0.clone())
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
