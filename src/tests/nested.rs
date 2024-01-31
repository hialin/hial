use crate::{base::*, utils::log::set_verbose};

#[test]
fn test_nested() -> Res<()> {
    set_verbose(true);

    let yxj = r#"{"one": ["<?xml?><root>mytext: This is my yaml string</root>"]}"#;

    let cell = Cell::from(yxj).to("^json/one/[0]^xml/root");
    assert_eq!(cell.read().value()?, "mytext: This is my yaml string");

    let cell = Cell::from(yxj).to("^json/one/[0]^xml/root^yaml/mytext");
    assert_eq!(cell.read().value()?, "This is my yaml string");

    Ok(())
}

#[test]
fn test_nested_mut() -> Res<()> {
    set_verbose(true);

    let text = Cell::from(r#"{"one": ["<?xml?><root><a>mytext: yaml string</a></root>"]}"#);
    let root = text.clone();

    assert_eq!(
        root.to("^json/one/[0]^xml/root/a^yaml/mytext")
            .read()
            .value()?,
        "yaml string"
    );

    {
        let cell = root.to("^json/one/[0]^xml/root/a^yaml/mytext");
        assert_eq!(cell.read().value()?, "yaml string");
        cell.write().set_value("new yaml string".into())?;
        root.save(root.origin())?;
    }

    assert_eq!(
        root.read().value()?,
        r#"{"one": ["<?xml?><root><a>mytext: new yaml string</a></root>"]}"#
    );

    Ok(())
}
