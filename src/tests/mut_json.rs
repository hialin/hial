use crate::{base::*, pprint::pprint};

#[test]
fn mutate_json() -> Res<()> {
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
fn mutate_and_write_json() -> Res<()> {
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

    // TODO: uncomment and fix this write_back() here
    json.domain().save(SaveTarget::Origin)?;

    assert_eq!(
        flattree.read().value()?.to_string(),
        r#"{"hosts":[{"host_id":null,"dummy":true},{"host_id":"1h51","labels":{"group2":true,"power":"weak as putty"}}]}"#
    );

    Ok(())
}
