use crate::base::{Cell as XCell, *};
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

impl Cell {
    pub fn from_str(s: &str) -> Res<XCell> {
        Cell::from_string(s.to_string())
    }

    pub fn from_string(s: String) -> Res<XCell> {
        Cell::from_value(OwnValue::String(s))
    }

    pub fn from_value(ov: OwnValue) -> Res<XCell> {
        let cell = Cell(Orc::new(ov));
        Ok(XCell {
            dyn_cell: DynCell::from(cell),
        })
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
