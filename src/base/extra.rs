use std::{
    fmt::{Debug, Write},
    rc::Rc,
};

use crate::{
    base::*, enumerated_dynamic_type, guard_ok, interpretations::*, pathlang::eval::EvalIter,
    pathlang::Path, warning,
};

const MAX_PATH_ITEMS: usize = 1000;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Cell {
    pub(crate) dyn_cell: DynCell,
    // keep it in a rc to avoid other allocations
    pub(crate) domain: Rc<Domain>,
}

pub(crate) fn new_cell(dyn_cell: DynCell, origin: Option<Cell>) -> Cell {
    Cell {
        dyn_cell,
        domain: Rc::new(Domain { origin }),
    }
}

#[derive(Clone, Debug)]
pub struct Domain {
    origin: Option<Cell>,
}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynCell {
        Error(HErr),
        Field(field::FieldCell),
        OwnValue(ownvalue::Cell),
        File(fs::Cell),
        Json(json::Cell),
        Toml(toml::Cell),
        Yaml(yaml::Cell),
        Xml(xml::Cell),
        Url(url::Cell),
        Path(path::Cell),
        Http(http::Cell),
        TreeSitter(treesitter::Cell),
    }
}

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub(crate) enum DynCellReader {
        Error(HErr),
        Field(field::FieldReader),
        OwnValue(ownvalue::CellReader),
        File(fs::CellReader),
        Json(json::CellReader),
        Toml(toml::CellReader),
        Yaml(yaml::CellReader),
        Xml(xml::CellReader),
        Url(url::CellReader),
        Path(path::CellReader),
        Http(http::CellReader),
        TreeSitter(treesitter::CellReader),
    }
}

#[derive(Debug)]
pub struct CellReader(DynCellReader);

enumerated_dynamic_type! {
    #[derive(Debug)]
    pub(crate) enum DynCellWriter {
        Error(HErr),
        Field(field::FieldWriter),
        OwnValue(ownvalue::CellWriter),
        File(fs::CellWriter),
        Json(json::CellWriter),
        Toml(toml::CellWriter),
        Yaml(yaml::CellWriter),
        Xml(xml::CellWriter),
        Url(url::CellWriter),
        Path(path::CellWriter),
        Http(http::CellWriter),
        TreeSitter(treesitter::Cell),
    }
}

#[derive(Debug)]
pub struct CellWriter(DynCellWriter);

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub enum DynGroup {
        Error(HErr),
        Field(field::FieldGroup),
        OwnValue(VoidGroup<ownvalue::Cell>),
        File(fs::Group),
        Json(json::Group),
        Toml(toml::Group),
        Yaml(yaml::Group),
        Xml(xml::Group),
        Url(VoidGroup<url::Cell>),
        Path(VoidGroup<path::Cell>),
        Http(http::Group),
        TreeSitter(treesitter::Cell),
    }
}

#[derive(Clone, Debug)]
pub enum Group {
    Dyn {
        dyn_group: DynGroup,
        domain: Rc<Domain>,
    },
    Elevation {
        elevation_group: ElevationGroup,
        domain: Rc<Domain>,
    },
    // Mixed(Vec<Cell>),
}

#[derive(Copy, Clone, Debug)]
pub enum WritePolicy {
    // no write access
    ReadOnly,
    // write access, but no automatic save
    ExplicitWrite,
    // write access, automatic save when all references are dropped
    WriteBackOnDrop,
    // write access, automatic save on every change
    WriteThrough,
}

impl From<OwnValue> for Cell {
    fn from(ov: OwnValue) -> Self {
        ownvalue::Cell::from_value(ov).unwrap()
    }
}

impl From<Value<'_>> for Cell {
    fn from(v: Value) -> Self {
        Cell::from(v.to_owned_value())
    }
}

impl From<&str> for Cell {
    fn from(s: &str) -> Self {
        ownvalue::Cell::from_str(s).unwrap()
    }
}

impl From<String> for Cell {
    fn from(s: String) -> Self {
        ownvalue::Cell::from_string(s).unwrap()
    }
}

