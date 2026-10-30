use mcsr_toolkit::keyd_helper::{
    dispatch, recovery_pending, DirectorySync, HelperFilesystem, HelperSystem,
};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const FRAGMENT: &str = "[mcsr_chat]\ns = s\nt = t\nenter = clearm(enter)\nesc = clearm(esc)\n";
const DEVICE: &str = "# MCSR Toolkit managed\n[ids]\n*\n\ninclude mcsr-toolkit\n";

struct Sandbox {
    dir: tempfile::TempDir,
    fs: HelperFilesystem,
}
impl Sandbox {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let config = dir.path().join("etc/keyd");
        fs::create_dir_all(&config).unwrap();
        for path in [dir.path().join("etc"), config.clone()] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let helper = dir.path().join("libexec/mcsr-toolkit");
        fs::create_dir_all(helper.parent().unwrap()).unwrap();
        fs::set_permissions(helper.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(&helper, b"test executable").unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
        fs::create_dir(dir.path().join("run")).unwrap();
        fs::set_permissions(dir.path().join("run"), fs::Permissions::from_mode(0o755)).unwrap();
        let fs = HelperFilesystem::sandbox(
            config,
            dir.path().join("var/lib/mcsr-toolkit/keyd"),
            helper,
            dir.path().to_path_buf(),
            unsafe { libc::geteuid() },
        );
        Self { dir, fs }
    }
    fn config(&self, name: &str) -> PathBuf {
        self.dir.path().join("etc/keyd").join(name)
    }
    fn backup(&self, id: &str) -> PathBuf {
        self.dir.path().join("var/lib/mcsr-toolkit/keyd").join(id)
    }
}

#[derive(Default)]
struct FakeSystem {
    now: Duration,
    calls: Vec<&'static str>,
    fail_restart_at: Option<usize>,
    fail_reset: bool,
    fail_health: bool,
    fail_stop: bool,
    restart_count: usize,
    slow_health_at: Option<usize>,
    health_count: usize,
    initial_errors: usize,
    fail_health_at: Option<usize>,
    error_health_at: Option<usize>,
    probe_delay: Duration,
    budgets: Vec<Duration>,
    keys: Option<String>,
}
impl HelperSystem for FakeSystem {
    fn list_keys(&mut self) -> Result<String, String> {
        Ok(self
            .keys
            .clone()
            .unwrap_or_else(|| "s\nt\nenter\nesc\n".into()))
    }
    fn systemctl(&mut self, action: &'static str) -> Result<bool, String> {
        self.calls.push(action);
        match action {
            "reset-failed" => Ok(!self.fail_reset),
            "restart" => {
                self.restart_count += 1;
                Ok(self.fail_restart_at != Some(self.restart_count))
            }
            "is-active" => {
                self.health_count += 1;
                self.now += self.probe_delay;
                if self.health_count <= self.initial_errors
                    || self.error_health_at == Some(self.health_count)
                {
                    return Err("IPC connection unavailable".into());
                }
                if self.slow_health_at == Some(self.health_count) {
                    self.now += Duration::from_millis(600);
                }
                Ok(!self.fail_health && self.fail_health_at != Some(self.health_count))
            }
            "stop" => Ok(!self.fail_stop),
            _ => panic!("unexpected systemctl action"),
        }
    }
    fn readiness(&mut self, budget: Duration) -> Result<bool, String> {
        self.budgets.push(budget);
        if budget.is_zero() {
            return Err("readiness budget exhausted".into());
        }
        self.systemctl("is-active")
    }
    fn now(&self) -> Duration {
        self.now
    }
    fn sleep(&mut self, duration: Duration) {
        assert!(duration > Duration::ZERO);
        self.now += duration;
    }
}

