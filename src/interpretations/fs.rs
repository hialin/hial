use std::borrow::Borrow;
use std::cell::OnceCell;
use std::{
    cmp::Ordering,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use linkme::distributed_slice;

use crate::{
    base::{Cell as XCell, *},
    guard_ok, guard_some,
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static PATH_TO_FILE: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["path"],
    target_interpretations: &["fs"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    files: Rc<Vec<Res<FileEntry>>>,
    ty: GroupType,
}

#[derive(Clone, Debug)]
pub(crate) enum GroupType {
    Folder,
    FileAttributes(u32),
}

#[derive(Debug)]
pub(crate) struct CellReader {
    group: Group,
    pos: u32,
    cached_value: OnceCell<String>,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    group: Group,
    pos: u32,
}

#[derive(Clone, Debug)]
struct FileEntry {
    path: PathBuf,
    metadata: Res<Metadata>,
}

#[derive(Clone, Debug)]
struct Metadata {
    os_name: OsString,
    name: String,
    filesize: u64,
    is_dir: bool,
    is_link: bool,
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos as usize)
    }

    fn label(&self) -> Res<Value> {
        match self.group.ty {
            GroupType::Folder => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                Ok(Value::Str(md.name.as_str()))
            }
            GroupType::FileAttributes(_) => {
                if self.pos != 0 {
                    return fault("invalid attribute index");
                }
                Ok(Value::Str("size"))
            }
        }
    }

    fn value(&self) -> Res<Value> {
        match self.group.ty {
            GroupType::Folder => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if md.is_dir {
                    return nores();
                }
                if self.cached_value.get().is_none() {
                    // TODO: reading file value should return bytes, not string
                    let content = std::fs::read_to_string(&fileentry.path).map_err(|e| {
                        caused(
                            HErrKind::IO,
                            format!("cannot read file: {:?}", fileentry.path),
                            e,
                        )
                    })?;
                    self.cached_value
                        .set(content)
                        .map_err(|_| faulterr("cannot set cached value, it is already set"))?;
                }
                Ok(Value::Str(self.cached_value.get().unwrap()))
            }
            GroupType::FileAttributes(fpos) => {
                let fileentry =
                    guard_ok!(&self.group.files[fpos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if self.pos != 0 {
                    return fault("invalid attribute index");
                }
                Ok(Value::Int(Int::U64(md.filesize)))
            }
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        let string_value = value.to_string();
        match self.group.ty {
            GroupType::Folder => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if md.is_dir {
                    return fault("cannot write to a directory");
                }
                std::fs::write(&fileentry.path, string_value).map_err(|e| {
                    caused(
                        HErrKind::IO,
                        format!("cannot write to file: {:?}", fileentry.path),
                        e,
                    )
                })
            }
            GroupType::FileAttributes(fpos) => {
                let fileentry =
                    guard_ok!(&self.group.files[fpos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if self.pos != 0 {
                    return fault("invalid attribute index");
                }
                let new_len = guard_some!(value.as_value().as_i128(), {
                    return userr("new file size must be an integer");
                });
                if new_len < 0 {
                    return userr("new file size must be positive");
                }
                if new_len > u64::MAX as i128 {
                    return userr("new file size must be smaller than 2^64");
                }

                let file = std::fs::File::options()
                    .write(true)
                    .open(&fileentry.path)
                    .map_err(|e| {
                        caused(
                            HErrKind::IO,
                            format!("cannot open file for writing: {:?}", fileentry.path),
                            e,
                        )
                    })?;
                file.set_len(new_len as u64).map_err(|e| {
                    caused(
                        HErrKind::IO,
                        format!("cannot set file length: {:?}", fileentry.path),
                        e,
                    )
                })
            }
        }
    }

    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        todo!() // add implementation
    }

    fn delete(&mut self) -> Res<()> {
        todo!() // add implementation
    }
}

impl Cell {
    pub(crate) fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        let path = cell.as_file_path()?;
        let file_cell = Cell {
            group: Group {
                files: Rc::new(vec![read_file(path)]),
                ty: GroupType::Folder,
            },
            pos: 0,
        };
        Ok(new_cell(DynCell::from(file_cell), Some(cell)))
    }

    pub(crate) fn from_str_path(path: impl Borrow<str>) -> Res<XCell> {
        let path = Path::new(path.borrow());
        let file_cell = Cell {
            group: Group {
                files: Rc::new(vec![read_file(path)]),
                ty: GroupType::Folder,
            },
            pos: 0,
        };
        Ok(new_cell(DynCell::from(file_cell), None))
    }

    pub(crate) fn as_path(&self) -> Res<&Path> {
        let fileentry =
            guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
        Ok(fileentry.path.as_path())
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "fs"
    }

    fn ty(&self) -> Res<&str> {
        match self.group.ty {
            GroupType::Folder => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                Ok(if md.is_dir { "dir" } else { "file" })
            }
            GroupType::FileAttributes(_) => {
                if self.pos == 0 {
                    Ok("attribute")
                } else {
                    fault("invalid attribute index")
                }
            }
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            group: self.group.clone(),
            pos: self.pos,
            cached_value: OnceCell::new(),
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            group: self.group.clone(),
            pos: self.pos,
        })
    }

    fn sub(&self) -> Res<Group> {
        match self.group.ty {
            GroupType::Folder => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if !md.is_dir {
                    return nores();
                }
                let files = read_files(&fileentry.path)?;
                Ok(Group {
                    files: Rc::new(files),
                    ty: GroupType::Folder,
                })
            }
            GroupType::FileAttributes(_) => nores(),
        }
    }

    fn attr(&self) -> Res<Group> {
        match self.group.ty {
            GroupType::Folder => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if md.is_dir {
                    return nores();
                }
                Ok(Group {
                    files: self.group.files.clone(),
                    ty: GroupType::FileAttributes(self.pos),
                })
            }
            GroupType::FileAttributes(_) => nores(),
        }
    }

    fn head(&self) -> Res<(Self, Relation)> {
        match self.group.ty {
            GroupType::Folder => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                let parent = guard_some!(fileentry.path.parent(), {
                    return nores();
                });
                if parent.as_os_str().is_empty() {
                    return nores();
                }
                let f = read_file(parent)?;
                let cell = Cell {
                    group: Group {
                        files: Rc::new(vec![read_file(parent)]),
                        ty: GroupType::Folder,
                    },
                    pos: 0,
                };
                Ok((cell, Relation::Sub))
            }
            GroupType::FileAttributes(fpos) => {
                let mut group = self.group.clone();
                group.ty = GroupType::Folder;
                let cell = Cell { group, pos: fpos };
                Ok((cell, Relation::Attr))
            }
        }
    }
}

