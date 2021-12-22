use std::borrow::Cow;
use std::marker::PhantomData;
use std::{fmt::Debug, path::Path};

use crate::base::*;

#[derive(Clone, Debug)]
pub enum DataSource<'s> {
    File(Cow<'s, Path>),
    String(Cow<'s, str>),
}

#[derive(Debug)]
pub enum DataDestination<'s> {
    File(&'s Path),
    String(&'s mut String),
}

pub trait InDomain: Clone + Debug {
    type Cell: InCell;
    type Group: InGroup;
    // type Trace: InTrace;

    fn interpretation(&self) -> &str;

    // fn accepts(source_interpretation: &str, source: DataSource) -> Res<bool>;
    fn new_from(source_interpretation: &str, source: DataSource) -> Res<Self> {
        todo!()
    }

    fn root(&self) -> Res<Self::Cell>;
    // fn cell(&self, trace: &Self::Trace) -> Self::Cell;

    //fn origin(&self) -> Res<Path>;
    // fn save_to_origin(&self) -> Res<()>;
    // fn save_to(&self, target: &InDomain>) -> Res<()>;
}

pub trait InTrace: Clone + Debug {}

// pub trait InRef<'v>: Debug + Deref<Target = Value<'v>> {}

// - dereference of a value must be a safe op in Rust, can be less safe in C
// - all cells must be invalidated when the domain is freed
// - creating a cell should not allocate: cells must not be counted (gc languages)

// implementation options for value dereference with mutable values:
// 1. cell returns Ref object -- cannot add it in trait due to associated lifetime limitations
// 2. cell returns direct value -- how do we replace it then?

// remove/move cells:
// 1. invalidate all affected cells         -> ok, but then you need a trace to get back to them
// 2. changes are stored in a new subtree   -> NO, because then you won't be able to read the new value if you're on a cloned cell
// 3. invalidate only the removed cells     -> yes, but how?
// 4. operation is refused if more than one cell points to the data -> let's work on this one

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

    fn set_value(&mut self, value: OwnedValue) -> Res<()> {
        todo!();
    }
    fn set_label(&mut self, value: OwnedValue) -> Res<()> {
        todo!();
    }
    fn delete(&mut self) -> Res<()> {
        todo!();
    }

    // todo remove this and replace it with raw_write_to
    fn as_data_source(&self) -> Option<Res<DataSource>> {
        todo!()
    }

    fn raw_write_to(&self, destination: DataDestination) {
        todo!()
    }

    // fn as_data_destination(&mut self) -> Option<Res<DataDestination>> {
    //     todo!()
    // }
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
