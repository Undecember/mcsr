use indexmap::IndexMap;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileError(pub String);

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ProfileError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySet(HashSet<String>);

impl KeySet {
    pub fn fallback() -> Self {
        let mut keys: HashSet<String> = "abcdefghijklmnopqrstuvwxyz0123456789"
            .chars()
            .map(|c| c.to_string())
            .collect();
        keys.extend(
            [
                "slash",
                "enter",
                "esc",
                "tab",
                "space",
                "leftalt",
                "rightalt",
                "leftshift",
                "rightshift",
                "leftcontrol",
                "rightcontrol",
                "leftmeta",
                "rightmeta",
                "apostrophe",
                "backslash",
                "backspace",
                "comma",
                "delete",
                "dot",
                "end",
                "equal",
                "grave",
                "home",
                "insert",
                "leftbrace",
                "minus",
                "pause",
                "rightbrace",
                "semicolon",
                "up",
                "down",
                "left",
                "right",
                "menu",
                "capslock",
                "numlock",
                "scrolllock",
                "pageup",
                "pagedown",
                "sysrq",
                "kpplus",
                "kpdot",
                "kpenter",
                "kpequal",
                "kpasterisk",
                "kpslash",
                "kpminus",
                "leftmouse",
                "middlemouse",
                "rightmouse",
                "mouse1",
                "mouse2",
            ]
            .map(str::to_string),
        );
        keys.extend((1..=24).map(|n| format!("f{n}")));
        keys.extend((0..=9).map(|n| format!("kp{n}")));
        Self(keys)
    }

    pub fn with_discovered<I, S>(keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut set = Self::fallback();
        set.0.extend(
            keys.into_iter()
                .map(|s| s.as_ref().trim().to_lowercase())
                .filter(|s| valid_key_syntax(s)),
        );
        set
    }

    pub fn contains(&self, key: &str) -> bool {
        self.0.contains(key)
    }
}

fn valid_key_syntax(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn key(value: &str, keys: &KeySet) -> Result<String, ProfileError> {
    let normalized = value.trim().to_lowercase();
    if valid_key_syntax(&normalized) && keys.contains(&normalized) {
        Ok(normalized)
    } else {
        Err(ProfileError(format!("unknown key: {value}")))
    }
}

fn modifier_family(key: &str) -> Option<&'static str> {
    match key {
        "leftcontrol" | "rightcontrol" => Some("control"),
        "leftshift" | "rightshift" => Some("shift"),
        "leftalt" | "rightalt" => Some("alt"),
        "leftmeta" | "rightmeta" => Some("meta"),
        _ => None,
    }
}

