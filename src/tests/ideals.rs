use crate::{base::*, pprint::pprint, utils::log::set_verbose};

const TREE: &str = r#"
a:
  x: xa
  b:
    x: xb
    b:
        x: xc
        b: bval
m: mval
n: nval
"#;

#[test]
fn tree() -> Res<()> {
    set_verbose(true);

    // this should set the json value on the cell
    // and write back by doing a put request
    let cell = Cell::from("./examples/write.json")
        .policy(WritePolicy::WriteBackOnDrop)
        .to("^path^fs^json");
    pprint(&cell, 0, 0);
    assert!(cell.clone().err().is_ok());
    // assert!(cell.write().set_value("weak as putty".into()).is_ok());

    // let cell = Cell::from(".")
    //     .policy(WritePolicy::WriteBackOnDrop)
    //     .to("^path^fs/examples/productiondump.json")
    //     .to("^json/stacks/*/dockerCompose")
    //     .to("^docker.compose/services/scheduler/image")
    //     .to("^docker.imagetag/tag")
    //     .err()?;

    // assert_eq!(
    //     cell.origin().to(cell.path()?.as_str()).read().value()?,
    //     "0.8.6"
    // );

    // // this should set the docker image tag specified in the docker compose string
    // // embedded in the json from the productiondump json file
    // cell.origin()
    //     .policy(WritePolicy::WriteBackOnDrop)
    //     .to(cell.path()?.as_str())
    //     .write()
    //     .set_value("0.8.8".into())?;

    // assert_eq!(
    //     cell.origin().to(cell.path()?.as_str()).read().value()?,
    //     "0.8.8"
    // );

    Ok(())
}
