use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub fn state_dir(data_home: &Path) -> PathBuf {
    data_home.join("mcsr-toolkit/rust-v1")
}

fn directory_fd(path: &Path) -> io::Result<File> {
    let name = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in data path"))?;
    let fd = unsafe {
        libc::open(
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn private_child(parent: &File, name: &std::ffi::CStr, uid: u32) -> io::Result<File> {
    let created = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) };
    if created < 0 && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let child = unsafe { File::from_raw_fd(fd) };
    let metadata = child.metadata()?;
    if metadata.uid() != uid {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unsafe state directory owner",
        ));
    }
    // Tighten the opened inode, so a replaced pathname cannot redirect chmod.
    if unsafe { libc::fchmod(child.as_raw_fd(), 0o700) } < 0 {
        return Err(io::Error::last_os_error());
    }
    if child.metadata()?.permissions().mode() & 0o7777 != 0o700 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unsafe state directory mode",
        ));
    }
    parent.sync_all()?;
    Ok(child)
}

fn prepare_private_state_for_uid(data_home: &Path, uid: u32) -> io::Result<PathBuf> {
    fs::create_dir_all(data_home)?;
    let data = directory_fd(data_home)?;
    let parent = private_child(&data, c"mcsr-toolkit", uid)?;
    let state = private_child(&parent, c"rust-v1", uid)?;
    state.sync_all()?;
    Ok(state_dir(data_home))
}

pub fn prepare_private_state(data_home: &Path) -> io::Result<PathBuf> {
    prepare_private_state_for_uid(data_home, unsafe { libc::geteuid() })
}

pub fn private_dir(path: &Path) -> io::Result<()> {
    if !path.exists() {
        fs::create_dir(path)?;
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unsafe state directory",
        ));
    }
    Ok(())
}

pub fn ensure_state(data_home: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(data_home)?;
    let parent = data_home.join("mcsr-toolkit");
    if !parent.exists() {
        fs::create_dir(&parent)?;
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700))?;
    }
    private_dir(&parent)?;
    let state = state_dir(data_home);
    if !state.exists() {
        fs::create_dir(&state)?;
        fs::set_permissions(&state, fs::Permissions::from_mode(0o700))?;
    }
    private_dir(&state)?;
    sync_dir(&parent)?;
    Ok(state)
}

pub fn sync_dir(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

pub fn safe_record(path: &Path) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unsafe recovery record",
        ));
    }
    fs::read(path)
}

pub fn atomic_replace(path: &Path, bytes: &[u8], record: bool) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    let mode = if record {
        0o600
    } else {
        let m = fs::symlink_metadata(path)?;
        if !m.is_file() || m.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "unsafe options target",
            ));
        }
        m.permissions().mode() & 0o777
    };
    let temp = parent.join(format!(
        ".mcsr-{}-{}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(mode)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        sync_dir(parent)
    })();
    let _ = fs::remove_file(&temp);
    result
}

pub fn remove_record(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => sync_dir(path.parent().unwrap()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod cutover_tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn private_state_creation_ignores_permissive_umask_and_runtime_accepts_it() {
        if std::env::var_os("MCSR_PRIVATE_UMASK_CHILD").is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("paths::cutover_tests::private_state_creation_ignores_permissive_umask_and_runtime_accepts_it")
                .env("MCSR_PRIVATE_UMASK_CHILD", "1")
                .status().unwrap();
            assert!(status.success());
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        fs::create_dir(&data).unwrap();
        unsafe { libc::umask(0o002) };
        let state = prepare_private_state(&data).unwrap();
        for path in [data.join("mcsr-toolkit"), state.clone()] {
            assert_eq!(
                fs::symlink_metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        assert_eq!(ensure_state(&data).unwrap(), state);
    }

    #[test]
    fn existing_permissive_parent_is_tightened_without_touching_records_or_data_home() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        let parent = data.join("mcsr-toolkit");
        let state = parent.join("rust-v1");
        let python = parent.join("state");
        fs::create_dir_all(&state).unwrap();
        fs::create_dir(&python).unwrap();
        fs::set_permissions(&data, fs::Permissions::from_mode(0o775)).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o775)).unwrap();
        fs::set_permissions(&state, fs::Permissions::from_mode(0o775)).unwrap();
        fs::set_permissions(&python, fs::Permissions::from_mode(0o775)).unwrap();
        let marker = state.join("install-pending.json");
        fs::write(&marker, b"live marker bytes\n").unwrap();
        prepare_private_state(&data).unwrap();
        assert_eq!(fs::read(marker).unwrap(), b"live marker bytes\n");
        assert_eq!(
            fs::symlink_metadata(data).unwrap().permissions().mode() & 0o777,
            0o775
        );
        assert_eq!(
            fs::symlink_metadata(python).unwrap().permissions().mode() & 0o777,
            0o775
        );
        assert_eq!(
            fs::symlink_metadata(parent).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::symlink_metadata(state).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert!(ensure_state(&temp.path().join("data")).is_ok());
    }

    #[test]
    fn symlink_and_foreign_owned_private_paths_are_refused() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        fs::create_dir(&data).unwrap();
        let target = temp.path().join("target");
        fs::create_dir(&target).unwrap();
        symlink(&target, data.join("mcsr-toolkit")).unwrap();
        assert!(prepare_private_state(&data).is_err());
        fs::remove_file(data.join("mcsr-toolkit")).unwrap();
        fs::create_dir(data.join("mcsr-toolkit")).unwrap();
        assert!(prepare_private_state_for_uid(&data, unsafe { libc::geteuid() } + 1).is_err());
        assert!(target.is_dir());
    }
}
