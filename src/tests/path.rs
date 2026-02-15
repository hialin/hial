use crate::{
    api::*,
    prog::{Path, PathStart, path::*, url::*},
};

#[test]
fn test_parse_paths() -> Res<()> {
    {
        let (start, path) = Path::parse_with_starter("./src")?;
        assert_eq!(start, PathStart::File("./src".to_string()));
        assert_eq!(path.to_string(), "");

        let (start, path) = Path::parse_with_starter("./src^fs")?;
        assert_eq!(start, PathStart::File("./src".to_string()));
        assert_eq!(path.to_string(), "^fs");

        let (start, path) = Path::parse_with_starter("./src/x.json^json")?;
        assert_eq!(start, PathStart::File("./src/x.json".to_string()));
        assert_eq!(path.to_string(), "^json");

        let (start, path) = Path::parse_with_starter("src/x.json^json")?;
        assert_eq!(start, PathStart::File("src/x.json".to_string()));
        assert_eq!(path.to_string(), "^json");
    }

    {
        let (start, path) = Path::parse_with_starter("http://localhost")?;
        assert_eq!(start, PathStart::Url(Url::parse("http://localhost")?));
        assert_eq!(path.to_string(), "");

        let (start, path) = Path::parse_with_starter("http://localhost:2/")?;
        assert_eq!(start, PathStart::Url(Url::parse("http://localhost:2")?));
        assert_eq!(path.to_string(), "");

        let (start, path) = Path::parse_with_starter("mongo://localhost^mongo")?;
        assert_eq!(start.to_string(), "mongo://localhost");
        assert_eq!(path.to_string(), "^mongo");

        let (start, path) = Path::parse_with_starter("mongo://localhost:4^mongo")?;
        assert_eq!(start.to_string(), "mongo://localhost:4");
        assert_eq!(path.to_string(), "^mongo");

        let (start, path) = Path::parse_with_starter("mongo://localhost:4/^mongo")?;
        assert_eq!(start.to_string(), "mongo://localhost:4");
        assert_eq!(path.to_string(), "^mongo");

        let (start, path) = Path::parse_with_starter("mongo://localhost/app^mongo")?;
        assert_eq!(start.to_string(), "mongo://localhost/app");
        assert_eq!(path.to_string(), "^mongo");

        let (start, path) = Path::parse_with_starter("mongo://localhost:5/app^mongo")?;
        assert_eq!(start.to_string(), "mongo://localhost:5/app");
        assert_eq!(path.to_string(), "^mongo");
    }

    Ok(())
}

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
                    name: None,
                    value: "w".into(),
                }]
            }),
            PathItem::Elevation(ElevationPathItem {
                interpretation: Selector::Str("fs.two"),
                params: vec![InterpretationParam {
                    name: Some("w".to_string()),
                    value: OwnValue::Int(1.into()),
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

    let path = Path::parse("/a[buildVersion==dev]")?;
    assert_eq!(
        path.0.as_slice(),
        &[PathItem::Normal(NormalPathItem {
            relation: Relation::Sub,
            selector: Some(Selector::Str("a")),
            index: None,
            filters: vec![Filter {
                expr: Expression::Ternary {
                    left: Path(vec![PathItem::Normal(NormalPathItem {
                        relation: Relation::Field,
                        selector: Some(Selector::Str("buildVersion")),
                        index: None,
                        filters: vec![],
                    })]),
                    op_right: Some(("==", OwnValue::String("dev".to_string())))
                }
            }],
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
