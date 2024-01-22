use crate::base::*;

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

    yaml.to("/hosts/[0]/labels/power")
        .write()
        .set_value("putty".into())?;
    assert_eq!(yaml.to("/hosts/[0]/labels/power").read().value()?, "putty");

    Ok(())
}

#[test]
fn yaml_write() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

#[test]
fn yaml_path() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

#[test]
fn yaml_save() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}
