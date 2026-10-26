//! Fixed-target installer transaction executed by the authenticated root helper.
use crate::install::{HELPER_PATH, KEYD_POLICY_PATH, MACCEL_POLICY_PATH, SERVICE_PATH};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Commit,
    Abort,
}
impl Decision {
    pub fn read(reader: &mut impl Read) -> Result<Self, String> {
        let mut bytes = [0; 8];
        let mut used = 0;
        while used < bytes.len() {
            let count = reader
                .read(&mut bytes[used..used + 1])
                .map_err(|e| e.to_string())?;
            if count == 0 {
                return Err("setup control pipe closed before decision".into());
            }
            used += count;
            if bytes[used - 1] == b'\n' {
                return match &bytes[..used] {
                    b"COMMIT\n" => Ok(Self::Commit),
                    b"ABORT\n" => Ok(Self::Abort),
                    _ => Err("invalid setup decision".into()),
                };
            }
        }
        Err("setup decision too long".into())
    }
}

#[derive(Clone)]
struct Target {
    path: PathBuf,
    bytes: Vec<u8>,
    mode: u32,
    policy: bool,
}
fn policy_targets(login: &str) -> [Target; 2] {
    [
        Target {
            path: KEYD_POLICY_PATH.into(),
            bytes: include_str!("../packaging/mcsr-toolkit-rust-keyd.sudoers")
                .replace("@DESKTOP_USER@", login)
                .into_bytes(),
            mode: 0o440,
            policy: true,
        },
        Target {
            path: MACCEL_POLICY_PATH.into(),
            bytes: include_str!("../packaging/mcsr-toolkit-rust-maccel.sudoers")
                .replace("@DESKTOP_USER@", login)
                .into_bytes(),
            mode: 0o440,
            policy: true,
        },
    ]
}
fn policy_state(targets: &[Target], boundary: &Path, uid: u32) -> &'static str {
    let mut all = true;
    for target in targets {
        if check_components(&target.path, boundary, uid).is_err() {
            return "mismatch";
        }
        match matches(target, uid) {
            Ok(true) => {}
            Ok(false) => all = false,
            Err(_) => return "mismatch",
        }
    }
    if all {
        "match"
    } else {
        "missing"
    }
}

fn parse_invoking_uid(raw: &str) -> Result<u32, String> {
    let uid = raw.parse::<u32>().map_err(|_| "invalid PKEXEC_UID")?;
    if uid == 0 || raw != uid.to_string() {
        return Err("invalid invoking UID".into());
    }
    Ok(uid)
}
fn valid_login(name: &str) -> bool {
    !name.is_empty()
        && name != "ALL"
        && !name.starts_with('-')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn login_for_invoker() -> Result<String, String> {
    let raw = std::env::var("PKEXEC_UID").map_err(|_| "PKEXEC_UID is required")?;
    let uid = parse_invoking_uid(&raw)?;
    desktop_login_for_uid(uid)
}
pub(crate) fn desktop_login_for_uid(uid: u32) -> Result<String, String> {
    if uid == 0 {
        return Err("root is not a desktop login".into());
    }
    let mut buffer = vec![0u8; 16_384];
    let mut pwd = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut found = std::ptr::null_mut();
    let result = unsafe {
        libc::getpwuid_r(
            uid,
            pwd.as_mut_ptr(),
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &mut found,
        )
    };
    if result != 0 || found.is_null() {
        return Err("invoking UID has no account".into());
    }
    let pwd = unsafe { pwd.assume_init() };
    let name = unsafe { std::ffi::CStr::from_ptr(pwd.pw_name) }
        .to_str()
        .map_err(|_| "invalid invoking login")?
        .to_owned();
    if !valid_login(&name) {
        return Err("invalid invoking login".into());
    }
    Ok(name)
}

fn check_components(path: &Path, boundary: &Path, uid: u32) -> Result<(), String> {
    if !path.starts_with(boundary) {
        return Err("setup target outside fixed root".into());
    }
    for part in path.ancestors() {
        let metadata = match fs::symlink_metadata(part) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.to_string()),
        };
        if metadata.file_type().is_symlink()
            || metadata.uid() != uid
            || metadata.permissions().mode() & 0o022 != 0
        {
            return Err(format!("unsafe root path component: {}", part.display()));
        }
        if part == boundary {
            break;
        }
    }
    Ok(())
}

