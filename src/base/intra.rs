use std::{fmt::Debug, marker::PhantomData};

use crate::base::*;

pub trait CellTrait: Clone + Debug {
    type Group: GroupTrait;
    type CellReader: CellReaderTrait;
    type CellWriter: CellWriterTrait;

    fn interpretation(&self) -> &str;
    fn ty(&self) -> Res<&str>;

    fn read(&self) -> Res<Self::CellReader>;
    fn write(&self) -> Res<Self::CellWriter>;

    fn sub(&self) -> Res<Self::Group> {
        nores()
    }

    fn attr(&self) -> Res<Self::Group> {
        nores()
    }

    fn head(&self) -> Res<(Self, Relation)>;
}

pub trait CellReaderTrait: Debug {
    fn index(&self) -> Res<usize>;

    fn label(&self) -> Res<Value>;

    fn value(&self) -> Res<Value>;

    fn serial(&self) -> Res<String>;
}

pub trait CellWriterTrait: Debug {
    fn set_value(&mut self, value: OwnValue) -> Res<()>;

    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        todo!() // TODO: remove this default implementation
    }

    fn delete(&mut self) -> Res<()> {
        todo!() // TODO: remove this default implementation
    }
}

pub trait GroupTrait: Clone + Debug {
    type Cell: CellTrait;
    // type SelectIterator: Iterator<Item = Res<Self::Cell>>;

    fn label_type(&self) -> LabelType;
    fn len(&self) -> Res<usize>;
    fn is_empty(&self) -> bool {
        self.len().map_or(false, |l| l == 0)
    }
    fn at(&self, index: usize) -> Res<Self::Cell>;
    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::Cell>;
    // fn get_all<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::SelectIterator>;

    fn add(&mut self) -> Res<()> {
        todo!() // remove this default implementation
    }
}

#[derive(Clone, Debug)]
pub struct VoidGroup<C>(PhantomData<C>);
impl<C> From<C> for VoidGroup<C> {
    fn from(_: C) -> Self {
        VoidGroup(PhantomData)
    }
}
impl<C: CellTrait> GroupTrait for VoidGroup<C> {
    type Cell = C;

    fn label_type(&self) -> LabelType {
        LabelType::default()
    }

    fn len(&self) -> Res<usize> {
        Ok(0)
    }

    fn at(&self, index: usize) -> Res<C> {
        nores()
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<C> {
        nores()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct LabelType {
    pub is_indexed: bool,
    pub unique_labels: bool,
}
