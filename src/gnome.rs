use std::{
    collections::HashSet,
    fmt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::{mpsc, watch, Mutex};

use crate::{
    focus::{is_minecraft_class, FocusState, SharedFocus},
    maccel::{MaccelPeer, SensitivityController},
    profile::Profile,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GnomeError(pub String);
const FOCUS_TIMEOUT: Duration = Duration::from_secs(2);
const METHOD_TIMEOUT: Duration = Duration::from_secs(5);

async fn bounded<T>(
    duration: Duration,
    operation: impl std::future::Future<Output = Result<T, GnomeError>>,
) -> Result<T, GnomeError> {
    tokio::time::timeout(duration, operation)
        .await
        .map_err(|_| GnomeError("GNOME extension call timed out".into()))?
}
impl fmt::Display for GnomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for GnomeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResizeError {
    InvalidSize,
    NotFocused,
    Inexact {
        requested: (u32, u32),
        actual: (u32, u32),
    },
    Gnome(GnomeError),
}
impl fmt::Display for ResizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ResizeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetError(pub String);
impl fmt::Display for PresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for PresetError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResizeResult {
    pub preset_id: String,
    pub width: u32,
    pub height: u32,
    pub sensitivity: Option<String>,
    pub full_size: bool,
}

pub struct PresetController<P: GnomePeer, H: MaccelPeer> {
    bridge: Arc<GnomeBridge<P>>,
    sensitivity: Arc<SensitivityController<H>>,
    focus: SharedFocus,
    operation: Mutex<()>,
    generation: AtomicU64,
}
impl<P: GnomePeer, H: MaccelPeer> PresetController<P, H> {
    pub fn new(
        bridge: Arc<GnomeBridge<P>>,
        sensitivity: Arc<SensitivityController<H>>,
        focus: SharedFocus,
    ) -> Self {
        Self {
            bridge,
            sensitivity,
            focus,
            operation: Mutex::new(()),
            generation: AtomicU64::new(0),
        }
    }

    /// Invalidates an awaited preset when focus or runtime ownership changes.
    pub fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    fn fresh(&self, generation: u64) -> Result<(), PresetError> {
        if self.generation.load(Ordering::SeqCst) != generation
            || !self
                .focus
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .effective_at(Instant::now())
        {
            Err(PresetError(
                "preset focus or generation became stale".into(),
            ))
        } else {
            Ok(())
        }
    }

    pub async fn apply(
        &self,
        profile: &Profile,
        preset_id: &str,
        shifted: bool,
        focus: &FocusState,
    ) -> Result<ResizeResult, PresetError> {
        self.apply_with_fence(profile, preset_id, shifted, focus, |_| Ok(()))
            .await
    }

    pub(crate) async fn apply_with_fence(
        &self,
        profile: &Profile,
        preset_id: &str,
        shifted: bool,
        focus: &FocusState,
        publish: impl FnOnce(&ResizeResult) -> Result<(), PresetError>,
    ) -> Result<ResizeResult, PresetError> {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let _guard = self.operation.lock().await;
        let result = self
            .apply_inner(profile, preset_id, shifted, focus, generation)
            .await?;
        if let Err(error) = publish(&result) {
            return if self.sensitivity.needs_restore() {
                match self.sensitivity.restore().await {
                    Ok(()) => Err(error),
                    Err(recovery) => Err(PresetError(format!(
                        "{error}; sensitivity recovery failed: {recovery}"
                    ))),
                }
            } else {
                Err(error)
            };
        }
        Ok(result)
    }

    async fn apply_inner(
        &self,
        profile: &Profile,
        preset_id: &str,
        shifted: bool,
        focus: &FocusState,
        generation: u64,
    ) -> Result<ResizeResult, PresetError> {
        let preset = profile
            .presets
            .iter()
            .find(|p| p.id == preset_id)
            .ok_or_else(|| PresetError(format!("unknown preset: {preset_id}")))?;
        let size = match (u32::try_from(preset.width), u32::try_from(preset.height)) {
            (Ok(width), Ok(height))
                if (100..=16384).contains(&width) && (100..=16384).contains(&height) =>
            {
                Ok((width, height))
            }
            _ => Err(PresetError("invalid preset size".into())),
        };
        let was_focused = focus.effective_at(Instant::now());
        let resize_result = match size {
            Err(error) => Err(error),
            Ok((width, height)) if was_focused => match self.fresh(generation) {
                Ok(()) => {
                    let result = if preset.eye_zoom {
                        self.bridge
                            .resize_focused(width, height)
                            .await
                            .map(|(w, h)| (w, h, false))
                    } else {
                        self.bridge.toggle_resize_focused(width, height).await
                    };
                    result.map_err(|e| PresetError(e.to_string()))
                }
                Err(error) => Err(error),
            },
            Ok(_) => Err(PresetError("Minecraft is not focused".into())),
        };
        if !preset.eye_zoom {
            let restore_result = if self.sensitivity.needs_restore() {
                self.sensitivity
                    .restore()
                    .await
                    .map_err(|e| PresetError(e.to_string()))
            } else {
                Ok(())
            };
            return match (resize_result, restore_result) {
                (Ok(actual), Ok(())) => {
                    self.fresh(generation)?;
                    Ok(ResizeResult {
                        preset_id: preset.id.clone(),
                        width: actual.0,
                        height: actual.1,
                        sensitivity: None,
                        full_size: actual.2,
                    })
                }
                (Err(resize), Ok(())) => Err(resize),
                (Ok(_), Err(restore)) => Err(restore),
                (Err(resize), Err(restore)) => Err(PresetError(format!(
                    "{resize}; sensitivity recovery failed: {restore}"
                ))),
            };
        }
        let actual = resize_result?;
        self.fresh(generation)?;
        if shifted {
            return Ok(ResizeResult {
                preset_id: preset.id.clone(),
                width: actual.0,
                height: actual.1,
                sensitivity: None,
                full_size: false,
            });
        }
        let eye_result = self
            .sensitivity
            .eye()
            .await
            .map_err(|e| PresetError(e.to_string()));
        let freshness = self.fresh(generation);
        let sensitivity = match (eye_result, freshness) {
            (Ok(value), Ok(())) => value,
            (eye, fresh) => {
                let error = match (eye, fresh) {
                    (Err(eye), Err(stale)) => PresetError(format!("{eye}; {stale}")),
                    (Err(eye), _) => eye,
                    (_, Err(stale)) => stale,
                    (Ok(_), Ok(())) => unreachable!(),
                };
                return match self.sensitivity.restore().await {
                    Ok(()) => Err(error),
                    Err(restore) => Err(PresetError(format!(
                        "{error}; sensitivity recovery failed: {restore}"
                    ))),
                };
            }
        };
        Ok(ResizeResult {
            preset_id: preset.id.clone(),
            width: actual.0,
            height: actual.1,
            sensitivity: Some(sensitivity),
            full_size: false,
        })
    }
}

pub trait GnomePeer: Send + Sync {
    fn get_focus(
        &self,
    ) -> impl std::future::Future<Output = Result<(String, String, u32, u32, bool), GnomeError>> + Send;
    fn configure_presets(
        &self,
        json: String,
    ) -> impl std::future::Future<Output = Result<(), GnomeError>> + Send;
    fn prepare_routing(
        &self,
        _generation: String,
    ) -> impl std::future::Future<Output = Result<(), GnomeError>> + Send {
        async { Ok(()) }
    }
    fn resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> impl std::future::Future<Output = Result<(u32, u32), GnomeError>> + Send;
    fn toggle_resize_focused(
        &self,
        _width: u32,
        _height: u32,
    ) -> impl std::future::Future<Output = Result<(u32, u32, bool), GnomeError>> + Send {
        async {
            Err(GnomeError(
                "toggle resize unsupported; upgrade the MCSR GNOME extension".into(),
            ))
        }
    }
}

pub struct GnomeBridge<P: GnomePeer> {
    peer: Arc<P>,
    focus: SharedFocus,
    configured: Mutex<HashSet<String>>,
    configuring: Mutex<()>,
}
impl<P: GnomePeer> GnomeBridge<P> {
    pub fn new(peer: Arc<P>, focus: SharedFocus) -> Self {
        Self {
            peer,
            focus,
            configured: Mutex::new(HashSet::new()),
            configuring: Mutex::new(()),
        }
    }

    pub async fn configure_presets(&self, profile: &Profile) -> Result<(), GnomeError> {
        self.configure_inner(profile, None).await
    }
    pub async fn configure_presets_with_fence(
        &self,
        profile: &Profile,
        emit: impl Fn() -> Result<(), String> + Send + Sync,
    ) -> Result<(), GnomeError> {
        self.configure_inner(profile, Some(&emit)).await
    }
    async fn configure_inner(
        &self,
        profile: &Profile,
        fence: Option<&(dyn Fn() -> Result<(), String> + Send + Sync)>,
    ) -> Result<(), GnomeError> {
        let _configuration = self.configuring.lock().await;
        if profile.presets.len() > 8 {
            return Err(GnomeError("too many presets".into()));
        }
        let plan = crate::routing::RoutingPlan::compile(profile).map_err(GnomeError)?;
        let mut configured = Vec::new();
        let mut ids = HashSet::new();
        let mut shortcuts = HashSet::new();
        let mut eye = None;
        for (preset, signal) in profile.presets.iter().zip(&plan.signals) {
            if !(100..=16384).contains(&preset.width) || !(100..=16384).contains(&preset.height) {
                return Err(GnomeError("invalid preset size".into()));
            }
            let valid_id = preset.id.len() <= 32
                && preset.id.starts_with(|c: char| c.is_ascii_lowercase())
                && preset
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
            if !valid_id || !ids.insert(preset.id.as_str()) {
                return Err(GnomeError("invalid or duplicate preset id".into()));
            }
            let translated = accelerator(&preset.shortcut)?;
            let canonical = canonical_shortcut(&translated);
            if !shortcuts.insert(canonical.clone()) {
                return Err(GnomeError("duplicate GNOME preset shortcut".into()));
            }
            if preset.eye_zoom {
                if eye.is_some() || canonical.contains("<Shift>") {
                    return Err(GnomeError("invalid eye zoom shortcut".into()));
                }
                eye = Some(canonical);
            }
            configured.push(PresetConfig {
                id: &preset.id,
                accelerator: signal.accelerator.clone(),
                eye_zoom: preset.eye_zoom,
            });
        }
        if eye.is_some_and(|eye| shortcuts.contains(&canonical_shortcut(&format!("<Shift>{eye}"))))
        {
            return Err(GnomeError(
                "shifted eye zoom conflicts with another preset".into(),
            ));
        }
        let activation = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let json = serde_json::to_string(&PresetConfiguration {
            protocol: "mcsr-keyd-v1",
            generation: &activation,
            profile_generation: &plan.generation,
            drain: fence.is_some(),
            presets: configured,
        })
        .map_err(|e| GnomeError(e.to_string()))?;
        // A failed or timed-out replacement must not keep accepting an old plan.
        self.configured.lock().await.clear();
        let configure = async {
            if let Some(emit) = fence {
                bounded(
                    METHOD_TIMEOUT,
                    self.peer.prepare_routing(activation.clone()),
                )
                .await?;
                emit().map_err(GnomeError)?;
            }
            loop {
                match self.peer.configure_presets(json.clone()).await {
                    Err(error)
                        if fence.is_some() && error.0.contains("routing input drain pending") =>
                    {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    result => return result,
                }
            }
        };
        if let Err(error) = bounded(METHOD_TIMEOUT, configure).await {
            if fence.is_some() {
                let _ = bounded(METHOD_TIMEOUT, self.peer.prepare_routing(String::new())).await;
            }
            return Err(error);
        }
        *self.configured.lock().await = profile
            .presets
            .iter()
            .map(|preset| format!("{}:{}", activation, preset.id))
            .collect();
        Ok(())
    }

    pub async fn preset_signal_id(&self, id: &str) -> Result<String, GnomeError> {
        self.configured
            .lock()
            .await
            .iter()
            .find(|wire| wire.split_once(':').is_some_and(|(_, value)| value == id))
            .cloned()
            .ok_or_else(|| GnomeError("preset routing is not configured".into()))
    }

    pub async fn accept_preset_signal(&self, id: &str, _shifted: bool) -> Result<bool, GnomeError> {
        if !self.configured.lock().await.contains(id) {
            return Ok(false);
        }
        if !self.effective_focus() {
            return Ok(false);
        }
        let (class, _, _, _, modal) = bounded(FOCUS_TIMEOUT, self.peer.get_focus()).await?;
        Ok(!modal
            && is_minecraft_class(&class)
            && self.effective_focus()
            && self.configured.lock().await.contains(id))
    }

    fn effective_focus(&self) -> bool {
        let focus = self
            .focus
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        focus.effective_at(Instant::now())
    }

    pub async fn toggle_resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(u32, u32, bool), ResizeError> {
        if !(100..=16384).contains(&width) || !(100..=16384).contains(&height) {
            return Err(ResizeError::InvalidSize);
        }
        let (class, _, _, _, modal) = bounded(FOCUS_TIMEOUT, self.peer.get_focus())
            .await
            .map_err(ResizeError::Gnome)?;
        if modal || !is_minecraft_class(&class) {
            return Err(ResizeError::NotFocused);
        }
        let (actual_width, actual_height, full_size) = bounded(
            METHOD_TIMEOUT,
            self.peer.toggle_resize_focused(width, height),
        )
        .await
        .map_err(ResizeError::Gnome)?;
        if !(100..=16384).contains(&actual_width) || !(100..=16384).contains(&actual_height) {
            return Err(ResizeError::InvalidSize);
        }
        if !full_size && (actual_width, actual_height) != (width, height) {
            return Err(ResizeError::Inexact {
                requested: (width, height),
                actual: (actual_width, actual_height),
            });
        }
        Ok((actual_width, actual_height, full_size))
    }

    pub async fn resize_focused(&self, width: u32, height: u32) -> Result<(u32, u32), ResizeError> {
        if !(100..=16384).contains(&width) || !(100..=16384).contains(&height) {
            return Err(ResizeError::InvalidSize);
        }
        let (class, _, _, _, modal) = bounded(FOCUS_TIMEOUT, self.peer.get_focus())
            .await
            .map_err(ResizeError::Gnome)?;
        if modal || !is_minecraft_class(&class) {
            return Err(ResizeError::NotFocused);
        }
        let actual = bounded(METHOD_TIMEOUT, self.peer.resize_focused(width, height))
            .await
            .map_err(ResizeError::Gnome)?;
        if actual != (width, height) {
            return Err(ResizeError::Inexact {
                requested: (width, height),
                actual,
            });
        }
        Ok(actual)
    }
}

#[derive(Serialize)]
struct PresetConfiguration<'a> {
    protocol: &'static str,
    generation: &'a str,
    profile_generation: &'a str,
    drain: bool,
    presets: Vec<PresetConfig<'a>>,
}

#[derive(Serialize)]
struct PresetConfig<'a> {
    id: &'a str,
    accelerator: String,
    eye_zoom: bool,
}

fn accelerator(shortcut: &str) -> Result<String, GnomeError> {
    let mut parts = shortcut.split('+').peekable();
    let mut result = String::new();
    while let Some(part) = parts.next() {
        if parts.peek().is_some() {
            let modifier = match part.trim_start_matches("left").trim_start_matches("right") {
                "control" => "Control",
                "shift" => "Shift",
                "alt" => "Alt",
                "meta" => "Super",
                _ => return Err(GnomeError(format!("unsupported GNOME modifier: {part}"))),
            };
            result.push_str(&format!("<{modifier}>"));
        } else {
            let symbol = match part {
                "enter" => "Return",
                "esc" => "Escape",
                "dot" => "period",
                "pageup" => "Page_Up",
                "pagedown" => "Page_Down",
                "backspace" => "BackSpace",
                "delete" => "Delete",
                "insert" => "Insert",
                "home" => "Home",
                "end" => "End",
                "up" => "Up",
                "down" => "Down",
                "left" => "Left",
                "right" => "Right",
                "leftbrace" => "bracketleft",
                "rightbrace" => "bracketright",
                "slash" => "slash",
                "space" => "space",
                "tab" => "Tab",
                "minus" => "minus",
                "equal" => "equal",
                "comma" => "comma",
                "semicolon" => "semicolon",
                "apostrophe" => "apostrophe",
                "backslash" => "backslash",
                "grave" => "grave",
                "capslock" => "Caps_Lock",
                "numlock" => "Num_Lock",
                "scrolllock" => "Scroll_Lock",
                "sysrq" => "Print",
                "pause" => "Pause",
                "menu" => "Menu",
                "kp0" => "KP_0",
                "kp1" => "KP_1",
                "kp2" => "KP_2",
                "kp3" => "KP_3",
                "kp4" => "KP_4",
                "kp5" => "KP_5",
                "kp6" => "KP_6",
                "kp7" => "KP_7",
                "kp8" => "KP_8",
                "kp9" => "KP_9",
                "kpplus" => "KP_Add",
                "kpminus" => "KP_Subtract",
                "kpasterisk" => "KP_Multiply",
                "kpslash" => "KP_Divide",
                "kpdot" => "KP_Decimal",
                "kpenter" => "KP_Enter",
                "kpequal" => "KP_Equal",
                _ if part.len() == 1
                    && part
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) =>
                {
                    part
                }
                _ if is_function_key(part) => {
                    result.push_str(&part.to_ascii_uppercase());
                    continue;
                }
                _ => {
                    return Err(GnomeError(format!(
                        "unsupported GNOME shortcut key: {part}"
                    )))
                }
            };
            result.push_str(symbol);
        }
    }
    Ok(result)
}

