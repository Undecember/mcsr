//! Owns the process lock and orders service startup and cleanup.

use serde_json::{json, Value};
use std::{
    fmt,
    fs::{self, File, OpenOptions},
    net::Ipv4Addr,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};

use crate::{
    api::{self, ApiConfig, ApiState, DownloadAdapter, NativeBackend, RuntimeAdapter},
    apply::{ApplyCoordinator, ApplyError},
    downloads::{self, Product, ReqwestClient},
    focus::{FocusListener, FocusState, SharedFocus},
    gnome::{GnomeBridge, GnomePeer, PresetController, ResizeResult, SessionPeer},
    instances::locate_launcher_root,
    keyd::{CommandRunner, KeydController, SystemCommandRunner},
    maccel::{InstalledMaccelPeer, MaccelPeer, SensitivityController},
    paths::{atomic_replace, ensure_state, remove_record, safe_record},
    profile::{parse_profile, KeySet, Profile},
};

#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub runtime_dir: PathBuf,
    pub data_home: PathBuf,
    pub managed_dir: PathBuf,
    pub port: u16,
    pub enable_live_keyd: bool,
    pub maccel_binary: PathBuf,
}

#[derive(Debug)]
pub struct RuntimeError(pub String);
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for RuntimeError {}
fn err(error: impl fmt::Display) -> RuntimeError {
    RuntimeError(error.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaccelAvailability {
    Available,
    Unavailable,
    RecoveryUncertain,
}

fn optional_maccel_availability(
    binary: &Path,
    device: &Path,
    helper: &Path,
    recovery: &Path,
) -> MaccelAvailability {
    fn present_or_unknown(path: &Path) -> bool {
        match fs::symlink_metadata(path) {
            Ok(_) => true,
            Err(error) => error.kind() != std::io::ErrorKind::NotFound,
        }
    }
    if present_or_unknown(binary) && present_or_unknown(device) && present_or_unknown(helper) {
        MaccelAvailability::Available
    } else if present_or_unknown(recovery) {
        MaccelAvailability::RecoveryUncertain
    } else {
        MaccelAvailability::Unavailable
    }
}

fn process_cwds_under(proc_root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(proc_root) else {
        return vec![];
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .as_encoded_bytes()
                .iter()
                .all(u8::is_ascii_digit)
        })
        .filter_map(|entry| entry.path().join("cwd").canonicalize().ok())
        .collect()
}

struct ServiceLock {
    _file: File,
}
impl ServiceLock {
    fn acquire(path: &Path) -> Result<Self, RuntimeError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)
            .map_err(err)?;
        let opened = file.metadata().map_err(err)?;
        let named = fs::symlink_metadata(path).map_err(err)?;
        if !opened.is_file()
            || opened.uid() != unsafe { libc::geteuid() }
            || opened.permissions().mode() & 0o077 != 0
            || opened.ino() != named.ino()
            || opened.dev() != named.dev()
        {
            return Err(RuntimeError("unsafe service lock".into()));
        }
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(RuntimeError("service is already running".into()));
        }
        Ok(Self { _file: file })
    }
}

