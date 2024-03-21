use std::{
    cell::{self, OnceCell},
    fmt::{self, Debug, Write},
    rc::Rc,
};

use crate::{
    api::{internal::*, interpretation::*, *},
    enumerated_dynamic_type, guard_ok, guard_some,
    interpretations::*,
    prog::{searcher::Searcher, Path},
    warning,
};

const MAX_PATH_ITEMS: usize = 1000;

#[repr(C)]
#[derive(Clone)]
pub struct Xell {
    pub(crate) dyn_cell: DynCell,
    // keep it in a rc to avoid other allocations
    domain: Rc<Domain>,
}

pub struct Domain {
    write_policy: cell::Cell<WritePolicy>,
    origin: Option<Xell>,
    pub(super) dyn_root: OnceCell<DynCell>,
    dirty: cell::Cell<bool>,
}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynCell {
        Error(HErr),
        Elevation(elevation::Cell),
        Field(field::Cell),
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
        Regex(regex::Cell),
    }
}

enumerated_dynamic_type! {
    #[derive(Debug)]
    enum DynCellReader {
        Error(HErr),
        Elevation(elevation::CellReader),
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
        Regex(regex::CellReader),
    }
}

#[derive(Debug)]
pub struct CellReader(DynCellReader);

enumerated_dynamic_type! {
    #[derive(Debug)]
    enum DynCellWriter {
        Error(HErr),
        Elevation(elevation::Cell),
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
        Regex(regex::CellWriter),
    }
}

#[derive(Debug)]
pub struct CellWriter {
    dyn_cell_writer: DynCellWriter,
    // keep it in a rc to avoid other allocations
    domain: Rc<Domain>,
}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynGroup {
        Error(HErr),
        Elevation(elevation::Group),
        Field(field::Group),
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
        Regex(regex::Group),
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    dyn_group: DynGroup,
    domain: Rc<Domain>,
}

#[derive(Clone, Debug)]
pub struct CellIterator {
    cell_iterator: CellIteratorKind,
}

#[derive(Clone, Debug)]
pub(crate) enum CellIteratorKind {
    DynCellIterator {
        dyn_cell: DynCellIterator,
        domain: Rc<Domain>,
    },
    Elevation(Res<Xell>),
}