impl From<Res<Cell>> for Cell {
    fn from(res: Res<Cell>) -> Self {
        match res {
            Ok(cell) => cell,
            Err(e) => Cell::from(e),
        }
    }
}

impl From<HErr> for Cell {
    fn from(herr: HErr) -> Self {
        Cell {
            dyn_cell: DynCell::from(herr),
            domain: Rc::new(Domain { origin: None }),
        }
    }
}

impl CellReaderTrait for CellReader {
    fn index(&self) -> Res<usize> {
        dispatch_dyn_cell_reader!(&self.0, |x| { Ok(x.index()?) })
    }
    fn label(&self) -> Res<Value> {
        dispatch_dyn_cell_reader!(&self.0, |x| { Ok(x.label()?) })
    }
    fn value(&self) -> Res<Value> {
        dispatch_dyn_cell_reader!(&self.0, |x| { Ok(x.value()?) })
    }
}
impl CellReader {
    pub fn err(self) -> Res<CellReader> {
        if let DynCellReader::Error(error) = self.0 {
            return Err(error);
        }
        Ok(self)
    }
}

impl CellWriterTrait for CellWriter {
    fn set_label(&mut self, value: OwnValue) -> Res<()> {
        dispatch_dyn_cell_writer!(&mut self.0, |x| { x.set_label(value) })
    }

    fn set_value(&mut self, ov: OwnValue) -> Res<()> {
        dispatch_dyn_cell_writer!(&mut self.0, |x| { x.set_value(ov) })
    }
}
impl CellWriter {
    pub fn err(self) -> Res<CellWriter> {
        if let DynCellWriter::Error(error) = self.0 {
            return Err(error);
        }
        Ok(self)
    }
}

impl Cell {
    pub fn err(self) -> Res<Cell> {
        if let DynCell::Error(error) = self.dyn_cell {
            return Err(error);
        }
        Ok(self)
    }

    pub fn interpretation(&self) -> &str {
        dispatch_dyn_cell!(&self.dyn_cell, |x| { x.interpretation() })
    }

    pub fn origin(&self) -> Cell {
        self.domain
            .origin
            .clone()
            .ok_or_else(|| noerr().with_path_res(self.path()))
            .unwrap_or_else(Cell::from)
    }

    pub fn ty(&self) -> Res<&str> {
        dispatch_dyn_cell!(&self.dyn_cell, |x| { x.ty() })
    }

