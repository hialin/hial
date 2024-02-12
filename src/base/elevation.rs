use std::{sync::Arc, sync::RwLock};

use indexmap::IndexMap;

use crate::{base::*, debug, guard_ok, guard_some, warning};

use linkme::distributed_slice;

pub type ElevateFn = fn(source: Cell, target_interpretation: &'static str) -> Res<Cell>;
type TargetMap = IndexMap<&'static str, ElevateFn>;
type ElevationsMap = IndexMap<&'static str, Arc<TargetMap>>;

#[derive(Debug, Clone)]
pub struct ElevationConstructor {
    pub source_interpretations: &'static [&'static str],
    pub target_interpretations: &'static [&'static str],
    pub constructor: ElevateFn,
}

#[distributed_slice]
pub static ELEVATION_CONSTRUCTORS: [ElevationConstructor];

static ELEVATION_MAP: RwLock<Option<ElevationsMap>> = RwLock::new(None);

fn elevation_map(interpretation: &str) -> Res<Arc<IndexMap<&'static str, ElevateFn>>> {
    fn read(interpretation: &str) -> Option<Res<Arc<IndexMap<&'static str, ElevateFn>>>> {
        let maybe_map = guard_ok!(ELEVATION_MAP.read(), err => {
            return Some(Err(caused(HErrKind::Internal, "elevation map read lock error",  err)));
        });
        let reader = guard_some!(maybe_map.as_ref(), { return None });
        // debug!(
        //     "-- elevation map {:?} -> {:?}",
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

    // initialize the map
    {
        let mut writer = guard_ok!(ELEVATION_MAP.write(), err => {
            return Err(caused(HErrKind::Internal, "elevation map read lock error", err));
        });

        // check first, it could have been initialized inbetween
        if writer.is_none() {
            let mut source_map: IndexMap<&'static str, IndexMap<&'static str, ElevateFn>> =
                IndexMap::new();

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
            //     debug!(
            //         "-- init final elevation map {:?} -> {:?}",
            //         k,
            //         v.keys().collect::<Vec<_>>()
            //     );
            // }
            *writer = Some(final_source_map);
        }
    }
    if let Some(x) = read(interpretation) {
        return x;
    }
    nores()
}

pub(crate) fn top_interpretation(cell: &Cell) -> Option<&str> {
    if cell.interpretation() == "fs" && cell.read().ty().ok()? == "fs" {
        if let Ok(reader) = cell.read().err() {
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
        if let Ok(reader) = cell.read().err() {
            if let Ok(Value::Str(s)) = reader.value() {
                if s.starts_with("http://") || s.starts_with("https://") {
                    return Some("http");
                } else if s.starts_with('.') || s.starts_with('/') {
                    return Some("fs");
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

    pub fn len(&self) -> Res<usize> {
        let e = guard_ok!(elevation_map(self.0.interpretation()), err => { return Err(err) });
        Ok(e.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |x| x == 0)
    }

    pub fn at(&self, index: usize) -> Res<Cell> {
        let e = guard_ok!(elevation_map(self.0.interpretation()), err => { return Err(err) });
        if let Some((target_interpretation, func)) = e.get_index(index) {
            func(self.0.clone(), target_interpretation)
        } else {
            nores()
        }
    }

    pub fn get(&self, key: Value) -> Res<Cell> {
        let old_interp = self.0.interpretation();
        let interp = match key {
            Value::None => guard_some!(top_interpretation(&self.0), {
                return nores();
            }),
            Value::Str(k) => k,
            _ => return userres("no interpretation for non-string value".to_string()),
        };
        if interp == old_interp {
            return Ok(self.0.clone());
        }
        if let Ok(e) = elevation_map(old_interp) {
            if let Some((_, target_interpretation, func)) = e.get_full(interp) {
                // debug!("elevate {} to {}", old_interp, key);
                return func(self.0.clone(), target_interpretation);
            }
        }
        if let Ok(e) = elevation_map("value") {
            if let Some((_, target_interpretation, func)) = e.get_full(interp) {
                // debug!("elevate as value from {} to {}", old_interp, key);
                return func(self.0.clone(), target_interpretation);
            }
        }
        debug!("no elevation from {} to {}", old_interp, interp);
        nores()
    }
}
