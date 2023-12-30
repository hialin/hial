use core::fmt::Display;
use std::fmt::Formatter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
#[allow(clippy::char_lit_as_u8)]
pub enum Relation {
    Attr = '@' as u8,
    Sub = '/' as u8,
    Interpretation = '^' as u8,
    Field = '#' as u8,
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
