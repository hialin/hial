use crate::api::*;
use std::fs;

#[test]
fn text_read_lines() -> Res<()> {
    let text = Xell::from("one\ntwo\nthree").be("text");

    assert_eq!(text.read().ty()?, "document");
    assert_eq!(text.sub().len()?, 3);
    assert_eq!(text.to("/[0]").read().value()?, "one");
    assert_eq!(text.to("/[1]").read().value()?, "two");
    assert_eq!(text.to("/[-1]").read().value()?, "three");

    Ok(())
}

#[test]
fn text_write_and_save_preserves_newline_style() -> Res<()> {
    let raw = Xell::from("one\r\ntwo\r\n").policy(WritePolicy::NoAutoWrite);
    let text = raw.be("text");

    text.to("/[1]").write().value("changed")?;
    assert_eq!(text.to("/[1]").read().value()?, "changed");

    text.save(&text.origin())?;

    assert_eq!(raw.read().value()?, "one\r\nchanged\r\n");
    Ok(())
}

#[test]
fn text_file_roundtrip() -> Res<()> {
    let path = "./src/tests/data/text_roundtrip.txt";
    fs::write(path, "alpha\nbeta").map_err(|e| caused(HErrKind::IO, "cannot seed text file", e))?;

    let file = Xell::new("./src/tests/data/text_roundtrip.txt^fs[w]^text").err()?;
    file.to("/[0]").write().value("omega")?;
    file.save(&file.origin())?;

    assert_eq!(
        Xell::new("./src/tests/data/text_roundtrip.txt^text/[0]")
            .read()
            .value()?,
        "omega"
    );

    fs::write(path, "alpha\nbeta")
        .map_err(|e| caused(HErrKind::IO, "cannot restore text file", e))?;
    Ok(())
}

#[test]
fn text_auto_interpretation_for_txt() -> Res<()> {
    let path = "./src/tests/data/text_autodetect.txt";
    fs::write(path, "left\nright")
        .map_err(|e| caused(HErrKind::IO, "cannot seed autodetect text file", e))?;

    let cell = Xell::from(".")
        .be("path")
        .be("fs")
        .to("/src/tests/data/text_autodetect.txt^/[1]");
    assert_eq!(cell.read().value()?, "right");

    fs::remove_file(path)
        .map_err(|e| caused(HErrKind::IO, "cannot cleanup autodetect text file", e))?;
    Ok(())
}

#[test]
fn text_empty_document_has_no_lines() -> Res<()> {
    let text = Xell::from("").be("text");
    assert_eq!(text.sub().len()?, 0);
    assert_eq!(text.read().serial()?, "");
    Ok(())
}
