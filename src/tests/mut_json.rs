use crate::rust_api::*;
use crate::*;

#[test]
fn mutate_json() -> Res<()> {
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

    pprint::pprint(&json, 0, 0);

    let path = "/hosts/[1]/labels/power";
    let newvalue = Value::Str("insanely strong");
    json.path(path)?.first()?.set(newvalue.to_owned_value())?;

    pprint::pprint(&json, 0, 0);

    assert_eq!(json.path(path)?.first()?.value()?, newvalue);
    Ok(())
}
