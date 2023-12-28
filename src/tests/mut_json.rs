use crate::base::*;

#[test]
fn mutate_json() -> Res<()> {
    let json = r#"{
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
        }"#;
    let json = Cell::from(json.to_string()).be("json")?;
    // pprint::pprint(&json, 0, 0);
    let path = "/hosts/[1]/labels/power";
    let newvalue = Value::Str("insanely strong");
    let f1 = json.search(path)?.first()?;
    f1.write()?.set_value(newvalue.to_owned_value())?;
    // pprint::pprint(&json, 0, 0);
    assert_eq!(json.search(path)?.first()?.read()?.value()?, newvalue);
    Ok(())
}

#[test]
fn mutate_and_write_json() -> Res<()> {
    let json = r#"{
            "hosts": [
                {
                    "host_id": "1h48",
                    "dummy": true
                },
                {
                    "labels": {
                        "group2": true,
                        "power": "strong"
                    }
                }
            ]
        }"#;
    let json_original = Cell::from(json.to_string());
    let json = json_original.clone().be("json")?;

    // pprint::pprint(&json, 0, 0);

    let path1 = "/hosts/[1]/labels/power";
    let newvalue = Value::Str("weak as putty");
    json.search(path1)?
        .first()?
        .write()?
        .set_value(newvalue.to_owned_value())?;

    let path2 = "/hosts/[0]/host_id";
    json.search(path2)?
        .first()?
        .write()?
        .set_value(OwnedValue::None)?;

    // pprint::pprint(&json, 0, 0);

    assert_eq!(json.search(path1)?.first()?.read()?.value()?, newvalue);
    assert_eq!(json.search(path2)?.first()?.read()?.value()?, Value::None);

    // TODO: uncomment and fix this write_back() here
    // json.domain().write_back()?;

    assert_eq!(
        json_original.read()?.value()?.to_string(),
        r#"{
            "hosts": [
                {
                    "host_id": null,
                    "dummy": true
                },
                {
                    "labels": {
                        "group2": true,
                        "power": "weak as putty"
                    }
                }
            ]
        }"#
    );

    Ok(())
}