fn install(s: &Sandbox, system: &mut FakeSystem, fragment: &str) -> String {
    let output = dispatch(
        &["privileged", "keyd", "install"],
        fragment.as_bytes(),
        &s.fs,
        system,
    )
    .unwrap()
    .unwrap();
    serde_json::from_str::<serde_json::Value>(&output).unwrap()["backup_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn restored_historical_snapshot_is_not_pending_recovery() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let id = install(&s, &mut sys, FRAGMENT);
    assert_eq!(
        dispatch(&["privileged", "keyd", "status"], b"", &s.fs, &mut sys).unwrap(),
        Some("active".into())
    );
    dispatch(
        &["privileged", "keyd", "restore", &id],
        b"",
        &s.fs,
        &mut sys,
    )
    .unwrap();
    assert!(!recovery_pending(&s.fs).unwrap());
    assert_eq!(
        dispatch(&["privileged", "keyd", "status"], b"", &s.fs, &mut sys).unwrap(),
        Some("clear".into())
    );
    assert!(s.backup(&id).join("manifest.json").exists());
    fs::write(
        s.dir.path().join("var/lib/mcsr-toolkit/keyd/unexpected"),
        b"x",
    )
    .unwrap();
    assert!(recovery_pending(&s.fs).is_err());
}

#[test]
fn status_recreates_volatile_lock_after_reboot_with_historical_snapshot() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let id = install(&s, &mut sys, FRAGMENT);
    dispatch(
        &["privileged", "keyd", "restore", &id],
        b"",
        &s.fs,
        &mut sys,
    )
    .unwrap();
    let lock = s.dir.path().join("run/mcsr-toolkit-keyd.lock");
    fs::remove_file(&lock).unwrap();
    assert_eq!(
        dispatch(&["privileged", "keyd", "status"], b"", &s.fs, &mut sys).unwrap(),
        Some("clear".into())
    );
    assert!(lock.exists());
}

#[test]
fn rejects_unknown_commands_extra_arguments_and_payloads_without_side_effects() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    for args in [
        vec!["privileged", "keyd"],
        vec!["privileged", "keyd", "install", "extra"],
        vec!["privileged", "keyd", "restore"],
        vec!["privileged", "keyd", "clear", "extra"],
        vec!["privileged", "keyd", "other"],
        vec!["serve", "keyd", "clear"],
        vec![
            "privileged",
            "keyd",
            "restore",
            "A0000000000000000000000000000000",
        ],
    ] {
        assert!(dispatch(&args, b"", &s.fs, &mut sys).is_err(), "{args:?}");
    }
    assert!(dispatch(&["privileged", "keyd", "clear"], b"x", &s.fs, &mut sys).is_err());
    assert!(dispatch(
        &["privileged", "keyd", "restore", "a"],
        b"x",
        &s.fs,
        &mut sys
    )
    .is_err());
    assert!(sys.calls.is_empty());
}

#[test]
fn concurrent_helper_invocation_is_refused_before_service_mutation() {
    struct BlockingSystem {
        entered: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
        now: Duration,
    }
    impl HelperSystem for BlockingSystem {
        fn list_keys(&mut self) -> Result<String, String> {
            Ok(String::new())
        }
        fn systemctl(&mut self, action: &'static str) -> Result<bool, String> {
            if action == "restart" {
                self.entered.send(()).unwrap();
                self.release.recv().unwrap();
            }
            Ok(true)
        }
        fn readiness(&mut self, _budget: Duration) -> Result<bool, String> {
            Ok(true)
        }
        fn now(&self) -> Duration {
            self.now
        }
        fn sleep(&mut self, duration: Duration) {
            self.now += duration;
        }
    }
    let s = Sandbox::new();
    let first_fs = s.fs.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let first = std::thread::spawn(move || {
        let mut system = BlockingSystem {
            entered: entered_tx,
            release: release_rx,
            now: Duration::ZERO,
        };
        dispatch(
            &["privileged", "keyd", "clear"],
            b"",
            &first_fs,
            &mut system,
        )
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let replacement = s.dir.path().join("libexec/replacement");
    fs::write(&replacement, b"new test executable").unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o755)).unwrap();
    fs::rename(&replacement, s.dir.path().join("libexec/mcsr-toolkit")).unwrap();
    let mut second_system = FakeSystem::default();
    let second = dispatch(
        &["privileged", "keyd", "clear"],
        b"",
        &s.fs,
        &mut second_system,
    );
    release_tx.send(()).unwrap();
    assert!(first.join().unwrap().is_ok());
    assert!(second.unwrap_err().contains("busy"));
    assert!(second_system.calls.is_empty());
}

