use hiallib::base::*;
// use hiallib::c_api::*;
// use std::ffi::CString;

fn main() -> Res<()> {
    // fn sel(s: &str) -> Selector {
    //     Selector::Str(s)
    // }

    // let cstring = CString::new(".".to_string().into_bytes()).unwrap();

    // let folder = be(cell_str(cstring.as_ptr()).to_res()?, "file")?;
    // let folder = sub(&get(&sub(&folder)?, sel("examples"))?)?;
    // let jsonfile = get(&folder, sel("productiondump.json"))?;
    // let jsondata = be(jsonfile, "json")?;

    // let stacks_node = get(&sub(&jsondata)?, sel("stacks"))?;
    // let stacks = sub(&stacks_node)?;
    // for i in 0..len(&stacks) {
    //     let stack = match at(&stacks, i) {
    //         Ok(stack) => stack,
    //         Err(HErr::NotFound(_)) => break,
    //         Err(e) => return Err(e),
    //     };

    //     let syst = get(&sub(&stack)?, sel("system_stack"))?;
    //     if value(&syst)? == sel("true") {
    //         continue;
    //     }

    //     let dc_node = get(&sub(&stack)?, sel("dockerCompose"))?;
    //     let dc_value = value(&dc_node)?.to_owned_value();
    //     let yaml = be(cell_value(dc_value), "yaml")?;

    //     let services_node = get(&sub(&yaml)?, sel("services"))?;
    //     let services = sub(&services_node)?;
    //     let first_service = at(&services, 0)?;
    //     let name = label(&first_service)?;
    //     let mut filename = format!("{}.yaml", name);
    //     while get(&folder, sel(&filename)).is_ok() {
    //         filename = format!("{}.yaml", filename);
    //     }
    //     println!("filename: {}", filename);

    //     // let new_file = new(&folder, filename);
    //     // set(&be(new_file, "yaml")?, yaml);
    // }
    Ok(())
}
