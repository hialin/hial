use std::{
    cell::Cell,
    fmt,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

// owned rc
pub struct OwnRc<T>(NonNull<RcBox<T>>);

// used rc
pub struct UseRc<T>(NonNull<RcBox<T>>);

struct RcBox<T> {
    pub(self) owners: Cell<isize>,
    pub(self) users: Cell<isize>,
    pub(self) value: T,
}

impl<T> OwnRc<T> {
    pub fn new(value: T) -> OwnRc<T> {
        OwnRc(RcBox::new(value))
    }

    pub fn tap(&self) -> UseRc<T> {
        unsafe {
            self.0.as_ref().inc_urc();
        }
        UseRc(self.0)
    }
}

impl<T> Clone for OwnRc<T> {
    #[inline]
    fn clone(&self) -> OwnRc<T> {
        unsafe {
            self.0.as_ref().inc_orc();
        }
        OwnRc(self.0)
    }
}

impl<T: fmt::Debug> fmt::Debug for OwnRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { write!(f, "Orc({:?})", &self.0.as_ref().value) }
    }
}

impl<T> Drop for OwnRc<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.as_mut().dec_orc();
            if self.0.as_ref().can_be_dropped() {
                drop(Box::from_raw(self.0.as_ptr()));
            }
        }
    }
}

impl<T> UseRc<T> {
    pub fn get_mut(&mut self) -> Option<&mut T> {
        unsafe {
            if self.0.as_ref().users.get() == 1 {
                Some(&mut self.0.as_mut().value)
            } else {
                None
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for UseRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { write!(f, "Urc({:?})", &self.0.as_ref().value) }
    }
}

impl<T> Drop for UseRc<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.as_mut().dec_urc();
            if self.0.as_ref().can_be_dropped() {
                drop(Box::from_raw(self.0.as_ptr()));
            }
        }
    }
}

impl<T> Deref for UseRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.0.as_ref().value }
    }
}

impl<T> DerefMut for UseRc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut self.0.as_mut().value }
    }
}

impl<T> RcBox<T> {
    fn new(value: T) -> NonNull<RcBox<T>> {
        let ptr = Box::new(RcBox {
            owners: Cell::new(1),
            users: Cell::new(0),
            value,
        });
        NonNull::from(Box::leak(ptr))
    }

    fn inc_orc(&self) {
        self.owners.set(self.owners.get() + 1);
    }
    fn inc_urc(&self) {
        self.users.set(self.users.get() + 1);
    }
    fn dec_orc(&mut self) {
        self.owners.set(self.owners.get() - 1);
    }
    fn dec_urc(&mut self) {
        self.users.set(self.users.get() - 1);
    }

    fn can_be_dropped(&self) -> bool {
        self.owners.get() == 0 && self.users.get() == 0
    }
}
