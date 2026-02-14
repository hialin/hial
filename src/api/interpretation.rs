use std::{fmt::Debug, io::Read, marker::PhantomData};

use crate::api::*;

pub trait CellTrait: Clone + Debug {
    type Group: GroupTrait;
    type CellReader: CellReaderTrait;
    type CellWriter: CellWriterTrait;

    fn interpretation(&self) -> &str;

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
    fn ty(&self) -> Res<&str>;

    fn index(&self) -> Res<usize>;

    fn label(&self) -> Res<Value<'_>>;

    fn value(&self) -> Res<Value<'_>>;

    fn value_read(&self) -> Res<Box<dyn Read + '_>> {
        nores()
    }

    /// provide a serialization of the data from the cell and its descendants
    /// to be used for saving the data to a data store.
    /// If this returns HErrKind::None, then no data needs to be saved
    /// (i.e. the data is already fully persisted).
    fn serial(&self) -> Res<String>;
}

pub trait CellWriterTrait: Debug {
    fn set_ty(&mut self, new_type: &str) -> Res<()> {
        todo!() // TODO: remove this default implementation
    }

    fn set_index(&mut self, value: usize) -> Res<()> {
        todo!() // TODO: remove this default implementation
    }

    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        todo!() // TODO: remove this default implementation
    }

    fn set_value(&mut self, value: OwnValue) -> Res<()>;

    fn set_serial(&mut self, value: OwnValue) -> Res<()> {
        todo!() // TODO: remove this default implementation
    }

    /// creates a "detached" cell, use "add" to add it to the group
    /// or leave it detached to delete it
    fn detach(&mut self) -> Res<()> {
        nores() // interpretations do not support removing cells by default
    }
}

pub trait GroupTrait: Clone + Debug {
    type Cell: CellTrait;
    type CellIterator: DoubleEndedIterator<Item = Res<Self::Cell>>;

    fn label_type(&self) -> LabelType;
    fn len(&self) -> Res<usize>;
    fn is_empty(&self) -> bool {
        self.len().is_ok_and(|l| l == 0)
    }
    fn at(&self, index: usize) -> Res<Self::Cell>;
    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator>;

    // creates a "detached" cell, use "add" to add it to the group
    fn create(&self, label: Option<OwnValue>, value: Option<OwnValue>) -> Res<Self::Cell> {
        nores() // interpretations do not support creating cells by default
    }

    fn add(&self, index: Option<usize>, cell: Self::Cell) -> Res<()> {
        nores() // interpretations do not support adding cells by default
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct LabelType {
    pub is_indexed: bool,
    pub unique_labels: bool,
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
    type CellIterator = std::iter::Empty<Res<C>>;

    fn label_type(&self) -> LabelType {
        LabelType::default()
    }

    fn len(&self) -> Res<usize> {
        Ok(0)
    }

    fn at(&self, index: usize) -> Res<C> {
        nores()
    }

    fn get_all(&self, label: Value) -> Res<Self::CellIterator> {
        nores()
    }
}

#[macro_export]
macro_rules! implement_try_from_xell {
    ($local_cell_type:ident, $xell_enum_type:ident) => {
        impl TryFrom<Xell> for $local_cell_type {
            type Error = HErr;

            fn try_from(x: Xell) -> Res<Self> {
                if let DynCell::$xell_enum_type(c) = x.dyn_cell {
                    Ok(c)
                } else {
                    userres(format!(
                        "cannot convert cell {} to {}",
                        x.interpretation(),
                        stringify!($xell_enum_type)
                    ))
                }
            }
        }
    };
}
