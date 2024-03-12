use indexmap::IndexMap;

use crate::{
    api::{interpretation::*, *},
    utils::{
        ownrc::OwnRc,
        ownrcutils::{read, write},
    },
};

// origin --^-> root group --[]-> interp1 --/-> sub group  --[]-> interp1_root_cell
//                                        --@-> attr group --[]-> interp1_param1
//                                                         --[]-> ...
//                                interp2 --/-> sub group  --[]-> interp2_root_cell
//                                        --@-> attr group --[]-> interp2_param1
//                                                         --[]-> ...

#[derive(Debug, Clone)]
struct Data {
    origin: Xell,
    map: IndexMap<&'static str, (ElevateFn, ElevateParams)>,
}

#[derive(Debug, Clone)]
pub struct Group {
    data: OwnRc<Data>,
    kind: GroupKind,
}

#[derive(Debug, Clone)]
enum GroupKind {
    Root,
    Attr(usize),
    Sub(usize),
}

#[derive(Debug, Clone)]
pub struct Cell {
    data: OwnRc<Data>,
    kind: CellKind,
}

#[derive(Debug, Clone)]
enum CellKind {
    Interpretation(usize),
    Param(usize, usize),
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = Cell;
    type CellWriter = Cell;

    fn interpretation(&self) -> &str {
        "interpretation"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(self.clone())
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(self.clone())
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }

    fn sub(&self) -> Res<Self::Group> {
        if let CellKind::Interpretation(i) = self.kind {
            Ok(Group {
                data: self.data.clone(),
                kind: GroupKind::Sub(i),
            })
        } else {
            nores()
        }
    }

    fn attr(&self) -> Res<Self::Group> {
        if let CellKind::Interpretation(i) = self.kind {
            Ok(Group {
                data: self.data.clone(),
                kind: GroupKind::Attr(i),
            })
        } else {
            nores()
        }
    }
}

impl CellReaderTrait for Cell {
    fn ty(&self) -> Res<&str> {
        match self.kind {
            CellKind::Interpretation(_) => Ok("elevation"),
            CellKind::Param(_, _) => Ok("param"),
        }
    }

    fn value(&self) -> Res<Value> {
        nores()
    }

    fn label(&self) -> Res<Value> {
        match self.kind {
            CellKind::Interpretation(i) => {
                let data = read(&self.data)?;
                let entry = data.map.get_index(i).ok_or_else(noerr)?;
                Ok(Value::Str(entry.0))
            }
            CellKind::Param(i, ip) => {
                let data = read(&self.data)?;
                let entry = data.map.get_index(i).ok_or_else(noerr)?;
                entry
                    .1
                     .1
                    .get_index(ip)
                    .ok_or_else(noerr)
                    .map(|x| Value::Str(x.0))
            }
        }
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            CellKind::Interpretation(i) => Ok(i),
            CellKind::Param(_, ip) => Ok(ip),
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for Cell {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.kind {
            CellKind::Interpretation(i) => userres("cannot set interpretation value"),
            CellKind::Param(i, ip) => {
                let mut data = write(&self.data)?;
                let entry = data.map.get_index_mut(i).ok_or_else(noerr)?;
                let param = entry.1 .1.get_index_mut(ip).ok_or_else(noerr)?;
                *param.1 = value;
                Ok(())
            }
        }
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Empty<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        match self.kind {
            GroupKind::Root => Ok(read(&self.data).map(|d| d.map.len())?),
            GroupKind::Attr(i) => {
                let data = read(&self.data)?;
                let entry = data.map.get_index(i).ok_or_else(noerr)?;
                Ok(entry.1 .1.len())
            }
            GroupKind::Sub(i) => Ok(1),
        }
    }

    fn is_empty(&self) -> bool {
        self.len().map_or(false, |x| x == 0)
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        // This cannot be implemented, we should return a Xell here but the
        // trait type does not allow us. This is fixed by Xell::at which
        // returns the correct type.
        unimplemented!()
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        // This cannot be implemented, we should return a Xell here but the
        // trait type does not allow us. This is fixed by Xell::get which
        // returns the correct type.
        unimplemented!()
    }
}

impl Group {
    pub(crate) fn new(origin: Xell) -> Res<Group> {
        let mut map = IndexMap::<&'static str, (ElevateFn, ElevateParams)>::new();
        {
            match elevation_registry(origin.interpretation()) {
                Ok(registry) => {
                    for (k, v) in registry.iter() {
                        map.insert(k, (*v, IndexMap::new()));
                    }
                }
                Err(err) => {
                    if err.kind != HErrKind::None {
                        return Err(err);
                    }
                }
            };

            if origin.read().value().unwrap_or(Value::None) != Value::None {
                match elevation_registry("value") {
                    Ok(registry) => {
                        for (k, v) in registry.iter() {
                            if !map.contains_key(k) {
                                map.insert(k, (*v, IndexMap::new()));
                            }
                        }
                    }
                    Err(err) => {
                        if err.kind != HErrKind::None {
                            return Err(err);
                        }
                    }
                };
            }

            if let Some(auto) = auto_interpretation(&origin) {
                if let Some((index, ..)) = map.get_full(auto) {
                    map.move_index(index, 0)
                }
            }
        }
        // debug!(
        //     "new elevate group for {} -> {:?}",
        //     origin.interpretation(),
        //     map
        // );
        Ok(Group {
            data: OwnRc::new(Data { origin, map }),
            kind: GroupKind::Root,
        })
    }

    // just like Xell::at but returns a Xell not a Cell
    pub(crate) fn at_(&self, index: usize) -> Res<Xell> {
        let data = read(&self.data)?;
        match self.kind {
            GroupKind::Root => {
                let cell = Cell {
                    data: self.data.clone(),
                    kind: CellKind::Interpretation(index),
                };
                Ok(new_xell(DynCell::from(cell), Some(data.origin.clone())))
            }
            GroupKind::Attr(i) => {
                let cell = Cell {
                    data: self.data.clone(),
                    kind: CellKind::Param(index, i),
                };
                Ok(new_xell(DynCell::from(cell), Some(data.origin.clone())))
            }
            GroupKind::Sub(i) => {
                let (target, (func, params)) = data.map.get_index(i).ok_or_else(noerr)?;
                let cell = func(data.origin.clone(), target, params)?;
                cell.set_self_as_domain_root();
                Ok(cell)
            }
        }
    }

    // just like Xell::get but returns a Xell not a Cell
    pub(crate) fn get_(&self, label: Value) -> Res<Xell> {
        let data = read(&self.data)?;
        match self.kind {
            GroupKind::Root => {
                let Some(entry) = data.map.get_full(label.as_cow_str().as_ref()) else {
                    return nores();
                };
                let cell = Cell {
                    data: self.data.clone(),
                    kind: CellKind::Interpretation(entry.0),
                };
                Ok(new_xell(DynCell::from(cell), Some(data.origin.clone())))
            }
            GroupKind::Attr(i) => {
                let Some(entry) = data.map.get_index(i) else {
                    return nores();
                };
                let Some(entry) = entry.1 .1.get_full(label.as_cow_str().as_ref()) else {
                    return nores();
                };
                let cell = Cell {
                    data: self.data.clone(),
                    kind: CellKind::Param(i, entry.0),
                };
                Ok(new_xell(DynCell::from(cell), Some(data.origin.clone())))
            }
            GroupKind::Sub(i) => {
                userres("cannot use get to access interpretation root, use at instead")
            }
        }
    }
}
