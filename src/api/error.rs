use std::{cell::OnceCell, error::Error, fmt, rc::Rc};

use super::{DynCell, Xell};
use crate::warning;

pub type Res<T> = Result<T, HErr>;

#[derive(Clone)]
#[repr(C)]
pub struct HErr {
    pub kind: HErrKind,
    pub data: Rc<HErrData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum HErrKind {
    // item not found
    None,
    // error caused by some parameter controlled by the user
    User,
    // error caused by an IO/fmt operation
    IO,
    // error caused by an net operation
    Net,
    // error caused by some error in the program
    Internal,
    // error caused by trying to write to a read-only data structure
    ReadOnly,
    // cannot change data because there are other readers
    CannotLock,
    // invalid format (e.g. invalid json)
    InvalidFormat,
}

impl fmt::Display for HErrKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HErrKind::None => write!(f, "no result"),
            HErrKind::User => write!(f, "user error"),
            HErrKind::Internal => write!(f, "internal error"),
            HErrKind::IO => write!(f, "io error"),
            HErrKind::Net => write!(f, "net error"),
            HErrKind::ReadOnly => write!(f, "read only"),
            HErrKind::CannotLock => write!(f, "cannot lock"),
            HErrKind::InvalidFormat => write!(f, "invalid format"),
        }
    }
}

#[derive(Debug)]
pub struct HErrData {
    pub msg: String,
    pub xell: OnceCell<Xell>,
    pub cause: Option<Box<dyn Error>>,
    pub backtrace: Option<Box<[String]>>,
}

impl HErr {
    pub(crate) fn with_xell(self, xell: Xell) -> Self {
        if matches!(xell.dyn_cell, DynCell::Error(_)) {
            return self;
        }
        if self.data.xell.set(xell).is_err() {
            warning!("cannot overwrite xell to augment HErr");
        }
        self
    }
}

pub fn noerrm(message: impl Into<String>) -> HErr {
    HErr {
        kind: HErrKind::None,
        data: Rc::new(HErrData {
            msg: message.into(),
            xell: OnceCell::new(),
            cause: None,
            backtrace: Some(capture_stack_trace()),
        }),
    }
}
pub fn noerr() -> HErr {
    noerrm(String::new())
}

pub fn nores<T>() -> Res<T> {
    Err(noerr())
}

pub fn noresm<T>(message: impl Into<String>) -> Res<T> {
    Err(noerrm(message))
}

pub fn usererr(reason: impl Into<String>) -> HErr {
    HErr {
        kind: HErrKind::User,
        data: Rc::new(HErrData {
            msg: reason.into(),
            xell: OnceCell::new(),
            cause: None,
            backtrace: Some(capture_stack_trace()),
        }),
    }
}

pub fn userres<T>(reason: impl Into<String>) -> Res<T> {
    Err(usererr(reason))
}

pub fn fault<T>(reason: impl Into<String>) -> Res<T> {
    Err(faulterr(reason))
}

pub fn faulterr(reason: impl Into<String>) -> HErr {
    let err = HErr {
        kind: HErrKind::Internal,
        data: Rc::new(HErrData {
            msg: reason.into(),
            xell: OnceCell::new(),
            cause: None,
            backtrace: Some(capture_stack_trace()),
        }),
    };

    if cfg!(debug_assertions) {
        println!("{}", err);
        panic!("internal error");
    } else {
        err
    }
}

pub fn caused(kind: HErrKind, reason: impl Into<String>, cause: impl Error + 'static) -> HErr {
    HErr {
        kind,
        data: Rc::new(HErrData {
            msg: reason.into(),
            xell: OnceCell::new(),
            cause: Some(Box::new(cause) as Box<dyn Error>),
            backtrace: Some(capture_stack_trace()),
        }),
    }
}

pub fn deformed(reason: impl Into<String>) -> HErr {
    HErr {
        kind: HErrKind::InvalidFormat,
        data: Rc::new(HErrData {
            msg: reason.into(),
            xell: OnceCell::new(),
            cause: None,
            backtrace: Some(capture_stack_trace()),
        }),
    }
}

pub fn lockerr(reason: impl Into<String>) -> HErr {
    HErr {
        kind: HErrKind::CannotLock,
        data: Rc::new(HErrData {
            msg: reason.into(),
            xell: OnceCell::new(),
            cause: None,
            backtrace: Some(capture_stack_trace()),
        }),
    }
}

impl std::error::Error for HErr {}
impl std::fmt::Display for HErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.data.msg.is_empty() {
            write!(f, "{}: {}", self.kind, self.data.msg)?;
        } else {
            write!(f, "{}", self.kind)?;
        }

        if let Some(cell) = self.data.xell.get()
            && let Ok(path) = cell.path()
        {
            write!(f, " -- cell path: {}", path)?;
        }
        if let Some(ref cause) = self.data.cause {
            write!(f, " -- caused by: {}", cause)?;
        }
        if let Some(ref backtrace) = self.data.backtrace {
            write!(f, "\n{}", backtrace.join("\n"))?;
        }
        Ok(())
    }
}
impl std::fmt::Debug for HErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

pub(crate) fn capture_stack_trace() -> Box<[String]> {
    let backtrace = format!("{}", std::backtrace::Backtrace::capture());
    let mut frames = vec![];
    for l in backtrace.split('\n') {
        let l = l.trim();
        if !l.starts_with("at ") {
            frames.push((l, String::new()));
        } else {
            frames.last_mut().unwrap().1 = l.to_string();
        }
    }

    let accepted: Vec<(String, String)> = frames
        .iter()
        .filter(|(func, point)| !func.contains("error::capture_stack_trace"))
        .filter(|(func, point)| !func.contains(" core::"))
        .filter(|(func, point)| !func.contains(" std::"))
        .filter(|(func, point)| !func.contains(" test::"))
        .filter(|(func, point)| !func.contains(" hiallib::base::error::"))
        // .filter(|(func, point)| !func.contains(" hiallib::base::"))
        .filter(|(func, point)| !func.contains("_pthread_"))
        .map(|(func, point)| (func.to_string(), point.to_string()))
        .collect();

    let columns = accepted
        .iter()
        .map(|(func, _)| func.len())
        .max()
        .unwrap_or(0);

    let lines: Vec<String> = accepted
        .into_iter()
        .map(|(func, point)| format!("    {:columns$} {}", func, point, columns = columns))
        .collect();
    lines.into_boxed_slice()
}
