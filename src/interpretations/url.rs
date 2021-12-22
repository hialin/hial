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
pub struct ValueRef(Domain, bool);

pub fn from_string(s: &str) -> Res<Cell> {
    Domain(Rc::new(Url::parse(s)?)).root()
}

impl Cell {
    pub fn as_str(&self) -> &str {
        self.0 .0.as_str()
    }
}

impl InValueRef for ValueRef {
    fn get(&self) -> Res<Value> {
        if self.1 {
            NotFound::NoLabel.into()
        } else {
            Ok(Value::Str(self.0 .0.as_str()))
        }
    }
}

impl InCell for Cell {
    type Domain = Domain;
    type ValueRef = ValueRef;

    fn domain(&self) -> &Domain {
        &self.0
    }

    fn typ(&self) -> Res<&str> {
        Ok("value")
    }

    fn index(&self) -> Res<usize> {
        NotFound::NoIndex.into()
    }

    fn label(&self) -> ValueRef {
        ValueRef(self.0.clone(), true)
    }

    fn value(&self) -> ValueRef {
        ValueRef(self.0.clone(), false)
    }

    fn sub(&self) -> Res<VoidGroup<Domain>> {
        NotFound::NoGroup(format!("/")).into()
    }

    fn attr(&self) -> Res<VoidGroup<Domain>> {
        NotFound::NoGroup(format!("@")).into()
    }
}

impl From<ParseError> for HErr {
    fn from(e: ParseError) -> HErr {
        HErr::Url(format!("{}", e))
    }
}
