use std::fmt::Debug;

use crate::base::*;

pub fn str_eval(root: Cell, path: &str) -> Res<Vec<String>> {
    root.search(path)?
        .into_iter()
        .map(|cres| -> Res<String> {
            // if let Ok(ref cell) = cres {
            //     if let Ok(path) = cell.path() {
            //         println!("--> found path: {}", path);
            //     }
            // }
            cres.map(|c| c.debug_string())
        })
        .collect::<Res<Vec<_>>>()
}

pub fn pr<T: Debug>(x: T) -> T {
    // println!("\npr: {:?}", x);
    x
}
