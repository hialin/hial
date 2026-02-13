use std::process::Command;

#[test]
fn cli_exits_successfully() {
    let status = Command::new(env!("CARGO_BIN_EXE_hiallib"))
        .arg("src/tests/data/assignment.json^json/a")
        .status()
        .expect("failed to run CLI binary");

    assert!(status.success(), "CLI exited with status: {status}");
}
