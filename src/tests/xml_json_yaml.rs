use crate::base::*;

#[test]
fn test_files() -> Res<()> {
    let examples = Cell::from(".".to_string())
        .be("file")?
        .sub()?
        .get("examples")?;
    assert_eq!(examples.label()?, "examples");
    assert_eq!(examples.value()?, "examples");
    Ok(())
}

#[test]
fn test_json() -> Res<()> {
    let json = r#"{
            "hosts": [
                {
                    "host_id": "1h48",
                    "labels": {
                        "power": "weak",
                        "gateway": "true"
                    }
                },
                {
                    "host_id": "1h51",
                    "labels": {
                        "group2": true,
                        "power": "strong"
                    }
                }
            ]
        }"#;
    let json = Cell::from(json.to_string()).be("json")?;
    // pprint::pprint(&json, 0, 0);
    let hosts = json.sub()?.get("hosts")?.sub()?;
    assert_eq!(hosts.len(), 2);
    let host1 = hosts.at(0)?;
    let host2 = hosts.at(1)?;
    let power1 = host1.sub()?.get("labels")?.sub()?.get("power")?;
    let power2 = host2.sub()?.get("labels")?.sub()?.get("power")?;
    let group2 = host2.sub()?.get("labels")?.sub()?.get("group2")?;
    assert_eq!(power1.value()?, Value::Str("weak"));
    assert_eq!(power2.value()?, Value::Str("strong"));
    assert_eq!(group2.value()?, Value::Bool(true));
    Ok(())
}

#[test]
fn test_yaml() -> Res<()> {
    let yaml = r#"
            hosts:
              - host_id: 1h48
                labels:
                  power: "weak"
                  gateway: "true"
              - host_id: "1h51"
                labels:
                  "group2": true
                  "power": "strong"
        "#;
    let yaml = Cell::from(yaml.to_string()).be("yaml")?;
    // pprint::pprint(&yaml, 0, 0);
    let hosts = yaml.sub()?.get("hosts")?.sub()?;
    assert_eq!(hosts.len(), 2);
    let host1 = hosts.at(0)?;
    let host2 = hosts.at(1)?;
    let power1 = host1.sub()?.get("labels")?.sub()?.get("power")?;
    let power2 = host2.sub()?.get("labels")?.sub()?.get("power")?;
    let group2 = host2.sub()?.get("labels")?.sub()?.get("group2")?;
    assert_eq!(power1.value()?, Value::Str("weak"));
    assert_eq!(power2.value()?, Value::Str("strong"));
    assert_eq!(group2.value()?, Value::Bool(true));
    Ok(())
}

#[test]
fn test_xml() -> Res<()> {
    let xml = r#"
            <?xml version="1.0"?>
            <!DOCTYPE entity PUBLIC "-//no idea//EN" "http://example.com/dtd">            
            <doc>
                <first>1</first>
                <double>2</double>
                <double>2</double>
                <triple/>
            </doc>
        "#;
    let xml = Cell::from(xml.to_string()).be("xml")?;
    // pprint::pprint(&xml, 0, 0);
    let decl = xml.sub()?.at(0)?;
    let doc = xml.sub()?.at(2)?;
    assert_eq!(doc.sub()?.len(), 4);
    assert_eq!(doc.sub()?.get("first")?.label()?, "first");
    assert_eq!(doc.sub()?.at(1)?.label()?, "double");
    assert_eq!(doc.sub()?.at(2)?.value()?, Value::Str("double"));
    assert_eq!(doc.sub()?.get("triple")?.value()?, Value::Str("triple"));
    Ok(())
}
