use crate::base::*;
use crate::utils::orc::{Orc, Urc};

#[derive(Clone, Debug)]
pub struct Domain(Orc<OwnValue>);

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "value"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell(self.clone()))
    }
}

#[derive(Clone, Debug)]
pub struct Cell(Domain);

#[derive(Debug)]
pub struct CellReader(Urc<OwnValue>);

#[derive(Debug)]
pub struct CellWriter(Urc<OwnValue>);
impl CellWriterTrait for CellWriter {}

#[derive(Debug)]
pub struct ValueRef(Urc<OwnValue>, bool);

impl From<OwnValue> for Domain {
    fn from(ov: OwnValue) -> Self {
        Domain(Orc::new(ov))
    }
}

impl From<Value<'_>> for Domain {
    fn from(v: Value) -> Self {
        Domain(Orc::new(v.to_owned_value()))
    }
}

impl From<String> for Domain {
    fn from(s: String) -> Self {
        Domain(Orc::new(OwnValue::String(s)))
    }
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(self.0.as_value())
    }
}

impl CellTrait for Cell {
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn typ(&self) -> Res<&str> {
        Ok("value")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0 .0.urc()))
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter(self.0 .0.urc()))
    }
}