fn is_function_key(part: &str) -> bool {
    part.strip_prefix('f').is_some_and(|number| {
        !number.starts_with('0')
            && number
                .parse::<u8>()
                .ok()
                .is_some_and(|number| (1..=24).contains(&number))
    })
}

fn canonical_shortcut(value: &str) -> String {
    let mut modifiers = Vec::new();
    let mut key = value;
    while let Some(rest) = key.strip_prefix('<') {
        if let Some((modifier, tail)) = rest.split_once('>') {
            modifiers.push(modifier);
            key = tail;
        } else {
            break;
        }
    }
    modifiers.sort_unstable();
    format!(
        "{}{}",
        modifiers
            .into_iter()
            .map(|modifier| format!("<{modifier}>"))
            .collect::<String>(),
        key
    )
}

const DESTINATION: &str = "org.gnome.Shell";
const OBJECT_PATH: &str = "/org/mcsr/Toolkit";
const INTERFACE: &str = "org.mcsr.Toolkit";

pub struct SessionPeer {
    connection: zbus::Connection,
}
impl SessionPeer {
    pub async fn connect() -> Result<Self, GnomeError> {
        Ok(Self {
            connection: bounded(METHOD_TIMEOUT, async {
                zbus::Connection::session().await.map_err(bus_error)
            })
            .await?,
        })
    }

