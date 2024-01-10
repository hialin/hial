use crate::base::*;
use std::{ffi::CStr, os::raw::c_char};

// #[repr(C)]
// #[derive(Clone, Debug)]
// pub enum CResult<T, E> {
//     Ok(T),
//     Err(E),
// }

// impl<T, E> From<Result<T, E>> for CResult<T, E> {
//     fn from(a: Result<T, E>) -> Self {
//         match a {
//             Ok(x) => CResult::Ok(x),
//             Err(x) => CResult::Err(x),
//         }
//     }
// }

// impl<T, E> CResult<T, E> {
//     pub fn to_res(self) -> Result<T, E> {
//         match self {
//             CResult::Ok(x) => Result::Ok(x),
//             CResult::Err(x) => Result::Err(x),
//         }
//     }
// }

#[no_mangle]
pub extern "C" fn cell_from(s: *const c_char) -> Cell {
    match unsafe { CStr::from_ptr(s) }.to_str() {
        Ok(s) => Cell::from(s),
        Err(_) => Cell::from(HErr::User("not an utf8 string".to_string())),
    }
}

#[no_mangle]
pub extern "C" fn interpretation(cell: &Cell) -> &str {
    cell.interpretation()
}

#[no_mangle]
pub extern "C" fn typ(cell: &Cell) -> Res<&str> {
    cell.typ()
}

#[no_mangle]
pub extern "C" fn read(cell: &Cell) -> CellReader {
    cell.read()
}

#[no_mangle]
pub extern "C" fn index(cell_reader: &CellReader) -> Res<usize> {
    cell_reader.index()
}

#[no_mangle]
pub extern "C" fn label(cell_reader: &CellReader) -> Res<Value> {
    cell_reader.label()
}

#[no_mangle]
pub extern "C" fn value(cell_reader: &CellReader) -> Res<Value> {
    cell_reader.value()
}

#[no_mangle]
pub extern "C" fn be(cell: Cell, interp: &str) -> Cell {
    cell.be(interp)
}

#[no_mangle]
pub extern "C" fn sub(cell: &Cell) -> Group {
    cell.sub()
}

#[no_mangle]
pub extern "C" fn label_type(group: &Group) -> LabelType {
    group.label_type()
}

#[no_mangle]
pub extern "C" fn get(group: &Group, key: Selector) -> Cell {
    group.get(key)
}

#[no_mangle]
pub extern "C" fn len(group: &Group) -> Res<usize> {
    group.len()
}

#[no_mangle]
pub extern "C" fn at(group: &Group, index: usize) -> Cell {
    group.at(index)
}
