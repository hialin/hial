use crate::{base::*, utils::log::set_verbose};

#[test]
fn test_multihop() -> Res<()> {
    set_verbose(true);

    let start = Cell::from("http://api.github.com");
    let path = "^http^json/rate_limit_url#value^http^json/resources/core";

    let results = start.search(path)?.all()?;
    assert_eq!(results.len(), 1);
    let result = &results[0];

    assert_eq!(result.path()?, "http://api.github.com/");
    assert_eq!(result.read()?.value()?, "1");

    Ok(())
}
