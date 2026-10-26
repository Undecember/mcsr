//! Fixed-target root helper for the observed maccel signed Q32 driver.
//! The public path and parameter adapters are internal test seams; the binary
//! constructs only `HelperPaths::production()` and `RealParameter`.

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

const FINGERPRINT: &str = "4AFA0F5B3EBD2F98EE54E56\n";
const EYE_RAW: i64 = 89_464_169;
const EYE_DECIMAL: &str = "0.02083000005222856998443603515625";
const LOCK_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug)]
pub struct HelperPaths {
    pub parameter: PathBuf,
    pub runtime: PathBuf,
    pub fingerprint: PathBuf,
    anchor: PathBuf,
    owner_uid: u32,
}

impl HelperPaths {
    pub fn production() -> Self {
        Self {
            parameter: "/sys/module/maccel/parameters/SENS_MULT".into(),
            runtime: "/run/mcsr-toolkit-maccel".into(),
            fingerprint: "/sys/module/maccel/srcversion".into(),
            anchor: "/".into(),
            owner_uid: 0,
        }
    }

    /// Builds a fixture rooted under an isolated directory for tests.
    pub fn sandbox(
        parameter: PathBuf,
        runtime: PathBuf,
        fingerprint: PathBuf,
        anchor: PathBuf,
        owner_uid: u32,
    ) -> Self {
        Self {
            parameter,
            runtime,
            fingerprint,
            anchor,
            owner_uid,
        }
    }

    fn check_dir(&self, file: &File, private: bool) -> Result<(), String> {
        let m = file.metadata().map_err(|e| e.to_string())?;
        let mode = m.permissions().mode() & 0o777;
        if !m.is_dir()
            || m.uid() != self.owner_uid
            || (if private {
                mode != 0o700
            } else {
                mode & 0o022 != 0
            })
        {
            return Err(format!(
                "unsafe directory ownership or mode: uid={}, mode={mode:o}, private={private}",
                m.uid()
            ));
        }
        Ok(())
    }

    fn directory(&self, path: &Path) -> Result<File, String> {
        let relative = path
            .strip_prefix(&self.anchor)
            .map_err(|_| "unexpected path")?;
        let mut file = open_path(&self.anchor, libc::O_RDONLY | libc::O_DIRECTORY)?;
        self.check_dir(&file, false)?;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err("unexpected path component".into());
            };
            file = open_at(
                &file,
                name.as_bytes(),
                libc::O_RDONLY | libc::O_DIRECTORY,
                0,
            )?;
            self.check_dir(&file, false)?;
        }
        Ok(file)
    }

    fn open_file(&self, parent: &File, name: &str, kind: Kind, flags: i32) -> Result<File, String> {
        let file = open_at(parent, name.as_bytes(), flags | libc::O_NONBLOCK, 0o600)?;
        self.check_file(file, kind)
    }

    fn check_file(&self, file: File, kind: Kind) -> Result<File, String> {
        let m = file.metadata().map_err(|e| e.to_string())?;
        let mode = m.permissions().mode() & 0o777;
        let mode_ok = match kind {
            Kind::Parameter => mode == 0o644 || mode == 0o664,
            Kind::Fingerprint => mode == 0o444 || mode == 0o644,
            Kind::Record => mode == 0o600,
        };
        if !m.is_file() || m.uid() != self.owner_uid || m.nlink() != 1 || !mode_ok {
            return Err("unsafe file ownership, mode, or type".into());
        }
        Ok(file)
    }

    fn runtime(&self, create: bool) -> Result<Option<File>, String> {
        let parent_path = self.runtime.parent().ok_or("unexpected runtime path")?;
        let name = file_name(&self.runtime)?;
        let parent = self.directory(parent_path)?;
        if create {
            let c = c_name(name.as_bytes())?;
            let result = unsafe { libc::mkdirat(parent.as_raw_fd(), c.as_ptr(), 0o700) };
            if result < 0 {
                let e = std::io::Error::last_os_error();
                if e.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(e.to_string());
                }
            } else {
                parent.sync_all().map_err(|e| e.to_string())?;
            }
        }
        match open_at_io(
            &parent,
            name.as_bytes(),
            libc::O_RDONLY | libc::O_DIRECTORY,
            0,
        ) {
            Ok(file) => {
                self.check_dir(&file, true)?;
                Ok(Some(file))
            }
            Err(error) if !create && error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn parameter(&self) -> Result<File, String> {
        let driver_parent = self.directory(
            self.fingerprint
                .parent()
                .ok_or("unexpected fingerprint path")?,
        )?;
        let mut fingerprint = self.open_file(
            &driver_parent,
            file_name(&self.fingerprint)?,
            Kind::Fingerprint,
            libc::O_RDONLY,
        )?;
        if read_text(&mut fingerprint)? != FINGERPRINT {
            return Err("unsupported maccel driver fingerprint".into());
        }
        let parameter_parent =
            self.directory(self.parameter.parent().ok_or("unexpected parameter path")?)?;
        self.open_file(
            &parameter_parent,
            file_name(&self.parameter)?,
            Kind::Parameter,
            libc::O_RDWR,
        )
    }
}

