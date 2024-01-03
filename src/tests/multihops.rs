use crate::{base::*, utils::log::set_verbose};

#[test]
fn test_multihop_trace() -> Res<()> {
    set_verbose(true);

    let start = Cell::from("http://api.github.com");
    let path = "^url^http^json/rate_limit_url#value^url^http^json/resources/core/limit";

    let results = start.search(path)?.all()?;
    assert_eq!(results.len(), 1);
    let result = &results[0];

    assert_eq!(result.read()?.value()?, Value::from(60));

    // TODO: implement path() without Box<Cell> parent
    // How do we implement a path() method for a multihop without keeping
    //  the parent cell in a box, which makes an allocation for every cell?
    // This current result is incorrect, and should be fixed.
    assert_eq!(result.path()?, "limit");

    Ok(())
}
