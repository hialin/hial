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
    // type Trace: InTrace;

    fn interpretation(&self) -> &str;

    // fn accepts(source_interpretation: &str, source: DataSource) -> Res<bool>;
    fn new_from(source_interpretation: &str, source: RawDataContainer) -> Res<Self> {
        todo!()
    }

    // TODO: this should return a group: json objects/arrays; xml root elements
    fn root(&self) -> Res<Self::Cell>;
    // fn cell(&self, trace: &Self::Trace) -> Self::Cell;

    // fn origin(&self) -> Res<Path>;
    // fn save_to_origin(&self) -> Res<()>;
    // fn save_to(&self, target: &InDomain>) -> Res<()>;
}

pub trait InTrace: Clone + Debug {}

pub trait InCell: Clone + Debug {
    type Domain: InDomain;
    type ValueRef: InValueRef;

    fn domain(&self) -> &Self::Domain;

    fn typ(&self) -> Res<&str>;
    fn index(&self) -> Res<usize>;
    fn label(&self) -> Self::ValueRef;
    fn value(&self) -> Self::ValueRef;

    fn sub(&self) -> Res<<Self::Domain as InDomain>::Group>;
    fn attr(&self) -> Res<<Self::Domain as InDomain>::Group>;

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

pub trait InValueRef: Debug {
    fn get(&self) -> Res<Value>;

    fn with<T>(&self, f: impl Fn(Res<Value>) -> T) -> T {
        f(self.get())
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

    // fn add(&mut self) -> Res<()> {
    //     todo!();
    // }
}

#[derive(Clone, Debug)]
pub struct DummyTrace {}
impl InTrace for DummyTrace {}

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
        NotFound::NoResult(format!("")).into()
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<D::Cell> {
        NotFound::NoResult(format!("")).into()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct LabelType {
    pub is_indexed: bool,
    pub unique_labels: bool,
}
