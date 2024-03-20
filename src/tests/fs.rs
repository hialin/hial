use crate::{api::*, prog::Path};

#[test]
fn test_files() -> Res<()> {
    crate::utils::log::set_verbose(true);
    let examples = Xell::from(".").be("path").be("fs").sub().get("examples");
    assert_eq!(examples.read().label()?, "examples");
    assert_eq!(examples.read().value().unwrap_err().kind, HErrKind::None);
    Ok(())
}

#[test]
fn file_not_found() -> Res<()> {
    crate::utils::log::set_verbose(true);
    let examples = Xell::new("./src/tests/data/assignment.jso");
    assert_eq!(examples.clone().err().unwrap_err().kind, HErrKind::None);
    println!("{:?}", examples.read());
    Ok(())
}

#[test]
fn test_file_path_tilde() -> Res<()> {
    crate::utils::log::set_verbose(true);
    let home = Xell::from("~").be("path").be("fs");
    assert!(home.sub().len()? > 0);
    Ok(())
}

#[test]
fn test_fs() -> Res<()> {
    crate::utils::log::set_verbose(true);
    let examples = Xell::from(".").be("path").be("fs").sub().get("examples");
    // assert_eq!(std::mem::size_of_val(&examples), 4 * 8); // TODO: file cell is too large
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
fn fs_write_prog_policy() -> Res<()> {
    let p = "^path^fs/src/tests/data/write.txt";
    let c = Xell::from(".")
        .policy(WritePolicy::NoAutoWrite)
        .to(p)
        .err()?;
    c.write().value("Hi there")?;
    assert_eq!(
        Xell::from(".").to(p).read().value()?,
        Value::Bytes("Hi there".as_bytes())
    );
    c.write().value("-")?;
    assert_eq!(
        Xell::from(".").to(p).read().value()?,
        Value::Bytes("-".as_bytes())
    );
    Ok(())
}

#[test]
fn fs_write_path_policy() -> Res<()> {
    let p = ".^fs[w]/src/tests/data/write2.txt";
    let c = Xell::new(p).err()?;
    c.write().value("Hi there")?;
    assert_eq!(
        Xell::new(p).read().value()?,
        Value::Bytes("Hi there".as_bytes())
    );
    c.write().value("-")?;
    assert_eq!(Xell::new(p).read().value()?, Value::Bytes("-".as_bytes()));
    Ok(())
}

#[test]
fn fs_path() -> Res<()> {
    let c = Xell::from(".")
        .be("path")
        .be("fs")
        .sub()
        .get("src")
        .sub()
        .get("tests")
        .sub()
        .get("data");
    assert_eq!(c.path()?, "`.`^path^fs/src/tests/data");
    Ok(())
}

#[test]
fn fs_write_beyond_fs() -> Res<()> {
    // test that the fs drop will not try to write back to the path cell itself
    let ov = Xell::from("./src/tests/data/write3.txt").policy(WritePolicy::WriteBackOnDrop);
    {
        ov.be("path").be("fs").write().value("A")?;
        ov.be("path").be("fs").write().value("B")?;
        // fs should try to write back now
    }
    // TODO: text failure: stop fs from writing back to the path cell
    // assert_eq!(ov.be("path").be("fs").read().value()?, "Hi there");
    Ok(())
}
