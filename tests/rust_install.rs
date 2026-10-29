use mcsr_toolkit::install::{
    install, preflight, repair, uninstall, Artifact, Environment, ExtensionAction, ExtensionState,
    FileState, InstallError, InstallGuard, RootFile, RootSetup, ServiceAction, System, HELPER_PATH,
    SERVICE_PATH,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, Sender},
    sync::Arc,
    sync::Mutex,
    thread,
    time::Duration,
};
const UNIT_PATH: &str = "/home/alice/.config/systemd/user/mcsr-toolkit.service";
const MARKER_PATH: &str = "/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json";
const EXTENSION_DIR: &str =
    "/home/alice/.local/share/gnome-shell/extensions/mcsr-toolkit@undecember.github.io";
const MANIFEST_PATH: &str = "/home/alice/.local/share/mcsr-toolkit/rust-v1/extension-manifest.json";

struct Fake {
    root: tempfile::TempDir,
    env: Mutex<Environment>,
    files: Mutex<BTreeMap<PathBuf, FileState>>,
    events: Mutex<Vec<String>>,
    fail: Mutex<BTreeSet<String>>,
    service_running: Mutex<bool>,
    service_enabled: Mutex<bool>,
    extension_running: Mutex<bool>,
    legacy_running: Mutex<bool>,
    legacy_enabled: Mutex<bool>,
    new_error: Mutex<bool>,
    new_configured: Mutex<bool>,
    enable_sets_error: Mutex<bool>,
    extension_disable_no_effect: Mutex<bool>,
    service_disable_no_effect: Mutex<bool>,
    object_stuck: Mutex<bool>,
    panic_after_legacy_disable: Mutex<bool>,
    extension_known: Mutex<bool>,
    shell_instance: Mutex<String>,
    enable_no_effect: Mutex<bool>,
    enable_mutates_then_errors: Mutex<bool>,
    inject_enabled_probe_error: Mutex<bool>,
    fail_enabled_probe_after_enable: Mutex<bool>,
    fail_after_extension_write: Mutex<bool>,
    corrupt_helper_on_write: Mutex<bool>,
    fail_root_after_first: Mutex<bool>,
    operation_lock: Mutex<()>,
    root_pause: Mutex<Option<RootPause>>,
    stop_hook: Mutex<Option<StopHook>>,
    dropins: Mutex<Vec<PathBuf>>,
    resolved_command: Mutex<Option<String>>,
    exit_after_start: Mutex<bool>,
    ready: Mutex<bool>,
    ready_sequence: Mutex<std::collections::VecDeque<bool>>,
}
#[derive(Clone, Copy)]
enum StopHook {
    KeydLineage,
    KeydUncertain,
    KeydRootPending,
    MaccelRootPending,
}
struct RootPause {
    entered: Sender<()>,
    release: Receiver<()>,
}

impl Fake {
    fn set_legacy_active(&self) {
        *self.legacy_running.lock().unwrap() = true;
        *self.legacy_enabled.lock().unwrap() = true;
        self.put(
            "/home/alice/.local/share/gnome-shell/extensions/mcsr-toolkit@local/metadata.json",
            b"legacy kept",
        );
    }
    fn legacy_is_active(&self) -> bool {
        *self.legacy_running.lock().unwrap()
    }
    fn set_new_error(&self) {
        *self.new_error.lock().unwrap() = true;
    }
    fn clear_new_error_and_activate(&self) {
        *self.new_error.lock().unwrap() = false;
        *self.extension_running.lock().unwrap() = true;
    }
    fn new() -> Self {
        let fake = Self {
            root: tempfile::tempdir().unwrap(),
            env: Mutex::new(Environment::supported(
                "alice",
                PathBuf::from("/home/alice"),
            )),
            files: Mutex::new(BTreeMap::new()),
            events: Mutex::new(Vec::new()),
            fail: Mutex::new(BTreeSet::new()),
            service_running: Mutex::new(false),
            service_enabled: Mutex::new(false),
            extension_running: Mutex::new(false),
            legacy_running: Mutex::new(false),
            legacy_enabled: Mutex::new(false),
            new_error: Mutex::new(false),
            new_configured: Mutex::new(false),
            enable_sets_error: Mutex::new(false),
            extension_disable_no_effect: Mutex::new(false),
            service_disable_no_effect: Mutex::new(false),
            object_stuck: Mutex::new(false),
            panic_after_legacy_disable: Mutex::new(false),
            extension_known: Mutex::new(true),
            shell_instance: Mutex::new("shell-1".into()),
            enable_no_effect: Mutex::new(false),
            enable_mutates_then_errors: Mutex::new(false),
            inject_enabled_probe_error: Mutex::new(false),
            fail_enabled_probe_after_enable: Mutex::new(false),
            fail_after_extension_write: Mutex::new(false),
            corrupt_helper_on_write: Mutex::new(false),
            fail_root_after_first: Mutex::new(false),
            operation_lock: Mutex::new(()),
            root_pause: Mutex::new(None),
            stop_hook: Mutex::new(None),
            dropins: Mutex::new(Vec::new()),
            resolved_command: Mutex::new(None),
            exit_after_start: Mutex::new(false),
            ready: Mutex::new(true),
            ready_sequence: Mutex::new(std::collections::VecDeque::new()),
        };
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(
            mcsr_toolkit::assets::extension_archive(),
        ))
        .unwrap();
        for name in [
            "extension.js",
            "metadata.json",
            "schemas/org.mcsr.toolkit.gschema.xml",
            "schemas/gschemas.compiled",
        ] {
            let mut bytes = Vec::new();
            archive
                .by_name(name)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            fake.put(&format!("{EXTENSION_DIR}/{name}"), &bytes);
        }
        fake
    }
    fn unseen_extension(&self) {
        self.files
            .lock()
            .unwrap()
            .retain(|path, _| !path.starts_with(EXTENSION_DIR));
        *self.extension_known.lock().unwrap() = false;
    }
    fn record_extension_lineage(&self) {
        let files = self.files.lock().unwrap();
        let mut digest = Sha256::new();
        let mut fingerprints = Vec::new();
        for name in [
            "extension.js",
            "metadata.json",
            "schemas/org.mcsr.toolkit.gschema.xml",
            "schemas/gschemas.compiled",
        ] {
            let path = format!("{EXTENSION_DIR}/{name}");
            let bytes = &files.get(Path::new(&path)).unwrap().bytes;
            let file_digest = format!("{:x}", Sha256::digest(bytes));
            digest.update(path.as_bytes());
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(0o644u32.to_le_bytes());
            digest.update(file_digest.as_bytes());
            fingerprints.push(serde_json::json!({
                "digest": file_digest,
                "mode": 0o644,
                "size": bytes.len() as u64,
            }));
        }
        drop(files);
        self.put(
            MANIFEST_PATH,
            format!(
                "{}\n",
                serde_json::json!({
                    "digest": format!("{:x}", digest.finalize()),
                    "files": fingerprints,
                })
            )
            .as_bytes(),
        );
        self.files
            .lock()
            .unwrap()
            .get_mut(Path::new(MANIFEST_PATH))
            .unwrap()
            .mode = 0o600;
    }
    fn artifact(&self) -> Artifact {
        Artifact::from_bytes(b"one downloaded executable".to_vec()).unwrap()
    }
    fn get(&self, p: &str) -> Option<FileState> {
        self.files.lock().unwrap().get(Path::new(p)).cloned()
    }
    fn put(&self, p: &str, bytes: &[u8]) {
        self.files.lock().unwrap().insert(
            p.into(),
            FileState {
                bytes: bytes.to_vec(),
                uid: unsafe { libc::geteuid() },
                mode: 0o644,
            },
        );
    }
    fn fail_on(&self, step: &str) {
        self.fail.lock().unwrap().insert(step.into());
    }
    fn step(&self, step: &str) -> Result<(), InstallError> {
        self.events.lock().unwrap().push(step.into());
        if self.fail.lock().unwrap().contains(step) {
            Err(InstallError::System(step.into()))
        } else {
            Ok(())
        }
    }
}

