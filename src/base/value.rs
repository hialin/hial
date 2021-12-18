use crate::guard_variant;
use core::{
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
};
use std::borrow::Borrow;

pub const DISPLAY_VALUE_NONE: &str = "ø"; // ø❍•⸰·

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

impl From<String> for OwnedValue {
    fn from(s: String) -> Self {
        OwnedValue::String(s)
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

    pub fn as_str(&self) -> Option<&str> {
        guard_variant!(self, Value::Str)
    }
}