impl GroupTrait for Group {
    type Cell = Cell;
    // type SelectIterator = std::vec::IntoIter<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        match self.ty {
            GroupType::Folder => Ok(self.files.len()),
            GroupType::FileAttributes(_) => Ok(1),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match self.ty {
            GroupType::Folder => {
                if index < self.files.len() {
                    Ok(Cell {
                        group: self.clone(),
                        pos: index as u32,
                    })
                } else {
                    nores()
                }
            }
            GroupType::FileAttributes(_) => {
                if index == 0 {
                    Ok(Cell {
                        group: self.clone(),
                        pos: 0,
                    })
                } else {
                    nores()
                }
            }
        }
    }

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, key: S) -> Res<Cell> {
        let key = key.into();
        // verbose!("get by key: {};   group.kind = {:?}", key, self.kind);
        match self.ty {
            GroupType::Folder => {
                for (pos, f) in self.files.iter().enumerate() {
                    let fileentry = guard_ok!(f, err => {continue});
                    let md = guard_ok!(&fileentry.metadata, err => {continue});
                    if key == md.name.as_str() {
                        return Ok(Cell {
                            group: self.clone(),
                            pos: pos as u32,
                        });
                    }
                }
                nores()
            }
            GroupType::FileAttributes(_) => {
                if key == "size" {
                    Ok(Cell {
                        group: self.clone(),
                        pos: 0,
                    })
                } else {
                    nores()
                }
            }
        }
    }
}

fn read_file(path: &Path) -> Res<FileEntry> {
    let os_name = path
        .file_name()
        .map(|x| x.to_os_string())
        .unwrap_or_default();
    let metadata = fs::metadata(path)
        .map(|xmd| Metadata {
            name: os_name.to_string_lossy().to_string(),
            os_name,
            filesize: xmd.len(),
            is_dir: xmd.is_dir(),
            is_link: xmd.file_type().is_symlink(),
        })
        .map_err(|e| caused(HErrKind::IO, "cannot query file metadata", e));
    Ok(FileEntry {
        path: path.to_path_buf(),
        metadata,
    })
}

fn read_files(parent_path: &Path) -> Res<Vec<Res<FileEntry>>> {
    fn custom_metadata(direntry: &fs::DirEntry, md: &fs::Metadata) -> Metadata {
        let is_link = md.file_type().is_symlink();
        let is_dir = {
            if !is_link {
                md.is_dir()
            } else {
                match fs::metadata(direntry.path()) {
                    Err(_) => false,
                    Ok(xmd) => xmd.is_dir(),
                }
            }
        };
        let os_name = direntry.file_name().to_os_string();
        Metadata {
            name: os_name.to_string_lossy().to_string(),
            os_name,
            filesize: md.len(),
            is_dir,
            is_link,
        }
    }

    // debug!("fs: read children of {:?}", path);
    let mut entries: Vec<Res<FileEntry>> = vec![];
    let files_iterator = std::fs::read_dir(parent_path).map_err(|e| {
        caused(
            HErrKind::IO,
            format!("cannot read dir: {}", parent_path.to_string_lossy()),
            e,
        )
    })?;
    for res_direntry in files_iterator {
        let direntry = guard_ok!(res_direntry, err => {
            entries.push(Err(caused(HErrKind::IO, "cannot read dir entry", err)));
            continue
        });
        let metadata = direntry
            .metadata()
            .map(|md| custom_metadata(&direntry, &md))
            .map_err(|e| {
                caused(
                    HErrKind::IO,
                    format!(
                        "cannot query file metadata: {}",
                        direntry.path().to_string_lossy()
                    ),
                    e,
                )
            });
        entries.push(Ok(FileEntry {
            path: direntry.path(),
            metadata,
        }));
    }
    entries.sort_by(|res1, res2| match (res1, res2) {
        (Ok(m1), Ok(m2)) => match (m1.metadata.as_ref(), m2.metadata.as_ref()) {
            (Ok(md1), Ok(md2)) => md1.name.cmp(&md2.name),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(e1), Err(e2)) => format!("{:?}", e1).cmp(&format!("{:?}", e2)),
        },
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(e1), Err(e2)) => format!("{:?}", e1).cmp(&format!("{:?}", e2)),
    });
    Ok(entries)
}
