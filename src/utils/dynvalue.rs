// dynamic value

use std::ops::Deref;

// how to use:
trait Tr {
    fn fn_a() -> usize;
}

pub struct X {
    data: [u8; 8 * 8],
}

impl X {
    pub fn new<T: Tr>(x: T) -> X {
        let mut value = X { data: [0; 64] };
        let src = &x as *const T as *const u8;
        let dst = &mut value.data as *mut u8;
        let size = std::mem::size_of::<T>();
        assert(size <= std::mem::size_of_val(&value.data));
        unsafe {
            std::ptr::copy_nonoverlapping(src, dst, size);
            std::mem::forget(x);
        }
        value
    }
}

impl<'a, Trait: ?Sized + 'a> Deref for ThinRef<'a, Trait> {
    type Target = Trait;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let VTableData { offset, vtable } = **self.ptr;
            let p = (self.ptr as *const _ as *const u8).offset(-offset) as *const ();
            internal::TransmuterTO::<Trait> {
                to: internal::TraitObject { data: p, vtable },
            }
            .ptr
        }
    }
}

#[doc(hidden)]
pub mod internal {
    /// Internal struct used by the macro generated code
    /// Copy of core::raw::TraitObject since it is unstable
    #[doc(hidden)]
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct TraitObject {
        pub data: *const (),
        pub vtable: *const (),
    }

    /// Internal struct used by the macro generated code
    #[doc(hidden)]
    pub union TransmuterTO<'a, T: ?Sized + 'a> {
        pub ptr: &'a T,
        pub to: TraitObject,
    }
}
///////////////////////////////////////////////////////////////////////////////////////////////////

#[macro_export]
macro_rules! dyn_value {
    (
        $(#[ $attr:meta ])?
        $pub:vis
        enum
        $enumname:ident
        {
            $( $subtypename:ident ($subtypetype:path) ,)+
        }
        $macroname:ident

    ) => {

        $(#[$attr])?
        $pub enum $enumname {
            $($subtypename($subtypetype),)+
        }

        $(
        impl From<$subtypetype> for $enumname {
            fn from(x: $subtypetype) -> Self {
                $enumname::$subtypename(x)
            }
        }
        )+

        #[allow(unused_macros)]
        #[macro_export]
        macro_rules! $macroname {
            ($on:expr, |$argname:ident| $body:block) => {
                match $on {
                    $($enumname::$subtypename($argname) => $body )*
                }
            }
        }

    };
}
