use crate::pathlang::eval::EvalIter;
use crate::pathlang::Path;
use crate::{
    base::{common::*, elevation::ElevationGroup, interpretation_api::*},
    interpretations::*,
};
use std::rc::Rc;

#[derive(Clone, Debug)]
#[repr(C)]
pub enum Cell {
    String(OwnedValue),
    File(file::Cell),
    Json(json::Cell),
    Toml(toml::Cell),
    Yaml(yaml::Cell),
    Xml(xml::Cell),
    Url(url::Cell),
    Http(http::Cell),
    TreeSitter(treesitter::Cell),
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct OwnedValue(pub(crate) Rc<String>);
impl OwnedValue {
    pub fn new(s: String) -> OwnedValue {
        OwnedValue(Rc::new(s))
    }
}

#[derive(Clone, Debug)]
pub enum Group {
    File(file::Group),
    Json(json::Group),
    Toml(toml::Group),
    Yaml(yaml::Group),
    Xml(xml::Group),
    Url(url::Cell),
    Http(http::Group),
    TreeSitter(treesitter::Group),
    Elevation(ElevationGroup),
    Mixed(Vec<Cell>),
}

impl Default for Cell {
    fn default() -> Self {
        Cell::String(OwnedValue::new(String::new()))
    }
}

impl From<&str> for Cell {
    fn from(s: &str) -> Cell {
        Cell::String(OwnedValue::new(s.into()))
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Cell {
        Cell::String(OwnedValue::new(s))
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Cell {
        Cell::String(OwnedValue::new(format!("{}", v)))
    }
}

impl InterpretationCell for Cell {
    type Group = Group;

    fn typ(&self) -> Res<&str> {
        match self {
            Cell::String(_) => Ok("value"),
            Cell::File(x) => Ok(x.typ()?),
            Cell::Json(x) => Ok(x.typ()?),
            Cell::Toml(x) => Ok(x.typ()?),
            Cell::Yaml(x) => Ok(x.typ()?),
            Cell::Xml(x) => Ok(x.typ()?),
            Cell::Url(x) => Ok(x.typ()?),
            Cell::Http(x) => Ok(x.typ()?),
            Cell::TreeSitter(x) => Ok(x.typ()?),
        }
    }

    fn index(&self) -> Res<usize> {
        match self {
            Cell::String(_) => NotFound::NoIndex().into(),
            Cell::File(x) => Ok(x.index()?),
            Cell::Json(x) => Ok(x.index()?),
            Cell::Toml(x) => Ok(x.index()?),
            Cell::Yaml(x) => Ok(x.index()?),
            Cell::Xml(x) => Ok(x.index()?),
            Cell::Url(x) => Ok(x.index()?),
            Cell::Http(x) => Ok(x.index()?),
            Cell::TreeSitter(x) => Ok(x.index()?),
        }
    }

    fn label(&self) -> Res<&str> {
        match self {
            Cell::String(_) => NotFound::NoLabel().into(),
            Cell::File(x) => Ok(x.label()?),
            Cell::Json(x) => Ok(x.label()?),
            Cell::Toml(x) => Ok(x.label()?),
            Cell::Yaml(x) => Ok(x.label()?),
            Cell::Xml(x) => Ok(x.label()?),
            Cell::Url(x) => Ok(x.label()?),
            Cell::Http(x) => Ok(x.label()?),
            Cell::TreeSitter(x) => Ok(x.label()?),
        }
    }

    fn value(&self) -> Res<Value> {
        match self {
            Cell::String(OwnedValue(x)) => Ok(Value::Str(x.as_str())),
            Cell::File(x) => Ok(x.value()?),
            Cell::Json(x) => Ok(x.value()?),
            Cell::Toml(x) => Ok(x.value()?),
            Cell::Yaml(x) => Ok(x.value()?),
            Cell::Xml(x) => Ok(x.value()?),
            Cell::Url(x) => Ok(x.value()?),
            Cell::Http(x) => Ok(x.value()?),
            Cell::TreeSitter(x) => Ok(x.value()?),
        }
    }

    fn sub(&self) -> Res<Group> {
        match self {
            Cell::String(_) => NotFound::NoGroup("/".into()).into(),
            Cell::File(x) => Ok(Group::File(x.sub()?)),
            Cell::Json(x) => Ok(Group::Json(x.sub()?)),
            Cell::Toml(x) => Ok(Group::Toml(x.sub()?)),
            Cell::Yaml(x) => Ok(Group::Yaml(x.sub()?)),
            Cell::Xml(x) => Ok(Group::Xml(x.sub()?)),
            Cell::Url(x) => Ok(Group::Url(x.sub()?)),
            Cell::Http(x) => Ok(Group::Http(x.sub()?)),
            Cell::TreeSitter(x) => Ok(Group::TreeSitter(x.sub()?)),
        }
    }

    fn attr(&self) -> Res<Group> {
        match self {
            Cell::String(_) => NotFound::NoGroup("@".into()).into(),
            Cell::File(x) => Ok(Group::File(x.attr()?)),
            Cell::Json(x) => Ok(Group::Json(x.attr()?)),
            Cell::Toml(x) => Ok(Group::Toml(x.attr()?)),
            Cell::Yaml(x) => Ok(Group::Yaml(x.attr()?)),
            Cell::Xml(x) => Ok(Group::Xml(x.attr()?)),
            Cell::Url(x) => Ok(Group::Url(x.attr()?)),
            Cell::Http(x) => Ok(Group::Http(x.attr()?)),
            Cell::TreeSitter(x) => Ok(Group::TreeSitter(x.attr()?)),
        }
    }
}

pub trait InterInterpretation {
    fn interpretation(&self) -> &str;

    fn standard_interpretation(&self) -> Option<&str>;

    fn elevate(&self) -> Res<Group>;

    fn be(&self, interp: &str) -> Res<Cell> {
        self.elevate()?.get(interp)
    }
}

impl InterInterpretation for Value<'_> {
    fn interpretation(&self) -> &str {
        "value"
    }

    fn standard_interpretation(&self) -> Option<&str> {
        None
    }

    fn elevate(&self) -> Res<Group> {
        Cell::from(*self).elevate()
    }
}

impl InterInterpretation for Cell {
    fn interpretation(&self) -> &str {
        match self {
            Cell::String(_) => "string",
            Cell::File(_) => "file",
            Cell::Json(_) => "json",
            Cell::Toml(_) => "toml",
            Cell::Yaml(_) => "yaml",
            Cell::Xml(_) => "xml",
            Cell::Url(_) => "url",
            Cell::Http(_) => "http",
            Cell::TreeSitter(cell) => cell.language(),
        }
    }

