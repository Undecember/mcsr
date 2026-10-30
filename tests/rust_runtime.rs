use mcsr_toolkit::api::DownloadAdapter;
use mcsr_toolkit::downloads::Product;
use mcsr_toolkit::keyd::{CommandFailure, CommandOutput, CommandRunner};
use mcsr_toolkit::runtime::{RuntimeConfig, ServiceRuntime};
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
    time::Duration,
};

fn config(temp: &tempfile::TempDir) -> RuntimeConfig {
    let runtime_dir = temp.path().join("run");
    let data_home = temp.path().join("data");
    fs::create_dir(&runtime_dir).unwrap();
    fs::set_permissions(&runtime_dir, fs::Permissions::from_mode(0o700)).unwrap();
    RuntimeConfig {
        runtime_dir,
        data_home,
        managed_dir: temp.path().join("managed"),
        port: free_port(),
        enable_live_keyd: false,
        maccel_binary: temp.path().join("unused-maccel"),
    }
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[derive(Default)]
struct FakeGnome;
impl mcsr_toolkit::gnome::GnomePeer for FakeGnome {
    async fn get_focus(
        &self,
    ) -> Result<(String, String, u32, u32, bool), mcsr_toolkit::gnome::GnomeError> {
        Ok(("Minecraft".into(), "Game".into(), 1920, 1080, false))
    }
    async fn configure_presets(&self, _: String) -> Result<(), mcsr_toolkit::gnome::GnomeError> {
        Ok(())
    }
    async fn resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(u32, u32), mcsr_toolkit::gnome::GnomeError> {
        Ok((width, height))
    }
    async fn toggle_resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(u32, u32, bool), mcsr_toolkit::gnome::GnomeError> {
        Ok((width, height, false))
    }
}

#[derive(Default)]
struct FakeMaccel {
    calls: Mutex<Vec<&'static str>>,
    fail_restore: bool,
    fail_restore_call: Option<usize>,
}
impl mcsr_toolkit::maccel::MaccelPeer for FakeMaccel {
    async fn preflight(&self) -> Result<(), mcsr_toolkit::maccel::SensitivityError> {
        Ok(())
    }
    async fn eye(&self, _: &str) -> Result<String, mcsr_toolkit::maccel::SensitivityError> {
        self.calls.lock().unwrap().push("eye");
        Ok("0.02083000005222856998443603515625".into())
    }
    async fn restore(&self) -> Result<String, mcsr_toolkit::maccel::SensitivityError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push("restore");
        let restore_count = calls.iter().filter(|name| **name == "restore").count();
        if self.fail_restore || self.fail_restore_call == Some(restore_count) {
            return Err(mcsr_toolkit::maccel::SensitivityError(
                "fake recovery refused".into(),
            ));
        }
        Ok("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into())
    }
}

async fn start(
    config: RuntimeConfig,
) -> Result<ServiceRuntime<FakeGnome, FakeMaccel>, mcsr_toolkit::runtime::RuntimeError> {
    ServiceRuntime::start_with_peers(config, Arc::new(FakeGnome), Arc::new(FakeMaccel::default()))
        .await
}

fn write_focus(config: &RuntimeConfig, class: &str, modal: bool) {
    let mut fifo = fs::OpenOptions::new()
        .write(true)
        .open(config.runtime_dir.join("mcsr-toolkit-focus.fifo"))
        .unwrap();
    writeln!(
        fifo,
        "{{\"class\":\"{class}\",\"title\":\"Game\",\"modal\":{modal}}}"
    )
    .unwrap();
}

async fn wait_focus<P: mcsr_toolkit::gnome::GnomePeer + 'static>(
    runtime: &ServiceRuntime<P, FakeMaccel>,
    focused: bool,
) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if runtime.focus_status()["minecraft_focused"] == focused {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

fn write_eye_profile(config: &RuntimeConfig) {
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
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
}

#[tokio::test]
async fn duplicate_runtime_refuses_same_session_lock() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let first = start(config.clone()).await.unwrap();
    assert!(start(config).await.is_err());
    drop(first);
}

#[tokio::test]
async fn clean_shutdown_releases_the_service_lock() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let mut first = start(config.clone()).await.unwrap();
    first.shutdown().await.unwrap();
    let mut second = start(config).await.unwrap();
    second.shutdown().await.unwrap();
}

