use crate::base::*;

#[test]
fn simple_path() -> Res<()> {
    const TREE: &str = r#"
            a:
              x: xa
              b:
                x: xb
                c:
                    x: xc
                    y: yc
            m: mval
            n: nval
        "#;
    let root = Cell::from(TREE).be("yaml");
    let x = root.to("/a/b/c/x");
    assert_eq!(x.path()?, "\"\\n            a:...\"^yaml/a/b/c/x");
    Ok(())
}