enumerated_dynamic_type! {
    #[derive(Clone, Debug)]
    pub(crate) enum DynCellIterator {
        Error(std::iter::Once<Res<HErr>>),
        Elevation(std::iter::Empty<Res<elevation::Cell>>),
        Field(std::iter::Once<Res<field::Cell>>),
        OwnValue(std::iter::Empty<Res<ownvalue::Cell>>),
        File(std::iter::Once<Res<fs::Cell>>),
        Json(std::iter::Once<Res<json::Cell>>),
        Toml(std::iter::Once<Res<toml::Cell>>),
        Yaml(std::iter::Once<Res<yaml::Cell>>),
        Xml(xml::CellIterator),
        Url(std::iter::Empty<Res<url::Cell>>),
        Path(std::iter::Empty<Res<path::Cell>>),
        Http(std::iter::Once<Res<http::Cell>>),
        TreeSitter(std::iter::Once<Res<treesitter::Cell>>),
        Regex(std::iter::Empty<Res<regex::Cell>>),
        // None is in addition to interpretation variants, used when nothing else matches
        None(std::iter::Empty<Res<HErr>>),
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WritePolicy {
    // no write access
    ReadOnly,
    // write access, but user must call save manually
    NoAutoWrite,
    // write access, automatic save when all references are dropped
    WriteBackOnDrop,
}

impl From<OwnValue> for Xell {
    fn from(ov: OwnValue) -> Self {
        ownvalue::Cell::from_value(ov).unwrap()
    }
}

impl From<Value<'_>> for Xell {
    fn from(v: Value) -> Self {
        Xell::from(v.to_owned_value())
    }
}

impl From<&str> for Xell {
    fn from(s: &str) -> Self {
        ownvalue::Cell::from_str(s).unwrap()
    }
}

impl From<String> for Xell {
    fn from(s: String) -> Self {
        ownvalue::Cell::from_string(s).unwrap()
    }
}

impl From<Res<Xell>> for Xell {
    fn from(res: Res<Xell>) -> Self {
        match res {
            Ok(cell) => cell,
            Err(e) => Xell::from(e),
        }
    }
}

impl From<HErr> for Xell {
    fn from(herr: HErr) -> Self {
        Xell {
            dyn_cell: DynCell::from(herr),
            domain: Rc::new(Domain {
                write_policy: cell::Cell::new(WritePolicy::ReadOnly),
                origin: None,
                dyn_root: OnceCell::new(),
                dirty: cell::Cell::new(false),
            }),
        }
    }
}

impl CellReader {
    pub fn ty(&self) -> Res<&str> {
        dispatch_dyn_cell_reader!(&self.0, |x| { x.ty() })
    }
    pub fn index(&self) -> Res<usize> {
        dispatch_dyn_cell_reader!(&self.0, |x| { x.index() })
    }
    pub fn label(&self) -> Res<Value> {
        dispatch_dyn_cell_reader!(&self.0, |x| { x.label() })
    }
    pub fn value(&self) -> Res<Value> {
        dispatch_dyn_cell_reader!(&self.0, |x| { x.value() })
    }
    pub fn serial(&self) -> Res<String> {
        dispatch_dyn_cell_reader!(&self.0, |x| { x.serial() })
    }
}
impl CellReader {
    pub fn as_file_path(&self) -> Res<&std::path::Path> {
        if let DynCellReader::File(ref file_cell) = self.0 {
            return file_cell.as_file_path();
        }
        if let DynCellReader::Path(ref path_cell) = self.0 {
            return path_cell.as_file_path();
        }
        userres("this interpretation has no file path")
    }

    pub fn err(self) -> Res<CellReader> {
        if let DynCellReader::Error(error) = self.0 {
            return Err(error);
        }
        Ok(self)
    }
}

impl CellWriter {
    pub fn label(&mut self, value: impl Into<OwnValue>) -> Res<()> {
        let value = value.into();
        self.domain.dirty.set(true);
        dispatch_dyn_cell_writer!(&mut self.dyn_cell_writer, |x| { x.set_label(value) })
    }

    pub fn value(&mut self, value: impl Into<OwnValue>) -> Res<()> {
        let value = value.into();
        self.domain.dirty.set(true);
        dispatch_dyn_cell_writer!(&mut self.dyn_cell_writer, |x| { x.set_value(value) })
    }

    pub fn index(&mut self, index: usize) -> Res<()> {
        self.domain.dirty.set(true);
        dispatch_dyn_cell_writer!(&mut self.dyn_cell_writer, |x| { x.set_index(index) })
    }

    pub fn ty(&mut self, ty: &str) -> Res<()> {
        self.domain.dirty.set(true);
        dispatch_dyn_cell_writer!(&mut self.dyn_cell_writer, |x| { x.set_ty(ty) })
    }

    pub fn serial(&mut self, serial: OwnValue) -> Res<()> {
        self.domain.dirty.set(true);
        dispatch_dyn_cell_writer!(&mut self.dyn_cell_writer, |x| { x.set_serial(serial) })
    }
}
impl CellWriter {
    pub fn err(self) -> Res<CellWriter> {
        if let DynCellWriter::Error(error) = self.dyn_cell_writer {
            return Err(error);
        }
        Ok(self)
    }
}

impl Xell {
    pub fn new(path_with_start: &str) -> Xell {
        match Self::try_new(path_with_start) {
            Ok(c) => c,
            Err(e) => Xell::from(e),
        }
    }

    pub fn try_new(path_with_start: &str) -> Res<Xell> {
        let (start, path) = Path::parse_with_starter(path_with_start)?;
        let root = start.eval()?;
        let mut searcher = Searcher::new(root, path);
        let path = format!("{}{}", start, searcher.unmatched_path().as_str());
        match searcher.next() {
            Some(Ok(c)) => Ok(c),
            Some(Err(e)) => Err(e.with_path(path)),
            None => nores().with_path(path),
        }
    }

    pub(crate) fn new_from(dyn_cell: DynCell, origin: Option<Xell>) -> Xell {
        let wp = origin
            .as_ref()
            .map_or(WritePolicy::ReadOnly, |c| c.domain.write_policy.get());
        Xell {
            dyn_cell,
            domain: Rc::new(Domain {
                write_policy: cell::Cell::new(wp),
                origin,
                dyn_root: OnceCell::new(),
                dirty: cell::Cell::new(false),
            }),
        }
    }

    pub(super) fn set_self_as_domain_root(&self) {
        if let Err(x) = self.domain.dyn_root.set(self.dyn_cell.clone()) {
            warning!("overwriting domain root, old root was: {:?}", x);
        }
    }

    pub fn err(self) -> Res<Xell> {
        if let DynCell::Error(ref error) = self.dyn_cell {
            return Err(error.clone());
        }
        Ok(self)
    }

    pub fn interpretation(&self) -> &str {
        dispatch_dyn_cell!(&self.dyn_cell, |x| { x.interpretation() })
    }

    pub fn origin(&self) -> Xell {
        self.domain
            .origin
            .clone()
            .ok_or_else(|| noerr().with_path_res(self.path()))
            .unwrap_or_else(Xell::from)
    }

    pub fn read(&self) -> CellReader {
        let reader: DynCellReader = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.read() {
                Ok(r) => DynCellReader::from(r),
                Err(e) => DynCellReader::from(e.with_path(self.path().unwrap_or_default())),
            }
        });
        CellReader(reader)
    }

    pub fn policy(&self, policy: WritePolicy) -> Xell {
        self.domain.write_policy.set(policy);
        self.clone()
    }

    pub fn write(&self) -> CellWriter {
        if self.domain.write_policy.get() == WritePolicy::ReadOnly
            // allow writing elevation cells, to set elevation parameters
            && !matches!(self.dyn_cell, DynCell::Elevation(_))
        {
            let err = usererr("cannot write, read-only domain")
                .with_path(self.path().unwrap_or_default());
            return CellWriter {
                dyn_cell_writer: DynCellWriter::from(err),
                domain: Rc::clone(&self.domain),
            };
        }

        let writer: DynCellWriter = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.write() {
                Ok(r) => DynCellWriter::from(r),
                Err(e) => DynCellWriter::from(e.with_path(self.path().unwrap_or_default())),
            }
        });
        CellWriter {
            dyn_cell_writer: writer,
            domain: Rc::clone(&self.domain),
        }
    }

    pub fn sub(&self) -> Group {
        let sub = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.sub() {
                Ok(r) => DynGroup::from(r),
                Err(e) => DynGroup::from(e.with_path(self.path().unwrap_or_default())),
            }
        });
        Group {
            dyn_group: sub,
            domain: Rc::clone(&self.domain),
        }
    }

    pub fn attr(&self) -> Group {
        let attr = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.attr() {
                Ok(r) => DynGroup::from(r),
                Err(e) => DynGroup::from(e.with_path(self.path().unwrap_or_default())),
            }
        });
        Group {
            dyn_group: attr,
            domain: Rc::clone(&self.domain),
        }
    }

    pub fn auto_interpretation(&self) -> Option<&str> {
        if let DynCell::Error(_) = self.dyn_cell {
            return None;
        }
        elevation_registry::auto_interpretation(self)
    }

    pub fn elevate(&self) -> Group {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Group {
                dyn_group: DynGroup::from(err.clone().with_path(self.path().unwrap_or_default())),
                domain: Rc::clone(&self.domain),
            };
        }
        let dyn_group = dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match elevation::Group::new(self.clone()) {
                Ok(r) => DynGroup::from(r),
                Err(e) => DynGroup::from(e.with_path(self.path().unwrap_or_default())),
            }
        });
        Group {
            dyn_group,
            domain: Rc::clone(&self.domain),
        }
    }

    pub fn field(&self) -> Group {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Group {
                dyn_group: DynGroup::from(err.clone().with_path(self.path().unwrap_or_default())),
                domain: Rc::clone(&self.domain),
            };
        }
        Group {
            dyn_group: DynGroup::from(field::Group {
                cell: Rc::new(self.clone()),
            }),
            domain: Rc::clone(&self.domain),
        }
    }

    pub fn be(&self, interpretation: &str) -> Xell {
        self.elevate().get(Value::Str(interpretation)).sub().at(0)
    }

    pub fn to(&self, path: &str) -> Xell {
        if let DynCell::Error(err) = &self.dyn_cell {
            return self.clone();
        }
        let path = guard_ok!(Path::parse(path), err =>
            return Xell {
                dyn_cell: DynCell::from(err),
                domain: Rc::clone(&self.domain),
            }
        );
        let mut searcher = Searcher::new(self.clone(), path);
        match searcher.next() {
            Some(Ok(c)) => c,
            Some(Err(e)) => {
                let mut path = self.path().unwrap_or_default();
                path += searcher.unmatched_path().as_str();
                warning!("💥 path search error, path={:?}; error= {:?}", path, e);
                Xell {
                    dyn_cell: DynCell::from(e.with_path(self.path().unwrap_or_default())),
                    domain: Rc::clone(&self.domain),
                }
            }
            None => {
                let mut path = self.path().unwrap_or_default();
                path += searcher.unmatched_path().as_str();
                warning!("💥 path search failed: {}", path);
                Xell {
                    dyn_cell: DynCell::from(noerr().with_path(path)),
                    domain: Rc::clone(&self.domain),
                }
            }
        }
    }

    pub fn search<'a>(&self, path: &'a str) -> Res<Searcher<'a>> {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Err(err.clone());
        }
        let path = guard_ok!(crate::prog::Path::parse(path), err => {return Err(err)});
        Ok(Searcher::new(self.clone(), path))
    }

    pub fn all(&self, path: &str) -> Res<Vec<Xell>> {
        self.search(path)?.collect()
    }

    pub fn head(&self) -> Res<(Xell, Relation)> {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Err(err.clone());
        }
        if let DynCell::Field(field) = &self.dyn_cell {
            return Ok(((*field.cell).clone(), Relation::Field));
        }
        dispatch_dyn_cell!(&self.dyn_cell, |x| {
            match x.head() {
                Ok((c, r)) => Ok((
                    Xell {
                        dyn_cell: DynCell::from(c),
                        domain: Rc::clone(&self.domain),
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
        if let DynCell::Error(err) = &self.dyn_cell {
            return Ok(String::new());
        }

        fn err_to_string(e: HErr) -> String {
            if e.kind == HErrKind::None {
                return String::new();
            }
            format!("<💥 {}>", e)
        }

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
                // avoid Xell::read which may call path again
                let mut s = dispatch_dyn_cell!(&cell.dyn_cell, |x| {
                    match x.read() {
                        Ok(r) => match r.value() {
                            Ok(v) => v.as_cow_str().as_ref().replace('\n', "\\n"),
                            Err(e) => err_to_string(e),
                        },
                        Err(e) => err_to_string(e),
                    }
                });
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

    pub fn save(&self, target: &Xell) -> Res<()> {
        if let DynCell::Error(err) = &self.dyn_cell {
            return Err(err.clone());
        }
        if let DynCell::Error(err) = &target.dyn_cell {
            return Err(err.clone());
        }
        Self::save_from_to(&self.dyn_cell, target)
    }

    pub fn save_domain(&self, target: &Xell) -> Res<()> {
        if let DynCell::Error(err) = &target.dyn_cell {
            return Err(err.clone());
        }
        let dyn_root = guard_some!(self.domain.dyn_root.get(), {
            return fault("domain root not found while saving domain");
        });
        Self::save_from_to(dyn_root, target)
    }

    fn save_from_to(dyn_cell: &DynCell, target: &Xell) -> Res<()> {
        let serial = dispatch_dyn_cell!(dyn_cell, |x| {
            match x.read()?.serial() {
                Ok(serial) => serial,
                Err(e) => {
                    if e.kind == HErrKind::None {
                        return Ok(()); // no serial, nothing to save
                    } else {
                        return Err(e);
                    }
                }
            }
        });
        target.write().value(OwnValue::String(serial))
    }
}

impl fmt::Debug for Xell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let orig = format!("{:?}", self.domain.origin).replace('\n', "\n\t");
        let root = format!("{:?}", self.domain.dyn_root).replace('\n', "\n\t");
        write!(
            f,
            "Cell{{\n\tdyn_cell={:?}, \n\tdomain={{ write_policy={:?}, is_dirty={}, root={}, origin={} }}",
            self.dyn_cell,
            self.domain.write_policy.get(),
            self.domain.dirty.get(),
            root,
            orig,
        )
    }
}

impl fmt::Debug for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Domain{{ write_policy={:?}, is_dirty={} root={:?}, origin={:?} }}",
            self.write_policy.get(),
            self.dirty.get(),
            self.dyn_root.get(),
            self.origin
                .as_ref()
                .map(|c| c.path().unwrap_or("<error>".into()))
                .unwrap_or_default(),
        )
    }
}

