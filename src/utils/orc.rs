use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;

// Overridable/owned rc
pub struct Orc<T> {
    ptr: NonNull<OrcBox<T>>,
    phantom: PhantomData<OrcBox<T>>,
}

struct OrcBox<T> {
    pub(self) references: Cell<isize>,
    pub(self) value: T,
}

impl<T> Deref for Orc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner().value
    }
}

impl<T> Orc<T> {
    pub fn new(value: T) -> Orc<T> {
        let box_value = Box::new(OrcBox {
            references: Cell::new(1),
            value,
        });
        Orc {
            ptr: NonNull::from(Box::leak(box_value)),
            phantom: PhantomData,
        }
    }

    pub fn get_mut(x: &mut Self, max_allowed_references: usize) -> Option<&mut T> {
        let x = x.inner_mut();
        if x.references.get() > max_allowed_references as isize {
            None
        } else {
            Some(&mut x.value)
        }
    }

    pub fn count(x: &Self) -> isize {
        let x = x.inner();
        x.references.get()
    }

    #[inline(always)]
    fn inner(&self) -> &OrcBox<T> {
        // this is safe
        unsafe { self.ptr.as_ref() }
    }

    #[inline(always)]
    fn inner_mut(&mut self) -> &mut OrcBox<T> {
        // this is safe
        unsafe { self.ptr.as_mut() }
    }

    #[inline(always)]
    fn add_reference(&self) -> isize {
        let mut references = self.inner().references.get();
        // has at least one from constructor
        if references > 0 {
            references += 1;
            self.inner().references.set(references);
        }
        references
    }

    #[inline(always)]
    fn remove_reference(&self) -> isize {
        let mut references = self.inner().references.get();
        if references > 0 {
            references -= 1;
            self.inner().references.set(references);
        }
        references
    }

    fn to_manual(&self) {
        self.inner().references.set(-1);
    }

    fn free(&self) {
        if self.inner().references.get() == -1 {
            unsafe { Box::from_raw(self.ptr.as_ptr()) };
        }
    }
}

impl<T> Clone for Orc<T> {
    #[inline]
    fn clone(&self) -> Orc<T> {
        self.add_reference();
        Orc {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Orc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> Drop for Orc<T> {
    fn drop(&mut self) {
        let references = self.remove_reference();
        if references == 0 {
            unsafe { Box::from_raw(self.ptr.as_ptr()) };
            // the boxed value is dropped and memory released here
        }
    }
}