/// Verifies the installed binary and every path component using directory
/// descriptors. The production caller supplies a fixed path and root UID.
pub fn verify_executable(path: &Path, anchor: &Path, owner_uid: u32) -> Result<(), String> {
    let paths = HelperPaths::sandbox(
        path.into(),
        path.into(),
        path.into(),
        anchor.into(),
        owner_uid,
    );
    let parent = paths.directory(path.parent().ok_or("unexpected executable path")?)?;
    let file = open_at(&parent, file_name(path)?.as_bytes(), libc::O_RDONLY, 0)?;
    let m = file.metadata().map_err(|e| e.to_string())?;
    let mode = m.permissions().mode();
    if !m.is_file()
        || m.uid() != owner_uid
        || m.nlink() != 1
        || mode & 0o022 != 0
        || mode & 0o7000 != 0
        || mode & 0o111 == 0
    {
        return Err("unsafe installed maccel executable".into());
    }
    Ok(())
}

/// Samples restore stdin without waiting for a terminal to close. Restore has
/// no payload; one available byte is enough for dispatch to reject it.
pub fn read_restore_input(fd: RawFd) -> Result<Vec<u8>, String> {
    let mut poll = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let ready = unsafe { libc::poll(&mut poll, 1, 0) };
    if ready < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if ready == 0 {
        return Ok(Vec::new());
    }
    if poll.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
        return Err("invalid restore stdin".into());
    }
    let mut byte = [0u8; 1];
    let count = unsafe { libc::read(fd, byte.as_mut_ptr().cast(), 1) };
    if count < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(byte[..count as usize].to_vec())
}

fn file_name(path: &Path) -> Result<&str, String> {
    path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "unexpected path".into())
}

fn c_name(name: &[u8]) -> Result<CString, String> {
    CString::new(name).map_err(|_| "unexpected path component".into())
}

fn open_path(path: &Path, flags: i32) -> Result<File, String> {
    let c = c_name(path.as_os_str().as_bytes())?;
    let fd = unsafe {
        libc::open(
            c.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn open_at(parent: &File, name: &[u8], flags: i32, mode: u32) -> Result<File, String> {
    open_at_io(parent, name, flags, mode).map_err(|e| e.to_string())
}

fn open_at_io(parent: &File, name: &[u8], flags: i32, mode: u32) -> std::io::Result<File> {
    let c = CString::new(name).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "unexpected path component",
        )
    })?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            c.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            mode,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn read_text(file: &mut File) -> Result<String, String> {
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    file.take(257)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > 256 {
        return Err("oversized record".into());
    }
    if !data.is_ascii() {
        return Err("non-ASCII record".into());
    }
    String::from_utf8(data).map_err(|e| e.to_string())
}

fn raw(text: &str, parameter: bool) -> Result<i64, String> {
    let text = if parameter && text.ends_with("\n\n") {
        &text[..text.len() - 1]
    } else {
        text
    };
    let digits = text.strip_suffix('\n').unwrap_or(text);
    if digits.is_empty()
        || digits.starts_with('0')
        || digits.len() > 19
        || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return Err("unsupported signed Q32 value".into());
    }
    digits
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "unsupported signed Q32 value".into())
}

fn token(input: &[u8]) -> Result<&str, String> {
    let input = input.strip_suffix(b"\n").unwrap_or(input);
    if input.len() != 64
        || !input
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err("missing or malformed generation".into());
    }
    std::str::from_utf8(input).map_err(|e| e.to_string())
}

enum Kind {
    Parameter,
    Fingerprint,
    Record,
}

fn record(paths: &HelperPaths, runtime: &File, name: &str) -> Result<Option<String>, String> {
    let file = match open_at_io(
        runtime,
        name.as_bytes(),
        libc::O_RDONLY | libc::O_NONBLOCK,
        0,
    ) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut file = paths.check_file(file, Kind::Record)?;
    read_text(&mut file).map(Some)
}

fn store(paths: &HelperPaths, runtime: &File, name: &str, text: &str) -> Result<(), String> {
    let mut file = paths.open_file(
        runtime,
        name,
        Kind::Record,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
    )?;
    file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    runtime.sync_all().map_err(|e| e.to_string())
}

