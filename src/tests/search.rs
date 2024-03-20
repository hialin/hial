use crate::{
    api::*,
    pprint,
    prog::path::{
        ElevationPathItem, Expression, Filter, InterpretationParam, NormalPathItem, Path, PathItem,
    },
    utils::log::set_verbose,
};

#[test]
fn path_simple_elevation() -> Res<()> {
    let path = Path::parse(" ^fs^fs.one[w]^fs.two[w=1] ")?;
    assert_eq!(
        path.0.as_slice(),
        &[
            PathItem::Elevation(ElevationPathItem {
                interpretation: Selector::Str("fs"),
                params: vec![]
            }),
            PathItem::Elevation(ElevationPathItem {
                interpretation: Selector::Str("fs.one"),
                params: vec![InterpretationParam {
                    name: "w".to_string(),
                    value: None
                }]
            }),
            PathItem::Elevation(ElevationPathItem {
                interpretation: Selector::Str("fs.two"),
                params: vec![InterpretationParam {
                    name: "w".to_string(),
                    value: Some(OwnValue::Int(1.into())),
                }]
            })
        ]
    );
    Ok(())
}

// TODO: top elevation
// #[test]
// fn path_top_elevation() -> Res<()> {
//     let path = Path::parse("^[0]")?;
//     assert_eq!(
//         path.0.as_slice(),
//         &[PathItem::Elevation(ElevationPathItem {
//             interpretation: Selector::Str("fs"),
//             params: vec![]
//         }),]
//     );
//     Ok(())
// }

#[test]
fn path_simple_selector() -> Res<()> {
    let path = Path::parse("/a[2]")?;
    assert_eq!(
        path.0.as_slice(),
        &[PathItem::Normal(NormalPathItem {
            relation: Relation::Sub,
            selector: Some(Selector::Str("a")),
            index: Some(2),
            filters: vec![],
        })]
    );

    let path = Path::parse("/a[-2]")?;
    assert_eq!(
        path.0.as_slice(),
        &[PathItem::Normal(NormalPathItem {
            relation: Relation::Sub,
            selector: Some(Selector::Str("a")),
            index: Some(-2),
            filters: vec![],
        }),]
    );
    Ok(())
}

#[test]
fn path_simple_type_expr() -> Res<()> {
    let path = Path::parse("/a[:fn_item0]")?;
    assert_eq!(
        path.0.as_slice(),
        &[PathItem::Normal(NormalPathItem {
            relation: Relation::Sub,
            selector: Some(Selector::Str("a")),
            index: None,
            filters: vec![Filter {
                expr: Expression::Type {
                    ty: "fn_item0".to_string()
                }
            }],
        })]
    );
    Ok(())
}

#[test]
fn path_ternary_expr() -> Res<()> {
    let path = Path::parse("/a[@attr==1][/x]")?;
    assert_eq!(
        path.0.as_slice(),
        &[PathItem::Normal(NormalPathItem {
            relation: Relation::Sub,
            selector: Some(Selector::Str("a")),
            index: None,
            filters: vec![
                Filter {
                    expr: Expression::Ternary {
                        left: Path(vec![PathItem::Normal(NormalPathItem {
                            relation: Relation::Attr,
                            selector: Some(Selector::Str("attr")),
                            index: None,
                            filters: vec![],
                        })]),
                        op_right: Some(("==", OwnValue::Int(1.into())))
                    }
                },
                Filter {
                    expr: Expression::Ternary {
                        left: Path(vec![PathItem::Normal(NormalPathItem {
                            relation: Relation::Sub,
                            selector: Some(Selector::Str("x")),
                            index: None,
                            filters: vec![],
                        })]),
                        op_right: None,
                    }
                }
            ],
        })]
    );
    Ok(())
}

#[test]
fn path_simple_or_expr() -> Res<()> {
    let path = Path::parse("/a[:x|:y|:z]")?;
    assert_eq!(
        path.0.as_slice(),
        &[PathItem::Normal(NormalPathItem {
            relation: Relation::Sub,
            selector: Some(Selector::Str("a")),
            index: None,
            filters: vec![Filter {
                expr: Expression::Or {
                    expressions: vec![
                        Expression::Type {
                            ty: "x".to_string()
                        },
                        Expression::Type {
                            ty: "y".to_string()
                        },
                        Expression::Type {
                            ty: "z".to_string()
                        }
                    ]
                }
            }],
        })]
    );
    Ok(())
}