impl System for Fake {
    fn lock_install(&self) -> Result<Box<dyn InstallGuard + '_>, InstallError> {
        Ok(Box::new(
            self.operation_lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ))
    }
    fn environment(&self) -> Result<Environment, InstallError> {
        Ok(self.env.lock().unwrap().clone())
    }
    fn read(&self, path: &Path) -> Result<Option<FileState>, InstallError> {
        Ok(self.files.lock().unwrap().get(path).cloned())
    }
    fn write_user(&self, path: &Path, bytes: &[u8], mode: u32) -> Result<(), InstallError> {
        self.step("write_user")?;
        let disk = self.root.path().join(path.strip_prefix("/").unwrap());
        std::fs::create_dir_all(disk.parent().unwrap()).unwrap();
        std::fs::write(disk, bytes).unwrap();
        self.files.lock().unwrap().insert(
            path.into(),
            FileState {
                bytes: bytes.to_vec(),
                uid: unsafe { libc::geteuid() },
                mode,
            },
        );
        if path.ends_with("install-pending.json") {
            self.step("sync_marker")?;
        }
        if path == Path::new(&format!("{EXTENSION_DIR}/metadata.json"))
            && std::mem::take(&mut *self.fail_after_extension_write.lock().unwrap())
        {
            return Err(InstallError::System(
                "extension write interrupted after replacement".into(),
            ));
        }
        Ok(())
    }
    fn remove_user(&self, path: &Path) -> Result<(), InstallError> {
        if path.ends_with("install-pending.json") {
            self.step("remove_marker")?;
        } else {
            self.step("remove_user")?;
        }
        self.files.lock().unwrap().remove(path);
        Ok(())
    }
    fn write_root(&self, files: &[RootFile]) -> Result<(), InstallError> {
        if let Some(pause) = self.root_pause.lock().unwrap().take() {
            pause.entered.send(()).unwrap();
            pause.release.recv().unwrap();
        }
        self.step("write_root")?;
        for (index, file) in files.iter().enumerate() {
            if self.files.lock().unwrap().contains_key(&file.path) {
                return Err(InstallError::System(
                    "root destination already exists".into(),
                ));
            }
            if file.path == Path::new("/etc/sudoers.d/mcsr-toolkit-rust-keyd")
                || file.path == Path::new("/etc/sudoers.d/mcsr-toolkit-rust-maccel")
            {
                self.events.lock().unwrap().push("policy_published".into());
            }
            self.files.lock().unwrap().insert(
                file.path.clone(),
                FileState {
                    bytes: file.bytes.clone(),
                    uid: 0,
                    mode: file.mode,
                },
            );
            if file.path == Path::new(HELPER_PATH) && *self.corrupt_helper_on_write.lock().unwrap()
            {
                self.files
                    .lock()
                    .unwrap()
                    .get_mut(Path::new(HELPER_PATH))
                    .unwrap()
                    .bytes = b"tampered".to_vec();
            }
            if index == 0 && *self.fail_root_after_first.lock().unwrap() {
                return Err(InstallError::System("root stage interrupted".into()));
            }
        }
        Ok(())
    }
    fn remove_root(&self, paths: &[PathBuf]) -> Result<(), InstallError> {
        self.step("remove_root")?;
        for p in paths {
            self.files.lock().unwrap().remove(p);
        }
        Ok(())
    }
    fn validate_sudoers(&self, rule: &[u8]) -> Result<(), InstallError> {
        self.step("validate_sudoers")?;
        assert!(!rule.contains(&b'*'));
        Ok(())
    }
    fn begin_root_setup(
        &self,
        artifact: &Artifact,
    ) -> Result<Box<dyn RootSetup + '_>, InstallError> {
        self.step("begin_root_setup")?;
        let mut files = vec![
            RootFile {
                path: SERVICE_PATH.into(),
                bytes: artifact.bytes().to_vec(),
                mode: 0o755,
            },
            RootFile {
                path: HELPER_PATH.into(),
                bytes: artifact.bytes().to_vec(),
                mode: 0o755,
            },
        ];
        for (path, template) in [
            (
                "/etc/sudoers.d/mcsr-toolkit-rust-keyd",
                include_str!("../packaging/mcsr-toolkit-rust-keyd.sudoers"),
            ),
            (
                "/etc/sudoers.d/mcsr-toolkit-rust-maccel",
                include_str!("../packaging/mcsr-toolkit-rust-maccel.sudoers"),
            ),
        ] {
            files.push(RootFile {
                path: path.into(),
                bytes: template
                    .replace("@DESKTOP_USER@", &self.env.lock().unwrap().login)
                    .into_bytes(),
                mode: 0o440,
            });
        }
        files.retain(|file| self.get(file.path.to_str().unwrap()).is_none());
        let created: Vec<PathBuf> = files.iter().map(|file| file.path.clone()).collect();
        let result = (|| {
            let binaries: Vec<_> = files
                .iter()
                .filter(|file| !file.path.starts_with("/etc/sudoers.d"))
                .cloned()
                .collect();
            let policies: Vec<_> = files
                .iter()
                .filter(|file| file.path.starts_with("/etc/sudoers.d"))
                .cloned()
                .collect();
            if !binaries.is_empty() {
                self.write_root(&binaries)?;
            }
            if self
                .get(HELPER_PATH)
                .as_ref()
                .is_none_or(|state| state.bytes != artifact.bytes())
            {
                return Err(InstallError::Refused(
                    "bootstrap helper hash mismatch".into(),
                ));
            }
            if !policies.is_empty() {
                self.write_root(&policies)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            let published: Vec<PathBuf> = created
                .into_iter()
                .filter(|p| self.files.lock().unwrap().contains_key(p))
                .collect();
            self.remove_root(&published)?;
            return Err(error);
        }
        Ok(Box::new(FakeSetup {
            fake: self,
            created,
        }))
    }
    fn begin_root_uninstall(&self) -> Result<Box<dyn RootSetup + '_>, InstallError> {
        self.step("begin_root_uninstall")?;
        if self.env.lock().unwrap().keyd_recovery_pending
            || self.env.lock().unwrap().maccel_recovery_pending
        {
            return Err(InstallError::Refused("input recovery pending".into()));
        }
        Ok(Box::new(FakeUninstallSetup { fake: self }))
    }
    fn service(&self, action: ServiceAction) -> Result<(), InstallError> {
        if action == ServiceAction::Disable && self.get(UNIT_PATH).is_none() {
            self.step("service:DisableMissing")?;
        }
        if action == ServiceAction::Start {
            *self.service_running.lock().unwrap() = true;
        }
        if action == ServiceAction::Stop {
            *self.service_running.lock().unwrap() = false;
            if let Some(hook) = self.stop_hook.lock().unwrap().take() {
                match hook {
                    StopHook::KeydLineage => self.put(
                        "/home/alice/.local/share/mcsr-toolkit/rust-v1/keyd.json",
                        br#"{"backups":["abc"]}"#,
                    ),
                    StopHook::KeydUncertain => self.put(
                        "/home/alice/.local/share/mcsr-toolkit/rust-v1/keyd-uncertain.json",
                        b"pending",
                    ),
                    StopHook::KeydRootPending => {
                        self.env.lock().unwrap().keyd_recovery_pending = true
                    }
                    StopHook::MaccelRootPending => {
                        self.env.lock().unwrap().maccel_recovery_pending = true
                    }
                }
            }
        }
        if action == ServiceAction::Enable {
            *self.service_enabled.lock().unwrap() = true;
        }
        if action == ServiceAction::Disable && *self.service_disable_no_effect.lock().unwrap() {
            return self.step("service:Disable");
        }
        if action == ServiceAction::Disable {
            *self.service_enabled.lock().unwrap() = false;
        }
        self.step(&format!("service:{action:?}"))
    }
    fn extension(&self, action: ExtensionAction) -> Result<(), InstallError> {
        self.step(&format!("extension:{action:?}"))?;
        if !*self.extension_known.lock().unwrap() {
            return Err(InstallError::System(
                "extension does not exist in Shell catalog".into(),
            ));
        }
        if action == ExtensionAction::Enable && *self.legacy_running.lock().unwrap() {
            *self.new_error.lock().unwrap() = true;
            *self.new_configured.lock().unwrap() = true;
            return Ok(());
        }
        if action == ExtensionAction::Enable && *self.enable_sets_error.lock().unwrap() {
            *self.new_error.lock().unwrap() = true;
            *self.new_configured.lock().unwrap() = true;
            return Ok(());
        }
        if action == ExtensionAction::Disable && *self.extension_disable_no_effect.lock().unwrap() {
            return Ok(());
        }
        if action != ExtensionAction::Enable || !*self.enable_no_effect.lock().unwrap() {
            *self.extension_running.lock().unwrap() = action == ExtensionAction::Enable;
            *self.new_configured.lock().unwrap() = action == ExtensionAction::Enable;
        }
        if action == ExtensionAction::Enable && *self.enable_mutates_then_errors.lock().unwrap() {
            if *self.inject_enabled_probe_error.lock().unwrap() {
                *self.fail_enabled_probe_after_enable.lock().unwrap() = true;
            }
            return Err(InstallError::System(
                "extension enable failed after changing state".into(),
            ));
        }
        Ok(())
    }
    fn service_active(&self) -> Result<bool, InstallError> {
        if *self.exit_after_start.lock().unwrap() {
            *self.service_running.lock().unwrap() = false;
        }
        Ok(*self.service_running.lock().unwrap())
    }
    fn service_dropins(&self) -> Result<Vec<PathBuf>, InstallError> {
        Ok(self.dropins.lock().unwrap().clone())
    }
    fn service_command(&self) -> Result<String, InstallError> {
        Ok(self
            .resolved_command
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                if self.get(UNIT_PATH).is_some_and(|file| {
                    file.bytes == include_bytes!("../packaging/mcsr-toolkit.service")
                }) {
                    "/usr/local/bin/mcsr-toolkit serve".into()
                } else {
                    "old Python service".into()
                }
            }))
    }
    fn service_ready(&self) -> Result<bool, InstallError> {
        Ok(self
            .ready_sequence
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(*self.ready.lock().unwrap()))
    }
    fn wait_readiness_poll(&self) {}
    fn service_enabled(&self) -> Result<bool, InstallError> {
        Ok(*self.service_enabled.lock().unwrap())
    }
    fn extension_enabled(&self) -> Result<bool, InstallError> {
        if std::mem::take(&mut *self.fail_enabled_probe_after_enable.lock().unwrap()) {
            return Err(InstallError::System(
                "GNOME enabled-state probe failed".into(),
            ));
        }
        Ok(*self.extension_running.lock().unwrap() || *self.new_configured.lock().unwrap())
    }
    fn extension_known(&self) -> Result<bool, InstallError> {
        Ok(*self.extension_known.lock().unwrap())
    }
    fn extension_state(&self) -> Result<ExtensionState, InstallError> {
        if *self.new_error.lock().unwrap() {
            Ok(ExtensionState::Error)
        } else if *self.extension_running.lock().unwrap() {
            Ok(ExtensionState::Active)
        } else {
            Ok(ExtensionState::Inactive)
        }
    }
    fn legacy_extension_known(&self) -> Result<bool, InstallError> {
        Ok(self
            .get("/home/alice/.local/share/gnome-shell/extensions/mcsr-toolkit@local/metadata.json")
            .is_some())
    }
    fn legacy_extension_enabled(&self) -> Result<bool, InstallError> {
        Ok(*self.legacy_enabled.lock().unwrap())
    }
    fn legacy_extension_state(&self) -> Result<ExtensionState, InstallError> {
        Ok(if self.legacy_is_active() {
            ExtensionState::Active
        } else {
            ExtensionState::Inactive
        })
    }
    fn legacy_extension(&self, action: ExtensionAction) -> Result<(), InstallError> {
        self.step(&format!("legacy:{action:?}"))?;
        *self.legacy_enabled.lock().unwrap() = action == ExtensionAction::Enable;
        *self.legacy_running.lock().unwrap() = action == ExtensionAction::Enable;
        if action == ExtensionAction::Disable
            && std::mem::take(&mut *self.panic_after_legacy_disable.lock().unwrap())
        {
            panic!("simulated installer process loss after legacy disable");
        }
        Ok(())
    }
    fn toolkit_object_exported(&self) -> Result<bool, InstallError> {
        Ok(self.legacy_is_active()
            || *self.extension_running.lock().unwrap()
            || *self.object_stuck.lock().unwrap())
    }
    fn ensure_private_state(&self, _data_home: &Path) -> Result<(), InstallError> {
        self.step("ensure_private_state")
    }
    fn shell_instance(&self) -> Result<String, InstallError> {
        Ok(self.shell_instance.lock().unwrap().clone())
    }
    fn recover_input(&self) -> Result<(), InstallError> {
        self.step("recover_input")
    }
}

