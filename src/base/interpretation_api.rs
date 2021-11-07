use super::common::*;
use std::fmt::Debug;

pub trait InterpretationCell: Clone + Debug {
    type Group: InterpretationGroup;

    fn typ(&self) -> Res<&str>;
    fn index(&self) -> Res<usize>;
    fn label(&self) -> Res<&str>;
    fn value(&self) -> Res<Value>;

    fn sub(&self) -> Res<Self::Group>;
    fn attr(&self) -> Res<Self::Group>;
}

pub trait InterpretationGroup: Clone + Debug {
    type Cell: InterpretationCell;
    // type SelectIterator: Iterator<Item = Res<Self::Cell>>;

    fn label_type(&self) -> LabelType;
    fn len(&self) -> usize;
    fn at(&self, index: usize) -> Res<Self::Cell>;
    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::Cell>;
    // fn get_all<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::SelectIterator>;
}