#[test]
fn path_items() -> Res<()> {
    let path = Path::parse("/a@name/[2]/*[#value=='3'][/x]")?;
    assert_eq!(
        path.0.as_slice(),
        &[
            PathItem::Normal(NormalPathItem {
                relation: Relation::Sub,
                selector: Some("a".into()),
                index: None,
                filters: vec![],
            }),
            PathItem::Normal(NormalPathItem {
                relation: Relation::Attr,
                selector: Some("name".into()),
                index: None,
                filters: vec![],
            }),
            PathItem::Normal(NormalPathItem {
                relation: Relation::Sub,
                selector: None,
                index: Some(2),
                filters: vec![],
            }),
            PathItem::Normal(NormalPathItem {
                relation: Relation::Sub,
                selector: Some(Selector::Star),
                index: None,
                filters: vec![
                    Filter {
                        expr: Expression::Ternary {
                            left: Path(vec![PathItem::Normal(NormalPathItem {
                                relation: Relation::Field,
                                selector: Some("value".into()),
                                index: None,
                                filters: vec![],
                            }),]),
                            op_right: Some(("==", OwnValue::String("3".to_string())))
                        }
                    },
                    Filter {
                        expr: Expression::Ternary {
                            left: Path(vec![PathItem::Normal(NormalPathItem {
                                relation: Relation::Sub,
                                selector: Some("x".into()),
                                index: None,
                                filters: vec![],
                            }),]),
                            op_right: None
                        }
                    }
                ],
            })
        ]
    );
    Ok(())
}

#[test]
fn search_simple_search() -> Res<()> {
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
    let root = Xell::from(TREE).be("yaml");

    let eval = str_eval(root.clone(), "/a/b/x")?;
    assert_eq!(eval, ["x:xb"]);

    let eval = str_eval(root.clone(), "/ a/b /x")?;
    assert_eq!(eval, ["x:xb"]);

    let eval = str_eval(root.clone(), "/ a/b /x")?;
    assert_eq!(eval, ["x:xb"]);

    Ok(())
}

#[test]
fn search_simple_search_with_index() -> Res<()> {
    const TREE: &str = r#"
    <test>
        <w>
            <x>1</x>
        </w>
        <w>
            <y>2</y>
        </w>
        <t/>
        <a>
            <x>1</x>
        </a>
        <a>
            <y>2</y>
        </a>
        <a>
            <z>3</z>
        </a>
        <s/>
    </test>
        "#;
    let root = Xell::from(TREE).be("xml");

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/test/a[0]/*")?;
    assert_eq!(eval, ["x:1"]);

    let eval = str_eval(root.clone(), "/test/a[1]/*")?;
    assert_eq!(eval, ["y:2"]);

    let eval = str_eval(root.clone(), "/test/a[2]/*")?;
    assert_eq!(eval, ["z:3"]);

    let eval = str_eval(root.clone(), "/test/a[-1]/*")?;
    assert_eq!(eval, ["z:3"]);

    let eval = str_eval(root.clone(), "/test/a[-2]/*")?;
    assert_eq!(eval, ["y:2"]);

    Ok(())
}

#[test]
fn search_kleene() -> Res<()> {
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
            o: null
        "#;
    let root = Xell::from(TREE).be("yaml");

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), pr("/*"))?;
    assert_eq!(eval, ["a:", "m:mval", "n:nval", "o:ø"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), pr("/*#label"))?;
    assert_eq!(eval, [":a", ":m", ":n", ":o"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), pr("/*[#label=='a']"))?;
    assert_eq!(eval, ["a:"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), pr("/*[#label=='a']#index"))?;
    assert_eq!(eval, [":0"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/*/x")?;
    assert_eq!(eval, ["x:xa"]);

    Ok(())
}

#[test]
fn search_filter() -> Res<()> {
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
    let root = Xell::from(TREE).be("yaml");
    let eval = str_eval(root.clone(), pr("/*[/x]"))?;
    assert_eq!(eval, ["a:"]);
    let eval = str_eval(root.clone(), pr("/a/*[/x]"))?;
    assert_eq!(eval, ["b:"]);
    let eval = str_eval(root.clone(), pr("/a/*[/x]#label"))?;
    assert_eq!(eval, [":b"]);
    Ok(())
}

#[test]
fn search_double_kleene_basic() -> Res<()> {
    const TREE_SIMPLE: &str = r#"
            a:
              x: xval
            m: mval
            n: nval
        "#;
    set_verbose(true);
    let root = Xell::from(TREE_SIMPLE).be("yaml");

    let path = "/**/m";
    println!("\npath: {}\n", path);
    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), path)?;
    assert_eq!(eval, ["m:mval"]);

    Ok(())
}

