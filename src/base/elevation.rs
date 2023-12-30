use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use lazy_static::lazy_static;

use crate::{base::*, debug, guard_ok, guard_some, interpretations::*, utils::log::verbose_error};

type ElevateFn = fn(Cell) -> Res<Cell>;
type VecMapOfElevateFn = IndexMap<&'static str, ElevateFn>;

fn get_elevation_map_for(interpretation: &str) -> Option<VecMapOfElevateFn> {
    use std::sync::RwLock;
    lazy_static! {
        static ref ELEVATION_MAP: RwLock<Option<IndexMap<&'static str, VecMapOfElevateFn>>> =
            RwLock::new(None);
    }

    {
        // try reading
        let reader_opt = guard_ok!(ELEVATION_MAP.read(), err => {
            verbose_error(HErr::Internal(format!("{:?}", err))); // TODO remove verbose_errors
            return None;
        });
        if let Some(reader) = reader_opt.as_ref() {
            return reader.get(interpretation).as_ref().map(|e| (**e).clone());
        }
    }

    let mut writer = guard_ok!(ELEVATION_MAP.write(), err => {
        verbose_error(HErr::Internal(format!("{:?}", err)));
        return None;
    });
    if writer.is_none() {
        // could have been initialized inbetween
        let mut map = IndexMap::new();
        map.insert("value", value_elevation_map());
        map.insert("file", file_elevation_map());
        map.insert("url", url_elevation_map());
        map.insert("path", path_elevation_map());
        map.insert("http", http_elevation_map());
        map.insert("rust", rust_elevation_map());

        *writer = Some(map);
    }
    writer
        .as_ref()
        .unwrap()
        .get(interpretation)
        .map(|e| (*e).clone())
}

macro_rules! add_elevation {
    ($ret:ident, $interp:literal, |$cellname:ident, $strname:ident| $body:block) => {
        $ret.insert($interp, |$cellname: Cell| -> Res<Cell> {
            let reader = $cellname.read()?;
            println!("••• {:?}", reader.value());
            if let Value::Str($strname) = reader.value()? {
                let domain = ($body)?;
                let root = domain.root()?;
                return Ok(Cell {
                    this: DynCell::from(root),
                });
            }
            fault("elevation: not a string")
        });
    };
}

fn value_elevation_map() -> IndexMap<&'static str, ElevateFn> {
    let mut ret: IndexMap<&'static str, ElevateFn> = IndexMap::new();
    add_elevation!(ret, "url", |cell, s| { url::from_string(s) });
    add_elevation!(ret, "path", |cell, s| { path::from_string(s) });
    add_elevation!(ret, "json", |cell, s| { json::from_string(s) });
    add_elevation!(ret, "toml", |cell, s| { toml::from_string(s) });
    add_elevation!(ret, "yaml", |cell, s| { yaml::from_string(s) });
    add_elevation!(ret, "xml", |cell, s| { xml::from_string(s) });
    add_elevation!(ret, "http", |cell, s| { http::from_string(s) });
    add_elevation!(ret, "rust", |cell, s| {
        treesitter::from_string(s.to_string(), "rust")
    });
    ret
}

fn file_elevation_map() -> IndexMap<&'static str, ElevateFn> {
    fn get_path(cell: &Cell) -> Res<&std::path::Path> {
        if let Cell {
            this: DynCell::File(ref file),
            ..
        } = cell
        {
            return file::get_path(file);
        }
        fault("elevation: not a file")
    }
    let mut ret: IndexMap<&'static str, ElevateFn> = IndexMap::new();
    add_elevation!(ret, "path", |cell, s| { path::from_path(get_path(&cell)?) });
    add_elevation!(ret, "json", |cell, s| { json::from_path(get_path(&cell)?) });
    add_elevation!(ret, "toml", |cell, s| { toml::from_path(get_path(&cell)?) });
    add_elevation!(ret, "yaml", |cell, s| { yaml::from_path(get_path(&cell)?) });
    add_elevation!(ret, "xml", |cell, s| { xml::from_path(get_path(&cell)?) });
    add_elevation!(ret, "rust", |cell, s| {
        treesitter::from_path(get_path(&cell)?, "rust")
    });
    ret
}

fn url_elevation_map() -> IndexMap<&'static str, ElevateFn> {
    fn get_url_as_string(cell: &Cell) -> Res<&str> {
        if let Cell {
            this: DynCell::Url(ref url),
            ..
        } = cell
        {
            return Ok(url.as_str());
        }
        fault("elevation: not a url")
    }
    let mut ret: IndexMap<&'static str, ElevateFn> = IndexMap::new();
    add_elevation!(ret, "file", |cell, s| {
        let path = PathBuf::from(get_url_as_string(&cell)?);
        file::from_path(&path)
    });
    ret
}

