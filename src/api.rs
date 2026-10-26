//! Local HTTP API with one serialized background job.
use crate::apply::{ApplyCoordinator, ApplyError, ApplyPreview, ApplyReport, KeydAdapter};
use crate::assets::web_asset;
use crate::downloads::Product;
use crate::instances::{discover_instances, locate_launcher_root};
use crate::paths::atomic_replace;
use crate::profile::{parse_profile, KeySet, Profile};
use axum::body::Body;
use axum::extract::{Path as UrlPath, State};
use axum::http::{header, Method, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use uuid::Uuid;

type ApiResult = Result<Json<Value>, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    detail: Value,
}
impl ApiError {
    fn code(status: StatusCode, code: &str, message: Option<&str>) -> Self {
        let detail = if let Some(message) = message {
            json!({"code":code,"message":message})
        } else {
            json!({"code":code})
        };
        Self { status, detail }
    }
    fn validation(message: impl ToString) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            detail: json!([{"msg":message.to_string()}]),
        }
    }
    fn internal(message: impl ToString) -> Self {
        Self::code(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            Some(&message.to_string()),
        )
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"detail":self.detail}))).into_response()
    }
}

pub trait ApiBackend: Send + Sync + 'static {
    fn preview(
        &self,
        root: &Path,
        profile: &Profile,
        process_cwds: Vec<PathBuf>,
    ) -> Result<ApplyPreview, String>;
    fn apply(
        &self,
        root: &Path,
        profile: &Profile,
        process_cwds: Vec<PathBuf>,
        publish: &dyn Fn(&Profile) -> Result<(), String>,
    ) -> Result<ApplyReport, String>;
    fn restore(
        &self,
        root: &Path,
        backup_id: &str,
        process_cwds: Vec<PathBuf>,
        before: &dyn Fn() -> Result<(), String>,
        publish: &dyn Fn(Option<&Profile>) -> Result<(), String>,
    ) -> Result<ApplyReport, String>;
}

pub struct NativeBackend {
    pub data_home: PathBuf,
    pub keyd: Arc<dyn KeydAdapter + Send + Sync>,
}
impl ApiBackend for NativeBackend {
    fn preview(
        &self,
        root: &Path,
        profile: &Profile,
        process_cwds: Vec<PathBuf>,
    ) -> Result<ApplyPreview, String> {
        ApplyCoordinator::new(root, &self.data_home, process_cwds)
            .map_err(|e| e.to_string())?
            .preview(profile)
            .map_err(|e| e.to_string())
    }
    fn apply(
        &self,
        root: &Path,
        profile: &Profile,
        process_cwds: Vec<PathBuf>,
        publish: &dyn Fn(&Profile) -> Result<(), String>,
    ) -> Result<ApplyReport, String> {
        ApplyCoordinator::new(root, &self.data_home, process_cwds)
            .map_err(|e| e.to_string())?
            .apply_with_publish(profile, self.keyd.as_ref(), &|profile| {
                publish(profile).map_err(ApplyError)
            })
            .map_err(|e| e.to_string())
    }
    fn restore(
        &self,
        root: &Path,
        backup_id: &str,
        process_cwds: Vec<PathBuf>,
        before: &dyn Fn() -> Result<(), String>,
        publish: &dyn Fn(Option<&Profile>) -> Result<(), String>,
    ) -> Result<ApplyReport, String> {
        ApplyCoordinator::new(root, &self.data_home, process_cwds)
            .map_err(|e| e.to_string())?
            .restore_with_hooks(
                backup_id,
                self.keyd.as_ref(),
                &|| before().map_err(ApplyError),
                &|profile| publish(profile).map_err(ApplyError),
            )
            .map_err(|e| e.to_string())
    }
}

