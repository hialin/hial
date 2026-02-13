use indexmap::IndexMap;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
    utils::{
        ownrc::{OwnRc, ReadRc},
        ownrcutils::{read, write},
    },
};

// origin --^-> root group --[]-> interp1 --/-> sub group  --[]-> interp1_root_cell
//                                        --@-> attr group --[]-> interp1_param1
//                                                         --[]-> ...
//                                interp2 --/-> sub group  --[]-> interp2_root_cell
//                                        --@-> attr group --[]-> interp2_param1
//                                                         --[]-> ...

const STD_ITP_PARAM_WRITE_BACK_ON_DROP: &str = "w";

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug)]
pub struct CellReader {
    data: ReadRc<Data>,
    kind: CellKind,
}

#[derive(Debug, Clone)]
enum CellKind {
    Interpretation(usize),
    Param(usize, usize),
    DetachedParam(Box<(OwnValue, OwnValue)>),
}

implement_try_from_xell!(Cell, Elevation);

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = Cell;

    fn interpretation(&self) -> &str {
        "interpretation"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            data: read(&self.data)?,
            kind: self.kind.clone(),
        })
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

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        match self.kind {
            CellKind::Interpretation(_) => Ok("elevation"),
            CellKind::Param(_, _) => Ok("param"),
            CellKind::DetachedParam { .. } => Ok("param"),
        }
    }

    fn value(&self) -> Res<Value<'_>> {
        match &self.kind {
            CellKind::Interpretation(_) => nores(),
            CellKind::Param(ii, ip) => Ok(self
                .data
                .map
                .get_index(*ii)
                .ok_or_else(noerr)?
                .1
                 .1
                .get_index(*ip)
                .ok_or_else(noerr)?
                .1
                .as_value()),
            CellKind::DetachedParam(b) => Ok(b.1.as_value()),
        }
    }

    fn label(&self) -> Res<Value<'_>> {
        match &self.kind {
            CellKind::Interpretation(i) => {
                Ok(Value::Str(self.data.map.get_index(*i).ok_or_else(noerr)?.0))
            }
            CellKind::Param(i, ip) => self
                .data
                .map
                .get_index(*i)
                .ok_or_else(noerr)?
                .1
                 .1
                .get_index(*ip)
                .ok_or_else(noerr)
                .map(|x| x.0.as_value()),
            CellKind::DetachedParam(b) => Ok(b.0.as_value()),
        }
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            CellKind::Interpretation(i) => Ok(i),
            CellKind::Param(_, ip) => Ok(ip),
            CellKind::DetachedParam { .. } => nores(),
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for Cell {
    fn set_label(&mut self, v: OwnValue) -> Res<()> {
        userres("cannot set labels on elevation cells")
    }

    fn set_value(&mut self, v: OwnValue) -> Res<()> {
        match &mut self.kind {
            CellKind::Interpretation(i) => userres("cannot set interpretation value"),
            CellKind::Param(i, ip) => {
                *write(&self.data)?
                    .map
                    .get_index_mut(*i)
                    .ok_or_else(noerr)?
                    .1
                     .1
                    .get_index_mut(*ip)
                    .ok_or_else(noerr)?
                    .1 = v;
                Ok(())
            }
            CellKind::DetachedParam(b) => {
                b.1 = v;
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
        self.len().is_ok_and(|x| x == 0)
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

    fn create(&self, label: Option<OwnValue>, value: Option<OwnValue>) -> Res<Self::Cell> {
        if self.kind == GroupKind::Root {
            return userres("cannot create new cells in interpretation root group");
        }
        if let GroupKind::Sub(_) = self.kind {
            return userres("cannot create new cells in interpretation sub group");
        }
        if let Some(label) = label {
            Ok(Cell {
                data: self.data.clone(),
                kind: CellKind::DetachedParam(Box::new((label, value.unwrap_or(OwnValue::None)))),
            })
        } else {
            userres("cannot create new cells without a label")
        }
    }

    fn add(&self, index: Option<usize>, cell: Self::Cell) -> Res<()> {
        match self.kind {
            GroupKind::Root => userres("cannot add cell to interpretaion root group"),
            GroupKind::Sub(_) => userres("cannot add cell to interpretation sub group"),
            GroupKind::Attr(i) => {
                let mut data = write(&self.data)?;
                let (target, (constructor, params)) =
                    data.map.get_index_mut(i).ok_or_else(noerr)?;
                if let CellKind::DetachedParam(b) = cell.kind {
                    if params.contains_key(&b.0.as_value()) {
                        return userres(format!(
                            "cannot add elevation param with label `{}` twice",
                            b.0
                        ));
                    }
                    params.insert(b.0, b.1);
                }
                if let Some(index) = index {
                    data.map.move_index(i, index);
                }
                Ok(())
            }
        }
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

            if let Some(auto) = auto_interpretation(&origin)
                && let Some((index, ..)) = map.get_full(auto)
            {
                map.move_index(index, 0)
            }
        }
        // println!(
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
                Ok(Xell::new_from(
                    DynCell::from(cell),
                    Some(data.origin.clone()),
                ))
            }
            GroupKind::Attr(i) => {
                let cell = Cell {
                    data: self.data.clone(),
                    kind: CellKind::Param(index, i),
                };
                Ok(Xell::new_from(
                    DynCell::from(cell),
                    Some(data.origin.clone()),
                ))
            }
            GroupKind::Sub(i) => {
                let (target, (constructor, params)) = data.map.get_index(i).ok_or_else(noerr)?;
                // println!("construct elevation {} {:?}", target, params);
                let cell = constructor(data.origin.clone(), target, params)?;
                cell.set_self_as_domain_root();

                for (k, v) in params.iter() {
                    if matches!(k, OwnValue::Int(_))
                        && v.as_value() == Value::Str(STD_ITP_PARAM_WRITE_BACK_ON_DROP)
                    {
                        cell.policy(WritePolicy::WriteBackOnDrop);
                    }
                }
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
                    return noresm(format!(
                        "no such interpretation `{}`",
                        label.as_cow_str().as_ref()
                    ));
                };
                let cell = Cell {
                    data: self.data.clone(),
                    kind: CellKind::Interpretation(entry.0),
                };
                Ok(Xell::new_from(
                    DynCell::from(cell),
                    Some(data.origin.clone()),
                ))
            }
            GroupKind::Attr(i) => {
                let Some(entry) = data.map.get_index(i) else {
                    return nores();
                };
                let Some(entry) = entry.1 .1.get_full(&label) else {
                    return nores();
                };
                let cell = Cell {
                    data: self.data.clone(),
                    kind: CellKind::Param(i, entry.0),
                };
                Ok(Xell::new_from(
                    DynCell::from(cell),
                    Some(data.origin.clone()),
                ))
            }
            GroupKind::Sub(i) => {
                userres("cannot use get to access interpretation root, use at instead")
            }
        }
    }
}
