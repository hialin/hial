use std::rc::Rc;

use crate::{
    base::*, enumerated_dynamic_type, guard_ok, interpretations::*, pathlang::eval::EvalIter,
    pathlang::Path,
};

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynDomain {
        Error(HErr),
        // field is only declared for type completeness, not actually used
        Field(field::FieldGroup),
        // own value cell is its own domain
        OwnValue(ownvalue::Cell),
        File(file::Domain),
        Json(json::Domain),
        Toml(toml::Domain),
        Yaml(yaml::Domain),
        Xml(xml::Domain),
        Url(url::Domain),
        Path(path::Domain),
        Http(http::Domain),
        TreeSitter(treesitter::Domain),
    }
}

#[derive(Clone, Debug)]
pub struct Domain {
    pub(crate) dyn_domain: DynDomain,
}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynCell {
        Error(HErr),
        Field(field::FieldCell),
        OwnValue(ownvalue::Cell),
        File(file::Cell),
        Json(json::Cell),
        Toml(toml::Cell),
        Yaml(yaml::Cell),
        Xml(xml::Cell),
        Url(url::Cell),
        Path(path::Cell),
        Http(http::Cell),
        TreeSitter(treesitter::Cell),
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Cell {
    pub(crate) dyn_cell: DynCell,
}

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub(crate) enum DynCellReader {
        Error(HErr),
        Field(field::FieldReader),
        OwnValue(ownvalue::CellReader),
        File(file::CellReader),
        Json(json::CellReader),
        Toml(toml::CellReader),
        Yaml(yaml::CellReader),
        Xml(xml::CellReader),
        Url(url::CellReader),
        Path(path::CellReader),
        Http(http::CellReader),
        TreeSitter(treesitter::CellReader),
    }
}

#[derive(Debug)]
pub struct CellReader(DynCellReader);

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub(crate) enum DynCellWriter {
        Error(HErr),
        Field(field::FieldWriter),
        OwnValue(ownvalue::CellWriter),
        File(file::CellWriter),
        Json(json::CellWriter),
        Toml(toml::CellWriter),
        Yaml(yaml::CellWriter),
        Xml(xml::CellWriter),
        Url(url::CellWriter),
        Path(path::CellWriter),
        Http(http::CellWriter),
        TreeSitter(treesitter::CellWriter),
    }
}

#[derive(Debug)]
pub struct CellWriter(DynCellWriter);

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub enum DynGroup {
        Error(HErr),
        Field(field::FieldGroup),
        OwnValue(VoidGroup<ownvalue::Cell>),
        File(file::Group),
        Json(json::Group),
        Toml(toml::Group),
        Yaml(yaml::Group),
        Xml(xml::Group),
        Url(VoidGroup<url::Cell>),
        Path(VoidGroup<path::Cell>),
        Http(http::Group),
        TreeSitter(treesitter::Group),
    }
}

#[derive(Clone, Debug)]
pub enum Group {
    Dyn(DynGroup),
    Elevation(ElevationGroup),
    // Mixed(Vec<Cell>),
}

impl Domain {
    pub fn interpretation(&self) -> &str {
        dispatch_dyn_domain!(&self.dyn_domain, |x| { x.interpretation() })
    }

    pub fn root(&self) -> Res<Cell> {
        if let DynDomain::Field(field) = &self.dyn_domain {
            return field.cell.domain().root();
        }
        dispatch_dyn_domain!(&self.dyn_domain, |x| {
            Ok(Cell {
                dyn_cell: DynCell::from(x.root()?),
            })
        })
    }

    pub fn origin(&self) -> Cell {
        dispatch_dyn_domain!(&self.dyn_domain, |x| {
            match x.origin() {
                Ok(c) => c,
                Err(e) => Cell {
                    dyn_cell: DynCell::from(e),
                },
            }
        })
    }

    pub fn write_policy(&self) -> Res<WritePolicy> {
        dispatch_dyn_domain!(&self.dyn_domain, |x| { x.write_policy() })
    }
    pub fn set_write_policy(&mut self, policy: WritePolicy) -> Res<()> {
        dispatch_dyn_domain!(&mut self.dyn_domain, |x| { x.set_write_policy(policy) })
    }

