use std::{
    borrow::Cow,
    cell::OnceCell,
    cmp::Ordering,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use indexmap::{indexmap, IndexMap};
use linkme::distributed_slice;

use crate::{
    api::{interpretation::*, *},
    guard_ok, guard_some, implement_try_from_xell,
    utils::ownrc::{OwnRc, ReadRc, WriteRc},
};

#[distributed_slice(ELEVATION_CONSTRUCTORS)]
static PATH_TO_FILE: ElevationConstructor = ElevationConstructor {
    source_interpretations: &["path", "fs"],
    target_interpretations: &["fs"],
    constructor: Cell::from_cell,
};

#[derive(Clone, Debug)]
pub(crate) struct Cell {
    group: Group,
    pos: u32,
}

#[derive(Debug)]
pub(crate) struct CellReader {
    files: ReadRc<FileList>,
    ty: GroupType,
    pos: u32,
    cached_value: OnceCell<Box<[u8]>>,
}

#[derive(Debug)]
pub(crate) struct CellWriter {
    files: WriteRc<FileList>,
    ty: GroupType,
    pos: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct Group {
    files: OwnRc<FileList>,
    ty: GroupType,
}

#[derive(Clone, Debug)]
pub(crate) struct FileList {
    list: IndexMap<String, Res<FileEntry>>,
    full: bool, // if true, all files from parent folder are present in the list
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum GroupType {
    Folder,
    // the u32 is the index of the file in the group
    FileAttributes(u32),
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

implement_try_from_xell!(Cell, File);

impl CellReaderTrait for CellReader {
    fn ty(&self) -> Res<&str> {
        let fe = self.fileentry()?;
        let md = fe.metadata.as_ref().map_err(|e| e.clone())?;

        match self.ty {
            GroupType::Folder => Ok(if md.is_dir { "dir" } else { "file" }),
            GroupType::FileAttributes(_) => {
                if self.pos == 0 {
                    Ok("attribute")
                } else {
                    fault("invalid attribute index")
                }
            }
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos as usize)
    }

    fn label(&self) -> Res<Value> {
        match self.ty {
            GroupType::Folder => {
                let fe = self.fileentry()?;
                let md = fe.metadata.as_ref().map_err(|e| e.clone())?;
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
        let fe = self.fileentry()?;
        let md = fe.metadata.as_ref().map_err(|e| e.clone())?;

        match self.ty {
            GroupType::Folder => {
                if md.is_dir {
                    return nores();
                }
                if self.cached_value.get().is_none() {
                    // TODO: reading file value should return bytes, not string
                    let content = std::fs::read(&fe.path).map_err(|e| {
                        caused(HErrKind::IO, format!("cannot read file: {:?}", fe.path), e)
                    })?;
                    // let content = String::from_utf8_lossy(&content);
                    self.cached_value
                        .set(content.into_boxed_slice())
                        .map_err(|_| faulterr("cannot set cached value, it is already set"))?;
                }
                // Ok(Value::Str(self.cached_value.get().unwrap()))
                Ok(Value::Bytes(self.cached_value.get().unwrap()))
            }
            GroupType::FileAttributes(fpos) => {
                if self.pos != 0 {
                    return fault("invalid attribute index");
                }
                Ok(Value::from(md.filesize))
            }
        }
    }

    fn serial(&self) -> Res<String> {
        nores()
    }
}

impl CellReader {
    pub(crate) fn as_file_path(&self) -> Res<&Path> {
        let fe = self.fileentry()?;
        Ok(fe.path.as_path())
    }

    fn fileentry(&self) -> Res<&FileEntry> {
        self.files
            .list
            .get_index(match self.ty {
                GroupType::Folder => self.pos as usize,
                GroupType::FileAttributes(fpos) => fpos as usize,
            })
            .ok_or_else(|| faulterr("file index out of bounds"))?
            .1
            .as_ref()
            .map_err(|e| e.clone())
    }
}

impl CellWriterTrait for CellWriter {
    fn set_value(&mut self, value: OwnValue) -> Res<()> {
        let string_value = value.to_string();
        let fe = self.fileentry()?;
        let md = fe.metadata.as_ref().map_err(|e| e.clone())?;
        match self.ty {
            GroupType::Folder => {
                if md.is_dir {
                    return fault("cannot write to a directory");
                }
                std::fs::write(&fe.path, string_value).map_err(|e| {
                    caused(
                        HErrKind::IO,
                        format!("cannot write to file: {:?}", fe.path),
                        e,
                    )
                })
            }
            GroupType::FileAttributes(fpos) => {
                if self.pos != 0 {
                    return fault("invalid attribute index");
                }
                let new_len = guard_some!(value.as_value().as_i128(), {
                    return userres("new file size must be an integer");
                });
                if new_len < 0 {
                    return userres("new file size must be positive");
                }
                if new_len > u64::MAX as i128 {
                    return userres("new file size must be smaller than 2^64");
                }

                let file = std::fs::File::options()
                    .write(true)
                    .open(&fe.path)
                    .map_err(|e| {
                        caused(
                            HErrKind::IO,
                            format!("cannot open file for writing: {:?}", fe.path),
                            e,
                        )
                    })?;
                file.set_len(new_len as u64).map_err(|e| {
                    caused(
                        HErrKind::IO,
                        format!("cannot set file length: {:?}", fe.path),
                        e,
                    )
                })
            }
        }
    }
}

impl CellWriter {
    fn fileentry(&self) -> Res<&FileEntry> {
        self.files
            .list
            .get_index(match self.ty {
                GroupType::Folder => self.pos as usize,
                GroupType::FileAttributes(fpos) => fpos as usize,
            })
            .ok_or_else(|| faulterr("file index out of bounds"))?
            .1
            .as_ref()
            .map_err(|e| e.clone())
    }
}

impl Cell {
    pub(crate) fn from_cell(origin: Xell, _: &str, params: &ElevateParams) -> Res<Xell> {
        let r = origin.read();
        let path = r.as_file_path()?;
        let path = Self::shell_tilde(path);
        let file_cell = Self::make_file_cell(path.as_ref())?;
        Ok(Xell::new_from(DynCell::from(file_cell), Some(origin)))
    }

    fn shell_tilde(path: &Path) -> Cow<Path> {
        if path.starts_with("~") {
            let home = dirs::home_dir().unwrap_or_default();
            home.join(path.strip_prefix("~").unwrap_or(path)).into()
        } else {
            path.to_path_buf().into()
        }
    }

    fn make_file_cell(path: &Path) -> Res<Cell> {
        let indexmap = [read_file(path)]
            .into_iter()
            .map(|fileres| {
                let fe = fileres?;
                let md = fe.metadata.as_ref().map_err(|e| e.clone())?;
                Ok((md.name.clone(), Ok(fe)))
            })
            .collect::<Res<IndexMap<String, Res<FileEntry>>>>()?;
        Ok(Cell {
            group: Group {
                files: OwnRc::new(FileList {
                    list: indexmap,
                    full: true,
                }),
                ty: GroupType::Folder,
            },
            pos: 0,
        })
    }
}

impl CellTrait for Cell {
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn interpretation(&self) -> &str {
        "fs"
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            files: self
                .group
                .files
                .read()
                .ok_or_else(|| lockerr("cannot read files"))?,
            ty: self.group.ty,
            pos: self.pos,
            cached_value: OnceCell::new(),
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {
            files: self
                .group
                .files
                .write()
                .ok_or_else(|| lockerr("cannot write files"))?,
            ty: self.group.ty,
            pos: self.pos,
        })
    }

    fn sub(&self) -> Res<Group> {
        match self.group.ty {
            GroupType::Folder => {
                let r = self
                    .group
                    .files
                    .read()
                    .ok_or_else(|| lockerr("cannot read files to subs"))?;
                let fe = r
                    .list
                    .get_index(self.pos as usize)
                    .ok_or(noerr())?
                    .1
                    .as_ref()
                    .map_err(|e| e.clone())?;
                let md = fe.metadata.as_ref().map_err(|e| e.clone())?;
                if !md.is_dir {
                    return nores();
                }
                let files = read_files(&fe.path)?;
                Ok(Group {
                    files: OwnRc::new(FileList {
                        list: files
                            .into_iter()
                            .map(|fileres| {
                                let fe = fileres?;
                                let md = fe.metadata.as_ref().map_err(|e| e.clone())?;
                                Ok((md.name.clone(), Ok(fe)))
                            })
                            .collect::<Res<IndexMap<String, Res<FileEntry>>>>()?,
                        full: true,
                    }),
                    ty: GroupType::Folder,
                })
            }
            GroupType::FileAttributes(_) => nores(),
        }
    }

    fn attr(&self) -> Res<Group> {
        match self.group.ty {
            GroupType::Folder => {
                let r = self
                    .group
                    .files
                    .read()
                    .ok_or_else(|| lockerr("cannot read files to subs"))?;
                let fe = r
                    .list
                    .get_index(self.pos as usize)
                    .ok_or(noerr())?
                    .1
                    .as_ref()
                    .map_err(|e| e.clone())?;
                let md = fe.metadata.as_ref().map_err(|e| e.clone())?;
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
                let r = self
                    .group
                    .files
                    .read()
                    .ok_or_else(|| lockerr("cannot read files to subs"))?;
                let fe = r
                    .list
                    .get_index(self.pos as usize)
                    .ok_or(noerr())?
                    .1
                    .as_ref()
                    .map_err(|e| e.clone())?;
                let md = fe.metadata.as_ref().map_err(|e| e.clone())?;
                let parent = guard_some!(fe.path.parent(), {
                    return nores();
                });
                if parent.as_os_str().is_empty() {
                    return nores();
                }
                let f = read_file(parent)?;
                let indexmap = indexmap! {
                    f.metadata.as_ref().map_err(|e| e.clone())?.name.clone() => Ok(f),
                };
                let cell = Cell {
                    group: Group {
                        files: OwnRc::new(FileList {
                            list: indexmap,
                            full: true,
                        }),
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
    type CellIterator = std::iter::Once<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    fn len(&self) -> Res<usize> {
        match self.ty {
            GroupType::Folder => Ok(self
                .files
                .read()
                .ok_or_else(|| lockerr("cannot read files"))?
                .list
                .len()),
            GroupType::FileAttributes(_) => Ok(1),
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match self.ty {
            GroupType::Folder => {
                if index < self.len()? {
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

    fn get_all(&self, key: Value) -> Res<Self::CellIterator> {
        let cell = match self.ty {
            GroupType::Folder => {
                let files = self
                    .files
                    .read()
                    .ok_or_else(|| lockerr("cannot read files"))?;
                let pos = match key {
                    Value::Str(key) => files.list.get_full(key).ok_or(noerr())?.0,
                    _ => return nores(),
                };
                Ok(Cell {
                    group: self.clone(),
                    pos: pos as u32,
                })
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
        };
        Ok(std::iter::once(cell))
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
        .map_err(|e| {
            caused(
                HErrKind::IO,
                format!("cannot query file metadata of {}", path.to_string_lossy()),
                e,
            )
        });
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
