use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
};

implement_try_from_xell!(HErr, Error);

impl CellTrait for HErr {
    type Group = HErr;
    type CellReader = HErr;
    type CellWriter = HErr;

    fn interpretation(&self) -> &str {
        "error"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Err(self.clone())
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Err(self.clone())
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }
}

impl CellReaderTrait for HErr {
    fn ty(&self) -> Res<&str> {
        Ok(match self.kind {
            HErrKind::None => "nores",
            HErrKind::User => "user",
            HErrKind::IO => "io",
            HErrKind::Net => "net",
            HErrKind::Internal => "internal",
            HErrKind::ReadOnly => "readonly",
            HErrKind::CannotLock => "cannotlock",
            HErrKind::InvalidFormat => "invalidformat",
        })
    }

    fn value(&self) -> Res<Value> {
        Err(self.clone())
    }

    fn label(&self) -> Res<Value> {
        Err(self.clone())
    }

    fn index(&self) -> Res<usize> {
        Err(self.clone())
    }

    fn serial(&self) -> Res<String> {
        Err(self.clone())
    }
}

impl CellWriterTrait for HErr {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        Err(self.clone())
    }
}

impl GroupTrait for HErr {
    type Cell = HErr;
    type CellIterator = std::iter::Empty<Res<HErr>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(0)
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        Err(self.clone())
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        Err(self.clone())
    }
}
