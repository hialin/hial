pub type Res<T> = Result<T, HErr>;

#[derive(Clone, Debug)]
#[repr(C)]
pub enum HErr {
    Internal(String),
    NotFound(NotFound),
    BadArgument(String),
    BadPath(String),
    BadContext(String),
    IO(std::io::ErrorKind, String),
    IncompatibleSource(String),

    // cannot change data because there are other readers
    ExclusivityRequired(String),

    Json(String),
    Toml(String),
    Yaml(String),
    Xml(String),
    Url(String),
    Http(String),
    Sitter(String),
    Other(String),
}

#[derive(Clone, Debug)]
pub enum NotFound {
    NoLabel, // the cell has no label
    NoIndex, // the cell has no index
    NoGroup(String),
    NoResult(String),
    NoInterpretation(String),
}

impl From<NotFound> for HErr {
    fn from(e: NotFound) -> Self {
        HErr::NotFound(e)
    }
}

impl<T> From<NotFound> for Res<T> {
    fn from(e: NotFound) -> Self {
        Err(HErr::NotFound(e))
    }
}

impl<T> From<HErr> for Res<T> {
    fn from(e: HErr) -> Self {
        Err(e)
    }
}

impl HErr {
    pub fn internal<T: Into<String>>(msg: T) -> HErr {
        if cfg!(debug_assertions) {
            eprintln!("{}", msg.into());
            panic!("internal error");
        } else {
            HErr::Internal(msg.into())
        }
    }
}
