use core::{
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
};
use std::borrow::Borrow;

pub type Res<T> = Result<T, HErr>;

pub const DISPLAY_VALUE_NONE: &str = "ø"; // ø❍•⸰·

#[derive(Clone, Debug)]
#[repr(C)]
pub enum HErr {
    Internal(String),
    NotFound(NotFound),
    BadArgument(String),
    BadPath(String),
    IO(std::io::ErrorKind, String),
    Json(String),
    Toml(String),
    Yaml(String),
    Xml(String),
    Url(String),
    Http(String),
    Sitter(String),
    Other(String),
}

#[derive(Clone, Debug)]
pub enum NotFound {
    NoLabel(), // the cell has no label
    NoIndex(), // the cell has no index
    NoGroup(String),
    NoResult(String),
    NoInterpretation(String),
}

impl From<NotFound> for HErr {
    fn from(e: NotFound) -> Self {
        HErr::NotFound(e)
    }
}

impl<T> From<NotFound> for Res<T> {
    fn from(e: NotFound) -> Self {
        Err(HErr::NotFound(e))
    }
}

impl<T> From<HErr> for Res<T> {
    fn from(e: HErr) -> Self {
        Err(e)
    }
}

impl HErr {
    pub fn internal<T: Into<String>>(msg: T) -> HErr {
        if cfg!(debug_assertions) {
            eprintln!("{}", msg.into());
            panic!("internal error");
        } else {
            HErr::Internal(msg.into())
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, Ord, Hash)]
pub enum Int {
    I64(i64),
    U64(u64),
    I32(i32),
    U32(u32),
}

impl PartialEq for Int {
    fn eq(&self, other: &Self) -> bool {
        match *self {
            Int::I64(a) => match *other {
                Int::I64(b) => a == b,
                Int::U64(b) => a >= 0 && a as u64 == b,
                Int::I32(b) => a == b as i64,
                Int::U32(b) => a >= 0 && a as u64 == b as u64,
            },
            Int::U64(a) => match *other {
                Int::I64(b) => b >= 0 && a == b as u64,
                Int::U64(b) => a == b,
                Int::I32(b) => b >= 0 && a == b as u64,
                Int::U32(b) => a == b as u64,
            },
            Int::I32(a) => match *other {
                Int::I64(b) => a as i64 == b,
                Int::U64(b) => a >= 0 && a as u64 == b,
                Int::I32(b) => a == b,
                Int::U32(b) => a >= 0 && a as u32 == b,
            },
            Int::U32(a) => match *other {
                Int::I64(b) => b >= 0 && a as u64 == b as u64,
                Int::U64(b) => a as u64 == b,
                Int::I32(b) => b >= 0 && a == b as u32,
                Int::U32(b) => a == b,
            },
        }
    }
}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let big = |x| match x {
            Int::I64(a) => a as i128,
            Int::U64(a) => a as i128,
            Int::I32(a) => a as i128,
            Int::U32(a) => a as i128,
        };
        big(*self).partial_cmp(&big(*other))
    }
}

impl<'a> Default for Int {
    fn default() -> Self {
        Int::I64(0)
    }
}

impl<'a> Display for Int {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Int::I32(x) => write!(buf, "{}", x),
            Int::U32(x) => write!(buf, "{}", x),
            Int::I64(x) => write!(buf, "{}", x),
            Int::U64(x) => write!(buf, "{}", x),
        }
    }
}

