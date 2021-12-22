use crate::{
    base::*, enumerated_dynamic_type, interpretations::*, pathlang::eval::EvalIter, pathlang::Path,
};

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub enum Domain {
        OwnedValue(ownedvalue::Domain),
        File(file::Domain),
        Json(json::Domain),
        Toml(toml::Domain),
        Yaml(yaml::Domain),
        Xml(xml::Domain),
        Url(url::Domain),
        Http (http::Domain),
        TreeSitter (treesitter::Domain),
    }
    with_domain
}

// todo: separate interpretation and ex-terpretation types
// pub enum InnerCell {OwnedValue, File, ...}
// pub enum Cell {Field, InnerCell}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub enum DynCell {
        OwnedValue(ownedvalue::Cell),
        File(file::Cell),
        Json(json::Cell),
        Toml(toml::Cell),
        Yaml(yaml::Cell),
        Xml(xml::Cell),
        Url(url::Cell),
        Http(http::Cell),
        TreeSitter(treesitter::Cell),
    }
    with_dyn_cell
}

#[derive(Clone, Debug)]
pub enum EnCell {
    Field(Field),
    Dyn(DynCell),
}

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub enum ValueRef {
        Field(field::ValueRef),
        OwnedValue(ownedvalue::ValueRef),
        File(file::ValueRef),
        Json(json::ValueRef),
        Toml(toml::ValueRef),
        Yaml(yaml::ValueRef),
        Xml(xml::ValueRef),
        Url(url::ValueRef),
        Http(http::ValueRef),
        TreeSitter(treesitter::ValueRef),
    }
    with_valueref
}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub enum DynGroup {
        OwnedValue(VoidGroup<ownedvalue::Domain>),
        File(file::Group),
        Json(json::Group),
        Toml(toml::Group),
        Yaml(yaml::Group),
        Xml(xml::Group),
        Url(VoidGroup<url::Domain>),
        Http(http::Group),
        TreeSitter(treesitter::Group),
    }
    with_dyn_group
}

#[derive(Clone, Debug)]
pub enum EnGroup {
    Elevation(ElevationGroup),
    Field(Field),
    // Mixed(Vec<Cell>),
    Dyn(DynGroup),
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub(crate) this: EnCell,
    pub(crate) prev: Option<(Box<Cell>, Relation)>, //todo: remove box
}

#[derive(Clone, Debug)]
pub struct Group {
    pub(crate) this: EnGroup,
    pub(crate) prev: Option<(Box<Cell>, Relation)>, //todo: remove box
}

impl Domain {
    pub fn root(&self) -> Res<Cell> {
        with_domain!(self, |x| {
            Ok(Cell {
                this: EnCell::Dyn(DynCell::from(x.root()?)),
                prev: None,
            })
        })
    }

    pub fn save_to_origin(&self) -> Res<()> {
        todo!()
    }

    pub fn interpretation(&self) -> &str {
        with_domain!(self, |x| { x.interpretation() })
    }
}

impl From<OwnedValue> for Cell {
    fn from(ov: OwnedValue) -> Self {
        Cell {
            this: EnCell::Dyn(DynCell::from(ownedvalue::Cell::from(ov))),
            prev: None,
        }
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell {
            this: EnCell::Dyn(DynCell::from(ownedvalue::Cell::from(v))),
            prev: None,
        }
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell {
            this: EnCell::Dyn(DynCell::from(ownedvalue::Cell::from(s))),
            prev: None,
        }
    }
}

impl ValueRef {
    pub fn get(&self) -> Res<Value> {
        with_valueref!(self, |x| { x.get() })
    }

    pub fn with<T>(&self, f: impl Fn(Res<Value>) -> T) -> T {
        f(self.get())
    }
}

impl Cell {
    pub fn typ(&self) -> Res<&str> {
        match &self.this {
            EnCell::Dyn(dyn_cell) => with_dyn_cell!(dyn_cell, |x| { Ok(x.typ()?) }),
            EnCell::Field(field_cell) => field_cell.typ(),
        }
    }

    pub fn index(&self) -> Res<usize> {
        match &self.this {
            EnCell::Dyn(dyn_cell) => with_dyn_cell!(dyn_cell, |x| { Ok(x.index()?) }),
            EnCell::Field(field_cell) => field_cell.index(),
        }
    }

    pub fn label(&self) -> ValueRef {
        match &self.this {
            EnCell::Dyn(dyn_cell) => {
                with_dyn_cell!(dyn_cell, |x| { ValueRef::from(x.label()) })
            }
            EnCell::Field(field_cell) => ValueRef::from(field_cell.label()),
        }
    }

