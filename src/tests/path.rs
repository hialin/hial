use crate::{api::*, utils::log::set_verbose};

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
    assert_eq!(x.path()?, "`\\n            a:...`^yaml/a/b/c/x");
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
        "`http://api.githu...`^http^json/rate_limit_url^http^json/resources/core/limit".to_string()
    );

    Ok(())
}
