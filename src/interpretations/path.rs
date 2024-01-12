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

#[derive(Clone, Debug)]
pub struct Domain(Rc<(PathBuf, String, Option<XCell>)>);

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "path"
    }

    fn root(&self) -> Res<Self::Cell> {
        Ok(Cell(self.clone()))
    }

    fn origin(&self) -> Res<XCell> {
        self.0 .2.as_ref().ok_or(HErr::None).map(|c| c.clone())
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

#[derive(Clone, Debug)]
pub struct Cell(Domain);

#[derive(Debug)]
pub struct CellReader(Domain);

#[derive(Debug)]
pub struct CellWriter {}
impl CellWriterTrait for CellWriter {}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        match cell.domain().interpretation() {
            "value" => {
                let r = cell.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Self::make_cell(PathBuf::from(value), value.to_owned(), Some(cell))
            }
            "file" => {
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

    pub fn from_string(url: impl Into<String>) -> Res<XCell> {
        let url = url.into();
        let path = PathBuf::from(&url);
        Self::make_cell(path, url, None)
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Res<XCell> {
        let path = path.into();
        Self::make_cell(path.clone(), path.to_string_lossy().into_owned(), None)
    }

    fn make_cell(path: PathBuf, string: String, origin: Option<XCell>) -> Res<XCell> {
        let domain = Domain(Rc::new((path, string, origin)));
        Ok(XCell {
            dyn_cell: DynCell::from(domain.root()?),
        })
    }

    pub fn as_path(&self) -> Res<&Path> {
        Ok(self.0 .0 .0.as_path())
    }
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
