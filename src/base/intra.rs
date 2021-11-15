use super::*;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

pub trait InDomain: Clone + Debug {
    type Cell: InCell;
    type Group: InGroup;
    // type Trace: InTrace;

    fn root(self: &Rc<Self>) -> Res<Self::Cell>;
    // fn cell(&self, trace: &Self::Trace) -> Self::Cell;

    //fn origin(self: &Rc<Self>) -> Res<Path>;
    // fn save_to_origin(self: &Rc<Self>) -> Res<()>;
    // fn save_to(self: &Rc<Self>, target: Rc<InDomain>) -> Res<()>;
}

pub trait InTrace: Clone + Debug {}

pub trait InRef<'v>: Debug + Deref<Target = Value<'v>> {}

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

    fn domain(&self) -> &Rc<Self::Domain>;

    fn typ(&self) -> Res<&str>;
    fn index(&self) -> Res<usize>;
    fn label(&self) -> Res<&str>;
    fn value(&self) -> Res<Value>;

    fn sub(&self) -> Res<<Self::Domain as InDomain>::Group>;
    fn attr(&self) -> Res<<Self::Domain as InDomain>::Group>;

    fn set(&mut self, value: OwnedValue) -> Res<()> {
        todo!();
    }
    fn delete(&mut self) -> Res<()> {
        todo!();
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