#[test]
fn clear_stops_keyd_when_config_is_conflicting_or_writable() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    fs::write(s.config("other.conf"), b"[main]\na = command(echo bad)\n").unwrap();
    assert!(dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).is_err());
    assert_eq!(sys.calls, ["stop"]);
    fs::remove_file(s.config("other.conf")).unwrap();
    let id = install(&s, &mut sys, FRAGMENT);
    assert_eq!(id.len(), 32);
    sys.calls.clear();
    fs::set_permissions(s.config("mcsr-toolkit"), fs::Permissions::from_mode(0o666)).unwrap();
    assert!(dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).is_err());
    assert_eq!(sys.calls, ["stop"]);
}

#[test]
fn clear_reports_unsafe_when_config_is_untrusted_and_stop_fails() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        fail_stop: true,
        ..Default::default()
    };
    fs::write(s.config("other.conf"), b"untrusted").unwrap();
    let error = dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).unwrap_err();
    assert!(error.contains("unsafe"), "{error}");
    assert_eq!(sys.calls, ["stop"]);
}

#[test]
fn recover_resets_start_limit_and_verifies_health_without_changing_configs() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let id = install(&s, &mut sys, FRAGMENT);
    let before = fs::read(s.config("mcsr-toolkit")).unwrap();
    sys.calls.clear();
    dispatch(&["privileged", "keyd", "recover"], b"", &s.fs, &mut sys).unwrap();
    assert_eq!(&sys.calls[..2], ["reset-failed", "restart"]);
    assert!(sys.calls.contains(&"is-active"));
    assert_eq!(fs::read(s.config("mcsr-toolkit")).unwrap(), before);
    assert!(s.backup(&id).join("manifest.json").is_file());
}

#[test]
fn recover_keeps_start_limit_when_configs_are_unsafe() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    fs::write(s.config("other.conf"), b"untrusted").unwrap();
    let error = dispatch(&["privileged", "keyd", "recover"], b"", &s.fs, &mut sys).unwrap_err();
    assert!(error.contains("unsafe keyd config"), "{error}");
    assert_eq!(sys.calls, ["stop"]);
}

#[test]
fn recover_propagates_reset_restart_and_health_failures() {
    for failure in ["reset", "restart", "health"] {
        let s = Sandbox::new();
        let mut sys = FakeSystem {
            fail_reset: failure == "reset",
            fail_restart_at: (failure == "restart").then_some(1),
            fail_health: failure == "health",
            ..Default::default()
        };
        assert!(
            dispatch(&["privileged", "keyd", "recover"], b"", &s.fs, &mut sys).is_err(),
            "{failure}"
        );
        if failure == "reset" {
            assert_eq!(sys.calls, ["reset-failed"]);
        } else {
            assert_eq!(sys.calls.last(), Some(&"stop"));
        }
    }
}

#[test]
fn recover_refuses_busy_helper_without_resetting_start_limit() {
    use std::os::fd::AsRawFd;
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    dispatch(&["privileged", "keyd", "status"], b"", &s.fs, &mut sys).unwrap();
    let lock = fs::File::open(s.dir.path().join("run/mcsr-toolkit-keyd.lock")).unwrap();
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let error = dispatch(&["privileged", "keyd", "recover"], b"", &s.fs, &mut sys).unwrap_err();
    assert!(error.contains("busy"), "{error}");
    assert!(sys.calls.is_empty());
}

