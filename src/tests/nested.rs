use crate::{base::*, utils::log::set_verbose};

#[test]
fn test_nested() -> Res<()> {
    set_verbose(true);

    let yxj = r#"{"one": ["<?xml?><root>mytext: This is my yaml string</root>"]}"#;

    let cell = Cell::from(yxj)
        .search("^json/one/[0]#value^xml/root/[0]")?
        .first();

    assert_eq!(
        cell.read().value()?,
        Value::Str("mytext: This is my yaml string")
    );

    let cell = Cell::from(yxj)
        .search("^json/one/[0]#value^xml/root/[0]#value^yaml/mytext#value")?
        .first();

    assert_eq!(cell.read().value()?, Value::Str("This is my yaml string"));

    Ok(())
}
