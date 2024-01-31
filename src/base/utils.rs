use crate::utils::ownrc::{OwnRc, ReadRc, WriteRc};

use super::{lockerr, Res};

pub fn read<T>(o: &OwnRc<T>) -> Res<ReadRc<T>> {
    o.read().ok_or_else(|| lockerr("cannot read"))
}

pub fn write<T>(o: &OwnRc<T>) -> Res<WriteRc<T>> {
    o.write().ok_or_else(|| lockerr("cannot write"))
}