pub trait RuntimeAdapter: Send + Sync + 'static {
    fn process_cwds(&self) -> Vec<PathBuf>;
    fn focus_status(&self) -> Value;
    fn live_activation_enabled(&self) -> bool;
    fn with_mutation_guard(
        &self,
        operation: &mut dyn FnMut() -> Result<Value, String>,
    ) -> Result<Value, String> {
        operation()
    }
    fn before_apply(&self, _profile: &Profile) -> Result<(), String> {
        Ok(())
    }
    fn on_applied(&self, _profile: &Profile) -> Result<(), String> {
        Ok(())
    }
    fn before_restore(&self) -> Result<(), String> {
        Ok(())
    }
    fn after_restore(&self, _profile: Option<&Profile>) -> Result<(), String> {
        Ok(())
    }
    fn mutation_failed(&self, _error: &str) {}
}

pub trait DownloadAdapter: Send + Sync + 'static {
    fn download(&self, product: Product, destination: &Path) -> Result<Value, String>;
}

#[derive(Clone)]
struct Job {
    id: String,
    status: &'static str,
    result: Option<Value>,
    error: Option<String>,
}
impl Job {
    fn json(&self) -> Value {
        json!({"id":self.id,"status":self.status,"result":self.result,"error":self.error})
    }
}
#[derive(Default)]
struct QueueInner {
    active: Option<String>,
    jobs: VecDeque<Job>,
    closing: bool,
}
#[derive(Default)]
struct JobQueue {
    inner: Mutex<QueueInner>,
    idle: Condvar,
}
impl JobQueue {
    fn require_idle(&self) -> Result<(), ApiError> {
        let inner = self.inner.lock().unwrap();
        if inner.active.is_some() || inner.closing {
            Err(ApiError::code(
                StatusCode::CONFLICT,
                "job_busy",
                Some("another operation is running"),
            ))
        } else {
            Ok(())
        }
    }
    fn submit(
        self: &Arc<Self>,
        operation: impl FnOnce() -> Result<Value, String> + Send + 'static,
    ) -> Result<Value, ApiError> {
        let id = Uuid::new_v4().simple().to_string();
        {
            let mut inner = self.inner.lock().unwrap();
            if inner.active.is_some() || inner.closing {
                return Err(ApiError::code(
                    StatusCode::CONFLICT,
                    "job_busy",
                    Some("another operation is running"),
                ));
            }
            inner.active = Some(id.clone());
            inner.jobs.push_back(Job {
                id: id.clone(),
                status: "running",
                result: None,
                error: None,
            });
            if inner.jobs.len() > 128 {
                inner.jobs.pop_front();
            }
        }
        let queue = self.clone();
        let accepted = json!({"id":id,"status":"running"});
        std::thread::spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
            let mut inner = queue.inner.lock().unwrap();
            if let Some(job) = inner.jobs.iter_mut().find(|job| job.id == id) {
                match outcome {
                    Ok(Ok(result)) => {
                        job.status = "succeeded";
                        job.result = Some(result);
                    }
                    Ok(Err(error)) => {
                        job.status = "failed";
                        job.error = Some(error);
                    }
                    Err(_) => {
                        job.status = "failed";
                        job.error = Some("operation panicked".into());
                    }
                }
            }
            inner.active = None;
            queue.idle.notify_all();
        });
        Ok(accepted)
    }
    fn get(&self, id: &str) -> Result<Value, ApiError> {
        self.inner
            .lock()
            .unwrap()
            .jobs
            .iter()
            .find(|job| job.id == id)
            .map(Job::json)
            .ok_or_else(|| ApiError::code(StatusCode::NOT_FOUND, "unknown_job", None))
    }
    fn drain(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.closing = true;
        while inner.active.is_some() {
            inner = self.idle.wait(inner).unwrap();
        }
    }
}

pub struct ApiConfig {
    pub profile_path: PathBuf,
    pub managed_dir: PathBuf,
    pub state_dir: PathBuf,
    pub port: u16,
    pub keys: KeySet,
}

