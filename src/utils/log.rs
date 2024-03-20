use std::sync::atomic::{AtomicBool, Ordering};

pub static VERBOSE: AtomicBool = AtomicBool::new(false);

#[macro_export]
macro_rules! warning {
    ( $($arg:tt)* ) => ( eprintln!("⚠ {}", format!($($arg)*)) );
}

#[macro_export]
macro_rules! debug {
    (
        $($arg:tt)*
    ) => (
        if $crate::utils::log::VERBOSE.load(std::sync::atomic::Ordering::SeqCst) {
            println!("‣ {}", format!($($arg)*))
        }
    );
}

#[macro_export]
macro_rules! debug_err {
    (
        $arg:expr
    ) => {
        if $crate::utils::log::VERBOSE.load(std::sync::atomic::Ordering::SeqCst) {
            if $arg.kind != HErrKind::None {
                println!("‣Error: {:?}", $arg)
            }
        }
    };
}

pub fn set_verbose(flag: bool) {
    VERBOSE.store(flag, Ordering::SeqCst);
}
