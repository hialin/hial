use std::borrow::Borrow;
use std::collections::HashMap;
use std::{
    cmp::Ordering,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use linkme::distributed_slice;

use crate::debug;
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
pub struct Domain(Rc<DomainData>);

#[derive(Debug)]
pub struct DomainData {
    file_map: HashMap<PathBuf, Rc<Vec<Res<FileEntry>>>>,
    root_path: PathBuf,
    root_pos: u32,
    origin: Option<XCell>,
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
pub struct CellReader {
    group: Group,
    pos: u32,
}

#[derive(Debug)]
pub struct CellWriter {}

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

impl DomainTrait for Domain {
    type Cell = Cell;

    fn interpretation(&self) -> &str {
        "fs"
    }

    fn root(&self) -> Res<Self::Cell> {
        let files = guard_some!(self.0.file_map.get(self.0.root_path.as_path()), {
            return fault("initial path not found");
        })
        .clone();
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

    fn origin(&self) -> Res<XCell> {
        match &self.0.origin {
            Some(c) => Ok(c.clone()),
            None => nores(),
        }
    }
}

impl SaveTrait for Domain {
    // TODO: add implementation
}

impl CellReaderTrait for CellReader {
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
            fault("invalid attribute index")
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
            fault("invalid attribute index")
        }
    }
}

impl CellWriterTrait for CellWriter {}

impl Cell {
    pub fn from_cell(cell: XCell, _: &str) -> Res<XCell> {
        let path = cell.as_file_path()?;
        let file_cell = from_path(path, Some(cell.clone()))?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(file_cell),
        })
    }

    pub fn from_path(path: impl Borrow<Path>) -> Res<XCell> {
        let file_cell = from_path(path.borrow(), None)?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(file_cell),
        })
    }

    pub fn from_str_path(path: impl Borrow<str>) -> Res<XCell> {
        let file_cell = from_path(Path::new(path.borrow()), None)?.root()?;
        Ok(XCell {
            dyn_cell: DynCell::from(file_cell),
        })
    }

    pub fn as_path(&self) -> Res<&Path> {
        get_path(self)
    }
}

impl CellTrait for Cell {
    type Domain = Domain;
    type Group = Group;
    type CellReader = CellReader;
    type CellWriter = CellWriter;

    fn domain(&self) -> Domain {
        self.group.domain.clone()
    }

    fn ty(&self) -> Res<&str> {
        if self.group.attribute_group_file_pos == u32::MAX {
            let fileentry =
                guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
            let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
            Ok(if md.is_dir { "dir" } else { "file" })
        } else if self.pos == 0 {
            Ok("attribute")
        } else {
            fault("invalid attribute index")
        }
    }

    fn read(&self) -> Res<Self::CellReader> {
        Ok(CellReader {
            group: self.group.clone(),
            pos: self.pos,
        })
    }

    fn write(&self) -> Res<Self::CellWriter> {
        Ok(CellWriter {})
    }

    fn sub(&self) -> Res<Group> {
        if self.group.attribute_group_file_pos == u32::MAX {
            let fileentry =
                guard_ok!(&self.group.files[self.pos  as usize], err => {return Err(err.clone())});
            let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
            if !md.is_dir {
                return nores();
            }
            let mut siblings = vec![];
            read_files(&fileentry.path, &mut siblings)?;
            Ok(Group {
                domain: self.group.domain.clone(),
                files: Rc::new(siblings),
                attribute_group_file_pos: u32::MAX,
            })
        } else {
            nores()
        }
    }

    fn attr(&self) -> Res<Group> {
        if self.group.attribute_group_file_pos == u32::MAX {
            let fileentry =
                guard_ok!(&self.group.files[self.pos as usize], err => {return Err(err.clone())});
            let md = guard_ok!(&fileentry.metadata, err => {return Err(err.clone())});
            if md.is_dir {
                return nores();
            }
            Ok(Group {
                domain: self.group.domain.clone(),
                files: self.group.files.clone(),
                attribute_group_file_pos: self.pos,
            })
        } else {
            nores()
        }
    }

    fn head(&self) -> Res<(Self, Relation)> {
        todo!()
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
        if self.attribute_group_file_pos == u32::MAX {
            Ok(self.files.len())
        } else {
            Ok(1)
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
                nores()
            }
        } else if index < 1 {
            Ok(Cell {
                group: self.clone(),
                pos: index as u32,
            })
        } else {
            nores()
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
            nores()
        } else if key == "size" {
            Ok(Cell {
                group: self.clone(),
                pos: 0,
            })
        } else {
            nores()
        }
    }
}

fn from_path(path: &Path, origin: Option<XCell>) -> Res<Domain> {
    let path = path
        .canonicalize()
        .map_err(|e| caused(HErrKind::IO, "cannot canonicalize path", e))?;
    if !path.exists() {
        debug!("fs: path {:?} does not exist", path);
        return nores();
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
            None => return nores(),
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
            .map_err(|e| caused(HErrKind::IO, "cannot query file metadata", e));
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
        origin,
    })))
}

fn get_path(file: &Cell) -> Res<&Path> {
    let fileentry =
        guard_ok!(&file.group.files[file.pos as usize], err => {return Err(err.clone())});
    Ok(fileentry.path.as_path())
}

fn read_files(path: &Path, entries: &mut Vec<Res<FileEntry>>) -> Res<()> {
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
    let files_iterator = std::fs::read_dir(path).map_err(|e| {
        caused(
            HErrKind::IO,
            format!("cannot read dir: {}", path.to_string_lossy()),
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
    Ok(())
}