impl Drop for Domain {
    fn drop(&mut self) {
        if self.write_policy.get() != WritePolicy::WriteBackOnDrop {
            return;
        }
        if !self.dirty.get() {
            return;
        }
        let target = guard_some!(self.origin.as_ref(), { return });
        let dyn_root = guard_some!(self.dyn_root.get(), {
            warning!("root of dirty domain not found");
            return;
        });

        if let Err(err) = Xell::save_from_to(dyn_root, target) {
            warning!("💥 while trying to auto-save domain: {:?}", err);
        }
    }
}

#[test]
fn test_cell_domain_path() -> Res<()> {
    let tree = r#"{"a": {"x": "xa", "b": {"x": "xb", "c": {"x": "xc"}}}, "m": [1, 2, 3]}"#;
    let root = Xell::from(tree).be("yaml");

    // assert_eq!(root.domain_path()?, r#""#);
    // let leaf = root.to("/a/b/c/x");
    // assert_eq!(leaf.domain_path()?, r#"/a/b/c/x"#);

    assert_eq!(root.path()?, r#"`{"a": {"x": "xa"...`^yaml"#);

    // assert_eq!(leaf.path()?, r#"`{"a": {"x": "xa"...`^yaml/a/b/c/x"#);

    Ok(())
}

impl Group {
    pub fn label_type(&self) -> LabelType {
        dispatch_dyn_group!(&self.dyn_group, |x| { x.label_type() })
    }

    pub fn len(&self) -> Res<usize> {
        dispatch_dyn_group!(&self.dyn_group, |x| { x.len() })
    }

    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |l| l == 0)
    }

    pub fn at(&self, index: usize) -> Xell {
        if let DynGroup::Elevation(group) = &self.dyn_group {
            // special case for elevation group
            return match group.at_(index) {
                Ok(cell) => cell,
                Err(e) => Xell::from(e),
            };
        }
        dispatch_dyn_group!(&self.dyn_group, |x| {
            Xell {
                dyn_cell: match x.at(index) {
                    Ok(c) => DynCell::from(c),
                    Err(e) => DynCell::from(e),
                },
                domain: Rc::clone(&self.domain),
            }
        })
    }

    pub fn get<'a>(&self, key: impl Into<Value<'a>>) -> Xell {
        let key = key.into();
        if let DynGroup::Elevation(group) = &self.dyn_group {
            // special case for elevation group
            match group.get_(key) {
                Ok(cell) => return cell,
                Err(e) => return Xell::from(e),
            }
        }
        dispatch_dyn_group!(&self.dyn_group, |x| {
            Xell {
                dyn_cell: match x.get_all(key) {
                    Ok(mut iter) => match iter.next() {
                        Some(Ok(cell)) => DynCell::from(cell),
                        Some(Err(err)) => DynCell::from(err),
                        None => DynCell::from(noerr()),
                    },
                    Err(e) => DynCell::from(e),
                },
                domain: Rc::clone(&self.domain),
            }
        })
    }

    pub fn get_all<'a>(&self, key: impl Into<Value<'a>>) -> CellIterator {
        let key = key.into();
        if let DynGroup::Elevation(group) = &self.dyn_group {
            // special case for elevation group
            let cell = group.get_(key);
            return CellIterator {
                cell_iterator: CellIteratorKind::Elevation(cell),
            };
        }
        let dci = dispatch_dyn_group!(&self.dyn_group, |x| {
            match x.get_all(key) {
                Ok(iter) => DynCellIterator::from(iter),
                Err(e) => {
                    if e.kind == HErrKind::None {
                        DynCellIterator::None(std::iter::empty())
                    } else {
                        DynCellIterator::Error(std::iter::once(Err(e)))
                    }
                }
            }
        });
        CellIterator {
            cell_iterator: CellIteratorKind::DynCellIterator {
                dyn_cell: dci,
                domain: Rc::clone(&self.domain),
            },
        }
    }

    pub fn create(&self, label: Option<OwnValue>, value: Option<OwnValue>) -> Res<Xell> {
        Ok(dispatch_dyn_group!(&self.dyn_group, |x| {
            Xell {
                dyn_cell: DynCell::from(x.create(label, value)?),
                domain: Rc::clone(&self.domain),
            }
        }))
    }

    pub fn add(&self, index: Option<usize>, cell: Xell) -> Res<()> {
        dispatch_dyn_group!(&self.dyn_group, |x| { x.add(index, cell.try_into()?) })
    }

    pub fn err(self) -> Res<Group> {
        match self.dyn_group {
            DynGroup::Error(error) => Err(error),
            _ => Ok(Group {
                dyn_group: self.dyn_group,
                domain: self.domain,
            }),
        }
    }
}

