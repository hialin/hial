use crate::base::*;
use std::fmt::Debug;

pub fn str_eval(root: Cell, path: &str) -> Res<Vec<String>> {
    root.path(path)?
        .into_iter()
        .map(|c| -> Res<String> {
            // println!("-- {:?}", c);
            let vref = c.clone()?.value()?;
            match c?.label() {
                Ok(lref) => match lref.get() {
                    Ok(label) => Ok(format!("{}:{}", label, vref.get()?)),
                    Err(HErr::NotFound(_)) => Ok(format!(":{}", vref.get()?)),
                    Err(e) => Err(e),
                },
                Err(HErr::NotFound(_)) => Ok(format!(":{}", vref.get()?)),
                Err(e) => Err(e),
            }
        })
        .collect::<Res<Vec<_>>>()
}

pub fn pr<T: Debug>(x: T) -> T {
    // println!("\npr: {:?}", x);
    x
}