struct FakeSetup<'a> {
    fake: &'a Fake,
    created: Vec<PathBuf>,
}
impl RootSetup for FakeSetup<'_> {
    fn commit(self: Box<Self>) -> Result<(), InstallError> {
        self.fake.step("setup:COMMIT")
    }
    fn abort(self: Box<Self>) -> Result<(), InstallError> {
        self.fake.step("setup:ABORT")?;
        self.fake.remove_root(&self.created)
    }
}
struct FakeUninstallSetup<'a> {
    fake: &'a Fake,
}
impl RootSetup for FakeUninstallSetup<'_> {
    fn commit(self: Box<Self>) -> Result<(), InstallError> {
        if self
            .fake
            .fail
            .lock()
            .unwrap()
            .contains("uninstall:ABORT_WAIT_ERROR")
        {
            return mcsr_toolkit::install::classify_root_commit_response(
                "ABORTED\n",
                Err(InstallError::System("wait timed out".into())),
            );
        }
        if self
            .fake
            .fail
            .lock()
            .unwrap()
            .contains("uninstall:ABORT_AT_COMMIT")
        {
            return Err(InstallError::RootAborted(
                "root helper reported ABORTED".into(),
            ));
        }
        self.fake.step("uninstall:COMMIT")?;
        let paths: Vec<PathBuf> = [
            "/etc/sudoers.d/mcsr-toolkit-rust-keyd",
            "/etc/sudoers.d/mcsr-toolkit-rust-maccel",
            SERVICE_PATH,
            HELPER_PATH,
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        self.fake.remove_root(&paths)?;
        if self
            .fake
            .fail
            .lock()
            .unwrap()
            .contains("uninstall:LOST_RESPONSE")
        {
            return Err(InstallError::System("COMMITTED response lost".into()));
        }
        Ok(())
    }
    fn abort(self: Box<Self>) -> Result<(), InstallError> {
        self.fake.step("uninstall:ABORT")
    }
}

fn paused_install() -> (
    Arc<Fake>,
    Sender<()>,
    thread::JoinHandle<Result<(), InstallError>>,
) {
    let fake = Arc::new(Fake::new());
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    *fake.root_pause.lock().unwrap() = Some(RootPause {
        entered: entered_tx,
        release: release_rx,
    });
    let worker = Arc::clone(&fake);
    let handle = thread::spawn(move || install(&*worker, &worker.artifact()));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    (fake, release_tx, handle)
}

#[test]
fn unseen_extension_stages_files_without_root_setup_then_repairs_after_shell_restart() {
    let fake = Fake::new();
    fake.unseen_extension();
    let artifact = fake.artifact();
    let first = install(&fake, &artifact).unwrap_err().to_string();
    assert!(
        first.contains("Log out") && first.contains("repair"),
        "{first}"
    );
    assert!(String::from_utf8(fake.get(MARKER_PATH).unwrap().bytes)
        .unwrap()
        .contains("shell-restart"));
    assert!(fake.get(&format!("{EXTENSION_DIR}/extension.js")).is_some());
    assert!(preflight(&fake)
        .observations
        .iter()
        .any(|line| line.contains("Log out") && line.contains("repair")));
    assert!(fake.get(SERVICE_PATH).is_none());
    assert!(fake.get(UNIT_PATH).is_none());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    let before = fake.events.lock().unwrap().len();
    let again = repair(&fake, &artifact).unwrap_err().to_string();
    assert!(again.contains("Log out"), "{again}");
    assert_eq!(fake.events.lock().unwrap().len(), before + 1);
    *fake.extension_known.lock().unwrap() = true;
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    repair(&fake, &artifact).unwrap();
    assert!(fake.get(MARKER_PATH).is_none());
    assert!(fake.get(SERVICE_PATH).is_some());
    assert!(*fake.service_running.lock().unwrap());
    let after = fake.events.lock().unwrap().len();
    repair(&fake, &artifact).unwrap();
    assert_eq!(fake.events.lock().unwrap().len(), after + 1);
}

