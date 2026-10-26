//! Fixed-target, root-owned keyd operations. Filesystem paths are injected for
//! sandbox tests; production always constructs the fixed paths below.

use crate::keyd::{run_command, CommandFailure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEVICE: &str = "# MCSR Toolkit managed\n[ids]\n*\n\ninclude mcsr-toolkit\n";
const DEVICE_NAME: &str = "mcsr-toolkit.conf";
const FRAGMENT_NAME: &str = "mcsr-toolkit";
const MAX_FRAGMENT: usize = 16_384;
const MAX_MANIFEST: u64 = 131_072;

pub trait DirectorySync: Send + Sync {
    fn sync_dir(&self, path: &Path) -> Result<(), String>;
}

struct RealDirectorySync;
impl DirectorySync for RealDirectorySync {
    fn sync_dir(&self, path: &Path) -> Result<(), String> {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|e| format!("cannot sync keyd directory: {e}"))
    }
}

#[derive(Clone)]
pub struct HelperFilesystem {
    config_dir: PathBuf,
    backup_root: PathBuf,
    helper_path: PathBuf,
    lock_path: PathBuf,
    trusted_root: PathBuf,
    required_uid: u32,
    directory_sync: Arc<dyn DirectorySync>,
}

impl HelperFilesystem {
    pub fn production() -> Self {
        Self {
            config_dir: PathBuf::from("/etc/keyd"),
            backup_root: PathBuf::from("/var/lib/mcsr-toolkit/keyd"),
            helper_path: PathBuf::from("/usr/local/libexec/mcsr-toolkit"),
            lock_path: PathBuf::from("/run/mcsr-toolkit-keyd.lock"),
            trusted_root: PathBuf::from("/"),
            required_uid: 0,
            directory_sync: Arc::new(RealDirectorySync),
        }
    }

    /// Restricts filesystem operations to a controlled tree in tests.
    pub fn sandbox(
        config_dir: PathBuf,
        backup_root: PathBuf,
        helper_path: PathBuf,
        trusted_root: PathBuf,
        required_uid: u32,
    ) -> Self {
        Self {
            config_dir,
            backup_root,
            helper_path,
            lock_path: trusted_root.join("run/mcsr-toolkit-keyd.lock"),
            trusted_root,
            required_uid,
            directory_sync: Arc::new(RealDirectorySync),
        }
    }

    pub fn with_config_dir(mut self, config_dir: PathBuf) -> Self {
        self.config_dir = config_dir;
        self
    }

    pub fn with_directory_sync(mut self, directory_sync: Arc<dyn DirectorySync>) -> Self {
        self.directory_sync = directory_sync;
        self
    }

    fn verify_directory(&self, path: &Path, private: bool) -> Result<(), String> {
        if !path.starts_with(&self.trusted_root) || !path.is_absolute() {
            return Err("unsafe keyd path".into());
        }
        for component in path.ancestors() {
            if !component.starts_with(&self.trusted_root) {
                break;
            }
            let metadata = fs::symlink_metadata(component)
                .map_err(|e| format!("unsafe keyd directory: {e}"))?;
            let mode = metadata.permissions().mode();
            if !metadata.is_dir()
                || metadata.file_type().is_symlink()
                || metadata.uid() != self.required_uid
                || mode & 0o022 != 0
                || (private && component == path && mode & 0o077 != 0)
            {
                return Err("unsafe keyd directory ownership or permissions".into());
            }
            if component == self.trusted_root {
                break;
            }
        }
        Ok(())
    }

    fn verify_file(&self, path: &Path, executable: bool, private: bool) -> Result<(), String> {
        self.verify_directory(path.parent().ok_or("unsafe keyd path")?, false)?;
        let metadata = fs::symlink_metadata(path).map_err(|e| format!("unsafe keyd file: {e}"))?;
        let mode = metadata.permissions().mode();
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.uid() != self.required_uid
            || mode & 0o022 != 0
            || mode & 0o7000 != 0
            || (private && mode & 0o077 != 0)
            || (executable && mode & 0o111 == 0)
        {
            return Err("unsafe keyd file ownership or permissions".into());
        }
        Ok(())
    }

    fn verify_executable(&self) -> Result<(), String> {
        self.verify_file(&self.helper_path, true, false)
    }

