use core::{
    cmp::Ordering,
    fmt::{self, Display},
    hash::{Hash, Hasher},
};
use std::borrow::{Borrow, Cow};

pub const DISPLAY_VALUE_NONE: &str = "ø"; // ❍•⸰·

#[derive(Copy, Clone, Debug)]
pub enum Int {
    I64(i64),
    U64(u64),
    I32(i32),
    U32(u32),
}

impl Hash for Int {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        embiggen(*self).hash(state);
    }
}

impl PartialEq for Int {
    fn eq(&self, other: &Self) -> bool {
        embiggen(*self) == embiggen(*other)
    }
}

impl Eq for Int {}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(embiggen(*self).cmp(&embiggen(*other)))
    }
}

impl Ord for Int {
    fn cmp(&self, other: &Self) -> Ordering {
        embiggen(*self).cmp(&embiggen(*other))
    }
}

fn embiggen(x: Int) -> i128 {
    match x {
        Int::I64(a) => a as i128,
        Int::U64(a) => a as i128,
        Int::I32(a) => a as i128,
        Int::U32(a) => a as i128,
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
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
            Value::Str(x) => write!(buf, "Value::Str(\"{}\")", x),
            // Value::OsStr(x) => write!(buf, "{}", x.to_string_lossy()),
            Value::Bytes(x) => {
                let sb = String::from_utf8_lossy(x);
                let s = sb.as_ref();
                if s.len() > 120 {
                    write!(buf, "Value::Bytes(len {}, \"{}\"...)", s.len(), &s[..120])
                } else {
                    write!(buf, "Value::Bytes(len {}, \"{}\")", s.len(), s)
                }
            }
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

///////////////////////////////////////////////////////////////////////////////
//  OwnValue

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

impl From<&str> for OwnValue {
    fn from(s: &str) -> Self {
        OwnValue::String(s.to_owned())
    }
}

impl From<String> for OwnValue {
    fn from(s: String) -> Self {
        OwnValue::String(s)
    }
}

impl<'a> From<&'a OwnValue> for Value<'a> {
    fn from(ov: &'a OwnValue) -> Self {
        match ov {
            OwnValue::None => Value::None,
            OwnValue::Bool(x) => Value::Bool(*x),
            OwnValue::Int(x) => Value::Int(*x),
            OwnValue::Float(x) => Value::Float(*x),
            OwnValue::String(x) => Value::Str(x.as_str()),
            OwnValue::Bytes(x) => Value::Bytes(x.as_ref()),
        }
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
impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Str(s)
    }
}
