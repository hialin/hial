use crate::base::*;
use crate::pprint::*;
use crate::*;

#[test]
fn test_nested() -> Res<()> {
    set_verbose(true);
    let mytext = "This is my yaml string";
    let yaml_string = format!("mytext: {}", mytext);
    let xml_string = format!("<xml><root>{}</root>", yaml_string);
    let json_string = format!("{{\"one\": [ \"{}\" ] }}", xml_string);

    let cell = Cell::from(json_string)
        .path("^json/one/[0]#value^xml/root/[0]#value^yaml/mytext#value")?
        .first()?;

    pprint(&cell, 0, 0);

    assert_eq!(cell.value()?, Value::Str(mytext));

    Ok(())
}
