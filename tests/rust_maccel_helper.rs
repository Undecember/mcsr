use mcsr_toolkit::maccel_helper::{
    dispatch, read_restore_input, recovery_pending, verify_executable, HelperPaths, ParameterIo,
};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

const NORMAL: &str = "4294967296\n";
const NONUNIT: &str = "6442450944\n";
const EYE: &str = "89464169\n";

struct Sandbox {
    dir: tempfile::TempDir,
    paths: HelperPaths,
}

#[test]
fn restored_generation_and_lock_are_healthy_recovery_state() {
    let s = Sandbox::new();
    s.restore();
    assert!(!recovery_pending(&s.paths).unwrap());
    assert_eq!(s.run("status", b"").unwrap(), "clear");
    fs::write(s.runtime().join("baseline"), NORMAL).unwrap();
    fs::set_permissions(
        s.runtime().join("baseline"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(recovery_pending(&s.paths).unwrap());
    assert_eq!(s.run("status", b"").unwrap(), "baseline_pending");
    fs::remove_file(s.runtime().join("baseline")).unwrap();
    fs::write(s.runtime().join("unexpected"), b"x").unwrap();
    assert!(recovery_pending(&s.paths).is_err());
}

#[test]
fn read_only_status_does_not_create_a_missing_lock() {
    let s = Sandbox::new();
    fs::create_dir(&s.paths.runtime).unwrap();
    fs::set_permissions(&s.paths.runtime, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(s.run("status", b"").is_err());
    assert!(!s.runtime().join("lock").exists());
}

impl Sandbox {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let parameter = dir.path().join("sys/module/maccel/parameters/SENS_MULT");
        let fingerprint = dir.path().join("sys/module/maccel/srcversion");
        fs::create_dir_all(parameter.parent().unwrap()).unwrap();
        for name in [
            "sys",
            "sys/module",
            "sys/module/maccel",
            "sys/module/maccel/parameters",
        ] {
            fs::set_permissions(dir.path().join(name), fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(&parameter, NORMAL).unwrap();
        fs::set_permissions(&parameter, fs::Permissions::from_mode(0o664)).unwrap();
        fs::write(&fingerprint, "4AFA0F5B3EBD2F98EE54E56\n").unwrap();
        fs::set_permissions(&fingerprint, fs::Permissions::from_mode(0o444)).unwrap();
        let runtime = dir.path().join("run/mcsr-toolkit-maccel");
        fs::create_dir(runtime.parent().unwrap()).unwrap();
        fs::set_permissions(runtime.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();
        let paths = HelperPaths::sandbox(
            parameter,
            runtime,
            fingerprint,
            dir.path().to_path_buf(),
            unsafe { libc::geteuid() },
        );
        Self { dir, paths }
    }
    fn parameter(&self) -> PathBuf {
        self.paths.parameter.clone()
    }
    fn runtime(&self) -> PathBuf {
        self.paths.runtime.clone()
    }
    fn run(&self, command: &str, stdin: &[u8]) -> Result<String, String> {
        dispatch(
            &["privileged", "maccel", command],
            stdin,
            &self.paths,
            &mut FakeParameter::default(),
        )
    }
    fn restore(&self) -> String {
        self.run("restore", b"").unwrap()
    }
    fn eye(&self, token: &str) -> Result<String, String> {
        self.run("eye", format!("{token}\n").as_bytes())
    }
}

#[derive(Default)]
struct FakeParameter {
    failure: Option<&'static str>,
}
impl ParameterIo for FakeParameter {
    fn write_raw(&mut self, file: &mut File, raw: i64) -> Result<(), String> {
        if self.failure == Some("write") {
            return Err("injected write failure".into());
        }
        file.set_len(0).map_err(|e| e.to_string())?;
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        if self.failure == Some("partial") {
            file.write_all(b"894").map_err(|e| e.to_string())?;
            return Err("partial parameter write".into());
        }
        if self.failure == Some("readback") {
            file.write_all(b"123\n").map_err(|e| e.to_string())?;
        } else {
            file.write_all(format!("{raw}\n").as_bytes())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn read_raw(&mut self, file: &mut File) -> Result<String, String> {
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let mut value = String::new();
        file.take(257)
            .read_to_string(&mut value)
            .map_err(|e| e.to_string())?;
        Ok(value)
    }
}

#[test]
fn exact_commands_and_token_grammar_reject_without_a_write() {
    let s = Sandbox::new();
    for args in [
        vec!["privileged", "maccel"],
        vec!["privileged", "maccel", "set"],
        vec!["privileged", "maccel", "eye", "1"],
        vec!["privileged", "maccel", "restore", "/tmp/x"],
        vec!["maccel", "eye"],
    ] {
        assert!(dispatch(&args, b"", &s.paths, &mut FakeParameter::default()).is_err());
    }
    for input in [
        b"".as_slice(),
        b"a".as_slice(),
        b"A".repeat(64).as_slice(),
        b"a".repeat(65).as_slice(),
        b"a".repeat(64)
            .iter()
            .chain(b"\nextra")
            .copied()
            .collect::<Vec<_>>()
            .as_slice(),
    ] {
        assert!(s.run("eye", input).is_err());
    }
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    assert!(!s.runtime().exists());
}

#[test]
fn fixed_production_targets_and_binary_gate() {
    let paths = HelperPaths::production();
    assert_eq!(
        paths.parameter,
        PathBuf::from("/sys/module/maccel/parameters/SENS_MULT")
    );
    assert_eq!(
        paths.fingerprint,
        PathBuf::from("/sys/module/maccel/srcversion")
    );
    assert_eq!(paths.runtime, PathBuf::from("/run/mcsr-toolkit-maccel"));
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mcsr-toolkit"))
        .args(["privileged", "maccel", "restore"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("installed root-owned executable"));
}

#[test]
fn nonunit_baseline_is_saved_once_and_restored_exactly() {
    let s = Sandbox::new();
    fs::write(s.parameter(), NONUNIT).unwrap();
    let token = s.restore();
    assert_eq!(token.len(), 64);
    assert_eq!(s.eye(&token).unwrap(), "0.02083000005222856998443603515625");
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), EYE);
    assert_eq!(
        fs::read_to_string(s.runtime().join("baseline")).unwrap(),
        NONUNIT
    );
    s.eye(&token).unwrap();
    assert_eq!(
        fs::read_to_string(s.runtime().join("baseline")).unwrap(),
        NONUNIT
    );
    assert_ne!(s.restore(), token);
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NONUNIT);
    assert!(!s.runtime().join("baseline").exists());
    assert_eq!(
        fs::metadata(s.runtime()).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for entry in fs::read_dir(s.runtime()).unwrap() {
        assert_eq!(
            entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn unsupported_fingerprint_and_malformed_raw_fail_before_runtime_creation() {
    for fingerprint in ["", "DEADBEEF\n"] {
        let s = Sandbox::new();
        fs::set_permissions(&s.paths.fingerprint, fs::Permissions::from_mode(0o644)).unwrap();
        fs::write(&s.paths.fingerprint, fingerprint).unwrap();
        assert!(s.run("restore", b"").is_err());
        assert!(!s.runtime().exists());
    }
    for raw in [
        "1.000000000\n",
        "nan\n",
        "0\n",
        "-1\n",
        "9223372036854775808\n",
    ] {
        let s = Sandbox::new();
        fs::write(s.parameter(), raw).unwrap();
        assert!(s.run("restore", b"").is_err(), "{raw}");
        assert!(!s.runtime().exists());
        assert_eq!(fs::read_to_string(s.parameter()).unwrap(), raw);
    }
}

#[test]
fn eye_write_and_readback_failures_keep_baseline_and_revoke_token() {
    for failure in ["write", "partial", "readback"] {
        let s = Sandbox::new();
        let token = s.restore();
        let mut adapter = FakeParameter {
            failure: Some(failure),
        };
        assert!(dispatch(
            &["privileged", "maccel", "eye"],
            format!("{token}\n").as_bytes(),
            &s.paths,
            &mut adapter
        )
        .is_err());
        assert_eq!(
            fs::read_to_string(s.runtime().join("baseline")).unwrap(),
            NORMAL
        );
        assert!(!s.runtime().join("generation").exists());
        assert!(s.eye(&token).is_err());
        s.restore();
        assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    }
}

#[test]
fn restore_write_and_readback_failures_keep_baseline_and_revoke_token() {
    for failure in ["write", "readback"] {
        let s = Sandbox::new();
        let token = s.restore();
        s.eye(&token).unwrap();
        let mut adapter = FakeParameter {
            failure: Some(failure),
        };
        assert!(dispatch(
            &["privileged", "maccel", "restore"],
            b"",
            &s.paths,
            &mut adapter
        )
        .is_err());
        assert!(s.runtime().join("baseline").exists());
        assert!(!s.runtime().join("generation").exists());
        assert!(s.eye(&token).is_err());
        s.restore();
        assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    }
}

#[test]
fn symlink_mode_and_hardlink_attacks_do_not_write_elsewhere() {
    for target in [
        "runtime",
        "parameter",
        "fingerprint",
        "lock",
        "generation",
        "baseline",
    ] {
        let s = Sandbox::new();
        let token = s.restore();
        let outside = s.dir.path().join("outside");
        fs::write(&outside, "unchanged").unwrap();
        let selected = match target {
            "runtime" => s.runtime(),
            "parameter" => s.parameter(),
            "fingerprint" => s.paths.fingerprint.clone(),
            other => s.runtime().join(other),
        };
        if selected.is_dir() {
            fs::remove_dir_all(&selected).unwrap();
        } else if selected.exists() {
            fs::remove_file(&selected).unwrap();
        }
        symlink(&outside, &selected).unwrap();
        assert!(s.eye(&token).is_err(), "{target}");
        assert_eq!(fs::read_to_string(outside).unwrap(), "unchanged");
    }
    for target in ["runtime", "parameter", "lock", "generation", "baseline"] {
        let s = Sandbox::new();
        let token = s.restore();
        let selected = if target == "runtime" {
            s.runtime()
        } else if target == "parameter" {
            s.parameter()
        } else {
            s.runtime().join(target)
        };
        if target == "baseline" {
            fs::write(&selected, NORMAL).unwrap();
        }
        fs::set_permissions(&selected, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(s.eye(&token).is_err(), "{target}");
    }
    let s = Sandbox::new();
    let token = s.restore();
    fs::hard_link(s.parameter(), s.dir.path().join("alias")).unwrap();
    assert!(s.eye(&token).is_err());
}

#[test]
fn stale_generation_is_rejected_after_restore() {
    let s = Sandbox::new();
    let old = s.restore();
    let new = s.restore();
    assert_ne!(old, new);
    assert!(s.eye(&old).is_err());
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    assert!(!s.runtime().join("baseline").exists());
}

#[test]
fn concurrent_eye_and_restore_serialize_without_losing_original_baseline() {
    let s = Sandbox::new();
    fs::write(s.parameter(), NONUNIT).unwrap();
    let token = s.restore();
    let (tx, rx) = mpsc::channel();
    let paths = s.paths.clone();
    let token_copy = token.clone();
    let eye = std::thread::spawn(move || {
        let result = dispatch(
            &["privileged", "maccel", "eye"],
            format!("{token_copy}\n").as_bytes(),
            &paths,
            &mut FakeParameter::default(),
        );
        tx.send(result).unwrap();
    });
    let eye_result = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    eye.join().unwrap();
    eye_result.unwrap();
    s.restore();
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NONUNIT);
}

#[test]
fn baseline_is_present_before_parameter_write() {
    struct InspectingParameter {
        runtime: PathBuf,
    }
    impl ParameterIo for InspectingParameter {
        fn read_raw(&mut self, file: &mut File) -> Result<String, String> {
            FakeParameter::default().read_raw(file)
        }
        fn write_raw(&mut self, file: &mut File, raw: i64) -> Result<(), String> {
            assert_eq!(
                fs::read_to_string(self.runtime.join("baseline")).unwrap(),
                NONUNIT
            );
            FakeParameter::default().write_raw(file, raw)
        }
    }
    let s = Sandbox::new();
    fs::write(s.parameter(), NONUNIT).unwrap();
    let token = s.restore();
    for _ in 0..2 {
        dispatch(
            &["privileged", "maccel", "eye"],
            format!("{token}\n").as_bytes(),
            &s.paths,
            &mut InspectingParameter {
                runtime: s.runtime(),
            },
        )
        .unwrap();
    }
    assert_eq!(
        fs::read_to_string(s.runtime().join("baseline")).unwrap(),
        NONUNIT
    );
}

#[test]
fn queued_old_eye_cannot_write_after_restore_rotates_generation() {
    struct BlockingParameter {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    impl ParameterIo for BlockingParameter {
        fn read_raw(&mut self, file: &mut File) -> Result<String, String> {
            self.entered.send(()).unwrap();
            self.release.recv_timeout(Duration::from_secs(5)).unwrap();
            FakeParameter::default().read_raw(file)
        }
        fn write_raw(&mut self, file: &mut File, raw: i64) -> Result<(), String> {
            FakeParameter::default().write_raw(file, raw)
        }
    }
    let s = Sandbox::new();
    let old = s.restore();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let restore_paths = s.paths.clone();
    let restore = std::thread::spawn(move || {
        dispatch(
            &["privileged", "maccel", "restore"],
            b"",
            &restore_paths,
            &mut BlockingParameter {
                entered: entered_tx,
                release: release_rx,
            },
        )
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let eye_paths = s.paths.clone();
    let old_copy = old.clone();
    let (eye_tx, eye_rx) = mpsc::channel();
    let eye = std::thread::spawn(move || {
        eye_tx
            .send(dispatch(
                &["privileged", "maccel", "eye"],
                format!("{old_copy}\n").as_bytes(),
                &eye_paths,
                &mut FakeParameter::default(),
            ))
            .unwrap();
    });
    assert!(eye_rx.recv_timeout(Duration::from_millis(50)).is_err());
    release_tx.send(()).unwrap();
    let new = restore.join().unwrap().unwrap();
    assert_ne!(old, new);
    let error = eye_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap_err();
    eye.join().unwrap();
    assert!(error.contains("stale generation"), "{error}");
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    assert!(!s.runtime().join("baseline").exists());
}

#[test]
fn failed_restore_preflight_revokes_old_generation() {
    let s = Sandbox::new();
    let old = s.restore();
    fs::remove_file(&s.paths.fingerprint).unwrap();
    assert!(s.run("restore", b"").is_err());
    assert!(!s.runtime().join("generation").exists());
    fs::write(&s.paths.fingerprint, "4AFA0F5B3EBD2F98EE54E56\n").unwrap();
    fs::set_permissions(&s.paths.fingerprint, fs::Permissions::from_mode(0o444)).unwrap();
    assert!(s.eye(&old).unwrap_err().contains("stale generation"));
}

#[test]
fn unsafe_runtime_evidence_is_preserved_on_restore_failure() {
    for target in ["unexpected", "generation", "baseline"] {
        let s = Sandbox::new();
        let old = s.restore();
        match target {
            "unexpected" => fs::write(s.runtime().join("unexpected"), "untouched").unwrap(),
            "generation" => fs::write(s.runtime().join("generation"), "garbage\n").unwrap(),
            _ => {
                fs::write(s.runtime().join("baseline"), "nan\n").unwrap();
                fs::set_permissions(
                    s.runtime().join("baseline"),
                    fs::Permissions::from_mode(0o600),
                )
                .unwrap();
            }
        }
        assert!(s.run("restore", b"").is_err(), "{target}");
        assert!(s.runtime().join("generation").exists(), "{target}");
        if target != "generation" {
            assert_eq!(
                fs::read_to_string(s.runtime().join("generation")).unwrap(),
                format!("{old}\n")
            );
        }
        assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    }
}

#[test]
fn wrong_owner_writable_ancestor_and_special_file_fail_closed() {
    let s = Sandbox::new();
    let wrong_owner = HelperPaths::sandbox(
        s.parameter(),
        s.runtime(),
        s.paths.fingerprint.clone(),
        s.dir.path().to_path_buf(),
        unsafe { libc::geteuid() } + 1,
    );
    assert!(dispatch(
        &["privileged", "maccel", "restore"],
        b"",
        &wrong_owner,
        &mut FakeParameter::default()
    )
    .is_err());
    assert!(!s.runtime().exists());

    let writable = s.dir.path().join("writable");
    fs::create_dir(&writable).unwrap();
    fs::set_permissions(&writable, fs::Permissions::from_mode(0o777)).unwrap();
    let alternate = writable.join("SENS_MULT");
    fs::write(&alternate, NORMAL).unwrap();
    fs::set_permissions(&alternate, fs::Permissions::from_mode(0o664)).unwrap();
    let paths = HelperPaths::sandbox(
        alternate.clone(),
        s.runtime(),
        s.paths.fingerprint.clone(),
        s.dir.path().to_path_buf(),
        unsafe { libc::geteuid() },
    );
    assert!(dispatch(
        &["privileged", "maccel", "restore"],
        b"",
        &paths,
        &mut FakeParameter::default()
    )
    .is_err());
    assert_eq!(fs::read_to_string(alternate).unwrap(), NORMAL);

    let token = s.restore();
    let baseline = s.runtime().join("baseline");
    let c_path = std::ffi::CString::new(baseline.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    assert!(s.eye(&token).is_err());
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
}

#[test]
fn executable_requires_trusted_parent_owner_mode_and_regular_file() {
    let s = Sandbox::new();
    let libexec = s.dir.path().join("usr/local/libexec");
    fs::create_dir_all(&libexec).unwrap();
    for name in ["usr", "usr/local", "usr/local/libexec"] {
        fs::set_permissions(s.dir.path().join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let executable = libexec.join("mcsr-toolkit");
    fs::write(&executable, "fixture").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let uid = unsafe { libc::geteuid() };
    assert!(verify_executable(&executable, s.dir.path(), uid).is_ok());
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o775)).unwrap();
    assert!(verify_executable(&executable, s.dir.path(), uid).is_err());
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    fs::set_permissions(&libexec, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(verify_executable(&executable, s.dir.path(), uid).is_err());
    fs::set_permissions(&libexec, fs::Permissions::from_mode(0o755)).unwrap();
    fs::remove_file(&executable).unwrap();
    symlink(s.parameter(), &executable).unwrap();
    assert!(verify_executable(&executable, s.dir.path(), uid).is_err());
}

#[test]
fn valid_eye_preflight_failure_revokes_token_but_stale_eye_preserves_it() {
    let s = Sandbox::new();
    let token = s.restore();
    let record = s.runtime().join("generation");
    fs::remove_file(&s.paths.fingerprint).unwrap();
    assert!(s.eye(&"a".repeat(64)).is_err());
    assert_eq!(fs::read_to_string(&record).unwrap(), format!("{token}\n"));
    assert!(s.eye(&token).is_err());
    assert!(!record.exists());
    fs::write(&s.paths.fingerprint, "4AFA0F5B3EBD2F98EE54E56\n").unwrap();
    fs::set_permissions(&s.paths.fingerprint, fs::Permissions::from_mode(0o444)).unwrap();
    assert!(s.eye(&token).unwrap_err().contains("stale generation"));
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    assert_ne!(s.restore(), token);
}

#[test]
fn valid_eye_initial_read_failure_rejects_another_queued_eye_until_restore() {
    struct BlockingFailedRead {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    impl ParameterIo for BlockingFailedRead {
        fn read_raw(&mut self, _file: &mut File) -> Result<String, String> {
            self.entered.send(()).unwrap();
            self.release.recv_timeout(Duration::from_secs(5)).unwrap();
            Err("injected initial read failure".into())
        }
        fn write_raw(&mut self, _file: &mut File, _raw: i64) -> Result<(), String> {
            panic!("eye must not write after initial read failure")
        }
    }
    let s = Sandbox::new();
    let token = s.restore();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let first_paths = s.paths.clone();
    let first_token = token.clone();
    let first = std::thread::spawn(move || {
        dispatch(
            &["privileged", "maccel", "eye"],
            format!("{first_token}\n").as_bytes(),
            &first_paths,
            &mut BlockingFailedRead {
                entered: entered_tx,
                release: release_rx,
            },
        )
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let queued_paths = s.paths.clone();
    let queued_token = token.clone();
    let (queued_tx, queued_rx) = mpsc::channel();
    let queued = std::thread::spawn(move || {
        queued_tx
            .send(dispatch(
                &["privileged", "maccel", "eye"],
                format!("{queued_token}\n").as_bytes(),
                &queued_paths,
                &mut FakeParameter::default(),
            ))
            .unwrap()
    });
    assert!(queued_rx.recv_timeout(Duration::from_millis(50)).is_err());
    release_tx.send(()).unwrap();
    assert!(first
        .join()
        .unwrap()
        .unwrap_err()
        .contains("injected initial read failure"));
    assert!(queued_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap_err()
        .contains("stale generation"));
    queued.join().unwrap();
    assert!(!s.runtime().join("generation").exists());
    assert_eq!(fs::read_to_string(s.parameter()).unwrap(), NORMAL);
    assert_ne!(s.restore(), token);
}

#[test]
fn restore_stdin_probe_rejects_available_payload_without_waiting_for_eof() {
    let s = Sandbox::new();
    let (mut sender, receiver) = UnixStream::pair().unwrap();
    assert_eq!(read_restore_input(receiver.as_raw_fd()).unwrap(), b"");
    sender.write_all(b"unexpected").unwrap();
    let input = read_restore_input(receiver.as_raw_fd()).unwrap();
    assert_eq!(input, b"u");
    assert!(dispatch(
        &["privileged", "maccel", "restore"],
        &input,
        &s.paths,
        &mut FakeParameter::default()
    )
    .is_err());
    assert!(!s.runtime().exists());
}
