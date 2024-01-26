/// A Field domain is not really a separate domain, but a view on a cell.
/// Most domain and cell operations are forwarded to the underlying cell.
///
use std::rc::Rc;

use crate::base::*;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FieldType {
    Value,
    Label,
    Type,
    Index,
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
}

#[derive(Debug)]
pub struct FieldWriter {
    pub(crate) cell: Rc<Cell>,
    pub(crate) ty: FieldType,
    pub(crate) writer: Box<CellWriter>,
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
        Ok(4)
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        let ty = match index {
            0 => FieldType::Value,
            1 => FieldType::Label,
            2 => FieldType::Type,
            3 => FieldType::Index,
            _ => return nores(),
        };
        // only return the field cell if the field is not empty
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
        Ok(FieldCell {
            cell: self.cell.clone(),
            ty,
        })
    }

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::Cell> {
        let label = label.into();
        if let Selector::Str(l) = label {
            return match l {
                "value" => self.at(0),
                "label" => self.at(1),
                "type" => self.at(2),
                "index" => self.at(3),
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

    fn save(&self, target: extra::Cell) -> Res<()> {
        todo!()
        // not sure if this is the right way to do it:
        // self.cell.save(target)
    }
}

impl CellReaderTrait for FieldReader {
    fn value(&self) -> Res<Value> {
        match self.ty {
            FieldType::Value => self.reader.value(),
            FieldType::Label => self.reader.label(),
            FieldType::Type => Ok(Value::Str(self.cell.ty()?)),
            FieldType::Index => Ok(Value::from(self.reader.index()? as u64)),
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.ty as usize)
    }

    fn label(&self) -> Res<Value> {
        nores()
        // match self.ty {
        //     FieldType::Value => Ok(Value::Str("value")),
        //     FieldType::Label => Ok(Value::Str("label")),
        //     FieldType::Type => Ok(Value::Str("type")),
        //     FieldType::Index => Ok(Value::Str("index")),
        // }
    }
}

impl CellWriterTrait for FieldWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        todo!() // remove this default implementation
    }
}
