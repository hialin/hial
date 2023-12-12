use std::rc::Rc;

use lazy_static::lazy_static;

use crate::{
    base::*, guard_ok, guard_some, interpretations::*, utils::vecmap::VecMap, verbose_error,
};

type ConstructorFn = fn(RawDataContainer) -> Res<Cell>;
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

macro_rules! add_elevation {
    ($ret:ident, $interp:literal, |$cellname:ident, $strname:ident| $body:block) => {
        $ret.put($interp, |$cellname: Cell| -> Res<Cell> {
            let vref = $cellname.value();
            println!("••• {:?}", vref.get());
            if let Value::Str($strname) = vref.get()? {
                let domain = ($body)?;
                let root = domain.root()?;
                return Ok(Cell {
                    domain: Rc::new(Domain {
                        this: DynDomain::from(domain),
                        source: Some($cellname.clone()),
                    }),
                    this: EnCell::Dyn(DynCell::from(root)),
                    prev: Some((Box::new($cellname), Relation::Interpretation)),
                });
            }
            HErr::internal("elevation: not a string").into()
        });
    };
}

fn value_elevation_map() -> VecMap<&'static str, ElevateFn> {
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    add_elevation!(ret, "url", |cell, s| { url::from_string(s) });
    add_elevation!(ret, "file", |cell, s| {
        file::Domain::new_from(cell.domain().interpretation(), cell.raw()?)
    });
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

fn file_elevation_map() -> VecMap<&'static str, ElevateFn> {
    fn get_path(cell: &Cell) -> Res<&std::path::Path> {
        if let Cell {
            this: EnCell::Dyn(DynCell::File(ref file)),
            ..
        } = cell
        {
            return file::get_path(file);
        }
        HErr::internal("elevation: not a file").into()
    }
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    add_elevation!(ret, "json", |cell, s| { json::from_path(get_path(&cell)?) });
    add_elevation!(ret, "toml", |cell, s| { toml::from_path(get_path(&cell)?) });
    add_elevation!(ret, "yaml", |cell, s| { yaml::from_path(get_path(&cell)?) });
    add_elevation!(ret, "xml", |cell, s| { xml::from_path(get_path(&cell)?) });
    add_elevation!(ret, "rust", |cell, s| {
        treesitter::from_path(get_path(&cell)?, "rust")
    });
    ret
}

fn url_elevation_map() -> VecMap<&'static str, ElevateFn> {
    fn get_url(cell: &Cell) -> Res<&str> {
        if let Cell {
            this: EnCell::Dyn(DynCell::Url(ref url)),
            ..
        } = cell
        {
            return Ok(url.as_str());
        }
        HErr::internal("elevation: not a url").into()
    }
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    add_elevation!(ret, "http", |cell, s| {
        http::from_string(get_url(&cell)?)
    });
    ret
}

fn http_elevation_map() -> VecMap<&'static str, ElevateFn> {
    fn http_as_string(cell: &Cell) -> Res<String> {
        if let Cell {
            this: EnCell::Dyn(DynCell::Http(ref h)),
            ..
        } = cell
        {
            return http::to_string(h);
        }
        HErr::internal("elevation: not a http cell").into()
    }
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    add_elevation!(ret, "json", |cell, s| {
        json::from_string(&http_as_string(&cell)?)
    });
    add_elevation!(ret, "xml", |cell, s| {
        xml::from_string(&http_as_string(&cell)?)
    });
    ret
}

fn rust_elevation_map() -> VecMap<&'static str, ElevateFn> {
    let mut ret: VecMap<&'static str, ElevateFn> = VecMap::new();
    fn ts_as_string(cell: &Cell) -> Res<String> {
        if let Cell {
            this: EnCell::Dyn(DynCell::TreeSitter(ref ts)),
            ..
        } = cell
        {
            let s = treesitter::get_underlying_string(&ts)?;
            Ok(s.to_string())
        } else {
            HErr::internal("rust to string elevation").into()
        }
    }
    add_elevation!(ret, "string", |cell, s| {
        let s = ts_as_string(&cell)?;
        let cell = ownedvalue::Cell::from(s);
        Res::Ok(cell.domain().clone())
    });
    ret
}

pub(crate) fn standard_interpretation(cell: &Cell) -> Option<&str> {
    if cell.domain().interpretation() == "file" && cell.typ().ok()? == "file" {
        let nameref = cell.label();
        if let Ok(Value::Str(name)) = nameref.get() {
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
    if cell.domain().interpretation() == "value" {
        let vref = cell.value();
        if let Ok(Value::Str(s)) = vref.get() {
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
        if let Some(e) = get_elevation_map_for(old_interp) {
            if let Some(func_tuple) = e.get(interp) {
                println!("elevate {} to {}", old_interp, key);
                return func_tuple.2(self.0.clone());
            }
        }
        if key == "value" {
            println!("elevate default {} to value", old_interp);
            let vref = self.0.value();
            return Ok(Cell::from(vref.get()?.to_owned_value()));
        }
        println!("no elevation from {} to {}", old_interp, interp);
        NotFound::NoInterpretation(format!("no elevation from {} to {}", old_interp, interp)).into()
    }
}
