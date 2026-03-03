use crate::{api::*, config::ColorPalette, pprint, utils::log::set_verbose};
use std::io::Read;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[test]
fn test_http_basic() -> Res<()> {
    set_verbose(true);
    if !can_reach("api.github.com", 80, Duration::from_secs(2)) {
        eprintln!("warning: skipping test_http_basic, network unavailable");
        return Ok(());
    }

    let cell = Xell::new("http://api.github.com^http");
    pprint(&cell, 0, 0, ColorPalette::None);
    assert_eq!(cell.to("@status/code").read().value()?, Value::from(200));
    assert_eq!(cell.read().value()?, Value::Bytes);
    let mut bytes = Vec::new();
    cell.read()
        .value_read()?
        .read_to_end(&mut bytes)
        .map_err(|e| caused(HErrKind::IO, "cannot read http body", e))?;
    assert!(bytes.len() > 10);

    let cell = Xell::new("http://api.github.com^http[HEAD]");
    pprint(&cell, 0, 0, ColorPalette::None);
    assert_eq!(cell.to("@status/code").read().value()?, Value::from(200));
    assert_eq!(cell.read().value()?, Value::Bytes);
    let mut bytes = Vec::new();
    cell.read()
        .value_read()?
        .read_to_end(&mut bytes)
        .map_err(|e| caused(HErrKind::IO, "cannot read http body", e))?;
    assert!(bytes.is_empty());

    Ok(())
}

fn can_reach(host: &str, port: u16, timeout: Duration) -> bool {
    let addrs = format!("{host}:{port}");
    let resolved: Vec<SocketAddr> = match addrs.to_socket_addrs() {
        Ok(iter) => iter.collect(),
        Err(_) => return false,
    };
    resolved
        .iter()
        .any(|addr| TcpStream::connect_timeout(addr, timeout).is_ok())
}
