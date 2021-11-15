use crate::base::value::*;
use core::{
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
};
use std::borrow::Borrow;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum Selector<'a> {
    Str(&'a str),
    Top,
    Star,
    DoubleStar,
}

impl<'a> From<&'a str> for Selector<'a> {
    fn from(s: &'a str) -> Self {
        if s == "*" {
            Selector::Star
        } else if s == "**" {
            Selector::DoubleStar
        } else {
            Selector::Str(s)
        }
    }
}

impl<'a> From<&'a String> for Selector<'a> {
    fn from(s: &'a String) -> Self {
        Self::from(s.as_str())
    }
}

impl<T> PartialEq<T> for Selector<'_>
where
    T: Borrow<str>,
{
    fn eq(&self, other: &T) -> bool {
        match self {
            Selector::Star | Selector::DoubleStar => true,
            Selector::Top => false,
            Selector::Str(s) => s.eq(&other.borrow()),
        }
    }
}

impl PartialEq<Value<'_>> for Selector<'_> {
    fn eq(&self, other: &Value) -> bool {
        if *self == Selector::Star || *self == Selector::DoubleStar {
            return true;
        }
        match other {
            Value::Str(svalue) => self.eq(svalue),
            _ => false,
        }
    }
}

impl PartialEq<Selector<'_>> for Value<'_> {
    fn eq(&self, other: &Selector) -> bool {
        match other {
            Selector::Str(svalue) => self.eq(svalue),
            Selector::Star => true,
            Selector::DoubleStar => true,
            Selector::Top => false,
        }
    }
}

impl<'a> Display for Selector<'a> {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Selector::DoubleStar => write!(buf, "**"),
            Selector::Star => write!(buf, "*"),
            Selector::Str(x) => write!(buf, "{}", x),
            Selector::Top => write!(buf, "^"),
        }
    }
}

impl PartialEq<Selector<'_>> for OwnedValue {
    fn eq(&self, other: &Selector) -> bool {
        match other {
            Selector::Str(svalue) => self.eq(svalue),
            Selector::Star => true,
            Selector::DoubleStar => true,
            Selector::Top => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LabelType {
    pub is_indexed: bool,
    pub unique_labels: bool,
}
