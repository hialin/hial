use crate::{base::*, pprint::pprint, utils::log::set_verbose};

#[test]
fn test_nested() -> Res<()> {
    set_verbose(true);
    let yxj = r#"{"one": ["<?xml?><root>mytext: This is my yaml string</root>"]}"#;

    let cell = Cell::from(OwnValue::from(yxj.to_string()))
        .search("^json/one/[0]#value^xml/root/[0]")?
        .first()?;
    pprint(&cell, 0, 0);
    assert_eq!(
        cell.read()?.value()?,
        Value::Str("mytext: This is my yaml string")
    );

    let cell = Cell::from(OwnValue::from(yxj.to_string()))
        .search("^json/one/[0]#value^xml/root/[0]#value^yaml/mytext#value")?
        .first()?;

    pprint(&cell, 0, 0);

    assert_eq!(cell.read()?.value()?, Value::Str("This is my yaml string"));

    Ok(())
}
