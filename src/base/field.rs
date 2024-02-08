/// A Field domain is not really a separate domain, but a view on a cell.
/// Most domain and cell operations are forwarded to the underlying cell.
///
use std::{cell::OnceCell, rc::Rc};

use crate::base::*;

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
pub struct FieldGroup {
    pub(crate) cell: Rc<Cell>,
}

#[derive(Clone, Debug)]
pub struct FieldCell {
    pub(crate) cell: Rc<Cell>,
    pub(crate) ty: FieldType,
}

#[derive(Debug)]
pub struct FieldReader {
    pub(crate) cell: Rc<Cell>,
    pub(crate) ty: FieldType,
    pub(crate) reader: Box<CellReader>,
    pub(crate) serial: OnceCell<Res<String>>,
}

#[derive(Debug)]
pub struct FieldWriter {
    pub(crate) cell: Rc<Cell>,
    pub(crate) ty: FieldType,
    pub(crate) writer: Box<CellWriter>,
}

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

impl GroupTrait for FieldGroup {
    type Cell = FieldCell;

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
        Ok(FieldCell {
            cell: self.cell.clone(),
            ty,
        })
    }

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::Cell> {
        let label = label.into();
        if let Selector::Str(l) = label {
            return match l {
                "value" => self.at(FieldType::Value as usize),
                "label" => self.at(FieldType::Label as usize),
                "type" => self.at(FieldType::Type as usize),
                "index" => self.at(FieldType::Index as usize),
                "serial" => self.at(FieldType::Serial as usize),
                _ => nores(),
            };
        }
        nores()
    }
}

impl CellTrait for FieldCell {
    type Group = FieldGroup;
    type CellReader = FieldReader;
    type CellWriter = FieldWriter;

    fn interpretation(&self) -> &str {
        self.cell.interpretation()
    }

    fn ty(&self) -> Res<&str> {
        Ok("field")
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
        // This cannot be implemented, we should return a XCell here but the
        // trait type does not allow us. This is fixed by extra::Cell which
        // returns the correct head.
        unimplemented!()
    }
}

impl CellReaderTrait for FieldReader {
    fn value(&self) -> Res<Value> {
        match self.ty {
            FieldType::Value => self.reader.value(),
            FieldType::Label => self.reader.label(),
            FieldType::Type => Ok(Value::Str(self.cell.ty()?)),
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
            FieldType::Value => self.writer.set_value(value),
            FieldType::Label => self.writer.set_label(value),
            FieldType::Type => userres("cannot change cell type"),
            FieldType::Index => self.writer.set_index(value),
            FieldType::Serial => self.writer.set_serial(value),
        }
    }
}
