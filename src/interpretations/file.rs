use std::collections::HashMap;
use std::{
    cmp::Ordering,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{base::*, guard_ok, guard_some};

#[derive(Clone, Debug)]
pub struct Domain(Rc<DomainData>);

#[derive(Debug)]
pub struct DomainData {
    file_map: HashMap<PathBuf, Rc<Vec<Res<FileEntry>>>>,
    root_path: PathBuf,
    root_pos: u32,
}

#[derive(Clone, Debug)]
pub struct Cell {
    group: Group,
    pos: u32,
}

#[derive(Clone, Debug)]
pub struct Group {
    domain: Domain,
    files: Rc<Vec<Res<FileEntry>>>,
    attribute_group_file_pos: u32,
}

#[derive(Debug)]
pub struct ValueRef {
    group: Group,
    pos: u32,
    is_label: bool,
}

#[derive(Debug)]
pub struct CellReader {
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

impl InDomain for Domain {
    type Cell = Cell;
    type Group = Group;

    fn interpretation(&self) -> &str {
        "file"
    }

    fn new_from(source_interpretation: &str, source: RawDataContainer) -> Res<Self> {
        match source {
            RawDataContainer::File(path) => from_path(path),
            RawDataContainer::String(string) if source_interpretation == "value" => {
                from_path(PathBuf::from(string))
            }
            _ => HErr::IncompatibleSource(format!(
                "cannot make a file from {}",
                source_interpretation
            ))
            .into(),
        }
    }

    fn root(&self) -> Res<Self::Cell> {
        let files = guard_some!(self.0.file_map.get(self.0.root_path.as_path()), {
            return HErr::internal("").into();
        })
        .clone();
        // .get(self.root_path.as_path());
        let group = Group {
            domain: self.clone(),
            files,
            attribute_group_file_pos: u32::MAX,
        };
        Ok(Cell {
            group,
            pos: self.0.root_pos,
        })
    }
}

impl InCellReader for CellReader {
    fn index(&self) -> Res<usize> {
        Ok(self.pos as usize)
    }

    fn label(&self) -> Res<Value> {
        if self.group.attribute_group_file_pos == u32::MAX {
            let fileentry =
                guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
            let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
            Ok(Value::Str(md.name.as_str()))
        } else if self.pos == 0 {
            Ok(Value::Str("size"))
        } else {
            HErr::internal("").into()
        }
    }

    fn value(&self) -> Res<Value> {
        let fpos = if self.group.attribute_group_file_pos == u32::MAX {
            self.pos
        } else {
            self.group.attribute_group_file_pos
        };
        let fileentry =
            guard_ok!(&self.group.files[fpos as usize], err => {return Err(err.clone())});
        let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
        if self.group.attribute_group_file_pos == u32::MAX {
            Ok(Value::Str(md.name.as_str()))
        } else if self.pos == 0 {
            Ok(Value::Int(Int::U64(md.filesize)))
        } else {
            HErr::internal("").into()
        }
    }
}

impl InCell for Cell {
    type Domain = Domain;
    type CellReader = CellReader;

    fn domain(&self) -> &Self::Domain {
        &self.group.domain
    }

    fn typ(&self) -> Res<&str> {
        if self.group.attribute_group_file_pos == u32::MAX {
            let fileentry =
                guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
            let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
            Ok(if md.is_dir { "dir" } else { "file" })
        } else if self.pos == 0 {
            Ok("attribute")
        } else {
            HErr::internal("").into()
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            group: self.group.clone(),
            pos: self.pos,
        })
    }

    fn sub(&self) -> Res<Group> {
        if self.group.attribute_group_file_pos == u32::MAX {
            let fileentry =
                guard_ok!(&self.group.files[self.pos  as usize], err => {return Err(err.clone())});
            let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
            if !md.is_dir {
                return NotFound::NoGroup(format!("file is not a folder")).into();
            }
            let mut siblings = vec![];
            read_files(&fileentry.path, &mut siblings)?;
            Ok(Group {
                domain: self.group.domain.clone(),
                files: Rc::new(siblings),
                attribute_group_file_pos: u32::MAX,
            })
        } else {
            NotFound::NoGroup(format!("FileAttributes")).into()
        }
    }

    fn attr(&self) -> Res<Group> {
        if self.group.attribute_group_file_pos == u32::MAX {
            let fileentry =
                guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
            let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
            if md.is_dir {
                return NotFound::NoGroup(format!("@")).into();
            }
            Ok(Group {
                domain: self.group.domain.clone(),
                files: self.group.files.clone(),
                attribute_group_file_pos: self.pos,
            })
        } else {
            NotFound::NoGroup(format!("FileAttributes@")).into()
        }
    }
}

impl InGroup for Group {
    type Domain = Domain;
    // type SelectIterator = std::vec::IntoIter<Res<Cell>>;

    fn label_type(&self) -> LabelType {
        LabelType {
            is_indexed: false,
            unique_labels: true,
        }
    }

    fn len(&self) -> usize {
        if self.attribute_group_file_pos == u32::MAX {
            self.files.len()
        } else {
            1
        }
    }

    fn at(&self, index: usize) -> Res<Cell> {
        if self.attribute_group_file_pos == u32::MAX {
            if index < self.files.len() {
                Ok(Cell {
                    group: self.clone(),
                    pos: index as u32,
                })
            } else {
                NotFound::NoResult(format!("{}", index)).into()
            }
        } else if index < 1 {
            Ok(Cell {
                group: self.clone(),
                pos: index as u32,
            })
        } else {
            NotFound::NoResult(format!("{}", index)).into()
        }
    }

    fn get<'s, 'a, S: Into<Selector<'a>>>(&'s self, key: S) -> Res<Cell> {
        let key = key.into();
        // verbose!("get by key: {};   group.kind = {:?}", key, self.kind);
        if self.attribute_group_file_pos == u32::MAX {
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
        } else if key == "size" {
            Ok(Cell {
                group: self.clone(),
                pos: 0,
            })
        } else {
            NotFound::NoResult(format!("{}", key)).into()
        }
    }
}

pub fn from_path(path: PathBuf) -> Res<Domain> {
    let path = path.canonicalize()?;
    if !path.exists() {
        return NotFound::NoResult(format!("file not found: {:?}", path)).into();
    }

    let mut siblings = vec![];
    let mut pos = 0;
    if let Some(parent) = path.clone().parent() {
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
        siblings.push(Ok(FileEntry {
            path: path.clone(),
            metadata,
        }));
    }
    let roots = Rc::new(siblings);
    Ok(Domain(Rc::new(DomainData {
        file_map: HashMap::from([(path.clone(), roots.clone())]),
        root_pos: pos as u32,
        root_path: path,
    })))
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
