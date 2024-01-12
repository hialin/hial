use std::io;

use crate::base::*;

pub type Res<T> = Result<T, HErr>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum HErr {
    None,
    User(String),
    Internal(String),

    IO(std::io::ErrorKind, String),
    ReadOnly(String),

    // cannot change data because there are other readers
    ExclusivityRequired { path: String, op: &'static str },

    Json(String),
    Toml(String),
    Yaml(String),
    Xml(String),
    Url(String),
    Http(String),
    Sitter(String),
    Other(String),

    NotYetImplemented,
}

fn print_stack_trace() {
    let s = format!("{}", std::backtrace::Backtrace::capture())
        .split('\n')
        .filter(|s| s.contains("hiallib::") || s.contains("./src"))
        .fold(String::new(), |mut acc, s| {
            acc.push_str(s);
            acc.push('\n');
            acc
        });
    println!("{}", s);
}

pub fn nores<T>() -> Res<T> {
    // print_stack_trace();
    Err(HErr::None)
}

pub fn unimplemented<T>() -> Res<T> {
    print_stack_trace();
    Err(HErr::NotYetImplemented)
}

pub fn userr<T>(reason: impl Into<String>) -> Res<T> {
    Err(HErr::User(reason.into()))
}

pub fn fault<T>(reason: impl Into<String>) -> Res<T> {
    if cfg!(debug_assertions) {
        eprintln!("{}", reason.into());
        panic!("internal error");
    } else {
        Err(HErr::Internal(reason.into()))
    }
}

impl std::error::Error for HErr {}
impl std::fmt::Display for HErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HErr::None => write!(f, "no result"),
            HErr::NotYetImplemented => write!(f, "not yet implemented"),
            HErr::User(msg) => write!(f, "user error: {}", msg),
            HErr::Internal(msg) => write!(f, "internal error: {}", msg),
            HErr::IO(kind, msg) => write!(f, "io error: {:?}: {}", kind, msg),
            HErr::ReadOnly(msg) => write!(f, "read only: {}", msg),
            HErr::ExclusivityRequired { path, op } => {
                write!(f, "exclusivity required for {}: {}", path, op)
            }
            HErr::Json(msg) => write!(f, "json error: {}", msg),
            HErr::Toml(msg) => write!(f, "toml error: {}", msg),
            HErr::Yaml(msg) => write!(f, "yaml error: {}", msg),
            HErr::Xml(msg) => write!(f, "xml error: {}", msg),
            HErr::Url(msg) => write!(f, "url error: {}", msg),
            HErr::Http(msg) => write!(f, "http error: {}", msg),
            HErr::Sitter(msg) => write!(f, "sitter error: {}", msg),
            HErr::Other(msg) => write!(f, "other error: {}", msg),
        }
    }
}

impl From<io::Error> for HErr {
    fn from(e: io::Error) -> HErr {
        HErr::IO(e.kind(), format!("{}", e))
    }
}

impl<T> From<HErr> for Res<T> {
    fn from(e: HErr) -> Self {
        Err(e)
    }
}

impl DomainTrait for HErr {
    type Cell = HErr;

    fn interpretation(&self) -> &str {
        "error"
    }

    fn root(&self) -> Res<Self::Cell> {
        Err(self.clone())
    }

    fn origin(&self) -> Res<super::extra::Cell> {
        Err(self.clone())
    }
}

impl SaveTrait for HErr {
    // TODO: add implementation
}

impl CellReaderTrait for HErr {
    fn value(&self) -> Res<Value> {
        Err(self.clone())
    }
}

impl CellWriterTrait for HErr {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        Err(self.clone())
    }
}

impl CellTrait for HErr {
    type Domain = HErr;
    type Group = HErr;
    type CellReader = HErr;
    type CellWriter = HErr;

    fn domain(&self) -> HErr {
        self.clone()
    }

    fn typ(&self) -> Res<&str> {
        Ok("error")
    }

    fn read(&self) -> Res<Self::CellReader> {
        Err(self.clone())
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Err(self.clone())
    }
}

impl GroupTrait for HErr {
    type Cell = HErr;

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

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, label: S) -> Res<Self::Cell> {
        Err(self.clone())
    }
}
