use crate::rust_api::*;
use crate::*;
use std::fmt::Debug;

pub fn str_eval(root: Cell, path: &str) -> Res<Vec<String>> {
    root.path(path)?
        .into_iter()
        .map(|c| {
            Ok(format!(
                "{}:{}",
                c.clone()?.label().unwrap_or(""),
                c?.value()?
            ))
        })
        .collect::<Res<Vec<_>>>()
}

pub fn pr<T: Debug>(x: T) -> T {
    println!("\npr: {:?}", x);
    x
}
