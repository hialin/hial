use std::rc::Rc;

use crate::{
    base::*, enumerated_dynamic_type, interpretations::*, pathlang::eval::EvalIter, pathlang::Path,
};

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynDomain {
        OwnValue(ownvalue::Domain),
        File(file::Domain),
        Json(json::Domain),
        Toml(toml::Domain),
        Yaml(yaml::Domain),
        Xml(xml::Domain),
        Url(url::Domain),
        Path(path::Domain),
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
    with_dyn_cell
}

#[derive(Clone, Debug)]
pub struct Cell {
    // pub(crate) domain: Rc<Domain>,
    pub(crate) this: DynCell,
}

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub(crate) enum DynCellReader {
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
    with_cell_reader
}

#[derive(Debug)]
pub struct CellReader(DynCellReader);

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub(crate) enum DynCellWriter {
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
    with_cell_writer
}

#[derive(Debug)]
pub struct CellWriter(DynCellWriter);

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub enum DynGroup {
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
    with_dyn_group
}

#[derive(Clone, Debug)]
pub enum Group {
    Dyn(DynGroup),
    Elevation(ElevationGroup),
    // Mixed(Vec<Cell>),
}

impl Domain {
    pub fn interpretation(&self) -> &str {
        with_dyn_domain!(&self.this, |x| { x.interpretation() })
    }

    pub fn root(&self) -> Res<Cell> {
        with_dyn_domain!(&self.this, |x| {
            Ok(Cell {
                // domain: Rc::new(self.clone()),
                this: DynCell::from(x.root()?),
            })
        })
    }

    // TODO: implement write_back()? or find a better way
    // pub fn write_back(&self) -> Res<()> {
    // }
}

impl From<OwnValue> for Cell {
    fn from(ov: OwnValue) -> Self {
        Cell {
            this: DynCell::from(ownvalue::Domain::from(ov).root().unwrap()),
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
        Cell::from(OwnValue::from(s.to_string()))
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell::from(OwnValue::from(s))
    }
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        with_cell_reader!(&self.0, |x| { Ok(x.index()?) })
    }
    fn label(&self) -> Res<Value> {
        with_cell_reader!(&self.0, |x| { Ok(x.label()?) })
    }
    fn value(&self) -> Res<Value> {
        with_cell_reader!(&self.0, |x| { Ok(x.value()?) })
    }
}

impl CellWriterTrait for CellWriter {
    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        with_cell_writer!(&mut self.0, |x| { x.set_label(value) })
    }

    fn set_value(&mut self, ov: OwnValue) -> Res<()> {
        with_cell_writer!(&mut self.0, |x| { x.set_value(ov) })
    }
}

impl Cell {
    pub fn interpretation(&self) -> &str {
        // TODO: this is not fully correct
        match &self.this {
            DynCell::Field(_) => "value",
            DynCell::OwnValue(_) => "value",
            DynCell::File(_) => "file",
            DynCell::Json(_) => "json",
            DynCell::Toml(_) => "toml",
            DynCell::Yaml(_) => "yaml",
            DynCell::Xml(_) => "xml",
            DynCell::Url(_) => "url",
            DynCell::Path(_) => "path",
            DynCell::Http(_) => "http",
            DynCell::TreeSitter(_) => "treesitter",
        }
    }

    pub fn typ(&self) -> Res<&str> {
        with_dyn_cell!(&self.this, |x| { Ok(x.typ()?) })
    }

    pub fn read(&self) -> Res<CellReader> {
        let reader: DynCellReader =
            with_dyn_cell!(&self.this, |x| { DynCellReader::from(x.read()?) });
        Ok(CellReader(reader))
    }

    pub fn write(&self) -> Res<CellWriter> {
        let writer: DynCellWriter =
            with_dyn_cell!(&self.this, |x| { DynCellWriter::from(x.write()?) });
        Ok(CellWriter(writer))
    }

    pub fn sub(&self) -> Res<Group> {
        let sub = with_dyn_cell!(&self.this, |x| { DynGroup::from(x.sub()?) });
        Ok(Group::Dyn(sub))
    }

    pub fn attr(&self) -> Res<Group> {
        let attr = with_dyn_cell!(&self.this, |x| { DynGroup::from(x.attr()?) });
        Ok(Group::Dyn(attr))
    }

    pub fn standard_interpretation(&self) -> Option<&str> {
        elevation::standard_interpretation(self)
    }

    pub fn elevate(self) -> Res<Group> {
        Ok(Group::Elevation(ElevationGroup(self.clone())))
    }

    pub fn field(&self) -> Res<Group> {
        Ok(Group::Dyn(DynGroup::from(FieldGroup {
            cell: Rc::new(self.clone()),
        })))
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
        // TODO: path is not complete/correct (requires prev cell)
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

        let v: Vec<(Cell, Relation)> = vec![];
        // {
        //     let mut a = self.prev.as_ref();
        //     while let Some((cell, rel)) = a {
        //         v.push((cell.clone(), *rel));
        //         a = cell.prev.as_ref();
        //     }
        // }

        let mut s = String::new();
        {
            let mut prev_relation = None;
            let interpretation = self.interpretation();
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

    pub fn debug_string(&self) -> String {
        let err_fn = |err| eprintln!("💥 str write error {}", err);
        use std::fmt::Write;
        let mut s = String::new();
        match self.read() {
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
}

impl Group {
    pub fn label_type(&self) -> LabelType {
        match self {
            Group::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| { x.label_type() })
            }
            Group::Elevation(elevation_group) => elevation_group.label_type(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Group::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| { x.len() })
            }
            Group::Elevation(elevation_group) => elevation_group.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn at(&self, index: usize) -> Res<Cell> {
        match self {
            Group::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| {
                    Ok(Cell {
                        // domain: self.domain.clone(),
                        this: DynCell::from(x.at(index)?),
                    })
                })
            }
            Group::Elevation(elevation_group) => elevation_group.at(index),
        }
    }

    pub fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        match self {
            Group::Dyn(dyn_group) => {
                with_dyn_group!(dyn_group, |x| {
                    Ok(Cell {
                        // domain: self.domain.clone(),
                        this: DynCell::from(x.get(key)?),
                    })
                })
            }
            Group::Elevation(elevation_group) => elevation_group.get(key),
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
        x.unwrap_or(nores())
    }
    pub fn all(self) -> Res<Vec<Cell>> {
        self.into_iter().collect::<Res<Vec<_>>>()
    }
}
