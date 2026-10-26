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
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let _guard = self.operation.lock().await;
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
                Ok(()) => self
                    .bridge
                    .resize_focused(width, height)
                    .await
                    .map_err(|e| PresetError(e.to_string())),
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
    fn resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> impl std::future::Future<Output = Result<(u32, u32), GnomeError>> + Send;
}

pub struct GnomeBridge<P: GnomePeer> {
    peer: Arc<P>,
    focus: SharedFocus,
    configured: Mutex<HashSet<String>>,
}
impl<P: GnomePeer> GnomeBridge<P> {
    pub fn new(peer: Arc<P>, focus: SharedFocus) -> Self {
        Self {
            peer,
            focus,
            configured: Mutex::new(HashSet::new()),
        }
    }

    pub async fn configure_presets(&self, profile: &Profile) -> Result<(), GnomeError> {
        if profile.presets.len() > 8 {
            return Err(GnomeError("too many presets".into()));
        }
        let mut configured = Vec::new();
        let mut ids = HashSet::new();
        let mut shortcuts = HashSet::new();
        let mut eye = None;
        for preset in &profile.presets {
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
                accelerator: translated,
                eye_zoom: preset.eye_zoom,
            });
        }
        if eye.is_some_and(|eye| shortcuts.contains(&canonical_shortcut(&format!("<Shift>{eye}"))))
        {
            return Err(GnomeError(
                "shifted eye zoom conflicts with another preset".into(),
            ));
        }
        let json = serde_json::to_string(&configured).map_err(|e| GnomeError(e.to_string()))?;
        bounded(METHOD_TIMEOUT, self.peer.configure_presets(json)).await?;
        *self.configured.lock().await = profile
            .presets
            .iter()
            .map(|preset| preset.id.clone())
            .collect();
        Ok(())
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
