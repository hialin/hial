use crate::{
    base::*, guard_ok, guard_some, interpretations::*, utils::vecmap::VecMap, verbose_error,
};
use lazy_static::lazy_static;
use std::path::Path;

type ConstructorFn = fn(DataSource) -> Res<Cell>;
type ElevateFn = fn(Cell) -> Res<Cell>;
type VecMapOfElevateFn = VecMap<&'static str, ElevateFn>;

fn get_elevation_map_for(interpretation: &str) -> Option<VecMapOfElevateFn> {
    use std::sync::RwLock;
    lazy_static! {
        static ref ELEVATION_MAP: RwLock<Option<VecMap<&'static str, VecMapOfElevateFn>>> =
            RwLock::new(None);
    }

    {
        // try reading
        let reader_opt = guard_ok!(ELEVATION_MAP.read(), err => {
            verbose_error(HErr::internal(format!("{:?}", err)));
            return None;
        });
        if let Some(reader) = reader_opt.as_ref() {
            return if let Some((_, _, e)) = reader.get(interpretation).as_ref() {
                Some((**e).clone())
            } else {
                None
            };
        }
    }

    let mut writer = guard_ok!(ELEVATION_MAP.write(), err => {
        verbose_error(HErr::internal(format!("{:?}", err)));
        return None;
    });
    if writer.is_none() {
        // could have been initialized inbetween
        let mut map = VecMap::new();
        map.put("value", value_elevation_map());
        map.put("file", file_elevation_map());
        map.put("url", url_elevation_map());
        map.put("http", http_elevation_map());
        map.put("rust", rust_elevation_map());

        *writer = Some(map);
    }
    if let Some((_, _, e)) = writer.as_ref().unwrap().get(interpretation) {
        Some((*e).clone())
    } else {
        None
    }
}

fn value_elevation_map() -> VecMap<&'static str, ElevateFn> {
    fn get_string_value(cell: &Cell) -> Res<&str> {
        if let Value::Str(s) = cell.value()? {
            return Ok(s);
        }
        HErr::internal("elevation: not a string").into()
    }
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    ret.put("url", |cell: Cell| {
        Ok(Cell::from(url::from_string(get_string_value(&cell)?)?))
    });
    ret.put("file", |cell: Cell| -> Res<Cell> {
        let datasource = guard_some!(cell.as_data_source(), {
            return HErr::internal("no data source").into();
        })?;
        let ds = if let DataSource::String(s) = datasource {
            DataSource::File(Path::new(s))
        } else {
            datasource
        };
        let domain = file::Domain::new_from(cell.domain().interpretation(), ds)?;
        Ok(Cell::from(domain.root()?))
    });
    ret.put("json", |cell: Cell| -> Res<Cell> {
        Ok(Cell::from(json::from_string(get_string_value(&cell)?)?))
    });
    ret.put("toml", |cell: Cell| -> Res<Cell> {
        Ok(Cell::from(toml::from_string(get_string_value(&cell)?)?))
    });
    ret.put("yaml", |cell: Cell| -> Res<Cell> {
        Ok(Cell::from(yaml::from_string(get_string_value(&cell)?)?))
    });
    ret.put("xml", |cell: Cell| -> Res<Cell> {
        Ok(Cell::from(xml::from_string(get_string_value(&cell)?)?))
    });
    ret.put("http", |cell: Cell| -> Res<Cell> {
        Ok(Cell::from(http::from_string(get_string_value(&cell)?)?))
    });
    ret.put("rust", |cell: Cell| -> Res<Cell> {
        Ok(Cell::from(treesitter::from_string(
            get_string_value(&cell)?.to_string(),
            "rust",
        )?))
    });
    ret
}

fn file_elevation_map() -> VecMap<&'static str, ElevateFn> {
    fn get_path(cell: &Cell) -> Res<&std::path::Path> {
        if let Cell::File(ref file) = cell {
            return file::get_path(file);
        }
        HErr::internal("elevation: not a file").into()
    }
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    ret.put("json", |cell: Cell| {
        let path = get_path(&cell)?;
        let json = json::from_path(path)?;
        return Ok(Cell::from(json));
    });
    ret.put("toml", |cell: Cell| {
        let path = get_path(&cell)?;
        let toml = toml::from_path(path)?;
        return Ok(Cell::from(toml));
    });
    ret.put("yaml", |cell: Cell| {
        let path = get_path(&cell)?;
        let yaml = yaml::from_path(path)?;
        return Ok(Cell::from(yaml));
    });
    ret.put("xml", |cell: Cell| {
        let path = get_path(&cell)?;
        let xml = xml::from_path(path)?;
        return Ok(Cell::from(xml));
    });
    ret.put("rust", |cell: Cell| {
        let path = get_path(&cell)?;
        let rust = treesitter::from_path(path, "rust")?;
        return Ok(Cell::from(rust));
    });
    ret
}

