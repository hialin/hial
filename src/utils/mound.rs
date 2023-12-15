// EXPERIMENT, CURRENTLY UNUSED
// TODO: Remove this file if it is not used in the future
#![allow(soft_unstable)]

use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug)]
pub struct Mound<T: Debug>(Rc<RefCell<InnerMound<T>>>);

#[derive(Debug)]
pub struct MoundRef<T: Debug> {
    mound: Mound<T>,
    index: usize,
}

#[derive(Debug)]
struct InnerMound<T: Debug> {
    data: Vec<Option<T>>,
    empty_slots: Vec<usize>,
}

impl<T: Debug> Mound<T> {
    pub fn new() -> Mound<T> {
        Mound(Rc::new(RefCell::new(InnerMound {
            data: Vec::with_capacity(4),
            empty_slots: Vec::with_capacity(4),
        })))
    }

    #[inline]
    pub fn add(&self, value: T) -> MoundRef<T> {
        let mut mound = self.0.borrow_mut();
        let index = {
            if let Some(idx) = mound.empty_slots.pop() {
                debug_assert!(matches!(mound.data.get(idx), Some(None)));
                mound.data[idx] = Some(value);
                idx
            } else {
                mound.data.push(Some(value));
                mound.data.len() - 1
            }
        };
        MoundRef {
            mound: Mound(self.0.clone()),
            index,
        }
    }

    #[inline]
    fn drop(&self, index: usize) {
        let mut mound = self.0.borrow_mut();
        if let Some(Some(v)) = mound.data.get_mut(index) {
            if index == mound.data.len() - 1 {
                mound.data.pop();
            } else {
                mound.data[index] = None;
                mound.empty_slots.push(index);
            }
        }
    }
}

impl<T: Debug> Drop for MoundRef<T> {
    fn drop(&mut self) {
        self.mound.drop(self.index);
    }
}

#[cfg(test)]
mod speed_test {
    use test::{black_box, Bencher};

    use crate::utils::mound::Mound;

    extern crate test;

    const K: usize = 1000;
    const SZ: usize = 100;

    #[derive(Debug)]
    struct Dum1(usize);

    #[derive(Debug)]
    struct Dum2(usize, usize, usize);

    #[bench]
    fn mound_add(b: &mut Bencher) {
        let mut refs1 = Vec::with_capacity(K * SZ + 1);
        let mound1 = Mound::<Dum1>::new();
        let mut refs2 = Vec::with_capacity(K * SZ + 1);
        let mound2 = Mound::<Dum2>::new();

        b.iter(|| {
            for k in 1..K {
                for i in 1..SZ {
                    let x = Dum1(i);
                    refs1.push(mound1.add(x));
                }
                for i in 1..SZ {
                    if i % 2 == 0 && i < refs1.len() {
                        refs1.swap_remove(i);
                    }
                }
                for i in 1..SZ {
                    let x = Dum2(i, i, i);
                    refs2.push(mound2.add(x));
                }
                for i in 1..SZ {
                    if i % 2 == 0 && i < refs2.len() {
                        refs2.swap_remove(i);
                    }
                }
            }
        });
        black_box(refs1);
        black_box(refs2);
    }

    #[bench]
    fn default_alloc_new(b: &mut Bencher) {
        let mut refs1 = Vec::with_capacity(K * SZ + 1);
        let mut refs2 = Vec::with_capacity(K * SZ + 1);

        b.iter(|| {
            for k in 1..K {
                for i in 1..SZ {
                    let x = Dum1(i);
                    refs1.push(Box::new(x));
                }
                for i in 1..SZ {
                    if i % 2 == 0 && i < refs1.len() {
                        refs1.swap_remove(i);
                    }
                }
                for i in 1..SZ {
                    let x = Dum2(i, i, i);
                    refs2.push(Box::new(x));
                }
                for i in 1..SZ {
                    if i % 2 == 0 && i < refs2.len() {
                        refs2.swap_remove(i);
                    }
                }
            }
        });
        black_box(refs1);
        black_box(refs2);
    }
}
