use crate::{base::*, pprint::pprint, utils::log::set_verbose};

#[test]
fn test_rust() -> Res<()> {
    set_verbose(true);
    let folder = Cell::from(".").be("path").be("fs").err().unwrap();
    let root = folder.to("/src/tests/rust.rs^rust");
    println!("{:?}", root.as_file_path());
    pprint(&root, 0, 0);
    assert_eq!(root.to("/hosts/[0]/labels/power").read().value()?, "");
    // yaml.to("/hosts/[0]/labels/power").write().value()?
    // yaml.to("/hosts/[0]/labels/power")
    //     .write()
    //     .set_value("putty".into())?;
    // assert_eq!(yaml.to("/hosts/[0]/labels/power").read().value()?, "putty");

    Ok(())
}

#[test]
fn rust_write() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

#[test]
fn rust_path() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

#[test]
fn rust_save() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}