#[tokio::test]
async fn unclean_drop_keeps_service_lock_until_process_exit() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let peer = Arc::new(FakeMaccel::default());
    let runtime =
        ServiceRuntime::start_with_peers(config.clone(), Arc::new(FakeGnome), peer.clone())
            .await
            .unwrap();
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    runtime.handle_preset("eye", false).await.unwrap();
    drop(runtime);
    let second = start(config).await;
    assert!(matches!(second,Err(error) if error.to_string().contains("already running")));
    assert_eq!(*peer.calls.lock().unwrap(), ["restore", "eye"]);
}

#[tokio::test]
async fn mutation_marker_blocks_start_without_removing_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let marker = state.join("mutation.json");
    fs::write(&marker, b"{}").unwrap();
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(start(config).await.is_err());
    assert_eq!(fs::read(marker).unwrap(), b"{}");
}

#[tokio::test]
async fn pending_eye_is_restored_before_a_new_eye_signal() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let peer = Arc::new(FakeMaccel::default());
    let mut runtime =
        ServiceRuntime::start_with_peers(config.clone(), Arc::new(FakeGnome), peer.clone())
            .await
            .unwrap();
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    runtime.handle_preset("eye", false).await.unwrap();
    assert_eq!(*peer.calls.lock().unwrap(), ["restore", "eye"]);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn failed_startup_restore_prevents_eye_acceptance_and_http_binding() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let peer = Arc::new(FakeMaccel {
        fail_restore: true,
        ..Default::default()
    });
    assert!(
        ServiceRuntime::start_with_peers(config.clone(), Arc::new(FakeGnome), peer.clone())
            .await
            .is_err()
    );
    assert_eq!(*peer.calls.lock().unwrap(), ["restore"]);
    assert!(std::net::TcpStream::connect(("127.0.0.1", config.port)).is_err());
}

#[tokio::test]
async fn focus_disconnect_restores_eye_before_accepting_another_signal() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let peer = Arc::new(FakeMaccel::default());
    let mut runtime =
        ServiceRuntime::start_with_peers(config.clone(), Arc::new(FakeGnome), peer.clone())
            .await
            .unwrap();
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    runtime.handle_preset("eye", false).await.unwrap();
    write_focus(&config, "Minecraft", true);
    wait_focus(&runtime, false).await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if peer.calls.lock().unwrap().len() == 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(*peer.calls.lock().unwrap(), ["restore", "eye", "restore"]);
    assert!(runtime.handle_preset("eye", false).await.is_err());
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn extension_connection_survives_a_non_minecraft_heartbeat() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let mut runtime = start(config.clone()).await.unwrap();
    write_focus(&config, "Terminal", false);
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(runtime.focus_status()["minecraft_focused"], false);
    assert_eq!(runtime.focus_status()["extension_connected"], true);
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_retries_restoration_after_focus_cleanup_fails() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let peer = Arc::new(FakeMaccel {
        fail_restore_call: Some(2),
        ..Default::default()
    });
    let mut runtime =
        ServiceRuntime::start_with_peers(config.clone(), Arc::new(FakeGnome), peer.clone())
            .await
            .unwrap();
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    runtime.handle_preset("eye", false).await.unwrap();
    write_focus(&config, "Terminal", false);
    wait_focus(&runtime, false).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    let _ = runtime.shutdown().await;
    assert_eq!(
        *peer.calls.lock().unwrap(),
        ["restore", "eye", "restore", "restore"]
    );
}

struct BlockingDownload {
    started: std::sync::mpsc::Sender<()>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}
impl DownloadAdapter for BlockingDownload {
    fn download(
        &self,
        _product: Product,
        _destination: &std::path::Path,
    ) -> Result<serde_json::Value, String> {
        self.started.send(()).unwrap();
        self.release.lock().unwrap().recv().unwrap();
        Ok(serde_json::json!({"done":true}))
    }
}

async fn http(port: u16, request: String) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    String::from_utf8(response).unwrap()
}