struct State {
    focus: SharedFocus,
    state_dir: PathBuf,
    live: bool,
    maccel_available: bool,
    maccel_recovery_pending: bool,
    sensitivity_recovery: Box<dyn Fn() -> bool + Send + Sync>,
    keyd: Arc<KeydController>,
    keys: KeySet,
    keyd_available: bool,
    active_profile: Mutex<Option<Profile>>,
    configured_profile: Mutex<Option<Profile>>,
    routing_configuration: tokio::sync::Mutex<()>,
    active_preset: Mutex<Option<(u64, String)>>,
    mutation: Mutex<()>,
    error: Mutex<Option<String>>,
    errors: Mutex<Vec<String>>,
    reconciliation: Mutex<Option<Value>>,
    accepting_presets: AtomicBool,
}
impl State {
    fn record_error(&self, error: impl ToString) {
        let error = error.to_string();
        let mut errors = self.errors.lock().unwrap();
        if errors.last() != Some(&error) {
            errors.push(error);
            if errors.len() > 5 {
                errors.remove(0);
            }
        }
    }
}
impl RuntimeAdapter for State {
    fn process_cwds(&self) -> Vec<PathBuf> {
        process_cwds_under(Path::new("/proc"))
    }
    fn focus_status(&self) -> Value {
        let focus = self.focus.lock().unwrap();
        let focused = focus.effective_at(Instant::now());
        let connected = focus.connected_at(Instant::now());
        let pending = self.state_dir.join("mutation.json").exists();
        let recovery = self.maccel_recovery_pending || (self.sensitivity_recovery)();
        let usable = self.maccel_available && !recovery;
        let detail = if self.maccel_recovery_pending {
            "maccel recovery unconfirmed"
        } else if !self.maccel_available {
            "maccel is not installed"
        } else if recovery {
            "sensitivity recovery pending"
        } else {
            "eye zoom available"
        };
        let (activate, clear) = self.keyd.event_counts();
        let active_preset = visible_active_preset(&focus, &self.active_preset);
        json!({"mode": if self.live && self.active_profile.lock().unwrap().is_some() { "live" } else { "observe-only" }, "minecraft_focused":focused, "extension_connected":connected, "mutation_recovery_pending":pending, "cleanup_error":self.error.lock().unwrap().clone(), "activation_suspended":pending || recovery, "recovery_clear_pending":pending, "unsafe_stop_failure":false, "active_preset":active_preset, "fifo_errors":[], "errors":self.errors.lock().unwrap().clone(), "reconciliation":self.reconciliation.lock().unwrap().clone(), "keyd_available":self.keyd_available, "events":{"activate":activate,"clear":clear}, "maccel":{"available":self.maccel_available,"usable":usable,"detail":detail,"recovery_pending":recovery}})
    }
    fn live_activation_enabled(&self) -> bool {
        self.live
            && !self.maccel_recovery_pending
            && !(self.sensitivity_recovery)()
            && self.error.lock().unwrap().is_none()
    }
    fn with_mutation_guard(
        &self,
        operation: &mut dyn FnMut() -> Result<Value, String>,
    ) -> Result<Value, String> {
        let _guard = self.mutation.lock().unwrap();
        operation()
    }
    fn before_apply(&self, _profile: &Profile) -> Result<(), String> {
        if self.state_dir.join("mutation.json").exists() {
            return Err("mutation recovery pending".into());
        }
        Ok(())
    }
    fn on_applied(&self, profile: &Profile) -> Result<(), String> {
        if self.configured_profile.lock().unwrap().as_ref() != Some(profile) {
            *self.configured_profile.lock().unwrap() = None;
        }
        let bytes = serde_json::to_vec(profile).map_err(|e| e.to_string())?;
        atomic_replace(&self.state_dir.join("active-profile.json"), &bytes, true)
            .map_err(|e| e.to_string())?;
        *self.active_profile.lock().unwrap() = Some(profile.clone());
        *self.active_preset.lock().unwrap() = None;
        Ok(())
    }
    fn before_restore(&self) -> Result<(), String> {
        if self.live {
            self.keyd.clear().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    fn after_restore(&self, profile: Option<&Profile>) -> Result<(), String> {
        if self.configured_profile.lock().unwrap().as_ref() != profile {
            *self.configured_profile.lock().unwrap() = None;
        }
        if let Some(profile) = profile {
            let bytes = serde_json::to_vec(profile).map_err(|e| e.to_string())?;
            atomic_replace(&self.state_dir.join("active-profile.json"), &bytes, true)
                .map_err(|e| e.to_string())?;
        } else {
            remove_record(&self.state_dir.join("active-profile.json"))
                .map_err(|e| e.to_string())?;
        }
        *self.active_profile.lock().unwrap() = profile.cloned();
        *self.active_preset.lock().unwrap() = None;
        Ok(())
    }
    fn mutation_failed(&self, error: &str) {
        *self.error.lock().unwrap() = Some(error.to_owned());
    }
}

struct NativeDownloads;
impl DownloadAdapter for NativeDownloads {
    fn download(&self, product: Product, destination: &Path) -> Result<Value, String> {
        let client = ReqwestClient::new().map_err(|e| e.to_string())?;
        let asset = downloads::resolve_release(product, &client).map_err(|e| e.to_string())?;
        let result =
            downloads::download_release(&asset, destination, &client).map_err(|e| e.to_string())?;
        Ok(json!({"version":result.version,"path":result.path,"byte_count":result.byte_count}))
    }
}

pub struct ServiceRuntime<P: GnomePeer = SessionPeer, H: MaccelPeer = InstalledMaccelPeer> {
    lock: Option<ServiceLock>,
    config: RuntimeConfig,
    state: Arc<State>,
    api: Arc<ApiState>,
    bridge: Arc<GnomeBridge<P>>,
    preset: Arc<PresetController<P, H>>,
    sensitivity: Arc<SensitivityController<H>>,
    sensitivity_available: bool,
    cancel: watch::Sender<bool>,
    effective: watch::Sender<bool>,
    tasks: Vec<JoinHandle<Result<(), RuntimeError>>>,
    api_task: Option<JoinHandle<Result<(), RuntimeError>>>,
    closed: bool,
}
impl<P: GnomePeer, H: MaccelPeer> Drop for ServiceRuntime<P, H> {
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            self.state.accepting_presets.store(false, Ordering::SeqCst);
            self.preset.invalidate();
            self.cancel.send_replace(true);
            self.effective.send_replace(false);
            for task in self.tasks.drain(..) {
                task.abort();
            }
            if let Some(task) = self.api_task.take() {
                task.abort();
            }
            // A process that skipped verified async cleanup must not admit a replacement.
            // The kernel closes this descriptor at process exit; the next process then recovers.
            std::mem::forget(lock);
        }
    }
}
impl ServiceRuntime<SessionPeer, InstalledMaccelPeer> {
    pub async fn start(config: RuntimeConfig) -> Result<Self, RuntimeError> {
        let availability = optional_maccel_availability(
            &config.maccel_binary,
            Path::new("/sys/module/maccel/parameters/SENS_MULT"),
            Path::new("/usr/local/libexec/mcsr-toolkit"),
            Path::new("/run/mcsr-toolkit-maccel"),
        );
        if availability == MaccelAvailability::RecoveryUncertain && config.enable_live_keyd {
            return Err(RuntimeError(
                "maccel recovery cannot be verified while the device or helper is absent".into(),
            ));
        }
        let peer = Arc::new(SessionPeer::connect().await.map_err(err)?);
        let maccel = Arc::new(InstalledMaccelPeer::new(config.maccel_binary.clone()));
        let mut runtime = Self::start_with_adapters_inner(
            config,
            peer,
            maccel,
            Arc::new(NativeDownloads),
            availability,
            Arc::new(SystemCommandRunner),
        )
        .await?;
        runtime.start_signal_listener();
        Ok(runtime)
    }
    fn start_signal_listener(&mut self) {
        let bridge = self.bridge.clone();
        let preset = self.preset.clone();
        let sensitivity = self.sensitivity.clone();
        let state = self.state.clone();
        let api = self.api.clone();
        let effective = self.effective.clone();
        let mut cancel = self.cancel.subscribe();
        self.tasks.push(tokio::spawn(async move {
            loop {
                if *cancel.borrow() { return Ok(()); }
                let initial: Result<Profile, RuntimeError> = Ok(state.active_profile.lock().unwrap().clone().unwrap_or_default());
                if let Ok(mut configured_profile) = initial {
                    if let Err(error) = configure_routes(&state, &bridge, &configured_profile).await {
                        state.record_error(error);
                    } else {
                    let (sender, mut receiver) = mpsc::channel(16);
                    let listen = bridge.listen_presets(cancel.clone(), sender);
                    tokio::pin!(listen);
                    let mut configure_tick = tokio::time::interval(std::time::Duration::from_secs(1));
                    loop {
                        tokio::select! {
                            changed = cancel.changed() => { if changed.is_err() || *cancel.borrow() { return Ok(()); } },
                            _ = configure_tick.tick() => {
                                let active: Result<Profile, RuntimeError> = Ok(state.active_profile.lock().unwrap().clone().unwrap_or_default());
                                match active {
                                    Ok(profile) if profile != configured_profile || state.configured_profile.lock().unwrap().as_ref() != Some(&profile) => {
                                        match configure_routes(&state, &bridge, &profile).await {
                                            Ok(()) => configured_profile = profile,
                                            Err(error) => { state.record_error(error); break; }
                                        }
                                    },
                                    Ok(_) => {},
                                    Err(error) => state.record_error(error),
                                }
                            },
                            signal = receiver.recv() => {
                                if let Some((id, shifted)) = signal {
                                    if let Err(error) = apply_signal(&api, &state, &bridge, &preset, &id, shifted, false).await {
                                        state.record_error(&error);
                                        if sensitivity.needs_restore() { *state.error.lock().unwrap() = Some(error.to_string()); }
                                    }
                                }
                            },
                            result = &mut listen => { if let Err(error) = result { state.record_error(error); } break; },
                        }
                    }
                    }
                } else if let Err(error) = initial {
                    state.record_error(error);
                }
                preset.invalidate();
                *state.configured_profile.lock().unwrap() = None;
                if state.live { state.keyd.clear().map_err(err)?; }
                let _ = bridge.configure_presets(&Profile::default()).await;
                *state.active_preset.lock().unwrap() = None;
                state.focus.lock().unwrap().disconnect();
                effective.send_replace(false);
                if sensitivity.needs_restore() { sensitivity.restore().await.map_err(err)?; }
                tokio::select! { _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}, _ = cancel.changed() => { if *cancel.borrow() { return Ok(()); } } }
            }
        }));
    }
}
impl<P: GnomePeer + 'static, H: MaccelPeer + 'static> ServiceRuntime<P, H> {
    pub async fn start_with_peers(
        config: RuntimeConfig,
        gnome: Arc<P>,
        maccel: Arc<H>,
    ) -> Result<Self, RuntimeError> {
        Self::start_with_adapters(config, gnome, maccel, Arc::new(NativeDownloads)).await
    }
    pub async fn start_with_adapters(
        config: RuntimeConfig,
        gnome: Arc<P>,
        maccel: Arc<H>,
        downloads: Arc<dyn DownloadAdapter>,
    ) -> Result<Self, RuntimeError> {
        Self::start_with_adapters_inner(
            config,
            gnome,
            maccel,
            downloads,
            MaccelAvailability::Available,
            Arc::new(SystemCommandRunner),
        )
        .await
    }
    pub async fn start_without_maccel(
        config: RuntimeConfig,
        gnome: Arc<P>,
        maccel: Arc<H>,
    ) -> Result<Self, RuntimeError> {
        Self::start_with_adapters_inner(
            config,
            gnome,
            maccel,
            Arc::new(NativeDownloads),
            MaccelAvailability::Unavailable,
            Arc::new(SystemCommandRunner),
        )
        .await
    }
    pub async fn start_with_runner(
        config: RuntimeConfig,
        gnome: Arc<P>,
        maccel: Arc<H>,
        runner: Arc<dyn CommandRunner>,
    ) -> Result<Self, RuntimeError> {
        Self::start_with_adapters_inner(
            config,
            gnome,
            maccel,
            Arc::new(NativeDownloads),
            MaccelAvailability::Available,
            runner,
        )
        .await
    }
    async fn start_with_adapters_inner(
        config: RuntimeConfig,
        gnome: Arc<P>,
        maccel: Arc<H>,
        downloads: Arc<dyn DownloadAdapter>,
        availability: MaccelAvailability,
        runner: Arc<dyn CommandRunner>,
    ) -> Result<Self, RuntimeError> {
        fs::create_dir_all(&config.runtime_dir).map_err(err)?;
        let runtime_meta = fs::symlink_metadata(&config.runtime_dir).map_err(err)?;
        if !runtime_meta.is_dir()
            || runtime_meta.file_type().is_symlink()
            || runtime_meta.uid() != unsafe { libc::geteuid() }
            || runtime_meta.permissions().mode() & 0o077 != 0
        {
            return Err(RuntimeError("unsafe runtime directory".into()));
        }
        let lock = ServiceLock::acquire(&config.runtime_dir.join("mcsr-toolkit-service.lock"))?;
        let state_dir = ensure_state(&config.data_home).map_err(err)?;
        let mut pending_marker = None;
        for name in ["mutation.json", "keyd-uncertain.json"] {
            match safe_record(&state_dir.join(name)) {
                Ok(_) => pending_marker = Some(name),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(err(e)),
            }
        }
        // The managed service may have been inactive before installation. Bootstrap
        // through the existing fixed clear helper before asking its client for IPC.
        if config.enable_live_keyd {
            runner.verify_helper().map_err(err)?;
            runner
                .run(
                    &[
                        "/usr/bin/sudo".into(),
                        "-n".into(),
                        crate::install::HELPER_PATH.into(),
                        "privileged".into(),
                        "keyd".into(),
                        "clear".into(),
                    ],
                    &[],
                )
                .map_err(err)?;
        }
        let (keys, keyd_available) = discover_keys(runner.as_ref());
        let keyd = Arc::new(KeydController::with_keys(
            &state_dir,
            runner,
            config.enable_live_keyd,
            keys.clone(),
        ));
        let active_profile = match safe_record(&state_dir.join("active-profile.json")) {
            Ok(bytes) => Some(parse_profile(&bytes, &keys).map_err(err)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(err(e)),
        };
        if active_profile.is_some() {
            keyd.clear().map_err(err)?;
        }
        if let Some(name) = pending_marker {
            return Err(RuntimeError(format!("{name} requires manual recovery")));
        }
        if config.enable_live_keyd {
            keyd.preflight_clear().map_err(err)?;
        }
        let sensitivity_available = availability == MaccelAvailability::Available;
        let sensitivity = if sensitivity_available {
            Arc::new(SensitivityController::new(maccel))
        } else {
            Arc::new(SensitivityController::unavailable(maccel))
        };
        if sensitivity_available {
            sensitivity.startup_restore().await.map_err(err)?;
        }
        let focus = Arc::new(Mutex::new(FocusState::default()));
        let listener = FocusListener::open(
            &config.runtime_dir.join("mcsr-toolkit-focus.fifo"),
            focus.clone(),
        )
        .map_err(err)?;
        let state = Arc::new(State {
            focus: focus.clone(),
            state_dir: state_dir.clone(),
            live: config.enable_live_keyd,
            maccel_available: sensitivity_available,
            maccel_recovery_pending: availability == MaccelAvailability::RecoveryUncertain,
            sensitivity_recovery: Box::new({
                let sensitivity = sensitivity.clone();
                move || sensitivity.recovery_pending()
            }),
            keyd: keyd.clone(),
            keys: keys.clone(),
            keyd_available,
            active_profile: Mutex::new(active_profile),
            configured_profile: Mutex::new(None),
            routing_configuration: tokio::sync::Mutex::new(()),
            active_preset: Mutex::new(None),
            mutation: Mutex::new(()),
            error: Mutex::new(None),
            errors: Mutex::new(Vec::new()),
            reconciliation: Mutex::new(None),
            accepting_presets: AtomicBool::new(true),
        });
        let bridge = Arc::new(GnomeBridge::new(gnome, focus.clone()));
        let initial_profile = state
            .active_profile
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default();
        if state.live {
            configure_routes(&state, &bridge, &initial_profile).await?;
        } else {
            bridge
                .configure_presets(&initial_profile)
                .await
                .map_err(err)?;
        }
        *state.configured_profile.lock().unwrap() = Some(initial_profile);
        let preset = Arc::new(PresetController::new(
            bridge.clone(),
            sensitivity.clone(),
            focus,
        ));
        let api = Arc::new(
            ApiState::new(
                ApiConfig {
                    profile_path: state_dir.join("profile.json"),
                    managed_dir: config.managed_dir.clone(),
                    state_dir: state_dir.clone(),
                    port: config.port,
                    keys,
                },
                Arc::new(NativeBackend {
                    data_home: config.data_home.clone(),
                    keyd,
                }),
                state.clone(),
                downloads,
            )
            .map_err(err)?,
        );
        let address = api::ApiState::bind_address(config.port).map_err(err)?;
        let tcp = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, address.port()))
            .await
            .map_err(err)?;
        let (cancel, _) = watch::channel(false);
        let (effective, mut effective_rx) = watch::channel(false);
        let mut tasks = Vec::new();
        tasks.push(tokio::spawn({
            let cancel = cancel.subscribe();
            let effective = effective.clone();
            async move { listener.run(cancel, effective).await.map_err(err) }
        }));
        let api_task = tokio::spawn({
            let api = api.clone();
            let cancel = cancel.subscribe();
            async move {
                axum::serve(tcp, api::router(api))
                    .with_graceful_shutdown(async move {
                        let mut cancel = cancel;
                        while !*cancel.borrow() {
                            if cancel.changed().await.is_err() {
                                break;
                            }
                        }
                    })
                    .await
                    .map_err(err)
            }
        });
        tasks.push(tokio::spawn({
            let state = state.clone();
            let preset = preset.clone();
            let sensitivity = sensitivity.clone();
            let mut cancel = cancel.subscribe();
            let config = config.clone();
            async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
                let mut reconcile_tick = tokio::time::interval(std::time::Duration::from_secs(2));
                loop {
                    tokio::select! {
                        changed = cancel.changed() => {
                            if changed.is_err() || *cancel.borrow() { break; }
                        },
                        changed = effective_rx.changed() => {
                            if changed.is_err() { break; }
                            if !*effective_rx.borrow() {
                                preset.invalidate();
                                *state.active_preset.lock().unwrap() = None;
                                if sensitivity.needs_restore() {
                                    sensitivity.restore().await.map_err(err)?;
                                }
                                if state.live {
                                    state.keyd.clear().map_err(err)?;
                                }
                            }
                        },
                        _ = tick.tick(), if state.live => {
                            if !state.state_dir.join("mutation.json").exists() {
                                let state = state.clone();
                                tokio::task::spawn_blocking(move || {
                                    let _mutation = state.mutation.lock().unwrap();
                                    let unsafe_state = state.error.lock().unwrap().is_some()
                                        || state.state_dir.join("mutation.json").exists();
                                    if !unsafe_state {
                                        if let Some(profile) = state.active_profile.lock().unwrap().clone() {
                                        if state.configured_profile.lock().unwrap().as_ref() == Some(&profile) {
                                            let focus = state.focus.lock().unwrap().clone();
                                            state.keyd.sync_focus(&profile, &focus).map_err(err)?;
                                        }
                                        }
                                    }
                                    Ok::<(),RuntimeError>(())
                                }).await.map_err(err)??;
                            }
                        },
                        _ = reconcile_tick.tick(), if state.live => {
                            let state = state.clone();
                            let config = config.clone();
                            tokio::task::spawn_blocking(move || reconcile_cycle(&state, &config)).await.map_err(err)??;
                        }
                    }
                }
                Ok(())
            }
        }));
        Ok(Self {
            lock: Some(lock),
            config,
            state,
            api,
            bridge,
            preset,
            sensitivity,
            sensitivity_available,
            cancel,
            effective,
            tasks,
            api_task: Some(api_task),
            closed: false,
        })
    }
    pub fn focus_status(&self) -> Value {
        self.state.focus_status()
    }
    pub fn attach_preset_signals(&mut self, mut receiver: mpsc::Receiver<(String, bool)>) {
        if self.closed || *self.cancel.borrow() {
            return;
        }
        let mut cancel = self.cancel.subscribe();
        let api = self.api.clone();
        let state = self.state.clone();
        let bridge = self.bridge.clone();
        let preset = self.preset.clone();
        self.tasks.push(tokio::spawn(async move {
            loop {
                tokio::select! {
                    changed = cancel.changed() => {
                        if changed.is_err() || *cancel.borrow() { return Ok(()); }
                    },
                    signal = receiver.recv() => {
                        let Some((id, shifted)) = signal else { return Ok(()); };
                        let _ = apply_signal(&api, &state, &bridge, &preset, &id, shifted, true).await;
                    }
                }
            }
        }));
    }
    pub async fn handle_preset(
        &self,
        id: &str,
        shifted: bool,
    ) -> Result<ResizeResult, RuntimeError> {
        if self.closed || self.state.state_dir.join("mutation.json").exists() {
            return Err(RuntimeError("preset acceptance blocked".into()));
        }
        apply_signal(
            &self.api,
            &self.state,
            &self.bridge,
            &self.preset,
            id,
            shifted,
            true,
        )
        .await
    }
    pub async fn run(&mut self) -> Result<(), RuntimeError> {
        #[cfg(unix)]
        {
            let mut term =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .map_err(err)?;
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(100));
            tokio::select! { _ = term.recv() => {}, result = tokio::signal::ctrl_c() => { result.map_err(err)?; }, _ = async { loop { tick.tick().await; if self.tasks.iter().any(JoinHandle::is_finished) || self.api_task.as_ref().is_some_and(JoinHandle::is_finished) { break; } } } => {} }
        }
        self.shutdown().await
    }
    pub async fn shutdown(&mut self) -> Result<(), RuntimeError> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.state.accepting_presets.store(false, Ordering::SeqCst);
        self.preset.invalidate();
        *self.state.active_preset.lock().unwrap() = None;
        self.cancel.send_replace(true);
        self.effective.send_replace(false);
        let api = self.api.clone();
        let mut errors = Vec::new();
        if let Err(error) = tokio::task::spawn_blocking(move || api.drain_jobs()).await {
            errors.push(error.to_string());
        }
        for task in self.tasks.drain(..) {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => errors.push(error.to_string()),
                Err(error) => errors.push(error.to_string()),
            }
        }
        if self.sensitivity_available {
            if let Err(error) = self.sensitivity.restore().await {
                errors.push(error.to_string());
            }
        }
        if self.config.enable_live_keyd {
            if let Err(error) = self.state.keyd.clear() {
                errors.push(error.to_string());
            }
        }
        if let Some(mut api_task) = self.api_task.take() {
            match tokio::time::timeout(std::time::Duration::from_secs(5), &mut api_task).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(error))) => errors.push(error.to_string()),
                Ok(Err(error)) => errors.push(error.to_string()),
                Err(_) => {
                    api_task.abort();
                    errors.push("HTTP shutdown timed out".into());
                }
            }
        }
        if errors.is_empty() {
            self.lock.take();
            Ok(())
        } else {
            Err(RuntimeError(errors.join("; ")))
        }
    }
}

