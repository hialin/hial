use std::rc::Rc;

use linkme::distributed_slice;
use reqwest::Url;
use url::ParseError;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_URL: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value"],
    target_interpretations: &["url"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Data {
    url: Url,
}

#[derive(Clone, Debug)]
pub(crate) struct Cell(Rc<Data>);

#[derive(Debug)]
pub(crate) struct CellReader(Rc<Data>);

#[derive(Debug)]
pub(crate) struct CellWriter {}

implement_try_from_xell!(Cell, Url);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        let r = origin.read();
        let v = r.value()?;
        let cow = v.as_cow_str();
        let value = cow.as_ref();
        let url_cell = Cell(Rc::new(Data {
            url: Url::parse(value)?,
        }));
        Ok(Xell::new_from(DynCell::from(url_cell), Some(origin)))
    }

    pub(crate) fn from_str(s: &str) -> Res<Xell> {
        let url_cell = Cell(Rc::new(Data {
            url: Url::parse(s)?,
        }));
        Ok(Xell::new_from(DynCell::from(url_cell), None))
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        Ok("url")
    }

    fn value(&self) -> Res<Value> {
        Ok(Value::Str(self.0.url.as_str()))
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
    pub(crate) fn as_url(&self) -> Res<Url> {
        Ok(self.0.url.clone())
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match value {
            OwnValue::String(s) => {
                let url = Url::parse(s.as_str())?;
                Ok(())
            }
            _ => userres(format!("cannot set url from non-string value {:?}", value)),
        }
    }
}

impl CellTrait for Cell {
    type Group = VoidGroup<Self>;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "url"
    }

    fn read(&self) -> Res<CellReader> {
        Ok(CellReader(self.0.clone()))
    }

    fn write(&self) -> Res<CellWriter> {
        Ok(CellWriter {})
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }
}

impl From<ParseError> for HErr {
    fn from(e: ParseError) -> HErr {
        caused(HErrKind::InvalidFormat, "cannot parse url", e)
    }
}