#[test]
fn exact_unregistered_extension_without_manifest_stages_then_repairs() {
    let fake = Fake::new();
    *fake.extension_known.lock().unwrap() = false;
    let before_files = fake.files.lock().unwrap().clone();
    let staged = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(
        staged.contains("Log out") && staged.contains("repair"),
        "{staged}"
    );
    for (path, state) in before_files {
        assert_eq!(fake.files.lock().unwrap().get(&path), Some(&state));
    }
    assert!(fake.get(MANIFEST_PATH).is_none());
    assert!(fake.get(UNIT_PATH).is_none());
    let events = fake.events.lock().unwrap();
    assert_eq!(
        events.iter().filter(|event| *event == "write_user").count(),
        2
    );
    assert!(!events.iter().any(|event| event == "begin_root_setup"));
    drop(events);
    let before_retry = fake.events.lock().unwrap().len();
    assert!(repair(&fake, &fake.artifact())
        .unwrap_err()
        .to_string()
        .contains("Log out"));
    assert_eq!(fake.events.lock().unwrap().len(), before_retry + 1);
    *fake.extension_known.lock().unwrap() = true;
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    repair(&fake, &fake.artifact()).unwrap();
    assert!(fake.get(MARKER_PATH).is_none());
    assert!(fake.get(SERVICE_PATH).is_some());
}

#[test]
fn partial_exact_unregistered_extension_stages_missing_file_only() {
    let fake = Fake::new();
    *fake.extension_known.lock().unwrap() = false;
    let missing = format!("{EXTENSION_DIR}/extension.js");
    fake.files.lock().unwrap().remove(Path::new(&missing));
    let staged = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(staged.contains("Log out"), "{staged}");
    assert!(fake.get(&missing).is_some());
    assert_eq!(
        fake.events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| *event == "write_user")
            .count(),
        3
    );
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event == "begin_root_setup"));
    assert!(repair(&fake, &fake.artifact())
        .unwrap_err()
        .to_string()
        .contains("Log out"));
}

#[test]
fn changed_owned_extension_with_verified_lineage_waits_for_new_shell() {
    let fake = Fake::new();
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(&format!("{EXTENSION_DIR}/extension.js")))
        .unwrap()
        .bytes
        .push(b' ');
    fake.record_extension_lineage();
    let staged = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(staged.contains("Log out"), "{staged}");
    assert!(fake.get(SERVICE_PATH).is_none());
    let same_session = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(same_session.contains("Log out"), "{same_session}");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    repair(&fake, &fake.artifact()).unwrap();
    assert!(fake.get(SERVICE_PATH).is_some());
}

#[test]
fn changed_extension_root_failure_preserves_shell_marker_and_old_lineage_for_retry() {
    let fake = Fake::new();
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(&format!("{EXTENSION_DIR}/extension.js")))
        .unwrap()
        .bytes
        .push(b' ');
    fake.record_extension_lineage();
    let old_manifest = fake.get(MANIFEST_PATH).unwrap().bytes;
    assert!(install(&fake, &fake.artifact()).is_err());
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    fake.fail_on("service:Start");
    assert!(repair(&fake, &fake.artifact()).is_err());
    assert!(String::from_utf8(fake.get(MARKER_PATH).unwrap().bytes)
        .unwrap()
        .contains("shell-restart"));
    assert_eq!(fake.get(MANIFEST_PATH).unwrap().bytes, old_manifest);
    fake.fail.lock().unwrap().clear();
    repair(&fake, &fake.artifact()).unwrap();
    assert!(fake.get(MARKER_PATH).is_none());
}

#[test]
fn failed_sticky_staging_preserves_prior_extension_lineage_for_retry() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(&format!("{EXTENSION_DIR}/extension.js")))
        .unwrap()
        .bytes
        .push(b' ');
    fake.record_extension_lineage();
    assert!(install(&fake, &fake.artifact())
        .unwrap_err()
        .to_string()
        .contains("Log out"));
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    fake.set_new_error();
    fake.fail_on("extension:Enable");
    assert!(repair(&fake, &fake.artifact()).is_err());
    assert!(fake.legacy_is_active());
    fake.fail.lock().unwrap().remove("extension:Enable");
    *fake.new_error.lock().unwrap() = false;
    repair(&fake, &fake.artifact()).unwrap();
    assert!(!fake.legacy_is_active());
}

#[test]
fn completed_manifest_write_with_shell_marker_recovers_after_interrupted_root_setup() {
    let fake = Fake::new();
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(&format!("{EXTENSION_DIR}/extension.js")))
        .unwrap()
        .bytes
        .push(b' ');
    fake.record_extension_lineage();
    assert!(install(&fake, &fake.artifact()).is_err());
    let known = Fake::new();
    install(&known, &known.artifact()).unwrap();
    let desired_manifest = known.get(MANIFEST_PATH).unwrap();
    fake.files
        .lock()
        .unwrap()
        .insert(MANIFEST_PATH.into(), desired_manifest);
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    repair(&fake, &fake.artifact()).unwrap();
    assert!(fake.get(MARKER_PATH).is_none());
    assert!(fake.get(SERVICE_PATH).is_some());
}

fn interrupted_changed_extension_stage() -> Fake {
    let fake = Fake::new();
    for name in [
        "extension.js",
        "metadata.json",
        "schemas/org.mcsr.toolkit.gschema.xml",
    ] {
        fake.files
            .lock()
            .unwrap()
            .get_mut(Path::new(&format!("{EXTENSION_DIR}/{name}")))
            .unwrap()
            .bytes
            .push(b' ');
    }
    fake.record_extension_lineage();
    *fake.fail_after_extension_write.lock().unwrap() = true;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("extension write interrupted"), "{error}");
    fake
}

#[test]
fn interrupted_mixed_extension_stage_resumes_from_durable_lineage() {
    let fake = interrupted_changed_extension_stage();
    assert!(fake
        .get(MARKER_PATH)
        .unwrap()
        .bytes
        .windows(17)
        .any(|bytes| bytes == b"extension-staging"));
    assert_ne!(
        fake.get(&format!("{EXTENSION_DIR}/extension.js"))
            .unwrap()
            .bytes
            .last(),
        Some(&b' ')
    );
    assert_eq!(
        fake.get(&format!(
            "{EXTENSION_DIR}/schemas/org.mcsr.toolkit.gschema.xml"
        ))
        .unwrap()
        .bytes
        .last(),
        Some(&b' ')
    );
    let staged = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(staged.contains("Log out"), "{staged}");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    repair(&fake, &fake.artifact()).unwrap();
    assert!(fake.get(MARKER_PATH).is_none());
}

#[test]
fn interrupted_stage_with_unrelated_file_change_refuses_repair() {
    let fake = interrupted_changed_extension_stage();
    fake.put(
        &format!("{EXTENSION_DIR}/extension.js"),
        b"unrelated extension",
    );
    let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("manual review"), "{error}");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    assert!(fake.get(MARKER_PATH).is_some());
}

#[test]
fn forged_staging_marker_cannot_authorize_foreign_extension_bytes() {
    for remove_manifest in [true, false] {
        let fake = interrupted_changed_extension_stage();
        let foreign = b"unrelated extension";
        fake.put(&format!("{EXTENSION_DIR}/extension.js"), foreign);
        let mut marker: serde_json::Value =
            serde_json::from_slice(&fake.get(MARKER_PATH).unwrap().bytes).unwrap();
        marker["prior_files"][0] = serde_json::json!({
            "digest": format!("{:x}", Sha256::digest(foreign)),
            "mode": 0o644,
            "size": foreign.len(),
        });
        if remove_manifest {
            fake.files.lock().unwrap().remove(Path::new(MANIFEST_PATH));
            marker["prior_manifest"] = serde_json::Value::Null;
        }
        fake.put(
            MARKER_PATH,
            serde_json::to_string(&marker).unwrap().as_bytes(),
        );
        let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
        assert!(error.contains("manual review"), "{error}");
        assert_eq!(
            fake.get(&format!("{EXTENSION_DIR}/extension.js"))
                .unwrap()
                .bytes,
            foreign
        );
        assert!(!fake
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e == "begin_root_setup"));
    }
}

