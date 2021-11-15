use crate::{
    base::*,
    pathlang::{eval::EvalIter, parseurl::*},
};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq)]
pub struct Path<'a>(pub(crate) Vec<PathItem<'a>>);

#[derive(Clone, Debug)]
pub enum CellRepresentation<'a> {
    Url(Url<'a>),
    File(&'a str),
    String(&'a str),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathItem<'a> {
    pub(crate) relation: Relation,
    pub(crate) selector: Option<Selector<'a>>, // field name (string) or '*' or '**'
    pub(crate) index: Option<usize>,           // or index
    pub(crate) filters: Vec<Filter<'a>>,       // [@size>0] or [.name.endswith('.rs')]
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum Relation {
    Attr = '@' as u8,
    Sub = '/' as u8,
    Interpretation = '^' as u8,
    Field = '#' as u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Filter<'a> {
    pub(crate) expr: Expression<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expression<'a> {
    pub(crate) left: Path<'a>,
    pub(crate) op: Option<&'a str>,
    pub(crate) right: Option<Value<'a>>,
}

impl Display for Relation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u8 as char)
    }
}

impl TryFrom<char> for Relation {
    type Error = ();

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let value = value as u8;
        if value == Relation::Attr as u8 {
            Ok(Relation::Attr)
        } else if value == Relation::Sub as u8 {
            Ok(Relation::Sub)
        } else if value == Relation::Interpretation as u8 {
            Ok(Relation::Interpretation)
        } else if value == Relation::Field as u8 {
            Ok(Relation::Field)
        } else {
            Err(())
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
        fmt_path_items(&self.expr.left.0, f)?;
        if let Some(op) = self.expr.op {
            write!(f, "{}", op)?;
        }
        match self.expr.right {
            Some(Value::Str(s)) => write!(f, "'{}'", s)?,
            Some(v) => write!(f, "{}", v)?,
            None => {}
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
            CellRepresentation::Url(url) => Cell::from(url.to_string()).elevate()?.get("url"),
            CellRepresentation::File(file) => Cell::from(file.to_string()).elevate()?.get("file"),
            CellRepresentation::String(str) => Ok(Cell::from(str.to_string())),
        }
    }
}

impl<'a> Path<'a> {
    pub fn eval(self, cell: Cell) -> EvalIter<'a> {
        EvalIter::new(cell, self)
    }
}
