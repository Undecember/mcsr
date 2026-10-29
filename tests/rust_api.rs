use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use mcsr_toolkit::api::{router, ApiBackend, ApiConfig, ApiState, DownloadAdapter, RuntimeAdapter};
use mcsr_toolkit::apply::{ApplyPreview, ApplyReport, InstanceResult};
use mcsr_toolkit::downloads::Product;
use mcsr_toolkit::profile::{KeySet, Profile};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tempfile::TempDir;
use tower::ServiceExt;

struct Runtime {
    live: bool,
}
impl RuntimeAdapter for Runtime {
    fn process_cwds(&self) -> Vec<PathBuf> {
        vec![]
    }
    fn focus_status(&self) -> Value {
        json!({"mode":"observe-only"})
    }
    fn live_activation_enabled(&self) -> bool {
        self.live
    }
}
struct FakeBackend {
    block: Option<Arc<(Mutex<bool>, Condvar)>>,
}
impl ApiBackend for FakeBackend {
    fn preview(&self, _: &Path, _: &Profile, _: Vec<PathBuf>) -> Result<ApplyPreview, String> {
        Ok(ApplyPreview {
            instances: vec![InstanceResult {
                instance_id: "one".into(),
                status: "ready".into(),
                detail: String::new(),
            }],
            mappings: vec![],
        })
    }
    fn apply(
        &self,
        _: &Path,
        profile: &Profile,
        _: Vec<PathBuf>,
        publish: &dyn Fn(&Profile) -> Result<(), String>,
    ) -> Result<ApplyReport, String> {
        if let Some(block) = &self.block {
            let (lock, changed) = &**block;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = changed.wait(released).unwrap();
            }
        }
        publish(profile)?;
        Ok(report())
    }
    fn restore(
        &self,
        _: &Path,
        _: &str,
        _: Vec<PathBuf>,
        before: &dyn Fn() -> Result<(), String>,
        publish: &dyn Fn(Option<&Profile>) -> Result<(), String>,
    ) -> Result<ApplyReport, String> {
        before()?;
        if let Some(block) = &self.block {
            let (lock, changed) = &**block;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = changed.wait(released).unwrap();
            }
        }
        publish(None)?;
        Ok(report())
    }
}
fn report() -> ApplyReport {
    ApplyReport {
        instances: vec![],
        backup_id: Some("a".repeat(32)),
        keyd_error: None,
        restored_profile: None,
    }
}
struct FakeDownload;
impl DownloadAdapter for FakeDownload {
    fn download(&self, _: Product, destination: &Path) -> Result<Value, String> {
        Ok(json!({"version":"v1","path":destination,"byte_count":12}))
    }
}
struct Fixture {
    temp: TempDir,
    state: Arc<ApiState>,
    options: PathBuf,
}
impl Fixture {
    fn new(live: bool, block: Option<Arc<(Mutex<bool>, Condvar)>>) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("launcher/instances/one/minecraft");
        fs::create_dir_all(&game).unwrap();
        fs::write(
            game.parent().unwrap().join("instance.json"),
            r#"{"id":"one"}"#,
        )
        .unwrap();
        let options = game.join("options.txt");
        fs::write(&options, b"key_key.back:key.keyboard.s\n").unwrap();
        let state = Arc::new(
            ApiState::new(
                ApiConfig {
                    profile_path: temp.path().join("profile.json"),
                    managed_dir: temp.path().to_owned(),
                    state_dir: temp.path().join("state"),
                    port: 8765,
                    keys: KeySet::fallback(),
                },
                Arc::new(FakeBackend { block }),
                Arc::new(Runtime { live }),
                Arc::new(FakeDownload),
            )
            .unwrap(),
        );
        Self {
            temp,
            state,
            options,
        }
    }
    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Value,
        origin: Option<&str>,
        token: Option<&str>,
        host: Option<&str>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("host", host.unwrap_or("127.0.0.1:8765"));
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        if let Some(token) = token {
            builder = builder.header("x-mcsr-session", token);
        }
        let response = router(self.state.clone())
            .oneshot(builder.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }
    async fn write(&self, method: Method, path: &str, body: Value) -> (StatusCode, Value) {
        self.request(
            method,
            path,
            body,
            Some("http://127.0.0.1:8765"),
            Some(&self.state.session_token),
            None,
        )
        .await
    }
}