#[test]
fn existing_repair_marker_and_verified_helper_stage_without_root_setup() {
    let fake = Fake::new();
    fake.unseen_extension();
    fake.put(MARKER_PATH, b"{\"phase\":\"repair\"}\n");
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(MARKER_PATH))
        .unwrap()
        .mode = 0o600;
    fake.files.lock().unwrap().insert(
        HELPER_PATH.into(),
        FileState {
            bytes: fake.artifact().bytes().to_vec(),
            uid: 0,
            mode: 0o755,
        },
    );
    let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("Log out"), "{error}");
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn foreign_extension_file_refuses_without_overwriting_or_root_setup() {
    let fake = Fake::new();
    fake.unseen_extension();
    fake.put(
        &format!("{EXTENSION_DIR}/extension.js"),
        b"foreign extension",
    );
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("manual review"), "{error}");
    assert_eq!(
        fake.get(&format!("{EXTENSION_DIR}/extension.js"))
            .unwrap()
            .bytes,
        b"foreign extension"
    );
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn foreign_manifest_is_not_overwritten_during_install() {
    let fake = Fake::new();
    fake.put(MANIFEST_PATH, b"foreign manifest");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("manual review"), "{error}");
    assert_eq!(fake.get(MANIFEST_PATH).unwrap().bytes, b"foreign manifest");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn malformed_pending_marker_cannot_skip_shell_restart_gate() {
    let fake = Fake::new();
    fake.put(MARKER_PATH, b"{\"phase\":\"shell-restart\"}");
    let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("manual review"), "{error}");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    assert!(fake.get(MARKER_PATH).is_some());
}

#[test]
fn extension_file_with_foreign_owner_refuses_before_staging() {
    let fake = Fake::new();
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(&format!("{EXTENSION_DIR}/metadata.json")))
        .unwrap()
        .uid = unsafe { libc::geteuid() } + 1;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("manual review"), "{error}");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn failed_extension_enable_before_mutation_does_not_disable_unknown_uuid_or_missing_unit() {
    let fake = Fake::new();
    fake.fail_on("extension:Enable");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("extension:Enable"), "{error}");
    let events = fake.events.lock().unwrap();
    assert!(!events.iter().any(|e| e == "extension:Disable"));
    assert!(!events
        .iter()
        .any(|e| e == "service:Disable" || e == "service:DisableMissing"));
    assert_eq!(
        fake.get(MARKER_PATH).unwrap().bytes,
        b"{\"phase\":\"repair\"}\n"
    );
}

#[test]
fn extension_enable_error_after_mutation_restores_prior_state() {
    let fake = Fake::new();
    *fake.enable_mutates_then_errors.lock().unwrap() = true;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("failed after changing state"), "{error}");
    assert!(!*fake.extension_running.lock().unwrap());
    assert!(fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "extension:Disable"));
    assert_eq!(
        fake.get(MARKER_PATH).unwrap().bytes,
        b"{\"phase\":\"repair\"}\n"
    );
}

#[test]
fn extension_probe_error_after_mutating_enable_restores_known_uuid() {
    let fake = Fake::new();
    *fake.enable_mutates_then_errors.lock().unwrap() = true;
    *fake.inject_enabled_probe_error.lock().unwrap() = true;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("enabled-state probe failed"), "{error}");
    assert!(!*fake.extension_running.lock().unwrap());
    assert!(fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "extension:Disable"));
    assert!(fake.get(MARKER_PATH).is_some());
}

#[test]
fn failure_after_extension_enable_restores_prior_disabled_state() {
    let fake = Fake::new();
    fake.fail_on("service:Start");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("service:Start"), "{error}");
    assert!(!*fake.extension_running.lock().unwrap());
    assert!(!*fake.service_enabled.lock().unwrap());
    assert!(fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "extension:Disable"));
}

#[test]
fn extension_enable_without_enabled_state_never_starts_root_setup() {
    let fake = Fake::new();
    *fake.enable_no_effect.lock().unwrap() = true;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("enabled"), "{error}");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    assert_eq!(
        fake.get(MARKER_PATH).unwrap().bytes,
        b"{\"phase\":\"repair\"}\n"
    );
}

#[test]
fn active_legacy_extension_hands_off_before_root_setup_and_remains_on_disk() {
    let fake = Fake::new();
    fake.set_legacy_active();
    install(&fake, &fake.artifact()).unwrap();
    assert!(!fake.legacy_is_active());
    assert!(*fake.extension_running.lock().unwrap());
    assert!(fake
        .get("/home/alice/.local/share/gnome-shell/extensions/mcsr-toolkit@local/metadata.json")
        .is_some());
    let events = fake.events.lock().unwrap();
    let disable = events.iter().position(|e| e == "legacy:Disable").unwrap();
    let enable = events.iter().position(|e| e == "extension:Enable").unwrap();
    let root = events.iter().position(|e| e == "begin_root_setup").unwrap();
    assert!(disable < enable && enable < root);
}

#[test]
fn legacy_collision_error_never_starts_root_and_restores_old() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.fail_on("extension:Enable");
    assert!(install(&fake, &fake.artifact()).is_err());
    assert!(fake.legacy_is_active());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn sticky_error_stages_single_uuid_for_next_shell_then_resumes_after_active() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.set_new_error();
    let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("Log out"), "{error}");
    assert!(!fake.legacy_is_active());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    let marker = fake.get(MARKER_PATH).unwrap();
    assert!(marker
        .bytes
        .windows(b"legacy-handoff-restart".len())
        .any(|part| part == b"legacy-handoff-restart"));
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    fake.clear_new_error_and_activate();
    repair(&fake, &fake.artifact()).unwrap();
    assert!(!fake.legacy_is_active());
    assert!(fake.get(MARKER_PATH).is_none());
}

#[test]
fn sticky_error_enable_command_failure_can_stage_when_new_is_configured() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.set_new_error();
    *fake.enable_mutates_then_errors.lock().unwrap() = true;
    let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("Log out"), "{error}");
    assert!(!fake.legacy_is_active());
    assert!(fake
        .get(MARKER_PATH)
        .unwrap()
        .bytes
        .windows(b"legacy-handoff-restart".len())
        .any(|part| part == b"legacy-handoff-restart"));
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn service_failure_after_legacy_handoff_restores_old_and_new_configuration() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.fail_on("service:Start");
    assert!(install(&fake, &fake.artifact()).is_err());
    assert!(fake.legacy_is_active());
    assert!(!*fake.extension_running.lock().unwrap());
    assert_eq!(
        fake.get(MARKER_PATH).unwrap().bytes,
        b"{\"phase\":\"repair\"}\n"
    );
}

#[test]
fn new_error_after_enable_restores_old_without_starting_root() {
    let fake = Fake::new();
    fake.set_legacy_active();
    *fake.enable_sets_error.lock().unwrap() = true;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("ERROR"), "{error}");
    assert!(fake.legacy_is_active());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn sticky_resume_error_restores_old_and_keeps_repair_marker() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.set_new_error();
    assert!(repair(&fake, &fake.artifact()).is_err());
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("prior @local state restored"), "{error}");
    assert!(fake.legacy_is_active());
    assert!(fake.get(MARKER_PATH).is_some());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn sticky_resume_service_failure_restores_old_and_can_retry() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.set_new_error();
    assert!(repair(&fake, &fake.artifact()).is_err());
    *fake.shell_instance.lock().unwrap() = "shell-2".into();
    fake.clear_new_error_and_activate();
    fake.fail_on("service:Start");
    assert!(repair(&fake, &fake.artifact()).is_err());
    assert!(fake.legacy_is_active());
    fake.fail.lock().unwrap().remove("service:Start");
    repair(&fake, &fake.artifact()).unwrap();
    assert!(!fake.legacy_is_active());
}

#[test]
fn failed_legacy_restore_reports_uncertainty_and_keeps_marker() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.fail_on("extension:Enable");
    fake.fail_on("legacy:Enable");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("rollback failed"), "{error}");
    assert!(fake.get(MARKER_PATH).is_some());
    let marker: serde_json::Value =
        serde_json::from_slice(&fake.get(MARKER_PATH).unwrap().bytes).unwrap();
    assert_eq!(marker["phase"], "handoff-uncertain");
    assert_eq!(marker["legacy_was_enabled"], true);
    assert_eq!(marker["new_was_enabled"], false);
    let before = fake.events.lock().unwrap().len();
    assert!(repair(&fake, &fake.artifact())
        .unwrap_err()
        .to_string()
        .contains("manual recovery"));
    assert_eq!(fake.events.lock().unwrap().len(), before);
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
}

