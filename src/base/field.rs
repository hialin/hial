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

impl DomainTrait for FieldGroup {
    type Cell = FieldCell;

    fn interpretation(&self) -> &str {
        self.cell.interpretation()
    }

    fn root(&self) -> Res<FieldCell> {
        nores()
    }
}

impl SaveTrait for FieldGroup {
    fn write_policy(&self) -> Res<WritePolicy> {
        self.cell.domain()?.write_policy()
    }

    fn set_write_policy(&mut self, policy: WritePolicy) -> Res<()> {
        self.cell.domain()?.set_write_policy(policy)
    }

    fn save(&self, target: SaveTarget) -> Res<()> {
        self.cell.domain()?.save(target)
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
        if ty == FieldType::Label && self.cell.read()?.label() == Err(HErr::None) {
            return nores();
        }
        if ty == FieldType::Value && self.cell.read()?.value() == Err(HErr::None) {
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

impl CellTrait for FieldCell {
    type Domain = FieldGroup;
    type Group = FieldGroup;
    type CellReader = FieldReader;
    type CellWriter = FieldWriter;

    fn domain(&self) -> Res<FieldGroup> {
        nores()
    }

    fn typ(&self) -> Res<&str> {
        Ok("field")
    }

    fn read(&self) -> Res<FieldReader> {
        Ok(FieldReader {
            cell: self.cell.clone(),
            ty: self.ty,
            reader: Box::new(self.cell.read()?),
        })
    }

    fn write(&self) -> Res<FieldWriter> {
        Ok(FieldWriter {
            cell: self.cell.clone(),
            ty: self.ty,
            writer: Box::new(self.cell.write()?),
        })
    }
}

impl CellReaderTrait for FieldReader {
    fn value(&self) -> Res<Value> {
        match self.ty {
            FieldType::Value => self.reader.value(),
            FieldType::Label => self.reader.label(),
            FieldType::Type => Ok(Value::Str(self.cell.typ()?)),
            FieldType::Index => Ok(Value::from(self.reader.index()? as u64)),
        }
    }
}

impl CellWriterTrait for FieldWriter {}
