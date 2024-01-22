use crate::base::*;
use crate::pathlang::path::Path;

#[test]
fn test_files() -> Res<()> {
    let examples = Cell::from(".").be("path").be("fs").sub().get("examples");
    assert_eq!(examples.read().label()?, "examples");
    assert_eq!(examples.read().value().unwrap_err().kind, HErrKind::None);
    Ok(())
}

#[test]
fn test_fs() -> Res<()> {
    crate::utils::log::set_verbose(true);
    println!("{:?}", Cell::from(".").be("path").be("fs"));
    for x in Cell::from(".").be("path").be("fs").sub().err()? {
        println!("{:?}", x.read().label());
    }
    let examples = Cell::from(".").be("path").be("fs").sub().get("examples");
    // assert_eq!(std::mem::size_of_val(&examples), 4 * 8); // todo file cell is too large
    assert_eq!(
        examples.read().label().unwrap_or(Value::None),
        Value::Str("examples")
    );
    assert_eq!(examples.read().value().unwrap_err().kind, HErrKind::None);
    Ok(())
}

#[test]
fn search_path_with_fs_starter() -> Res<()> {
    let path = "./LICENSE@size";
    let (root, path) = Path::parse_with_starter(path)?;
    let eval = path
        .eval(root.eval()?)
        .map(|c| Ok(c?.read().value()?.to_string()))
        .collect::<Res<Vec<_>>>()?;
    assert_eq!(eval, ["26526"]);
    Ok(())
}

#[test]
fn fs_write() -> Res<()> {
    let t = "Hi there";
    let p = "^path^fs/examples/write.txt";
    let c = Cell::from(".").to(p).err()?;
    c.write().set_value(t.into())?;
    assert_eq!(Cell::from(".").to(p).read().value()?, t);
    c.write().set_value("-".into())?;
    assert_eq!(Cell::from(".").to(p).read().value()?, "-");
    Ok(())
}

#[test]
fn fs_path() -> Res<()> {
    let c = Cell::from(".").be("path").be("fs").sub().get("examples");
    assert_eq!(c.path()?, "`.`^path^fs/examples");
    Ok(())
}
