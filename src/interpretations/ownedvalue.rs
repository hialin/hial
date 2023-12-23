use crate::base::*;
use crate::utils::orc::{Orc, Urc};

#[derive(Clone, Debug)]
pub struct Domain(Orc<OwnedValue>);

impl InDomain for Domain {
    type Cell = Cell;
    type Group = VoidGroup<Domain>;

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
pub struct CellReader(Urc<OwnedValue>);

#[derive(Debug)]
pub struct ValueRef(Urc<OwnedValue>, bool);

impl From<OwnedValue> for Cell {
    fn from(ov: OwnedValue) -> Self {
        Cell(Domain(Orc::new(ov)))
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell(Domain(Orc::new(v.to_owned_value())))
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell(Domain(Orc::new(OwnedValue::String(s))))
    }
}

impl InCellReader for CellReader {
    fn index(&self) -> Res<usize> {
        NotFound::NoIndex.into()
    }

    fn label(&self) -> Res<Value> {
        NotFound::NoLabel.into()
    }

    fn value(&self) -> Res<Value> {
        Ok(self.0.as_value())
    }
}

impl InCell for Cell {
    type Domain = Domain;
    type CellReader = CellReader;

    fn domain(&self) -> &Self::Domain {
        &self.0
    }

    fn typ(&self) -> Res<&str> {
        Ok("value")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0 .0.urc()))
    }

    fn sub(&self) -> Res<VoidGroup<Domain>> {
        NotFound::NoGroup(format!("/")).into()
    }

    fn attr(&self) -> Res<VoidGroup<Domain>> {
        NotFound::NoGroup(format!("@")).into()
    }

    fn raw(&self) -> Res<RawDataContainer> {
        let vref = self.0 .0.urc();
        if let Value::Str(s) = vref.as_value() {
            Ok(RawDataContainer::String(s.to_owned()))
        } else {
            NotFound::NoResult("".into()).into()
        }
    }

    fn set_raw(&self, raw: RawDataContainer) -> Res<()> {
        let mut vref = self.0 .0.urc();
        if let Some(v) = vref.get_mut() {
            match raw {
                RawDataContainer::String(s) => *v = OwnedValue::from(s),
                RawDataContainer::File(pathbuf) => {
                    let s = std::fs::read_to_string(pathbuf)?;
                    *v = OwnedValue::from(s)
                }
            }
            Ok(())
        } else {
            Err(HErr::ExclusivityRequired {
                path: "".into(),
                op: "set_raw",
            })
        }
    }
}
