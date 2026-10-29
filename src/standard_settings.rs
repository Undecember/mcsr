use indexmap::IndexMap;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::{
    collections::HashSet,
    fmt, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEdit {
    pub path: PathBuf,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardSettingsError(pub String);
impl fmt::Display for StandardSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for StandardSettingsError {}

fn fail(message: impl Into<String>) -> StandardSettingsError {
    StandardSettingsError(message.into())
}

// Checking each component prevents following a redirect through a config directory.
fn local_file(root: &Path, parts: &[&str]) -> Result<Option<PathBuf>, StandardSettingsError> {
    let mut path = root.to_path_buf();
    for (index, part) in std::iter::once(None)
        .chain(parts.iter().map(|p| Some(*p)))
        .enumerate()
    {
        if let Some(part) = part {
            path.push(part);
        }
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && index > 0 => return Ok(None),
            Err(e) => return Err(fail(format!("cannot inspect {}: {e}", path.display()))),
        };
        if meta.file_type().is_symlink() {
            return Err(fail(format!(
                "symlink is not supported: {}",
                path.display()
            )));
        }
        let last = index == parts.len();
        if (last && !meta.is_file()) || (!last && !meta.is_dir()) {
            return Err(fail(format!(
                "unsafe configuration path: {}",
                path.display()
            )));
        }
    }
    Ok(Some(path))
}

// serde_json's Value normally silently accepts duplicate names, obscuring which
// value the mod will use. Reject them while decoding, including object fields.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Unique;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON value without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
                Ok(Unique(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
                Ok(Unique(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
                Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
                Ok(Unique(Value::String(v.into())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
                Ok(Unique(Value::String(v)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
                let mut values = Vec::new();
                while let Some(Unique(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
                let mut values = Map::new();
                while let Some((k, Unique(v))) = map.next_entry::<String, Unique>()? {
                    if values.insert(k.clone(), v).is_some() {
                        return Err(de::Error::custom(format!("duplicate JSON key {k}")));
                    }
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(V)
    }
}

fn json_edit(
    bytes: &[u8],
    bindings: &IndexMap<String, String>,
    windowed: bool,
) -> Result<Vec<u8>, StandardSettingsError> {
    let Unique(mut value) = serde_json::from_slice(bytes)
        .map_err(|e| fail(format!("invalid Standard Settings JSON: {e}")))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| fail("Standard Settings JSON must be an object"))?;
    let mut changed = false;
    for (key, entry) in object.iter_mut().filter(|(key, _)| key.starts_with("key_")) {
        let target = match entry {
            Value::String(s) => Some(s),
            Value::Object(obj) => {
                let enabled = obj
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| fail(format!("invalid enabled flag for {key}")))?;
                let value = obj
                    .get_mut("value")
                    .and_then(|v| match v {
                        Value::String(s) => Some(s),
                        _ => None,
                    })
                    .ok_or_else(|| fail(format!("invalid value for {key}")))?;
                if enabled {
                    Some(value)
                } else {
                    None
                }
            }
            _ => return Err(fail(format!("invalid control entry {key}"))),
        };
        if let (Some(current), Some(new)) = (target, bindings.get(key)) {
            if current != new {
                *current = new.clone();
                changed = true;
            }
        }
    }
    if windowed {
        if let Some(entry) = object.get_mut("fullscreen") {
            let target = match entry {
                Value::Bool(value) => Some(value),
                Value::Object(obj) => match obj.get("enabled").and_then(Value::as_bool) {
                    Some(false) => None,
                    Some(true) => Some(
                        obj.get_mut("value")
                            .and_then(|v| {
                                if let Value::Bool(b) = v {
                                    Some(b)
                                } else {
                                    None
                                }
                            })
                            .ok_or_else(|| fail("invalid active fullscreen boolean"))?,
                    ),
                    None => return Err(fail("invalid fullscreen enabled flag")),
                },
                _ => return Err(fail("invalid active fullscreen boolean")),
            };
            if let Some(value) = target {
                changed |= *value;
                *value = false;
            }
        }
    }
    if changed {
        serde_json::to_vec_pretty(&value).map_err(|e| fail(e.to_string()))
    } else {
        Ok(bytes.to_vec())
    }
}

fn legacy_edit(
    bytes: &[u8],
    bindings: &IndexMap<String, String>,
    windowed: bool,
) -> Result<Vec<u8>, StandardSettingsError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| fail(format!("invalid Standard Settings text: {e}")))?;
    let trimmed = text.trim();
    if !trimmed.is_empty()
        && !trimmed.contains(['\r', '\n'])
        && (!trimmed.contains(':')
            || (trimmed.len() > 2
                && trimmed.as_bytes()[1] == b':'
                && matches!(trimmed.as_bytes()[2], b'\\' | b'/')))
    {
        return Err(fail(
            "shared Standard Settings path redirect is unsupported; use a local config",
        ));
    }
    let mut seen = HashSet::new();
    let mut output = String::new();
    let mut start = 0;
    while start < text.len() {
        let bytes = text.as_bytes();
        let mut end = start;
        while end < bytes.len() && !matches!(bytes[end], b'\r' | b'\n') {
            end += 1;
        }
        let mut next = end;
        if next < bytes.len() {
            next += 1;
            if bytes[end] == b'\r' && bytes.get(next) == Some(&b'\n') {
                next += 1;
            }
        }
        let line = &text[start..end];
        if windowed && (line == "fullscreen" || line.starts_with("fullscreen:")) {
            if !seen.insert("fullscreen") {
                return Err(fail("duplicate fullscreen entry"));
            }
            let old = line
                .strip_prefix("fullscreen:")
                .ok_or_else(|| fail("invalid fullscreen entry"))?;
            if !old.trim().is_empty() {
                if !matches!(old, "true" | "false") {
                    return Err(fail("invalid active fullscreen boolean"));
                }
                output.push_str("fullscreen:false");
                output.push_str(&text[end..next]);
                start = next;
                continue;
            }
        }
        if let Some((key, old)) = line.split_once(':') {
            if key.starts_with("key_") {
                if !seen.insert(key) {
                    return Err(fail(format!("duplicate control entry: {key}")));
                }
                if !old.trim().is_empty() {
                    if let Some(new) = bindings.get(key) {
                        output.push_str(key);
                        output.push(':');
                        output.push_str(new);
                        output.push_str(&text[end..next]);
                        start = next;
                        continue;
                    }
                }
            }
        }
        output.push_str(&text[start..next]);
        start = next;
    }
    Ok(output.into_bytes())
}

pub fn prepare_local(
    game_dir: &Path,
    bindings: &IndexMap<String, String>,
) -> Result<Vec<ConfigEdit>, StandardSettingsError> {
    prepare_local_with_windowed(game_dir, bindings, false)
}

pub fn prepare_local_with_windowed(
    game_dir: &Path,
    bindings: &IndexMap<String, String>,
    windowed: bool,
) -> Result<Vec<ConfigEdit>, StandardSettingsError> {
    if local_file(game_dir, &["config", "mcsr", "standardsettings.global"])?.is_some() {
        return Err(fail(
            "shared Standard Settings .global redirect is unsupported; use a local config",
        ));
    }
    let mut edits = Vec::new();
    for (parts, modern) in [
        (&["config", "mcsr", "standardsettings.json"][..], true),
        (&["config", "standardoptions.txt"][..], false),
    ] {
        if let Some(path) = local_file(game_dir, parts)? {
            let before = fs::read(&path)
                .map_err(|e| fail(format!("cannot read {}: {e}", path.display())))?;
            let after = if modern {
                json_edit(&before, bindings, windowed)?
            } else {
                legacy_edit(&before, bindings, windowed)?
            };
            edits.push(ConfigEdit {
                path,
                before,
                after,
            });
        }
    }
    Ok(edits)
}
