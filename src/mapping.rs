use crate::options::OptionsDocument;
use crate::profile::Profile;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedState {
    pub original: IndexMap<String, String>,
    pub last_emitted: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlan {
    pub updates: IndexMap<String, String>,
    pub preview: Vec<(String, String, String)>,
    pub conflicts: Vec<String>,
    pub next_state: ManagedState,
    pub keyd_bindings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingError(pub String);
impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for MappingError {}

fn keyd_to_minecraft(key: &str) -> Option<String> {
    if key.len() == 1
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return Some(key.into());
    }
    if let Some(number) = key.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
        if (1..=24).contains(&number) && key == format!("f{number}") {
            return Some(key.into());
        }
    }
    if let Some(number) = key.strip_prefix("kp").and_then(|n| n.parse::<u8>().ok()) {
        if number <= 9 && key == format!("kp{number}") {
            return Some(format!("keypad.{number}"));
        }
    }
    let mapped = match key {
        "apostrophe" | "backslash" | "backspace" | "comma" | "delete" | "end" | "enter"
        | "equal" | "home" | "insert" | "minus" | "pause" | "semicolon" | "slash" | "space"
        | "tab" | "up" | "down" | "left" | "right" | "menu" => key,
        "esc" | "escape" => "escape",
        "dot" => "period",
        "leftbrace" => "left.bracket",
        "rightbrace" => "right.bracket",
        "grave" => "grave.accent",
        "capslock" => "caps.lock",
        "numlock" => "num.lock",
        "scrolllock" => "scroll.lock",
        "pageup" => "page.up",
        "pagedown" => "page.down",
        "leftshift" => "left.shift",
        "rightshift" => "right.shift",
        "leftcontrol" => "left.control",
        "rightcontrol" => "right.control",
        "leftalt" => "left.alt",
        "rightalt" => "right.alt",
        "leftmeta" => "left.win",
        "rightmeta" => "right.win",
        "sysrq" => "print.screen",
        "kpplus" => "keypad.add",
        "kpdot" => "keypad.decimal",
        "kpenter" => "keypad.enter",
        "kpequal" => "keypad.equal",
        "kpasterisk" => "keypad.multiply",
        "kpslash" => "keypad.divide",
        "kpminus" => "keypad.subtract",
        _ => return None,
    };
    Some(mapped.into())
}

fn minecraft_to_keyd(key: &str) -> Option<String> {
    if key == "escape" {
        return Some("esc".into());
    }
    if key == "keypad.0"
        || key == "keypad.1"
        || key == "keypad.2"
        || key == "keypad.3"
        || key == "keypad.4"
        || key == "keypad.5"
        || key == "keypad.6"
        || key == "keypad.7"
        || key == "keypad.8"
        || key == "keypad.9"
    {
        return Some(format!("kp{}", key.chars().last()?));
    }
    if keyd_to_minecraft(key).as_deref() == Some(key) {
        return Some(key.into());
    }
    for alias in [
        "dot",
        "leftbrace",
        "rightbrace",
        "grave",
        "capslock",
        "numlock",
        "scrolllock",
        "pageup",
        "pagedown",
        "leftshift",
        "rightshift",
        "leftcontrol",
        "rightcontrol",
        "leftalt",
        "rightalt",
        "leftmeta",
        "rightmeta",
        "sysrq",
        "kpplus",
        "kpdot",
        "kpenter",
        "kpequal",
        "kpasterisk",
        "kpslash",
        "kpminus",
    ] {
        if keyd_to_minecraft(alias).as_deref() == Some(key) {
            return Some(alias.into());
        }
    }
    None
}

fn encode(key: &str) -> Result<String, MappingError> {
    let mouse = match key {
        "leftmouse" => Some("left"),
        "middlemouse" => Some("middle"),
        "rightmouse" => Some("right"),
        "mouse1" => Some("4"),
        "mouse2" => Some("5"),
        _ => None,
    };
    if let Some(button) = mouse {
        return Ok(format!("key.mouse.{button}"));
    }
    keyd_to_minecraft(key)
        .map(|name| format!("key.keyboard.{name}"))
        .ok_or_else(|| MappingError(format!("unsupported Minecraft key: {key}")))
}

fn decode(binding: &str) -> Option<String> {
    if let Some(key) = binding.strip_prefix("key.keyboard.") {
        return minecraft_to_keyd(key);
    }
    match binding.strip_prefix("key.mouse.")? {
        "left" => Some("leftmouse".into()),
        "middle" => Some("middlemouse".into()),
        "right" => Some("rightmouse".into()),
        "4" => Some("mouse1".into()),
        "5" => Some("mouse2".into()),
        _ => None,
    }
}

