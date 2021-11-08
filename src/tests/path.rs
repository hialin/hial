use super::utils::*;
use crate::base::common::DISPLAY_VALUE_NONE as NOVAL;
use crate::pathlang::path::{Expression, Filter, Path, PathItem, Relation};
use crate::rust_api::*;
use crate::set_verbose;
use crate::*;

#[test]
fn test_path_items() -> Res<()> {
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

#[test]
fn test_path_simple_search() -> Res<()> {
    let root = Cell::from(TREE.to_string()).be("yaml")?;
    let eval = str_eval(root.clone(), "/a/b/x")?;
    assert_eq!(eval, ["xb"]);
    let eval = str_eval(root.clone(), "/ a/b /x")?;
    assert_eq!(eval, ["xb"]);
    Ok(())
}

#[test]
fn test_path_kleene() -> Res<()> {
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    let eval = str_eval(root.clone(), pr("/*"))?;
    assert_eq!(eval, [NOVAL, "mval", "nval"]);

    let eval = str_eval(root.clone(), pr("/*#label"))?;
    assert_eq!(eval, ["a", "m", "n"]);

    let eval = str_eval(root.clone(), pr("/*[#label=='a']"))?;
    assert_eq!(eval, [NOVAL]);

    let eval = str_eval(root.clone(), pr("/*[#label=='a']#index"))?;
    assert_eq!(eval, ["0"]);

    let eval = str_eval(root.clone(), "/*/x")?;
    assert_eq!(eval, ["xa"]);

    Ok(())
}

#[test]
fn test_path_filter() -> Res<()> {
    let root = Cell::from(TREE.to_string()).be("yaml")?;
    let eval = str_eval(root.clone(), pr("/*[/x]"))?;
    assert_eq!(eval, [NOVAL]);
    let eval = str_eval(root.clone(), pr("/a/*[/x]"))?;
    assert_eq!(eval, [NOVAL]);
    let eval = str_eval(root.clone(), pr("/a/*[/x]#label"))?;
    assert_eq!(eval, ["b"]);
    Ok(())
}

#[test]
fn test_path_double_kleene() -> Res<()> {
    set_verbose(true);
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    let eval = str_eval(root.clone(), "/**/x")?;
    assert_eq!(eval, ["xa", "xb", "xc"]);

    let eval = str_eval(root.clone(), "/**/y")?;
    assert_eq!(eval, ["yc"]);

    Ok(())
}

#[test]
fn test_path_double_kleene_all() -> Res<()> {
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    let eval = str_eval(root.clone(), "/**")?;
    assert_eq!(
        eval,
        [NOVAL, "xa", NOVAL, "xb", NOVAL, "xc", "yc", "mval", "nval"]
    );

    let eval = str_eval(root.clone(), "/**#label")?;
    assert_eq!(eval, ["a", "x", "b", "x", "c", "x", "y", "m", "n"]);

    Ok(())
}

#[test]
fn test_path_double_kleene_top_filter() -> Res<()> {
    set_verbose(true);
    let root = Cell::from(TREE.to_string()).be("yaml")?;

    let eval = str_eval(root.clone(), pr("/*[#label=='a']/**[=='xa']"))?;
    assert_eq!(eval, ["xa"]);

    let eval = str_eval(root.clone(), pr("/*[#label=='a']/**/x[=='xc']"))?;
    assert_eq!(eval, ["xc"]);
    Ok(())
}

#[test]
fn test_path_double_kleene_deep_filter() -> Res<()> {
    let root = Cell::from(TREE.to_string()).be("yaml")?;
    let eval = str_eval(root.clone(), "/**/*[#label=='x']")?;
    assert_eq!(eval, ["xa", "xb", "xc"]);
    let eval = str_eval(root.clone(), "/a/**[#label!='x']/y")?;
    assert_eq!(eval, ["yc"]);
    let eval = str_eval(root.clone(), "/a/**/*[=='xb']")?;
    assert_eq!(eval, ["xb"]);
    Ok(())
}