#[test]
fn service_binds_only_loopback() {
    assert_eq!(
        ApiState::bind_address(8765).unwrap().to_string(),
        "127.0.0.1:8765"
    );
    assert!(ApiState::bind_address(0).is_err());
}

#[tokio::test]
async fn embedded_assets_session_and_status_have_ui_contract() {
    let fixture = Fixture::new(false, None);
    let response = router(fixture.state.clone())
        .oneshot(
            Request::builder()
                .uri("/app.js")
                .header("host", "127.0.0.1:8765")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/javascript; charset=utf-8"
    );
    let (status, session) = fixture
        .request(Method::GET, "/api/session", Value::Null, None, None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(session["token"], fixture.state.session_token);
    let (_, status) = fixture
        .request(Method::GET, "/api/status", Value::Null, None, None, None)
        .await;
    assert_eq!(status["live_activation_enabled"], false);
    assert_eq!(status["instances"][0]["id"], "one");
    assert_eq!(status["focus"]["mode"], "observe-only");
}

#[tokio::test]
async fn profile_preview_stale_apply_and_observe_only() {
    let fixture = Fixture::new(true, None);
    let root = fixture
        .temp
        .path()
        .join("launcher")
        .to_string_lossy()
        .to_string();
    let first = json!({"crafting":{"s":"k"},"launcher_root":root});
    let (status, saved) = fixture.write(Method::PUT, "/api/profile", first).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["crafting"]["s"], "k");
    let (status, preview) = fixture
        .write(Method::POST, "/api/preview", saved.clone())
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["instances"][0]["status"], "ready");
    let second = json!({"crafting":{"s":"x"},"launcher_root":root});
    assert_eq!(
        fixture.write(Method::PUT, "/api/profile", second).await.0,
        StatusCode::OK
    );
    let (status, body) = fixture.write(Method::POST, "/api/apply", saved).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["detail"]["code"], "stale_profile");
    let observe = Fixture::new(false, None);
    let (status, body) = observe.write(Method::POST, "/api/apply", json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["detail"]["code"], "safety_gate");
}

#[tokio::test]
async fn security_rejections_preserve_profile_and_options_bytes() {
    let fixture = Fixture::new(true, None);
    let profile = fixture.state.profile_path.clone();
    fs::write(&profile, b"{}\n").unwrap();
    let before_options = fs::read(&fixture.options).unwrap();
    let cases = [
        (
            Some("http://evil.test"),
            Some(fixture.state.session_token.as_str()),
            None,
            "bad_origin",
        ),
        (
            None,
            Some(fixture.state.session_token.as_str()),
            None,
            "bad_session",
        ),
        (Some("http://127.0.0.1:8765"), None, None, "bad_session"),
        (
            Some("http://127.0.0.1:8765"),
            Some("wrong"),
            None,
            "bad_session",
        ),
        (
            Some("http://127.0.0.1:8765"),
            Some(fixture.state.session_token.as_str()),
            Some("evil.test"),
            "bad_host",
        ),
    ];
    for (origin, token, host, code) in cases {
        let (status, body) = fixture
            .request(
                Method::PUT,
                "/api/profile",
                json!({"crafting":{"s":"k"}}),
                origin,
                token,
                host,
            )
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["detail"]["code"], code);
        assert_eq!(fs::read(&profile).unwrap(), b"{}\n");
        assert_eq!(fs::read(&fixture.options).unwrap(), before_options);
    }
    let (status, body) = fixture
        .request(
            Method::GET,
            "/api/session",
            Value::Null,
            Some("http://evil.test"),
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["detail"]["code"], "bad_origin");
}