fn matches(target: &Target, uid: u32) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(&target.path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };
    if !metadata.file_type().is_file()
        || metadata.uid() != uid
        || metadata.permissions().mode() & 0o7777 != target.mode
        || fs::read(&target.path).map_err(|e| e.to_string())? != target.bytes
    {
        return Err(format!(
            "existing setup target differs: {}",
            target.path.display()
        ));
    }
    Ok(true)
}

fn rollback(created: &[Target], uid: u32) -> Result<(), String> {
    let mut errors = Vec::new();
    for target in created.iter().rev() {
        match matches(target, uid) {
            Ok(true) => {
                if let Err(e) = fs::remove_file(&target.path) {
                    errors.push(format!("{}: {e}", target.path.display()));
                } else if let Some(parent) = target.path.parent() {
                    if let Err(e) = File::open(parent).and_then(|directory| directory.sync_all()) {
                        errors.push(format!("{}: {e}", parent.display()));
                    }
                }
            }
            Ok(false) => {}
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn transaction(
    targets: &[Target],
    boundary: &Path,
    uid: u32,
    reader: &mut impl Read,
    writer: &mut impl Write,
    validate_policy: impl Fn(&Path) -> Result<(), String>,
) -> Result<(), String> {
    // Validate all existing destinations before publishing anything.
    let mut missing = Vec::new();
    for target in targets {
        check_components(&target.path, boundary, uid)?;
        if !matches(target, uid)? {
            missing.push(target);
        }
    }
    let mut staged = Vec::new();
    for target in &missing {
        let parent = target.path.parent().ok_or("missing setup parent")?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        temp.write_all(&target.bytes).map_err(|e| e.to_string())?;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(target.mode))
            .map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        let meta = temp.as_file().metadata().map_err(|e| e.to_string())?;
        if meta.uid() != uid
            || meta.permissions().mode() & 0o7777 != target.mode
            || Sha256::digest(fs::read(temp.path()).map_err(|e| e.to_string())?)
                != Sha256::digest(&target.bytes)
        {
            return Err("root stage verification failed".into());
        }
        if target.policy {
            validate_policy(temp.path())?;
        }
        staged.push(((*target).clone(), temp));
    }
    let mut created = Vec::new();
    let result = (|| {
        for (target, temp) in staged {
            check_components(&target.path, boundary, uid)?;
            temp.persist_noclobber(&target.path)
                .map_err(|e| format!("root publish {}: {}", target.path.display(), e.error))?;
            created.push(target.clone());
            if !matches(&target, uid)? {
                return Err("published root file verification failed".into());
            }
            File::open(target.path.parent().ok_or("missing setup parent")?)
                .and_then(|directory| directory.sync_all())
                .map_err(|e| e.to_string())?;
        }
        writer.write_all(b"READY\n").map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        if Decision::read(reader)? == Decision::Abort {
            return Err("setup aborted".into());
        }
        writer
            .write_all(b"COMMITTED\n")
            .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        return match rollback(&created, uid) {
            Ok(()) => {
                let _ = writer.write_all(b"ABORTED\n");
                let _ = writer.flush();
                Err(error)
            }
            Err(rollback) => Err(format!("{error}; root rollback failed: {rollback}")),
        };
    }
    Ok(())
}

fn uninstall_stage(target: &Target) -> Result<PathBuf, String> {
    let name = target
        .path
        .file_name()
        .ok_or("invalid fixed uninstall target")?
        .to_str()
        .ok_or("non-UTF8 uninstall target")?;
    Ok(target
        .path
        .with_file_name(format!(".{name}-mcsr-uninstall-stage")))
}
fn sync_target_parent(path: &Path) -> Result<(), String> {
    File::open(path.parent().ok_or("missing target parent")?)
        .and_then(|directory| directory.sync_all())
        .map_err(|e| e.to_string())
}
fn restore_uninstall_stages(targets: &[Target], boundary: &Path, uid: u32) -> Result<(), String> {
    for target in targets.iter().rev() {
        let stage = uninstall_stage(target)?;
        check_components(&target.path, boundary, uid)?;
        check_components(&stage, boundary, uid)?;
        let staged = Target {
            path: stage.clone(),
            ..target.clone()
        };
        if matches(&staged, uid)? {
            if matches(target, uid)? {
                return Err("uninstall stage and target both exist".into());
            }
            fs::rename(&stage, &target.path).map_err(|e| e.to_string())?;
            sync_target_parent(&target.path)?;
        }
    }
    Ok(())
}
fn remove_transaction(
    targets: &[Target],
    boundary: &Path,
    uid: u32,
    reader: &mut impl Read,
    writer: &mut impl Write,
    mut recheck_recovery: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let (helper, others) = targets.split_last().ok_or("missing uninstall helper")?;
    if !matches(helper, uid)? {
        return Err("installed helper missing during uninstall".into());
    }
    recheck_recovery()?;
    restore_uninstall_stages(others, boundary, uid)?;
    let mut staged = Vec::new();
    let mut deleting = false;
    let result = (|| {
        for target in others {
            check_components(&target.path, boundary, uid)?;
            let stage = uninstall_stage(target)?;
            check_components(&stage, boundary, uid)?;
            if matches(target, uid)? {
                if fs::symlink_metadata(&stage).is_ok() {
                    return Err("unexpected uninstall stage exists".into());
                }
                fs::rename(&target.path, &stage).map_err(|e| e.to_string())?;
                staged.push(target.clone());
                sync_target_parent(&target.path)?;
            }
        }
        writer.write_all(b"READY\n").map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        if Decision::read(reader)? == Decision::Abort {
            return Err("uninstall aborted".into());
        }
        recheck_recovery()?;
        // The executable helper remains in place until every other target is gone.
        deleting = true;
        for target in &staged {
            let stage = uninstall_stage(target)?;
            let staged_target = Target {
                path: stage.clone(),
                ..target.clone()
            };
            if !matches(&staged_target, uid)? {
                return Err("uninstall stage changed before commit".into());
            }
            fs::remove_file(&stage).map_err(|e| e.to_string())?;
            sync_target_parent(&stage)?;
        }
        if matches(helper, uid)? {
            fs::remove_file(&helper.path).map_err(|e| e.to_string())?;
            sync_target_parent(&helper.path)?;
        }
        writer
            .write_all(b"COMMITTED\n")
            .map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        // A missing stage means commit may have started. Keep the helper and marker for repair.
        if !deleting
            && staged
                .iter()
                .all(|target| uninstall_stage(target).is_ok_and(|path| path.exists()))
        {
            return match restore_uninstall_stages(&staged, boundary, uid) {
                Ok(()) => {
                    let _ = writer.write_all(b"ABORTED\n");
                    let _ = writer.flush();
                    Err(error)
                }
                Err(restore) => Err(format!("{error}; uninstall rollback failed: {restore}")),
            };
        }
        return Err(format!(
            "{error}; uninstall commit incomplete; repair required"
        ));
    }
    Ok(())
}

fn wait_control_pipe(deadline: std::time::Instant) -> Result<(), String> {
    let mut descriptor = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN | libc::POLLHUP,
        revents: 0,
    };
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() {
        return Err("setup decision timed out".into());
    }
    let result = unsafe {
        libc::poll(
            &mut descriptor,
            1,
            remaining.as_millis().min(i32::MAX as u128) as i32,
        )
    };
    if result == 0 {
        Err("setup decision timed out".into())
    } else if result < 0 {
        Err(io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

pub fn run_stdio() -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("setup requires root".into());
    }
    let login = login_for_invoker()?;
    let helper = Path::new(HELPER_PATH);
    check_components(helper, Path::new("/"), 0)?;
    let meta = fs::symlink_metadata(helper).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() || meta.uid() != 0 || meta.permissions().mode() & 0o7777 != 0o755
    {
        return Err("unsafe installed setup helper".into());
    }
    let mut bytes = Vec::new();
    File::open(helper)
        .map_err(|e| e.to_string())?
        .take(100_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 100_000_000 {
        return Err("invalid installed helper size".into());
    }
    let policies = policy_targets(&login);
    let targets = [
        Target {
            path: SERVICE_PATH.into(),
            bytes,
            mode: 0o755,
            policy: false,
        },
        policies[0].clone(),
        policies[1].clone(),
    ];
    restore_uninstall_stages(&targets, Path::new("/"), 0)?;
    let mut input = io::stdin();
    let mut output = io::stdout();
    transaction(
        &targets,
        Path::new("/"),
        0,
        &mut TimedInput {
            input: &mut input,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(120),
        },
        &mut output,
        |path| {
            let result = std::process::Command::new("/usr/sbin/visudo")
                .args(["-cf", path.to_str().ok_or("invalid policy stage path")?])
                .output()
                .map_err(|e| e.to_string())?;
            if result.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "sudoers validation failed: {}",
                    String::from_utf8_lossy(&result.stderr).trim()
                ))
            }
        },
    )
}

pub fn uninstall_stdio() -> Result<(), String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("uninstall requires root".into());
    }
    let login = login_for_invoker()?;
    let helper = Path::new(HELPER_PATH);
    check_components(helper, Path::new("/"), 0)?;
    let meta = fs::symlink_metadata(helper).map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() || meta.uid() != 0 || meta.permissions().mode() & 0o7777 != 0o755
    {
        return Err("unsafe installed uninstall helper".into());
    }
    let bytes = fs::read(helper).map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 100_000_000 {
        return Err("invalid installed helper size".into());
    }
    let policies = policy_targets(&login);
    let targets = [
        policies[0].clone(),
        policies[1].clone(),
        Target {
            path: SERVICE_PATH.into(),
            bytes: bytes.clone(),
            mode: 0o755,
            policy: false,
        },
        Target {
            path: HELPER_PATH.into(),
            bytes,
            mode: 0o755,
            policy: false,
        },
    ];
    let keyd_paths = crate::keyd_helper::HelperFilesystem::production();
    let maccel_paths = crate::maccel_helper::HelperPaths::production();
    // Fixed order: keyd first, then maccel. Keep both locks until all root targets commit or restore.
    let _keyd_lock = keyd_paths.lock_operation()?;
    let _maccel_lock = crate::maccel_helper::hold_operation_lock(&maccel_paths)?;
    let mut input = io::stdin();
    let mut output = io::stdout();
    remove_transaction(
        &targets,
        Path::new("/"),
        0,
        &mut TimedInput {
            input: &mut input,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(120),
        },
        &mut output,
        || {
            let keyd = crate::keyd_helper::status_under_lock(&keyd_paths)?;
            let maccel = crate::maccel_helper::recovery_pending(&maccel_paths)?;
            if keyd == "clear" && !maccel {
                Ok(())
            } else {
                Err("input recovery pending; retain helper and policy".into())
            }
        },
    )
}