fn load_profile(api: &ApiState) -> Result<Profile, RuntimeError> {
    match safe_record(&api.profile_path) {
        Ok(bytes) => parse_profile(&bytes, &api.keys).map_err(err),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Profile::default()),
        Err(e) => Err(err(e)),
    }
}

fn discover_keys(runner: &dyn CommandRunner) -> (KeySet, bool) {
    let Ok(binary) = runner.keyd_binary() else {
        return (KeySet::fallback(), false);
    };
    let args = [binary.to_string(), "list-keys".into()];
    let keys = runner
        .run(&args, &[])
        .map(|output| KeySet::with_discovered(output.stdout.lines()))
        .unwrap_or_else(|_| KeySet::fallback());
    (keys, true)
}

fn reconciliation_unsafe<'a>(
    keyd_error: Option<&str>,
    statuses: impl IntoIterator<Item = &'a str>,
) -> bool {
    keyd_error.is_some()
        || statuses
            .into_iter()
            .any(|status| !matches!(status, "updated" | "unchanged" | "deferred"))
}

fn reconcile_cycle(state: &State, config: &RuntimeConfig) -> Result<(), RuntimeError> {
    let _mutation = state.mutation.lock().unwrap();
    if state.state_dir.join("mutation.json").exists() || state.error.lock().unwrap().is_some() {
        return Ok(());
    }
    let profile = match safe_record(&state.state_dir.join("pending.json")) {
        Ok(bytes) => parse_profile(&bytes, &state.keys).map_err(err)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let Some(profile) = state.active_profile.lock().unwrap().clone() else {
                return Ok(());
            };
            profile
        }
        Err(error) => return Err(err(error)),
    };
    let processes = state.process_cwds();
    let Some(root) = locate_launcher_root(
        &config.managed_dir,
        profile.launcher_root.as_deref().map(Path::new),
        &processes,
    ) else {
        let error = "launcher root unavailable during reconciliation".to_owned();
        *state.reconciliation.lock().unwrap() = Some(json!({"error":error}));
        *state.error.lock().unwrap() = Some(error.clone());
        state.record_error(&error);
        state.keyd.clear().map_err(err)?;
        return Ok(());
    };
    let result = ApplyCoordinator::new(&root, &config.data_home, processes)
        .map_err(err)?
        .reconcile_new_instances_with_publish(&profile, state.keyd.as_ref(), &|profile| {
            state.on_applied(profile).map_err(ApplyError)
        });
    match result {
        Ok(report) => {
            let unsafe_report = reconciliation_unsafe(
                report.keyd_error.as_deref(),
                report.instances.iter().map(|item| item.status.as_str()),
            );
            *state.reconciliation.lock().unwrap() = Some(
                json!({"instances":report.instances.iter().map(|item| json!({"instance_id":item.instance_id,"status":item.status,"detail":item.detail})).collect::<Vec<_>>(),"backup_id":report.backup_id,"keyd_error":report.keyd_error}),
            );
            if unsafe_report {
                let error = "reconciliation requires input recovery".to_owned();
                *state.error.lock().unwrap() = Some(error.clone());
                state.record_error(&error);
                state.keyd.clear().map_err(err)?;
            }
            Ok(())
        }
        Err(error) => {
            let message = error.to_string();
            *state.reconciliation.lock().unwrap() = Some(json!({"error":message}));
            *state.error.lock().unwrap() = Some(message.clone());
            state.record_error(message);
            state.keyd.clear().map_err(err)
        }
    }
}

