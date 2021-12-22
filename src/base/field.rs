use std::borrow::Cow;

use crate::base::*;
use crate::guard_ok;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FieldType {
    Value,
    Label,
    Type,
    Index,
}

#[derive(Clone, Debug)]
pub struct Field(pub(crate) Box<Cell>, pub(crate) FieldType); // todo remove this boxing

#[derive(Debug)]
pub enum ValueRef {
    // todo remove this boxing
    ValueRef(Box<extra::ValueRef>),
    // todo remove this boxing
    Field(Box<Cell>, FieldType),
    Label(FieldType),
}

impl ValueRef {
    pub fn get(&self) -> Res<Value> {
        match self {
            ValueRef::ValueRef(vref) => vref.get(),
            ValueRef::Field(cell, fieldtype) => match fieldtype {
                FieldType::Value => HErr::internal("unexpected value").into(),
                FieldType::Label => HErr::internal("unexpected label").into(),
                FieldType::Type => Ok(Value::Str(cell.typ()?)),
                FieldType::Index => Ok(Value::Int(Int::U64(cell.index()? as u64))),
            },
            ValueRef::Label(fieldtype) => match fieldtype {
                FieldType::Value => Ok(Value::Str("value")),
                FieldType::Label => Ok(Value::Str("label")),
                FieldType::Type => Ok(Value::Str("type")),
                FieldType::Index => Ok(Value::Str("index")),
            },
        }
    }

    pub fn with<T>(&self, f: impl Fn(Res<Value>) -> T) -> T {
        f(self.get())
    }
}

impl Field {
    pub fn domain(&self) -> Domain {
        self.0.domain()
    }

    pub fn typ(&self) -> Res<&str> {
        Ok("field")
    }

    pub fn index(&self) -> Res<usize> {
        Ok(self.1 as u8 as usize)
    }

    pub fn label(&self) -> Res<ValueRef> {
        Ok(ValueRef::Label(self.1))
    }

    pub fn value(&self) -> Res<ValueRef> {
        if self.1 == FieldType::Value {
            Ok(ValueRef::ValueRef(Box::new(self.0.value()?)))
        } else if self.1 == FieldType::Label {
            Ok(ValueRef::ValueRef(Box::new(self.0.label()?)))
        } else {
            Ok(ValueRef::Field(Box::new(self.0.as_ref().clone()), self.1))
        }
    }

    pub fn sub(&self) -> Res<Field> {
        NotFound::NoGroup(format!("/")).into()
    }

    pub fn attr(&self) -> Res<Field> {
        NotFound::NoGroup(format!("@")).into()
    }

    pub fn as_data_source(&self) -> Option<Res<DataSource>> {
        match self.1 {
            FieldType::Value => {
                let value = guard_ok!(self.0.value(), err => {return Some(Err(err))});
                if let Ok(Value::Str(s)) = value.get() {
                    Some(Ok(DataSource::String(Cow::from(s.to_string()))))
                } else {
                    None
                }
            }
            FieldType::Label => {
                let value = guard_ok!(self.0.label(), err => {return Some(Err(err))});
                if let Ok(Value::Str(s)) = value.get() {
                    Some(Ok(DataSource::String(Cow::from(s.to_string()))))
                } else {
                    None
                }
            }
            FieldType::Type => {
                let value = guard_ok!(self.0.typ(), err => {return Some(Err(err))});
                Some(Ok(DataSource::String(Cow::from(value))))
            }
            FieldType::Index => None,
        }
    }

    pub fn as_data_destination(&mut self) -> Option<Res<DataDestination>> {
        todo!()
    }

    pub fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    pub fn len(&self) -> usize {
        4
    }

    pub fn at(&self, index: usize) -> Res<Field> {
        match index {
            0 => Ok(Field(self.0.clone(), FieldType::Value)),
            1 => Ok(Field(self.0.clone(), FieldType::Label)),
            2 => Ok(Field(self.0.clone(), FieldType::Type)),
            3 => Ok(Field(self.0.clone(), FieldType::Index)),
            _ => Err(HErr::BadArgument(format!(
                "field index must be in 0..<4; was: {}",
                index
            ))),
        }
    }

    pub fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, key: S) -> Res<Field> {
        let key = key.into();
        if let Selector::Str(key) = key {
            if key == "value" {
                self.0.value()?.get()?;
                return Ok(Field(self.0.clone(), FieldType::Value));
            } else if key == "label" {
                self.0.label()?.get()?;
                return Ok(Field(self.0.clone(), FieldType::Label));
            } else if key == "type" {
                self.0.typ()?;
                return Ok(Field(self.0.clone(), FieldType::Type));
            } else if key == "index" {
                self.0.index()?;
                return Ok(Field(self.0.clone(), FieldType::Index));
            }
        }
        Err(HErr::BadArgument(format!(
            "field keys must be one of [value, label, type, index]; was: {}",
            key
        )))
    }

    pub fn set_value(&mut self, ov: OwnedValue) -> Res<()> {
        match self.1 {
            FieldType::Value => self.0.set_value(ov),
            FieldType::Label => self.0.set_label(ov),
            FieldType::Type => todo!(),
            FieldType::Index => todo!(),
        }
    }
    pub fn set_label(&mut self, ov: OwnedValue) -> Res<()> {
        todo!()
    }
}