#[tokio::test]
async fn one_job_at_a_time_and_drain_on_shutdown() {
    let block = Arc::new((Mutex::new(false), Condvar::new()));
    let fixture = Fixture::new(true, Some(block.clone()));
    let root = fixture
        .temp
        .path()
        .join("launcher")
        .to_string_lossy()
        .to_string();
    let (_, profile) = fixture
        .write(Method::PUT, "/api/profile", json!({"launcher_root":root}))
        .await;
    let (status, accepted) = fixture
        .write(Method::POST, "/api/apply", profile.clone())
        .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let job_id = accepted["id"].as_str().unwrap();
    let (status, body) = fixture.write(Method::POST, "/api/apply", profile).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["detail"]["code"], "job_busy");
    let (status, body) = fixture
        .write(
            Method::PUT,
            "/api/profile",
            json!({"crafting":{"s":"k"},"launcher_root":root}),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["detail"]["code"], "job_busy");
    let (status, body) = fixture
        .request(
            Method::GET,
            &format!("/api/jobs/{job_id}"),
            Value::Null,
            None,
            None,
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "running");
    {
        let (lock, changed) = &*block;
        *lock.lock().unwrap() = true;
        changed.notify_all();
    }
    fixture.state.drain_jobs();
    let (_, body) = fixture
        .request(
            Method::GET,
            &format!("/api/jobs/{job_id}"),
            Value::Null,
            None,
            None,
            None,
        )
        .await;
    assert_eq!(body["status"], "succeeded");
    assert_eq!(body["result"]["backup_id"], "a".repeat(32));
}