    pub fn read(&self) -> CellReader {
        let reader: DynCellReader = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.read() {
                Ok(r) => DynCellReader::from(r),
                Err(e) => DynCellReader::from(e),
            }
        });
        CellReader(reader)
    }

    pub fn policy(&self, policy: WritePolicy) -> Cell {
        // TODO: implement extra::Cell::policy
        self.clone()
    }

    pub fn write(&self) -> CellWriter {
        let writer: DynCellWriter = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.write() {
                Ok(r) => DynCellWriter::from(r),
                Err(e) => DynCellWriter::from(e),
            }
        });
        CellWriter(writer)
    }

    pub fn sub(&self) -> Group {
        let sub = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.sub() {
                Ok(r) => DynGroup::from(r),
                Err(e) => DynGroup::from(e),
            }
        });
        Group::Dyn {
            dyn_group: sub,
            domain: self.domain.clone(),
        }
    }

    pub fn attr(&self) -> Group {
        let attr = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.attr() {
                Ok(r) => DynGroup::from(r),
                Err(e) => DynGroup::from(e),
            }
        });
        Group::Dyn {
            dyn_group: attr,
            domain: self.domain.clone(),
        }
    }

    pub fn top_interpretation(&self) -> Option<&str> {
        if let DynCell::Error(_) = self.dyn_cell {
            return None;
        }
        elevation::top_interpretation(self)
    }

    pub fn elevate(&self) -> Group {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Group::Dyn {
                dyn_group: DynGroup::from(err.clone()),
                domain: self.domain.clone(),
            };
        }
        Group::Elevation {
            elevation_group: ElevationGroup(self.clone()),
            domain: Rc::new(Domain {
                origin: Some(self.clone()),
            }),
        }
    }

    pub fn field(&self) -> Group {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Group::Dyn {
                dyn_group: DynGroup::from(err.clone()),
                domain: self.domain.clone(),
            };
        }
        Group::Dyn {
            dyn_group: DynGroup::from(FieldGroup {
                cell: Rc::new(self.clone()),
            }),
            domain: self.domain.clone(),
        }
    }

    pub fn be(&self, interpretation: &str) -> Cell {
        self.elevate().get(interpretation)
    }

    pub fn to(&self, path: &str) -> Cell {
        let path = guard_ok!(crate::pathlang::Path::parse(path), err =>
            return Cell {
                dyn_cell: DynCell::from(err),
                domain: self.domain.clone(),
            }
        );
        PathSearch::new(self.clone(), path).first()
    }

    pub fn search<'a>(&self, path: &'a str) -> Res<PathSearch<'a>> {
        let path = guard_ok!(crate::pathlang::Path::parse(path), err => {return Err(err)});
        Ok(PathSearch::new(self.clone(), path))
    }

    pub fn head(&self) -> Res<(Cell, Relation)> {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Err(err.clone());
        }
        if let DynCell::Field(field) = &self.dyn_cell {
            return Ok(((*field.cell).clone(), Relation::Field));
        }
        dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.head() {
                Ok((c, r)) => Ok((
                    Cell {
                        dyn_cell: DynCell::from(c),
                        domain: self.domain.clone(),
                    },
                    r,
                )),
                Err(e) => Err(e),
            }
        })
    }

    /// Returns the path of head cells and relations in the current domain.
    /// The current cell is not included. If the path is empty, the current
    /// cell is the domain root. HErrKind::None is never returned.
    fn domain_path_items(&self) -> Res<Vec<(Self, Relation)>> {
        let mut v: Vec<(Self, Relation)> = vec![];

        let mut head = self.head();
        while let Ok(h) = head {
            v.push((h.0.clone(), h.1));
            head = h.0.head();
            if v.len() > MAX_PATH_ITEMS {
                return fault("domain path item iteration limit reached");
            }
        }

        let err = head.unwrap_err();
        if err.kind == HErrKind::None {
            v.reverse();
            Ok(v)
        } else {
            Err(err)
        }
    }

    /// Returns the path of head cells and relations in the current domain.
    /// The current cell is included. HErrKind::None is never returned.
    /// The path is returned as a string of labels separated by slashes.
    fn domain_path(&self) -> Res<String> {
        fn write_index(reader: CellReader, s: &mut String) -> Res<()> {
            match reader.index() {
                Ok(i) => {
                    write!(s, "[{}]", i).map_err(|e| caused(HErrKind::IO, "write error", e))?;
                    Ok(())
                }
                Err(e) => {
                    if e.kind != HErrKind::None {
                        Err(e)
                    } else {
                        Ok(())
                    }
                }
            }
        }

        let mut s = String::new();
        let path_items = self.domain_path_items()?;
        for (i, (c, r)) in path_items.iter().enumerate() {
            let reader = c.read().err()?;
            if i == 0 {
                write!(s, "{}", r).map_err(|e| caused(HErrKind::IO, "write error", e))?;
                continue;
            }
            if i > MAX_PATH_ITEMS {
                return fault("domain path iteration limit reached");
            }

            let lres = reader.label();
            match lres {
                Ok(l) => {
                    if !l.is_empty() {
                        write!(s, "{}{}", l, r)
                            .map_err(|e| caused(HErrKind::IO, "write error", e))?;
                    } else {
                        write_index(reader, &mut s)?;
                        write!(s, "{}", r).map_err(|e| caused(HErrKind::IO, "write error", e))?;
                    }
                }
                Err(e) => {
                    if e.kind == HErrKind::None {
                        write_index(reader, &mut s)?;
                        write!(s, "{}", r).map_err(|e| caused(HErrKind::IO, "write error", e))?;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if !path_items.is_empty() {
            // write current cell's label/index, if it's not the root
            let reader = self.read().err()?;
            let lres = reader.label();
            match lres {
                Ok(l) => {
                    if !l.is_empty() {
                        write!(s, "{}", l).map_err(|e| caused(HErrKind::IO, "write error", e))?;
                    } else {
                        write_index(reader, &mut s)?;
                    }
                }
                Err(e) => {
                    if e.kind == HErrKind::None {
                        write_index(reader, &mut s)?;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Ok(s)
    }

    /// Returns the full path of head cells and relations in and between domains.
    /// The current cell is not included. If the path is empty, the current
    /// cell is the domain root. HErrKind::None is never returned.
    /// The path is returned as a string of labels separated by slashes.
    pub fn path(&self) -> Res<String> {
        let mut v: Vec<String> = Vec::new();
        let mut some_orig = Some(self.clone());
        let mut iteration = 0;
        while let Some(ref cell) = some_orig {
            iteration += 1;
            if iteration > MAX_PATH_ITEMS {
                return fault("path iteration limit reached");
            }

            v.push(cell.domain_path()?);

            let base_str = if cell.domain.origin.is_some() {
                format!("{}{}", Relation::Interpretation, cell.interpretation())
            } else {
                let mut s = format!(
                    "{}",
                    cell.read()
                        .value()
                        .unwrap_or(Value::Str("<💥 cell read error>"))
                )
                .replace('\n', "\\n");
                if s.len() > 16 {
                    s.truncate(16);
                    s += "...";
                }
                format!("`{}`", s)
            };
            v.push(base_str);
            some_orig = cell.domain.origin.clone();
        }
        v.reverse();
        let s = v.join("");
        Ok(s)
    }

    pub fn debug_string(&self) -> String {
        let err_fn = |err| warning!("💥 str write error {}", err);
        let mut s = String::new();
        match self.read().err() {
            Ok(reader) => {
                match reader.label() {
                    Ok(l) => write!(s, "{}", l).unwrap_or_else(err_fn),
                    Err(e) if e.kind == HErrKind::None => {}
                    Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
                };
                write!(s, ":").unwrap_or_else(err_fn);
                match reader.value() {
                    Ok(v) => write!(s, "{}", v).unwrap_or_else(err_fn),
                    Err(e) if e.kind == HErrKind::None => {}
                    Err(e) => write!(s, "<💥{:?}>", e).unwrap_or_else(err_fn),
                };
            }
            Err(e) => {
                write!(s, "<💥cannot read: {:?}>", e).unwrap_or_else(err_fn);
            }
        }
        s
    }

    pub fn as_file_path(&self) -> Res<&std::path::Path> {
        if let DynCell::File(ref file_cell) = self.dyn_cell {
            return file_cell.as_path();
        }
        if let DynCell::Path(ref path_cell) = self.dyn_cell {
            return path_cell.as_path();
        }
        nores().with_path_res(self.path())
    }

    pub fn save(&self, target: Cell) -> Res<()> {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Err(err.clone());
        }
        if let DynCell::Error(err) = &target.dyn_cell {
            return Err(err.clone());
        }
        dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.save(target) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        })
    }
}

#[test]
fn test_cell_domain_path() -> Res<()> {
    let tree = r#"{"a": {"x": "xa", "b": {"x": "xb", "c": {"x": "xc"}}}, "m": [1, 2, 3]}"#;
    let root = Cell::from(tree).be("yaml");

    assert_eq!(root.domain_path()?, r#""#);
    let leaf = root.to("/a/b/c/x");
    assert_eq!(leaf.domain_path()?, r#"/a/b/c/x"#);

    assert_eq!(root.path()?, r#"`{"a": {"x": "xa"...`^yaml"#);

    assert_eq!(leaf.path()?, r#"`{"a": {"x": "xa"...`^yaml/a/b/c/x"#);

    Ok(())
}

impl Group {
    pub fn label_type(&self) -> LabelType {
        match self {
            Group::Dyn { dyn_group, .. } => {
                dispatch_dyn_group!(dyn_group, |x| { x.label_type() })
            }
            Group::Elevation {
                elevation_group, ..
            } => elevation_group.label_type(),
        }
    }

    pub fn len(&self) -> Res<usize> {
        match self {
            Group::Dyn { dyn_group, .. } => {
                dispatch_dyn_group!(dyn_group, |x| { x.len() })
            }
            Group::Elevation {
                elevation_group, ..
            } => elevation_group.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |l| l == 0)
    }

    pub fn at(&self, index: usize) -> Cell {
        match self {
            Group::Dyn { dyn_group, domain } => {
                dispatch_dyn_group!(dyn_group, |x| {
                    Cell {
                        dyn_cell: match x.at(index) {
                            Ok(c) => DynCell::from(c),
                            Err(e) => DynCell::from(e),
                        },
                        domain: domain.clone(),
                    }
                })
            }
            Group::Elevation {
                elevation_group,
                domain,
            } => match elevation_group.at(index) {
                Ok(c) => c,
                Err(e) => Cell {
                    dyn_cell: DynCell::from(e),
                    domain: domain.clone(),
                },
            },
        }
    }

    pub fn get<'a, S: Into<Selector<'a>>>(&self, key: S) -> Cell {
        let key = key.into();
        match self {
            Group::Dyn { dyn_group, domain } => {
                dispatch_dyn_group!(dyn_group, |x| {
                    Cell {
                        dyn_cell: match x.get(key) {
                            Ok(c) => DynCell::from(c),
                            Err(e) => DynCell::from(e),
                        },
                        domain: domain.clone(),
                    }
                })
            }
            Group::Elevation {
                elevation_group,
                domain,
            } => match elevation_group.get(key) {
                Ok(c) => c,
                Err(e) => Cell {
                    dyn_cell: DynCell::from(e),
                    domain: domain.clone(),
                },
            },
        }
    }

    pub fn err(self) -> Res<Group> {
        match self {
            Group::Dyn { dyn_group, domain } => match dyn_group {
                DynGroup::Error(error) => Err(error),
                _ => Ok(Group::Dyn { dyn_group, domain }),
            },
            _ => Ok(self),
        }
    }
}

impl IntoIterator for Group {
    type Item = Cell;
    type IntoIter = GroupIter;

    fn into_iter(self) -> Self::IntoIter {
        GroupIter(self, 0)
    }
}

#[derive(Debug)]
pub struct GroupIter(Group, usize);
impl Iterator for GroupIter {
    type Item = Cell;
    fn next(&mut self) -> Option<Self::Item> {
        if self.1 >= self.0.len().unwrap_or(0) {
            return None;
        }
        self.1 += 1;
        Some(self.0.at(self.1 - 1))
    }
}

#[derive(Clone, Debug)]
pub struct PathSearch<'a> {
    start: Cell,
    eval_iter: EvalIter<'a>,
}
impl<'a> PathSearch<'a> {
    pub fn new(cell: Cell, path: Path<'a>) -> Self {
        PathSearch {
            start: cell.clone(),
            eval_iter: EvalIter::new(cell, path),
        }
    }

    pub fn first(mut self) -> Cell {
        match self.eval_iter.next() {
            Some(Ok(c)) => c,
            Some(Err(e)) => Cell {
                dyn_cell: DynCell::from(e),
                domain: self.start.domain.clone(),
            },
            None => {
                let mut path = self.start.path().unwrap_or_default();
                path += self.eval_iter.unmatched_path().as_str();
                warning!("💥 path search failed: {}", path);
                Cell {
                    dyn_cell: DynCell::from(noerr().with_path(path)),
                    domain: self.start.domain.clone(),
                }
            }
        }
    }

    pub fn all(self) -> Res<Vec<Cell>> {
        self.eval_iter.collect::<Res<Vec<_>>>()
    }
}