fn url_elevation_map() -> VecMap<&'static str, ElevateFn> {
    fn get_url(cell: &Cell) -> Res<&str> {
        if let Cell::Url(url) = cell {
            return Ok(url.as_str());
        }
        HErr::internal("elevation: not a url").into()
    }
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    ret.put("http", |cell: Cell| {
        let http = http::from_string(get_url(&cell)?)?;
        return Ok(Cell::from(http));
    });
    ret
}

fn http_elevation_map() -> VecMap<&'static str, ElevateFn> {
    fn http_as_string(cell: &Cell) -> Res<String> {
        if let Cell::Http(ref h) = cell {
            return http::to_string(h);
        }
        HErr::internal("elevation: not a http cell").into()
    }
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    // ret.put("value", |cell: Cell| {
    //     return Ok(Cell::OwnedValue(OwnedValue::Bytes(cell.value()?)));
    // });
    ret.put("json", |cell: Cell| {
        let string = http_as_string(&cell)?;
        return Ok(Cell::from(json::from_string(&string)?));
    });
    ret.put("xml", |cell: Cell| {
        let string = http_as_string(&cell)?;
        return Ok(Cell::from(xml::from_string(&string)?));
    });
    ret
}

fn rust_elevation_map() -> VecMap<&'static str, ElevateFn> {
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    ret.put("string", |cell: Cell| {
        if let Cell::TreeSitter(ts) = cell {
            let s = treesitter::get_underlying_string(&ts)?;
            let ovcell = ownedvalue::Cell::from(s.to_string());
            Ok(Cell::from(ovcell))
        } else {
            HErr::internal("rust to string elevation").into()
        }
    });
    ret
}

fn standard_interpretation(cell: &Cell) -> Option<&str> {
    if cell.domain().interpretation() == "file" && cell.typ().ok()? == "file" {
        let name = cell.label().ok()?;
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
    if cell.domain().interpretation() == "value" {
        if let Ok(Value::Str(s)) = cell.value() {
            if s.starts_with("http://") || s.starts_with("https://") {
                return Some("http");
            } else if s.starts_with(".") || s.starts_with("/") {
                return Some("file");
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct ElevationGroup(pub Cell);

#[derive(Debug, Clone)]
pub struct Mixed(pub Cell);

impl ElevationGroup {
    pub fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    pub fn len(&self) -> usize {
        if let Some(e) = get_elevation_map_for(self.0.domain().interpretation()) {
            e.len()
        } else {
            0
        }
    }

    pub fn at(&self, index: usize) -> Res<Cell> {
        let domain = self.0.domain();
        let interp = domain.interpretation();
        if let Some(e) = get_elevation_map_for(interp) {
            if let Some(func_tuple) = e.at(index) {
                func_tuple.1(self.0.clone())
            } else {
                NotFound::NoInterpretation(format!("index {}", index)).into()
            }
        } else {
            HErr::internal(format!("no elevation map (at) for {}", interp)).into()
        }
    }

    pub fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        let domain = self.0.domain();
        let old_interp = domain.interpretation();
        let interp = match key {
            Selector::Str(k) => k,
            Selector::Top => guard_some!(standard_interpretation(&self.0), {
                return NotFound::NoInterpretation(format!("'^'")).into();
            }),
            Selector::Star => {
                return HErr::BadArgument(format!("no interpretation for '*'")).into()
            }
            Selector::DoubleStar => {
                return HErr::BadArgument(format!("no interpretation for '**'")).into()
            }
        };
        if interp == old_interp {
            return Ok(self.0.clone());
        }
        println!("get elevation from {} to {}", old_interp, key);
        if let Some(e) = get_elevation_map_for(old_interp) {
            if let Some(func_tuple) = e.get(interp) {
                func_tuple.2(self.0.clone())
            } else {
                NotFound::NoResult(format!("")).into()
            }
        } else {
            return NotFound::NoInterpretation(format!("no elevation map for this interpretation"))
                .into();
        }
    }
}
