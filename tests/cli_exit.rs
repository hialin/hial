use std::process::Command;

#[test]
fn cli_exits_successfully() {
    let temp_home = std::env::temp_dir().join(format!("hial-cli-exit-home-{}", std::process::id()));
    std::fs::create_dir_all(&temp_home).expect("failed to create temp home");
    let status = Command::new(env!("CARGO_BIN_EXE_hiallib"))
        .arg("src/tests/data/assignment.json^json/a")
        .env("HOME", &temp_home)
        .status()
        .expect("failed to run CLI binary");

    assert!(status.success(), "CLI exited with status: {status}");
    let _ = std::fs::remove_dir_all(&temp_home);
}