    pub fn value(&self) -> ValueRef {
        match &self.this {
            EnCell::Dyn(dyn_cell) => {
                with_dyn_cell!(dyn_cell, |x| { ValueRef::from(x.value()) })
            }
            EnCell::Field(field_cell) => ValueRef::from(field_cell.value()),
        }
    }

    pub fn sub(&self) -> Res<Group> {
        match &self.this {
            EnCell::Dyn(dyn_cell) => with_dyn_cell!(dyn_cell, |x| {
                Ok(Group {
                    this: EnGroup::Dyn(DynGroup::from(x.sub()?)),
                    prev: Some((Box::new(self.clone()), Relation::Sub)),
                })
            }),
            EnCell::Field(field_cell) => NotFound::NoGroup(format!("/")).into(),
        }
    }

    pub fn attr(&self) -> Res<Group> {
        match &self.this {
            EnCell::Dyn(dyn_cell) => with_dyn_cell!(dyn_cell, |x| {
                Ok(Group {
                    this: EnGroup::Dyn(DynGroup::from(x.attr()?)),
                    prev: Some((Box::new(self.clone()), Relation::Attr)),
                })
            }),
            EnCell::Field(field_cell) => NotFound::NoGroup(format!("@")).into(),
        }
    }

    pub fn standard_interpretation(&self) -> Option<&str> {
        elevation::standard_interpretation(self)
    }

    pub fn elevate(self) -> Res<Group> {
        Ok(Group {
            this: EnGroup::Elevation(ElevationGroup(self.clone())),
            prev: Some((Box::new(self), Relation::Interpretation)),
        })
    }

    pub fn field(&self) -> Res<Group> {
        Ok(Group {
            this: EnGroup::Field(Field(Box::new(self.clone()), FieldType::Value)),
            prev: Some((Box::new(self.clone()), Relation::Field)),
        })
    }

    pub fn be(self, interpretation: &str) -> Res<Cell> {
        self.elevate()?.get(interpretation)
    }

    pub fn path<'a>(&self, path: &'a str) -> Res<PathSearch<'a>> {
        Ok(PathSearch {
            cell: self.clone(),
            path: crate::pathlang::Path::parse(path)?,
        })
    }

    pub fn get_path(&self) -> Res<String> {
        use std::fmt::Write;
        let err_fn = |err| eprintln!("💥 str write error {}", err);
        let write_label_fn =
            |s: &mut String, cell: &Cell, is_interpretation: bool| match cell.label().get() {
                Ok(l) => write!(s, "{}", l).unwrap_or_else(err_fn),
                Err(HErr::NotFound(_)) => {
                    if is_interpretation {
                        write!(s, "{}", cell.domain().interpretation()).unwrap_or_else(err_fn)
                    } else if let Ok(index) = cell.index() {
                        write!(s, "[{}]", index).unwrap_or_else(err_fn)
                    } else {
                        write!(s, "<?>").unwrap_or_else(err_fn)
                    }
                }
                Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
            };

        let write_value_fn = |s: &mut String, cell: &Cell| match cell.value().get() {
            Ok(value) => {
                if cell.domain().interpretation() == "value" {
                    let mut v = format!("{}", value).replace("\n", "\\n");
                    if v.len() > 4 {
                        v.truncate(4);
                        v += "...";
                    }
                    write!(s, "\"{}\"", v).unwrap_or_else(err_fn);
                } else {
                    write!(s, "{}", value).unwrap_or_else(err_fn);
                }
            }
            Err(HErr::NotFound(_)) => write!(s, "<?>").unwrap_or_else(err_fn),
            Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
        };

        let mut v = vec![];
        {
            let mut a = self.prev.as_ref();
            while let Some((cell, rel)) = a {
                v.push((cell.clone(), *rel));
                a = cell.prev.as_ref();
            }
        }

        let mut s = String::new();
        {
            let mut prev_relation = None;
            for a in v.iter().rev() {
                let (cell, rel) = a;
                if prev_relation.is_none() {
                    write_value_fn(&mut s, cell);
                } else {
                    write_label_fn(
                        &mut s,
                        cell,
                        prev_relation == Some(Relation::Interpretation),
                    );
                }
                write!(s, "{}", rel).unwrap_or_else(err_fn);
                prev_relation = Some(*rel);
            }
            write_label_fn(
                &mut s,
                self,
                prev_relation == Some(Relation::Interpretation),
            );
        }
        Ok(s)
    }

    pub fn set_value(&mut self, ov: OwnedValue) -> Res<()> {
        match &mut self.this {
            EnCell::Dyn(dyn_cell) => {
                with_dyn_cell!(dyn_cell, |x| { x.set_value(ov) })
            }
            EnCell::Field(field_cell) => field_cell.set_value(ov),
        }
    }

    pub fn set_label(&mut self, ov: OwnedValue) -> Res<()> {
        match &mut self.this {
            EnCell::Dyn(dyn_cell) => {
                with_dyn_cell!(dyn_cell, |x| { x.set_label(ov) })
            }
            EnCell::Field(field_cell) => field_cell.set_label(ov),
        }
    }

    pub fn domain(&self) -> Domain {
        match &self.this {
            EnCell::Dyn(dyn_cell) => {
                with_dyn_cell!(dyn_cell, |x| { Domain::from(x.domain().clone()) })
            }
            EnCell::Field(field_cell) => field_cell.domain(),
        }
    }

    pub fn as_data_source(&self) -> Option<Res<DataSource>> {
        match &self.this {
            EnCell::Dyn(dyn_cell) => {
                with_dyn_cell!(dyn_cell, |x| { x.as_data_source() })
            }
            EnCell::Field(field_cell) => field_cell.as_data_source(),
        }
    }

    // pub fn as_data_destination(&mut self) -> Option<Res<DataDestination>> {
    //     with_cell!(self, |x| { x.as_data_destination() })
    // }

    pub fn debug_string(&self) -> String {
        let err_fn = |err| eprintln!("💥 str write error {}", err);
        let lr = self.label();
        let vr = self.value();
        use std::fmt::Write;
        let mut s = String::new();
        match self.label().get() {
            Ok(l) => {
                if !matches!(self.this, EnCell::Field(_)) {
                    write!(s, "{}", l).unwrap_or_else(err_fn)
                }
            }
            Err(HErr::NotFound(_)) => {}
            Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
        };
        write!(s, ":").unwrap_or_else(err_fn);
        match self.value().get() {
            Ok(v) => write!(s, "{}", v).unwrap_or_else(err_fn),
            Err(HErr::NotFound(_)) => {}
            Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
        };
        s
    }
}

