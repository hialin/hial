use hiallib::api::*;
use hiallib::{config::ColorPalette, pprint};

// examples = "."^file/examples;
// for stack in examples/productiondump.json^json/stacks/*[/system_stack != true]:
//      compose = stack/dockerCompose^yaml
//      examples/.new(compose/services/[0].key + ".json") = compose

fn main() -> Res<()> {
    test_rustapi()?;
    test_rustapi_with_path()
}

fn test_rustapi() -> Res<()> {
    let examples = Xell::from(".").be("fs").sub().get("examples");
    pprint(&examples, 0, 0, ColorPalette::None);
    let folder = examples.sub();
    let stacks = folder
        .get("productiondump.json")
        .be("json")
        .sub()
        .get("stacks");
    for stack in stacks.sub() {
        pprint(&stack, 0, 0, ColorPalette::None);
        let stack_sub = stack.sub();
        if stack_sub.get("system_stack").read().value()? == Value::Bool(true) {
            continue;
        }

        let yaml = stack_sub
            .get("dockerCompose")
            .read()
            .value()?
            .to_owned_value();
        let yaml = Xell::from(yaml).be("yaml");
        pprint(&yaml, 0, 0, ColorPalette::None);
        let service_node = yaml.sub().get("services").sub().at(0);
        let mut filename = format!("{}.yaml", service_node.read().label()?);
        while folder.get(filename.as_str()).err().is_ok() {
            filename = format!("{}.yaml", filename);
        }
        println!("{}", filename);
        return Ok(());

        // folder.new(filename).be("yaml").set(yaml)
    }
    Ok(())
}

fn test_rustapi_with_path() -> Res<()> {
    let folder = Xell::from(".").to("^file/examples");
    let stacks = folder.to("/productiondump.json^json/stacks");
    for stack in stacks.sub() {
        if stack.to("/system_stack").read().value()? == "true" {
            continue;
        }
        let service = stack.to("/dockerCompose^string^yaml/services");
        println!("service found");
        let mut filename = format!("{}.yaml", service.read().value()?);
        while folder.sub().get(filename.as_str()).err().is_ok() {
            filename = format!("{}.yaml", filename);
        }
        // folder.new(filename).be("yaml").set(yaml)
    }
    Ok(())
}