async fn configure_routes<P: GnomePeer>(
    state: &State,
    bridge: &GnomeBridge<P>,
    profile: &Profile,
) -> Result<(), RuntimeError> {
    let _routing = state.routing_configuration.lock().await;
    if !state.live {
        bridge
            .configure_presets(&Profile::default())
            .await
            .map_err(err)?;
        *state.configured_profile.lock().unwrap() = Some(profile.clone());
        return Ok(());
    }
    {
        let _mutation = state.mutation.lock().unwrap();
        *state.configured_profile.lock().unwrap() = None;
        if state.live {
            state.keyd.clear().map_err(err)?;
        }
    }
    bridge
        .configure_presets_with_fence(profile, || {
            state.keyd.routing_fence().map_err(|e| e.to_string())
        })
        .await
        .map_err(err)?;
    let _mutation = state.mutation.lock().unwrap();
    if state
        .active_profile
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default()
        != *profile
    {
        return Err(err("applied profile changed during shortcut configuration"));
    }
    *state.configured_profile.lock().unwrap() = Some(profile.clone());
    Ok(())
}

async fn apply_signal<P: GnomePeer, H: MaccelPeer>(
    api: &ApiState,
    state: &State,
    bridge: &GnomeBridge<P>,
    preset: &PresetController<P, H>,
    id: &str,
    shifted: bool,
    configure: bool,
) -> Result<ResizeResult, RuntimeError> {
    if !state.accepting_presets.load(Ordering::SeqCst) {
        return Err(RuntimeError("preset acceptance stopped".into()));
    }
    let focus_generation = state.focus.lock().unwrap().generation();
    if state.state_dir.join("mutation.json").exists() {
        return Err(RuntimeError("preset acceptance blocked".into()));
    }
    let applied = state.active_profile.lock().unwrap().clone();
    let profile = match applied {
        Some(profile) => profile,
        None if configure && !state.live => load_profile(api)?,
        None => return Err(err("no applied profile for preset routing")),
    };
    if !state.maccel_available
        && !shifted
        && profile
            .presets
            .iter()
            .any(|item| item.id == id && item.eye_zoom)
    {
        return Err(RuntimeError("maccel unavailable".into()));
    }
    if configure {
        if state.live {
            configure_routes(state, bridge, &profile).await?;
        } else {
            bridge.configure_presets(&profile).await.map_err(err)?;
        }
    }
    let wire_id = if configure {
        bridge.preset_signal_id(id).await.map_err(err)?
    } else {
        id.to_owned()
    };
    if !bridge
        .accept_preset_signal(&wire_id, shifted)
        .await
        .map_err(err)?
    {
        return Err(RuntimeError("stale preset signal".into()));
    }
    if !state.accepting_presets.load(Ordering::SeqCst) {
        return Err(RuntimeError("preset acceptance stopped".into()));
    }
    let focus = state.focus.lock().unwrap().clone();
    let preset_id = wire_id
        .split_once(':')
        .map(|(_, id)| id)
        .ok_or_else(|| err("invalid preset signal"))?;
    let result = preset
        .apply(&profile, preset_id, shifted, &focus)
        .await
        .map_err(err)?;
    if !publish_active_preset(
        &state.focus,
        &state.active_preset,
        focus_generation,
        &result.preset_id,
    ) {
        return Err(RuntimeError("preset focus changed during apply".into()));
    }
    Ok(result)
}

