use super::common::*;
use std::fmt::Debug;
use std::rc::Rc;

pub trait InDomain: Clone + Debug {
    type Cell: InCell;
    type Group: InGroup;
    // type Trace: InTrace;

    fn root(self: &Rc<Self>) -> Res<Self::Cell>;
    // fn cell(&self, trace: &Self::Trace) -> Self::Cell;
}

pub trait InTrace: Clone + Debug {}

pub trait InCell: Clone + Debug {
    type Domain: InDomain;

    fn domain(&self) -> &Rc<Self::Domain>;

    fn typ(&self) -> Res<&str>;
    fn index(&self) -> Res<usize>;
    fn label(&self) -> Res<&str>;
    fn value(&self) -> Res<Value>;

    fn sub(&self) -> Res<<Self::Domain as InDomain>::Group>;
    fn attr(&self) -> Res<<Self::Domain as InDomain>::Group>;

    fn set(&mut self, value: OwnedValue) -> Res<()> {
        todo!()
    }
}

pub trait InGroup: Clone + Debug {
    type Domain: InDomain;
    // type SelectIterator: Iterator<Item = Res<Self::Cell>>;

    fn label_type(&self) -> LabelType;
    fn len(&self) -> usize;
    fn at(&self, index: usize) -> Res<<Self::Domain as InDomain>::Cell>;
    fn get<'s, 'a, S: Into<Selector<'a>>>(
        &'s self,
        label: S,
    ) -> Res<<Self::Domain as InDomain>::Cell>;
    // fn get_all<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::SelectIterator>;
}

#[derive(Clone, Debug)]
pub struct DummyTrace {}
impl InTrace for DummyTrace {}
