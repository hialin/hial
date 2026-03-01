use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
};

use linkme::distributed_slice;

use crate::{
    api::{interpretation::*, *},
    implement_try_from_xell,
    utils::ownrc::{OwnRc, ReadRc, WriteRc},
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static VALUE_TO_PATH: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["value", "fs"],
    target_interpretations: &["path"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    path: OwnRc<PathBuf>,
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
    Root,
    Dir,
    Name,
    Ext,
    Stem,
}

#[derive(Debug)]
pub(crate) struct CellReader {
    path: ReadRc<PathBuf>,
    kind: Kind,
    cached_value: OnceCell<String>,
    cached_path: OnceCell<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    path: WriteRc<PathBuf>,
    kind: Kind,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    path: OwnRc<PathBuf>,
}

implement_try_from_xell!(Cell, Path);

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, _: &ElevateParams) -> Res<Xell> {
        match origin.interpretation() {
            "fs" => {
                let r = origin.read();
                let path = r.as_file_path()?;
                Self::make_cell(path.to_owned(), Some(origin))
            }
            _ => {
                let r = origin.read();
                let v = r.value()?;
                let cow = v.as_cow_str();
                let value = cow.as_ref();
                Self::make_cell(PathBuf::from(value), Some(origin))
            }
        }
    }

    fn make_cell(path: PathBuf, origin: Option<Xell>) -> Res<Xell> {
        let path_cell = Cell {
            path: OwnRc::new(path),
            kind: Kind::Root,
        };
        Ok(Xell::new_from(DynCell::from(path_cell), origin))
    }
}

impl CellReader {
    pub(crate) fn as_file_path(&self) -> Res<&Path> {
        if matches!(self.kind, Kind::Root) {
            return Ok(self.path.as_path());
        }

        if self.cached_path.get().is_none() {
            let value = part_value(self.path.as_path(), &self.kind);
            self.cached_path
                .set(PathBuf::from(value))
                .map_err(|_| faulterr("cannot set cached path"))?;
        }

        Ok(self
            .cached_path
            .get()
            .ok_or_else(|| faulterr("cannot read cached path"))?)
    }
}

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        Ok("path")
    }

    fn value(&self) -> Res<Value<'_>> {
        let value = self.cached_value.get_or_init(|| match self.kind {
            Kind::Root => self.path.as_os_str().to_string_lossy().to_string(),
            _ => part_value(self.path.as_path(), &self.kind),
        });

        Ok(Value::Str(value))
    }

    fn label(&self) -> Res<Value<'_>> {
        match self.kind {
            Kind::Root => nores(),
            _ => Ok(Value::Str(part_label(&self.kind))),
        }
    }

    fn index(&self) -> Res<usize> {
        match self.kind {
            Kind::Root => Ok(0),
            Kind::Dir => Ok(0),
            Kind::Name => Ok(1),
            Kind::Ext => Ok(2),
            Kind::Stem => Ok(3),
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        match self.kind {
            Kind::Root => match value {
                OwnValue::String(s) => {
                    *(self.path) = PathBuf::from(s);
                    Ok(())
                }
                _ => userres(format!("cannot set fs path to {:?}", value)),
            },
            Kind::Dir => {
                let dir = value.as_cow_str().to_string();
                let base = self.path.file_name().map(|x| x.to_os_string());
                let mut next = PathBuf::from(dir);
                if let Some(base) = base {
                    next.push(base);
                }
                *(self.path) = next;
                Ok(())
            }
            Kind::Name => {
                let name = value.as_cow_str().to_string();
                if name.is_empty() {
                    return userres("name cannot be empty");
                }
                self.path.set_file_name(name);
                Ok(())
            }
            Kind::Ext => {
                let mut ext = value.as_cow_str().to_string();
                if ext.starts_with('.') {
                    ext = ext.trim_start_matches('.').to_string();
                }
                if self.path.file_name().is_none() {
                    return nores();
                }
                self.path.set_extension(ext);
                Ok(())
            }
            Kind::Stem => {
                let stem = value.as_cow_str().to_string();
                if stem.is_empty() {
                    return userres("stem cannot be empty");
                }
                let ext = self
                    .path
                    .extension()
                    .map(|x| x.to_string_lossy().to_string());
                let mut base = stem;
                if let Some(ext) = ext {
                    if !ext.is_empty() {
                        base.push('.');
                        base.push_str(ext.as_str());
                    }
                }
                self.path.set_file_name(base);
                Ok(())
            }
        }
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "path"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            path: self
                .path
                .read()
                .ok_or_else(|| lockerr("cannot lock path for reading"))?,
            kind: self.kind.clone(),
            cached_value: OnceCell::new(),
            cached_path: OnceCell::new(),
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            path: self
                .path
                .write()
                .ok_or_else(|| lockerr("cannot lock path for writing"))?,
            kind: self.kind.clone(),
        })
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.kind {
            Kind::Root => nores(),
            _ => Ok((
                Cell {
                    path: self.path.clone(),
                    kind: Kind::Root,
                },
                Relation::Sub,
            )),
        }
    }

    fn sub(&self) -> Res<Self::Group> {
        match self.kind {
            Kind::Root => Ok(Group {
                path: self.path.clone(),
            }),
            _ => nores(),
        }
    }

    fn attr(&self) -> Res<Self::Group> {
        nores()
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    type CellIterator = std::iter::Once<Res<Self::Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        Ok(4)
    }

    fn at(&self, index: usize) -> Res<Self::Cell> {
        let kind = match index {
            0 => Kind::Dir,
            1 => Kind::Name,
            2 => Kind::Ext,
            3 => Kind::Stem,
            _ => return nores(),
        };
        Ok(Cell {
            path: self.path.clone(),
            kind,
        })
    }

    fn get_all(&self, label: Value<'_>) -> Res<Self::CellIterator> {
        let kind = match label {
            Value::Str("dir") => Kind::Dir,
            Value::Str("name") => Kind::Name,
            Value::Str("ext") => Kind::Ext,
            Value::Str("stem") => Kind::Stem,
            _ => return nores(),
        };

        Ok(std::iter::once(Ok(Cell {
            path: self.path.clone(),
            kind,
        })))
    }
}

fn part_label(kind: &Kind) -> &'static str {
    match kind {
        Kind::Root => "",
        Kind::Dir => "dir",
        Kind::Name => "name",
        Kind::Ext => "ext",
        Kind::Stem => "stem",
    }
}

fn part_value(path: &Path, kind: &Kind) -> String {
    match kind {
        Kind::Root => path.as_os_str().to_string_lossy().to_string(),
        Kind::Dir => path.parent().map_or_else(String::new, |p| {
            let s = p.as_os_str().to_string_lossy().to_string();
            if s.is_empty() { ".".to_string() } else { s }
        }),
        Kind::Name => path
            .file_name()
            .map_or_else(String::new, |x| x.to_string_lossy().to_string()),
        Kind::Ext => path
            .extension()
            .map_or_else(String::new, |x| format!(".{}", x.to_string_lossy())),
        Kind::Stem => path
            .file_stem()
            .map_or_else(String::new, |x| x.to_string_lossy().to_string()),
    }
}
