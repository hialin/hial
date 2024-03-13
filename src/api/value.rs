use core::{
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
};
use std::{
    borrow::{Borrow, Cow},
    fmt::Write,
};

use indexmap::Equivalent;

pub const DISPLAY_VALUE_NONE: &str = "ø"; // ❍•⸰·
pub const DISPLAY_BYTES_VALUE_LEN: usize = 72;

#[derive(Copy, Clone, Debug)]
pub enum Int {
    I64(i64),
    U64(u64),
    I32(i32),
    U32(u32),
}

impl Int {
    pub fn as_i128(&self) -> i128 {
        match self {
            Int::I64(x) => *x as i128,
            Int::U64(x) => *x as i128,
            Int::I32(x) => *x as i128,
            Int::U32(x) => *x as i128,
        }
    }
}

impl Hash for Int {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        self.as_i128().hash(state);
    }
}

impl PartialEq for Int {
    fn eq(&self, other: &Self) -> bool {
        self.as_i128() == other.as_i128()
    }
}

impl Eq for Int {}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.as_i128().cmp(&other.as_i128()))
    }
}

impl Ord for Int {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_i128().cmp(&other.as_i128())
    }
}

impl Default for Int {
    fn default() -> Self {
        Int::I64(0)
    }
}

impl Display for Int {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Int::I32(x) => write!(buf, "{}", x),
            Int::U32(x) => write!(buf, "{}", x),
            Int::I64(x) => write!(buf, "{}", x),
            Int::U64(x) => write!(buf, "{}", x),
        }
    }
}

impl From<i32> for Int {
    fn from(x: i32) -> Self {
        Int::I32(x)
    }
}
impl From<u32> for Int {
    fn from(x: u32) -> Self {
        Int::U32(x)
    }
}
impl From<i64> for Int {
    fn from(x: i64) -> Self {
        Int::I64(x)
    }
}
impl From<u64> for Int {
    fn from(x: u64) -> Self {
        Int::U64(x)
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
            other.0.is_nan()
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

// Value is a simple value, either null or a primitive or a string or bytes
// It implements most of the traits that are useful for a simple value
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum Value<'a> {
    #[default]
    None,
    Bool(bool),
    Int(Int),
    Float(StrFloat),
    Str(&'a str),
    // OsStr(&'a OsStr),
    Bytes(&'a [u8]),
}

impl Hash for Value<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::None => DISPLAY_VALUE_NONE.hash(state),
            Value::Bool(x) => x.hash(state),
            Value::Int(x) => x.hash(state),
            Value::Float(x) => x.hash(state),
            Value::Str(x) => x.hash(state),
            // Value::OsStr(x) => x.to_string_lossy().hash(state),
            Value::Bytes(x) => x.hash(state),
        }
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

impl<'a> fmt::Debug for Value<'a> {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::None => write!(buf, "Value::None"),
            Value::Bool(x) => write!(buf, "Value::Bool({})", x),
            Value::Int(x) => write!(buf, "Value::Int({})", x),
            Value::Float(x) => write!(buf, "Value::Float({})", x),
            Value::Str(x) => write!(buf, "Value::Str({:?})", x),
            // Value::OsStr(x) => write!(buf, "{}", x.to_string_lossy()),
            Value::Bytes(x) => {
                write!(buf, "Value::Bytes(len {}, \"", x.len())?;
                write_bytes(buf, x)?;
                write!(buf, "\")")
            }
        }
    }
}

