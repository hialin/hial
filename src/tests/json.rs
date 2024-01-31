use crate::base::*;
use crate::pprint::pprint;
use crate::utils::log::set_verbose;

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
fn json_path() -> Res<()> {
    set_verbose(true);
    let treestring = r#"{
        "hosts": [
            {
                "host_id": "1h48",
                "dummy": true
            },
                {
                        "host_id": "1h51",
                        "labels": {
                                "group2": true,
                                "power": "strong"
                        }
                }
        ]
}"#
    .replace([' ', '\t', '\n'], "");
    let json = Cell::from(treestring).be("json");
    let path = "/hosts/[1]/labels/power";
    let target = json.to(path).err()?;
    assert_eq!(
        target.path()?,
        r#"`{"hosts":[{"host...`^json"#.to_string() + path,
    );
    Ok(())
}

#[test]
fn json_write() -> Res<()> {
    let treestring = r#"{
        "hosts": [
            {},
            {
                "host_id": "1h51",
                "labels": {
                    "group2": true,
                    "power": "strong"
                }
            }
        ]
    }"#
    .replace([' ', '\t', '\n'], "");
    let flattree = Cell::from(treestring);
    let json = flattree.be("json");
    pprint(&json, 0, 0);
    {
        let path = "/hosts/[1]/labels/power";
        let newvalue = "insanely strong";
        let f1 = json.to(path);
        println!("{}", f1.path()?);
        f1.write().set_value(newvalue.into())?;
        pprint(&json, 0, 0);
        assert_eq!(json.to(path).read().value()?, newvalue);
    }
    {
        pprint(&json, 0, 0);
        let path = "/hosts/[1]/labels/power";
        let newvalue = "intensity";
        let f1 = json.to(path);
        f1.write().set_label(newvalue.into())?;
        assert_eq!(f1.read().label()?, newvalue);
        pprint(&json, 0, 0);
        assert_eq!(
            json.to("/hosts/[1]/labels/intensity").read().label()?,
            newvalue
        );
    }

    Ok(())
}

#[test]
fn json_write_and_save() -> Res<()> {
    let treestring = r#"{
        "hosts": [
            {
                "host_id": "1h48",
                "dummy": true
            },
                {
                        "host_id": "1h51",
                        "labels": {
                                "group2": true,
                                "power": "strong"
                        }
                }
        ]
}"#
    .replace([' ', '\t', '\n'], "");
    println!("{}", treestring);
    let flattree = Cell::from(treestring);
    let json = flattree.be("json");

    // pprint::pprint(&json, 0, 0);

    let path1 = "/hosts/[1]/labels/power";
    let newvalue = "weak as putty";
    json.to(path1).write().set_value(newvalue.into())?;

    let path2 = "/hosts/[0]/host_id";
    json.to(path2).write().set_value(OwnValue::None)?;

    // pprint::pprint(&json, 0, 0);

    assert_eq!(json.to(path1).read().value()?, newvalue);
    assert_eq!(json.to(path2).read().value()?, Value::None);

    json.save(json.origin())?;

    assert_eq!(
        flattree.read().value()?.to_string(),
        r#"{"hosts":[{"host_id":null,"dummy":true},{"host_id":"1h51","labels":{"group2":true,"power":"weak as putty"}}]}"#
    );

    Ok(())
}