pub struct ApiState {
    pub profile_path: PathBuf,
    pub managed_dir: PathBuf,
    pub state_dir: PathBuf,
    pub port: u16,
    pub session_token: String,
    pub keys: KeySet,
    pub backend: Arc<dyn ApiBackend>,
    pub runtime: Arc<dyn RuntimeAdapter>,
    pub downloads: Arc<dyn DownloadAdapter>,
    profile_lock: Mutex<()>,
    jobs: Arc<JobQueue>,
}
impl ApiState {
    pub fn new(
        config: ApiConfig,
        backend: Arc<dyn ApiBackend>,
        runtime: Arc<dyn RuntimeAdapter>,
        downloads: Arc<dyn DownloadAdapter>,
    ) -> Result<Self, String> {
        Self::bind_address(config.port)?;
        Ok(Self {
            profile_path: config.profile_path,
            managed_dir: config.managed_dir,
            state_dir: config.state_dir,
            port: config.port,
            session_token: Uuid::new_v4().to_string(),
            keys: config.keys,
            backend,
            runtime,
            downloads,
            profile_lock: Mutex::new(()),
            jobs: Arc::new(JobQueue::default()),
        })
    }
    pub fn bind_address(port: u16) -> Result<SocketAddr, String> {
        if port == 0 {
            return Err("invalid local service port".into());
        }
        Ok(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
    }
    pub fn drain_jobs(&self) {
        self.jobs.drain();
    }
    fn load_profile(&self) -> Result<Profile, ApiError> {
        match fs::symlink_metadata(&self.profile_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Profile::default()),
            Err(e) => Err(ApiError::internal(e)),
            Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                Err(ApiError::internal("unsafe profile path"))
            }
            Ok(_) => parse_profile(
                &fs::read(&self.profile_path).map_err(ApiError::internal)?,
                &self.keys,
            )
            .map_err(ApiError::internal),
        }
    }
    fn save_profile(&self, profile: &Profile) -> Result<(), ApiError> {
        let bytes = serde_json::to_vec_pretty(profile).map_err(ApiError::internal)?;
        let parent = self
            .profile_path
            .parent()
            .ok_or_else(|| ApiError::internal("profile path has no parent"))?;
        fs::create_dir_all(parent).map_err(ApiError::internal)?;
        atomic_replace(&self.profile_path, &bytes, true).map_err(ApiError::internal)
    }
    fn root(&self, profile: &Profile) -> Result<PathBuf, ApiError> {
        let processes = self.runtime.process_cwds();
        let root = locate_launcher_root(
            &self.managed_dir,
            profile.launcher_root.as_deref().map(Path::new),
            &processes,
        )
        .ok_or_else(|| {
            ApiError::code(
                StatusCode::UNPROCESSABLE_ENTITY,
                "launcher_root",
                Some("MCSRLauncher root not found"),
            )
        })?;
        let instances = discover_instances(&root).map_err(ApiError::internal)?;
        if instances.is_empty() {
            return Err(ApiError::code(
                StatusCode::UNPROCESSABLE_ENTITY,
                "launcher_root",
                Some("no safe MCSRLauncher instances"),
            ));
        }
        Ok(root)
    }
    fn parse(&self, bytes: &[u8]) -> Result<Profile, ApiError> {
        parse_profile(bytes, &self.keys).map_err(|e| ApiError::validation(e.to_string()))
    }
    fn mutation_pending(&self) -> bool {
        self.state_dir
            .join("mutation.json")
            .symlink_metadata()
            .is_ok()
    }
}

async fn gate(State(state): State<Arc<ApiState>>, request: Request<Body>, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let permitted =
        host == format!("127.0.0.1:{}", state.port) || host == format!("localhost:{}", state.port);
    if !permitted {
        return ApiError::code(StatusCode::FORBIDDEN, "bad_host", None).into_response();
    }
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let valid_origin = origin == format!("http://{host}");
    if !origin.is_empty() && !valid_origin {
        return ApiError::code(StatusCode::FORBIDDEN, "bad_origin", None).into_response();
    }
    if !matches!(
        request.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    ) {
        let token = request
            .headers()
            .get("x-mcsr-session")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !valid_origin || !constant_time_eq(token.as_bytes(), state.session_token.as_bytes()) {
            return ApiError::code(StatusCode::FORBIDDEN, "bad_session", None).into_response();
        }
    }
    next.run(request).await
}
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0_u8;
    for (x, y) in a.iter().zip(b) {
        result |= x ^ y;
    }
    result == 0
}

