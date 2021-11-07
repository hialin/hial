use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct RelPtr<T>(i8, PhantomData<T>);

impl<T> Default for RelPtr<T> {
    fn default() -> Self {
        RelPtr(0, PhantomData::default())
    }
}

impl<T> RelPtr<T> {
    pub fn null() -> RelPtr<T> {
        return RelPtr(0, PhantomData::default());
    }

    pub fn set(&mut self, x: &T) {
        let this = self as *const RelPtr<T> as *const ();
        let delta = unsafe { this.offset_from(x as *const T as *const ()) };
        if delta as i8 as isize != delta {
            panic!("relative pointer too far away")
        }
        self.0 = delta as i8;
    }
    pub unsafe fn get(&self) -> &T {
        if self.0 == 0 {
            panic!("Null RelPtr");
        }
        let this = self as *const RelPtr<T> as *const ();
        let that = this.offset(self.0 as isize) as *const T;
        &*that
    }
    // pub unsafe fn get_mut(&self) -> &mut T {
    //     let this = self as *const RelPtr as *const ();
    //     (self.0 + this) as *mut () as *mut T
    // }
}
