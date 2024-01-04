use crate::{
    base::*,
    pathlang::path::{Expression, Filter, Path, PathItem},
    utils::log::set_verbose,
};

use super::utils::*;

#[test]
fn path_items() -> Res<()> {
    let path = Path::parse("/a@name/[2]/*[#value=='3'][/x]")?;
    assert_eq!(
        path.0.as_slice(),
        &[
            PathItem {
                relation: Relation::Sub,
                selector: Some("a".into()),
                index: None,
                filters: vec![],
            },
            PathItem {
                relation: Relation::Attr,
                selector: Some("name".into()),
                index: None,
                filters: vec![],
            },
            PathItem {
                relation: Relation::Sub,
                selector: None,
                index: Some(2),
                filters: vec![],
            },
            PathItem {
                relation: Relation::Sub,
                selector: Some(Selector::Star),
                index: None,
                filters: vec![
                    Filter {
                        expr: Expression {
                            left: Path(vec![PathItem {
                                relation: Relation::Field,
                                selector: Some("value".into()),
                                index: None,
                                filters: vec![],
                            },]),
                            op: Some("=="),
                            right: Some(Value::Str("3"))
                        }
                    },
                    Filter {
                        expr: Expression {
                            left: Path(vec![PathItem {
                                relation: Relation::Sub,
                                selector: Some("x".into()),
                                index: None,
                                filters: vec![],
                            },]),
                            op: None,
                            right: None,
                        }
                    }
                ],
            }
        ]
    );
    Ok(())
}

#[test]
fn path_simple_search() -> Res<()> {
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
    let root = Cell::from(TREE.to_string()).be("yaml")?;
    let eval = str_eval(root.clone(), "/a/b/x")?;
    assert_eq!(eval, ["x:xb"]);
    let eval = str_eval(root.clone(), "/ a/b /x")?;
    assert_eq!(eval, ["x:xb"]);
    Ok(())
}

#[test]
fn path_kleene() -> Res<()> {
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
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    let eval = str_eval(root.clone(), pr("/*"))?;
    assert_eq!(eval, ["a:ø", "m:mval", "n:nval"]);

    let eval = str_eval(root.clone(), pr("/*#label"))?;
    assert_eq!(eval, [":a", ":m", ":n"]);

    let eval = str_eval(root.clone(), pr("/*[#label=='a']"))?;
    assert_eq!(eval, ["a:ø"]);

    let eval = str_eval(root.clone(), pr("/*[#label=='a']#index"))?;
    assert_eq!(eval, [":0"]);

    let eval = str_eval(root.clone(), "/*/x")?;
    assert_eq!(eval, ["x:xa"]);

    Ok(())
}

#[test]
fn path_filter() -> Res<()> {
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
    let root = Cell::from(TREE.to_string()).be("yaml")?;
    let eval = str_eval(root.clone(), pr("/*[/x]"))?;
    assert_eq!(eval, ["a:ø"]);
    let eval = str_eval(root.clone(), pr("/a/*[/x]"))?;
    assert_eq!(eval, ["b:ø"]);
    let eval = str_eval(root.clone(), pr("/a/*[/x]#label"))?;
    assert_eq!(eval, [":b"]);
    Ok(())
}

#[test]
fn path_double_kleene_simple() -> Res<()> {
    const TREE_SIMPLE: &str = r#"
            a:
              x: xval
            m: mval
            n: nval
        "#;
    set_verbose(true);
    let root = Cell::from(TREE_SIMPLE.to_string()).be("yaml")?;

    crate::pprint::pprint(&root, 0, 0);

    let path = "/**/m";
    println!("\npath: {}\n", path);
    let eval = str_eval(root.clone(), path)?;
    assert_eq!(eval, ["m:mval"]);

    Ok(())
}

