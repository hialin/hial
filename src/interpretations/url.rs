use std::rc::Rc;

use reqwest::Url;
use url::ParseError;

use crate::base::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(Rc<Url>);

impl InDomain for Domain {
    type Cell = Cell;
    type Group = VoidGroup<Domain>;

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
pub struct ValueRef(Domain, bool);

pub fn from_string(s: &str) -> Res<Domain> {
    Ok(Domain(Rc::new(Url::parse(s)?)))
}

impl Cell {
    pub fn as_str(&self) -> &str {
        self.0 .0.as_str()
    }
}

impl InCellReader for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(Value::Str(self.0 .0.as_str()))
    }
}

impl InCell for Cell {
    type Domain = Domain;
    type CellReader = CellReader;

    fn domain(&self) -> &Domain {
        &self.0
    }

    fn typ(&self) -> Res<&str> {
        Ok("value")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0.clone()))
    }
}

impl From<ParseError> for HErr {
    fn from(e: ParseError) -> HErr {
        HErr::Url(format!("{}", e))
    }
}
