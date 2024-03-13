use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
};

use linkme::distributed_slice;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
    utils::ownrc::{OwnRc, ReadRc, WriteRc},
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_PATH: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["path"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell(OwnRc<PathBuf>);

#[derive(Debug)]
pub(crate) struct CellReader(ReadRc<PathBuf>, OnceCell<String>);

#[derive(Debug)]
pub(crate) struct CellWriter(WriteRc<PathBuf>);

implement_try_from_xell!(Cell, Path);

impl Cell {
    pub(crate) fn from_cell(cell: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        match cell.interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Self::make_cell(PathBuf::from(value), value.to_owned(), Some(cell))
            }
            "fs" => {
                let r = cell.read();
                let path = r.as_file_path()?;
                Self::make_cell(
                    path.to_owned(),
                    path.to_string_lossy().into_owned(),
                    Some(cell),
                )
            }
            _ => nores(),
        }
    }

    fn make_cell(path: PathBuf, string: String, origin: Option<Xell>) -> Res<Xell> {
        let path_cell = Cell(OwnRc::new(path));
        Ok(Xell::new_from(DynCell::from(path_cell), origin))
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        Ok("path")
    }

    fn value(&self) -> Res<Value> {
        let s = self
            .1
            .get_or_init(|| self.0.as_os_str().to_string_lossy().to_string());
        Ok(Value::Str(s))
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

impl CellReader {
    pub(crate) fn as_file_path(&self) -> Res<&Path> {
        Ok(self.0.as_path())
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match value {
            OwnValue::String(s) => {
                *(self.0) = PathBuf::from(s);
                Ok(())
            }
            _ => userres(format!("cannot set fs path to {:?}", value)),
        }
    }
}

impl CellTrait for Cell {
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "path"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(
            self.0
                .read()
                .ok_or_else(|| lockerr("cannot lock path for reading"))?,
            OnceCell::new(),
        ))
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter(
            self.0
                .write()
                .ok_or_else(|| lockerr("cannot lock path for writing"))?,
        ))
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }
}