#[test]
fn path_double_kleene() -> Res<()> {
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

    set_verbose(true);
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    //  doublestar should match on multiple levels
    let eval = str_eval(root.clone(), "/**/x")?;
    assert_eq!(eval, ["x:xa", "x:xb", "x:xc"]);

    let eval = str_eval(root.clone(), "/**/y")?;
    assert_eq!(eval, ["y:yc"]);

    //  doublestar should match even nothing at all
    let eval = str_eval(root.clone(), "/**/m")?;
    assert_eq!(eval, ["m:mval"]);

    let eval = str_eval(root.clone(), "/a/**/b")?;
    assert_eq!(eval, ["b:ø"]);

    Ok(())
}

#[test]
fn path_double_kleene_top_filter() -> Res<()> {
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

    set_verbose(true);
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    let eval = str_eval(root.clone(), pr("/*[#label=='a']/**[=='xa']"))?;
    assert_eq!(eval, ["x:xa"]);

    let eval = str_eval(root.clone(), pr("/*[#label=='a']/**/x[=='xc']"))?;
    assert_eq!(eval, ["x:xc"]);
    Ok(())
}

#[test]
fn path_double_kleene_deep_filter() -> Res<()> {
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

    let root = Cell::from(TREE.to_string()).be("yaml")?;
    let eval = str_eval(root.clone(), "/**/*[#label=='x']")?;
    assert_eq!(eval, ["x:xa", "x:xb", "x:xc"]);
    let eval = str_eval(root.clone(), "/a/**[#label!='x']/y")?;
    assert_eq!(eval, ["y:yc"]);
    let eval = str_eval(root.clone(), "/a/**/*[=='xb']")?;
    assert_eq!(eval, ["x:xb"]);
    Ok(())
}

#[test]
fn path_double_kleene_all() -> Res<()> {
    set_verbose(true);
    const TREE: &str = r#"
            a:
              x: xa
              b:
                x: xb
                c:
                    x: xc
                    y: yc
                    z: [r, s]
            m: mval
            n: nval
        "#;

    let root = Cell::from(TREE.to_string()).be("yaml")?;

    let eval = str_eval(root.clone(), "/**")?;
    assert_eq!(
        eval,
        [
            ":ø", "a:ø", "x:xa", "b:ø", "x:xb", "c:ø", "x:xc", "y:yc", "z:ø", ":r", ":s", "m:mval",
            "n:nval"
        ]
    );

    Ok(())
}

#[test]
fn path_double_kleene_labels_json() -> Res<()> {
    set_verbose(true);
    const TREE: &str = r#"{
        "a": {
          "x": "xa",
          "b": {
            "x": "xb",
            "c": {
                "x": "xc",
                "y": "yc",
                "z": ["r", "s"]
            }
          }
        },
        "m": "mval",
        "n": "nval"
    }"#;

    let root = Cell::from(TREE).be("json")?;

    // crate::pprint::pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**#label")?;
    assert_eq!(
        eval,
        [":a", ":x", ":b", ":x", ":c", ":x", ":y", ":z", ":m", ":n"]
    );

    Ok(())
}

#[test]
fn path_double_kleene_labels_yaml() -> Res<()> {
    set_verbose(true);
    const TREE: &str = r#"
        a:
          x: xa
          b:
            x: xb
            c:
                x: xc
                y: yc
                z: [r, s]
        m: mval
        n: nval
    "#;

    let root = Cell::from(TREE).be("yaml")?;

    // crate::pprint::pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**#label")?;
    assert_eq!(
        eval,
        [":a", ":x", ":b", ":x", ":c", ":x", ":y", ":z", ":m", ":n"]
    );

    Ok(())
}

#[test]
fn path_double_kleene_repeat() -> Res<()> {
    const TREE: &str = r#"
            a:
              x: xa
              b:
                x: xb
                b:
                    x: xc
                    b: bval
            m: mval
            n: nval
        "#;
    set_verbose(true);
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    // crate::pprint::pprint(&root, 0, 0);
    // println!("\npath: {}\n", "/**/b/b");

    let eval = str_eval(root.clone(), "/**/b/b")?;
    assert_eq!(eval, ["b:ø", "b:bval"]);

    let eval = str_eval(root.clone(), "/**/b/**/b")?;
    assert_eq!(eval, ["b:ø", "b:bval"]);

    Ok(())
}