fn path_elevation_map() -> IndexMap<&'static str, ElevateFn> {
    fn get_path(cell: &Cell) -> Res<&Path> {
        if let Cell {
            this: DynCell::Path(ref path),
            ..
        } = cell
        {
            return path.as_path();
        }
        fault("elevation: not a url")
    }
    let mut ret: IndexMap<&'static str, ElevateFn> = IndexMap::new();
    add_elevation!(ret, "file", |cell, s| { file::from_path(get_path(&cell)?) });
    ret
}

fn http_elevation_map() -> IndexMap<&'static str, ElevateFn> {
    fn http_as_string(cell: &Cell) -> Res<String> {
        if let Cell {
            this: DynCell::Http(ref h),
            ..
        } = cell
        {
            return http::to_string(h);
        }
        fault("elevation: not a http cell")
    }
    let mut ret: IndexMap<&'static str, ElevateFn> = IndexMap::new();
    add_elevation!(ret, "json", |cell, s| {
        json::from_string(&http_as_string(&cell)?)
    });
    add_elevation!(ret, "xml", |cell, s| {
        xml::from_string(&http_as_string(&cell)?)
    });
    ret
}

fn rust_elevation_map() -> IndexMap<&'static str, ElevateFn> {
    let mut ret: IndexMap<&'static str, ElevateFn> = IndexMap::new();
    fn ts_as_string(cell: &Cell) -> Res<String> {
        if let Cell {
            this: DynCell::TreeSitter(ref ts),
            ..
        } = cell
        {
            let s = treesitter::get_underlying_string(ts)?;
            Ok(s.to_string())
        } else {
            fault("rust to string elevation")
        }
    }
    add_elevation!(ret, "string", |cell, s| {
        let s = ts_as_string(&cell)?;
        let domain = ownvalue::Cell::from(s);
        Res::Ok(domain)
    });
    ret
}

pub(crate) fn standard_interpretation(cell: &Cell) -> Option<&str> {
    if cell.interpretation() == "file" && cell.typ().ok()? == "file" {
        if let Ok(reader) = cell.read() {
            if let Ok(Value::Str(name)) = reader.label() {
                if name.ends_with(".c") {
                    return Some("c");
                } else if name.ends_with(".javascript") {
                    return Some("javascript");
                } else if name.ends_with(".json") {
                    return Some("json");
                } else if name.ends_with(".rs") {
                    return Some("rust");
                } else if name.ends_with(".toml") {
                    return Some("toml");
                } else if name.ends_with(".xml") {
                    return Some("xml");
                } else if name.ends_with(".yaml") || name.ends_with(".yml") {
                    return Some("yaml");
                }
            }
        }
    }
    if cell.interpretation() == "value" {
        if let Ok(reader) = cell.read() {
            if let Ok(Value::Str(s)) = reader.value() {
                if s.starts_with("http://") || s.starts_with("https://") {
                    return Some("http");
                } else if s.starts_with('.') || s.starts_with('/') {
                    return Some("file");
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct ElevationGroup(pub Cell);

impl ElevationGroup {
    pub fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    pub fn len(&self) -> usize {
        if let Some(e) = get_elevation_map_for(self.0.interpretation()) {
            e.len()
        } else {
            0
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn at(&self, index: usize) -> Res<Cell> {
        let interp = self.0.interpretation();
        if let Some(e) = get_elevation_map_for(interp) {
            if let Some(func_tuple) = e.get_index(index) {
                func_tuple.1(self.0.clone())
            } else {
                nores()
            }
        } else {
            fault(format!("no elevation map (at) for {}", interp))
        }
    }

    pub fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        let old_interp = self.0.interpretation();
        let interp = match key {
            Selector::Str(k) => k,
            Selector::Top => guard_some!(standard_interpretation(&self.0), { return nores() }),
            Selector::Star => return HErr::User("no interpretation for '*'".to_string()).into(),
            Selector::DoubleStar => {
                return HErr::User("no interpretation for '**'".to_string()).into()
            }
        };
        if interp == old_interp {
            return Ok(self.0.clone());
        }
        if let Some(e) = get_elevation_map_for(old_interp) {
            if let Some(func) = e.get(interp) {
                debug!("elevate {} to {}", old_interp, key);
                return func(self.0.clone());
            }
        }
        debug!("no elevation from {} to {}", old_interp, interp);
        nores()
    }
}
