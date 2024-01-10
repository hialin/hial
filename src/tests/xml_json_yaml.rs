use crate::base::*;
use crate::pprint;

#[test]
fn test_files() -> Res<()> {
    let examples = Cell::from(".").be("path").be("file").sub().get("examples");
    assert_eq!(examples.read().label()?, "examples");
    assert_eq!(examples.read().value()?, "examples");
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
    let json = Cell::from(json).be("json");
    // pprint::pprint(&json, 0, 0);
    let hosts = json.sub().get("hosts").sub();
    assert_eq!(hosts.len()?, 2);
    let host1 = hosts.at(0);
    let host2 = hosts.at(1);
    let power1 = host1.sub().get("labels").sub().get("power");
    let power2 = host2.sub().get("labels").sub().get("power");
    let group2 = host2.sub().get("labels").sub().get("group2");
    assert_eq!(power1.read().value()?, Value::Str("weak"));
    assert_eq!(power2.read().value()?, Value::Str("strong"));
    assert_eq!(group2.read().value()?, Value::Bool(true));
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
    let yaml = Cell::from(yaml.to_string()).be("yaml");
    // pprint::pprint(&yaml, 0, 0);
    let hosts = yaml.sub().get("hosts").sub();
    assert_eq!(hosts.len()?, 2);
    let host1 = hosts.at(0);
    let host2 = hosts.at(1);
    let power1 = host1.sub().get("labels").sub().get("power");
    let power2 = host2.sub().get("labels").sub().get("power");
    let group2 = host2.sub().get("labels").sub().get("group2");
    assert_eq!(power1.read().value()?, Value::Str("weak"));
    assert_eq!(power2.read().value()?, Value::Str("strong"));
    assert_eq!(group2.read().value()?, Value::Bool(true));
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
    let xml = Cell::from(xml.to_string()).be("xml");
    // pprint::pprint(&xml, 0, 0);

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
    assert_eq!(doc.sub().len()?, 4);
    assert_eq!(doc.sub().get("first").read().label()?, "first");
    assert_eq!(doc.sub().at(1).read().label()?, "double");
    assert_eq!(doc.sub().at(2).read().value()?, Value::Str("double"));
    assert_eq!(
        doc.sub().get("triple").read().value()?,
        Value::Str("triple")
    );
    Ok(())
}

#[test]
fn test_toml() -> Res<()> {
    let toml = r#"
        # This is a TOML document

        title = "TOML Example"

        [owner]
        name = "Tom Preston-Werner"
        dob = 1979-05-27T07:32:00-08:00

        [database]
        enabled = true
        ports = [ 8000, 8001, 8002 ]
        data = [ ["delta", "phi"], [3.14] ]
        temp_targets = { cpu = 79.5, case = 72.0 }

        [servers]

        [servers.alpha]
        ip = "10.0.0.1"
        role = "frontend"

        [servers.beta]
        ip = "10.0.0.2"
        role = "backend"
    "#;
    let toml = Cell::from(toml.to_string()).be("toml");
    pprint::pprint(&toml, 0, 0);
    let value = toml.sub().get("database").sub().get("enabled");
    assert_eq!(value.read().value()?, Value::Bool(true));
    Ok(())
}
