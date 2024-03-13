use crate::api::{interpretation::*, *};
use crate::implement_try_from_xell;
use crate::utils::ownrc::{OwnRc, ReadRc, WriteRc};

#[derive(Clone, Debug)]
pub(crate) struct Cell(OwnRc<OwnValue>);

#[derive(Debug)]
pub(crate) struct CellReader(ReadRc<OwnValue>);

#[derive(Debug)]
pub(crate) struct CellWriter(WriteRc<OwnValue>);

implement_try_from_xell!(Cell, OwnValue);

impl Cell {
    pub(crate) fn from_str(s: &str) -> Res<Xell> {
        Cell::from_string(s.to_string())
    }

    pub(crate) fn from_string(s: String) -> Res<Xell> {
        Cell::from_value(OwnValue::String(s))
    }

    pub(crate) fn from_value(ov: OwnValue) -> Res<Xell> {
        let cell = Cell(OwnRc::new(ov));
        Ok(Xell::new_from(DynCell::from(cell), None))
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        Ok("value")
    }

    fn value(&self) -> Res<Value> {
        Ok(self.0.as_value())
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn index(&self) -> Res<usize> {
        nores()
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        *self.0 = value;
        Ok(())
    }
}

impl CellTrait for Cell {
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "value"
    }

    fn read(&self) -> Res<CellReader> {
        let r = self.0.read().ok_or_else(|| lockerr("cannot read cell"))?;
        Ok(CellReader(r))
    }

    fn write(&self) -> Res<CellWriter> {
        let w = self.0.write().ok_or_else(|| lockerr("cannot write cell"))?;
        Ok(CellWriter(w))
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }
}