#[test]
fn recovery_fences_requests_created_before_recovery() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let delayed = s.fs.fence_input(FRAGMENT.as_bytes()).unwrap();
    dispatch(&["privileged", "keyd", "recover"], b"", &s.fs, &mut sys).unwrap();
    sys.calls.clear();
    let error = dispatch(
        &["privileged", "keyd", "install"],
        &delayed,
        &s.fs,
        &mut sys,
    )
    .unwrap_err();
    assert!(error.contains("superseded by recovery"), "{error}");
    assert!(!s.config("mcsr-toolkit.conf").exists());
    assert!(sys.calls.is_empty());
    let fresh = s.fs.fence_input(FRAGMENT.as_bytes()).unwrap();
    dispatch(&["privileged", "keyd", "install"], &fresh, &s.fs, &mut sys).unwrap();
    assert_eq!(
        fs::read(s.config("mcsr-toolkit")).unwrap(),
        FRAGMENT.as_bytes()
    );
    let delayed_empty = s.fs.fence_input(b"").unwrap();
    dispatch(&["privileged", "keyd", "recover"], b"", &s.fs, &mut sys).unwrap();
    sys.calls.clear();
    for args in [
        vec!["privileged", "keyd", "clear"],
        vec!["privileged", "keyd", "recover"],
        vec![
            "privileged",
            "keyd",
            "restore",
            "00000000000000000000000000000000",
        ],
    ] {
        let error = dispatch(&args, &delayed_empty, &s.fs, &mut sys).unwrap_err();
        assert!(error.contains("superseded by recovery"), "{error}");
    }
    assert!(sys.calls.is_empty());
}

#[derive(Default)]
struct RecordingSync {
    calls: Mutex<Vec<PathBuf>>,
    fail_on: Mutex<Option<PathBuf>>,
}
impl DirectorySync for RecordingSync {
    fn sync_dir(&self, path: &std::path::Path) -> Result<(), String> {
        self.calls.lock().unwrap().push(path.to_path_buf());
        if self.fail_on.lock().unwrap().as_deref() == Some(path) {
            return Err("injected directory sync failure".into());
        }
        Ok(())
    }
}

#[test]
fn new_snapshot_directory_is_synced_before_config_changes() {
    let mut s = Sandbox::new();
    let sync = Arc::new(RecordingSync::default());
    let backup_root = s.dir.path().join("var/lib/mcsr-toolkit/keyd");
    s.fs = s.fs.with_directory_sync(sync.clone());
    let mut sys = FakeSystem::default();
    install(&s, &mut sys, FRAGMENT);
    let calls = sync.calls.lock().unwrap().clone();
    for parent in [
        s.dir.path().to_path_buf(),
        s.dir.path().join("var"),
        s.dir.path().join("var/lib"),
        s.dir.path().join("var/lib/mcsr-toolkit"),
        backup_root.clone(),
    ] {
        assert!(
            calls.contains(&parent),
            "missing directory sync: {}",
            parent.display()
        );
    }
    assert!(
        calls
            .iter()
            .any(|path| path.parent() == Some(backup_root.as_path())),
        "snapshot directory was not synced"
    );
    let mut failing = Sandbox::new();
    let fail_sync = Arc::new(RecordingSync::default());
    *fail_sync.fail_on.lock().unwrap() = Some(failing.dir.path().join("var/lib/mcsr-toolkit/keyd"));
    failing.fs = failing.fs.with_directory_sync(fail_sync);
    let mut fail_sys = FakeSystem::default();
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &failing.fs,
        &mut fail_sys
    )
    .is_err());
    assert_eq!(
        fs::read_dir(failing.dir.path().join("etc/keyd"))
            .unwrap()
            .count(),
        0
    );
    assert!(fail_sys.calls.is_empty());
}

