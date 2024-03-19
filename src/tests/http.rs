use crate::{api::*, pprint, utils::log::set_verbose};

#[test]
fn test_http_basic() -> Res<()> {
    set_verbose(true);

    let cell = Xell::new("http://api.github.com^http");
    pprint(&cell, 0, 0);
    assert_eq!(cell.to("@status/code").read().value()?, Value::from(200));
    assert!(cell.read().value()?.as_cow_str().len() > 10);

    let cell = Xell::new("http://api.github.com^http[HEAD]");
    pprint(&cell, 0, 0);
    assert_eq!(cell.to("@status/code").read().value()?, Value::from(200));
    assert!(cell.read().value()?.as_cow_str().len() == 0);

    Ok(())
}
