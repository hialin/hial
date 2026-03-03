use crate::{
    api::*,
    prog::{Path, PathStart, Program, ProgramParams, program::Statement},
    utils::log::set_verbose,
};
use std::fs;

#[test]
fn program_simple_path() -> Res<()> {
    set_verbose(true);
    let prog = Program::parse(".^regex[a] ")?;
    match &prog.0[0] {
        Statement::Path(start, path) => {
            assert_eq!(start, &PathStart::File(".".to_string()));
            assert_eq!(path, &Path::parse("^regex[a]")?);
        }
        Statement::Assignment(_, _, _) => panic!("Expected a path statement!"),
        Statement::VarBind(_, _, _) => panic!("Expected a path statement!"),
    }
    Ok(())
}

#[test]
fn program_simple_assign() -> Res<()> {
    set_verbose(true);
    assert_eq!(
        Xell::new("./src/tests/data/assignment.json^json/a")
            .read()
            .value()?,
        Int::from(1)
    );
    Program::parse("./src/tests/data/assignment.json^fs[w]^json/a = 2")?
        .run(ProgramParams::default())?;
    assert_eq!(
        Xell::new("./src/tests/data/assignment.json^json/a")
            .read()
            .value()?,
        Int::from(2)
    );
    Program::parse("./src/tests/data/assignment.json^fs[w]^json/a = 1")?
        .run(ProgramParams::default())?;
    assert_eq!(
        Xell::new("./src/tests/data/assignment.json^json/a")
            .read()
            .value()?,
        Int::from(1)
    );
    Ok(())
}

#[test]
fn program_var_bind_parse() -> Res<()> {
    let prog = Program::parse("$cfg := ./src/tests/data/assignment.json^json")?;
    match &prog.0[0] {
        Statement::VarBind(name, start, path) => {
            assert_eq!(name, "cfg");
            assert_eq!(
                start,
                &PathStart::File("./src/tests/data/assignment.json".to_string())
            );
            assert_eq!(path, &Path::parse("^json")?);
        }
        _ => panic!("Expected a variable bind statement!"),
    }
    Ok(())
}

#[test]
fn program_var_bind_use_in_path() -> Res<()> {
    let test_file = "./src/tests/data/assignment_varbind.json";
    fs::write(test_file, r#"{"a":1}"#).expect("failed to seed assignment_varbind.json");
    Program::parse(
        "$cfg := ./src/tests/data/assignment_varbind.json^fs[w]^json; $cfg/a = 4; $cfg/a = 1",
    )?
    .run(ProgramParams::default())?;
    assert_eq!(
        Xell::new("./src/tests/data/assignment_varbind.json^json/a")
            .read()
            .value()?,
        Int::from(1)
    );
    fs::remove_file(test_file).expect("failed to cleanup assignment_varbind.json");
    Ok(())
}

#[test]
fn program_undefined_variable_errors() -> Res<()> {
    let err = Program::parse("$missing/a")?
        .run(ProgramParams::default())
        .expect_err("expected undefined variable error");
    assert!(format!("{}", err).contains("undefined variable :missing"));
    Ok(())
}
