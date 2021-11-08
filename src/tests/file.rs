use crate::pathlang::path::Path;
use crate::rust_api::*;
use crate::*;

#[test]
fn test_files() -> Res<()> {
    let examples = Cell::from(".".to_string())
        .be("file")?
        .sub()?
        .get("examples")?;
    assert_eq!(std::mem::size_of_val(&examples), 4 * 8);
    assert_eq!(examples.label()?, "examples");
    assert_eq!(examples.value()?, "examples");
    Ok(())
}

#[test]
fn test_path_with_starter() -> Res<()> {
    let path = "./LICENSE@size";
    let (root, path) = Path::parse_with_starter(path)?;
    let eval = path
        .eval(root.eval()?)
        .map(|c| Ok(c?.value()?.to_string()))
        .collect::<Res<Vec<_>>>()?;
    assert_eq!(eval, ["26526"]);
    Ok(())
}