/// StrFloat is the human representation of a float, and such representations
/// are fully ordered (Nan < -Inf < .. < -0 < +0 < +Inf) and fully equivalent
#[derive(Copy, Clone, Debug)]
pub struct StrFloat(pub f64);
impl PartialOrd for StrFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StrFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        // copied from total_cmp which is currently on nightly
        let mut left = self.0.to_bits() as i64;
        let mut right = other.0.to_bits() as i64;

        left ^= (((left >> 63) as u64) >> 1) as i64;
        right ^= (((right >> 63) as u64) >> 1) as i64;

        left.cmp(&right)
    }
}
impl PartialEq for StrFloat {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() {
            return other.0.is_nan();
        } else {
            self.0 == other.0
        }
    }
}
impl Eq for StrFloat {}
impl Hash for StrFloat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.0 == 0.0 {
            if self.0.is_sign_negative() {
                "-0".hash(state)
            } else {
                "+0".hash(state)
            }
        } else if self.0.is_finite() {
            self.0.to_bits().hash(state);
        } else if self.0.is_nan() {
            "NaN".hash(state);
        } else if self.0.is_infinite() {
            if self.0.is_sign_negative() {
                "-".hash(state)
            } else {
                "+".hash(state)
            };
            "Inf".hash(state);
        }
    }
}
impl Display for StrFloat {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        write!(buf, "{}", self.0)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value<'a> {
    None,
    Bool(bool),
    Int(Int),
    Float(StrFloat),
    Str(&'a str),
    // OsStr(&'a OsStr),
    Bytes(&'a [u8]),
}

impl<'a> Default for Value<'a> {
    fn default() -> Self {
        Value::None
    }
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::None => write!(buf, "{}", DISPLAY_VALUE_NONE),
            Value::Bool(x) => write!(buf, "{}", x),
            Value::Int(x) => write!(buf, "{}", x),
            Value::Float(x) => write!(buf, "{}", x),
            Value::Str(x) => write!(buf, "{}", x),
            // Value::OsStr(x) => write!(buf, "{}", x.to_string_lossy()),
            Value::Bytes(x) => write!(buf, "{}", String::from_utf8_lossy(x)),
        }
    }
}

impl<T> PartialEq<T> for Value<'_>
where
    T: Borrow<str>,
{
    fn eq(&self, other: &T) -> bool {
        match self {
            Value::Str(s) => s.eq(&other.borrow()),
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

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum OwnedValue {
    None,
    Bool(bool),
    Int(Int),
    Float(StrFloat),
    String(String),
    // OsString(OsString),
    Bytes(Vec<u8>),
}

impl Default for OwnedValue {
    fn default() -> Self {
        OwnedValue::None
    }
}

impl Display for OwnedValue {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OwnedValue::None => write!(buf, "{}", DISPLAY_VALUE_NONE),
            OwnedValue::Bool(x) => write!(buf, "{}", x),
            OwnedValue::Int(x) => write!(buf, "{}", x),
            OwnedValue::Float(x) => write!(buf, "{}", x),
            OwnedValue::String(x) => write!(buf, "{}", x),
            OwnedValue::Bytes(x) => write!(buf, "{}", String::from_utf8_lossy(x)),
        }
    }
}

impl<T> PartialEq<T> for OwnedValue
where
    T: Borrow<str>,
{
    fn eq(&self, other: &T) -> bool {
        match self {
            OwnedValue::String(s) => s.eq(&other.borrow()),
            _ => false,
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

impl<'a> From<&'a OwnedValue> for Value<'a> {
    fn from(ov: &'a OwnedValue) -> Self {
        match ov {
            OwnedValue::None => Value::None,
            OwnedValue::Bool(x) => Value::Bool(*x),
            OwnedValue::Int(x) => Value::Int(*x),
            OwnedValue::Float(x) => Value::Float(*x),
            OwnedValue::String(x) => Value::Str(&x),
            OwnedValue::Bytes(x) => Value::Bytes(&x),
        }
    }
}

impl Value<'_> {
    pub fn to_owned_value(&self) -> OwnedValue {
        match self {
            Value::None => OwnedValue::None,
            Value::Bool(x) => OwnedValue::Bool(*x),
            Value::Int(x) => OwnedValue::Int(*x),
            Value::Float(x) => OwnedValue::Float(*x),
            Value::Str(x) => OwnedValue::String(x.to_string()),
            Value::Bytes(x) => OwnedValue::Bytes(Vec::from(*x)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LabelType {
    pub is_indexed: bool,
    pub unique_labels: bool,
}