    pub(crate) fn lock_operation(&self) -> Result<File, String> {
        let parent = self.lock_path.parent().ok_or("unsafe keyd lock path")?;
        self.verify_directory(parent, false)?;
        let newly_created = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&self.lock_path);
        let (file, created) = match newly_created {
            Ok(file) => (file, true),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(&self.lock_path)
                    .map_err(|e| format!("unsafe keyd helper lock: {e}"))?,
                false,
            ),
            Err(error) => return Err(format!("cannot create keyd helper lock: {error}")),
        };
        self.verify_file(&self.lock_path, false, true)?;
        let opened = file.metadata().map_err(|e| e.to_string())?;
        let named = fs::symlink_metadata(&self.lock_path).map_err(|e| e.to_string())?;
        if opened.dev() != named.dev() || opened.ino() != named.ino() {
            return Err("keyd helper lock changed".into());
        }
        if created {
            file.sync_all().map_err(|e| e.to_string())?;
            self.directory_sync.sync_dir(parent)?;
        }
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("keyd helper busy".into());
        }
        Ok(file)
    }

    fn ensure_backup_root(&self) -> Result<(), String> {
        let mut missing = Vec::new();
        let mut existing = self.backup_root.as_path();
        while fs::symlink_metadata(existing)
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            missing.push(existing.to_path_buf());
            existing = existing.parent().ok_or("unsafe keyd backup path")?;
        }
        self.verify_directory(existing, false)?;
        for path in missing.iter().rev() {
            fs::create_dir(path)
                .map_err(|e| format!("cannot create keyd backup directory: {e}"))?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|e| format!("cannot secure keyd backup directory: {e}"))?;
            self.verify_directory(path, true)?;
            self.directory_sync.sync_dir(path)?;
            self.directory_sync
                .sync_dir(path.parent().ok_or("unsafe keyd backup path")?)?;
        }
        self.verify_directory(&self.backup_root, true)
    }
}

pub trait HelperSystem {
    fn list_keys(&mut self) -> Result<String, String>;
    fn systemctl(&mut self, action: &'static str) -> Result<bool, String>;
    fn now(&self) -> Duration;
    fn sleep(&mut self, duration: Duration);
}

pub struct RealSystem {
    started: Instant,
}
impl Default for RealSystem {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}
impl HelperSystem for RealSystem {
    fn list_keys(&mut self) -> Result<String, String> {
        let fs = HelperFilesystem::production();
        for path in ["/usr/bin/keyd.rvaiya", "/usr/bin/keyd"] {
            if fs.verify_file(Path::new(path), true, false).is_ok() {
                return run_command(
                    &[path.into(), "list-keys".into()],
                    &[],
                    Duration::from_secs(5),
                )
                .map(|result| result.stdout)
                .map_err(|e| format!("keyd list-keys failed: {e}"));
            }
        }
        Err("keyd client unavailable".into())
    }

