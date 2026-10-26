use mcsr_toolkit::{
    gnome::{GnomeError, GnomePeer},
    maccel::{MaccelPeer, SensitivityError},
    runtime::{RuntimeConfig, ServiceRuntime},
};
use std::{
    fs,
    io::{BufRead, Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

struct FakeGnome;
impl GnomePeer for FakeGnome {
    async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
        Ok(("Minecraft".into(), "Game".into(), 1920, 1080, false))
    }
    async fn configure_presets(&self, _: String) -> Result<(), GnomeError> {
        Ok(())
    }
    async fn resize_focused(&self, width: u32, height: u32) -> Result<(u32, u32), GnomeError> {
        Ok((width, height))
    }
}
struct FakeMaccel(PathBuf);
impl MaccelPeer for FakeMaccel {
    async fn preflight(&self) -> Result<(), SensitivityError> {
        Ok(())
    }
    async fn eye(&self, _: &str) -> Result<String, SensitivityError> {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.0)
            .map_err(|e| SensitivityError(e.to_string()))?
            .write_all(b"eye\n")
            .map_err(|e| SensitivityError(e.to_string()))?;
        Ok("0.02083000005222856998443603515625".into())
    }
    async fn restore(&self) -> Result<String, SensitivityError> {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.0)
            .map_err(|e| SensitivityError(e.to_string()))?
            .write_all(b"restore\n")
            .map_err(|e| SensitivityError(e.to_string()))?;
        Ok("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into())
    }
}

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn legacy_service_lock_child() {
    let Some(path) = std::env::var_os("MCSR_RUNTIME_LEGACY_LOCK_CHILD") else {
        return;
    };
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .unwrap();
    assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) }, 0);
    println!("LOCKED");
    std::io::stdout().flush().unwrap();
    loop {
        std::thread::park();
    }
}

#[test]
fn fake_service_child() {
    let Ok(root) = std::env::var("MCSR_RUNTIME_FAKE_CHILD") else {
        return;
    };
    let root = PathBuf::from(root);
    let port: u16 = std::env::var("MCSR_RUNTIME_FAKE_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let config = RuntimeConfig {
        runtime_dir: root.join("run"),
        data_home: root.join("data"),
        managed_dir: root.join("managed"),
        port,
        enable_live_keyd: false,
        maccel_binary: root.join("unused"),
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut runtime = ServiceRuntime::start_with_peers(
            config,
            Arc::new(FakeGnome),
            Arc::new(FakeMaccel(root.join("maccel.log"))),
        )
        .await
        .unwrap();
        if std::env::var_os("MCSR_RUNTIME_FAKE_EYE").is_some() {
            let mut fifo = fs::OpenOptions::new()
                .write(true)
                .open(root.join("run/mcsr-toolkit-focus.fifo"))
                .unwrap();
            writeln!(
                fifo,
                "{{\"class\":\"Minecraft\",\"title\":\"Game\",\"modal\":false}}"
            )
            .unwrap();
            tokio::time::timeout(Duration::from_secs(2), async {
                while runtime.focus_status()["minecraft_focused"] != true {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            runtime.handle_preset("eye", false).await.unwrap();
        }
        println!("READY");
        std::io::stdout().flush().unwrap();
        runtime.run().await.unwrap();
    });
}

#[test]
fn observe_only_process_serves_http_and_restores_on_sigterm() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("run")).unwrap();
    fs::set_permissions(temp.path().join("run"), fs::Permissions::from_mode(0o700)).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fake_service_child", "--nocapture"])
        .env("MCSR_RUNTIME_FAKE_CHILD", temp.path())
        .env("MCSR_RUNTIME_FAKE_PORT", port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut ready = false;
    for _ in 0..5 {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.contains("READY") {
            ready = true;
            break;
        }
    }
    assert!(ready, "fake service did not start");
    let deadline = Instant::now() + Duration::from_secs(3);
    let response = loop {
        if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            write!(
                stream,
                "GET /api/status HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            break response;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("\"mode\":\"observe-only\""), "{response}");
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    assert!(child.wait().unwrap().success());
    assert_eq!(
        fs::read_to_string(temp.path().join("maccel.log")).unwrap(),
        "restore\nrestore\n"
    );
}

#[test]
fn active_python_style_service_lock_blocks_rust_start() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("run")).unwrap();
    fs::set_permissions(temp.path().join("run"), fs::Permissions::from_mode(0o700)).unwrap();
    let lock_path = temp.path().join("run/mcsr-toolkit-service.lock");
    let mut child = KillOnDrop(
        Command::new(std::env::current_exe().unwrap())
            .env("PATH", "/nonexistent")
            .args(["--exact", "legacy_service_lock_child", "--nocapture"])
            .env("MCSR_RUNTIME_LEGACY_LOCK_CHILD", &lock_path)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        for _ in 0..5 {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            if line.contains("LOCKED") {
                let _ = ready_tx.send(true);
                return;
            }
        }
        let _ = ready_tx.send(false);
    });
    let locked = ready_rx
        .recv_timeout(Duration::from_secs(3))
        .unwrap_or(false);
    assert!(locked, "legacy lock child did not acquire the lock");
    reader.join().unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let config = RuntimeConfig {
        runtime_dir: temp.path().join("run"),
        data_home: temp.path().join("data"),
        managed_dir: temp.path().join("managed"),
        port,
        enable_live_keyd: false,
        maccel_binary: temp.path().join("unused"),
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let outcome = rt.block_on(ServiceRuntime::start_with_peers(
        config,
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel(temp.path().join("maccel.log"))),
    ));
    drop(child);
    assert!(matches!(outcome, Err(error) if error.to_string().contains("already running")));
    assert!(!temp.path().join("maccel.log").exists());
}