fn supported_other_binding(binding: &str) -> bool {
    binding == "key.keyboard.unknown"
        || matches!(
            binding,
            "key.mouse.left" | "key.mouse.right" | "key.mouse.middle"
        )
        || binding
            .strip_prefix("key.mouse.")
            .is_some_and(|n| matches!(n, "4" | "5" | "6" | "7" | "8"))
}

pub fn plan_controls(
    document: &OptionsDocument,
    profile: &Profile,
    previous: Option<&ManagedState>,
) -> Result<ControlPlan, MappingError> {
    let routing = crate::routing::RoutingPlan::compile(profile).map_err(MappingError)?;
    let empty = ManagedState::default();
    let prior = previous.unwrap_or(&empty);
    let mut conflicts = Vec::new();
    let mut updates = IndexMap::new();
    let mut preview = Vec::new();
    let mut original = IndexMap::new();
    let mut last_emitted = IndexMap::new();
    let mut effective = IndexMap::new();
    let mut affected = HashSet::new();
    for key in profile.controls.values() {
        encode(key)?;
    }
    encode(&profile.chat.open_chat)?;
    encode(&profile.chat.open_command)?;
    for action in profile.controls.keys() {
        if !document.bindings.contains_key(action) {
            conflicts.push(format!("unknown Minecraft control: {action}"));
        }
    }
    let mut bindings = document.bindings.clone();
    for (action, key, default) in [
        ("key_key.chat", &profile.chat.open_chat, "t"),
        ("key_key.command", &profile.chat.open_command, "slash"),
    ] {
        if !bindings.contains_key(action) && key != default {
            bindings.insert(action.into(), encode(default)?);
        }
    }
    for (action, current) in &bindings {
        let observed = decode(current);
        if observed.is_none() && !supported_other_binding(current) {
            return Err(MappingError(format!(
                "unsupported Minecraft binding for {action}: {current}"
            )));
        }
        let mut physical = if prior.last_emitted.get(action) == Some(current) {
            prior.original.get(action).cloned().or(observed)
        } else {
            observed
        };
        if let Some(control) = profile.controls.get(action) {
            physical = Some(control.clone());
        }
        if let Some(chat_physical) = match action.as_str() {
            "key_key.chat" => Some(&profile.chat.open_chat),
            "key_key.command" => Some(&profile.chat.open_command),
            _ => None,
        } {
            if profile.controls.contains_key(action) && physical.as_ref() != Some(chat_physical) {
                conflicts.push(format!(
                    "Minecraft chat control differs from chat key: {action}"
                ));
            }
            physical = Some(chat_physical.clone());
        }
        let Some(physical) = physical else {
            effective.insert(action.clone(), current.clone());
            continue;
        };
        let emitted = profile
            .crafting
            .get(&physical)
            .cloned()
            .unwrap_or_else(|| physical.clone());
        let target = encode(&emitted)?;
        effective.insert(action.clone(), target.clone());
        if target != *current || !document.bindings.contains_key(action) {
            updates.insert(action.clone(), target.clone());
        }
        if updates.contains_key(action)
            || prior.original.contains_key(action)
            || profile.controls.contains_key(action)
        {
            affected.insert(action.clone());
            preview.push((physical.clone(), emitted, action.clone()));
            original.insert(action.clone(), physical.clone());
            last_emitted.insert(action.clone(), target);
        }
        if profile.ninjabrain.contains_key(&physical) {
            conflicts.push(format!(
                "Ninjabrain shortcut uses Minecraft control key: {action}"
            ));
        }
    }
    for (action, key) in [
        ("key_key.chat", &profile.chat.open_chat),
        ("key_key.command", &profile.chat.open_command),
    ] {
        effective.entry(action.into()).or_insert(encode(key)?);
        affected.insert(action.into());
    }
    for signal in &routing.signals {
        let key = signal
            .accelerator
            .rsplit('>')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let encoded = encode(&key)?;
        for (action, output) in &effective {
            if output == &encoded {
                conflicts.push(format!(
                    "internal resize signal {key} conflicts with Minecraft control {action}"
                ));
            }
        }
    }
    let mut by_key: IndexMap<String, Vec<String>> = IndexMap::new();
    for (action, output) in effective {
        by_key.entry(output).or_default().push(action);
    }
    for (output, actions) in by_key {
        if actions.len() > 1 && actions.iter().any(|action| affected.contains(action)) {
            conflicts.push(format!(
                "emitted {output} conflicts: {}",
                actions.join(", ")
            ));
        }
    }
    Ok(ControlPlan {
        updates,
        preview,
        conflicts,
        next_state: ManagedState {
            original,
            last_emitted,
        },
        keyd_bindings: routing.bindings,
    })
}
