#[cfg(test)]
mod tests {
    use crate::base::common::*;
    use crate::base::interpretation_api::*;
    use crate::base::rust_api::*;
    use crate::pathlang::path::Path;
    use crate::set_verbose;

    #[test]
    fn test_files() -> Res<()> {
        let examples = Cell::from(".").be("file")?.sub()?.get("examples")?;
        assert_eq!(std::mem::size_of_val(&examples), 4 * 8);
        assert_eq!(examples.label()?, "examples");
        assert_eq!(examples.value()?, "examples");
        Ok(())
    }

    #[test]
    fn test_json() -> Res<()> {
        let json = r#"{
            "hosts": [
                {
                    "host_id": "1h48",
                    "labels": {
                        "power": "weak",
                        "gateway": "true"
                    }
                },
                {
                    "host_id": "1h51",
                    "labels": {
                        "group2": true,
                        "power": "strong"
                    }
                }
            ]
        }"#;
        let json = Cell::from(json).be("json")?;
        // pprint::pprint(&json, 0, 0);
        let hosts = json.sub()?.get("hosts")?.sub()?;
        assert_eq!(hosts.len(), 2);
        let host1 = hosts.at(0)?;
        let host2 = hosts.at(1)?;
        let power1 = host1.sub()?.get("labels")?.sub()?.get("power")?;
        let power2 = host2.sub()?.get("labels")?.sub()?.get("power")?;
        let group2 = host2.sub()?.get("labels")?.sub()?.get("group2")?;
        assert_eq!(power1.value()?, Value::Str("weak"));
        assert_eq!(power2.value()?, Value::Str("strong"));
        assert_eq!(group2.value()?, Value::Bool(true));
        Ok(())
    }

    #[test]
    fn test_yaml() -> Res<()> {
        let yaml = r#"
            hosts:
              - host_id: 1h48
                labels:
                  power: "weak"
                  gateway: "true"
              - host_id: "1h51"
                labels:
                  "group2": true
                  "power": "strong"
        "#;
        let yaml = Cell::from(yaml).be("yaml")?;
        // pprint::pprint(&yaml, 0, 0);
        let hosts = yaml.sub()?.get("hosts")?.sub()?;
        assert_eq!(hosts.len(), 2);
        let host1 = hosts.at(0)?;
        let host2 = hosts.at(1)?;
        let power1 = host1.sub()?.get("labels")?.sub()?.get("power")?;
        let power2 = host2.sub()?.get("labels")?.sub()?.get("power")?;
        let group2 = host2.sub()?.get("labels")?.sub()?.get("group2")?;
        assert_eq!(power1.value()?, Value::Str("weak"));
        assert_eq!(power2.value()?, Value::Str("strong"));
        assert_eq!(group2.value()?, Value::Bool(true));
        Ok(())
    }

    #[test]
    fn test_xml() -> Res<()> {
        let xml = r#"
            <?xml version="1.0"?>
            <!DOCTYPE entity PUBLIC "-//no idea//EN" "http://example.com/dtd">            
            <doc>
                <first>1</first>
                <double>2</double>
                <double>2</double>
                <triple/>
            </doc>
        "#;
        let xml = Cell::from(xml).be("xml")?;
        // pprint::pprint(&xml, 0, 0);
        let decl = xml.sub()?.at(0)?;
        let doc = xml.sub()?.at(2)?;
        assert_eq!(doc.sub()?.len(), 4);
        assert_eq!(doc.sub()?.get("first")?.label()?, "first");
        assert_eq!(doc.sub()?.at(1)?.label()?, "double");
        assert_eq!(doc.sub()?.at(2)?.value()?, Value::Str("double"));
        assert_eq!(doc.sub()?.get("triple")?.value()?, Value::Str("triple"));
        Ok(())
    }

    #[test]
    fn test_path_with_starter() -> Res<()> {
        let path = "./LICENSE@size";
        let (root, path) = Path::parse_with_starter(path)?;
        let eval = path
            .eval(root.eval()?)
            .map(|c| Ok(c?.value()?.to_string()))
            .collect::<Res<Vec<_>>>()?;
        assert_eq!(eval, ["26526"]);
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
        "#;

    #[test]
    fn test_path() -> Res<()> {
        let root = Cell::from(TREE).be("yaml")?;
        let eval = str_eval(root, "/a/b/x")?;
        assert_eq!(eval, ["xb"]);
        Ok(())
    }

    #[test]
    fn test_path_kleene() -> Res<()> {
        let root = Cell::from(TREE).be("yaml")?;
        let eval = str_eval(root, "/*/x")?;
        assert_eq!(eval, ["xa"]);
        Ok(())
    }

    #[test]
    fn test_path_double_kleene() -> Res<()> {
        let root = Cell::from(TREE).be("yaml")?;
        // pprint::pprint(&root, 0, 0);
        let eval = str_eval(root.clone(), "/**/x")?;
        assert_eq!(eval, ["xa", "xb", "xc"]);
        let eval = str_eval(root.clone(), "/**/y")?;
        assert_eq!(eval, ["yc"]);
        Ok(())
    }

    const TREE_2: &str = r#"
            a:
              x: xa
              b:
                x: xb
                c:
                    x: xc
                    y: yc
        "#;

    #[test]
    fn test_path_double_kleene_top_filter() -> Res<()> {
        set_verbose(true);
        let root = Cell::from(TREE_2).be("yaml")?;
        let eval = str_eval(root.clone(), "/*[.label=='a']/**[=='xa']")?;
        assert_eq!(eval, ["xa"]);
        let eval = str_eval(root.clone(), "/*[.label=='a']/**/x[=='xc']")?;
        assert_eq!(eval, ["xc"]);
        Ok(())
    }

    #[test]
    fn test_path_double_kleene_deep_filter() -> Res<()> {
        let root = Cell::from(TREE_2).be("yaml")?;
        let eval = str_eval(root.clone(), "/**/*[.label=='x']")?;
        assert_eq!(eval, ["xa", "xb", "xc"]);
        let eval = str_eval(root.clone(), "/a/**[.label!='x']/y")?;
        assert_eq!(eval, ["yc"]);
        let eval = str_eval(root.clone(), "/a/**/*[=='xb']")?;
        assert_eq!(eval, ["xb"]);
        Ok(())
    }

    fn str_eval(root: Cell, path: &str) -> Res<Vec<String>> {
        Path::parse(path)?
            .eval(root)
            .map(|c| Ok(c?.value()?.to_string()))
            .collect::<Res<Vec<_>>>()
    }
}
