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

    let cell = Cell::from("./examples/write.json");
    pprint(&cell, 0, 0);
    // let cell = cell.to("^path^fs[w]^json");
    // pprint(&cell, 0, 0);
    // assert!(cell.clone().err().is_ok());
    // assert!(cell.write().value("weak as putty".into()).is_ok());

    // let cell = Cell::from(".")
    //     .to("^path^fs[w]/examples/productiondump.json")
    //     .to("^json/stacks/*/dockerCompose")
    //     .to("^yaml/services/scheduler/image")
    //     .to("^split(':')/[-1]")
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
    //     .value("0.8.8".into())?;

    // assert_eq!(
    //     cell.origin().to(cell.path()?.as_str()).read().value()?,
    //     "0.8.8"
    // );

    Ok(())
}
