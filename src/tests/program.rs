use crate::{
    api::*,
    prog::{program::Statement, Path, PathStart, Program, ProgramParams},
    utils::log::set_verbose,
};

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