#[test]
fn extension_disable_noop_reports_uncertain_rollback_before_old_reenable() {
    let fake = Fake::new();
    fake.set_legacy_active();
    *fake.extension_disable_no_effect.lock().unwrap() = true;
    fake.fail_on("service:Start");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("rollback failed"), "{error}");
    assert!(!fake.legacy_is_active());
    assert!(fake.extension_enabled().unwrap());
    let marker: serde_json::Value =
        serde_json::from_slice(&fake.get(MARKER_PATH).unwrap().bytes).unwrap();
    assert_eq!(marker["phase"], "handoff-uncertain");
}

#[test]
fn old_disabled_branch_detects_extension_disable_noop() {
    let fake = Fake::new();
    *fake.extension_disable_no_effect.lock().unwrap() = true;
    fake.fail_on("service:Start");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("rollback failed"), "{error}");
    assert!(fake.extension_enabled().unwrap());
}

#[test]
fn legacy_object_must_disappear_after_old_becomes_inactive() {
    let fake = Fake::new();
    fake.set_legacy_active();
    *fake.object_stuck.lock().unwrap() = true;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("release"), "{error}");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|e| e == "begin_root_setup"));
    *fake.object_stuck.lock().unwrap() = false;
    repair(&fake, &fake.artifact()).unwrap();
}

#[test]
fn foreign_or_permissive_marker_is_refused_before_handoff() {
    for foreign in [false, true] {
        let fake = Fake::new();
        fake.set_legacy_active();
        fake.put(MARKER_PATH, b"{\"phase\":\"repair\"}\n");
        let mut files = fake.files.lock().unwrap();
        let marker = files.get_mut(Path::new(MARKER_PATH)).unwrap();
        if foreign {
            marker.uid += 1;
            marker.mode = 0o600;
        } else {
            marker.mode = 0o664;
        }
        drop(files);
        let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
        assert!(error.contains("unsafe install marker"), "{error}");
        assert!(!fake
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|e| e == "legacy:Disable" || e == "begin_root_setup"));
    }
}

#[test]
fn identical_repair_start_failure_restores_prior_extension_and_unit_enablement() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.service_running.lock().unwrap() = false;
    *fake.service_enabled.lock().unwrap() = false;
    *fake.extension_running.lock().unwrap() = false;
    *fake.new_configured.lock().unwrap() = false;
    fake.fail_on("service:Start");
    assert!(repair(&fake, &fake.artifact()).is_err());
    assert!(!fake.extension_enabled().unwrap());
    assert!(!fake.service_enabled().unwrap());
    assert!(!fake.service_active().unwrap());
}

#[test]
fn identical_repair_uncertain_rollback_retains_prior_flags_and_blocks_retry() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.service_running.lock().unwrap() = false;
    *fake.service_enabled.lock().unwrap() = false;
    *fake.extension_running.lock().unwrap() = false;
    *fake.new_configured.lock().unwrap() = false;
    *fake.extension_disable_no_effect.lock().unwrap() = true;
    fake.fail_on("service:Start");
    let error = repair(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("rollback failed"), "{error}");
    let marker: serde_json::Value =
        serde_json::from_slice(&fake.get(MARKER_PATH).unwrap().bytes).unwrap();
    assert_eq!(marker["phase"], "handoff-uncertain");
    assert_eq!(marker["new_was_enabled"], false);
    let before = fake.events.lock().unwrap().len();
    assert!(repair(&fake, &fake.artifact())
        .unwrap_err()
        .to_string()
        .contains("manual recovery"));
    assert_eq!(fake.events.lock().unwrap().len(), before);
}

#[test]
fn service_disable_noop_after_failed_start_keeps_authoritative_marker() {
    let fake = Fake::new();
    *fake.service_disable_no_effect.lock().unwrap() = true;
    fake.fail_on("service:Start");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("rollback failed"), "{error}");
    assert!(fake.service_enabled().unwrap());
    let marker: serde_json::Value =
        serde_json::from_slice(&fake.get(MARKER_PATH).unwrap().bytes).unwrap();
    assert_eq!(marker["phase"], "handoff-uncertain");
    assert_eq!(marker["new_was_enabled"], false);
}

#[test]
fn conflicting_prior_enabled_uuids_are_not_silently_claimed_restored() {
    let fake = Fake::new();
    fake.set_legacy_active();
    *fake.new_configured.lock().unwrap() = true;
    fake.fail_on("service:Start");
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("both UUIDs"), "{error}");
    assert!(fake.legacy_is_active());
    assert!(!fake.extension_enabled().unwrap());
    assert!(fake.get(MARKER_PATH).is_some());
}

#[test]
fn interrupted_clean_handoff_retains_prior_state_for_repair_rollback() {
    let fake = Fake::new();
    fake.set_legacy_active();
    *fake.panic_after_legacy_disable.lock().unwrap() = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| install(
            &fake,
            &fake.artifact()
        )))
        .is_err()
    );
    assert!(!fake.legacy_is_active());
    let marker = fake.get(MARKER_PATH).unwrap();
    assert!(marker
        .bytes
        .windows(b"legacy_was_enabled".len())
        .any(|part| part == b"legacy_was_enabled"));
    fake.fail_on("extension:Enable");
    assert!(repair(&fake, &fake.artifact()).is_err());
    assert!(fake.legacy_is_active());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event == "begin_root_setup"));
}

#[test]
fn unsafe_state_directory_refuses_before_root_or_extension_switch() {
    let fake = Fake::new();
    fake.set_legacy_active();
    fake.fail_on("ensure_private_state");
    assert!(install(&fake, &fake.artifact()).is_err());
    let events = fake.events.lock().unwrap();
    assert!(!events
        .iter()
        .any(|e| e == "begin_root_setup" || e == "legacy:Disable"));
}

#[test]
fn install_and_repair_serialize_before_preflight_and_peer_rollback() {
    let (fake, release, install_thread) = paused_install();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let worker = Arc::clone(&fake);
    let repair_thread = thread::spawn(move || {
        started_tx.send(()).unwrap();
        done_tx.send(repair(&*worker, &worker.artifact())).unwrap();
    });
    started_rx.recv().unwrap();
    let early = done_rx.recv_timeout(Duration::from_millis(500));
    release.send(()).unwrap();
    let first = install_thread.join().unwrap();
    let second = match early {
        Ok(result) => result,
        Err(_) => done_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
    };
    repair_thread.join().unwrap();
    assert!(first.is_ok(), "{first:?}");
    assert!(second.is_ok(), "{second:?}");
    assert!(fake.get(HELPER_PATH).is_some());
}

#[test]
fn uninstall_waits_for_install_transaction_then_removes_its_files() {
    let (fake, release, install_thread) = paused_install();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let worker = Arc::clone(&fake);
    let uninstall_thread = thread::spawn(move || {
        started_tx.send(()).unwrap();
        done_tx.send(uninstall(&*worker)).unwrap();
    });
    started_rx.recv().unwrap();
    let early = done_rx.recv_timeout(Duration::from_millis(500));
    release.send(()).unwrap();
    let first = install_thread.join().unwrap();
    let second = match early {
        Ok(result) => result,
        Err(_) => done_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
    };
    uninstall_thread.join().unwrap();
    assert!(first.is_ok(), "{first:?}");
    assert!(second.is_ok(), "{second:?}");
    assert!(fake.get(HELPER_PATH).is_none());
}

#[test]
fn download_installs_identical_service_and_root_helper_then_repair_is_idempotent() {
    let fake = Fake::new();
    let artifact = fake.artifact();
    install(&fake, &artifact).unwrap();
    assert_eq!(fake.get(SERVICE_PATH).unwrap().bytes, artifact.bytes());
    let helper = fake.get(HELPER_PATH).unwrap();
    assert_eq!(helper.bytes, artifact.bytes());
    assert_eq!((helper.uid, helper.mode), (0, 0o755));
    assert!(String::from_utf8(fake.get(UNIT_PATH).unwrap().bytes)
        .unwrap()
        .contains("ExecStart=/usr/local/bin/mcsr-toolkit serve"));
    let setup_events = fake.events.lock().unwrap().clone();
    assert_eq!(
        setup_events
            .iter()
            .filter(|event| *event == "begin_root_setup")
            .count(),
        1
    );
    assert_eq!(
        setup_events
            .iter()
            .filter(|event| *event == "setup:COMMIT")
            .count(),
        1
    );
    assert!(!setup_events.iter().any(|event| event == "remove_root"));
    let before = fake.events.lock().unwrap().len();
    repair(&fake, &artifact).unwrap();
    assert_eq!(fake.events.lock().unwrap().len(), before + 1);
}

