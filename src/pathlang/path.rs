use std::fmt::{Display, Formatter};

use crate::{
    base::*,
    interpretations::*,
    pathlang::{eval::EvalIter, parseurl::*},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Path<'a>(pub(crate) Vec<PathItem<'a>>);

#[derive(Clone, Debug)]
pub enum PathStart<'a> {
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

impl Display for Path<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt_path_items(&self.0, f)?;
        Ok(())
    }
}

impl Display for PathItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fmt_path_item(self, f)?;
        Ok(())
    }
}

impl Display for PathStart<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            PathStart::Url(x) => write!(f, "{}", x)?,
            PathStart::File(x) => write!(f, "{}", x)?,
            PathStart::String(x) => write!(f, "'{}'", x)?,
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

impl<'a> PathStart<'a> {
    pub fn eval(&self) -> Res<Cell> {
        match self {
            PathStart::Url(u) => {
                let domain = url::from_string(&u.to_string())?;
                let root = domain.root()?;
                Cell {
                    // domain: Rc::new(Domain {
                    //     this: DynDomain::from(domain),
                    //     source: None,
                    // }),
                    this: DynCell::Url(root),
                }
                .elevate()?
                .get("url")
            }
            PathStart::File(f) => {
                let path = std::path::Path::new(f);
                let domain = file::from_path(path)?;
                let root = domain.root()?;
                Cell {
                    // domain: Rc::new(Domain {
                    //     this: DynDomain::from(domain),
                    //     source: None,
                    // }),
                    this: DynCell::File(root),
                }
                .elevate()?
                .get("file")
            }
            PathStart::String(str) => {
                let root = ownvalue::Domain::from(str.to_string()).root()?;
                Ok(Cell {
                    // domain: Rc::new(Domain {
                    //     this: DynDomain::from(domain),
                    //     source: None,
                    // }),
                    this: DynCell::from(root),
                })
            }
        }
    }
}

impl<'a> Path<'a> {
    pub fn eval(self, cell: Cell) -> EvalIter<'a> {
        EvalIter::new(cell, self)
    }
}