#[tokio::test]
async fn shutdown_waits_for_an_accepted_download_job_before_restoring() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let download = Arc::new(BlockingDownload {
        started: started_tx,
        release: Mutex::new(release_rx),
    });
    let peer = Arc::new(FakeMaccel::default());
    let mut runtime = ServiceRuntime::start_with_adapters(
        config.clone(),
        Arc::new(FakeGnome),
        peer.clone(),
        download,
    )
    .await
    .unwrap();
    let session = http(
        config.port,
        format!(
            "GET /api/session HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            config.port
        ),
    )
    .await;
    let token =
        serde_json::from_str::<serde_json::Value>(session.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["token"]
            .as_str()
            .unwrap()
            .to_owned();
    let response = http(config.port,format!("POST /api/download/launcher HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: http://127.0.0.1:{}\r\nX-MCSR-Session: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",config.port,config.port,token)).await;
    assert!(response.starts_with("HTTP/1.1 202"), "{response}");
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut shutdown = tokio::spawn(async move { runtime.shutdown().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut shutdown)
            .await
            .is_err()
    );
    assert_eq!(*peer.calls.lock().unwrap(), ["restore"]);
    release_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), shutdown)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(*peer.calls.lock().unwrap(), ["restore", "restore"]);
}

#[tokio::test]
async fn absent_optional_maccel_serves_observe_only_without_helper_calls() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let peer = Arc::new(FakeMaccel::default());
    let mut runtime =
        ServiceRuntime::start_without_maccel(config.clone(), Arc::new(FakeGnome), peer.clone())
            .await
            .unwrap();
    assert_eq!(runtime.focus_status()["maccel"]["available"], false);
    assert_eq!(runtime.focus_status()["maccel"]["usable"], false);
    assert!(runtime.focus_status()["maccel"]["detail"].is_string());
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    assert!(runtime.handle_preset("eye", false).await.is_err());
    runtime.shutdown().await.unwrap();
    assert!(peer.calls.lock().unwrap().is_empty());
}

#[derive(Default)]
struct FakeKeyd {
    calls: Mutex<Vec<String>>,
    discovered_keys: Mutex<Option<String>>,
    list_keys_fails: Mutex<bool>,
}
impl CommandRunner for FakeKeyd {
    fn run(&self, args: &[String], _: &[u8]) -> Result<CommandOutput, CommandFailure> {
        self.calls.lock().unwrap().push(args.join(" "));
        if args.last().is_some_and(|item| item == "list-keys")
            && *self.list_keys_fails.lock().unwrap()
        {
            return Err(CommandFailure::Failed("list-keys unavailable".into()));
        }
        let stdout = if args.last().is_some_and(|item| item == "list-keys") {
            self.discovered_keys
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_default()
        } else if args.last().is_some_and(|item| item == "install") {
            "{\"backup_id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}".into()
        } else {
            String::new()
        };
        Ok(CommandOutput { stdout })
    }
    fn verify_helper(&self) -> Result<(), String> {
        Ok(())
    }
    fn keyd_binary(&self) -> Result<&'static str, String> {
        Ok("/fake/keyd")
    }
}