fn shortcut(value: &str, keys: &KeySet) -> Result<String, ProfileError> {
    let normalized = value.trim().to_lowercase();
    let parts: Vec<_> = normalized.split('+').collect();
    let mut families = HashSet::new();
    let valid = parts
        .iter()
        .all(|part| valid_key_syntax(part) && keys.contains(part))
        && parts
            .last()
            .is_some_and(|part| modifier_family(part).is_none())
        && parts[..parts.len() - 1]
            .iter()
            .all(|part| modifier_family(part).is_some_and(|family| families.insert(family)));
    if valid {
        Ok(normalized)
    } else {
        Err(ProfileError(format!("invalid shortcut: {value}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChatKeys {
    pub open_chat: String,
    pub open_command: String,
    pub send: String,
    pub cancel: String,
}
impl Default for ChatKeys {
    fn default() -> Self {
        Self {
            open_chat: "t".into(),
            open_command: "slash".into(),
            send: "enter".into(),
            cancel: "esc".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResizePreset {
    pub id: String,
    #[serde(deserialize_with = "deserialize_dimension")]
    pub width: i64,
    #[serde(deserialize_with = "deserialize_dimension")]
    pub height: i64,
    pub shortcut: String,
    #[serde(default, deserialize_with = "deserialize_bool")]
    pub eye_zoom: bool,
}

fn deserialize_dimension<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                return Ok(integer);
            }
            let float = number
                .as_f64()
                .ok_or_else(|| D::Error::custom("invalid dimension"))?;
            if float.is_finite()
                && float.fract() == 0.0
                && float >= i64::MIN as f64
                && float < i64::MAX as f64
            {
                Ok(float as i64)
            } else {
                Err(D::Error::custom("invalid dimension"))
            }
        }
        serde_json::Value::String(text) => {
            parse_dimension_string(&text).ok_or_else(|| D::Error::custom("invalid dimension"))
        }
        _ => Err(D::Error::custom("invalid dimension")),
    }
}

fn parse_dimension_string(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    let unsigned = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty() {
        return None;
    }
    if !whole
        .split('_')
        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    if unsigned.contains('.') && (fraction.is_empty() || !fraction.bytes().all(|b| b == b'0')) {
        return None;
    }
    let number = trimmed.replace('_', "");
    let integer = number.split('.').next()?.parse::<i64>().ok()?;
    Some(integer)
}

fn deserialize_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Bool(value) => Ok(value),
        serde_json::Value::Number(number) if number.as_f64() == Some(1.0) => Ok(true),
        serde_json::Value::Number(number) if number.as_f64() == Some(0.0) => Ok(false),
        serde_json::Value::String(value) => match value.to_lowercase().as_str() {
            "true" | "t" | "yes" | "y" | "on" | "1" => Ok(true),
            "false" | "f" | "no" | "n" | "off" | "0" => Ok(false),
            _ => Err(D::Error::custom("invalid boolean")),
        },
        _ => Err(D::Error::custom("invalid boolean")),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    pub controls: IndexMap<String, String>,
    pub crafting: IndexMap<String, String>,
    pub snippets: Vec<String>,
    pub ninjabrain: IndexMap<String, String>,
    pub chat: ChatKeys,
    pub presets: Vec<ResizePreset>,
    pub launcher_root: Option<String>,
}

pub fn parse_profile(bytes: &[u8], keys: &KeySet) -> Result<Profile, ProfileError> {
    let mut profile: Profile =
        serde_json::from_slice(bytes).map_err(|e| ProfileError(e.to_string()))?;
    let mut physical_controls = HashSet::new();
    for (action, physical) in &mut profile.controls {
        if action.is_empty() || action.contains(['\n', ':']) {
            return Err(ProfileError("invalid Minecraft control id".into()));
        }
        *physical = key(physical, keys)?;
        if !physical_controls.insert(physical.clone()) {
            return Err(ProfileError("duplicate physical control key".into()));
        }
    }
    let mut crafting = IndexMap::new();
    for (source, target) in &profile.crafting {
        crafting.insert(key(source, keys)?, key(target, keys)?);
    }
    profile.crafting = crafting;
    for snippet in &mut profile.snippets {
        *snippet = snippet.to_uppercase();
        if snippet.is_empty()
            || snippet.len() > 24
            || !snippet
                .bytes()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Err(ProfileError(
                "snippets must use 1-24 letters or digits".into(),
            ));
        }
    }
    let mut ninjabrain = IndexMap::new();
    for (source, target) in &profile.ninjabrain {
        ninjabrain.insert(key(source, keys)?, shortcut(target, keys)?);
    }
    profile.ninjabrain = ninjabrain;
    profile.chat.open_chat = key(&profile.chat.open_chat, keys)?;
    profile.chat.open_command = key(&profile.chat.open_command, keys)?;
    profile.chat.send = key(&profile.chat.send, keys)?;
    profile.chat.cancel = key(&profile.chat.cancel, keys)?;
    let chat = [
        &profile.chat.open_chat,
        &profile.chat.open_command,
        &profile.chat.send,
        &profile.chat.cancel,
    ];
    let chat_set: HashSet<&String> = chat.into_iter().collect();
    if chat_set.len() != 4
        && !(chat_set.len() == 3
            && profile.chat.open_chat == "enter"
            && profile.chat.send == "enter")
    {
        return Err(ProfileError("chat keys must be distinct".into()));
    }
    if profile.presets.len() > 8 {
        return Err(ProfileError(
            "no more than eight presets are supported".into(),
        ));
    }
    let (mut ids, mut shortcuts) = (HashSet::new(), HashSet::new());
    let mut eye_count = 0;
    for preset in &mut profile.presets {
        let mut chars = preset.id.chars();
        if preset.id.len() > 32
            || !chars.next().is_some_and(|c| c.is_ascii_lowercase())
            || !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(ProfileError("preset id must be a lowercase name".into()));
        }
        if !(100..=16384).contains(&preset.width) {
            return Err(ProfileError("invalid preset width".into()));
        }
        if !(100..=16384).contains(&preset.height) {
            return Err(ProfileError("invalid preset height".into()));
        }
        preset.shortcut = shortcut(&preset.shortcut, keys)?;
        let destination = preset.shortcut.rsplit('+').next().unwrap_or_default();
        if matches!(destination, "leftmouse" | "middlemouse" | "rightmouse")
            || destination.starts_with("mouse")
        {
            return Err(ProfileError(
                "resize shortcuts require a keyboard key".into(),
            ));
        }
        if !ids.insert(&preset.id) {
            return Err(ProfileError("duplicate preset id".into()));
        }
        if !shortcuts.insert(&preset.shortcut) {
            return Err(ProfileError("duplicate preset shortcut".into()));
        }
        eye_count += usize::from(preset.eye_zoom);
    }
    if eye_count > 1 {
        return Err(ProfileError("only one eye zoom preset is supported".into()));
    }
    if chat_set.iter().any(|k| {
        profile.crafting.contains_key(*k)
            || profile.ninjabrain.contains_key(*k)
            || profile.controls.values().any(|v| v == *k)
    }) {
        return Err(ProfileError(
            "reserved chat key is assigned elsewhere".into(),
        ));
    }
    if profile
        .crafting
        .keys()
        .any(|k| profile.ninjabrain.contains_key(k))
    {
        return Err(ProfileError(
            "physical key is assigned to both crafting and Ninjabrain".into(),
        ));
    }
    Ok(profile)
}