#[test]
fn search_double_kleene_simple() -> Res<()> {
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
    let root = Xell::from(TREE).be("yaml");

    //  doublestar should match on multiple levels
    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**/x")?;
    assert_eq!(eval, ["x:xa", "x:xb", "x:xc"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**/y")?;
    assert_eq!(eval, ["y:yc"]);

    //  doublestar should match even nothing at all
    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**/m")?;
    assert_eq!(eval, ["m:mval"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/a/**/b")?;
    assert_eq!(eval, ["b:"]);

    Ok(())
}

#[test]
fn search_double_kleene_top_filter() -> Res<()> {
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
    let root = Xell::from(TREE).be("yaml");

    let eval = str_eval(root.clone(), pr("/*[#label=='a']/**[=='xa']"))?;
    assert_eq!(eval, ["x:xa"]);

    let eval = str_eval(root.clone(), pr("/*[#label=='a']/**/x[=='xc']"))?;
    assert_eq!(eval, ["x:xc"]);
    Ok(())
}

#[test]
fn search_double_kleene_deep_filter() -> Res<()> {
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

    let root = Xell::from(TREE).be("yaml");
    let eval = str_eval(root.clone(), "/**/*[#label=='x']")?;
    assert_eq!(eval, ["x:xa", "x:xb", "x:xc"]);
    let eval = str_eval(root.clone(), "/a/**[#label!='x']/y")?;
    assert_eq!(eval, ["y:yc"]);
    let eval = str_eval(root.clone(), "/a/**/*[=='xb']")?;
    assert_eq!(eval, ["x:xb"]);
    Ok(())
}

#[test]
fn search_double_kleene_all() -> Res<()> {
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

    let root = Xell::from(TREE).be("yaml");

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**")?;
    assert_eq!(
        eval,
        [
            ":", "a:", "x:xa", "b:", "x:xb", "c:", "x:xc", "y:yc", "z:", ":r", ":s", "m:mval",
            "n:nval"
        ]
    );

    Ok(())
}

#[test]
fn search_double_kleene_labels_json() -> Res<()> {
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

    let root = Xell::from(TREE).be("json");

    // crate::pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**#label")?;
    assert_eq!(
        eval,
        [":a", ":x", ":b", ":x", ":c", ":x", ":y", ":z", ":m", ":n"]
    );

    Ok(())
}

#[test]
fn search_double_kleene_labels_yaml() -> Res<()> {
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

    let root = Xell::from(TREE).be("yaml");

    // crate::pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**#label")?;
    assert_eq!(
        eval,
        [":a", ":x", ":b", ":x", ":c", ":x", ":y", ":z", ":m", ":n"]
    );

    Ok(())
}

#[test]
fn search_double_kleene_repeat() -> Res<()> {
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
    let root = Xell::from(TREE).be("yaml");

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**/b/b")?;
    assert_eq!(eval, ["b:", "b:bval"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/**/b/**/b")?;
    assert_eq!(eval, ["b:", "b:bval", "b:bval"]); // two ways to reach node b:bval

    Ok(())
}

#[test]
fn search_double_kleene_with_filter() -> Res<()> {
    const TREE: &str = r#"
        dir1:
            f1:
                size: null
            dir2:
                size: null
                f2:
                    size: 2
            dir3:
                f3:
                    size: 3
        "#;
    set_verbose(true);
    let root = Xell::from(TREE).be("yaml");

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/dir1/**")?;
    assert_eq!(
        eval,
        ["dir1:", "f1:", "size:ø", "dir2:", "size:ø", "f2:", "size:2", "dir3:", "f3:", "size:3"]
    );

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/dir1/**[/size]")?;
    assert_eq!(eval, ["f1:", "dir2:", "f2:"]);

    pprint(&root, 0, 0);
    let eval = str_eval(root.clone(), "/dir1/**/*[/size]")?;
    assert_eq!(eval, ["f1:", "dir2:", "f2:", "f3:"]);

    Ok(())
}

pub fn str_eval(root: Xell, path: &str) -> Res<Vec<String>> {
    root.all(path)?
        .into_iter()
        .map(|cell| -> Res<String> {
            // if let Ok(ref cell) = cres {
            //     if let Ok(path) = cell.path() {
            //         println!("--> found path: {}", path);
            //     }
            // }
            Ok(cell.err()?.debug_string())
        })
        .collect::<Res<Vec<_>>>()
}

pub fn pr<T: std::fmt::Debug>(x: T) -> T {
    // println!("\npr: {:?}", x);
    x
}
