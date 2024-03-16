use crate::{api::*, utils::log::set_verbose};

#[test]
fn test_simple_regex_kane() -> Res<()> {
    set_verbose(true);

    let hay = Xell::from("'Citizen Kane' (1941), 'The Wizard of Oz' (1939), 'M' (1931).");
    let re = hay
        .to(r#"^regex["'([^\']+)\'\s+\(([0-9]{4})\)"]"#)
        .err()
        .unwrap();
    assert_eq!(re.sub().len()?, 3);

    assert_eq!(re.to("/[0]").read().value()?, "'Citizen Kane' (1941)");
    assert_eq!(re.to("/[0]/[0]").read().value()?, "Citizen Kane");
    assert_eq!(re.to("/[0]/[1]").read().value()?, "1941");
    assert_eq!(re.sub().at(0).sub().len()?, 2);

    assert_eq!(re.to("/[1]").read().value()?, "'The Wizard of Oz' (1939)");
    assert_eq!(re.to("/[1]/[0]").read().value()?, "The Wizard of Oz");
    assert_eq!(re.to("/[1]/[1]").read().value()?, "1939");
    assert_eq!(re.sub().at(1).sub().len()?, 2);

    assert_eq!(re.to("/[2]").read().value()?, "'M' (1931)");
    assert_eq!(re.to("/[2]/[0]").read().value()?, "M");
    assert_eq!(re.to("/[2]/[1]").read().value()?, "1931");
    assert_eq!(re.sub().at(2).sub().len()?, 2);

    Ok(())
}

#[test]
fn test_simple_regex_phone() -> Res<()> {
    set_verbose(true);

    let text = Xell::from("one - john.doe@example.com - two - x.y@z - three");
    let re = text
        .to(r#"^regex["(?<name>\w+.(\w+))@([\w\.]+)"]"#)
        .err()
        .unwrap();

    assert_eq!(re.sub().len()?, 2);

    assert_eq!(re.to("/[0]").read().value()?, "john.doe@example.com");
    assert_eq!(re.to("/[0]/[0]").read().value()?, "john.doe");
    assert_eq!(re.to("/[0]/[1]").read().value()?, "doe");
    assert_eq!(re.to("/[0]/[2]").read().value()?, "example.com");
    assert_eq!(re.sub().at(0).sub().len()?, 3);

    assert_eq!(re.to("/[1]").read().value()?, "x.y@z");
    assert_eq!(re.to("/[1]/[0]").read().value()?, "x.y");
    assert_eq!(re.to("/[1]/[1]").read().value()?, "y");
    assert_eq!(re.to("/[1]/[2]").read().value()?, "z");
    assert_eq!(re.sub().at(0).sub().len()?, 3);

    Ok(())
}