#[test]
fn immediate_service_exit_keeps_repair_marker_and_reports_failure() {
    let fake = Fake::new();
    *fake.exit_after_start.lock().unwrap() = true;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("active"), "{error}");
    assert!(fake
        .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
        .is_some());
}

#[test]
fn service_without_http_readiness_is_not_reported_installed() {
    let fake = Fake::new();
    *fake.ready.lock().unwrap() = false;
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("ready"), "{error}");
    assert!(fake.get(SERVICE_PATH).is_none());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_none());
    assert!(fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event == "setup:ABORT"));
}

#[test]
fn service_that_becomes_ready_after_two_probes_installs() {
    let fake = Fake::new();
    fake.ready_sequence
        .lock()
        .unwrap()
        .extend([false, false, true]);
    install(&fake, &fake.artifact()).unwrap();
    assert!(*fake.service_running.lock().unwrap());
    assert!(fake
        .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
        .is_none());
}

#[test]
fn documented_python_dropin_refuses_before_service_cutover() {
    let fake = Fake::new();
    fake.dropins
        .lock()
        .unwrap()
        .push("/home/alice/.config/systemd/user/mcsr-toolkit.service.d/override.conf".into());
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("drop-in"), "{error}");
    assert!(fake.events.lock().unwrap().is_empty());
}

#[test]
fn resolved_python_command_refuses_before_start_and_preserves_previous_unit() {
    let fake = Fake::new();
    fake.put(UNIT_PATH, b"previous unit");
    *fake.resolved_command.lock().unwrap() = Some("/home/alice/mcsr/.venv/bin/mcsr-toolkit".into());
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("ExecStart"), "{error}");
    assert_eq!(fake.get(UNIT_PATH).unwrap().bytes, b"previous unit");
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event == "service:Start"));
}

#[test]
fn resolved_extra_python_execstart_refuses_even_if_rust_command_is_present() {
    let fake = Fake::new();
    *fake.resolved_command.lock().unwrap() = Some("{ path=/usr/local/bin/mcsr-toolkit ; argv[]=/usr/local/bin/mcsr-toolkit serve ; } { path=/home/alice/.venv/bin/mcsr-toolkit ; argv[]=/home/alice/.venv/bin/mcsr-toolkit ; }".into());
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("ExecStart"), "{error}");
}

#[test]
fn identical_repair_restarts_stopped_service_and_reenables_extension() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.service_running.lock().unwrap() = false;
    *fake.service_enabled.lock().unwrap() = false;
    *fake.extension_running.lock().unwrap() = false;
    repair(&fake, &fake.artifact()).unwrap();
    assert!(*fake.service_running.lock().unwrap());
    assert!(*fake.service_enabled.lock().unwrap());
    assert!(*fake.extension_running.lock().unwrap());
}

#[test]
fn unsupported_desktop_and_python_recovery_refuse_before_mutation() {
    let fake = Fake::new();
    fake.env.lock().unwrap().gnome_major = 49;
    assert!(!preflight(&fake).ready());
    assert!(install(&fake, &fake.artifact()).is_err());
    fake.env.lock().unwrap().gnome_major = 50;
    fake.env.lock().unwrap().python_service_active = true;
    assert!(install(&fake, &fake.artifact()).is_err());
    fake.env.lock().unwrap().python_service_active = false;
    fake.env.lock().unwrap().other_toolkit_active = true;
    assert!(install(&fake, &fake.artifact()).is_err());
    fake.env.lock().unwrap().other_toolkit_active = false;
    fake.put(
        "/home/alice/.local/share/mcsr-toolkit/state/mutation.json",
        b"pending",
    );
    assert!(install(&fake, &fake.artifact()).is_err());
    assert!(fake.events.lock().unwrap().is_empty());
}

#[test]
fn failed_activation_restores_previous_unit_and_keeps_repair_marker() {
    let fake = Fake::new();
    fake.put(UNIT_PATH, b"old Python unit");
    fake.fail_on("service:Start");
    assert!(install(&fake, &fake.artifact()).is_err());
    assert_eq!(fake.get(UNIT_PATH).unwrap().bytes, b"old Python unit");
    assert!(fake
        .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
        .is_some());
    assert!(!*fake.service_running.lock().unwrap());
    assert!(!*fake.extension_running.lock().unwrap());
}

#[test]
fn tampered_helper_or_policy_fails_closed() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(HELPER_PATH))
        .unwrap()
        .uid = 1000;
    assert!(repair(&fake, &fake.artifact()).is_err());
    assert!(fake.events.lock().unwrap().last().unwrap() != "service:Start");
}

#[test]
fn uninstall_preserves_helpers_until_recovery_is_clear() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.env.lock().unwrap().maccel_recovery_pending = true;
    assert!(uninstall(&fake).is_err());
    assert!(fake.get(HELPER_PATH).is_some());
    fake.env.lock().unwrap().maccel_recovery_pending = false;
    fake.env.lock().unwrap().keyd_recovery_pending = true;
    assert!(uninstall(&fake).is_err());
    assert!(fake.get(HELPER_PATH).is_some());
    fake.env.lock().unwrap().keyd_recovery_pending = false;
    uninstall(&fake).unwrap();
    assert!(fake.get(HELPER_PATH).is_none());
    assert!(fake.get(SERVICE_PATH).is_none());
}

#[test]
fn uninstall_recovery_failure_retains_helpers_and_restarts_previous_service() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.fail_on("recover_input");
    let before = fake.events.lock().unwrap().len();
    assert!(uninstall(&fake)
        .unwrap_err()
        .to_string()
        .contains("recover_input"));
    assert!(*fake.service_running.lock().unwrap());
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
    assert!(fake.get(UNIT_PATH).is_some());
    let events = fake.events.lock().unwrap();
    assert_eq!(
        &events[before..],
        &["service:Stop", "recover_input", "service:Start"]
    );
}

#[test]
fn policy_grants_only_fixed_helper_commands_for_desktop_login() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    assert_eq!(
        fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd")
            .unwrap()
            .bytes,
        b"alice ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit privileged keyd clear\n"
    );
    assert_eq!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-maccel").unwrap().bytes,
        b"alice ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit privileged maccel eye, /usr/local/libexec/mcsr-toolkit privileged maccel restore\n");
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new("/etc/sudoers.d/mcsr-toolkit-rust-keyd"))
        .unwrap()
        .bytes
        .push(b'*');
    assert!(repair(&fake, &fake.artifact()).is_err());
}

#[test]
fn sudoers_reserved_all_login_is_rejected_before_policy_publication() {
    let fake = Fake::new();
    fake.env.lock().unwrap().login = "ALL".into();
    let error = install(&fake, &fake.artifact()).unwrap_err().to_string();
    assert!(error.contains("invalid desktop login"), "{error}");
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_none());
}

#[test]
fn root_stage_failure_leaves_previous_unit_and_repair_can_finish() {
    let fake = Fake::new();
    fake.put(UNIT_PATH, b"old inactive unit");
    fake.fail_on("write_root");
    assert!(install(&fake, &fake.artifact()).is_err());
    assert_eq!(fake.get(UNIT_PATH).unwrap().bytes, b"old inactive unit");
    assert!(fake.get(HELPER_PATH).is_none());
    fake.fail.lock().unwrap().clear();
    repair(&fake, &fake.artifact()).unwrap();
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake
        .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
        .is_none());
}

#[test]
fn failed_marker_directory_sync_prevents_root_setup() {
    let fake = Fake::new();
    fake.fail_on("sync_marker");
    assert!(install(&fake, &fake.artifact()).is_err());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event == "begin_root_setup"));
}

#[test]
fn doctor_preflight_is_read_only_and_reports_interrupted_install() {
    let fake = Fake::new();
    fake.put(
        "/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json",
        b"pending",
    );
    fake.files
        .lock()
        .unwrap()
        .get_mut(Path::new(MARKER_PATH))
        .unwrap()
        .mode = 0o600;
    let report = preflight(&fake);
    assert!(report.ready());
    assert!(report.repair_pending);
    assert!(fake.events.lock().unwrap().is_empty());
}

