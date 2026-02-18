use crate::{api::*, config::ColorPalette, pprint, utils::log::set_verbose};

#[test]
fn test_nested_0() -> Res<()> {
    set_verbose(true);

    let yxj = r#"{"one": ["<?xml version='1.0'?><root>mytext: This is my yaml string</root>"]}"#;

    let cell = Xell::from(yxj).to("^json/one/[0]^xml/root");
    assert_eq!(cell.read().value()?, "mytext: This is my yaml string");

    let cell = Xell::from(yxj).to("^json/one/[0]^xml/root^yaml/mytext");
    assert_eq!(cell.read().value()?, "This is my yaml string");

    Ok(())
}

#[test]
fn test_nested_mut() -> Res<()> {
    set_verbose(true);

    println!("1");
    let s = r#"{"one": ["<?xml version='1.0'?><root><a>mytext: yaml string</a></root>"]}"#;
    let text = Xell::from(s).policy(WritePolicy::WriteBackOnDrop);

    println!("2");
    {
        let mytext = text.be("json");
    }
    println!("3");

    {
        let mytext = text
            .be("json")
            .sub()
            .get("one")
            .sub()
            .at(0)
            .be("xml")
            .sub()
            .get("root")
            .sub()
            .get("a")
            .be("yaml")
            .sub()
            .get("mytext");
        println!("4");
    }

    println!("5");

    {
        let cell = text.to("^json/one/[0]^xml/root/a^yaml/mytext");
        pprint(&cell, 0, 0, ColorPalette::None);
        println!("6");
        assert_eq!(cell.read().value()?, "yaml string");
        println!("7");
        cell.write().value("NEW YAML STRING")?;
        println!("8");
        println!("mytext cell: {:?}\n", cell);
    }

    assert_eq!(
        text.read().value()?,
        r#"{"one":["<?xml version=\"1.0\"?><root><a>mytext: NEW YAML STRING</a></root>"]}"#
    );

    Ok(())
}