async fn session(State(state): State<Arc<ApiState>>) -> ApiResult {
    Ok(Json(json!({"token":state.session_token})))
}
async fn status(State(state): State<Arc<ApiState>>) -> ApiResult {
    let profile = state.load_profile()?;
    let root = locate_launcher_root(
        &state.managed_dir,
        profile.launcher_root.as_deref().map(Path::new),
        &state.runtime.process_cwds(),
    );
    let instances = if let Some(root) = &root {
        discover_instances(root).map_err(ApiError::internal)?.into_iter()
        .map(|item| json!({"id":item.id,"path":item.path,"options_path":item.options_path,"problem":item.problem})).collect::<Vec<_>>()
    } else {
        vec![]
    };
    Ok(Json(
        json!({"live_activation_enabled":state.runtime.live_activation_enabled() && !state.mutation_pending(),
        "launcher_root":root,"instances":instances,"focus":state.runtime.focus_status()}),
    ))
}
async fn get_profile(State(state): State<Arc<ApiState>>) -> ApiResult {
    Ok(Json(json!(state.load_profile()?)))
}
async fn put_profile(State(state): State<Arc<ApiState>>, body: axum::body::Bytes) -> ApiResult {
    let profile = state.parse(&body)?;
    if profile.launcher_root.is_some() {
        state.root(&profile)?;
    }
    let _guard = state.profile_lock.lock().unwrap();
    state.jobs.require_idle()?;
    state.save_profile(&profile)?;
    Ok(Json(json!(profile)))
}
async fn preview(State(state): State<Arc<ApiState>>, body: axum::body::Bytes) -> ApiResult {
    let profile = state.parse(&body)?;
    let root = state.root(&profile)?;
    let result = state
        .backend
        .preview(&root, &profile, state.runtime.process_cwds())
        .map_err(ApiError::internal)?;
    Ok(Json(preview_json(result)))
}
fn preview_json(result: ApplyPreview) -> Value {
    json!({"instances":result.instances.into_iter().map(|item| json!({"instance_id":item.instance_id,"status":item.status,"detail":item.detail})).collect::<Vec<_>>(),
        "mappings":result.mappings.into_iter().map(|item| json!({"instance_id":item.instance_id,"physical":item.physical,"emitted":item.emitted,"action":item.action})).collect::<Vec<_>>()})
}
fn report_json(report: ApplyReport) -> Value {
    json!({"instances":report.instances.into_iter().map(|item| json!({"instance_id":item.instance_id,"status":item.status,"detail":item.detail})).collect::<Vec<_>>(),
        "backup_id":report.backup_id,"keyd_error":report.keyd_error,"restored_profile":report.restored_profile})
}
async fn apply(
    State(state): State<Arc<ApiState>>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let expected = state.parse(&body)?;
    if !state.runtime.live_activation_enabled() || state.mutation_pending() {
        return Err(ApiError::code(
            StatusCode::FORBIDDEN,
            "safety_gate",
            Some("live keyd activation is disabled"),
        ));
    }
    let _guard = state.profile_lock.lock().unwrap();
    let profile = state.load_profile()?;
    if profile != expected {
        return Err(ApiError::code(
            StatusCode::CONFLICT,
            "stale_profile",
            Some("profile changed; save and preview again"),
        ));
    }
    let root = state.root(&profile)?;
    let backend = state.backend.clone();
    let runtime = state.runtime.clone();
    let state_for_job = state.clone();
    let accepted = state.jobs.submit(move || {
        runtime.with_mutation_guard(&mut || {
            let outcome = (|| {
                runtime.before_apply(&profile)?;
                let publish = |profile: &Profile| runtime.on_applied(profile);
                backend.apply(&root, &profile, runtime.process_cwds(), &publish)
            })();
            if state_for_job.mutation_pending() {
                let error = match &outcome {
                    Ok(report) => report
                        .keyd_error
                        .clone()
                        .or_else(|| {
                            report
                                .instances
                                .iter()
                                .find(|item| item.status == "failed")
                                .map(|item| item.detail.clone())
                        })
                        .unwrap_or_else(|| "apply requires manual recovery".into()),
                    Err(error) => error.clone(),
                };
                runtime.mutation_failed(&error);
            }
            outcome.map(report_json)
        })
    })?;
    Ok((StatusCode::ACCEPTED, Json(accepted)))
}
async fn restore(
    State(state): State<Arc<ApiState>>,
    body: axum::body::Bytes,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let request: Value = serde_json::from_slice(&body).map_err(ApiError::validation)?;
    let object = request
        .as_object()
        .ok_or_else(|| ApiError::validation("expected object"))?;
    let backup_id = object
        .get("backup_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::validation("backup_id is required"))?;
    if object.len() != 1 {
        return Err(ApiError::validation("extra fields are not permitted"));
    }
    let _guard = state.profile_lock.lock().unwrap();
    let profile = state.load_profile()?;
    let root = state.root(&profile)?;
    let backend = state.backend.clone();
    let runtime = state.runtime.clone();
    let state_for_job = state.clone();
    let backup_id = backup_id.to_owned();
    let accepted = state.jobs.submit(move || {
        runtime.with_mutation_guard(&mut || {
            let before = || runtime.before_restore();
            let publish = |profile: Option<&Profile>| {
                let _guard = state_for_job.profile_lock.lock().unwrap();
                if let Some(profile) = profile {
                    state_for_job
                        .save_profile(profile)
                        .map_err(|e| e.detail.to_string())?;
                }
                runtime.after_restore(profile)
            };
            let outcome =
                backend.restore(&root, &backup_id, runtime.process_cwds(), &before, &publish);
            if let Err(error) = &outcome {
                if state_for_job.mutation_pending() {
                    runtime.mutation_failed(error);
                }
            }
            outcome.map(report_json)
        })
    })?;
    Ok((StatusCode::ACCEPTED, Json(accepted)))
}
async fn download(
    State(state): State<Arc<ApiState>>,
    UrlPath(product): UrlPath<String>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let (product, name) = match product.as_str() {
        "launcher" => (Product::Launcher, "MCSRLauncher.jar"),
        "ninjabrain" => (Product::Ninjabrain, "Ninjabrain-Bot.jar"),
        _ => return Err(ApiError::validation("invalid product")),
    };
    let destination = state.managed_dir.join("downloads").join(name);
    let downloads = state.downloads.clone();
    let accepted = state
        .jobs
        .submit(move || downloads.download(product, &destination))?;
    Ok((StatusCode::ACCEPTED, Json(accepted)))
}
async fn job(State(state): State<Arc<ApiState>>, UrlPath(id): UrlPath<String>) -> ApiResult {
    Ok(Json(state.jobs.get(&id)?))
}
async fn asset(UrlPath(path): UrlPath<String>) -> Response {
    asset_at(&format!("/{path}"))
}
async fn index() -> Response {
    asset_at("/")
}
fn asset_at(path: &str) -> Response {
    match web_asset(path) {
        Some((bytes, mime)) => ([(header::CONTENT_TYPE, mime)], bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub fn router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/api/session", get(session))
        .route("/api/status", get(status))
        .route("/api/profile", get(get_profile).put(put_profile))
        .route("/api/preview", post(preview))
        .route("/api/apply", post(apply))
        .route("/api/restore", post(restore))
        .route("/api/download/{product}", post(download))
        .route("/api/jobs/{id}", get(job))
        .route("/", get(index))
        .route("/{path}", get(asset))
        .layer(middleware::from_fn_with_state(state.clone(), gate))
        .with_state(state)
}