#[test]
fn helper_executable_symlink_and_backup_root_symlink_are_refused() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let helper = s.dir.path().join("libexec/mcsr-toolkit");
    fs::remove_file(&helper).unwrap();
    let outside = s.dir.path().join("outside");
    fs::write(&outside, b"test executable").unwrap();
    symlink(&outside, &helper).unwrap();
    assert!(dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).is_err());
    assert!(sys.calls.is_empty());
    fs::remove_file(&helper).unwrap();
    fs::write(&helper, b"test executable").unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
    let backup_parent = s.dir.path().join("var/lib/mcsr-toolkit");
    fs::create_dir_all(&backup_parent).unwrap();
    fs::set_permissions(s.dir.path().join("var"), fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(
        s.dir.path().join("var/lib"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    fs::set_permissions(&backup_parent, fs::Permissions::from_mode(0o700)).unwrap();
    let backup_target = s.dir.path().join("backup-outside");
    fs::create_dir(&backup_target).unwrap();
    symlink(&backup_target, backup_parent.join("keyd")).unwrap();
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    assert_eq!(
        fs::read_dir(s.dir.path().join("etc/keyd")).unwrap().count(),
        0
    );
    assert_eq!(fs::read_dir(backup_target).unwrap().count(), 0);
}

#[test]
fn install_rejects_injected_fragment_before_any_write_or_restart() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    for fragment in [
        "[mcsr_chat]\ns = command(echo bad)\n",
        "[mcsr_chat]\ns = s\n[main]\na = b\n",
        "[mcsr_chat]\ns = s\ns = s\n",
        "[mcsr_chat]\ns = clearm(t)\n",
        "[mcsr_chat]\nunknown = unknown\n",
    ] {
        assert!(dispatch(
            &["privileged", "keyd", "install"],
            fragment.as_bytes(),
            &s.fs,
            &mut sys
        )
        .is_err());
    }
    assert_eq!(
        fs::read_dir(s.dir.path().join("etc/keyd")).unwrap().count(),
        0
    );
    assert!(sys.calls.is_empty());
}

#[test]
fn install_rejects_fallback_key_missing_from_live_keyd_vocabulary() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let fragment = "[mcsr_chat]\ntab = tab\n";
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        fragment.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    assert_eq!(
        fs::read_dir(s.dir.path().join("etc/keyd")).unwrap().count(),
        0
    );
}

