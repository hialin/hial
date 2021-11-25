use crate::{
    base::*, interpretations::*, pathlang::eval::EvalIter, pathlang::Path,
    pub_enumerated_dynamic_type,
};

pub_enumerated_dynamic_type! {
    enum Domain {
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

pub_enumerated_dynamic_type! {
    enum Cell {
        Field(Field),
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
    with_cell
}

pub_enumerated_dynamic_type! {
    enum Group {
        // Void(VoidGroup<VoidDomain<>>),
        Elevation(ElevationGroup),
        Field(Field),
        // Mixed(Vec<Cell>),

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
    with_group
}

impl Domain {
    pub fn root(&self) -> Res<Cell> {
        with_domain!(self, |x| { Ok(Cell::from(x.root()?)) })
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
        Cell::from(ownedvalue::Cell::from(ov))
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell::from(ownedvalue::Cell::from(v))
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        Cell::from(ownedvalue::Cell::from(s))
    }
}

impl Cell {
    pub fn typ(&self) -> Res<&str> {
        with_cell!(self, |x| { Ok(x.typ()?) })
    }

    pub fn index(&self) -> Res<usize> {
        with_cell!(self, |x| { Ok(x.index()?) })
    }

    pub fn label(&self) -> Res<&str> {
        with_cell!(self, |x| { Ok(x.label()?) })
    }

    pub fn value(&self) -> Res<Value> {
        with_cell!(self, |x| { Ok(x.value()?) })
    }

    pub fn sub(&self) -> Res<Group> {
        with_cell!(self, |x| { Ok(Group::from(x.sub()?)) })
    }

    pub fn attr(&self) -> Res<Group> {
        with_cell!(self, |x| { Ok(Group::from(x.attr()?)) })
    }

    pub fn standard_interpretation(&self) -> Option<&str> {
        match self {
            Cell::OwnedValue(ov) => {
                if let Ok(Value::Str(s)) = ov.value() {
                    if s.starts_with("http://") || s.starts_with("https://") {
                        return Some("http");
                    } else if s.starts_with(".") || s.starts_with("/") {
                        return Some("file");
                    }
                }
            }
            Cell::File(file) => {
                if file.typ().ok()? == "file" {
                    let name = file.label().ok()?;
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
            _ => {}
        }
        None
    }

    pub fn elevate(self) -> Res<Group> {
        Ok(Group::Elevation(ElevationGroup(self)))
    }

    pub fn field(&self) -> Res<Group> {
        Ok(Group::Field(Field(
            Box::new(self.clone()),
            FieldType::Value,
        )))
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

    pub fn set_value(&mut self, ov: OwnedValue) -> Res<()> {
        with_cell!(self, |x| { x.set_value(ov) })
    }

    pub fn set_label(&mut self, ov: OwnedValue) -> Res<()> {
        with_cell!(self, |x| { x.set_label(ov) })
    }

    pub fn domain(&self) -> Domain {
        with_cell!(self, |x| { Domain::from(x.domain().clone()) })
    }

    pub fn as_data_source(&self) -> Option<Res<DataSource>> {
        with_cell!(self, |x| { x.as_data_source() })
    }

    // pub fn as_data_destination(&mut self) -> Option<Res<DataDestination>> {
    //     with_cell!(self, |x| { x.as_data_destination() })
    // }
}

impl Group {
    pub fn label_type(&self) -> LabelType {
        with_group!(self, |x| { x.label_type() })
    }

    pub fn len(&self) -> usize {
        with_group!(self, |x| { x.len() })
    }

    pub fn at(&self, index: usize) -> Res<Cell> {
        with_group!(self, |x| { Ok(Cell::from(x.at(index)?)) })
    }

    pub fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        match self {
            Group::Elevation(elevation_group) => elevation_group.get(key),
            Group::Field(field) => Ok(Cell::Field(field.clone())),
            // Group::Mixed(v) => {
            //     for x in v {
            //         if Selector::Str(x.label()?) == key {
            //             return Ok(x.clone());
            //         }
            //     }
            //     NotFound::NoResult(format!("")).into()
            // }
            Group::OwnedValue(x) => Ok(Cell::OwnedValue(x.get(key)?)),
            Group::File(x) => Ok(Cell::File(x.get(key)?)),
            Group::Json(x) => Ok(Cell::Json(x.get(key)?)),
            Group::Toml(x) => Ok(Cell::Toml(x.get(key)?)),
            Group::Yaml(x) => Ok(Cell::Yaml(x.get(key)?)),
            Group::Xml(x) => Ok(Cell::Xml(x.get(key)?)),
            Group::Url(x) => Ok(Cell::Url(x.get(key)?)),
            Group::Http(x) => Ok(Cell::Http(x.get(key)?)),
            Group::TreeSitter(x) => Ok(Cell::TreeSitter(x.get(key)?)),
        }
    }
}

// impl From<intra::VoidGroup<facade::Domain>> for Group {
//     fn from(_: VoidGroup<Domain>) -> Self {
//         Group::Void(VoidGroup::from(()))
//     }
// }

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