    async fn proxy(&self) -> Result<zbus::Proxy<'_>, GnomeError> {
        bounded(METHOD_TIMEOUT, async {
            zbus::Proxy::new(&self.connection, DESTINATION, OBJECT_PATH, INTERFACE)
                .await
                .map_err(bus_error)
        })
        .await
    }
}

impl GnomePeer for SessionPeer {
    async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
        bounded(FOCUS_TIMEOUT, async {
            self.proxy()
                .await?
                .call("GetFocus", &())
                .await
                .map_err(bus_error)
        })
        .await
    }
    async fn prepare_routing(&self, generation: String) -> Result<(), GnomeError> {
        bounded(METHOD_TIMEOUT, async {
            self.proxy()
                .await?
                .call("PrepareRouting", &(generation,))
                .await
                .map_err(bus_error)
        })
        .await
    }
    async fn configure_presets(&self, json: String) -> Result<(), GnomeError> {
        bounded(METHOD_TIMEOUT, async {
            self.proxy()
                .await?
                .call("ConfigurePresets", &(json,))
                .await
                .map_err(bus_error)
        })
        .await
    }
    async fn toggle_resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(u32, u32, bool), GnomeError> {
        bounded(METHOD_TIMEOUT, async {
            self.proxy().await?.call("ToggleResizeFocused", &(width, height)).await.map_err(|error| {
                if matches!(&error, zbus::Error::MethodError(name, _, _) if name.as_str() == "org.freedesktop.DBus.Error.UnknownMethod") {
                    GnomeError("toggle resize unsupported; upgrade the MCSR GNOME extension".into())
                } else { bus_error(error) }
            })
        }).await
    }
    async fn resize_focused(&self, width: u32, height: u32) -> Result<(u32, u32), GnomeError> {
        bounded(METHOD_TIMEOUT, async {
            self.proxy()
                .await?
                .call("ResizeFocused", &(width, height))
                .await
                .map_err(bus_error)
        })
        .await
    }
}

