use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::base::*;

// TODO: is this needed? remove this
// data container alternatives for serialized forms
#[derive(Clone, Debug)]
pub enum RawDataContainer {
    File(PathBuf),
    String(String),
}

pub trait InDomain: Clone + Debug {
    type Cell: InCell;
    type Group: InGroup;

    fn interpretation(&self) -> &str;

    // fn accepts(source_interpretation: &str, source: DataSource) -> Res<bool>;
    fn new_from(source_interpretation: &str, source: RawDataContainer) -> Res<Self> {
        todo!()
    }

    fn root(&self) -> Res<Self::Cell>;

    // fn origin(&self) -> Res<Path>;
    // fn save_to_origin(&self) -> Res<()>;
    // fn save_to(&self, target: &InDomain>) -> Res<()>;
}

pub trait InCell: Clone + Debug {
    type Domain: InDomain;
    type CellReader: InCellReader;

    fn domain(&self) -> &Self::Domain;

    fn typ(&self) -> Res<&str>;
    fn read(&self) -> Res<Self::CellReader>;

    fn sub(&self) -> Res<<Self::Domain as InDomain>::Group> {
        nores()
    }
    fn attr(&self) -> Res<<Self::Domain as InDomain>::Group> {
        nores()
    }

    // get serialized data for this subtree
    fn raw(&self) -> Res<RawDataContainer> {
        todo!()
    }

    // set the subtree as serialized data
    fn set_raw(&self, raw: RawDataContainer) -> Res<()> {
        todo!()
    }

    fn set_value(&mut self, value: OwnedValue) -> Res<()> {
        todo!();
    }
    fn set_label(&mut self, value: OwnedValue) -> Res<()> {
        todo!();
    }

    fn delete(&mut self) -> Res<()> {
        todo!();
    }
}

pub trait InCellReader: Debug {
    fn index(&self) -> Res<usize> {
        nores()
    }
    fn label(&self) -> Res<Value> {
        nores()
    }
    fn value(&self) -> Res<Value> {
        nores()
    }
}

// pub trait InCellWriter: Debug {
// }

pub trait InGroup: Clone + Debug {
    type Domain: InDomain;
    // type SelectIterator: Iterator<Item = Res<Self::Cell>>;

    fn label_type(&self) -> LabelType;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn at(&self, index: usize) -> Res<<Self::Domain as InDomain>::Cell>;
    fn get<'s, 'a, S: Into<Selector<'a>>>(
        &'s self,
        label: S,
    ) -> Res<<Self::Domain as InDomain>::Cell>;
    // fn get_all<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::SelectIterator>;

    // fn add(&mut self) -> Res<()> {
    //     todo!();
    // }
}

#[derive(Clone, Debug)]
pub struct VoidGroup<D>(PhantomData<D>);
impl<D> From<D> for VoidGroup<D> {
    fn from(_: D) -> Self {
        VoidGroup(PhantomData)
    }
}
impl<D: InDomain> InGroup for VoidGroup<D> {
    type Domain = D;

    fn label_type(&self) -> LabelType {
        LabelType::default()
    }

    fn len(&self) -> usize {
        0
    }

    fn at(&self, index: usize) -> Res<D::Cell> {
        nores()
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<D::Cell> {
        nores()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct LabelType {
    pub is_indexed: bool,
    pub unique_labels: bool,
}
