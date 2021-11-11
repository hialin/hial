use crate::{
    base::{
        common::*,
        in_api::{InCell, InGroup},
        rust_api::*,
    },
    guard_ok,
};
use std::{ffi::CStr, os::raw::c_char};

#[repr(C)]
#[derive(Clone, Debug)]
pub enum CResult<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> From<Result<T, E>> for CResult<T, E> {
    fn from(a: Result<T, E>) -> Self {
        match a {
            Ok(x) => CResult::Ok(x),
            Err(x) => CResult::Err(x),
        }
    }
}

impl<T, E> CResult<T, E> {
    pub fn to_res(self) -> Result<T, E> {
        match self {
            CResult::Ok(x) => Result::Ok(x),
            CResult::Err(x) => Result::Err(x),
        }
    }
}

#[no_mangle]
pub extern "C" fn cell_str(s: *const c_char) -> CResult<Cell, HErr> {
    let s = unsafe { CStr::from_ptr(s) };
    let s = guard_ok!(s.to_str(), err =>{
        return CResult::Err(HErr::BadArgument(format!("not an utf8 string")));
    });
    CResult::Ok(Cell::from(s.to_string()))
}

pub extern "C" fn cell_value(ov: OwnedValue) -> Cell {
    Cell::from(ov)
}

pub extern "C" fn interpretation(cell: &Cell) -> &str {
    cell.interpretation()
}

pub extern "C" fn typ(cell: &Cell) -> Res<&str> {
    cell.typ()
}

pub extern "C" fn index(cell: &Cell) -> Res<usize> {
    cell.index()
}

pub extern "C" fn label(cell: &Cell) -> Res<&str> {
    cell.label()
}

pub extern "C" fn value(cell: &Cell) -> Res<Value> {
    cell.value()
}

pub extern "C" fn be(cell: Cell, interp: &str) -> Res<Cell> {
    cell.be(interp)
}

pub extern "C" fn sub(cell: &Cell) -> Res<Group> {
    cell.sub()
}

pub extern "C" fn label_type(group: &Group) -> LabelType {
    group.label_type()
}

pub extern "C" fn get(group: &Group, key: Selector) -> Res<Cell> {
    group.get(key)
}

pub extern "C" fn len(group: &Group) -> usize {
    group.len()
}

pub extern "C" fn at(group: &Group, index: usize) -> Res<Cell> {
    group.at(index)
}
