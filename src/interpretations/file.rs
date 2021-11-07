use crate::{
    base::common::*,
    base::interpretation_api::{InterpretationCell, InterpretationGroup},
    guard_ok,
};
use std::{
    cmp::Ordering,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: u32,
}

#[derive(Clone, Debug)]
pub struct Group {
    files: Rc<Vec<Res<FileEntry>>>,
    kind: GroupKind,
}

#[derive(Clone, Debug)]
pub enum GroupKind {
    Files,
    AttributesOfFile(u32),
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

pub fn from_string_path(path: &str) -> Res<Cell> {
    from_path(PathBuf::from(path))
}

fn from_path(path: PathBuf) -> Res<Cell> {
    let path = path.canonicalize()?;
    if !path.exists() {
        return NotFound::NoResult(format!("file not found: {:?}", path)).into();
    }

    let mut siblings = vec![];
    let mut pos = 0;
    if let Some(parent) = path.parent() {
        read_files(parent, &mut siblings)?;
        let pos_res = siblings
            .iter()
            .position(|f| f.is_ok() && f.as_ref().unwrap().path == path);
        pos = match pos_res {
            Some(pos) => pos,
            None => {
                return NotFound::NoResult(format!("file removed concurrently: {:?}", path)).into();
            }
        };
    } else {
        let os_name = path
            .file_name()
            .map(|x| x.to_os_string())
            .unwrap_or(OsString::new());
        let metadata = fs::metadata(path.clone())
            .map(|xmd| Metadata {
                name: os_name.to_string_lossy().to_string(),
                os_name,
                filesize: xmd.len(),
                is_dir: xmd.is_dir(),
                is_link: xmd.file_type().is_symlink(),
            })
            .map_err(HErr::from);
        siblings.push(Ok(FileEntry { path, metadata }));
    }
    let group = Group {
        files: Rc::new(siblings),
        kind: GroupKind::Files,
    };
    let cell = Cell {
        group,
        pos: pos as u32,
    };
    Ok(cell)
}

impl InterpretationCell for Cell {
    type Group = Group;

    fn typ(&self) -> Res<&str> {
        match self.group.kind {
            GroupKind::Files => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                Ok(if md.is_dir { "dir" } else { "file" })
            }
            GroupKind::AttributesOfFile(fpos) => {
                if self.pos == 0 {
                    Ok("attribute")
                } else {
                    HErr::internal("").into()
                }
            }
        }
    }

    fn index(&self) -> Res<usize> {
        Ok(self.pos as usize)
    }