    fn systemctl(&mut self, action: &'static str) -> Result<bool, String> {
        let args = match action {
            "restart" => vec!["/usr/bin/systemctl", "restart", "keyd.service"],
            "is-active" => vec!["/usr/bin/systemctl", "is-active", "--quiet", "keyd.service"],
            "stop" => vec!["/usr/bin/systemctl", "stop", "keyd.service"],
            _ => return Err("invalid keyd service action".into()),
        };
        match run_command(
            &args.into_iter().map(str::to_string).collect::<Vec<_>>(),
            &[],
            Duration::from_secs(5),
        ) {
            Ok(_) => Ok(true),
            Err(CommandFailure::Failed(_)) => Ok(false),
            Err(CommandFailure::TimedOut(message)) => Err(message),
        }
    }
    fn now(&self) -> Duration {
        self.started.elapsed()
    }
    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

fn restart_healthy(system: &mut dyn HelperSystem) -> Result<(), String> {
    let attempt = (|| -> Result<(), String> {
        if !system.systemctl("restart")? {
            return Err("keyd restart failed".into());
        }
        let deadline = system.now() + Duration::from_secs(5);
        let mut stable_since = None;
        let mut last_healthy = None;
        loop {
            let healthy = system.systemctl("is-active")?;
            let now = system.now();
            if stable_since.is_none() && now > deadline {
                return Err("keyd health check timed out".into());
            }
            if healthy {
                if let Some(since) = stable_since {
                    if last_healthy
                        .is_some_and(|last| now.saturating_sub(last) > Duration::from_millis(200))
                    {
                        return Err("keyd health check gap during stability window".into());
                    }
                    if now.saturating_sub(since) >= Duration::from_millis(500) {
                        return Ok(());
                    }
                } else {
                    stable_since = Some(now);
                }
                last_healthy = Some(now);
            } else if stable_since.is_some() {
                return Err("keyd health check failed during stability window".into());
            }
            if stable_since.is_none() && now >= deadline {
                return Err("keyd health check timed out".into());
            }
            let end = stable_since.map_or(deadline, |since| since + Duration::from_millis(500));
            let remaining = end.saturating_sub(now);
            system.sleep(remaining.min(Duration::from_millis(100)));
        }
    })();
    if let Err(failure) = attempt {
        match system.systemctl("stop") {
            Ok(true) => Err(failure),
            Ok(false) => Err(format!("keyd stop failed; input state unsafe: {failure}")),
            Err(error) => Err(format!(
                "keyd stop failed; input state unsafe: {error}; {failure}"
            )),
        }
    } else {
        Ok(())
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_key(key: &str) -> bool {
    let mut chars = key.bytes();
    chars
        .next()
        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && chars.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn validate_fragment(fragment: &str, keys: &HashSet<String>) -> Result<(), String> {
    if fragment.len() > MAX_FRAGMENT {
        return Err("invalid keyd fragment size".into());
    }
    let mut lines = fragment.split_terminator('\n');
    if lines.next() != Some("[mcsr_chat]") {
        return Err("invalid keyd fragment section".into());
    }
    let mut seen = HashSet::new();
    for line in lines {
        let (key, action) = line
            .split_once(" = ")
            .ok_or("invalid keyd fragment binding")?;
        if !valid_key(key) || !keys.contains(key) || !seen.insert(key) {
            return Err("invalid keyd fragment binding".into());
        }
        if action != key
            && action != format!("clearm({key})")
            && action != "clearm(enter)"
            && action != "clearm(esc)"
        {
            return Err("invalid keyd fragment action".into());
        }
    }
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    text: String,
    mode: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    device: Option<Snapshot>,
    fragment: Option<Snapshot>,
}

fn snapshot(fs_adapter: &HelperFilesystem, path: &Path) -> Result<Option<Snapshot>, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            fs_adapter.verify_file(path, false, false)?;
            let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
            let text = fs::read_to_string(path).map_err(|e| format!("invalid keyd target: {e}"))?;
            Ok(Some(Snapshot {
                text,
                mode: metadata.permissions().mode() & 0o777,
            }))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("cannot inspect keyd target: {e}")),
    }
}

fn current(fs_adapter: &HelperFilesystem, keys: &HashSet<String>) -> Result<Manifest, String> {
    fs_adapter.verify_directory(&fs_adapter.config_dir, false)?;
    for entry in fs::read_dir(&fs_adapter.config_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(".conf") && name != DEVICE_NAME {
            return Err("existing keyd config requires manual integration".into());
        }
    }
    let device = snapshot(fs_adapter, &fs_adapter.config_dir.join(DEVICE_NAME))?;
    let fragment = snapshot(fs_adapter, &fs_adapter.config_dir.join(FRAGMENT_NAME))?;
    let manifest = Manifest { device, fragment };
    validate_manifest(&manifest, keys)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &Manifest, keys: &HashSet<String>) -> Result<(), String> {
    if manifest.device.is_some() != manifest.fragment.is_some() {
        return Err("incomplete keyd backup or existing config".into());
    }
    for snapshot in [&manifest.device, &manifest.fragment].into_iter().flatten() {
        if snapshot.mode > 0o777 || snapshot.mode & 0o022 != 0 {
            return Err("invalid keyd backup snapshot".into());
        }
    }
    if manifest.device.as_ref().is_some_and(|s| s.text != DEVICE) {
        return Err("invalid keyd backup device config".into());
    }
    if let Some(fragment) = &manifest.fragment {
        validate_fragment(&fragment.text, keys)?;
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = path.parent().ok_or("unsafe keyd path")?;
    let mut temp = tempfile::Builder::new()
        .prefix(".mcsr-keyd-")
        .tempfile_in(parent)
        .map_err(|e| format!("cannot stage keyd file: {e}"))?;
    temp.as_file_mut()
        .set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|e| e.to_string())?;
    temp.write_all(bytes)
        .and_then(|()| temp.as_file_mut().sync_all())
        .map_err(|e| format!("cannot write keyd file: {e}"))?;
    temp.persist(path)
        .map_err(|e| format!("cannot install keyd file: {e}"))?;
    File::open(parent)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())
}

fn restore_snapshot(
    fs_adapter: &HelperFilesystem,
    path: &Path,
    wanted: &Option<Snapshot>,
) -> Result<(), String> {
    snapshot(fs_adapter, path)?;
    if let Some(wanted) = wanted {
        atomic_write(path, wanted.text.as_bytes(), wanted.mode)
    } else {
        match fs::remove_file(path) {
            Ok(()) => File::open(path.parent().ok_or("unsafe keyd path")?)
                .and_then(|file| file.sync_all())
                .map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn apply_snapshot(fs_adapter: &HelperFilesystem, manifest: &Manifest) -> Result<(), String> {
    restore_snapshot(
        fs_adapter,
        &fs_adapter.config_dir.join(DEVICE_NAME),
        &manifest.device,
    )?;
    restore_snapshot(
        fs_adapter,
        &fs_adapter.config_dir.join(FRAGMENT_NAME),
        &manifest.fragment,
    )
}

fn save_manifest(fs_adapter: &HelperFilesystem, manifest: &Manifest) -> Result<String, String> {
    fs_adapter.ensure_backup_root()?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let directory = fs_adapter.backup_root.join(&id);
    fs::create_dir(&directory).map_err(|e| format!("cannot create keyd snapshot: {e}"))?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
        .map_err(|e| e.to_string())?;
    fs_adapter.verify_directory(&directory, true)?;
    fs_adapter.directory_sync.sync_dir(&directory)?;
    fs_adapter
        .directory_sync
        .sync_dir(&fs_adapter.backup_root)?;
    let bytes = serde_json::to_vec(manifest).map_err(|e| e.to_string())?;
    atomic_write(&directory.join("manifest.json"), &bytes, 0o600)?;
    Ok(id)
}

fn load_manifest(
    fs_adapter: &HelperFilesystem,
    id: &str,
    keys: &HashSet<String>,
) -> Result<Manifest, String> {
    if !valid_id(id) {
        return Err("unknown keyd backup".into());
    }
    fs_adapter.verify_directory(&fs_adapter.backup_root, true)?;
    let directory = fs_adapter.backup_root.join(id);
    fs_adapter.verify_directory(&directory, true)?;
    let path = directory.join("manifest.json");
    fs_adapter.verify_file(&path, false, true)?;
    let mut bytes = Vec::new();
    File::open(&path)
        .map_err(|e| e.to_string())?
        .take(MAX_MANIFEST + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_MANIFEST {
        return Err("invalid keyd backup snapshot".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(&bytes).map_err(|_| "invalid keyd backup".to_string())?;
    validate_manifest(&manifest, keys)?;
    Ok(manifest)
}

pub fn recovery_pending(fs_adapter: &HelperFilesystem) -> Result<bool, String> {
    match fs::symlink_metadata(&fs_adapter.backup_root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
        Ok(_) => fs_adapter.verify_directory(&fs_adapter.backup_root, true)?,
    }
    for entry in fs::read_dir(&fs_adapter.backup_root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let id = entry.file_name();
        let id = id.to_str().ok_or("unexpected keyd snapshot entry")?;
        if !valid_id(id) {
            return Err("unexpected keyd snapshot entry".into());
        }
        let directory = entry.path();
        fs_adapter.verify_directory(&directory, true)?;
        let mut names = fs::read_dir(&directory).map_err(|e| e.to_string())?;
        let manifest_path = directory.join("manifest.json");
        let only_manifest = names
            .next()
            .transpose()
            .map_err(|e| e.to_string())?
            .is_some_and(|entry| entry.file_name() == "manifest.json")
            && names.next().is_none();
        if !only_manifest {
            return Err("unexpected keyd snapshot entry".into());
        }
        fs_adapter.verify_file(&manifest_path, false, true)?;
        let mut bytes = Vec::new();
        File::open(&manifest_path)
            .map_err(|e| e.to_string())?
            .take(MAX_MANIFEST + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_MANIFEST {
            return Err("invalid keyd backup snapshot".into());
        }
        let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        for snapshot in [&manifest.device, &manifest.fragment].into_iter().flatten() {
            if snapshot.mode > 0o777 || snapshot.mode & 0o022 != 0 {
                return Err("invalid keyd backup snapshot".into());
            }
        }
    }
    Ok(false)
}

pub(crate) fn status_under_lock(fs_adapter: &HelperFilesystem) -> Result<&'static str, String> {
    recovery_pending(fs_adapter)?;
    for name in [DEVICE_NAME, FRAGMENT_NAME] {
        match fs::symlink_metadata(fs_adapter.config_dir.join(name)) {
            Ok(_) => return Ok("active"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok("clear")
}

fn apply_and_restart(
    fs_adapter: &HelperFilesystem,
    wanted: &Manifest,
    previous: &Manifest,
    system: &mut dyn HelperSystem,
) -> Result<(), String> {
    let attempt = apply_snapshot(fs_adapter, wanted).and_then(|()| restart_healthy(system));
    if let Err(failure) = attempt {
        let rollback = apply_snapshot(fs_adapter, previous).and_then(|()| restart_healthy(system));
        return match rollback {
            Ok(()) => Err(failure),
            Err(rollback_failure) => Err(format!(
                "keyd operation and rollback failed: {failure}; {rollback_failure}"
            )),
        };
    }
    Ok(())
}

/// Returns one JSON response for install/restore, and no output for clear.
pub fn dispatch(
    args: &[&str],
    stdin: &[u8],
    fs_adapter: &HelperFilesystem,
    system: &mut dyn HelperSystem,
) -> Result<Option<String>, String> {
    let operation = match args {
        ["privileged", "keyd", "install"] if !stdin.is_empty() => "install",
        ["privileged", "keyd", "restore", id] if stdin.is_empty() && valid_id(id) => "restore",
        ["privileged", "keyd", "clear"] if stdin.is_empty() => "clear",
        ["privileged", "keyd", "status"] if stdin.is_empty() => "status",
        _ => return Err("invalid privileged keyd command or arguments".into()),
    };
    fs_adapter.verify_executable()?;
    let _operation_lock = fs_adapter.lock_operation()?;
    if fs_adapter.trusted_root == Path::new("/") && fs_adapter.required_uid == 0 {
        fs_adapter.verify_executable()?;
        let running = fs::metadata("/proc/self/exe").map_err(|e| e.to_string())?;
        let installed = fs::symlink_metadata(&fs_adapter.helper_path).map_err(|e| e.to_string())?;
        if running.dev() != installed.dev() || running.ino() != installed.ino() {
            return Err("keyd helper was replaced or removed while queued".into());
        }
    }
    if operation == "status" {
        return Ok(Some(status_under_lock(fs_adapter)?.into()));
    }
    if operation == "clear" {
        let config_safety = (|| -> Result<(), String> {
            let keys: HashSet<String> = system.list_keys()?.lines().map(str::to_string).collect();
            current(fs_adapter, &keys)?;
            Ok(())
        })();
        if let Err(error) = config_safety {
            return match system.systemctl("stop") {
                Ok(true) => Err(format!("unsafe keyd config: {error}")),
                Ok(false) => Err(format!("keyd stop failed; input state unsafe: {error}")),
                Err(stop_error) => Err(format!(
                    "keyd stop failed; input state unsafe: {stop_error}; {error}"
                )),
            };
        }
        restart_healthy(system)?;
        return Ok(None);
    }
    let keys: HashSet<String> = system.list_keys()?.lines().map(str::to_string).collect();
    if operation == "install" {
        let fragment =
            std::str::from_utf8(stdin).map_err(|_| "invalid keyd fragment encoding".to_string())?;
        validate_fragment(fragment, &keys)?;
        let previous = current(fs_adapter, &keys)?;
        let id = save_manifest(fs_adapter, &previous)?;
        let wanted = Manifest {
            device: Some(Snapshot {
                text: DEVICE.into(),
                mode: 0o644,
            }),
            fragment: Some(Snapshot {
                text: fragment.into(),
                mode: 0o644,
            }),
        };
        apply_and_restart(fs_adapter, &wanted, &previous, system)?;
        Ok(Some(serde_json::json!({"backup_id": id}).to_string()))
    } else {
        let id = args[3];
        let wanted = load_manifest(fs_adapter, id, &keys)?;
        let previous = current(fs_adapter, &keys)?;
        apply_and_restart(fs_adapter, &wanted, &previous, system)?;
        Ok(Some(serde_json::json!({"restored": id}).to_string()))
    }
}
