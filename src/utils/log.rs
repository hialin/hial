pub static mut VERBOSE: bool = false;

#[macro_export]
macro_rules! warning {
    ( $($arg:tt)* ) => ( eprintln!("‣ {}", format!($($arg)*)) );
}

#[macro_export]
macro_rules! debug {
    (
        $($arg:tt)*
    ) => (
        if unsafe{$crate::utils::log::VERBOSE} {
            println!("‣ {}", format!($($arg)*))
        }
    );
}

#[macro_export]
macro_rules! debug_err {
    (
        $arg:expr
    ) => {
        if unsafe { $crate::utils::log::VERBOSE } {
            if !matches!($arg, HErr::None) {
                println!("‣Error: {:?}", $arg)
            }
        }
    };
}

pub fn set_verbose(flag: bool) {
    unsafe { VERBOSE = flag }
}