pub fn recovery_status() -> Result<String, String> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("status requires root".into());
    }
    check_components(Path::new(HELPER_PATH), Path::new("/"), 0)?;
    let keyd = match fs::symlink_metadata("/var/lib/mcsr-toolkit/keyd") {
        Err(e) if e.kind() == io::ErrorKind::NotFound => "clear".to_owned(),
        _ => crate::keyd_helper::dispatch(
            &["privileged", "keyd", "status"],
            &[],
            &crate::keyd_helper::HelperFilesystem::production(),
            &mut crate::keyd_helper::RealSystem::default(),
        )
        .ok()
        .flatten()
        .unwrap_or_else(|| "active".into()),
    };
    let maccel = match fs::symlink_metadata("/run/mcsr-toolkit-maccel") {
        Err(e) if e.kind() == io::ErrorKind::NotFound => "clear".to_owned(),
        _ => crate::maccel_helper::dispatch(
            &["privileged", "maccel", "status"],
            &[],
            &crate::maccel_helper::HelperPaths::production(),
            &mut crate::maccel_helper::RealParameter,
        )
        .unwrap_or_else(|_| "baseline_pending".into()),
    };
    let login = login_for_invoker()?;
    let policy = policy_state(&policy_targets(&login), Path::new("/"), 0);
    Ok(format!("keyd={keyd} maccel={maccel} policies={policy}"))
}

