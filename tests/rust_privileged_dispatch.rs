#[test]
fn executable_rejects_privileged_mode_outside_installed_path() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mcsr-toolkit"))
        .args(["privileged", "keyd", "clear"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("installed root-owned executable"));
}

#[test]
fn executable_rejects_setup_mode_and_extra_arguments_outside_installed_path() {
    for args in [
        vec!["privileged", "setup"],
        vec!["privileged", "setup", "status"],
        vec!["privileged", "setup", "arbitrary"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_mcsr-toolkit"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
}
