//! A deterministic physical-key plan shared by keyd and GNOME.
use crate::profile::{game_bindings, render_keyd_fragment, Profile};
use indexmap::IndexMap;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal {
    pub id: String,
    pub accelerator: String,
    pub eye_zoom: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingPlan {
    pub generation: String,
    pub signals: Vec<Signal>,
    pub bindings: Vec<String>,
    pub fragment: String,
}

fn shortcut_parts(shortcut: &str) -> Result<(String, String, String), String> {
    let mut parts: Vec<_> = shortcut.split('+').collect();
    let key = parts
        .pop()
        .filter(|s| !s.is_empty())
        .ok_or("empty resize shortcut")?;
    let mut modifiers = BTreeSet::new();
    for part in parts {
        let modifier = match part {
            "leftcontrol" | "rightcontrol" => (0, "C", "<Control>"),
            "leftshift" | "rightshift" => (1, "S", "<Shift>"),
            "leftalt" => (2, "A", "<Alt>"),
            "rightalt" => {
                return Err("AltGr resize shortcuts are not supported by the GNOME relay".into())
            }
            "leftmeta" | "rightmeta" => (4, "M", "<Super>"),
            _ => return Err(format!("unsupported resize modifier: {part}")),
        };
        if !modifiers.insert(modifier) {
            return Err("duplicate resize modifier".into());
        }
    }
    Ok((
        key.into(),
        modifiers.iter().map(|(_, m, _)| format!("{m}-")).collect(),
        modifiers.iter().map(|(_, _, m)| *m).collect(),
    ))
}
impl RoutingPlan {
    pub fn compile(profile: &Profile) -> Result<Self, String> {
        let generation = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(profile).map_err(|e| e.to_string())?)
        );
        let mut plan = Self {
            generation,
            signals: vec![],
            bindings: game_bindings(profile),
            fragment: render_keyd_fragment(profile),
        };
        if profile.presets.is_empty() {
            return Ok(plan);
        }
        if profile.presets.len() > 8 {
            return Err("no more than eight resize presets".into());
        }
        let mut used: BTreeSet<String> = profile.controls.values().cloned().collect();
        for (source, target) in profile.crafting.iter().chain(&profile.ninjabrain) {
            used.insert(source.clone());
            used.extend(target.split('+').map(str::to_string));
        }
        used.extend([
            profile.chat.open_chat.clone(),
            profile.chat.open_command.clone(),
            profile.chat.send.clone(),
            profile.chat.cancel.clone(),
        ]);
        for preset in &profile.presets {
            used.extend(preset.shortcut.split('+').map(str::to_string));
        }
        let mut available = (13..=23)
            .map(|n| format!("f{n}"))
            .filter(|k| !used.contains(k));
        let mut actions: IndexMap<String, String> = plan
            .bindings
            .iter()
            .map(|line| {
                let (key, value) = line.split_once(" = ").expect("generated binding");
                (key.into(), value.into())
            })
            .collect();
        let mut f3_sources = BTreeSet::new();
        if !actions.contains_key("f3") || profile.crafting.get("f3").is_some_and(|k| k == "f3") {
            f3_sources.insert("f3".to_owned());
        }
        for (source, target) in &profile.crafting {
            if target == "f3" {
                f3_sources.insert(source.clone());
            }
        }
        let mut identity: BTreeSet<String> = actions.keys().cloned().collect();
        identity.extend(["b".into(), "g".into(), "f".into(), "f3".into()]);
        let mut shortcuts = BTreeSet::new();
        for preset in &profile.presets {
            let (key, mods, gnome_mods) = shortcut_parts(&preset.shortcut)?;
            if f3_sources.contains(&key)
                || key == profile.chat.open_chat
                || key == profile.chat.open_command
            {
                return Err(format!(
                    "resize shortcut conflicts with F3 or chat key: {key}"
                ));
            }
            if !shortcuts.insert((key.clone(), mods.clone())) {
                return Err("duplicate resize shortcut".into());
            }
            let signal = available.next().ok_or(
                "not enough unused internal resize signal keys (F13-F23; F24 is the drain marker)",
            )?;
            let mut variants = vec![mods.clone()];
            if preset.eye_zoom {
                if mods.contains("S-") {
                    return Err("eye zoom shortcut must not include Shift".into());
                }
                let shifted = format!("S-{mods}");
                variants.push(shifted);
            }
            let fallback = actions.get(&key).cloned().unwrap_or_else(|| key.clone());
            let action = variants.into_iter().fold(fallback, |fallback, mods| {
                format!("matchmods({mods}{signal}, {fallback})")
            });
            actions.insert(key.clone(), action);
            identity.insert(key);
            plan.signals.push(Signal {
                id: preset.id.clone(),
                accelerator: format!("{gnome_mods}{}", signal.to_uppercase()),
                eye_zoom: preset.eye_zoom,
            });
        }
        // A shifted eye variant must not collide with another physical preset.
        for preset in profile.presets.iter().filter(|p| p.eye_zoom) {
            let (key, mods, _) = shortcut_parts(&preset.shortcut)?;
            let mut shifted: Vec<_> = mods.split('-').filter(|s| !s.is_empty()).collect();
            shifted.push("S");
            shifted.sort_unstable();
            for other in profile.presets.iter().filter(|p| p.id != preset.id) {
                let (other_key, other_mods, _) = shortcut_parts(&other.shortcut)?;
                let mut other_parts: Vec<_> =
                    other_mods.split('-').filter(|s| !s.is_empty()).collect();
                other_parts.sort_unstable();
                if key == other_key && shifted == other_parts {
                    return Err("shifted eye zoom conflicts with another preset".into());
                }
            }
        }
        for source in &f3_sources {
            actions.insert(source.clone(), "layerk(mcsr_f3, f3)".into());
            identity.insert(source.clone());
        }
        // Chat must override every managed physical source, including native F3.
        for key in &identity {
            if key != &profile.chat.send
                && key != &profile.chat.cancel
                && !plan
                    .fragment
                    .lines()
                    .any(|l| l.starts_with(&format!("{key} = ")))
            {
                plan.fragment.push_str(&format!("{key} = {key}\n"));
            }
        }
        plan.fragment.push_str("\n[mcsr_f3]\n");
        for key in identity {
            let debug_command = matches!(
                key.as_str(),
                "a" | "b" | "c" | "d" | "f" | "g" | "h" | "i" | "n" | "p" | "q" | "t" | "f4"
            );
            let action = if f3_sources.contains(&key) {
                "layerk(mcsr_f3, f3)".to_string()
            } else if !debug_command
                && (key == profile.chat.open_chat || key == profile.chat.open_command)
            {
                format!("togglem(mcsr_chat, {key})")
            } else {
                key.clone()
            };
            plan.fragment.push_str(&format!("{key} = {action}\n"));
        }
        plan.bindings = actions
            .into_iter()
            .map(|(k, v)| format!("{k} = {v}"))
            .collect();
        Ok(plan)
    }
}
