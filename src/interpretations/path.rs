use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::base::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(Rc<(PathBuf, String)>);

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "path"
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

pub fn from_string(s: impl Into<String>) -> Res<Domain> {
    let s = s.into();
    let data = (PathBuf::from(&s), s);
    Ok(Domain(Rc::new(data)))
}

impl Cell {
    pub fn as_path(&self) -> Res<&Path> {
        Ok(self.0 .0 .0.as_path())
    }
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(Value::Str(&self.0 .0 .1))
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
