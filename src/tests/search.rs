use crate::{api::*, config::ColorPalette, pprint, utils::log::set_verbose};

#[test]
fn test_simple_path() -> Res<()> {
    const TREE: &str = r#"
            a:
              x: xa
              b:
                x: xb
                c:
                    x: xc
                    y: yc
        "#;
    let root = Xell::from(TREE).be("yaml");
    let x = root.to("/a/b/c/x");
    assert_eq!(x.path()?, "`\\n            a:\\n …`^yaml/a/b/c/x");
    Ok(())
}

#[test]
fn test_multihop_path() -> Res<()> {
    set_verbose(true);

    let start = Xell::from("http://api.github.com");
    let path = "^http^json/rate_limit_url^http^json/resources/core/limit";

    let results = start.all(path)?;
    assert_eq!(results.len(), 1);
    let result = &results[0];

    assert_eq!(result.read().value()?, Value::from(60));

    assert_eq!(
        result.path()?,
        "`http://api.github.c…`^http^json/rate_limit_url^http^json/resources/core/limit"
            .to_string()
    );

    Ok(())
}

#[test]
fn search_empty_elevation_uses_auto_interpretation() -> Res<()> {
    let root = Xell::from(".").be("path").be("fs");
    let cell = root.to("/src/tests/data/assignment.json^/a");
    assert_eq!(cell.read().value()?, Int::from(1));
    assert_eq!(
        cell.path()?,
        "`.`^path^fs/src/tests/data/assignment.json^json/a"
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

    pprint(&root, 0, 0, ColorPalette::None);
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

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), pr("/*"))?;
    assert_eq!(eval, ["a:", "m:mval", "n:nval", "o:ø"]);

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), pr("/*#label"))?;
    assert_eq!(eval, [":a", ":m", ":n", ":o"]);

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), pr("/*[#label=='a']"))?;
    assert_eq!(eval, ["a:"]);

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), pr("/*[#label=='a']#index"))?;
    assert_eq!(eval, [":0"]);

    pprint(&root, 0, 0, ColorPalette::None);
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
    pprint(&root, 0, 0, ColorPalette::None);
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
    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), "/**/x")?;
    assert_eq!(eval, ["x:xa", "x:xb", "x:xc"]);

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), "/**/y")?;
    assert_eq!(eval, ["y:yc"]);

    //  doublestar should match even nothing at all
    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), "/**/m")?;
    assert_eq!(eval, ["m:mval"]);

    pprint(&root, 0, 0, ColorPalette::None);
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

    pprint(&root, 0, 0, ColorPalette::None);
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

    // crate::pprint(&root, 0, 0, ColorPalette::None);
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

    // crate::pprint(&root, 0, 0, ColorPalette::None);
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

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), "/**/b/b")?;
    assert_eq!(eval, ["b:", "b:bval"]);

    pprint(&root, 0, 0, ColorPalette::None);
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

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), "/dir1/**")?;
    assert_eq!(
        eval,
        [
            "dir1:", "f1:", "size:ø", "dir2:", "size:ø", "f2:", "size:2", "dir3:", "f3:", "size:3"
        ]
    );

    pprint(&root, 0, 0, ColorPalette::None);
    let eval = str_eval(root.clone(), "/dir1/**[/size]")?;
    assert_eq!(eval, ["f1:", "dir2:", "f2:"]);

    pprint(&root, 0, 0, ColorPalette::None);
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
