use crate::base::{common::*, in_api::*};
use reqwest::Url;
use std::rc::Rc;
use url::ParseError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(Url);
impl InDomain for Domain {
    type Cell = Cell;
    type Group = Cell;

    fn root(self: &Rc<Self>) -> Res<Self::Cell> {
        Ok(Cell(self.clone()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell(Rc<Domain>);

pub fn from_string(s: &str) -> Res<Cell> {
    Rc::new(Domain(Url::parse(s)?)).root()
}

impl Cell {
    pub fn as_str(&self) -> &str {
        self.0 .0.as_str()
    }
}

impl InCell for Cell {
    type Domain = Domain;

    fn domain(&self) -> &Rc<Self::Domain> {
        &self.0
    }

    fn typ(&self) -> Res<&str> {
        Ok("value")
    }

    fn index(&self) -> Res<usize> {
        NotFound::NoIndex().into()
    }

    fn label(&self) -> Res<&str> {
        NotFound::NoLabel().into()
    }

    fn value(&self) -> Res<Value> {
        Ok(Value::Str(self.0 .0.as_str()))
    }

    fn sub(&self) -> Res<Cell> {
        NotFound::NoGroup(format!("/")).into()
    }

    fn attr(&self) -> Res<Cell> {
        NotFound::NoGroup(format!("@")).into()
    }
}

impl InGroup for Cell {
    type Domain = Domain;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: false,
        }
    }

    fn len(&self) -> usize {
        0
    }

    fn at(&self, index: usize) -> Res<Cell> {
        NotFound::NoResult(format!("")).into()
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        NotFound::NoResult(format!("")).into()
    }
}

impl From<ParseError> for HErr {
    fn from(e: ParseError) -> HErr {
        HErr::Url(format!("{}", e))
    }
}
