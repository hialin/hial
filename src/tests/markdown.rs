use crate::api::*;
use std::fs;

#[test]
fn markdown_builds_heading_hierarchy_with_block_children() -> Res<()> {
    let md = Xell::from(
        "# One\n\nalpha\n\n```rs\nlet x = 1;\n```\n\n## Two\n\nbeta\n\n### Three\n\ngamma\n\n# Four\n\ndelta\n",
    )
    .be("markdown");

    assert_eq!(md.read().ty()?, "document");
    assert_eq!(md.sub().len()?, 2);

    let one = md.to("/One");
    assert_eq!(one.read().ty()?, "title");
    assert_eq!(one.read().label()?, "One");
    assert!(one.read().value().is_err());
    assert_eq!(one.sub().len()?, 3);

    let one_text = one.sub().at(0);
    assert_eq!(one_text.read().ty()?, "text");
    assert_eq!(one_text.read().value()?, "alpha");

    let one_code = one.sub().at(1);
    assert_eq!(one_code.read().ty()?, "code");
    assert_eq!(one_code.read().value()?, "let x = 1;");

    let two = one.to("/Two");
    assert_eq!(two.read().label()?, "Two");
    assert_eq!(two.sub().len()?, 2);
    assert_eq!(two.sub().at(0).read().ty()?, "text");
    assert_eq!(two.sub().at(0).read().value()?, "beta");

    let three = two.to("/Three");
    assert_eq!(three.read().label()?, "Three");
    assert_eq!(three.sub().len()?, 1);
    assert_eq!(three.sub().at(0).read().value()?, "gamma");

    let four = md.to("/Four");
    assert_eq!(four.read().label()?, "Four");
    assert_eq!(four.sub().len()?, 1);
    assert_eq!(four.sub().at(0).read().value()?, "delta");

    Ok(())
}

#[test]
fn markdown_exposes_preamble_blocks() -> Res<()> {
    let md = Xell::from("intro line\n\n```txt\npre\n```\n\n# Heading\n\nbody\n").be("markdown");
    let preamble = md.to("/preamble");

    assert_eq!(preamble.read().ty()?, "preamble");
    assert!(preamble.read().value().is_err());
    assert_eq!(preamble.sub().len()?, 2);
    assert_eq!(preamble.sub().at(0).read().ty()?, "text");
    assert_eq!(preamble.sub().at(0).read().value()?, "intro line");
    assert_eq!(preamble.sub().at(1).read().ty()?, "code");
    assert_eq!(preamble.sub().at(1).read().value()?, "pre");

    let heading = md.to("/Heading");
    assert_eq!(heading.sub().at(0).read().value()?, "body");

    Ok(())
}

#[test]
fn markdown_skips_empty_headings() -> Res<()> {
    let md = Xell::from("# \n\nalpha\n\n## Named\n\nbeta\n").be("markdown");

    assert_eq!(md.sub().len()?, 2);

    let preamble = md.to("/preamble");
    assert_eq!(preamble.sub().at(0).read().value()?, "alpha");

    let named = md.to("/Named");
    assert_eq!(named.sub().at(0).read().value()?, "beta");

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
    assert_eq!(cell.sub().at(0).read().value()?, "world");

    fs::remove_file(path)
        .map_err(|e| caused(HErrKind::IO, "cannot cleanup markdown test file", e))?;
    Ok(())
}