    fn standard_interpretation(&self) -> Option<&str> {
        match self {
            Cell::String(OwnedValue(str)) => {
                if str.starts_with("http://") || str.starts_with("https://") {
                    return Some("http");
                } else if str.starts_with(".") || str.starts_with("/") {
                    return Some("file");
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

    fn elevate(&self) -> Res<Group> {
        Ok(Group::Elevation(ElevationGroup(self.clone())))
    }
}

impl InterpretationGroup for Group {
    type Cell = Cell;

    fn label_type(&self) -> LabelType {
        match self {
            Group::File(x) => x.label_type(),
            Group::Json(x) => x.label_type(),
            Group::Toml(x) => x.label_type(),
            Group::Yaml(x) => x.label_type(),
            Group::Xml(x) => x.label_type(),
            Group::Url(x) => x.label_type(),
            Group::Http(x) => x.label_type(),
            Group::TreeSitter(x) => x.label_type(),
            Group::Elevation(x) => x.label_type(),
            Group::Mixed(_) => LabelType {
                is_indexed: false,
                unique_labels: false,
            },
        }
    }

    fn len(&self) -> usize {
        match self {
            Group::File(x) => x.len(),
            Group::Json(x) => x.len(),
            Group::Toml(x) => x.len(),
            Group::Yaml(x) => x.len(),
            Group::Xml(x) => x.len(),
            Group::Url(x) => x.len(),
            Group::Http(x) => x.len(),
            Group::TreeSitter(x) => x.len(),
            Group::Elevation(x) => x.len(),
            Group::Mixed(x) => x.len(),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match self {
            Group::File(x) => Ok(Cell::File(x.at(index)?)),
            Group::Json(x) => Ok(Cell::Json(x.at(index)?)),
            Group::Toml(x) => Ok(Cell::Toml(x.at(index)?)),
            Group::Yaml(x) => Ok(Cell::Yaml(x.at(index)?)),
            Group::Xml(x) => Ok(Cell::Xml(x.at(index)?)),
            Group::Url(x) => Ok(Cell::Url(x.at(index)?)),
            Group::Http(x) => Ok(Cell::Http(x.at(index)?)),
            Group::TreeSitter(x) => Ok(Cell::TreeSitter(x.at(index)?)),
            Group::Elevation(x) => x.at(index),
            Group::Mixed(x) => x
                .get(index)
                .map(|x| x.clone())
                .ok_or_else(|| NotFound::NoResult(format!("{}", index)).into()),
        }
    }

    fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Res<Cell> {
        let key = key.into();
        // println!("generic get: {:?}", key);
        match self {
            Group::File(x) => Ok(Cell::File(x.get(key)?)),
            Group::Json(x) => Ok(Cell::Json(x.get(key)?)),
            Group::Toml(x) => Ok(Cell::Toml(x.get(key)?)),
            Group::Yaml(x) => Ok(Cell::Yaml(x.get(key)?)),
            Group::Xml(x) => Ok(Cell::Xml(x.get(key)?)),
            Group::Url(x) => Ok(Cell::Url(x.get(key)?)),
            Group::Http(x) => Ok(Cell::Http(x.get(key)?)),
            Group::TreeSitter(x) => Ok(Cell::TreeSitter(x.get(key)?)),
            Group::Elevation(elevation_group) => elevation_group.get(key),
            Group::Mixed(v) => {
                for x in v {
                    if Selector::Str(x.label()?) == key {
                        return Ok(x.clone());
                    }
                }
                NotFound::NoResult(format!("")).into()
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

impl Cell {
    pub fn path<'a>(&self, path: &'a str) -> Res<PathSearch<'a>> {
        Ok(PathSearch {
            cell: self.clone(),
            path: crate::pathlang::Path::parse(path)?,
        })
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
