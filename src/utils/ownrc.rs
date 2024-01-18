use std::{
    cell::Cell,
    fmt,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

// owned rc
pub struct OwnRc<T>(NonNull<RcBox<T>>);

// read rc
pub struct ReadRc<T>(NonNull<RcBox<T>>);

// write rc
pub struct WriteRc<T>(NonNull<RcBox<T>>);

struct RcBox<T> {
    // number of owners; if 0 then the value can be dropped
    pub(self) owners: Cell<usize>,
    // number of readers or -1 if there is a writer
    pub(self) users: Cell<isize>,
    // the value
    pub(self) value: T,
}

impl<T> OwnRc<T> {
    pub fn new(value: T) -> OwnRc<T> {
        OwnRc(RcBox::new(value))
    }

    pub fn read(&self) -> Option<ReadRc<T>> {
        let r = unsafe { self.0.as_ref() };
        if r.inc_readers() {
            Some(ReadRc(self.0))
        } else {
            None
        }
    }

    pub fn write(&self) -> Option<WriteRc<T>> {
        let r = unsafe { self.0.as_ref() };
        if r.set_writer() {
            Some(WriteRc(self.0))
        } else {
            None
        }
    }

    fn drop_if_possible(this: &NonNull<RcBox<T>>) {
        let r = unsafe { this.as_ref() };
        if r.owners.get() == 0 && r.users.get() == 0 {
            unsafe { drop(Box::from_raw(this.as_ptr())) };
        }
    }
}

impl<T> Clone for OwnRc<T> {
    #[inline]
    fn clone(&self) -> OwnRc<T> {
        unsafe { self.0.as_ref() }.inc_owners();
        OwnRc(self.0)
    }
}

impl<T> Drop for OwnRc<T> {
    fn drop(&mut self) {
        unsafe { self.0.as_ref() }.dec_owners();
        Self::drop_if_possible(&self.0);
    }
}

impl<T> Drop for ReadRc<T> {
    fn drop(&mut self) {
        unsafe { self.0.as_ref() }.dec_readers();
        OwnRc::drop_if_possible(&self.0);
    }
}

impl<T> Drop for WriteRc<T> {
    fn drop(&mut self) {
        unsafe { self.0.as_ref() }.unset_writer();
        OwnRc::drop_if_possible(&self.0);
    }
}

impl<T> Deref for ReadRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.0.as_ref().value }
    }
}

impl<T> Deref for WriteRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &self.0.as_ref().value }
    }
}

impl<T> DerefMut for WriteRc<T> {
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

    fn inc_owners(&self) {
        assert!(self.owners.get() > 0);
        self.owners.set(self.owners.get() + 1);
    }
    fn dec_owners(&self) {
        assert!(self.owners.get() > 0);
        self.owners.set(self.owners.get() - 1);
    }

    fn inc_readers(&self) -> bool {
        if self.users.get() < 0 {
            // there is a writer
            return false;
        }
        self.users.set(self.users.get() + 1);
        true
    }

    fn dec_readers(&self) {
        assert!(self.users.get() > 0);
        self.users.set(self.users.get() - 1);
    }

    fn set_writer(&self) -> bool {
        if self.users.get() != 0 {
            // there is a writer or reader(s)
            return false;
        }
        self.users.set(-1);
        true
    }

    fn unset_writer(&self) {
        assert_eq!(self.users.get(), -1);
        self.users.set(0);
    }

    fn can_be_dropped(&self) -> bool {
        self.owners.get() == 0 && self.users.get() == 0
    }
}

impl<T: fmt::Debug> fmt::Debug for OwnRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { write!(f, "OwnRc({:?})", &self.0.as_ref().value) }
    }
}

impl<T: fmt::Debug> fmt::Debug for ReadRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { write!(f, "ReadRc({:?})", &self.0.as_ref().value) }
    }
}

impl<T: fmt::Debug> fmt::Debug for WriteRc<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { write!(f, "WriteRc({:?})", &self.0.as_ref().value) }
    }
}

#[test]
fn test_ownrc() {
    use std::cell::OnceCell;
    let rcptr = OnceCell::new();
    let rc = OwnRc::new(42);
    {
        rcptr.set(rc.0.as_ptr()).unwrap();
        let rc2 = rc.clone();
        assert_eq!(rc2.0.as_ptr(), *rcptr.get().unwrap());
        assert_eq!(unsafe { &**(rcptr.get().unwrap()) }.owners.get(), 2);
        assert_eq!(unsafe { &**(rcptr.get().unwrap()) }.users.get(), 0);

        {
            let reader = rc.read().unwrap();
            let reader2 = rc.read().unwrap();
            assert_eq!(*reader, 42);
            assert_eq!(*reader2, 42);
            assert_eq!(*rc.read().unwrap(), 42);
            assert_eq!(*rc2.read().unwrap(), 42);
        }
        {
            let mut writer = rc.write().unwrap();
            *writer = 4242;
            assert_eq!(*writer, 4242);
            assert!(rc.write().is_none());
            assert!(rc2.write().is_none());
            assert!(rc.read().is_none());
            assert!(rc2.read().is_none());
        }
        {
            let reader = rc.read().unwrap();
            let reader2 = rc.read().unwrap();
            assert_eq!(*reader, 4242);
            assert_eq!(*reader2, 4242);
            assert_eq!(*rc.read().unwrap(), 4242);
            assert_eq!(*rc2.read().unwrap(), 4242);
        }
    }
    assert_eq!(unsafe { &**(rcptr.get().unwrap()) }.owners.get(), 1);
    assert_eq!(unsafe { &**(rcptr.get().unwrap()) }.users.get(), 0);
}
