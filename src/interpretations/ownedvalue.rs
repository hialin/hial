use crate::base::*;
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(Rc<OwnedValue>);
impl InDomain for Domain {
    type Cell = Cell;
    type Group = VoidGroup<Domain>;

    // fn new_from(source_interpretation: &str, source: DataSource) -> Res<Rc<Self>> {
    //     if let DataSource::File(path) = source {
    //         from_path(path.to_path_buf())
    //     } else {
    //         Err(HErr::IncompatibleSource("".into()))
    //     }
    // }

    fn interpretation(&self) -> &str {
        "value"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell(self.clone()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell(Domain);

#[derive(Debug)]
pub struct ValueRef(Domain);

impl From<OwnedValue> for Cell {
    fn from(ov: OwnedValue) -> Self {
        Cell(Domain(Rc::new(ov)))
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell(Domain(Rc::new(v.to_owned_value())))
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell(Domain(Rc::new(OwnedValue::String(s))))
    }
}

impl InValueRef for ValueRef {
    fn get(&self) -> Res<Value> {
        let v: &OwnedValue = &self.0 .0;
        Ok(v.into())
    }
}

impl InCell for Cell {
    type Domain = Domain;
    type ValueRef = ValueRef;

    fn domain(&self) -> &Self::Domain {
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

    fn as_data_source(&self) -> Option<Res<DataSource>> {
        if let OwnedValue::String(ref s) = *(self.0 .0) {
            Some(Ok(DataSource::String(Cow::from(s))))
        } else {
            None
        }
    }
}