#[tokio::test]
async fn download_allowlist_and_unknown_job() {
    let fixture = Fixture::new(false, None);
    let (status, body) = fixture
        .write(Method::POST, "/api/download/ninjabrain", Value::Null)
        .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    let id = body["id"].as_str().unwrap();
    for _ in 0..50 {
        let (_, job) = fixture
            .request(
                Method::GET,
                &format!("/api/jobs/{id}"),
                Value::Null,
                None,
                None,
                None,
            )
            .await;
        if job["status"] == "succeeded" {
            assert!(job["result"]["path"]
                .as_str()
                .unwrap()
                .ends_with("Ninjabrain-Bot.jar"));
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert_eq!(
        fixture
            .write(Method::POST, "/api/download/evil", Value::Null)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (_, unknown) = fixture
        .request(
            Method::GET,
            "/api/jobs/absent",
            Value::Null,
            None,
            None,
            None,
        )
        .await;
    assert_eq!(unknown["detail"]["code"], "unknown_job");
}

#[tokio::test]
async fn invalid_profile_and_launcher_root_do_not_write_files() {
    let fixture = Fixture::new(true, None);
    let before = fs::read(&fixture.options).unwrap();
    let (status, _) = fixture
        .write(
            Method::PUT,
            "/api/profile",
            json!({"launcher_root":fixture.temp.path()}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .write(Method::PUT, "/api/profile", json!({"command":"rm -rf /"}))
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, _) = fixture
        .write(
            Method::PUT,
            "/api/profile",
            json!({"crafting":{"s":"k; id"}}),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(!fixture.state.profile_path.exists());
    assert_eq!(fs::read(&fixture.options).unwrap(), before);
}

#[tokio::test]
async fn mutation_marker_blocks_apply_and_status_live_gate() {
    let fixture = Fixture::new(true, None);
    fs::create_dir_all(&fixture.state.state_dir).unwrap();
    fs::write(fixture.state.state_dir.join("mutation.json"), b"{}").unwrap();
    let (_, status) = fixture
        .request(Method::GET, "/api/status", Value::Null, None, None, None)
        .await;
    assert_eq!(status["live_activation_enabled"], false);
    let (code, body) = fixture.write(Method::POST, "/api/apply", json!({})).await;
    assert_eq!(code, StatusCode::FORBIDDEN);
    assert_eq!(body["detail"]["code"], "safety_gate");
}

#[derive(Default)]
struct NativeKeyd {
    history: Mutex<Vec<String>>,
}
impl mcsr_toolkit::apply::KeydAdapter for NativeKeyd {
    fn install(&self, _: &Profile) -> Result<(), String> {
        let mut history = self.history.lock().unwrap();
        let next = history.len() + 1;
        history.push(format!("snapshot-{next}"));
        Ok(())
    }
    fn checkpoint(&self) -> Result<Vec<String>, String> {
        Ok(self.history.lock().unwrap().clone())
    }
    fn restore_to(&self, checkpoint: &[String]) -> Result<(), String> {
        *self.history.lock().unwrap() = checkpoint.to_vec();
        Ok(())
    }
    fn rollback(&self) -> Result<(), String> {
        self.history.lock().unwrap().pop();
        Ok(())
    }
}

#[tokio::test]
async fn native_restore_publishes_prior_profile_before_marker_removal() {
    use mcsr_toolkit::api::{ApiConfig, NativeBackend};
    use mcsr_toolkit::paths::state_dir;
    let mut fixture = Fixture::new(true, None);
    let data_home = fixture.temp.path().join("data");
    fixture.state = Arc::new(
        ApiState::new(
            ApiConfig {
                profile_path: fixture.temp.path().join("profile.json"),
                managed_dir: fixture.temp.path().to_owned(),
                state_dir: state_dir(&data_home),
                port: 8765,
                keys: KeySet::fallback(),
            },
            Arc::new(NativeBackend {
                data_home,
                keyd: Arc::new(NativeKeyd::default()),
            }),
            Arc::new(Runtime { live: true }),
            Arc::new(FakeDownload),
        )
        .unwrap(),
    );
    let root = fixture
        .temp
        .path()
        .join("launcher")
        .to_string_lossy()
        .to_string();
    let (_, first) = fixture
        .write(
            Method::PUT,
            "/api/profile",
            json!({"launcher_root":root,"crafting":{"s":"k"}}),
        )
        .await;
    let (_, job) = fixture
        .write(Method::POST, "/api/apply", first.clone())
        .await;
    let first_result = completed_job(&fixture, job["id"].as_str().unwrap()).await;
    assert_eq!(first_result["status"], "succeeded");
    let (_, second) = fixture
        .write(
            Method::PUT,
            "/api/profile",
            json!({"launcher_root":root,"crafting":{"s":"j"}}),
        )
        .await;
    let (_, job) = fixture.write(Method::POST, "/api/apply", second).await;
    let second_result = completed_job(&fixture, job["id"].as_str().unwrap()).await;
    assert_eq!(second_result["status"], "succeeded");
    let backup_id = second_result["result"]["backup_id"].as_str().unwrap();
    let (_, job) = fixture
        .write(Method::POST, "/api/restore", json!({"backup_id":backup_id}))
        .await;
    let restored = completed_job(&fixture, job["id"].as_str().unwrap()).await;
    assert_eq!(restored["status"], "succeeded");
    let (_, profile) = fixture
        .request(Method::GET, "/api/profile", Value::Null, None, None, None)
        .await;
    assert_eq!(profile, first);
    assert!(!fixture.state.state_dir.join("mutation.json").exists());
    assert_eq!(
        fs::read_to_string(&fixture.options).unwrap(),
        "key_key.back:key.keyboard.k\n"
    );
}

async fn completed_job(fixture: &Fixture, id: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let (_, job) = fixture
                .request(
                    Method::GET,
                    &format!("/api/jobs/{id}"),
                    Value::Null,
                    None,
                    None,
                    None,
                )
                .await;
            if job["status"] != "running" {
                return job;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("job did not complete within 10 seconds")
}

struct BlockingRuntime {
    entered: std::sync::atomic::AtomicBool,
    released: Mutex<bool>,
    changed: Condvar,
}
impl RuntimeAdapter for BlockingRuntime {
    fn process_cwds(&self) -> Vec<PathBuf> {
        self.entered
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.changed.wait(released).unwrap();
        }
        vec![]
    }
    fn focus_status(&self) -> Value {
        json!({"mode":"observe-only"})
    }
    fn live_activation_enabled(&self) -> bool {
        true
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn restore_holds_profile_lock_until_job_is_reserved() {
    let fixture = Fixture::new(true, None);
    let job_release = Arc::new((Mutex::new(false), Condvar::new()));
    let runtime = Arc::new(BlockingRuntime {
        entered: std::sync::atomic::AtomicBool::new(false),
        released: Mutex::new(false),
        changed: Condvar::new(),
    });
    let profile_path = fixture.temp.path().join("profile.json");
    let root = fixture
        .temp
        .path()
        .join("launcher")
        .to_string_lossy()
        .to_string();
    fs::write(&profile_path, json!({"launcher_root":root}).to_string()).unwrap();
    let state = Arc::new(
        ApiState::new(
            ApiConfig {
                profile_path: profile_path.clone(),
                managed_dir: fixture.temp.path().to_owned(),
                state_dir: fixture.temp.path().join("state"),
                port: 8765,
                keys: KeySet::fallback(),
            },
            Arc::new(FakeBackend {
                block: Some(job_release.clone()),
            }),
            runtime.clone(),
            Arc::new(FakeDownload),
        )
        .unwrap(),
    );
    let origin = "http://127.0.0.1:8765";
    let restore_router = router(state.clone());
    let token = state.session_token.clone();
    let restore = tokio::spawn(async move {
        restore_router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/restore")
                    .header("host", "127.0.0.1:8765")
                    .header("origin", origin)
                    .header("x-mcsr-session", token)
                    .body(Body::from(json!({"backup_id":"a".repeat(32)}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    });
    for _ in 0..100 {
        if runtime.entered.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert!(runtime.entered.load(std::sync::atomic::Ordering::SeqCst));
    let put_router = router(state.clone());
    let token = state.session_token.clone();
    let mut put = tokio::spawn(async move {
        put_router
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/profile")
                    .header("host", "127.0.0.1:8765")
                    .header("origin", origin)
                    .header("x-mcsr-session", token)
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut put)
            .await
            .is_err(),
        "PUT saved while Restore selected its profile/root"
    );
    {
        *runtime.released.lock().unwrap() = true;
        runtime.changed.notify_all();
    }
    let restore_status = restore.await.unwrap();
    let put_status = put.await.unwrap();
    // Keep the reserved restore running until PUT has observed the reservation.
    // An immediately completed fake job made the old assertion scheduler-dependent.
    *job_release.0.lock().unwrap() = true;
    job_release.1.notify_all();
    assert_eq!(restore_status, StatusCode::ACCEPTED);
    assert_eq!(put_status, StatusCode::CONFLICT);
    assert_ne!(fs::read(&profile_path).unwrap(), b"{}");
    state.drain_jobs();
}

struct TrackingRuntime {
    events: Mutex<Vec<&'static str>>,
}
impl RuntimeAdapter for TrackingRuntime {
    fn process_cwds(&self) -> Vec<PathBuf> {
        vec![]
    }
    fn focus_status(&self) -> Value {
        json!({"mode":"observe-only"})
    }
    fn live_activation_enabled(&self) -> bool {
        true
    }
    fn with_mutation_guard(
        &self,
        operation: &mut dyn FnMut() -> Result<Value, String>,
    ) -> Result<Value, String> {
        self.events.lock().unwrap().push("guard_enter");
        let result = operation();
        self.events.lock().unwrap().push("guard_exit");
        result
    }
    fn before_apply(&self, _: &Profile) -> Result<(), String> {
        self.events.lock().unwrap().push("before_apply");
        Ok(())
    }
    fn on_applied(&self, _: &Profile) -> Result<(), String> {
        self.events.lock().unwrap().push("on_applied");
        Ok(())
    }
    fn before_restore(&self) -> Result<(), String> {
        self.events.lock().unwrap().push("before_restore");
        Ok(())
    }
    fn after_restore(&self, _: Option<&Profile>) -> Result<(), String> {
        self.events.lock().unwrap().push("after_restore");
        Ok(())
    }
    fn mutation_failed(&self, _: &str) {
        self.events.lock().unwrap().push("mutation_failed");
    }
}

#[tokio::test]
async fn runtime_hooks_run_in_serial_mutation_guard() {
    let fixture = Fixture::new(true, None);
    let runtime = Arc::new(TrackingRuntime {
        events: Mutex::new(vec![]),
    });
    let root = fixture
        .temp
        .path()
        .join("launcher")
        .to_string_lossy()
        .to_string();
    let state = Arc::new(
        ApiState::new(
            ApiConfig {
                profile_path: fixture.temp.path().join("profile.json"),
                managed_dir: fixture.temp.path().to_owned(),
                state_dir: fixture.temp.path().join("state"),
                port: 8765,
                keys: KeySet::fallback(),
            },
            Arc::new(FakeBackend { block: None }),
            runtime.clone(),
            Arc::new(FakeDownload),
        )
        .unwrap(),
    );
    fs::write(
        &state.profile_path,
        json!({"launcher_root":root}).to_string(),
    )
    .unwrap();
    let mut fixture = fixture;
    fixture.state = state;
    let (_, profile) = fixture
        .request(Method::GET, "/api/profile", Value::Null, None, None, None)
        .await;
    let (_, accepted) = fixture.write(Method::POST, "/api/apply", profile).await;
    assert_eq!(
        completed_job(&fixture, accepted["id"].as_str().unwrap()).await["status"],
        "succeeded"
    );
    let (_, accepted) = fixture
        .write(
            Method::POST,
            "/api/restore",
            json!({"backup_id":"a".repeat(32)}),
        )
        .await;
    assert_eq!(
        completed_job(&fixture, accepted["id"].as_str().unwrap()).await["status"],
        "succeeded"
    );
    assert_eq!(
        *runtime.events.lock().unwrap(),
        vec![
            "guard_enter",
            "before_apply",
            "on_applied",
            "guard_exit",
            "guard_enter",
            "before_restore",
            "after_restore",
            "guard_exit"
        ]
    );
}

struct RejectPublicationRuntime {
    failures: Mutex<Vec<String>>,
    reject_restore: bool,
}
impl RuntimeAdapter for RejectPublicationRuntime {
    fn process_cwds(&self) -> Vec<PathBuf> {
        vec![]
    }
    fn focus_status(&self) -> Value {
        json!({"mode":"observe-only"})
    }
    fn live_activation_enabled(&self) -> bool {
        true
    }
    fn on_applied(&self, _: &Profile) -> Result<(), String> {
        if self.reject_restore {
            Ok(())
        } else {
            Err("runtime Apply publication failed".into())
        }
    }
    fn after_restore(&self, _: Option<&Profile>) -> Result<(), String> {
        if self.reject_restore {
            Err("runtime Restore publication failed".into())
        } else {
            Ok(())
        }
    }
    fn mutation_failed(&self, error: &str) {
        self.failures.lock().unwrap().push(error.into());
    }
}

#[tokio::test]
async fn failed_runtime_apply_publication_keeps_marker_and_notifies_runtime() {
    use mcsr_toolkit::api::NativeBackend;
    use mcsr_toolkit::paths::state_dir;
    let mut fixture = Fixture::new(true, None);
    let data_home = fixture.temp.path().join("data");
    let runtime = Arc::new(RejectPublicationRuntime {
        failures: Mutex::new(vec![]),
        reject_restore: false,
    });
    fixture.state = Arc::new(
        ApiState::new(
            ApiConfig {
                profile_path: fixture.temp.path().join("profile.json"),
                managed_dir: fixture.temp.path().to_owned(),
                state_dir: state_dir(&data_home),
                port: 8765,
                keys: KeySet::fallback(),
            },
            Arc::new(NativeBackend {
                data_home,
                keyd: Arc::new(NativeKeyd::default()),
            }),
            runtime.clone(),
            Arc::new(FakeDownload),
        )
        .unwrap(),
    );
    let root = fixture
        .temp
        .path()
        .join("launcher")
        .to_string_lossy()
        .to_string();
    let (_, profile) = fixture
        .write(
            Method::PUT,
            "/api/profile",
            json!({"launcher_root":root,"crafting":{"s":"k"}}),
        )
        .await;
    let (_, accepted) = fixture.write(Method::POST, "/api/apply", profile).await;
    let failed = completed_job(&fixture, accepted["id"].as_str().unwrap()).await;
    assert_eq!(failed["status"], "failed");
    assert!(failed["error"]
        .as_str()
        .unwrap()
        .contains("runtime Apply publication failed"));
    assert!(fixture.state.state_dir.join("mutation.json").exists());
    assert_eq!(runtime.failures.lock().unwrap().len(), 1);
    assert_eq!(
        fs::read_to_string(&fixture.options).unwrap(),
        "key_key.back:key.keyboard.s\n"
    );
}

#[tokio::test]
async fn failed_runtime_restore_publication_keeps_marker_and_notifies_runtime() {
    use mcsr_toolkit::api::NativeBackend;
    use mcsr_toolkit::paths::state_dir;
    let mut fixture = Fixture::new(true, None);
    let data_home = fixture.temp.path().join("data");
    let runtime = Arc::new(RejectPublicationRuntime {
        failures: Mutex::new(vec![]),
        reject_restore: true,
    });
    fixture.state = Arc::new(
        ApiState::new(
            ApiConfig {
                profile_path: fixture.temp.path().join("profile.json"),
                managed_dir: fixture.temp.path().to_owned(),
                state_dir: state_dir(&data_home),
                port: 8765,
                keys: KeySet::fallback(),
            },
            Arc::new(NativeBackend {
                data_home,
                keyd: Arc::new(NativeKeyd::default()),
            }),
            runtime.clone(),
            Arc::new(FakeDownload),
        )
        .unwrap(),
    );
    let root = fixture
        .temp
        .path()
        .join("launcher")
        .to_string_lossy()
        .to_string();
    let (_, profile) = fixture
        .write(
            Method::PUT,
            "/api/profile",
            json!({"launcher_root":root,"crafting":{"s":"k"}}),
        )
        .await;
    let (_, accepted) = fixture.write(Method::POST, "/api/apply", profile).await;
    assert_eq!(
        completed_job(&fixture, accepted["id"].as_str().unwrap()).await["status"],
        "succeeded"
    );
    let (_, accepted) = fixture
        .write(
            Method::POST,
            "/api/restore",
            json!({"backup_id":completed_backup_id(&fixture)}),
        )
        .await;
    let failed = completed_job(&fixture, accepted["id"].as_str().unwrap()).await;
    assert_eq!(failed["status"], "failed");
    assert!(failed["error"]
        .as_str()
        .unwrap()
        .contains("runtime Restore publication failed"));
    assert!(fixture.state.state_dir.join("mutation.json").exists());
    assert_eq!(runtime.failures.lock().unwrap().len(), 1);
}

fn completed_backup_id(fixture: &Fixture) -> String {
    let state = fixture.state.state_dir.join("lineage.json");
    let record: Value = serde_json::from_slice(&fs::read(state).unwrap()).unwrap();
    record["head"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn native_windowed_preview_exposes_instance_details_without_keyboard_mapping() {
    struct NoKeyd;
    impl mcsr_toolkit::apply::KeydAdapter for NoKeyd {
        fn install(&self, _: &Profile) -> Result<(), String> {
            unreachable!()
        }
        fn checkpoint(&self) -> Result<Vec<String>, String> {
            Ok(vec![])
        }
        fn restore_to(&self, _: &[String]) -> Result<(), String> {
            unreachable!()
        }
        fn rollback(&self) -> Result<(), String> {
            unreachable!()
        }
    }
    let mut fixture = Fixture::new(false, None);
    fs::write(&fixture.options, b"fullscreen:true\n").unwrap();
    fixture.state = Arc::new(
        ApiState::new(
            ApiConfig {
                profile_path: fixture.temp.path().join("profile.json"),
                managed_dir: fixture.temp.path().to_owned(),
                state_dir: fixture.temp.path().join("state"),
                port: 8765,
                keys: KeySet::fallback(),
            },
            Arc::new(mcsr_toolkit::api::NativeBackend {
                data_home: fixture.temp.path().join("data"),
                keyd: Arc::new(NoKeyd),
            }),
            Arc::new(Runtime { live: false }),
            Arc::new(FakeDownload),
        )
        .unwrap(),
    );
    let (status, result) = fixture
        .write(
            Method::POST,
            "/api/preview",
            json!({
                "launcher_root": fixture.temp.path().join("launcher"),
                "presets":[{"id":"thin","width":300,"height":900,"shortcut":"b"}]
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["instances"][0]["status"], "ready");
    assert!(result["instances"][0]["detail"]
        .as_str()
        .unwrap()
        .contains("windowed"));
    assert!(result["mappings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|m| m["action"] != "fullscreen"));
    assert_eq!(fs::read(&fixture.options).unwrap(), b"fullscreen:true\n");
}
