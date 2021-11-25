// enumerated dynamic type

#[macro_export]
macro_rules! pub_enumerated_dynamic_type {
    (
        enum
        $enumname:ident
        {
            $( $subtypename:ident ($subtypetype:path) ,)+
        }
        $macroname:ident

    ) => {

        #[derive(Clone, Debug)]
        pub enum $enumname {
            $($subtypename($subtypetype),)+
        }

        $(
        impl From<$subtypetype> for $enumname {
            fn from(x: $subtypetype) -> Self {
                $enumname::$subtypename(x)
            }
        }
        )+

        macro_rules! $macroname {
            ($on:expr, |$argname:ident| $body:block) => {
                match $on {
                    $($enumname::$subtypename($argname) => $body )*
                }
            }
        }

    };
}