impl GnomeBridge<SessionPeer> {
    pub async fn listen_presets(
        &self,
        mut cancel: watch::Receiver<bool>,
        sender: mpsc::Sender<(String, bool)>,
    ) -> Result<(), GnomeError> {
        if *cancel.borrow() {
            return Ok(());
        }
        let proxy = tokio::select! {
            biased;
            _ = cancel.changed() => return Ok(()),
            proxy = self.peer.proxy() => proxy?,
        };
        let mut signals = tokio::select! {
            biased;
            _ = cancel.changed() => return Ok(()),
            signals = bounded(METHOD_TIMEOUT, async { proxy.receive_signal("PresetActivated").await.map_err(bus_error) }) => signals?,
        };
        loop {
            tokio::select! {
                biased;
                change = cancel.changed() => { if change.is_err() || *cancel.borrow() { return Ok(()); } }
                signal = signals.next() => {
                    let signal = signal.ok_or_else(|| GnomeError("preset signal stream ended".into()))?;
                    let (id, shifted): (String, bool) = signal.body().deserialize().map_err(bus_error)?;
                    let accepted = tokio::select! {
                        biased;
                        change = cancel.changed() => { if change.is_err() || *cancel.borrow() { return Ok(()); } else { continue; } }
                        accepted = self.accept_preset_signal(&id, shifted) => accepted?,
                    };
                    if accepted {
                        tokio::select! {
                            biased;
                            change = cancel.changed() => { if change.is_err() || *cancel.borrow() { return Ok(()); } }
                            sent = sender.send((id, shifted)) => { if sent.is_err() { return Ok(()); } }
                        }
                    }
                }
            }
        }
    }
}