    pub fn save(&self, target: SaveTarget) -> Res<()> {
        dispatch_dyn_domain!(&self.dyn_domain, |x| { x.save(target) })
    }

    pub fn err(self) -> Res<Domain> {
        if let DynDomain::Error(error) = self.dyn_domain {
            return Err(error);
        }
        Ok(self)
    }
}

impl From<OwnValue> for Cell {
    fn from(ov: OwnValue) -> Self {
        ownvalue::Cell::from_value(ov).unwrap()
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell::from(v.to_owned_value())
    }
}

impl From<&str> for Cell {
    fn from(s: &str) -> Self {
        ownvalue::Cell::from_str(s).unwrap()
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        ownvalue::Cell::from_string(s).unwrap()
    }
}

impl From<Res<Cell>> for Cell {
    fn from(res: Res<Cell>) -> Self {
        match res {
            Ok(cell) => cell,
            Err(e) => Cell::from(e),
        }
    }
}

impl From<HErr> for Cell {
    fn from(herr: HErr) -> Self {
        Cell {
            dyn_cell: DynCell::from(herr),
        }
    }
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        dispatch_dyn_cell_reader!(&self.0, |x| { Ok(x.index()?) })
    }
    fn label(&self) -> Res<Value> {
        dispatch_dyn_cell_reader!(&self.0, |x| { Ok(x.label()?) })
    }
    fn value(&self) -> Res<Value> {
        dispatch_dyn_cell_reader!(&self.0, |x| { Ok(x.value()?) })
    }
}
impl CellReader {
    pub fn err(self) -> Res<CellReader> {
        if let DynCellReader::Error(error) = self.0 {
            return Err(error);
        }
        Ok(self)
    }
}

