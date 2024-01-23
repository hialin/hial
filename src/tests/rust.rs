use crate::{base::*, pprint::pprint, utils::log::set_verbose};

#[test]
fn test_rust() -> Res<()> {
    set_verbose(true);

    let folder = Cell::from(".").be("path").be("fs").err().unwrap();
    let root = folder.to("/src/tests/rust.rs^rust");
    assert_eq!(
        root.search("/*[#type=='function_item']/*[#type=='name']")?
            .all()?
            .into_iter()
            .map(|c| -> Res<String> { Ok(c.err()?.debug_string()) })
            .collect::<Res<Vec<_>>>()?,
        vec![
            ":test_rust",
            ":rust_path",
            ":rust_write",
            ":rust_save",
            ":editable_rust_fn_name"
        ]
    );

    Ok(())
}

#[test]
fn rust_path() -> Res<()> {
    set_verbose(true);

    let folder = Cell::from(".").be("path").be("fs").err().unwrap();
    let root = folder.to("/src/tests/rust.rs^rust");
    root.to("/x").err()?;
    assert_eq!(
        root.to("/*[#type=='function_item']/*[#type=='name']")
            .path()?,
        "`.`^path^fs/src/tests/rust.rs^rust/",
    );

    Ok(())
}

#[test]
fn rust_write() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

#[test]
fn rust_save() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

fn editable_rust_fn_name() {}
