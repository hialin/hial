#[macro_export]
macro_rules! guard_ok {
    ($var:expr, $err:ident => $else_block:expr) => {
        match $var {
            Ok(x) => x,
            Err($err) => $else_block,
        }
    };
}

#[macro_export]
macro_rules! guard_some {
    ($var:expr, $else_block:expr) => {
        match $var {
            Some(x) => x,
            None => $else_block,
        }
    };
}
