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
    let mut f1 = json.path(path)?.first()?;
    f1.set(newvalue.to_owned_value())?;
    // pprint::pprint(&json, 0, 0);
    assert_eq!(json.path(path)?.first()?.value()?, newvalue);
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
    json.path(path1)?.first()?.set(newvalue.to_owned_value())?;

    let path2 = "/hosts/[0]/host_id";
    json.path(path2)?.first()?.set(OwnedValue::None)?;

    // pprint::pprint(&json, 0, 0);

    assert_eq!(json.path(path1)?.first()?.value()?, newvalue);
    assert_eq!(json.path(path2)?.first()?.value()?, Value::None);

    // json.domain().save_to_origin();
    assert_eq!(
        json_original.value()?,
        r#"{
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
        }"#
    );

    Ok(())
}