#[tokio::test]
async fn runtime_accepts_discovered_keyd_vocabulary_in_profile_api() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let runner = Arc::new(FakeKeyd::default());
    *runner.discovered_keys.lock().unwrap() = Some("kp1\n".into());
    let mut runtime = ServiceRuntime::start_with_runner(
        config.clone(),
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        runner,
    )
    .await
    .unwrap();
    let session = http(
        config.port,
        format!(
            "GET /api/session HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            config.port
        ),
    )
    .await;
    let token =
        serde_json::from_str::<serde_json::Value>(session.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["token"]
            .as_str()
            .unwrap()
            .to_owned();
    let body = r#"{"crafting":{"kp1":"s"}}"#;
    let response = http(config.port, format!("PUT /api/profile HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: http://127.0.0.1:{}\r\nX-MCSR-Session: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", config.port, config.port, token, body.len(), body)).await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn failed_keyd_discovery_keeps_fallback_vocabulary() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let runner = Arc::new(FakeKeyd::default());
    *runner.list_keys_fails.lock().unwrap() = true;
    let mut runtime = ServiceRuntime::start_with_runner(
        config.clone(),
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        runner,
    )
    .await
    .unwrap();
    let session = http(
        config.port,
        format!(
            "GET /api/session HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            config.port
        ),
    )
    .await;
    let token =
        serde_json::from_str::<serde_json::Value>(session.split("\r\n\r\n").nth(1).unwrap())
            .unwrap()["token"]
            .as_str()
            .unwrap()
            .to_owned();
    for (body, status) in [
        (r#"{"crafting":{"f8":"s"}}"#, "200"),
        (r#"{"crafting":{"kp1":"s"}}"#, "200"),
        (r#"{"crafting":{"backspace":"s"}}"#, "200"),
        (r#"{"crafting":{"mouse1":"s"}}"#, "200"),
        (r#"{"crafting":{"not_a_key":"s"}}"#, "422"),
    ] {
        let response = http(config.port, format!("PUT /api/profile HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: http://127.0.0.1:{}\r\nX-MCSR-Session: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", config.port, config.port, token, body.len(), body)).await;
        assert!(
            response.starts_with(&format!("HTTP/1.1 {status}")),
            "{response}"
        );
    }
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn focus_status_tracks_active_preset_and_keyd_event_shape() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let mut runtime = ServiceRuntime::start_with_runner(
        config.clone(),
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        Arc::new(FakeKeyd::default()),
    )
    .await
    .unwrap();
    let initial = runtime.focus_status();
    assert_eq!(initial["keyd_available"], true);
    assert_eq!(
        initial["events"],
        serde_json::json!({"activate":0,"clear":0})
    );
    assert!(initial["active_preset"].is_null());
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    runtime.handle_preset("eye", true).await.unwrap();
    assert_eq!(runtime.focus_status()["active_preset"], "eye");
    write_focus(&config, "Terminal", false);
    wait_focus(&runtime, false).await;
    assert!(runtime.focus_status()["active_preset"].is_null());
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn observe_only_restart_clears_a_previous_live_profile_before_serving() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let active = state.join("active-profile.json");
    fs::write(
        &active,
        serde_json::to_vec(&mcsr_toolkit::profile::Profile::default()).unwrap(),
    )
    .unwrap();
    fs::set_permissions(&active, fs::Permissions::from_mode(0o600)).unwrap();
    let keyd = Arc::new(FakeKeyd::default());
    let mut runtime = ServiceRuntime::start_with_runner(
        config,
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        keyd.clone(),
    )
    .await
    .unwrap();
    let calls: Vec<_> = keyd
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|call| !call.ends_with("list-keys"))
        .cloned()
        .collect();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].ends_with("bind reset"));
    assert!(calls[1].ends_with("privileged keyd clear"));
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn unsafe_reconciliation_cannot_reactivate_keyd_on_next_focus_tick() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = config(&temp);
    config.enable_live_keyd = true;
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let active = state.join("active-profile.json");
    fs::write(
        &active,
        serde_json::to_vec(&mcsr_toolkit::profile::Profile::default()).unwrap(),
    )
    .unwrap();
    fs::set_permissions(&active, fs::Permissions::from_mode(0o600)).unwrap();
    let runner = Arc::new(FakeKeyd::default());
    let mut runtime = ServiceRuntime::start_with_runner(
        config.clone(),
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        runner.clone(),
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while runtime.focus_status()["cleanup_error"].is_null() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    tokio::time::sleep(Duration::from_millis(600)).await;
    let calls = runner.calls.lock().unwrap().clone();
    assert!(
        !calls
            .iter()
            .any(|call| call.starts_with("/fake/keyd bind reset ")),
        "{calls:?}"
    );
    runtime.shutdown().await.unwrap();
}

struct BlockingGnome {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl mcsr_toolkit::gnome::GnomePeer for BlockingGnome {
    async fn get_focus(
        &self,
    ) -> Result<(String, String, u32, u32, bool), mcsr_toolkit::gnome::GnomeError> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(("Minecraft".into(), "Game".into(), 1920, 1080, false))
    }
    async fn configure_presets(&self, _: String) -> Result<(), mcsr_toolkit::gnome::GnomeError> {
        Ok(())
    }
    async fn resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(u32, u32), mcsr_toolkit::gnome::GnomeError> {
        Ok((width, height))
    }
}

#[tokio::test]
async fn shutdown_waits_for_inflight_signal_before_final_restore() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let gnome = Arc::new(BlockingGnome {
        entered: entered.clone(),
        release: release.clone(),
    });
    let peer = Arc::new(FakeMaccel::default());
    let mut runtime = ServiceRuntime::start_with_peers(config.clone(), gnome, peer.clone())
        .await
        .unwrap();
    write_focus(&config, "Minecraft", false);
    tokio::time::timeout(Duration::from_secs(2), async {
        while runtime.focus_status()["minecraft_focused"] != true {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    runtime.attach_preset_signals(receiver);
    sender.send(("eye".into(), false)).await.unwrap();
    entered.notified().await;
    let mut shutdown = tokio::spawn(async move { runtime.shutdown().await });
    assert!(
        tokio::time::timeout(Duration::from_millis(40), &mut shutdown)
            .await
            .is_err()
    );
    assert_eq!(*peer.calls.lock().unwrap(), ["restore"]);
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), shutdown)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(*peer.calls.lock().unwrap(), ["restore", "restore"]);
}

#[tokio::test]
async fn failed_eye_recovery_still_clears_previous_keyd_bindings() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let active = state.join("active-profile.json");
    fs::write(
        &active,
        serde_json::to_vec(&mcsr_toolkit::profile::Profile::default()).unwrap(),
    )
    .unwrap();
    fs::set_permissions(&active, fs::Permissions::from_mode(0o600)).unwrap();
    let keyd = Arc::new(FakeKeyd::default());
    let maccel = Arc::new(FakeMaccel {
        fail_restore: true,
        ..Default::default()
    });
    assert!(
        ServiceRuntime::start_with_runner(config, Arc::new(FakeGnome), maccel, keyd.clone())
            .await
            .is_err()
    );
    assert_eq!(
        keyd.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| !call.ends_with("list-keys"))
            .count(),
        2
    );
}

#[tokio::test]
async fn mutation_marker_blocks_start_after_clearing_known_live_bindings() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    for (name, bytes) in [
        (
            "active-profile.json",
            serde_json::to_vec(&mcsr_toolkit::profile::Profile::default()).unwrap(),
        ),
        ("mutation.json", b"{}".to_vec()),
    ] {
        let path = state.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let keyd = Arc::new(FakeKeyd::default());
    assert!(ServiceRuntime::start_with_runner(
        config,
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        keyd.clone()
    )
    .await
    .is_err());
    assert_eq!(
        keyd.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| !call.ends_with("list-keys"))
            .count(),
        2
    );
    assert!(state.join("mutation.json").exists());
}