#[test]
fn install_and_restore_use_only_fixed_targets_and_snapshot_original_modes() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let first = install(&s, &mut sys, FRAGMENT);
    assert_eq!(
        fs::read_to_string(s.config("mcsr-toolkit.conf")).unwrap(),
        DEVICE
    );
    assert_eq!(
        fs::read_to_string(s.config("mcsr-toolkit")).unwrap(),
        FRAGMENT
    );
    assert_eq!(first.len(), 32);
    fs::set_permissions(s.config("mcsr-toolkit"), fs::Permissions::from_mode(0o600)).unwrap();
    let changed = FRAGMENT.replace("s = s", "s = clearm(s)");
    let second = install(&s, &mut sys, &changed);
    let output = dispatch(
        &["privileged", "keyd", "restore", &second],
        b"",
        &s.fs,
        &mut sys,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&output).unwrap()["restored"],
        second
    );
    assert_eq!(
        fs::read_to_string(s.config("mcsr-toolkit")).unwrap(),
        FRAGMENT
    );
    assert_eq!(
        fs::metadata(s.config("mcsr-toolkit"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let output = dispatch(
        &["privileged", "keyd", "restore", &first],
        b"",
        &s.fs,
        &mut sys,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&output).unwrap()["restored"],
        first
    );
    assert_eq!(
        fs::read_dir(s.dir.path().join("etc/keyd")).unwrap().count(),
        0
    );
    assert_eq!(
        fs::metadata(s.backup(&first).join("manifest.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn existing_config_conflict_and_target_symlink_are_refused() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    fs::write(s.config("other.conf"), b"unchanged").unwrap();
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    assert_eq!(fs::read(s.config("other.conf")).unwrap(), b"unchanged");
    fs::remove_file(s.config("other.conf")).unwrap();
    let outside = s.dir.path().join("outside");
    fs::write(&outside, b"unchanged").unwrap();
    symlink(&outside, s.config("mcsr-toolkit")).unwrap();
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    assert_eq!(fs::read(outside).unwrap(), b"unchanged");
    assert!(sys.calls.is_empty());
}

#[test]
fn existing_managed_target_with_special_mode_is_refused() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    fs::write(s.config("mcsr-toolkit.conf"), DEVICE).unwrap();
    fs::write(s.config("mcsr-toolkit"), FRAGMENT).unwrap();
    fs::set_permissions(
        s.config("mcsr-toolkit.conf"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    fs::set_permissions(s.config("mcsr-toolkit"), fs::Permissions::from_mode(0o4644)).unwrap();
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    assert!(sys.calls.is_empty());
}

#[test]
fn symlink_and_writable_ancestors_and_snapshots_are_refused() {
    let mut s = Sandbox::new();
    let mut sys = FakeSystem::default();
    let alias = s.dir.path().join("alias");
    symlink(s.dir.path().join("etc"), &alias).unwrap();
    s.fs = s.fs.with_config_dir(alias.join("keyd"));
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    s.fs = s.fs.with_config_dir(s.dir.path().join("etc/keyd"));
    fs::set_permissions(s.dir.path().join("etc"), fs::Permissions::from_mode(0o777)).unwrap();
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    fs::set_permissions(s.dir.path().join("etc"), fs::Permissions::from_mode(0o755)).unwrap();
    let id = install(&s, &mut sys, FRAGMENT);
    let manifest = s.backup(&id).join("manifest.json");
    fs::remove_file(&manifest).unwrap();
    let outside = s.dir.path().join("outside");
    fs::write(&outside, b"{}").unwrap();
    symlink(&outside, &manifest).unwrap();
    assert!(dispatch(
        &["privileged", "keyd", "restore", &id],
        b"",
        &s.fs,
        &mut sys
    )
    .is_err());
    assert_eq!(
        fs::read_to_string(s.config("mcsr-toolkit")).unwrap(),
        FRAGMENT
    );
}

#[test]
fn restore_rejects_manifest_with_user_writable_snapshot_mode() {
    let s = Sandbox::new();
    let mut sys = FakeSystem::default();
    install(&s, &mut sys, FRAGMENT);
    let id = install(&s, &mut sys, &FRAGMENT.replace("s = s", "s = clearm(s)"));
    let path = s.backup(&id).join("manifest.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["fragment"]["mode"] = 0o666.into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(dispatch(
        &["privileged", "keyd", "restore", &id],
        b"",
        &s.fs,
        &mut sys
    )
    .is_err());
    assert_eq!(
        fs::read_to_string(s.config("mcsr-toolkit")).unwrap(),
        FRAGMENT.replace("s = s", "s = clearm(s)")
    );
}

#[test]
fn failed_restart_rolls_back_and_failed_stop_reports_unsafe() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        fail_restart_at: Some(1),
        ..Default::default()
    };
    assert!(dispatch(
        &["privileged", "keyd", "install"],
        FRAGMENT.as_bytes(),
        &s.fs,
        &mut sys
    )
    .is_err());
    assert_eq!(
        fs::read_dir(s.dir.path().join("etc/keyd")).unwrap().count(),
        0
    );
    assert!(sys.calls.starts_with(&["restart", "stop"]));
    assert_eq!(sys.restart_count, 2);
    let mut bad_stop = FakeSystem {
        fail_restart_at: Some(1),
        fail_stop: true,
        ..Default::default()
    };
    let error = dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut bad_stop).unwrap_err();
    assert!(error.contains("unsafe"), "{error}");
    assert_eq!(bad_stop.calls, ["restart", "stop"]);
}

#[test]
fn health_check_is_bounded_and_stops_on_gap() {
    let s = Sandbox::new();
    let mut timeout = FakeSystem {
        fail_health: true,
        ..Default::default()
    };
    assert!(
        dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut timeout)
            .unwrap_err()
            .contains("health")
    );
    assert_eq!(timeout.calls.last(), Some(&"stop"));
    assert!(timeout.now <= Duration::from_millis(5100));
    let mut gap = FakeSystem {
        slow_health_at: Some(2),
        ..Default::default()
    };
    assert!(
        dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut gap)
            .unwrap_err()
            .contains("stability")
    );
    assert_eq!(gap.calls.last(), Some(&"stop"));
}

