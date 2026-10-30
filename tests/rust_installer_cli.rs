use std::process::Command;

#[test]
fn installer_rejects_unknown_flags_before_host_discovery() {
    let output = Command::new(env!("CARGO_BIN_EXE_mcsr-toolkit"))
        .args(["install", "--unknown"])
        .env("USER", "invalid-user-before-discovery")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("unexpected install argument: --unknown"),
        "{error}"
    );
}

#[test]
fn doctor_rejects_mutating_setup_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_mcsr-toolkit"))
        .args(["doctor", "--yes"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("doctor takes no setup flags"), "{error}");
}
