use std::{cell::OnceCell, error::Error, fmt, rc::Rc};

use crate::{base::*, warning};

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
    pub cell_path: OnceCell<String>,
    pub cause: Option<Box<dyn Error>>,
    pub backtrace: Option<Box<[String]>>,
}

impl HErr {
    pub(crate) fn with_path(self, path: impl Into<String>) -> Self {
        let path = path.into();
        if let Err(old_path) = self.data.cell_path.set(path.clone()) {
            warning!(
                "overwrote cell path to augment HErr: {} -> {}",
                old_path,
                path
            );
        }
        self
    }
    pub(crate) fn with_path_res(self, path: impl Into<Res<String>>) -> Self {
        match path.into() {
            Ok(p) => self.with_path(p),
            Err(e) => {
                warning!("cannot get cell path to augment HErr: {}", e);
                self
            }
        }
    }
}

pub trait ResHErrAugmentation {
    fn with_path(self, path: impl Into<String>) -> Self;
    fn with_path_res(self, path: impl Into<Res<String>>) -> Self;
}

impl<T> ResHErrAugmentation for Res<T> {
    fn with_path(self, path: impl Into<String>) -> Self {
        let path = path.into();
        self.map_err(|err| err.with_path(path))
    }

    fn with_path_res(self, path: impl Into<Res<String>>) -> Self {
        let pathres = path.into();
        self.map_err(|err| err.with_path_res(pathres))
    }
}

pub fn noerr() -> HErr {
    HErr {
        kind: HErrKind::None,
        data: Rc::new(HErrData {
            msg: String::new(),
            cell_path: OnceCell::new(),
            cause: None,
            backtrace: Some(capture_stack_trace()),
        }),
    }
}

pub fn nores<T>() -> Res<T> {
    Err(noerr())
}

pub fn usererr(reason: impl Into<String>) -> HErr {
    HErr {
        kind: HErrKind::User,
        data: Rc::new(HErrData {
            msg: reason.into(),
            cell_path: OnceCell::new(),
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
            cell_path: OnceCell::new(),
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
            cell_path: OnceCell::new(),
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
            cell_path: OnceCell::new(),
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
            cell_path: OnceCell::new(),
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

        if let Some(path) = self.data.cell_path.get() {
            write!(f, " -- at cell path: {}", path)?;
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

impl CellReaderTrait for HErr {
    fn ty(&self) -> Res<&str> {
        Ok(match self.kind {
            HErrKind::None => "nores",
            HErrKind::User => "user",
            HErrKind::IO => "io",
            HErrKind::Net => "net",
            HErrKind::Internal => "internal",
            HErrKind::ReadOnly => "readonly",
            HErrKind::CannotLock => "cannotlock",
            HErrKind::InvalidFormat => "invalidformat",
        })
    }

    fn value(&self) -> Res<Value> {
        Err(self.clone())
    }

    fn label(&self) -> Res<Value> {
        Err(self.clone())
    }

    fn index(&self) -> Res<usize> {
        Err(self.clone())
    }

    fn serial(&self) -> Res<String> {
        Err(self.clone())
    }
}

impl CellWriterTrait for HErr {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        Err(self.clone())
    }
}

impl CellTrait for HErr {
    type Group = HErr;
    type CellReader = HErr;
    type CellWriter = HErr;

    fn interpretation(&self) -> &str {
        "error"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Err(self.clone())
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Err(self.clone())
    }

    fn head(&self) -> Res<(Self, Relation)> {
        nores()
    }
}

impl GroupTrait for HErr {
    type Cell = HErr;
    type CellIterator = std::iter::Empty<Res<HErr>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(0)
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        Err(self.clone())
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        Err(self.clone())
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