#[test]
fn uninstall_keeps_helper_when_keyd_lineage_remains_even_if_keyd_is_inactive() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.put(
        "/home/alice/.local/share/mcsr-toolkit/rust-v1/keyd.json",
        br#"{"backups":["abc"]}"#,
    );
    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("Restore"), "{error}");
    assert!(
        *fake.service_running.lock().unwrap(),
        "Restore needs the running service"
    );
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
}

#[test]
fn keyd_root_recovery_refusal_keeps_service_available_for_restore() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.env.lock().unwrap().keyd_recovery_pending = true;
    assert!(uninstall(&fake).is_err());
    assert!(*fake.service_running.lock().unwrap());
    assert!(fake.get(HELPER_PATH).is_some());
}

#[test]
fn lineage_created_during_stop_restores_prior_service_before_refusal() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.stop_hook.lock().unwrap() = Some(StopHook::KeydLineage);
    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("Restore"), "{error}");
    assert!(*fake.service_running.lock().unwrap());
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
}

#[test]
fn uncertainty_created_during_stop_restores_prior_service() {
    for hook in [
        StopHook::KeydUncertain,
        StopHook::KeydRootPending,
        StopHook::MaccelRootPending,
    ] {
        let fake = Fake::new();
        install(&fake, &fake.artifact()).unwrap();
        *fake.stop_hook.lock().unwrap() = Some(hook);
        assert!(uninstall(&fake).is_err());
        assert!(*fake.service_running.lock().unwrap());
        assert!(fake.get(HELPER_PATH).is_some());
        assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
    }
}

#[test]
fn uninstall_refuses_recovery_created_after_service_stop_before_root_mutation() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.stop_hook.lock().unwrap() = Some(StopHook::KeydRootPending);
    let before = fake.events.lock().unwrap().len();
    assert!(uninstall(&fake).is_err());
    let events = fake.events.lock().unwrap();
    assert!(events[before..].iter().any(|event| event == "service:Stop"));
    assert!(!events[before..].iter().any(|event| event == "remove_root"));
    assert!(fake.get(HELPER_PATH).is_some());
}

#[test]
fn uninstall_user_failure_aborts_one_root_transaction_and_restores_service() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.fail_on("service:Disable");
    let before = fake.events.lock().unwrap().len();
    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("service:Disable"), "{error}");
    let events = fake.events.lock().unwrap();
    assert_eq!(
        events[before..]
            .iter()
            .filter(|event| *event == "begin_root_uninstall")
            .count(),
        1
    );
    assert!(events[before..]
        .iter()
        .any(|event| event == "uninstall:ABORT"));
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get(UNIT_PATH).is_some());
    assert!(*fake.service_running.lock().unwrap());
}

#[test]
fn explicit_root_abort_at_commit_restores_user_unit_service_and_extension() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.fail_on("uninstall:ABORT_AT_COMMIT");
    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("ABORTED"), "{error}");
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get(UNIT_PATH).is_some());
    assert!(*fake.service_running.lock().unwrap());
    assert!(*fake.service_enabled.lock().unwrap());
    assert!(*fake.extension_running.lock().unwrap());
}

#[test]
fn aborted_root_commit_with_wait_error_still_restores_user_integration() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    fake.fail_on("uninstall:ABORT_WAIT_ERROR");
    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("wait timed out"), "{error}");
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get(UNIT_PATH).is_some());
    assert!(*fake.service_running.lock().unwrap());
    assert!(*fake.service_enabled.lock().unwrap());
    assert!(*fake.extension_running.lock().unwrap());
}

#[test]
fn repair_reconciles_completed_uninstall_marker_without_helper_despite_protected_history() {
    for failure in ["remove_marker", "uninstall:LOST_RESPONSE"] {
        let fake = Fake::new();
        install(&fake, &fake.artifact()).unwrap();
        fake.fail_on(failure);
        assert!(uninstall(&fake).is_err());
        assert!(fake.get(HELPER_PATH).is_none());
        assert!(fake
            .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
            .is_some());
        fake.fail.lock().unwrap().clear();
        fake.env.lock().unwrap().keyd_recovery_pending = true;
        repair(&fake, &fake.artifact()).unwrap();
        assert!(fake
            .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
            .is_none());
        assert!(fake.get(HELPER_PATH).is_none());
    }
}

#[test]
fn refusal_reports_restart_failure_and_retains_recovery_tools() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.stop_hook.lock().unwrap() = Some(StopHook::KeydLineage);
    fake.fail_on("service:Start");
    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("keyd backup lineage"), "{error}");
    assert!(error.contains("restart"), "{error}");
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
}

#[test]
fn recovery_failure_after_stop_restarts_prior_service_and_preserves_helpers() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.stop_hook.lock().unwrap() = Some(StopHook::KeydLineage);
    fake.fail_on("recover_input");

    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("recover_input"), "{error}");
    assert!(*fake.service_running.lock().unwrap());
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
    assert!(fake
        .get("/etc/sudoers.d/mcsr-toolkit-rust-maccel")
        .is_some());
}

#[test]
fn recovery_failure_and_restart_failure_report_both_and_preserve_helpers() {
    let fake = Fake::new();
    install(&fake, &fake.artifact()).unwrap();
    *fake.stop_hook.lock().unwrap() = Some(StopHook::KeydLineage);
    fake.fail_on("recover_input");
    fake.fail_on("service:Start");

    let error = uninstall(&fake).unwrap_err().to_string();
    assert!(error.contains("recover_input"), "{error}");
    assert!(error.contains("restart failed"), "{error}");
    assert!(error.contains("service:Start"), "{error}");
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
    assert!(fake
        .get("/etc/sudoers.d/mcsr-toolkit-rust-maccel")
        .is_some());
}

#[test]
fn corrupted_staged_helper_never_publishes_passwordless_policy() {
    let fake = Fake::new();
    *fake.corrupt_helper_on_write.lock().unwrap() = true;
    assert!(install(&fake, &fake.artifact()).is_err());
    assert!(!fake
        .events
        .lock()
        .unwrap()
        .iter()
        .any(|event| event == "policy_published"));
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_none());
}

#[test]
fn partial_root_write_keeps_repair_marker_and_can_be_repaired() {
    let fake = Fake::new();
    *fake.fail_root_after_first.lock().unwrap() = true;
    assert!(install(&fake, &fake.artifact()).is_err());
    assert!(fake.get(HELPER_PATH).is_none());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_none());
    assert!(fake
        .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
        .is_some());
    *fake.fail_root_after_first.lock().unwrap() = false;
    repair(&fake, &fake.artifact()).unwrap();
    assert!(fake.get(HELPER_PATH).is_some());
}

#[test]
fn marker_removal_failure_keeps_durable_repair_marker_and_verified_root_files() {
    let fake = Fake::new();
    fake.put(UNIT_PATH, b"prior inactive unit");
    fake.fail_on("remove_marker");
    assert!(install(&fake, &fake.artifact()).is_err());
    assert_eq!(fake.get(UNIT_PATH).unwrap().bytes, b"prior inactive unit");
    assert!(!*fake.service_running.lock().unwrap());
    assert!(fake.get(HELPER_PATH).is_some());
    assert!(fake.get("/etc/sudoers.d/mcsr-toolkit-rust-keyd").is_some());
    assert!(fake
        .get("/home/alice/.local/share/mcsr-toolkit/rust-v1/install-pending.json")
        .is_some());
    fake.fail.lock().unwrap().clear();
    repair(&fake, &fake.artifact()).unwrap();
}

#[test]
fn root_verification_errors_block_mutation_and_remain_visible_with_only_policies() {
    for reason in [
        "root status helper hash differs; manual review required",
        "authentication denied (exit status: 126)",
        "malformed authenticated root status response",
    ] {
        let fake = Fake::new();
        fake.put("/etc/sudoers.d/mcsr-toolkit-rust-keyd", b"existing policy");
        let before = fake.files.lock().unwrap().clone();
        fake.env.lock().unwrap().root_status_error = Some(reason.into());
        let report = preflight(&fake);
        assert!(
            report
                .problems
                .iter()
                .any(|problem| problem.contains(reason)),
            "{report:?}"
        );
        for error in [
            install(&fake, &fake.artifact()).unwrap_err(),
            uninstall(&fake).unwrap_err(),
        ] {
            assert!(error.to_string().contains(reason), "{error}");
        }
        assert_eq!(*fake.files.lock().unwrap(), before);
        assert!(
            fake.events.lock().unwrap().is_empty(),
            "verification must precede mutation"
        );
    }
}
