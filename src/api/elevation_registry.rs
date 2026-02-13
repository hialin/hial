use std::{sync::Arc, sync::RwLock};

use indexmap::IndexMap;

use crate::{api::*, guard_ok, guard_some, warning};

use linkme::distributed_slice;

pub type ElevateParams = IndexMap<OwnValue, OwnValue>;
pub type ElevateFn =
    fn(source: Xell, target_interpretation: &'static str, params: &ElevateParams) -> Res<Xell>;
type TargetMap = IndexMap<&'static str, ElevateFn>;
type ElevationRegistry = IndexMap<&'static str, Arc<TargetMap>>;

#[derive(Debug, Clone)]
pub struct ElevationConstructor {
    pub source_interpretations: &'static [&'static str],
    pub target_interpretations: &'static [&'static str],
    pub constructor: ElevateFn,
}

#[distributed_slice]
pub static ELEVATION_CONSTRUCTORS: [ElevationConstructor];

static ELEVATION_REGISTRY: RwLock<Option<ElevationRegistry>> = RwLock::new(None);

pub(super) fn elevation_registry(
    interpretation: &str,
) -> Res<Arc<IndexMap<&'static str, ElevateFn>>> {
    fn read(interpretation: &str) -> Option<Res<Arc<IndexMap<&'static str, ElevateFn>>>> {
        let maybe_map = guard_ok!(ELEVATION_REGISTRY.read(), err => {
            return Some(Err(caused(HErrKind::Internal, "elevation registry read lock error",  err)));
        });
        let reader = guard_some!(maybe_map.as_ref(), { return None });
        // debug!("-- elevation map {:?} -> {:?}",
        //     interpretation,
        //     reader
        //         .get(interpretation)
        //         .map(|m| m.keys().collect::<Vec<_>>())
        // );
        reader.get(interpretation).map(|e| Ok(e.clone()))
    }

    if let Some(x) = read(interpretation) {
        return x;
    }
    init_elevation_registry()?;
    if let Some(x) = read(interpretation) {
        return x;
    }
    nores()
}

fn init_elevation_registry() -> Res<()> {
    let mut writer = guard_ok!(ELEVATION_REGISTRY.write(), err => {
        return Err(caused(HErrKind::Internal, "elevation map read lock error", err));
    });

    // check first, it could have been initialized inbetween
    if writer.is_some() {
        return Ok(());
    }

    let mut source_map: IndexMap<&'static str, IndexMap<&'static str, ElevateFn>> = IndexMap::new();

    for ec in ELEVATION_CONSTRUCTORS {
        for source_interpretation in ec.source_interpretations {
            let target_map = source_map.entry(source_interpretation).or_default();
            for target_interpretation in ec.target_interpretations {
                if target_map.contains_key(target_interpretation) {
                    warning!(
                        "elevation map: {} -> {} already exists",
                        source_interpretation,
                        target_interpretation
                    );
                } else {
                    target_map.insert(target_interpretation, ec.constructor);
                }
            }
        }
    }

    let mut final_source_map = IndexMap::new();
    for (source, target_map) in source_map {
        final_source_map.insert(source, Arc::new(target_map.clone()));
    }
    // for (k, v) in final_source_map.iter() {
    //     let v = v.keys().collect::<Vec<_>>();
    //     debug!("init elevation_registry {:?} -> {:?}", k, v);
    // }
    *writer = Some(final_source_map);
    Ok(())
}

pub(crate) fn auto_interpretation(cell: &Xell) -> Option<&str> {
    if cell.interpretation() == "fs"
        && cell.read().ty().ok()? == "file"
        && let Ok(reader) = cell.read().err()
        && let Ok(Value::Str(name)) = reader.label()
    {
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
    if cell.interpretation() == "value"
        && let Ok(reader) = cell.read().err()
        && let Ok(Value::Str(s)) = reader.value()
    {
        if s.starts_with("http://") || s.starts_with("https://") {
            return Some("http");
        } else if s.starts_with('.') || s.starts_with('/') {
            return Some("fs");
        }
    }
    None
}
