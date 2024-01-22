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

#[test]
fn toml_read() -> Res<()> {
    let toml = Cell::from(TOML).be("toml");
    let value = toml.sub().get("database").sub().get("enabled");
    pprint(&value, 0, 0);
    assert_eq!(value.read().value()?, Value::Bool(true));
    Ok(())
}

#[test]
fn toml_write() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

#[test]
fn toml_path() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}

#[test]
fn toml_save() -> Res<()> {
    assert_eq!(1, 0);
    Ok(())
}