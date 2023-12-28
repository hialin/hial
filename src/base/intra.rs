use std::fmt::Debug;
use std::marker::PhantomData;

use crate::base::*;

pub trait DomainTrait: Clone + Debug {
    type Cell: CellTrait;

    fn interpretation(&self) -> &str;

    fn root(&self) -> Res<Self::Cell>;

    // fn origin(&self) -> Res<Path>;
    // fn save_to_origin(&self) -> Res<()>;
    // fn save_to(&self, target: &InDomain>) -> Res<()>;
}

pub trait CellTrait: Clone + Debug {
    type Group: GroupTrait;
    type CellReader: CellReaderTrait;
    type CellWriter: CellWriterTrait;

    fn typ(&self) -> Res<&str>;
    fn read(&self) -> Res<Self::CellReader>;
    fn write(&self) -> Res<Self::CellWriter>;

    fn sub(&self) -> Res<Self::Group> {
        nores()
    }
    fn attr(&self) -> Res<Self::Group> {
        nores()
    }
}

pub trait CellReaderTrait: Debug {
    fn index(&self) -> Res<usize> {
        nores()
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn value(&self) -> Res<Value> {
        nores()
    }

    // TODO: add fn to get the subtree as serialized data
}

pub trait CellWriterTrait: Debug {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        todo!();
    }

    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        todo!();
    }

    fn delete(&mut self) -> Res<()> {
        todo!();
    }
}

pub trait GroupTrait: Clone + Debug {
    type Cell: CellTrait;
    // type SelectIterator: Iterator<Item = Res<Self::Cell>>;

    fn label_type(&self) -> LabelType;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn at(&self, index: usize) -> Res<Self::Cell>;
    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::Cell>;
    // fn get_all<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::SelectIterator>;

    // fn add(&mut self) -> Res<()> {
    //     todo!();
    // }
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

    fn len(&self) -> usize {
        0
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