fn fixture_instance(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let dir = root.join("instances").join(name);
    fs::create_dir_all(dir.join("minecraft")).unwrap();
    fs::write(dir.join("instance.json"), format!(r#"{{"id":"{name}"}}"#)).unwrap();
    let options = dir.join("minecraft/options.txt");
    fs::write(&options, b"key_key.back:key.keyboard.s\n").unwrap();
    options
}

#[tokio::test]
async fn live_runtime_reconciles_a_new_launcher_instance() {
    use mcsr_toolkit::apply::ApplyCoordinator;
    let temp = tempfile::tempdir().unwrap();
    let mut config = config(&temp);
    config.enable_live_keyd = true;
    let root = config.managed_dir.join("launcher");
    let one = fixture_instance(&root, "one");
    let mut profile = mcsr_toolkit::profile::Profile::default();
    profile.crafting.insert("s".into(), "k".into());
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let runner = Arc::new(FakeKeyd::default());
    let keyd = mcsr_toolkit::keyd::KeydController::new(&state, runner.clone(), true);
    ApplyCoordinator::new(&root, &config.data_home, vec![])
        .unwrap()
        .apply(&profile, &keyd)
        .unwrap();
    assert!(fs::read(&one)
        .unwrap()
        .ends_with(b"key_key.back:key.keyboard.k\n"));
    let active = state.join("active-profile.json");
    fs::write(&active, serde_json::to_vec(&profile).unwrap()).unwrap();
    fs::set_permissions(&active, fs::Permissions::from_mode(0o600)).unwrap();
    let mut runtime = ServiceRuntime::start_with_runner(
        config,
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        runner,
    )
    .await
    .unwrap();
    let two = fixture_instance(&root, "two");
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if fs::read(&two)
                .unwrap()
                .ends_with(b"key_key.back:key.keyboard.k\n")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn live_startup_does_not_restart_keyd_after_verified_clear_for_route_setup() {
    use mcsr_toolkit::apply::ApplyCoordinator;
    let temp = tempfile::tempdir().unwrap();
    let mut config = config(&temp);
    config.enable_live_keyd = true;
    let root = config.managed_dir.join("launcher");
    fixture_instance(&root, "one");
    let profile = mcsr_toolkit::profile::Profile::default();
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let runner = Arc::new(FakeKeyd::default());
    let keyd = mcsr_toolkit::keyd::KeydController::new(&state, runner.clone(), true);
    ApplyCoordinator::new(&root, &config.data_home, vec![])
        .unwrap()
        .apply(&profile, &keyd)
        .unwrap();
    let active = state.join("active-profile.json");
    fs::write(&active, serde_json::to_vec(&profile).unwrap()).unwrap();
    fs::set_permissions(&active, fs::Permissions::from_mode(0o600)).unwrap();
    runner.calls.lock().unwrap().clear();
    let mut runtime = ServiceRuntime::start_with_runner(
        config,
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        runner.clone(),
    )
    .await
    .unwrap();
    let clears = runner
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|call| {
            call.starts_with("/usr/bin/sudo -n /usr/local/libexec/")
                && call.ends_with("privileged keyd clear")
        })
        .count();
    assert_eq!(
        clears,
        2,
        "startup should use bootstrap and active-profile clears only; status={:?}; calls={:?}",
        runtime.focus_status(),
        runner.calls.lock().unwrap()
    );
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn live_runtime_finishes_deferred_apply_after_instance_closes() {
    use mcsr_toolkit::apply::ApplyCoordinator;
    let temp = tempfile::tempdir().unwrap();
    let mut config = config(&temp);
    config.enable_live_keyd = true;
    let root = config.managed_dir.join("launcher");
    let options = fixture_instance(&root, "one");
    let mut profile = mcsr_toolkit::profile::Profile::default();
    profile.crafting.insert("s".into(), "k".into());
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let runner = Arc::new(FakeKeyd::default());
    let keyd = mcsr_toolkit::keyd::KeydController::new(&state, runner.clone(), true);
    let report = ApplyCoordinator::new(
        &root,
        &config.data_home,
        vec![options.parent().unwrap().to_path_buf()],
    )
    .unwrap()
    .apply(&profile, &keyd)
    .unwrap();
    assert_eq!(report.instances[0].status, "deferred");
    assert!(state.join("pending.json").exists());
    let mut runtime = ServiceRuntime::start_with_runner(
        config,
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        runner,
    )
    .await
    .unwrap();
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if fs::read(&options)
                .unwrap()
                .ends_with(b"key_key.back:key.keyboard.k\n")
                && state.join("active-profile.json").exists()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn saved_edits_cannot_change_an_applied_resize() {
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    write_eye_profile(&config);
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let mut active: mcsr_toolkit::profile::Profile =
        serde_json::from_slice(&fs::read(state.join("profile.json")).unwrap()).unwrap();
    active.presets[0].width = 800;
    active.presets[0].height = 600;
    active.presets[0].eye_zoom = false;
    mcsr_toolkit::paths::atomic_replace(
        &state.join("active-profile.json"),
        &serde_json::to_vec(&active).unwrap(),
        true,
    )
    .unwrap();
    let mut runtime = ServiceRuntime::start_with_runner(
        config.clone(),
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        Arc::new(FakeKeyd::default()),
    )
    .await
    .unwrap();
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    let result = runtime.handle_preset("eye", false).await.unwrap();
    assert_eq!((result.width, result.height), (800, 600));
    runtime.shutdown().await.unwrap();
}

#[tokio::test]
async fn unchanged_reconciliation_preserves_activation_acknowledgement() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = config(&temp);
    config.enable_live_keyd = true;
    let game = config.managed_dir.join("instances/one/minecraft");
    fs::create_dir_all(&game).unwrap();
    fs::write(
        game.parent().unwrap().join("instance.json"),
        r#"{"id":"one"}"#,
    )
    .unwrap();
    fs::write(game.join("options.txt"), "key_key.back:key.keyboard.s\n").unwrap();
    struct AppliedKeyd;
    impl mcsr_toolkit::apply::KeydAdapter for AppliedKeyd {
        fn install(&self, _: &mcsr_toolkit::profile::Profile) -> Result<(), String> {
            Ok(())
        }
        fn checkpoint(&self) -> Result<Vec<String>, String> {
            Ok(vec![])
        }
        fn restore_to(&self, _: &[String]) -> Result<(), String> {
            Ok(())
        }
        fn rollback(&self) -> Result<(), String> {
            Ok(())
        }
    }
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let active = mcsr_toolkit::profile::Profile {
        launcher_root: Some(config.managed_dir.to_string_lossy().into()),
        ..Default::default()
    };
    mcsr_toolkit::apply::ApplyCoordinator::new(&config.managed_dir, &config.data_home, vec![])
        .unwrap()
        .apply(&active, &AppliedKeyd)
        .unwrap();
    mcsr_toolkit::paths::atomic_replace(
        &state.join("active-profile.json"),
        &serde_json::to_vec(&active).unwrap(),
        true,
    )
    .unwrap();
    let mut runtime = ServiceRuntime::start_with_runner(
        config.clone(),
        Arc::new(FakeGnome),
        Arc::new(FakeMaccel::default()),
        Arc::new(FakeKeyd::default()),
    )
    .await
    .unwrap();
    // The first reconciliation interval ticks immediately, republishing the same profile.
    tokio::time::sleep(Duration::from_millis(200)).await;
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    tokio::time::sleep(Duration::from_millis(550)).await;
    let status = runtime.focus_status();
    runtime.shutdown().await.unwrap();
    assert!(
        status["events"]["activate"].as_u64().unwrap() > 0,
        "{status}"
    );
}

#[tokio::test]
async fn status_tracks_preset_full_size_and_focus_invalidation() {
    use mcsr_toolkit::gnome::{GnomeError, GnomePeer};
    struct ToggleGnome(Mutex<bool>);
    impl GnomePeer for ToggleGnome {
        async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
            Ok(("Minecraft".into(), "Game".into(), 800, 600, false))
        }
        async fn configure_presets(&self, _: String) -> Result<(), GnomeError> {
            Ok(())
        }
        async fn resize_focused(&self, w: u32, h: u32) -> Result<(u32, u32), GnomeError> {
            Ok((w, h))
        }
        async fn toggle_resize_focused(
            &self,
            w: u32,
            h: u32,
        ) -> Result<(u32, u32, bool), GnomeError> {
            let mut full = self.0.lock().unwrap();
            let reply = if *full {
                (1920, 1080, true)
            } else {
                (w, h, false)
            };
            *full = !*full;
            Ok(reply)
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let config = config(&temp);
    let state = mcsr_toolkit::paths::ensure_state(&config.data_home).unwrap();
    let mut profile = mcsr_toolkit::profile::Profile::default();
    profile.presets.push(mcsr_toolkit::profile::ResizePreset {
        id: "normal".into(),
        width: 800,
        height: 600,
        shortcut: "f6".into(),
        eye_zoom: false,
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
    let mut runtime = ServiceRuntime::start_with_runner(
        config.clone(),
        Arc::new(ToggleGnome(Mutex::new(false))),
        Arc::new(FakeMaccel::default()),
        Arc::new(FakeKeyd::default()),
    )
    .await
    .unwrap();
    assert!(runtime.focus_status()["window_mode"].is_null());
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    runtime.handle_preset("normal", false).await.unwrap();
    assert_eq!(runtime.focus_status()["window_mode"], "preset");
    assert_eq!(runtime.focus_status()["active_preset"], "normal");
    runtime.handle_preset("normal", false).await.unwrap();
    assert_eq!(runtime.focus_status()["window_mode"], "full_size");
    assert!(runtime.focus_status()["active_preset"].is_null());
    runtime.handle_preset("normal", false).await.unwrap();
    assert_eq!(runtime.focus_status()["window_mode"], "preset");
    assert_eq!(runtime.focus_status()["active_preset"], "normal");
    write_focus(&config, "Terminal", false);
    wait_focus(&runtime, false).await;
    write_focus(&config, "Minecraft", false);
    wait_focus(&runtime, true).await;
    assert!(runtime.focus_status()["window_mode"].is_null());
    assert!(runtime.focus_status()["active_preset"].is_null());
    runtime.shutdown().await.unwrap();
}
