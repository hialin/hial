use crate::{api::*, config::ColorPalette, pprint};

#[test]
fn test_xml() -> Res<()> {
    let xml = r#"
            <?xml version="1.0"?>
            <!DOCTYPE entity PUBLIC "-//no idea//EN" "http://example.com/dtd">
            <doc>
                <first>1</first>
                <double>2</double>
                <double>2+</double>
                <triple/>
                <q>
                    <qq>4</qq>
                </q>
            </doc>
        "#;
    let xml = Xell::from(xml.to_string()).be("xml");
    pprint(&xml, 0, 0, ColorPalette::None);

    let decl = xml.sub().at(0);
    assert_eq!(decl.read().label()?, "xml");
    assert_eq!(decl.attr().len()?, 1);
    let decl_reader = decl.attr().at(0).read();
    assert_eq!(decl_reader.label()?, "version");
    assert_eq!(decl_reader.value()?, Value::Str("1.0"));

    let doctype = xml.sub().at(1);
    assert_eq!(doctype.read().label()?, "DOCTYPE");
    assert_eq!(
        doctype.read().value()?,
        "entity PUBLIC \"-//no idea//EN\" \"http://example.com/dtd\""
    );

    let doc = xml.sub().at(2);
    assert_eq!(doc.sub().len()?, 5);

    assert_eq!(doc.sub().at(0).read().label()?, "first");
    assert_eq!(doc.sub().get("first").read().label()?, "first");
    assert_eq!(doc.sub().get("first").read().value()?, "1");
    assert_eq!(doc.sub().get("first").sub().len()?, 0);

    assert_eq!(doc.sub().get("double").read().label()?, "double");
    assert_eq!(doc.sub().at(1).read().label()?, "double");
    assert_eq!(doc.sub().at(1).read().value()?, "2");
    assert_eq!(doc.sub().at(1).sub().len()?, 0);

    assert_eq!(doc.sub().at(2).read().label()?, "double");
    assert_eq!(doc.sub().at(2).read().value()?, "2+");
    assert_eq!(doc.sub().at(2).sub().len()?, 0);

    assert_eq!(doc.sub().at(3).read().value()?, "");
    assert_eq!(doc.sub().get("triple").read().value()?, "");
    assert_eq!(doc.sub().get("triple").sub().len()?, 0);

    assert_eq!(doc.sub().at(4).sub().at(0).read().label()?, "qq");
    assert_eq!(doc.sub().get("q").sub().get("qq").read().value()?, "4");
    assert_eq!(xml.to("/doc/q/qq").read().value()?, Value::Str("4"));
    assert_eq!(xml.to("/doc/q/qq").sub().len()?, 0);

    Ok(())
}

#[test]
fn xml_path() -> Res<()> {
    let xml = r#"<?xml version="1.0"?>
            <!DOCTYPE entity PUBLIC "-//no idea//EN" "http://example.com/dtd">
            <doc>
                <first>1</first>
                <double>2</double>
                <double>2+</double>
                <triple/>
                <q>
                    <qq>4</qq>
                </q>
            </doc>
        "#;
    let xml = Xell::from(xml.to_string()).to("^xml/doc/q/qq");
    assert_eq!(xml.path()?, "`<?xml version=\"1.0\"…`^xml/doc/q/qq");
    Ok(())
}

#[test]
fn xml_write_and_save() -> Res<()> {
    let text = Xell::from(
        r#" <?xml version="1.0"?>
            <doc>
                <first>1</first>
                <double>2</double>
                <double>2+</double>
                <triple/>
                <q>
                    <qq>4</qq>
                </q>
            </doc>
        "#
        .replace(['\n', '\t'], ""),
    )
    .policy(WritePolicy::NoAutoWrite);
    let xml = text.be("xml");

    pprint(&xml, 0, 0, ColorPalette::None);
    assert_eq!(xml.to("/doc/q/qq").read().value()?, "4");

    xml.to("/doc/q/qq").write().value("444")?;
    assert_eq!(xml.to("/doc/q/qq").read().value()?, "444");

    xml.save(&xml.origin())?;
    let v = text.to("^xml/doc/q/qq");
    assert_eq!(v.read().value()?, "444");

    Ok(())
}
