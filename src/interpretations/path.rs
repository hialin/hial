use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use linkme::distributed_slice;

use crate::base::{Cell as XCell, *};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_PATH: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "file"],
    target_interpretations: &["path"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Domain(Rc<(PathBuf, String)>);

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "path"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell(self.clone()))
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell(Domain);

#[derive(Debug)]
pub struct CellReader(Domain);

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.domain().interpretation() {
            "value" => Cell::from_string(cell.read().value()?.to_string()),
            "file" => Cell::from_path(cell.as_file_path()?),
            _ => nores(),
        }
    }

    pub fn from_string(url: impl Into<String>) -> Res<XCell> {
        let path_cell = from_string(url.into())?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(path_cell),
        })
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Res<XCell> {
        let path_cell = from_path(path.into())?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(path_cell),
        })
    }

    pub fn as_path(&self) -> Res<&Path> {
        Ok(self.0 .0 .0.as_path())
    }
}

fn from_string(s: impl Into<String>) -> Res<Domain> {
    let s = s.into();
    let data = (PathBuf::from(&s), s);
    Ok(Domain(Rc::new(data)))
}

fn from_path(s: impl Into<PathBuf>) -> Res<Domain> {
    let path = s.into();
    let s = path.to_string_lossy().to_string();
    Ok(Domain(Rc::new((path, s))))
}

impl CellReaderTrait for CellReader {
    fn value(&self) -> Res<Value> {
        Ok(Value::Str(&self.0 .0 .1))
    }
}

impl CellTrait for Cell {
    type Domain = Domain;
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn domain(&self) -> Domain {
        self.0.clone()
    }

    fn typ(&self) -> Res<&str> {
        Ok("path")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader(self.0.clone()))
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }
}