fn unlink(runtime: &File, name: &str) -> Result<(), String> {
    let c = c_name(name.as_bytes())?;
    let result = unsafe { libc::unlinkat(runtime.as_raw_fd(), c.as_ptr(), 0) };
    if result < 0 {
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(e.to_string());
        }
    }
    runtime.sync_all().map_err(|e| e.to_string())
}

fn runtime_names(runtime: &File) -> Result<(), String> {
    // A fresh open file description keeps a second validation from inheriting
    // an exhausted directory offset after the restore preflight check.
    let fd = open_at(runtime, b".", libc::O_RDONLY | libc::O_DIRECTORY, 0)?.into_raw_fd();
    let dir = unsafe { libc::fdopendir(fd) };
    if dir.is_null() {
        unsafe { libc::close(fd) };
        return Err(std::io::Error::last_os_error().to_string());
    }
    let mut result = Ok(());
    unsafe {
        *libc::__errno_location() = 0;
    }
    loop {
        let entry = unsafe { libc::readdir(dir) };
        if entry.is_null() {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(0) {
                result = Err(error.to_string());
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if !matches!(name, b"." | b".." | b"lock" | b"generation" | b"baseline") {
            result = Err("unexpected runtime entry".into());
            break;
        }
        unsafe {
            *libc::__errno_location() = 0;
        }
    }
    unsafe { libc::closedir(dir) };
    result
}

fn validated_records(
    paths: &HelperPaths,
    runtime: &File,
) -> Result<(Option<String>, Option<i64>), String> {
    runtime_names(runtime)?;
    let generation = record(paths, runtime, "generation")?
        .map(|text| token(text.as_bytes()).map(str::to_owned))
        .transpose()?;
    let baseline = record(paths, runtime, "baseline")?
        .map(|text| raw(&text, false))
        .transpose()?;
    Ok((generation, baseline))
}

pub fn recovery_pending(paths: &HelperPaths) -> Result<bool, String> {
    let Some(runtime) = paths.runtime(false)? else {
        return Ok(false);
    };
    let _lock = lock(paths, &runtime, false)?;
    let (_, baseline) = validated_records(paths, &runtime)?;
    Ok(baseline.is_some())
}

pub(crate) struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

fn lock(paths: &HelperPaths, runtime: &File, create: bool) -> Result<Lock, String> {
    let flags = libc::O_RDWR | if create { libc::O_CREAT } else { 0 };
    let file = paths.open_file(runtime, "lock", Kind::Record, flags)?;
    let deadline = Instant::now() + LOCK_TIMEOUT;
    loop {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(Lock(file));
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EWOULDBLOCK) {
            return Err(error.to_string());
        }
        if Instant::now() >= deadline {
            return Err("helper lock timed out".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(crate) fn hold_operation_lock(paths: &HelperPaths) -> Result<Lock, String> {
    let parent = paths.directory(paths.runtime.parent().ok_or("unexpected runtime parent")?)?;
    let file = paths.open_file(
        &parent,
        ".mcsr-toolkit-maccel-operation.lock",
        Kind::Record,
        libc::O_RDWR | libc::O_CREAT,
    )?;
    parent.sync_all().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + LOCK_TIMEOUT;
    loop {
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(Lock(file));
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EWOULDBLOCK) {
            return Err(error.to_string());
        }
        if Instant::now() >= deadline {
            return Err("maccel operation lock timed out".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub trait ParameterIo {
    fn read_raw(&mut self, file: &mut File) -> Result<String, String>;
    fn write_raw(&mut self, file: &mut File, raw: i64) -> Result<(), String>;
}

#[derive(Default)]
pub struct RealParameter;
impl ParameterIo for RealParameter {
    fn read_raw(&mut self, file: &mut File) -> Result<String, String> {
        read_text(file)
    }
    fn write_raw(&mut self, file: &mut File, raw: i64) -> Result<(), String> {
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let data = raw.to_string();
        if file.write(data.as_bytes()).map_err(|e| e.to_string())? != data.len() {
            return Err("partial parameter write".into());
        }
        Ok(())
    }
}

fn write_checked(
    io: &mut dyn ParameterIo,
    parameter: &mut File,
    raw_value: i64,
) -> Result<(), String> {
    io.write_raw(parameter, raw_value)?;
    if raw(&io.read_raw(parameter)?, true)? != raw_value {
        return Err("parameter readback mismatch".into());
    }
    Ok(())
}

fn new_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn transition(
    command: &str,
    baseline: Option<i64>,
    paths: &HelperPaths,
    runtime: &File,
    parameter: &mut File,
    io: &mut dyn ParameterIo,
) -> Result<String, String> {
    let current = raw(&io.read_raw(parameter)?, true)?;
    if command == "eye" {
        if baseline.is_none() {
            store(paths, runtime, "baseline", &format!("{current}\n"))?;
        } else {
            let file = paths.open_file(runtime, "baseline", Kind::Record, libc::O_RDONLY)?;
            file.sync_all().map_err(|e| e.to_string())?;
            runtime.sync_all().map_err(|e| e.to_string())?;
        }
        write_checked(io, parameter, EYE_RAW)?;
        Ok(EYE_DECIMAL.into())
    } else {
        if let Some(baseline) = baseline {
            write_checked(io, parameter, baseline)?;
            unlink(runtime, "baseline")?;
        }
        let token = new_token();
        store(paths, runtime, "generation", &format!("{token}\n"))?;
        Ok(token)
    }
}

/// Executes only the fixed eye and restore commands. Eye input is captured
/// before lock acquisition so a queued invocation cannot adopt a new token.
pub fn dispatch(
    args: &[&str],
    stdin: &[u8],
    paths: &HelperPaths,
    io: &mut dyn ParameterIo,
) -> Result<String, String> {
    let (command, eye_token) = match args {
        ["privileged", "maccel", "eye"] => ("eye", Some(token(stdin)?)),
        ["privileged", "maccel", "restore"] if stdin.is_empty() => ("restore", None),
        ["privileged", "maccel", "status"] if stdin.is_empty() => ("status", None),
        _ => return Err("invalid privileged maccel command or arguments".into()),
    };
    let _operation_lock = hold_operation_lock(paths)?;
    if paths.anchor == Path::new("/") && paths.owner_uid == 0 {
        let helper = Path::new("/usr/local/libexec/mcsr-toolkit");
        verify_executable(helper, Path::new("/"), 0)?;
        let running = std::fs::metadata("/proc/self/exe").map_err(|e| e.to_string())?;
        let installed = std::fs::symlink_metadata(helper).map_err(|e| e.to_string())?;
        if running.dev() != installed.dev() || running.ino() != installed.ino() {
            return Err("maccel helper was replaced or removed while queued".into());
        }
    }
    if command == "status" {
        return Ok(if recovery_pending(paths)? {
            "baseline_pending"
        } else {
            "clear"
        }
        .into());
    }
    if let Some(runtime) = paths.runtime(false)? {
        let _lock = lock(paths, &runtime, true)?;
        let (generation, baseline) = validated_records(paths, &runtime)?;
        if command == "restore" {
            unlink(&runtime, "generation")?;
            let mut parameter = paths.parameter()?;
            transition(command, baseline, paths, &runtime, &mut parameter, io)
        } else {
            if generation.as_deref() != eye_token {
                return Err("stale generation".into());
            }
            let result = paths.parameter().and_then(|mut parameter| {
                transition(command, baseline, paths, &runtime, &mut parameter, io)
            });
            if result.is_err() {
                // A valid eye token is consumed by any failure after validation,
                // including driver preflight and the initial raw read.
                unlink(&runtime, "generation")?;
            }
            result
        }
    } else {
        let mut parameter = paths.parameter()?;
        raw(&io.read_raw(&mut parameter)?, true)?;
        if command == "eye" {
            return Err("stale generation".into());
        }
        let runtime = paths.runtime(true)?.ok_or("runtime directory missing")?;
        let _lock = lock(paths, &runtime, true)?;
        let (_, baseline) = validated_records(paths, &runtime)?;
        unlink(&runtime, "generation")?;
        transition(command, baseline, paths, &runtime, &mut parameter, io)
    }
}

#[cfg(test)]
mod operation_lock_tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn held_uninstall_operation_lock_blocks_concurrent_maccel_action() {
        let fixture = tempfile::tempdir().unwrap();
        std::fs::set_permissions(fixture.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let run = fixture.path().join("run");
        std::fs::create_dir(&run).unwrap();
        std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o755)).unwrap();
        let paths = HelperPaths::sandbox(
            fixture.path().join("parameter"),
            run.join("mcsr-toolkit-maccel"),
            fixture.path().join("fingerprint"),
            fixture.path().to_path_buf(),
            unsafe { libc::geteuid() },
        );
        let held = hold_operation_lock(&paths).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let second = paths.clone();
        let worker = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let _lock = hold_operation_lock(&second).unwrap();
            acquired_tx.send(()).unwrap();
        });
        started_rx.recv().unwrap();
        assert!(acquired_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err());
        drop(held);
        acquired_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.join().unwrap();
    }
}
