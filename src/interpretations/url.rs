use crate::base::*;
use reqwest::Url;
use std::rc::Rc;
use url::ParseError;

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
pub struct ValueRef(Domain);

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
        Ok(Value::Str(self.0 .0.as_str()))
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

    fn label(&self) -> Res<ValueRef> {
        NotFound::NoLabel.into()
    }

    fn value(&self) -> Res<ValueRef> {
        Ok(ValueRef(self.0.clone()))
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
