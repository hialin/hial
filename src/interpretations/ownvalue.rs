use crate::base::*;
use crate::utils::orc::{Orc, Urc};

#[derive(Clone, Debug)]
pub struct Cell(Orc<OwnValue>);

impl DomainTrait for Cell {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "value"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(self.clone())
    }
}

#[derive(Debug)]
pub struct CellReader(Urc<OwnValue>);

#[derive(Debug)]
pub struct CellWriter(Urc<OwnValue>);
impl CellWriterTrait for CellWriter {}

impl From<OwnValue> for Cell {
    fn from(ov: OwnValue) -> Self {
        Cell(Orc::new(ov))
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell(Orc::new(v.to_owned_value()))
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell(Orc::new(OwnValue::String(s)))
    }
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(self.0.as_value())
    }
}

impl CellTrait for Cell {
    type Domain = Cell;
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn domain(&self) -> Res<Cell> {
        Ok(self.clone())
    }

    fn typ(&self) -> Res<&str> {
        Ok("value")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0.urc()))
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter(self.0.urc()))
    }
}