pub(crate) fn write_bytes(buf: &mut impl Write, x: &[u8]) -> fmt::Result {
    let sb = String::from_utf8_lossy(x);
    let s = sb.as_ref();
    let not_ascii = |c| !(' '..='~').contains(&c);
    if s.contains(not_ascii) {
        let s = s.replace(not_ascii, ".");
        if s.len() >= DISPLAY_BYTES_VALUE_LEN {
            write!(buf, "{}…", &s[..DISPLAY_BYTES_VALUE_LEN])
        } else {
            write!(buf, "{}", s)
        }
    } else if s.len() >= DISPLAY_BYTES_VALUE_LEN {
        write!(buf, "{}…", &s[..DISPLAY_BYTES_VALUE_LEN])
    } else {
        write!(buf, "{}", s)
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

///////////////////////////////////////////////////////////////////////////////
//  OwnValue

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum OwnValue {
    #[default]
    None,
    Bool(bool),
    Int(Int),
    Float(StrFloat),
    String(String),
    // OsString(OsString),
    Bytes(Vec<u8>),
}

impl Display for OwnValue {
    fn fmt(&self, buf: &mut fmt::Formatter) -> fmt::Result {
        self.as_value().fmt(buf)
    }
}

impl<T> PartialEq<T> for OwnValue
where
    T: Borrow<str>,
{
    fn eq(&self, other: &T) -> bool {
        match self {
            OwnValue::String(s) => s.eq(&other.borrow()),
            _ => false,
        }
    }
}

impl<'a, T: Into<Value<'a>>> From<T> for OwnValue {
    fn from(x: T) -> Self {
        x.into().to_owned_value()
    }
}
impl From<String> for OwnValue {
    fn from(s: String) -> Self {
        OwnValue::String(s)
    }
}

impl OwnValue {
    pub fn as_value(&self) -> Value {
        match self {
            OwnValue::None => Value::None,
            OwnValue::Bool(x) => Value::Bool(*x),
            OwnValue::Int(x) => Value::Int(*x),
            OwnValue::Float(x) => Value::Float(*x),
            OwnValue::String(x) => Value::Str(x.as_str()),
            OwnValue::Bytes(x) => Value::Bytes(x.as_ref()),
        }
    }
}

impl Hash for OwnValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // same hash as Value
        self.as_value().hash(state);
    }
}

impl Value<'_> {
    pub fn to_owned_value(&self) -> OwnValue {
        match self {
            Value::None => OwnValue::None,
            Value::Bool(x) => OwnValue::Bool(*x),
            Value::Int(x) => OwnValue::Int(*x),
            Value::Float(x) => OwnValue::Float(*x),
            Value::Str(x) => OwnValue::String(x.to_string()),
            Value::Bytes(x) => OwnValue::Bytes(Vec::from(*x)),
        }
    }

    pub fn as_i128(&self) -> Option<i128> {
        match self {
            Value::Int(x) => Some(x.as_i128()),
            _ => None,
        }
    }

    pub fn as_cow_str(&self) -> Cow<str> {
        match self {
            Value::Str(x) => Cow::Borrowed(x),
            _ => Cow::Owned(self.to_string()),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Value::None => true,
            Value::Str(x) => x.is_empty(),
            Value::Bytes(x) => x.is_empty(),
            _ => false,
        }
    }
}

impl From<bool> for Value<'_> {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}
impl From<Int> for Value<'_> {
    fn from(i: Int) -> Self {
        Value::Int(i)
    }
}
impl From<i32> for Value<'_> {
    fn from(x: i32) -> Self {
        Value::Int(Int::I32(x))
    }
}
impl From<u32> for Value<'_> {
    fn from(x: u32) -> Self {
        Value::Int(Int::U32(x))
    }
}
impl From<i64> for Value<'_> {
    fn from(x: i64) -> Self {
        Value::Int(Int::I64(x))
    }
}
impl From<u64> for Value<'_> {
    fn from(x: u64) -> Self {
        Value::Int(Int::U64(x))
    }
}
impl From<StrFloat> for Value<'_> {
    fn from(f: StrFloat) -> Self {
        Value::Float(f)
    }
}
impl From<f64> for Value<'_> {
    fn from(f: f64) -> Self {
        Value::Float(StrFloat(f))
    }
}
impl From<f32> for Value<'_> {
    fn from(f: f32) -> Self {
        Value::Float(StrFloat(f as f64))
    }
}
impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Str(s)
    }
}
impl<'a> From<&'a String> for Value<'a> {
    fn from(s: &'a String) -> Self {
        Value::Str(s.as_str())
    }
}

impl Equivalent<OwnValue> for Value<'_> {
    fn equivalent(&self, key: &OwnValue) -> bool {
        match key {
            OwnValue::None => matches!(self, Value::None),
            OwnValue::Bool(x) => matches!(self, Value::Bool(y) if x == y),
            OwnValue::Int(x) => matches!(self, Value::Int(y) if x == y),
            OwnValue::Float(x) => matches!(self, Value::Float(y) if x == y),
            OwnValue::String(x) => matches!(self, Value::Str(y) if x == y),
            OwnValue::Bytes(x) => matches!(self, Value::Bytes(y) if x == y),
        }
    }
}

#[cfg(test)]
#[test]
fn test_equivalence() {
    let ov = OwnValue::from("hello");
    let v = Value::from("hello");
    assert!(v.equivalent(&ov));

    let ov = Value::from(1).to_owned_value();
    let v = Value::from(1);
    assert!(v.equivalent(&ov));
}