impl Group {
    pub fn label_type(&self) -> LabelType {
        match &self.this {
            EnGroup::Elevation(elevation_group) => elevation_group.label_type(),
            EnGroup::Field(field_group) => field_group.label_type(),
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| { x.label_type() })
            }
        }
    }

    pub fn len(&self) -> usize {
        match &self.this {
            EnGroup::Elevation(elevation_group) => elevation_group.len(),
            EnGroup::Field(field_group) => field_group.len(),
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| { x.len() })
            }
        }
    }

    pub fn at(&self, index: usize) -> Res<Cell> {
        match &self.this {
            EnGroup::Elevation(elevation_group) => elevation_group.at(index),
            EnGroup::Field(field_group) => Ok(Cell {
                this: EnCell::Field(field_group.at(index)?),
                prev: self.prev.clone(),
            }),
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| {
                    Ok(Cell {
                        this: EnCell::Dyn(DynCell::from(x.at(index)?)),
                        prev: self.prev.clone(),
                    })
                })
            }
        }
    }

    pub fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        match &self.this {
            EnGroup::Elevation(elevation_group) => elevation_group.get(key),
            EnGroup::Field(field_group) => Ok(Cell {
                this: EnCell::Field(field_group.get(key)?),
                prev: self.prev.clone(),
            }),
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| {
                    Ok(Cell {
                        this: EnCell::Dyn(DynCell::from(x.get(key)?)),
                        prev: self.prev.clone(),
                    })
                })
            }
        }
    }
}

impl IntoIterator for Group {
    type Item = Res<Cell>;
    type IntoIter = GroupIter;

    fn into_iter(self) -> Self::IntoIter {
        GroupIter(self, 0)
    }
}

#[derive(Debug)]
pub struct GroupIter(Group, usize);
impl Iterator for GroupIter {
    type Item = Res<Cell>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.1 >= self.0.len() {
            return None;
        }
        self.1 += 1;
        Some(self.0.at(self.1 - 1))
    }
}

#[derive(Clone, Debug)]
pub struct PathSearch<'a> {
    cell: Cell,
    path: Path<'a>,
}
impl<'a> IntoIterator for PathSearch<'a> {
    type Item = Res<Cell>;
    type IntoIter = EvalIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        EvalIter::new(self.cell, self.path)
    }
}
impl<'a> PathSearch<'a> {
    pub fn first(self) -> Res<Cell> {
        let x = self.into_iter().next();
        x.unwrap_or(NotFound::NoResult(format!("no result for this path")).into())
    }
}
