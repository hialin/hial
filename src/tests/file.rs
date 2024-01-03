use crate::base::*;
use crate::pathlang::path::Path;

#[test]
fn test_files() -> Res<()> {
    crate::utils::log::set_verbose(true);
    println!("{:?}", Cell::from(".").be("path")?.be("file"));
    for x in Cell::from(".").be("path")?.be("file")?.sub()? {
        println!("{:?}", x?.read()?.label());
    }
    let examples = Cell::from(".")
        .be("path")?
        .be("file")?
        .sub()?
        .get("examples")?;
    // assert_eq!(std::mem::size_of_val(&examples), 4 * 8); // todo file cell is too large
    assert_eq!(examples.read()?.label(), Ok("examples".into()));
    assert_eq!(examples.read()?.value(), Ok("examples".into()));
    Ok(())
}

#[test]
fn test_path_with_starter() -> Res<()> {
    let path = "./LICENSE@size";
    let (root, path) = Path::parse_with_starter(path)?;
    let eval = path
        .eval(root.eval()?)
        .map(|c| Ok(c?.read()?.value()?.to_string()))
        .collect::<Res<Vec<_>>>()?;
    assert_eq!(eval, ["26526"]);
    Ok(())
}
