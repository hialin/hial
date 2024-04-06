use crate::{api::*, utils::log::set_verbose};

#[test]
fn test_simple_split() -> Res<()> {
    set_verbose(true);

    let hay = Xell::from("1, 2, 3, 4, 5");
    let re = hay.to(r#"^split[","]"#).err().unwrap();
    assert_eq!(re.sub().len()?, 5);

    assert_eq!(re.to("/[0]").read().value()?, "1");
    assert_eq!(re.to("/[1]").read().value()?, " 2");
    assert_eq!(re.to("/[2]").read().value()?, " 3");
    assert_eq!(re.to("/[3]").read().value()?, " 4");
    assert_eq!(re.to("/[4]").read().value()?, " 5");

    let re = hay.to(r#"^split[", "]"#).err().unwrap();
    assert_eq!(re.sub().len()?, 5);

    assert_eq!(re.to("/[0]").read().value()?, "1");
    assert_eq!(re.to("/[1]").read().value()?, "2");
    assert_eq!(re.to("/[2]").read().value()?, "3");
    assert_eq!(re.to("/[3]").read().value()?, "4");
    assert_eq!(re.to("/[4]").read().value()?, "5");

    Ok(())
}

#[test]
fn test_split_write() -> Res<()> {
    set_verbose(true);

    let hay = Xell::from("1, 2, 3").policy(WritePolicy::WriteBackOnDrop);
    {
        let re = hay.to(r#"^split[","]"#).err().unwrap();
        assert_eq!(re.sub().len()?, 3);

        assert_eq!(re.to("/[0]").read().value()?, "1");
        assert_eq!(re.to("/[1]").read().value()?, " 2");
        assert_eq!(re.to("/[2]").read().value()?, " 3");

        re.to("/[1]").write().value("4")?;
        assert_eq!(re.to("/[1]").read().value()?, "4");
    }
    assert_eq!(hay.read().value()?, "1,4, 3");

    Ok(())
}
