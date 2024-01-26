use crate::{base::*, utils::log::set_verbose};

#[test]
fn test_rust() -> Res<()> {
    set_verbose(true);

    let folder = Cell::from(".").be("path").be("fs").err().unwrap();
    let root = folder.to("/src/tests/rust.rs^rust");
    assert_eq!(
        root.search("/*[#type=='function_item']/name")?
            .all()?
            .into_iter()
            .map(|c| c.debug_string())
            .collect::<Vec<_>>(),
        vec![
            "name:test_rust",
            "name:rust_path",
            "name:rust_write",
            "name:rust_save",
            "name:editable_rust_fn"
        ]
    );

    Ok(())
}

#[test]
fn rust_path() -> Res<()> {
    set_verbose(true);
    let folder = Cell::from(".").be("path").be("fs").err().unwrap();
    let root = folder.to("/src/tests/rust.rs^rust");
    // pprint(&root, 0, 0);
    let func = root.to("/*[#type=='function_item']/name");
    assert_eq!(func.path()?, "`.`^path^fs/src/tests/rust.rs^rust/[2]/name",);

    Ok(())
}

#[test]
fn rust_write() -> Res<()> {
    set_verbose(true);
    let root = Cell::from(".").to("^path^fs/src/tests/rust.rs^rust");

    assert_eq!(root.to("/[9]/[1]").read().value()?, "editable_rust_fn");

    root.to("/[9]/[1]")
        .write()
        .set_value("modified_rust_fn".into())?;
    assert_eq!(root.to("/[9]/[1]").read().value()?, "modified_rust_fn");

    root.to("/[9]/[1]")
        .write()
        .set_value("editable_rust_fn".into())?;
    assert_eq!(root.to("/[9]/[1]").read().value()?, "editable_rust_fn");

    Ok(())
}

#[test]
fn rust_save() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

fn editable_rust_fn() {}
