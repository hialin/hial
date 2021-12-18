// Example: guard_ok!(my_thing, err => {return Err("bad my_thing")})
#[macro_export]
macro_rules! guard_ok {
    ($var:expr, $err:ident => $else_block:expr) => {
        match $var {
            Ok(x) => x,
            Err($err) => $else_block,
        }
    };
}

// Example: guard_some!(my_thing, {return Err("bad my_thing")})
#[macro_export]
macro_rules! guard_some {
    ($var:expr, $else_block:expr) => {
        match $var {
            Some(x) => x,
            None => $else_block,
        }
    };
}

// Example: guard_variant!(my_thing, Variant::Alpha(a)})
#[macro_export]
macro_rules! guard_variant {
    ($var:expr, $variant:path) => {
        if let $variant(x) = $var {
            Some(x)
        } else {
            None
        }
    };
}
