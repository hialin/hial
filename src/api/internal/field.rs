/// A Field domain is not really a separate domain, but a view on a cell.
/// Most domain and cell operations are forwarded to the underlying cell.
///
use std::{cell::OnceCell, rc::Rc};

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FieldType {
    Value,
    Label,
    Type,
    Index,
    Serial,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub(crate) cell: Rc<Xell>,
}

#[derive(Clone, Debug)]
pub struct Cell {
    pub(crate) cell: Rc<Xell>,
    pub(crate) ty: FieldType,
}

#[derive(Debug)]
pub struct FieldReader {
    pub(crate) cell: Rc<Xell>,
    pub(crate) ty: FieldType,
    pub(crate) reader: Box<CellReader>,
    pub(crate) serial: OnceCell<Res<String>>,
}

#[derive(Debug)]
pub struct FieldWriter {
    pub(crate) cell: Rc<Xell>,
    pub(crate) ty: FieldType,
    pub(crate) writer: Box<CellWriter>,
}

implement_try_from_xell!(Cell, Field);

impl TryFrom<usize> for FieldType {
    type Error = HErr;

    fn try_from(v: usize) -> Result<Self, Self::Error> {
        match v {
            x if x == Self::Value as usize => Ok(Self::Value),
            x if x == Self::Label as usize => Ok(Self::Label),
            x if x == Self::Type as usize => Ok(Self::Type),
            x if x == Self::Index as usize => Ok(Self::Index),
            x if x == Self::Serial as usize => Ok(Self::Serial),
            _ => nores(),
        }
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Once<Res<Self::Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: true,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(5)
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        let ty = FieldType::try_from(index)?;
        // only return the field cell if the field is not empty
        if ty == FieldType::Value
            && self
                .cell
                .read()
                .value()
                .err()
                .map_or(false, |e| e.kind == HErrKind::None)
        {
            return nores();
        }
        if ty == FieldType::Label
            && self
                .cell
                .read()
                .label()
                .err()
                .map_or(false, |e| e.kind == HErrKind::None)
        {
            return nores();
        }
        Ok(Cell {
            cell: self.cell.clone(),
            ty,
        })
    }

    fn get_all(&self, label: Value) -> Res<Self::CellIterator> {
        let cell = if let Value::Str(l) = label {
            match l {
                "value" => self.at(FieldType::Value as usize),
                "label" => self.at(FieldType::Label as usize),
                "type" => self.at(FieldType::Type as usize),
                "index" => self.at(FieldType::Index as usize),
                "serial" => self.at(FieldType::Serial as usize),
                _ => return nores(),
            }
        } else {
            return nores();
        };
        Ok(std::iter::once(cell))
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = FieldReader;
    type CellWriter = FieldWriter;

    fn interpretation(&self) -> &str {
        self.cell.interpretation()
    }

    fn read(&self) -> Res<FieldReader> {
        Ok(FieldReader {
            cell: self.cell.clone(),
            ty: self.ty,
            reader: Box::new(self.cell.read().err()?),
            serial: OnceCell::new(),
        })
    }

    fn write(&self) -> Res<FieldWriter> {
        Ok(FieldWriter {
            cell: self.cell.clone(),
            ty: self.ty,
            writer: Box::new(self.cell.write().err()?),
        })
    }

    fn head(&self) -> Res<(Self, Relation)> {
        // This cannot be implemented, we should return a Xell here but the
        // trait type does not allow us. This is fixed by Xell::head which
        // returns the correct head.
        unimplemented!()
    }
}

impl CellReaderTrait for FieldReader {
    fn ty(&self) -> Res<&str> {
        Ok("field")
    }

    fn value(&self) -> Res<Value> {
        match self.ty {
            FieldType::Value => self.reader.value(),
            FieldType::Label => self.reader.label(),
            FieldType::Type => self.reader.ty().map(Value::Str),
            FieldType::Index => Ok(Value::from(self.reader.index()? as u64)),
            FieldType::Serial => self
                .serial
                .get_or_init(|| self.reader.serial())
                .as_deref()
                .map_err(|e| e.clone())
                .map(Value::Str),
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.ty as usize)
    }

    fn label(&self) -> Res<Value> {
        nores()
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for FieldWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.ty {
            FieldType::Value => self.writer.value(value),
            FieldType::Label => self.writer.label(value),
            FieldType::Type => {
                if let OwnValue::String(t) = value {
                    self.writer.ty(t.as_str())
                } else {
                    userres("set type argument must be a string")
                }
            }
            FieldType::Index => {
                if let OwnValue::Int(i) = value {
                    self.writer.index(i.as_i128() as usize)
                } else {
                    userres("set type argument must be a string")
                }
            }
            FieldType::Serial => self.writer.serial(value),
        }
    }
}
