use crate::base::*;
use crate::pprint::pprint;

const TOML: &str = r#"
# This is a TOML document
title = "TOML Example"

[owner]
name = "xxx"

[database]
enabled = true
ports = [ 8000, 8001, 8002 ]
data = [ ["delta", "phi"], [23.1415] ]
temp_targets = { cpu = 79.5, case = 72.0 }

[servers]

[servers.alpha]
ip = "10.0.0.1"
role = "frontend"

[servers.beta]
ip = "10.0.0.2"
role = "backend"
"#;

#[test]
fn toml_read() -> Res<()> {
    let toml = Cell::from(TOML).be("toml");
    let value = toml.sub().get("database").sub().get("enabled");
    pprint(&value, 0, 0);
    assert_eq!(value.read().value()?, Value::Bool(true));
    Ok(())
}

#[test]
fn toml_path() -> Res<()> {
    let toml = Cell::from(TOML).be("toml").to("/database/data/[0]/[1]");
    assert_eq!(
        toml.path()?,
        "`\\n# This is a TO...`^toml/database/data/[0]/[1]"
    );
    Ok(())
}

#[test]
fn toml_write_and_save() -> Res<()> {
    let data = Cell::from("[number]\nx = 23.1415");
    let toml = data.be("toml");

    let v = toml.to("/number/x");
    assert_eq!(v.read().value()?, Value::from(23.1415));

    v.write().set_value(1.1415.into())?;
    let v = toml.to("/number/x");
    assert_eq!(v.read().value()?, Value::from(1.1415));

    toml.save(toml.origin())?;
    let v = data.to("^toml/number/x");
    assert_eq!(v.read().value()?, Value::from(1.1415));

    Ok(())
}
