use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use linkme::distributed_slice;

use crate::base::{Cell as XCell, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_PATH: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["path"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Data(PathBuf, String);

#[derive(Clone, Debug)]
pub(crate) struct Cell(Rc<Data>);

#[derive(Debug)]
pub(crate) struct CellReader(Rc<Data>);

#[derive(Debug)]
pub(crate) struct CellWriter(Rc<Data>);

impl Cell {
    pub(crate) fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Self::make_cell(PathBuf::from(value), value.to_owned(), Some(cell))
            }
            "fs" => {
                let path = cell.as_file_path()?;
                Self::make_cell(
                    path.to_owned(),
                    path.to_string_lossy().into_owned(),
                    Some(cell),
                )
            }
            _ => nores(),
        }
    }

    fn make_cell(path: PathBuf, string: String, origin: Option<XCell>) -> Res<XCell> {
        let path_cell = Cell(Rc::new(Data(path, string)));
        Ok(new_cell(DynCell::from(path_cell), origin))
    }

    pub(crate) fn as_path(&self) -> Res<&Path> {
        Ok(self.0 .0.as_path())
    }
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(Value::Str(&self.0 .1))
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn index(&self) -> Res<usize> {
        nores()
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        todo!() // remove this default implementation
    }
}

impl CellTrait for Cell {
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "path"
    }

    fn ty(&self) -> Res<&str> {
        Ok("path")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0.clone()))
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter(self.0.clone()))
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }
}
