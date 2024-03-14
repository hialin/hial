use std::fmt::{Display, Formatter};

use crate::{
    api::*,
    search::{searcher::Searcher, url::*},
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
pub(crate) enum PathItem<'a> {
    Elevation(ElevationPathItem<'a>),
    Normal(NormalPathItem<'a>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ElevationPathItem<'a> {
    pub(crate) interpretation: Selector<'a>,
    pub(crate) params: Vec<InterpretationParam<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InterpretationParam<'a> {
    pub(crate) name: &'a str,
    pub(crate) value: Option<Value<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NormalPathItem<'a> {
    pub(crate) relation: Relation,
    pub(crate) selector: Option<Selector<'a>>, // field name (string) or '*' or '**'
    pub(crate) index: Option<isize>,
    pub(crate) filters: Vec<Filter<'a>>, // [@size>0] or [.name.endswith('.rs')]
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Filter<'a> {
    pub(crate) expr: Expression<'a>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expression<'a> {
    Ternary {
        left: Path<'a>,
        op: Option<&'a str>,
        right: Option<Value<'a>>,
    },
    Type {
        ty: &'a str,
    },
    Or {
        expressions: Vec<Expression<'a>>,
    },
}

impl Display for Path<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for it in &self.0 {
            write!(f, "{}", it)?;
        }
        Ok(())
    }
}

impl Display for PathItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PathItem::Elevation(e) => {
                write!(f, "{}", e)?;
            }
            PathItem::Normal(n) => {
                write!(f, "{}", n)?;
            }
        }
        Ok(())
    }
}

impl Display for ElevationPathItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "^{}", self.interpretation)?;
        if !self.params.is_empty() {
            write!(f, "[")?;
            for (i, param) in self.params.iter().enumerate() {
                write!(f, "{}", param)?;
                if i < self.params.len() - 1 {
                    write!(f, ",")?;
                }
            }
            write!(f, "]")?;
        }
        Ok(())
    }
}

impl Display for NormalPathItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.relation)?;
        if let Some(sel) = self.selector {
            write!(f, "{}", sel)?;
        }
        if let Some(idx) = self.index {
            write!(f, "[{}]", idx)?;
        }
        for filter in &self.filters {
            write!(f, "{}", filter)?;
        }
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
        write!(f, "[{}]", self.expr)?;
        Ok(())
    }
}

impl Display for Expression<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Ternary { left, op, right } => {
                write!(f, "{}", left)?;
                if let Some(op) = op {
                    write!(f, "{}", op)?;
                }
                if let Some(right) = right {
                    write!(f, "{:?}", right)?;
                }
            }
            Expression::Type { ty } => {
                write!(f, ":{}", ty)?;
            }
            Expression::Or { expressions } => {
                for (i, expr) in expressions.iter().enumerate() {
                    write!(f, "{}", expr)?;
                    if i < expressions.len() - 1 {
                        write!(f, "|")?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl Display for InterpretationParam<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(v) = self.value {
            write!(f, "={}", v)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayRefPath<'a, 'b: 'a, 'c: 'b>(pub(crate) &'c Vec<&'b PathItem<'a>>);
impl<'a, 'b: 'a, 'c: 'b> Display for DisplayRefPath<'a, 'b, 'c> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for it in self.0 {
            write!(f, "{}", it)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DisplayPath<'a, 'b: 'a>(pub(crate) &'b Vec<PathItem<'a>>);
impl<'a, 'b: 'a> Display for DisplayPath<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for it in self.0 {
            write!(f, "{}", it)?;
        }
        Ok(())
    }
}

impl<'a> PathStart<'a> {
    pub fn eval(&self) -> Res<Xell> {
        match self {
            PathStart::Url(s) => Xell::from(s.to_string()).be("url").err(),
            PathStart::File(s) => Xell::from(*s).be("path").be("fs").err(),
            PathStart::String(s) => Xell::from(*s).err(),
        }
    }
}

impl<'a> Path<'a> {
    pub fn eval(self, cell: Xell) -> Searcher<'a> {
        Searcher::new(cell, self)
    }
}