impl IntoIterator for Group {
    type Item = Xell;
    type IntoIter = GroupIter;

    fn into_iter(self) -> Self::IntoIter {
        GroupIter(self, 0)
    }
}

#[derive(Debug)]
pub struct GroupIter(Group, usize);
impl Iterator for GroupIter {
    type Item = Xell;
    fn next(&mut self) -> Option<Self::Item> {
        if self.1 >= self.0.len().unwrap_or(0) {
            return None;
        }
        self.1 += 1;
        Some(self.0.at(self.1 - 1))
    }
}

impl Iterator for CellIterator {
    type Item = Xell;
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.cell_iterator {
            CellIteratorKind::DynCellIterator { dyn_cell, domain } => {
                dispatch_dyn_cell_iterator!(dyn_cell, |x| {
                    x.next().map(|cell_res| Xell {
                        dyn_cell: match cell_res {
                            Ok(cell) => DynCell::from(cell),
                            Err(err) => DynCell::from(err),
                        },
                        domain: Rc::clone(domain),
                    })
                })
            }
            CellIteratorKind::Elevation(cell_res) => match cell_res {
                Ok(cell) => {
                    let cell = cell.clone();
                    *cell_res = nores();
                    Some(cell)
                }
                Err(err) => {
                    if err.kind == HErrKind::None {
                        None
                    } else {
                        Some(Xell {
                            dyn_cell: DynCell::from(err.clone()),
                            domain: Rc::new(Domain {
                                write_policy: cell::Cell::new(WritePolicy::ReadOnly),
                                origin: None,
                                dyn_root: OnceCell::new(),
                                dirty: cell::Cell::new(false),
                            }),
                        })
                    }
                }
            },
        }
    }
}

