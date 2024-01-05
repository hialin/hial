use std::fmt::Debug;
use std::marker::PhantomData;

use crate::base::*;

pub trait CellTrait: Clone + Debug {
    type Domain: DomainTrait;
    type Group: GroupTrait;
    type CellReader: CellReaderTrait;
    type CellWriter: CellWriterTrait;

    fn domain(&self) -> Res<Self::Domain>;
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

pub trait DomainTrait: Debug + SaveTrait {
    type Cell: CellTrait;

    fn interpretation(&self) -> &str;

    fn root(&self) -> Res<Self::Cell>;
}

pub trait SaveTrait: Debug {
    fn write_policy(&self) -> Res<WritePolicy> {
        unimplemented()
    }

    fn set_write_policy(&mut self, policy: WritePolicy) -> Res<()> {
        unimplemented()
    }

    fn save(&self, target: SaveTarget) -> Res<()> {
        unimplemented()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum WritePolicy {
    // no write access
    ReadOnly,
    // write access, but no automatic save
    ExplicitWrite,
    // write access, automatic save on every change
    WriteThrough,
    // write access, automatic save when all references are dropped
    WriteBackOnDrop,
}

#[derive(Clone, Debug)]
pub enum SaveTarget {
    // save to the domain origin or source
    Origin,
    // save to a new target cell
    Cell(Cell),
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
        unimplemented()
    }

    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        unimplemented()
    }

    fn delete(&mut self) -> Res<()> {
        unimplemented()
    }
}

pub trait GroupTrait: Clone + Debug {
    type Cell: CellTrait;
    // type SelectIterator: Iterator<Item = Res<Self::Cell>>;

    fn label_type(&self) -> LabelType;
    fn len(&self) -> Res<usize>;
    fn is_empty(&self) -> bool {
        self.len() == Ok(0)
    }
    fn at(&self, index: usize) -> Res<Self::Cell>;
    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::Cell>;
    // fn get_all<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::SelectIterator>;

    fn add(&mut self) -> Res<()> {
        unimplemented()
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