struct TimedInput<'a> {
    input: &'a mut io::Stdin,
    deadline: std::time::Instant,
}
impl Read for TimedInput<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        wait_control_pipe(self.deadline).map_err(io::Error::other)?;
        self.input.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn fixture() -> (tempfile::TempDir, Vec<Target>, u32) {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::create_dir(root.path().join("bin")).unwrap();
        fs::create_dir(root.path().join("sudoers")).unwrap();
        fs::set_permissions(root.path().join("bin"), fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(
            root.path().join("sudoers"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        let uid = unsafe { libc::geteuid() };
        let targets = vec![
            Target {
                path: root.path().join("bin/tool"),
                bytes: b"verified executable".to_vec(),
                mode: 0o755,
                policy: false,
            },
            Target {
                path: root.path().join("sudoers/keyd"),
                bytes: b"alice exact rule\n".to_vec(),
                mode: 0o440,
                policy: true,
            },
        ];
        (root, targets, uid)
    }

    #[test]
    fn exact_unreadable_to_desktop_user_policy_is_verified_by_root_status() {
        let (root, targets, uid) = fixture();
        let policy = &targets[1];
        assert_eq!(policy_state(&targets[1..], root.path(), uid), "missing");
        fs::write(&policy.path, &policy.bytes).unwrap();
        fs::set_permissions(&policy.path, fs::Permissions::from_mode(0o440)).unwrap();
        assert_eq!(policy_state(&targets[1..], root.path(), uid), "match");
        fs::set_permissions(&policy.path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(policy_state(&targets[1..], root.path(), uid), "mismatch");
    }

    #[test]
    fn invoking_identity_rejects_root_and_sudoers_syntax() {
        for raw in ["0", "00", "-1", "1000x", "+1000", "01000"] {
            assert!(parse_invoking_uid(raw).is_err(), "{raw}");
        }
        assert_eq!(parse_invoking_uid("1000"), Ok(1000));
        for login in ["", "ALL", "-alice", "alice ALL=(root)", "a:b", "a\nrule"] {
            assert!(!valid_login(login), "{login}");
        }
        assert!(valid_login("alice_1"));
    }

    #[test]
    fn ready_then_commit_publishes_fixed_files_once() {
        let (root, targets, uid) = fixture();
        let mut output = Vec::new();
        let validated = std::cell::Cell::new(0);
        transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"COMMIT\n"),
            &mut output,
            |path| {
                assert_eq!(fs::read(path).unwrap(), b"alice exact rule\n");
                validated.set(validated.get() + 1);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(output, b"READY\nCOMMITTED\n");
        assert_eq!(validated.get(), 1);
        for target in &targets {
            assert!(matches(target, uid).unwrap());
        }
    }

    #[test]
    fn abort_and_eof_roll_back_only_new_files() {
        for decision in [b"ABORT\n".as_slice(), b"".as_slice()] {
            let (root, targets, uid) = fixture();
            let existing = &targets[0];
            fs::write(&existing.path, &existing.bytes).unwrap();
            fs::set_permissions(&existing.path, fs::Permissions::from_mode(existing.mode)).unwrap();
            let mut output = Vec::new();
            assert!(transaction(
                &targets,
                root.path(),
                uid,
                &mut Cursor::new(decision),
                &mut output,
                |_| Ok(())
            )
            .is_err());
            assert_eq!(output, b"READY\nABORTED\n");
            assert!(existing.path.exists());
            assert!(!targets[1].path.exists());
        }
    }

    #[test]
    fn timed_out_control_pipe_rolls_back_created_files() {
        struct Timeout;
        impl Read for Timeout {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::TimedOut.into())
            }
        }
        let (root, targets, uid) = fixture();
        let mut output = Vec::new();
        let error = transaction(
            &targets,
            root.path(),
            uid,
            &mut Timeout,
            &mut output,
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        assert_eq!(output, b"READY\nABORTED\n");
        for target in &targets {
            assert!(!target.path.exists());
        }
    }

    #[test]
    fn uninstall_abort_restores_exact_root_targets_and_commit_removes_them() {
        for (decision, removed) in [
            (b"ABORT\n".as_slice(), false),
            (b"COMMIT\n".as_slice(), true),
        ] {
            let (root, mut targets, uid) = fixture();
            targets.push(Target {
                path: root.path().join("bin/helper"),
                bytes: b"verified helper".to_vec(),
                mode: 0o755,
                policy: false,
            });
            for target in &targets {
                fs::write(&target.path, &target.bytes).unwrap();
                fs::set_permissions(&target.path, fs::Permissions::from_mode(target.mode)).unwrap();
            }
            let mut output = Vec::new();
            let result = remove_transaction(
                &targets,
                root.path(),
                uid,
                &mut Cursor::new(decision),
                &mut output,
                || Ok(()),
            );
            assert_eq!(result.is_ok(), removed);
            assert!(output.starts_with(b"READY\n"));
            for target in &targets {
                assert_eq!(target.path.exists(), !removed);
            }
        }
    }

    #[test]
    fn uninstall_eof_and_stale_stage_repair_preserve_helper() {
        let (root, mut targets, uid) = fixture();
        targets.push(Target {
            path: root.path().join("bin/helper"),
            bytes: b"verified helper".to_vec(),
            mode: 0o755,
            policy: false,
        });
        for target in &targets {
            fs::write(&target.path, &target.bytes).unwrap();
            fs::set_permissions(&target.path, fs::Permissions::from_mode(target.mode)).unwrap();
        }
        let mut output = Vec::new();
        assert!(remove_transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b""),
            &mut output,
            || Ok(())
        )
        .is_err());
        assert_eq!(output, b"READY\nABORTED\n");
        for target in &targets {
            assert!(target.path.exists());
        }
        let stage = uninstall_stage(&targets[0]).unwrap();
        fs::rename(&targets[0].path, &stage).unwrap();
        let mut output = Vec::new();
        assert!(remove_transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"ABORT\n"),
            &mut output,
            || Ok(())
        )
        .is_err());
        assert!(targets[0].path.exists());
        assert!(!stage.exists());
        assert!(targets.last().unwrap().path.exists());
    }

    #[test]
    fn uninstall_rechecks_recovery_before_staging() {
        let (root, mut targets, uid) = fixture();
        targets.push(Target {
            path: root.path().join("bin/helper"),
            bytes: b"helper".to_vec(),
            mode: 0o755,
            policy: false,
        });
        for target in &targets {
            fs::write(&target.path, &target.bytes).unwrap();
            fs::set_permissions(&target.path, fs::Permissions::from_mode(target.mode)).unwrap();
        }
        assert!(remove_transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"COMMIT\n"),
            &mut Vec::new(),
            || Err("new keyd recovery".into())
        )
        .is_err());
        for target in &targets {
            assert!(target.path.exists());
        }
    }

    #[test]
    fn uninstall_rechecks_recovery_again_before_commit() {
        let (root, mut targets, uid) = fixture();
        targets.push(Target {
            path: root.path().join("bin/helper"),
            bytes: b"helper".to_vec(),
            mode: 0o755,
            policy: false,
        });
        for target in &targets {
            fs::write(&target.path, &target.bytes).unwrap();
            fs::set_permissions(&target.path, fs::Permissions::from_mode(target.mode)).unwrap();
        }
        let calls = std::cell::Cell::new(0);
        let mut output = Vec::new();
        assert!(remove_transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"COMMIT\n"),
            &mut output,
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 2 {
                    Err("new recovery before commit".into())
                } else {
                    Ok(())
                }
            },
        )
        .is_err());
        assert_eq!(calls.get(), 2);
        assert_eq!(output, b"READY\nABORTED\n");
        for target in &targets {
            assert!(target.path.exists());
        }
    }

    #[test]
    fn uninstall_decision_timeout_reports_abort_and_restores_targets() {
        struct Timeout;
        impl Read for Timeout {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::TimedOut.into())
            }
        }
        let (root, mut targets, uid) = fixture();
        targets.push(Target {
            path: root.path().join("bin/helper"),
            bytes: b"helper".to_vec(),
            mode: 0o755,
            policy: false,
        });
        for target in &targets {
            fs::write(&target.path, &target.bytes).unwrap();
            fs::set_permissions(&target.path, fs::Permissions::from_mode(target.mode)).unwrap();
        }
        let mut output = Vec::new();
        assert!(remove_transaction(
            &targets,
            root.path(),
            uid,
            &mut Timeout,
            &mut output,
            || Ok(())
        )
        .is_err());
        assert_eq!(output, b"READY\nABORTED\n");
        for target in &targets {
            assert!(target.path.exists());
        }
    }

    #[test]
    fn mismatched_policy_and_symlink_or_writable_ancestor_refuse_before_publish() {
        let (root, targets, uid) = fixture();
        fs::write(&targets[1].path, b"broader policy").unwrap();
        fs::set_permissions(&targets[1].path, fs::Permissions::from_mode(0o440)).unwrap();
        let mut output = Vec::new();
        assert!(transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"COMMIT\n"),
            &mut output,
            |_| Ok(())
        )
        .is_err());
        assert!(!targets[0].path.exists());
        fs::remove_file(&targets[1].path).unwrap();
        std::os::unix::fs::symlink(root.path().join("elsewhere"), &targets[1].path).unwrap();
        assert!(transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"COMMIT\n"),
            &mut output,
            |_| Ok(())
        )
        .is_err());
        fs::remove_file(&targets[1].path).unwrap();
        fs::set_permissions(
            root.path().join("sudoers"),
            fs::Permissions::from_mode(0o777),
        )
        .unwrap();
        assert!(transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"COMMIT\n"),
            &mut output,
            |_| Ok(())
        )
        .is_err());
        assert!(!targets[0].path.exists());
    }

    #[test]
    fn failed_policy_validation_never_publishes_any_target() {
        let (root, targets, uid) = fixture();
        assert!(transaction(
            &targets,
            root.path(),
            uid,
            &mut Cursor::new(b"COMMIT\n"),
            &mut Vec::new(),
            |_| Err("visudo rejected".into())
        )
        .is_err());
        for target in &targets {
            assert!(!target.path.exists());
        }
    }
}
