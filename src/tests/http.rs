use crate::{api::*, pprint, utils::log::set_verbose};
use std::io::Read;

#[test]
fn test_http_basic() -> Res<()> {
    set_verbose(true);

    let cell = Xell::new("http://api.github.com^http");
    pprint(&cell, 0, 0);
    assert_eq!(cell.to("@status/code").read().value()?, Value::from(200));
    assert_eq!(cell.read().value()?, Value::Bytes);
    let mut bytes = Vec::new();
    cell.read().value_read()?.read_to_end(&mut bytes).map_err(|e| {
        caused(HErrKind::IO, "cannot read http body", e)
    })?;
    assert!(bytes.len() > 10);

    let cell = Xell::new("http://api.github.com^http[HEAD]");
    pprint(&cell, 0, 0);
    assert_eq!(cell.to("@status/code").read().value()?, Value::from(200));
    assert_eq!(cell.read().value()?, Value::Bytes);
    let mut bytes = Vec::new();
    cell.read().value_read()?.read_to_end(&mut bytes).map_err(|e| {
        caused(HErrKind::IO, "cannot read http body", e)
    })?;
    assert!(bytes.is_empty());

    Ok(())
}