impl CellWriterTrait for CellWriter {
    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        dispatch_dyn_cell_writer!(&mut self.0, |x| { x.set_label(value) })
    }

    fn set_value(&mut self, ov: OwnValue) -> Res<()> {
        dispatch_dyn_cell_writer!(&mut self.0, |x| { x.set_value(ov) })
    }
}
impl CellWriter {
    pub fn err(self) -> Res<CellWriter> {
        if let DynCellWriter::Error(error) = self.0 {
            return Err(error);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CellPath(pub String);

impl Cell {
    pub fn err(self) -> Res<Cell> {
        if let DynCell::Error(error) = self.dyn_cell {
            return Err(error);
        }
        Ok(self)
    }

    pub fn domain(&self) -> Domain {
        if let DynCell::Field(fieldcell) = &self.dyn_cell {
            return fieldcell.cell.domain();
        }
        let domain = dispatch_dyn_cell!(&self.dyn_cell, |x| { DynDomain::from(x.domain()) });
        Domain { dyn_domain: domain }
    }

    pub fn typ(&self) -> Res<&str> {
        dispatch_dyn_cell!(&self.dyn_cell, |x| { x.typ() })
    }

    pub fn read(&self) -> CellReader {
        let reader: DynCellReader = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.read() {
                Ok(r) => DynCellReader::from(r),
                Err(e) => DynCellReader::from(e),
            }
        });
        CellReader(reader)
    }

    pub fn policy(&self, policy: WritePolicy) -> Cell {
        // TODO: implement extra::Cell::policy
        self.clone()
    }

    pub fn write(&self) -> CellWriter {
        let writer: DynCellWriter = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.write() {
                Ok(r) => DynCellWriter::from(r),
                Err(e) => DynCellWriter::from(e),
            }
        });
        CellWriter(writer)
    }

    pub fn sub(&self) -> Group {
        let sub = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.sub() {
                Ok(r) => DynGroup::from(r),
                Err(e) => DynGroup::from(e),
            }
        });
        Group::Dyn(sub)
    }

    pub fn attr(&self) -> Group {
        let attr = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.attr() {
                Ok(r) => DynGroup::from(r),
                Err(e) => DynGroup::from(e),
            }
        });
        Group::Dyn(attr)
    }

    pub fn top_interpretation(&self) -> Option<&str> {
        if let DynCell::Error(_) = self.dyn_cell {
            return None;
        }
        elevation::top_interpretation(self)
    }

    pub fn elevate(&self) -> Group {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Group::Dyn(DynGroup::from(err.clone()));
        }
        Group::Elevation(ElevationGroup(self.clone()))
    }

    pub fn field(&self) -> Res<Group> {
        Ok(Group::Dyn(DynGroup::from(FieldGroup {
            cell: Rc::new(self.clone()),
            interpretation: self.domain().interpretation().to_string(),
        })))
    }

    pub fn be(&self, interpretation: &str) -> Cell {
        self.elevate().get(interpretation)
    }

    pub fn to(&self, path: &str) -> Cell {
        let path = guard_ok!(crate::pathlang::Path::parse(path), err =>
            return Cell {
                dyn_cell: DynCell::from(err),
            }
        );
        let search = PathSearch {
            cell: self.clone(),
            path,
        };
        Cell::from(search.first())
    }

    pub fn search<'a>(&self, path: &'a str) -> Res<PathSearch<'a>> {
        Ok(PathSearch {
            cell: self.clone(),
            path: crate::pathlang::Path::parse(path)?,
        })
    }

    pub fn head(&self) -> Res<(Cell, Relation)> {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Err(err.clone());
        }
        dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.head() {
                Ok((c, r)) => Ok((
                    Cell {
                        dyn_cell: DynCell::from(c),
                    },
                    r,
                )),
                Err(e) => Err(e),
            }
        })
    }

    /// Returns the path of head cells and relations in the current domain.
    /// The current cell is not included. If the path is empty, the current
    /// cell is the domain root. HErr::None is never returned.
    fn domain_path_items(&self) -> Res<Vec<(Cell, Relation)>> {
        let mut v: Vec<(Cell, Relation)> = vec![];

        let mut head = self.head();
        while let Ok(h) = head {
            v.push((h.0.clone(), h.1));
            head = h.0.head();
        }

        let err = head.unwrap_err();
        if err == HErr::None {
            v.reverse();
            Ok(v)
        } else {
            Err(err)
        }
    }

    pub fn path(&self) -> Res<CellPath> {
        use std::fmt::Write;
        let err_fn = |err| eprintln!("💥 str write error {}", err);
        let write_label_fn =
            |s: &mut String, reader: &CellReader, interpretation: &str, is_interpretation: bool| {
                match reader.label() {
                    Ok(l) => write!(s, "{}", l).unwrap_or_else(err_fn),
                    Err(HErr::None) => {
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
                        let mut v = format!("{}", value).replace('\n', "\\n");
                        if v.len() > 4 {
                            v.truncate(4);
                            v += "...";
                        }
                        write!(s, "\"{}\"", v).unwrap_or_else(err_fn);
                    } else {
                        write!(s, "{}", value).unwrap_or_else(err_fn);
                    }
                }
                Err(HErr::None) => write!(s, "<?>").unwrap_or_else(err_fn),
                Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
            };

        let v: Vec<(Cell, Relation)> = self.domain_path_items()?;

        let mut s = String::new();
        {
            let mut prev_relation = None;
            let domain = self.domain();
            let reader = self.read();
            for (cell, rel) in v.iter().rev() {
                if prev_relation.is_none() {
                    write_value_fn(&mut s, &reader, domain.interpretation());
                } else {
                    write_label_fn(
                        &mut s,
                        &reader,
                        domain.interpretation(),
                        prev_relation == Some(Relation::Interpretation),
                    );
                }
                write!(s, "{}", rel).unwrap_or_else(err_fn);
                prev_relation = Some(*rel);
            }
            write_label_fn(
                &mut s,
                &reader,
                domain.interpretation(),
                prev_relation == Some(Relation::Interpretation),
            );
        }
        Ok(CellPath(s))
    }

    pub fn debug_string(&self) -> String {
        let err_fn = |err| eprintln!("💥 str write error {}", err);
        use std::fmt::Write;
        let mut s = String::new();
        match self.read().err() {
            Ok(reader) => {
                match reader.label() {
                    Ok(l) => write!(s, "{}", l).unwrap_or_else(err_fn),
                    Err(HErr::None) => {}
                    Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
                };
                write!(s, ":").unwrap_or_else(err_fn);
                match reader.value() {
                    Ok(v) => write!(s, "{}", v).unwrap_or_else(err_fn),
                    Err(HErr::None) => {}
                    Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
                };
            }
            Err(e) => {
                write!(s, "<💥cannot read: {:?}>", e).unwrap_or_else(err_fn);
            }
        }
        s
    }

    pub fn as_file_path(&self) -> Res<&std::path::Path> {
        if let DynCell::File(ref file_cell) = self.dyn_cell {
            return file_cell.as_path();
        }
        if let DynCell::Path(ref path_cell) = self.dyn_cell {
            return path_cell.as_path();
        }
        nores()
    }

    pub fn as_url_str(&self) -> Res<&str> {
        if let DynCell::Url(ref url) = self.dyn_cell {
            return Ok(url.as_url_str());
        }
        nores()
    }
    // fn http_as_string(cell: &Cell) -> Res<String> {
    //     if let Cell {
    //         dyn_cell: DynCell::Http(ref h),
    //         ..
    //     } = cell
    //     {
    //         return http::to_string(h);
    //     }
    //     fault("elevation: not a http cell")
    // }
    // fn ts_as_string(cell: &Cell) -> Res<String> {
    //     if let Cell {
    //         dyn_cell: DynCell::TreeSitter(ref ts),
    //         ..
    //     } = cell
    //     {
    //         let s = treesitter::get_underlying_string(ts)?;
    //         Ok(s.to_string())
    //     } else {
    //         fault("rust to string elevation")
    //     }
    // }
}