impl DoubleEndedIterator for CellIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.cell_iterator {
            CellIteratorKind::DynCellIterator { dyn_cell, domain } => {
                dispatch_dyn_cell_iterator!(dyn_cell, |x| {
                    x.next_back().map(|cell_res| Xell {
                        dyn_cell: match cell_res {
                            Ok(cell) => DynCell::from(cell),
                            Err(err) => DynCell::from(err),
                        },
                        domain: Rc::clone(domain),
                    })
                })
            }
            CellIteratorKind::Elevation(cell_res) => match cell_res {
                Ok(cell) => {
                    let cell = cell.clone();
                    *cell_res = nores();
                    Some(cell)
                }
                Err(err) => {
                    if err.kind == HErrKind::None {
                        None
                    } else {
                        Some(Xell {
                            dyn_cell: DynCell::from(err.clone()),
                            domain: Rc::new(Domain {
                                write_policy: cell::Cell::new(WritePolicy::ReadOnly),
                                origin: None,
                                dyn_root: OnceCell::new(),
                                dirty: cell::Cell::new(false),
                            }),
                        })
                    }
                }
            },
        }
    }
}

impl CellIterator {
    pub fn err(self) -> Res<CellIterator> {
        match self.cell_iterator {
            CellIteratorKind::DynCellIterator { dyn_cell, domain } => match dyn_cell {
                DynCellIterator::Error(mut error_iterator) => match error_iterator.next() {
                    Some(Ok(err)) => Err(err),
                    Some(Err(err)) => Err(err),
                    None => Ok(CellIterator {
                        cell_iterator: CellIteratorKind::DynCellIterator {
                            dyn_cell: DynCellIterator::None(std::iter::empty()),
                            domain,
                        },
                    }),
                },
                _ => Ok(CellIterator {
                    cell_iterator: CellIteratorKind::DynCellIterator { dyn_cell, domain },
                }),
            },
            CellIteratorKind::Elevation(cell_res) => match cell_res {
                Ok(cell) => Ok(CellIterator {
                    cell_iterator: CellIteratorKind::Elevation(Ok(cell)),
                }),
                Err(err) => Err(err),
            },
        }
    }
}
