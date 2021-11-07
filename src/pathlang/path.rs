use crate::{
    base::common::*,
    base::rust_api::*,
    pathlang::{eval::EvalIter, parseurl::*},
};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub struct Path<'a>(pub(crate) Vec<PathItem<'a>>);

#[derive(Clone, Debug)]
pub enum CellRepresentation<'a> {
    Url(Url<'a>),
    File(&'a str),
    String(&'a str),
}

#[derive(Clone, Debug)]
pub struct PathItem<'a> {
    pub(crate) relation: Relation,
    pub(crate) selector: Option<Selector<'a>>, // field name (string) or '*' or '**'
    pub(crate) index: Option<usize>,           // or index
    pub(crate) filters: Vec<Filter<'a>>,       // [@size>0] or [.name.endswith('.rs')]
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Relation {
    Attr,           // @
    Sub,            // /
    Interpretation, // ^
}

#[derive(Clone, Debug)]
pub struct Filter<'a> {
    pub(crate) expr: Expression<'a>,
}

#[derive(Clone, Debug)]
pub struct Expression<'a> {
    pub(crate) left_path: Path<'a>,
    pub(crate) left_accessor: Option<&'a str>,
    pub(crate) op: &'a str,
    pub(crate) right: Value<'a>,
}

impl Display for Relation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            Relation::Attr => write!(f, "@"),
            Relation::Sub => write!(f, "/"),
            Relation::Interpretation => write!(f, "^"),
        }
    }
}

impl TryFrom<char> for Relation {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '@' => Ok(Relation::Attr),
            '/' => Ok(Relation::Sub),
            '^' => Ok(Relation::Interpretation),
            _ => Err(()),
        }
    }
}

impl Display for Path<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt_path_items(&self.0, f)?;
        Ok(())
    }
}

impl Display for PathItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt_path_item(&self, f)?;
        Ok(())
    }
}

impl Display for CellRepresentation<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            CellRepresentation::Url(x) => write!(f, "{}", x)?,
            CellRepresentation::File(x) => write!(f, "{}", x)?,
            CellRepresentation::String(x) => write!(f, "'{}'", x)?,
        }
        Ok(())
    }
}

impl Display for Filter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        fmt_path_items(&self.expr.left_path.0, f)?;
        if let Some(accessor) = self.expr.left_accessor {
            write!(f, ".{}", accessor)?;
        }
        write!(f, "{}", self.expr.op)?;
        match self.expr.right {
            Value::Str(s) => write!(f, "'{}'", s)?,
            x @ _ => write!(f, "{}", x)?,
        }
        write!(f, "]")?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DisplayRefPath<'a, 'b: 'a, 'c: 'b>(pub(crate) &'c Vec<&'b PathItem<'a>>);
impl<'a, 'b: 'a, 'c: 'b> Display for DisplayRefPath<'a, 'b, 'c> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for it in self.0 {
            fmt_path_item(it, f)?
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct DisplayPath<'a, 'b: 'a>(pub(crate) &'b Vec<PathItem<'a>>);
impl<'a, 'b: 'a> Display for DisplayPath<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for it in self.0 {
            fmt_path_item(it, f)?
        }
        Ok(())
    }
}

fn fmt_path_items(path_items: &Vec<PathItem>, f: &mut Formatter<'_>) -> std::fmt::Result {
    for it in path_items {
        fmt_path_item(it, f)?
    }
    Ok(())
}
fn fmt_path_item(path_item: &PathItem, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", path_item.relation)?;
    if let Some(sel) = path_item.selector {
        write!(f, "{}", sel)?;
    }
    if let Some(idx) = path_item.index {
        write!(f, "[{}]", idx)?;
    }
    for filter in &path_item.filters {
        write!(f, "{}", filter)?;
    }
    Ok(())
}

impl<'a> CellRepresentation<'a> {
    pub fn eval(&self) -> Res<Cell> {
        match self {
            CellRepresentation::Url(url) => Cell::from(url.to_string()).be("url"),
            CellRepresentation::File(file) => Cell::from(file.to_string()).be("file"),
            CellRepresentation::String(str) => Ok(Cell::from(*str)),
        }
    }
}

impl<'a> Path<'a> {
    pub fn eval(self, cell: Cell) -> EvalIter<'a> {
        EvalIter::new(cell, self)
    }
}
