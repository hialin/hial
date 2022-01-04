use hiallib::base::*;
use hiallib::pprint::pprint;

// examples = "."^file/examples;
// for stack in examples/productiondump.json^json/stacks/*[/system_stack != true]:
//      compose = stack/dockerCompose^yaml
//      examples/.new(compose/services/[0].key + ".json") = compose

fn main() -> Res<()> {
    test_rustapi()?;
    test_rustapi_with_path()
}

fn test_rustapi() -> Res<()> {
    let examples = Cell::from(".".to_string())
        .be("file")?
        .sub()?
        .get("examples")?;
    pprint(&examples, 0, 0);
    let folder = examples.sub()?;
    let stacks = folder
        .get("productiondump.json")?
        .be("json")?
        .sub()?
        .get("stacks")?;
    for stack in stacks.sub()? {
        let stack = stack?;
        pprint(&stack, 0, 0);
        let stack_sub = stack.sub()?;
        if stack_sub.get("system_stack")?.value().get()? == Value::Bool(true) {
            continue;
        }

        let yaml = stack_sub
            .get("dockerCompose")?
            .value()
            .get()?
            .to_owned_value();
        let yaml = Cell::from(yaml).be("yaml")?;
        pprint(&yaml, 0, 0);
        let service_node = yaml.sub()?.get("services")?.sub()?.at(0)?;
        let nameref = service_node.label();
        let mut filename = format!("{}.yaml", nameref.get()?);
        while folder.get(&filename).is_ok() {
            filename = format!("{}.yaml", filename);
        }
        println!("{}", filename);
        return Ok(());

        // folder.new(filename).be("yaml").set(yaml)
    }
    Ok(())
}

fn test_rustapi_with_path() -> Res<()> {
    let folder = Cell::from(".".to_string())
        .be("file")?
        .sub()?
        .get("examples")?;
    let stacks = folder.search("/productiondump.json^json/stacks")?.first()?;
    for stack in stacks.sub()? {
        let stack = stack?;
        if stack.search("/system_stack")?.first()?.value().get()? == "true" {
            continue;
        }
        let service = stack
            .search("/dockerCompose^string^yaml/services")?
            .first()?;
        println!("service found");
        let nameref = service.value();
        let name = nameref.get()?;
        let mut filename = format!("{}.yaml", name);
        while folder.sub()?.get(&filename).is_ok() {
            filename = format!("{}.yaml", filename);
        }
        // folder.new(filename).be("yaml").set(yaml)
    }
    Ok(())
}