impl Group {
    pub fn label_type(&self) -> LabelType {
        match self {
            Group::Dyn(dyn_group) => {
                dispatch_dyn_group!(dyn_group, |x| { x.label_type() })
            }
            Group::Elevation(elevation_group) => elevation_group.label_type(),
        }
    }

    pub fn len(&self) -> Res<usize> {
        match self {
            Group::Dyn(dyn_group) => {
                dispatch_dyn_group!(dyn_group, |x| { x.len() })
            }
            Group::Elevation(elevation_group) => elevation_group.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == Ok(0)
    }

    pub fn at(&self, index: usize) -> Cell {
        match self {
            Group::Dyn(dyn_group) => {
                dispatch_dyn_group!(dyn_group, |x| {
                    Cell {
                        dyn_cell: match x.at(index) {
                            Ok(c) => DynCell::from(c),
                            Err(e) => DynCell::from(e),
                        },
                    }
                })
            }
            Group::Elevation(elevation_group) => match elevation_group.at(index) {
                Ok(c) => c,
                Err(e) => Cell {
                    dyn_cell: DynCell::from(e),
                },
            },
        }
    }

    pub fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Cell {
        let key = key.into();
        match self {
            Group::Dyn(dyn_group) => {
                dispatch_dyn_group!(dyn_group, |x| {
                    Cell {
                        dyn_cell: match x.get(key) {
                            Ok(c) => DynCell::from(c),
                            Err(e) => DynCell::from(e),
                        },
                    }
                })
            }
            Group::Elevation(elevation_group) => match elevation_group.get(key) {
                Ok(c) => c,
                Err(e) => Cell {
                    dyn_cell: DynCell::from(e),
                },
            },
        }
    }

    pub fn err(self) -> Res<Group> {
        match self {
            Group::Dyn(dyn_group) => match dyn_group {
                DynGroup::Error(error) => Err(error),
                _ => Ok(Group::Dyn(dyn_group)),
            },
            _ => Ok(self),
        }
    }
}

impl IntoIterator for Group {
    type Item = Cell;
    type IntoIter = GroupIter;

    fn into_iter(self) -> Self::IntoIter {
        GroupIter(self, 0)
    }
}

#[derive(Debug)]
pub struct GroupIter(Group, usize);
impl Iterator for GroupIter {
    type Item = Cell;
    fn next(&mut self) -> Option<Self::Item> {
        if self.1 >= self.0.len().unwrap_or(0) {
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
        x.unwrap_or(nores())
    }
    pub fn all(self) -> Res<Vec<Cell>> {
        self.into_iter().collect::<Res<Vec<_>>>()
    }
}