fn publish_active_preset(
    focus: &SharedFocus,
    active: &Mutex<Option<(u64, String)>>,
    generation: u64,
    id: &str,
) -> bool {
    let focus = focus.lock().unwrap();
    if focus.generation() != generation || !focus.effective_at(Instant::now()) {
        return false;
    }
    *active.lock().unwrap() = Some((generation, id.to_owned()));
    true
}

fn visible_active_preset(
    focus: &FocusState,
    active: &Mutex<Option<(u64, String)>>,
) -> Option<String> {
    if !focus.effective_at(Instant::now()) {
        return None;
    }
    active
        .lock()
        .unwrap()
        .as_ref()
        .filter(|(generation, _)| *generation == focus.generation())
        .map(|(_, id)| id.clone())
}

#[cfg(test)]
mod tests {
    #[test]
    fn blur_and_quick_refocus_cannot_publish_stale_preset() {
        use std::sync::{Arc, Mutex};
        let focus = Arc::new(Mutex::new(crate::focus::FocusState::default()));
        let active = Mutex::new(None);
        let now = std::time::Instant::now();
        let event = |class: &str| crate::focus::FocusEvent {
            window_class: class.into(),
            title: "Game".into(),
            modal: false,
        };
        focus.lock().unwrap().observe_at(event("Minecraft"), now);
        let generation = focus.lock().unwrap().generation();
        focus.lock().unwrap().observe_at(event("Terminal"), now);
        focus.lock().unwrap().observe_at(event("Minecraft"), now);
        assert!(!super::publish_active_preset(
            &focus, &active, generation, "eye"
        ));
        assert!(active.lock().unwrap().is_none());
        let new_generation = focus.lock().unwrap().generation();
        assert!(super::publish_active_preset(
            &focus,
            &active,
            new_generation,
            "normal"
        ));
        focus.lock().unwrap().observe_at(event("Terminal"), now);
        focus.lock().unwrap().observe_at(event("Minecraft"), now);
        assert!(super::visible_active_preset(&focus.lock().unwrap(), &active).is_none());
    }
    #[test]
    fn deferred_reconciliation_is_retryable_but_failed_status_requires_suspension() {
        assert!(!super::reconciliation_unsafe(None, ["deferred"]));
        assert!(super::reconciliation_unsafe(None, ["failed"]));
        assert!(super::reconciliation_unsafe(
            Some("keyd failed"),
            ["unchanged"]
        ));
    }
    #[test]
    fn missing_optional_device_is_unavailable_unless_recovery_root_exists() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join("maccel");
        let device = temp.path().join("device");
        let helper = temp.path().join("helper");
        let recovery = temp.path().join("recovery");
        assert_eq!(
            super::optional_maccel_availability(&binary, &device, &helper, &recovery),
            super::MaccelAvailability::Unavailable
        );
        std::fs::create_dir(&recovery).unwrap();
        assert_eq!(
            super::optional_maccel_availability(&binary, &device, &helper, &recovery),
            super::MaccelAvailability::RecoveryUncertain
        );
        std::fs::remove_dir(&recovery).unwrap();
        std::fs::write(&binary, b"fake").unwrap();
        std::fs::write(&device, b"1").unwrap();
        assert_eq!(
            super::optional_maccel_availability(&binary, &device, &helper, &recovery),
            super::MaccelAvailability::Unavailable
        );
        std::fs::write(&helper, b"fake").unwrap();
        assert_eq!(
            super::optional_maccel_availability(&binary, &device, &helper, &recovery),
            super::MaccelAvailability::Available
        );
    }

    #[test]
    fn process_cwd_discovery_ignores_non_process_entries_and_missing_links() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("launcher");
        std::fs::create_dir(&target).unwrap();
        std::fs::create_dir(temp.path().join("123")).unwrap();
        std::fs::create_dir(temp.path().join("abc")).unwrap();
        std::fs::create_dir(temp.path().join("456")).unwrap();
        std::os::unix::fs::symlink(&target, temp.path().join("123/cwd")).unwrap();
        std::os::unix::fs::symlink("/nonexistent", temp.path().join("456/cwd")).unwrap();
        assert_eq!(super::process_cwds_under(temp.path()), [target]);
    }
}
