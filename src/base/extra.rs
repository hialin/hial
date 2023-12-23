use std::ops::Deref;
use std::rc::Rc;

use crate::{
    base::*, enumerated_dynamic_type, interpretations::*, pathlang::eval::EvalIter, pathlang::Path,
};

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynDomain {
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
    with_dyn_domain
}

#[derive(Clone, Debug)]
pub struct Domain {
    pub(crate) this: DynDomain,
    pub(crate) source: Option<Cell>,
}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynCell {
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
pub struct Cell {
    pub(crate) domain: Rc<Domain>,
    pub(crate) this: DynCell,
    // todo: remove box
    pub(crate) prev: Option<(Box<Cell>, Relation)>,
}

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub(crate) enum DynCellReader {
        OwnedValue(ownedvalue::CellReader),
        File(file::CellReader),
        Json(json::CellReader),
        Toml(toml::CellReader),
        Yaml(yaml::CellReader),
        Xml(xml::CellReader),
        Url(url::CellReader),
        Http(http::CellReader),
        TreeSitter(treesitter::CellReader),
    }
    with_cell_reader
}

#[derive(Debug)]
pub struct CellReader(DynCellReader);

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynGroup {
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
pub(crate) enum EnGroup {
    Elevation(ElevationGroup),
    // Mixed(Vec<Cell>),
    Dyn(DynGroup),
}

#[derive(Clone, Debug)]
pub struct Group {
    pub(crate) domain: Rc<Domain>,
    pub(crate) this: EnGroup,
    // todo: remove box
    pub(crate) prev: Option<(Box<Cell>, Relation)>,
}

impl Domain {
    pub fn root(&self) -> Res<Cell> {
        let prev = if let Some(ref source) = self.source {
            Some((Box::new(source.clone()), Relation::Interpretation))
        } else {
            None
        };
        with_dyn_domain!(&self.this, |x| {
            Ok(Cell {
                domain: Rc::new(self.clone()),
                this: DynCell::from(x.root()?),
                prev,
            })
        })
    }

    pub fn write_back(&self) -> Res<()> {
        if let Some(ref source) = self.source {
            let rawdata = self.root()?.raw()?;
            source.set_raw(rawdata)?;
            Ok(())
        } else {
            NotFound::NoSource.into()
        }
    }

    pub fn interpretation(&self) -> &str {
        with_dyn_domain!(&self.this, |x| { x.interpretation() })
    }
}

impl From<OwnedValue> for Cell {
    fn from(ov: OwnedValue) -> Self {
        let c = ownedvalue::Cell::from(ov);
        let domain = Domain {
            this: DynDomain::from(c.domain().clone()),
            source: None,
        };
        Cell {
            domain: Rc::new(domain),
            this: DynCell::from(c),
            prev: None,
        }
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell::from(v.to_owned_value())
    }
}

impl From<&str> for Cell {
    fn from(s: &str) -> Self {
        Cell::from(s.to_string())
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell::from(OwnedValue::from(s))
    }
}

impl CellReader {
    pub fn index(&self) -> Res<usize> {
        with_cell_reader!(&self.0, |x| { Ok(x.index()?) })
    }
    pub fn label(&self) -> Res<Value> {
        with_cell_reader!(&self.0, |x| { Ok(x.label()?) })
    }
    pub fn value(&self) -> Res<Value> {
        with_cell_reader!(&self.0, |x| { Ok(x.value()?) })
    }
}

impl DynCell {
    pub(crate) fn domain(&self) -> DynDomain {
        with_dyn_cell!(self, |x| { DynDomain::from(x.domain().clone()) })
    }

    pub(crate) fn typ(&self) -> Res<&str> {
        with_dyn_cell!(self, |x| { Ok(x.typ()?) })
    }
    pub(crate) fn read(&self) -> Res<DynCellReader> {
        with_dyn_cell!(self, |x| { Ok(DynCellReader::from(x.read()?)) })
    }

    pub(crate) fn sub(&self) -> Res<DynGroup> {
        with_dyn_cell!(self, |x| { Ok(DynGroup::from(x.sub()?)) })
    }
    pub(crate) fn attr(&self) -> Res<DynGroup> {
        with_dyn_cell!(self, |x| { Ok(DynGroup::from(x.attr()?)) })
    }

    pub(crate) fn set_value(&mut self, value: OwnedValue) -> Res<()> {
        with_dyn_cell!(self, |x| { x.set_value(value) })
    }
    pub(crate) fn set_label(&mut self, value: OwnedValue) -> Res<()> {
        with_dyn_cell!(self, |x| { x.set_label(value) })
    }
}

impl Cell {
    pub fn domain(&self) -> Domain {
        self.domain.deref().clone()
    }

    pub fn typ(&self) -> Res<&str> {
        self.this.typ()
    }

    pub fn read(&self) -> Res<CellReader> {
        Ok(CellReader(self.this.read()?))
    }

    pub fn sub(&self) -> Res<Group> {
        Ok(Group {
            domain: self.domain.clone(),
            this: EnGroup::Dyn(self.this.sub()?),
            prev: Some((Box::new(self.clone()), Relation::Sub)),
        })
    }

    pub fn attr(&self) -> Res<Group> {
        Ok(Group {
            domain: self.domain.clone(),
            this: EnGroup::Dyn(self.this.attr()?),
            prev: Some((Box::new(self.clone()), Relation::Sub)),
        })
    }

    pub fn standard_interpretation(&self) -> Option<&str> {
        elevation::standard_interpretation(self)
    }

    pub fn elevate(self) -> Res<Group> {
        Ok(Group {
            domain: self.domain.clone(),
            this: EnGroup::Elevation(ElevationGroup(self.clone())),
            prev: Some((Box::new(self), Relation::Interpretation)),
        })
    }

    pub fn field(&self) -> Res<Group> {
        todo!()
    }

    pub fn be(self, interpretation: &str) -> Res<Cell> {
        self.elevate()?.get(interpretation)
    }

    pub fn search<'a>(&self, path: &'a str) -> Res<PathSearch<'a>> {
        Ok(PathSearch {
            cell: self.clone(),
            path: crate::pathlang::Path::parse(path)?,
        })
    }

    pub fn path(&self) -> Res<String> {
        use std::fmt::Write;
        let err_fn = |err| eprintln!("💥 str write error {}", err);
        let write_label_fn =
            |s: &mut String, reader: &CellReader, interpretation: &str, is_interpretation: bool| {
                match reader.label() {
                    Ok(l) => write!(s, "{}", l).unwrap_or_else(err_fn),
                    Err(HErr::NotFound(_)) => {
                        if is_interpretation {
                            write!(s, "{}", interpretation).unwrap_or_else(err_fn)
                        } else if let Ok(index) = reader.index() {
                            write!(s, "[{}]", index).unwrap_or_else(err_fn)
                        } else {
                            write!(s, "<?>").unwrap_or_else(err_fn)
                        }
                    }
                    Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
                }
            };

        let write_value_fn =
            |s: &mut String, reader: &CellReader, interpretation: &str| match reader.value() {
                Ok(value) => {
                    if interpretation == "value" {
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
            let interpretation = self.domain.interpretation();
            let reader = self.read()?;
            for a in v.iter().rev() {
                let (cell, rel) = a;
                if prev_relation.is_none() {
                    write_value_fn(&mut s, &reader, interpretation);
                } else {
                    write_label_fn(
                        &mut s,
                        &reader,
                        interpretation,
                        prev_relation == Some(Relation::Interpretation),
                    );
                }
                write!(s, "{}", rel).unwrap_or_else(err_fn);
                prev_relation = Some(*rel);
            }
            write_label_fn(
                &mut s,
                &reader,
                interpretation,
                prev_relation == Some(Relation::Interpretation),
            );
        }
        Ok(s)
    }

    pub fn set_value(&mut self, ov: OwnedValue) -> Res<()> {
        with_dyn_cell!(&mut self.this, |x| { x.set_value(ov) })
    }

    pub fn raw(&self) -> Res<RawDataContainer> {
        with_dyn_cell!(&self.this, |x| { x.raw() })
    }

    pub fn set_raw(&self, raw: RawDataContainer) -> Res<()> {
        with_dyn_cell!(&self.this, |x| { x.set_raw(raw) })
    }

    pub fn debug_string(&self) -> String {
        let err_fn = |err| eprintln!("💥 str write error {}", err);
        use std::fmt::Write;
        let mut s = String::new();
        match self.read() {
            Ok(reader) => {
                match reader.label() {
                    Ok(l) => write!(s, "{}", l).unwrap_or_else(err_fn),
                    Err(HErr::NotFound(_)) => {}
                    Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
                };
                write!(s, ":").unwrap_or_else(err_fn);
                match reader.value() {
                    Ok(v) => write!(s, "{}", v).unwrap_or_else(err_fn),
                    Err(HErr::NotFound(_)) => {}
                    Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
                };
            }
            Err(e) => {
                write!(s, "<💥cannot read: {:?}>", e).unwrap_or_else(err_fn);
            }
        }
        s
    }
}

impl Group {
    pub fn label_type(&self) -> LabelType {
        match &self.this {
            EnGroup::Elevation(elevation_group) => elevation_group.label_type(),
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| { x.label_type() })
            }
        }
    }

    pub fn len(&self) -> usize {
        match &self.this {
            EnGroup::Elevation(elevation_group) => elevation_group.len(),
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| { x.len() })
            }
        }
    }

    pub fn at(&self, index: usize) -> Res<Cell> {
        match &self.this {
            EnGroup::Elevation(elevation_group) => elevation_group.at(index),
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| {
                    Ok(Cell {
                        domain: self.domain.clone(),
                        this: DynCell::from(x.at(index)?),
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
            EnGroup::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| {
                    Ok(Cell {
                        domain: self.domain.clone(),
                        this: DynCell::from(x.get(key)?),
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
    pub fn all(self) -> Res<Vec<Cell>> {
        self.into_iter().collect::<Res<Vec<_>>>()
    }
}