#[test]
fn signal_restart_keeps_writer_drains_stale_fifo_and_restores_before_new_eye() {
    for signal in [libc::SIGTERM, libc::SIGKILL] {
        assert_restart_with_retained_writer(signal);
    }
}

fn assert_restart_with_retained_writer(signal: libc::c_int) {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("run")).unwrap();
    fs::set_permissions(temp.path().join("run"), fs::Permissions::from_mode(0o700)).unwrap();
    let state = mcsr_toolkit::paths::ensure_state(&temp.path().join("data")).unwrap();
    let mut profile = mcsr_toolkit::profile::Profile::default();
    profile.presets.push(mcsr_toolkit::profile::ResizePreset {
        id: "eye".into(),
        width: 1920,
        height: 1080,
        shortcut: "f7".into(),
        eye_zoom: true,
    });
    fs::write(
        state.join("profile.json"),
        serde_json::to_vec(&profile).unwrap(),
    )
    .unwrap();
    fs::set_permissions(
        state.join("profile.json"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "fake_service_child", "--nocapture"])
        .env("MCSR_RUNTIME_FAKE_CHILD", temp.path())
        .env("MCSR_RUNTIME_FAKE_PORT", port.to_string())
        .env("MCSR_RUNTIME_FAKE_EYE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut reader = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut ready = false;
    for _ in 0..5 {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.contains("READY") {
            ready = true;
            break;
        }
    }
    assert!(ready);
    let fifo_path = temp.path().join("run/mcsr-toolkit-focus.fifo");
    let mut stale = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&fifo_path)
        .unwrap();
    let original = stale.metadata().unwrap();
    unsafe {
        assert_eq!(libc::kill(child.id() as i32, signal), 0);
    }
    let status = child.wait().unwrap();
    assert_eq!(status.success(), signal == libc::SIGTERM);
    let retained = fs::symlink_metadata(&fifo_path).expect("service must retain the FIFO pathname");
    assert_eq!(
        (retained.dev(), retained.ino()),
        (original.dev(), original.ino())
    );
    stale
        .write_all(b"{\"class\":\"Minecraft\",\"title\":\"old\",\"modal\":false}\n")
        .unwrap();
    let config = RuntimeConfig {
        runtime_dir: temp.path().join("run"),
        data_home: temp.path().join("data"),
        managed_dir: temp.path().join("managed"),
        port,
        enable_live_keyd: false,
        maccel_binary: temp.path().join("unused"),
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let log_before_fresh_eye = rt.block_on(async {
        let mut runtime = ServiceRuntime::start_with_peers(
            config,
            Arc::new(FakeGnome),
            Arc::new(FakeMaccel(temp.path().join("maccel.log"))),
        )
        .await
        .unwrap();
        let reopened = fs::symlink_metadata(&fifo_path).unwrap();
        assert_eq!(
            (reopened.dev(), reopened.ino()),
            (original.dev(), original.ino())
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(runtime.focus_status()["minecraft_focused"], false);
        assert_eq!(runtime.focus_status()["extension_connected"], false);
        assert!(runtime.handle_preset("eye", false).await.is_err());
        let startup_log = fs::read_to_string(temp.path().join("maccel.log")).unwrap();
        let restorations = startup_log
            .strip_prefix("restore\neye\n")
            .expect("the child must enter eye mode after startup restoration");
        assert!(restorations.lines().all(|line| line == "restore"));
        assert!(restorations.lines().count() >= if signal == libc::SIGTERM { 2 } else { 1 });
        stale
            .write_all(b"{\"class\":\"Minecraft\",\"title\":\"fresh\",\"modal\":false}\n")
            .unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while runtime.focus_status()["minecraft_focused"] != true {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(runtime.focus_status()["extension_connected"], true);
        runtime.handle_preset("eye", false).await.unwrap();
        runtime.shutdown().await.unwrap();
        startup_log
    });
    let log = fs::read_to_string(temp.path().join("maccel.log")).unwrap();
    let after_fresh_eye = log
        .strip_prefix(&format!("{log_before_fresh_eye}eye\n"))
        .expect("startup restoration must precede the fresh eye request");
    // Focus-loss cleanup and explicit shutdown may each restore the baseline.
    assert!(matches!(
        after_fresh_eye,
        "restore\n" | "restore\nrestore\n"
    ));
}
