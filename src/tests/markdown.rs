use crate::api::*;
use std::fs;

#[test]
fn markdown_builds_heading_hierarchy() -> Res<()> {
    let md = Xell::from(
        "# One\n\nalpha\n\n## Two\n\nbeta\n\n### Three\n\ngamma\n\n# Four\n\ndelta\n",
    )
    .be("markdown");

    assert_eq!(md.read().ty()?, "document");
    assert_eq!(md.sub().len()?, 2);

    let one = md.to("/One");
    assert_eq!(one.read().ty()?, "section");
    assert_eq!(one.read().label()?, "One");
    assert_eq!(one.read().value()?, "alpha");
    assert_eq!(one.to("@level").read().value()?, Int::from(1));

    let two = one.to("/Two");
    assert_eq!(two.read().label()?, "Two");
    assert_eq!(two.read().value()?, "beta");
    assert_eq!(two.to("@level").read().value()?, Int::from(2));

    let three = two.to("/Three");
    assert_eq!(three.read().label()?, "Three");
    assert_eq!(three.read().value()?, "gamma");

    let four = md.to("/Four");
    assert_eq!(four.read().label()?, "Four");
    assert_eq!(four.read().value()?, "delta");

    Ok(())
}

#[test]
fn markdown_exposes_preamble() -> Res<()> {
    let md = Xell::from("intro line\n\n# Heading\n\nbody\n").be("markdown");
    let preamble = md.to("/preamble");

    assert_eq!(preamble.read().ty()?, "preamble");
    assert_eq!(preamble.read().value()?, "intro line");
    assert_eq!(md.to("/Heading").read().value()?, "body");

    Ok(())
}

#[test]
fn markdown_auto_interpretation_for_md() -> Res<()> {
    let path = "./src/tests/data/markdown_autodetect.md";
    fs::write(path, "# Hello\n\nworld\n")
        .map_err(|e| caused(HErrKind::IO, "cannot seed markdown test file", e))?;

    let cell = Xell::from(".")
        .be("path")
        .be("fs")
        .to("/src/tests/data/markdown_autodetect.md^/Hello");
    assert_eq!(cell.read().label()?, "Hello");
    assert_eq!(cell.read().value()?, "world");

    fs::remove_file(path)
        .map_err(|e| caused(HErrKind::IO, "cannot cleanup markdown test file", e))?;
    Ok(())
}