fn bus_error(error: impl fmt::Display) -> GnomeError {
    GnomeError(error.to_string())
}

#[cfg(test)]
mod publication_tests {
    use super::*;
    use crate::{focus::FocusEvent, maccel::SensitivityError, profile::ResizePreset};
    use std::sync::atomic::AtomicBool;
    use tokio::sync::Notify;

    struct Peer;
    impl GnomePeer for Peer {
        async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
            Ok(("Minecraft".into(), "Game".into(), 800, 600, false))
        }
        async fn configure_presets(&self, _: String) -> Result<(), GnomeError> {
            Ok(())
        }
        async fn resize_focused(&self, w: u32, h: u32) -> Result<(u32, u32), GnomeError> {
            Ok((w, h))
        }
    }
    #[derive(Default)]
    struct Helper {
        block_restore: AtomicBool,
        block_eye: AtomicBool,
        eye_started: Notify,
        eye_release: Notify,
        restoring: Notify,
        release: Notify,
        calls: std::sync::Mutex<Vec<&'static str>>,
    }
    impl MaccelPeer for Helper {
        async fn preflight(&self) -> Result<(), SensitivityError> {
            Ok(())
        }
        async fn eye(&self, _: &str) -> Result<String, SensitivityError> {
            self.calls.lock().unwrap().push("eye");
            if self.block_eye.swap(false, Ordering::SeqCst) {
                self.eye_started.notify_one();
                self.eye_release.notified().await;
            }
            Ok("0.02083000005222856998443603515625".into())
        }
        async fn restore(&self) -> Result<String, SensitivityError> {
            self.calls.lock().unwrap().push("restore");
            if self.block_restore.swap(false, Ordering::SeqCst) {
                self.restoring.notify_one();
                self.release.notified().await;
            }
            Ok("a".repeat(64))
        }
    }
    #[tokio::test]
    async fn rejected_profile_publication_recovers_before_newer_eye_write() {
        let focus = Arc::new(std::sync::Mutex::new(FocusState::default()));
        focus.lock().unwrap().observe(FocusEvent {
            window_class: "Minecraft".into(),
            title: "Game".into(),
            modal: false,
        });
        let helper = Arc::new(Helper::default());
        let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
        sensitivity.startup_restore().await.unwrap();
        let controller = Arc::new(PresetController::new(
            Arc::new(GnomeBridge::new(Arc::new(Peer), focus.clone())),
            sensitivity.clone(),
            focus.clone(),
        ));
        let mut profile = Profile::default();
        profile.presets.push(ResizePreset {
            id: "eye".into(),
            width: 400,
            height: 300,
            shortcut: "f7".into(),
            eye_zoom: true,
        });
        let snapshot = focus.lock().unwrap().clone();
        helper.block_restore.store(true, Ordering::SeqCst);
        helper.block_eye.store(true, Ordering::SeqCst);
        let profile_generation = Arc::new(AtomicU64::new(0));
        let first = {
            let controller = controller.clone();
            let profile = profile.clone();
            let snapshot = snapshot.clone();
            let profile_generation = profile_generation.clone();
            tokio::spawn(async move {
                controller
                    .apply_with_fence(&profile, "eye", false, &snapshot, |_| {
                        if profile_generation.load(Ordering::SeqCst) != 0 {
                            Err(PresetError("profile changed during preset apply".into()))
                        } else {
                            Ok(())
                        }
                    })
                    .await
            })
        };
        tokio::time::timeout(Duration::from_secs(2), helper.eye_started.notified())
            .await
            .unwrap();
        profile_generation.store(1, Ordering::SeqCst);
        helper.eye_release.notify_one();
        tokio::time::timeout(Duration::from_secs(2), helper.restoring.notified())
            .await
            .unwrap();
        let second = {
            let controller = controller.clone();
            let profile = profile.clone();
            let snapshot = snapshot.clone();
            tokio::spawn(async move { controller.apply(&profile, "eye", false, &snapshot).await })
        };
        tokio::task::yield_now().await;
        assert_eq!(*helper.calls.lock().unwrap(), ["restore", "eye", "restore"]);
        helper.release.notify_one();
        assert!(first.await.unwrap().is_err());
        assert!(second.await.unwrap().is_ok());
        assert_eq!(
            *helper.calls.lock().unwrap(),
            ["restore", "eye", "restore", "eye"]
        );
        assert!(sensitivity.needs_restore());
    }
}