#[test]
fn clear_retries_initial_ipc_errors_then_requires_stable_readiness() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        initial_errors: 2,
        ..Default::default()
    };
    let result = dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys);
    assert!(result.is_ok(), "{result:?}");
    assert!(sys.now >= Duration::from_millis(700));
    assert!(!sys.calls.contains(&"stop"));
}

#[test]
fn readiness_timeout_preserves_last_ipc_error_and_stop_uncertainty() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        initial_errors: usize::MAX,
        fail_stop: true,
        ..Default::default()
    };
    let error = dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).unwrap_err();
    assert!(error.contains("timed out"), "{error}");
    assert!(error.contains("IPC connection unavailable"), "{error}");
    assert!(error.contains("input state unsafe"), "{error}");
    assert_eq!(sys.now, Duration::from_secs(5));
}

#[test]
fn initial_probe_finishing_after_deadline_is_rejected() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        probe_delay: Duration::from_millis(5001),
        ..Default::default()
    };
    assert!(
        dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys)
            .unwrap_err()
            .contains("timed out")
    );
    assert_eq!(sys.calls.last(), Some(&"stop"));
}

#[test]
fn readiness_loss_during_stability_stops_immediately() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        fail_health_at: Some(2),
        ..Default::default()
    };
    assert!(
        dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys)
            .unwrap_err()
            .contains("stability")
    );
    assert_eq!(sys.health_count, 2);
    assert_eq!(sys.calls.last(), Some(&"stop"));
}

#[test]
fn polling_passes_only_remaining_initial_and_stability_budget() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        initial_errors: 1,
        probe_delay: Duration::from_millis(50),
        ..Default::default()
    };
    dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).unwrap();
    assert_eq!(
        &sys.budgets[..2],
        &[Duration::from_millis(5000), Duration::from_millis(4850)]
    );
    assert_eq!(
        sys.budgets[2..],
        [100, 100, 100, 150].map(Duration::from_millis)
    );
    assert!(!sys.calls.contains(&"stop"));
}

#[test]
fn ipc_error_after_readiness_starts_stops_without_retry() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        error_health_at: Some(2),
        ..Default::default()
    };
    let error = dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).unwrap_err();
    assert!(error.contains("stability"), "{error}");
    assert!(error.contains("IPC connection unavailable"), "{error}");
    assert_eq!(sys.health_count, 2);
    assert_eq!(sys.calls.last(), Some(&"stop"));
}

#[test]
fn not_ready_observations_retain_last_probe_error_on_timeout() {
    let s = Sandbox::new();
    let mut sys = FakeSystem {
        initial_errors: 1,
        fail_health: true,
        ..Default::default()
    };
    let error = dispatch(&["privileged", "keyd", "clear"], b"", &s.fs, &mut sys).unwrap_err();
    assert!(error.contains("timed out"), "{error}");
    assert!(error.contains("IPC connection unavailable"), "{error}");
    assert_eq!(sys.calls.last(), Some(&"stop"));
}
