use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub id: String,
    pub path: PathBuf,
    pub options_path: PathBuf,
    pub problem: Option<String>,
}

#[derive(Debug)]
pub struct DiscoveryError(pub String);
impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DiscoveryError {}

fn launcher_root(candidate: &Path) -> Option<PathBuf> {
    // Reject links at the managed boundary, including a linked instances directory.
    if fs::symlink_metadata(candidate)
        .ok()?
        .file_type()
        .is_symlink()
    {
        return None;
    }
    let instances = candidate.join("instances");
    let metadata = fs::symlink_metadata(&instances).ok()?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return None;
    }
    candidate.canonicalize().ok()
}

pub fn locate_launcher_root(
    managed_dir: &Path,
    override_root: Option<&Path>,
    process_cwds: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(configured) = override_root {
        return launcher_root(configured);
    }
    for candidate in [managed_dir.join("launcher"), managed_dir.to_path_buf()] {
        if let Some(root) = launcher_root(&candidate) {
            return Some(root);
        }
    }
    for cwd in process_cwds {
        if let Some(root) = launcher_root(&cwd.join("launcher")) {
            return Some(root);
        }
    }
    None
}

pub fn discover_instances(root: &Path) -> Result<Vec<Instance>, DiscoveryError> {
    let root =
        launcher_root(root).ok_or_else(|| DiscoveryError("not a safe MCSRLauncher root".into()))?;
    let mut entries = fs::read_dir(root.join("instances"))
        .map_err(|error| DiscoveryError(format!("cannot read instances: {error}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| DiscoveryError(format!("cannot read instances: {error}")))?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut found = Vec::new();
    for entry in entries {
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| DiscoveryError(format!("cannot inspect instance: {error}")))?;
        if !kind.is_dir() || kind.is_symlink() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let config_path = path.join("instance.json");
        if !fs::symlink_metadata(&config_path)
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        {
            continue;
        }
        let Ok(bytes) = fs::read(&config_path) else {
            continue;
        };
        let Ok(Value::Object(config)) = serde_json::from_slice::<Value>(&bytes) else {
            continue;
        };
        if config.get("id").and_then(Value::as_str) != Some(id.as_str()) {
            continue;
        }
        let preferred = path.join("minecraft");
        let legacy = path.join(".minecraft");
        let game = if fs::symlink_metadata(&preferred).is_ok() || !legacy.exists() {
            preferred
        } else {
            legacy
        };
        let options_path = game.join("options.txt");
        let problem = {
            if fs::symlink_metadata(&game).is_ok_and(|m| m.file_type().is_symlink())
                || fs::symlink_metadata(&options_path).is_ok_and(|m| m.file_type().is_symlink())
            {
                Some("unsafe game path".into())
            } else if !options_path.is_file() {
                Some("options.txt is missing".into())
            } else {
                None
            }
        };
        found.push(Instance {
            id,
            path,
            options_path,
            problem,
        });
    }
    Ok(found)
}
