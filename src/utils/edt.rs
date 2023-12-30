// enumerated dynamic type

#[macro_export]
macro_rules! enumerated_dynamic_type {
    (
        $(#[ $attr:meta ])?
        $pub:vis
        enum
        $enumname:ident
        {
            $( $subtypename:ident ($subtypetype:path) ,)+
        }
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

        paste::paste! {
            #[allow(unused_macros)]
            #[macro_export]
            macro_rules! [<dispatch_ $enumname:snake>]  {
                ($on:expr, |$argname:ident| $body:block) => {
                    match $on {
                        $($enumname::$subtypename($argname) => $body )*
                    }
                }
            }
        }
    };
}