    fn label(&self) -> Res<&str> {
        match self.group.kind {
            GroupKind::Files => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                Ok(md.name.as_str())
            }
            GroupKind::AttributesOfFile(_) => {
                if self.pos == 0 {
                    Ok("size")
                } else {
                    HErr::internal("").into()
                }
            }
        }
    }

    fn value(&self) -> Res<Value> {
        let fpos = if let GroupKind::AttributesOfFile(fpos) = self.group.kind {
            fpos
        } else {
            self.pos
        };
        let fileentry =
            guard_ok!(&self.group.files[fpos as usize], err => {return Err(err.clone())});
        let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
        match self.group.kind {
            GroupKind::Files => Ok(Value::Str(md.name.as_str())),
            GroupKind::AttributesOfFile(_) => {
                if self.pos == 0 {
                    Ok(Value::Int(Int::U64(md.filesize)))
                } else {
                    HErr::internal("").into()
                }
            }
        }
    }

    fn sub(&self) -> Res<Group> {
        match self.group.kind {
            GroupKind::Files => {
                let fileentry = guard_ok!(&self.group.files[self.pos  as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if !md.is_dir {
                    return NotFound::NoGroup(format!("file is not a folder")).into();
                }
                let mut siblings = vec![];
                read_files(&fileentry.path, &mut siblings)?;
                Ok(Group {
                    files: Rc::new(siblings),
                    kind: GroupKind::Files,
                })
            }
            GroupKind::AttributesOfFile(_) => NotFound::NoGroup(format!("FileAttributes")).into(),
        }
    }

    fn attr(&self) -> Res<Group> {
        match self.group.kind {
            GroupKind::Files => {
                let fileentry = guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
                let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
                if md.is_dir {
                    return NotFound::NoGroup(format!("@")).into();
                }
                Ok(Group {
                    files: self.group.files.clone(),
                    kind: GroupKind::AttributesOfFile(self.pos),
                })
            }
            GroupKind::AttributesOfFile(_) => NotFound::NoGroup(format!("FileAttributes@")).into(),
        }
    }
}

impl InterpretationGroup for Group {
    type Cell = Cell;
    // type SelectIterator = std::vec::IntoIter<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    fn len(&self) -> usize {
        match self.kind {
            GroupKind::Files => self.files.len(),
            GroupKind::AttributesOfFile(_) => 1,
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        match self.kind {
            GroupKind::Files => {
                if index < self.files.len() {
                    Ok(Cell {
                        group: self.clone(),
                        pos: index as u32,
                    })
                } else {
                    NotFound::NoResult(format!("{}", index)).into()
                }
            }
            GroupKind::AttributesOfFile(_) => {
                if index < 1 {
                    Ok(Cell {
                        group: self.clone(),
                        pos: index as u32,
                    })
                } else {
                    NotFound::NoResult(format!("{}", index)).into()
                }
            }
        }
    }

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, key: S) -> Res<Self::Cell> {
        let key = key.into();
        // verbose!("get by key: {};   group.kind = {:?}", key, self.kind);
        match self.kind {
            GroupKind::Files => {
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
                NotFound::NoResult(format!("{}", key)).into()
            }
            GroupKind::AttributesOfFile(_) => {
                if key == "size" {
                    Ok(Cell {
                        group: self.clone(),
                        pos: 0,
                    })
                } else {
                    NotFound::NoResult(format!("{}", key)).into()
                }
            }
        }
    }
    //
    // fn get_all<'s, 'a, S: Into<Selector<'a>>>(&'s self, key: S) -> Res<Self::SelectIterator> {
    //     match self.get(key) {
    //         Ok(cell) => {
    //             let v = vec![Ok(cell)];
    //             Ok(v.into_iter())
    //         }
    //         Err(HErr::NotFound(_)) => Ok(Vec::new().into_iter()),
    //         Err(err) => Err(err),
    //     }
    // }
}

pub fn get_path(file: &Cell) -> Res<&Path> {
    let fileentry =
        guard_ok!(&file.group.files[file.pos as usize], err => {return Err(err.clone())});
    Ok(fileentry.path.as_path())
}

fn read_files(path: &Path, entries: &mut Vec<Res<FileEntry>>) -> Res<()> {
    // verbose!("file: read children of {:?}", path);
    let files_iterator = std::fs::read_dir(path)?;
    for res_direntry in files_iterator {
        let direntry = guard_ok!(res_direntry, err => {
            entries.push(Err(HErr::from(err)));
            continue
        });
        let metadata = direntry
            .metadata()
            .map(|md| {
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
            })
            .map_err(HErr::from);
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
            (Err(HErr::IO(ek1, es1)), Err(HErr::IO(ek2, es2))) => ek1.cmp(&ek2),
            (Err(e1), Err(e2)) => format!("{:?}", e1).cmp(&format!("{:?}", e2)),
        },
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(HErr::IO(ek1, es1)), Err(HErr::IO(ek2, es2))) => ek1.cmp(&ek2),
        (Err(e1), Err(e2)) => format!("{:?}", e1).cmp(&format!("{:?}", e2)),
    });
    Ok(())
}

impl From<io::Error> for HErr {
    fn from(e: io::Error) -> HErr {
        HErr::IO(e.kind(), format!("{}", e))
    }
}
