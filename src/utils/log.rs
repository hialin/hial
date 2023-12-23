use std::borrow::Borrow;

pub static mut VERBOSE: bool = false;

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

pub fn set_verbose(flag: bool) {
    unsafe { VERBOSE = flag }
}

pub fn verbose_error(e: impl Borrow<crate::base::HErr>) {
    let e = e.borrow();
    if !matches!(e, crate::base::HErr::None) {
        debug!("Error: {:?}", e)
    }
}