pub fn game_bindings(profile: &Profile) -> Vec<String> {
    let mut bindings: Vec<String> = profile
        .crafting
        .iter()
        .map(|(source, target)| format!("{source} = {target}"))
        .collect();
    bindings.extend(profile.ninjabrain.iter().map(|(source, target)| {
        let parts: Vec<_> = target.split('+').collect();
        let mut action: Vec<&str> = parts[..parts.len() - 1]
            .iter()
            .map(|part| match *part {
                "leftcontrol" | "rightcontrol" => "C",
                "leftshift" | "rightshift" => "S",
                "leftalt" => "A",
                "rightalt" => "G",
                _ => "M",
            })
            .collect();
        action.push(parts[parts.len() - 1]);
        format!("{source} = {}", action.join("-"))
    }));
    bindings.push(format!(
        "{} = togglem(mcsr_chat, {})",
        profile.chat.open_chat, profile.chat.open_chat
    ));
    bindings.push(format!(
        "{} = togglem(mcsr_chat, {})",
        profile.chat.open_command, profile.chat.open_command
    ));
    bindings
}

pub fn render_keyd_fragment(profile: &Profile) -> String {
    let mut identity: Vec<&str> = profile
        .crafting
        .keys()
        .chain(profile.ninjabrain.keys())
        .map(String::as_str)
        .chain([
            profile.chat.open_chat.as_str(),
            profile.chat.open_command.as_str(),
        ])
        .filter(|k| *k != profile.chat.send && *k != profile.chat.cancel)
        .collect();
    identity.sort_unstable();
    identity.dedup();
    let mut lines = vec!["[mcsr_chat]".to_string()];
    lines.extend(identity.into_iter().map(|key| format!("{key} = {key}")));
    lines.push(format!("{} = clearm(enter)", profile.chat.send));
    lines.push(format!("{} = clearm(esc)", profile.chat.cancel));
    lines.join("\n") + "\n"
}
