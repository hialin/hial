use crate::{api::*, pprint::pprint, utils::log::set_verbose};

#[test]
fn test_nested_0() -> Res<()> {
    set_verbose(true);

    let yxj = r#"{"one": ["<?xml?><root>mytext: This is my yaml string</root>"]}"#;

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
    let text = Xell::from(r#"{"one": ["<?xml?><root><a>mytext: yaml string</a></root>"]}"#)
        .policy(WritePolicy::WriteBackOnDrop);

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
        pprint(&cell, 0, 0);
        println!("6");
        assert_eq!(cell.read().value()?, "yaml string");
        println!("7");
        cell.write().value("NEW YAML STRING")?;
        println!("8");
        println!("mytext cell: {:?}\n", cell);
    }

    assert_eq!(
        text.read().value()?,
        r#"{"one":["<?xml?><root><a>mytext: NEW YAML STRING</a></root>"]}"#
    );

    Ok(())
}
