use crate::apply::KeydAdapter;
use crate::focus::FocusState;
use crate::paths::{atomic_replace, remove_record, safe_record};
use crate::profile::{parse_profile, KeySet, Profile};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const HELPER: &str = "/usr/local/libexec/mcsr-toolkit";

#[derive(Debug, Clone)]
pub struct KeydError(pub String);
impl fmt::Display for KeydError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for KeydError {}

#[derive(Debug)]
pub struct CommandOutput {
    pub stdout: String,
}

#[derive(Debug, Clone)]
pub enum CommandFailure {
    Failed(String),
    TimedOut(String),
}
impl fmt::Display for CommandFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(message) | Self::TimedOut(message) => f.write_str(message),
        }
    }
}
impl From<String> for CommandFailure {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}
impl From<&str> for CommandFailure {
    fn from(message: &str) -> Self {
        Self::Failed(message.into())
    }
}

pub trait CommandRunner: Send + Sync {
    fn run(&self, args: &[String], input: &[u8]) -> Result<CommandOutput, CommandFailure>;
    fn verify_helper(&self) -> Result<(), String>;
    fn keyd_binary(&self) -> Result<&'static str, String>;
}

pub struct SystemCommandRunner;
impl CommandRunner for SystemCommandRunner {
    fn run(&self, args: &[String], input: &[u8]) -> Result<CommandOutput, CommandFailure> {
        let deadline = if args.first().is_some_and(|arg| arg == "/usr/bin/pkexec") {
            Duration::from_secs(120)
        } else if args.first().is_some_and(|arg| arg == "/usr/bin/sudo")
            && args.last().is_some_and(|arg| arg == "clear")
            && !args.iter().any(|arg| arg == "-l")
        {
            Duration::from_secs(30)
        } else {
            Duration::from_secs(3)
        };
        run_command(args, input, deadline)
    }
    fn verify_helper(&self) -> Result<(), String> {
        let path = Path::new(HELPER);
        for component in path.ancestors() {
            let meta =
                fs::symlink_metadata(component).map_err(|e| format!("unsafe helper path: {e}"))?;
            if meta.uid() != 0
                || meta.permissions().mode() & 0o022 != 0
                || meta.file_type().is_symlink()
                || (component == path
                    && (!meta.is_file() || meta.permissions().mode() & 0o100 == 0))
                || (component != path && !meta.is_dir())
            {
                return Err("unsafe root-owned keyd helper".into());
            }
        }
        Ok(())
    }
    fn keyd_binary(&self) -> Result<&'static str, String> {
        let path = Path::new(crate::managed_keyd::BINARY_PATH);
        for component in path.ancestors() {
            let meta = fs::symlink_metadata(component)
                .map_err(|_| "toolkit-managed keyd is unavailable; run install/repair")?;
            if meta.uid() != 0
                || meta.permissions().mode() & 0o022 != 0
                || meta.file_type().is_symlink()
                || (component == path
                    && (!meta.is_file() || meta.permissions().mode() & 0o7777 != 0o755))
                || (component != path && !meta.is_dir())
            {
                return Err("unsafe toolkit-managed keyd path".into());
            }
        }
        if fs::read(path).map_err(|e| e.to_string())? != crate::managed_keyd::binary() {
            return Err(
                "toolkit-managed keyd differs from this release; run install/repair".into(),
            );
        }
        verify_managed_daemon(crate::managed_keyd::BINARY_PATH)?;
        Ok(crate::managed_keyd::BINARY_PATH)
    }
}

pub(crate) fn verify_managed_daemon(binary: &str) -> Result<(), String> {
    let response = run_command(
        &[binary.into(), "mcsr-daemon-capabilities".into()],
        &[],
        Duration::from_secs(3),
    )
    .map_err(|e| format!("managed keyd capability probe failed: {e}"))?;
    verify_managed_capabilities(&response.stdout)
}
fn verify_managed_capabilities(response: &str) -> Result<(), String> {
    for &capability in crate::managed_keyd::CAPABILITIES {
        if !response.split_whitespace().any(|token| token == capability) {
            return Err(format!(
                "running keyd daemon lacks {capability}; run install/repair"
            ));
        }
    }
    Ok(())
}

pub(crate) fn run_command(
    args: &[String],
    input: &[u8],
    deadline: Duration,
) -> Result<CommandOutput, CommandFailure> {
    let (program, rest) = args.split_first().ok_or("empty command")?;
    let mut stdin = tempfile::tempfile().map_err(|e| format!("command input failed: {e}"))?;
    stdin
        .write_all(input)
        .map_err(|e| format!("command input failed: {e}"))?;
    stdin
        .rewind()
        .map_err(|e| format!("command input failed: {e}"))?;
    let mut stdout = tempfile::tempfile().map_err(|e| format!("command output failed: {e}"))?;
    let mut stderr = tempfile::tempfile().map_err(|e| format!("command output failed: {e}"))?;
    let mut child = Command::new(program)
        .args(rest)
        .process_group(0)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::from(stdout.try_clone().map_err(|e| e.to_string())?))
        .stderr(Stdio::from(stderr.try_clone().map_err(|e| e.to_string())?))
        .spawn()
        .map_err(|e| format!("command failed: {e}"))?;
    let started = Instant::now();
    let status = loop {
        match child
            .try_wait()
            .map_err(|e| format!("command wait failed: {e}"))?
        {
            Some(status) => break status,
            None if started.elapsed() >= deadline => {
                // Descendants may change process group or privilege; the durable
                // uncertainty marker still gates activation after this attempt.
                unsafe { libc::killpg(child.id() as i32, libc::SIGKILL) };
                let _ = child.kill();
                let _ = child.wait();
                return Err(CommandFailure::TimedOut(
                    "command timed out; immediate child reaped; privileged descendants may remain"
                        .into(),
                ));
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    stdout.rewind().map_err(|e| e.to_string())?;
    stderr.rewind().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    stdout
        .take(1024 * 1024)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    stderr
        .take(1024 * 1024)
        .read_to_end(&mut err)
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(CommandFailure::Failed(format!(
            "command failed: {status}; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&out).trim(),
            String::from_utf8_lossy(&err).trim()
        )));
    }
    String::from_utf8(out)
        .map(|stdout| CommandOutput { stdout })
        .map_err(|e| CommandFailure::Failed(format!("invalid command output: {e}")))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backups {
    backups: Vec<String>,
}

#[derive(Default)]
struct Transition {
    active: bool,
    cleanup_failed: bool,
    clear_verified: bool,
    activate_events: u64,
    clear_events: u64,
}

pub struct KeydController {
    state: PathBuf,
    runner: Arc<dyn CommandRunner>,
    keys: KeySet,
    live_enabled: bool,
    transition: Mutex<Transition>,
}

impl KeydController {
    pub fn new(
        state: impl AsRef<Path>,
        runner: Arc<dyn CommandRunner>,
        live_enabled: bool,
    ) -> Self {
        Self::with_keys(state, runner, live_enabled, KeySet::fallback())
    }
    pub fn with_keys(
        state: impl AsRef<Path>,
        runner: Arc<dyn CommandRunner>,
        live_enabled: bool,
        keys: KeySet,
    ) -> Self {
        Self {
            state: state.as_ref().to_path_buf(),
            runner,
            keys,
            live_enabled,
            transition: Mutex::new(Transition::default()),
        }
    }
    pub fn production(state: impl AsRef<Path>) -> Self {
        Self::new(state, Arc::new(SystemCommandRunner), false)
    }
    fn run(&self, args: &[&str], input: &[u8]) -> Result<CommandOutput, KeydError> {
        self.runner
            .run(
                &args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
                input,
            )
            .map_err(|e| KeydError(e.to_string()))
    }
    fn marker_clear(&self) -> Result<(), KeydError> {
        match fs::symlink_metadata(self.state.join("mutation.json")) {
            Ok(_) => Err(KeydError("recovery required: unfinished mutation".into())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(KeydError(format!("cannot inspect mutation marker: {e}"))),
        }
    }
    fn uncertainty_clear(&self) -> Result<(), KeydError> {
        match fs::symlink_metadata(self.state.join("keyd-uncertain.json")) {
            Ok(_) => Err(KeydError(
                "keyd privileged operation may be incomplete; explicit external recovery required"
                    .into(),
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(KeydError(format!(
                "cannot inspect keyd uncertainty marker: {e}"
            ))),
        }
    }
    fn begin_privileged(&self) -> Result<Vec<u8>, KeydError> {
        crate::paths::private_dir(&self.state)
            .map_err(|e| KeydError(format!("cannot persist keyd uncertainty: {e}")))?;
        let path = self.state.join("keyd-uncertain.json");
        let bytes = format!("{{\"in_flight\":\"{}\"}}\n", uuid::Uuid::new_v4()).into_bytes();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    KeydError("keyd privileged operation may be incomplete; explicit external recovery required".into())
                } else {
                    KeydError(format!("cannot persist keyd uncertainty: {e}"))
                }
            })?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .and_then(|()| crate::paths::sync_dir(&self.state))
            .map_err(|e| KeydError(format!("cannot persist keyd uncertainty: {e}")))?;
        Ok(bytes)
    }
    fn finish_privileged(&self, bytes: &[u8]) -> Result<(), KeydError> {
        let path = self.state.join("keyd-uncertain.json");
        if safe_record(&path)
            .map_err(|e| KeydError(format!("unsafe keyd uncertainty marker: {e}")))?
            != bytes
        {
            return Err(KeydError(
                "keyd uncertainty marker changed during privileged operation".into(),
            ));
        }
        remove_record(&path)
            .map_err(|e| KeydError(format!("cannot complete keyd privileged operation: {e}")))
    }
    pub fn preflight_clear(&self) -> Result<(), KeydError> {
        self.uncertainty_clear()?;
        self.runner.verify_helper().map_err(KeydError)?;
        self.run(
            &[
                "/usr/bin/sudo",
                "-n",
                "-l",
                HELPER,
                "privileged",
                "keyd",
                "clear",
            ],
            &[],
        )?;
        Ok(())
    }
    pub fn activate(&self, profile: &Profile, focus: &FocusState) -> Result<(), KeydError> {
        let mut transition = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !self.live_enabled {
            return Err(KeydError("live keyd activation disabled".into()));
        }
        if transition.cleanup_failed {
            return Err(KeydError(
                "keyd cleanup unresolved; input safety unverified".into(),
            ));
        }
        self.uncertainty_clear()?;
        self.marker_clear()?;
        if !focus.effective_at(Instant::now()) {
            return Err(KeydError("Minecraft does not have effective focus".into()));
        }
        let profile = parse_profile(
            &serde_json::to_vec(profile).map_err(|e| KeydError(e.to_string()))?,
            &self.keys,
        )
        .map_err(|e| KeydError(e.to_string()))?;
        self.preflight_clear()?;
        self.marker_clear()?;
        if !focus.effective_at(Instant::now()) {
            return Err(KeydError("Minecraft focus expired".into()));
        }
        let mut args = vec![
            self.runner.keyd_binary().map_err(KeydError)?.to_string(),
            "bind".into(),
            "reset".into(),
        ];
        args.extend(
            crate::routing::RoutingPlan::compile(&profile)
                .map_err(KeydError)?
                .bindings,
        );
        if let Err(error) = self.runner.run(&args, &[]) {
            transition.cleanup_failed = true;
            transition.clear_verified = false;
            return Err(KeydError(format!(
                "keyd activation failed; input safety unverified: {error}"
            )));
        }
        transition.active = true;
        transition.clear_verified = false;
        transition.activate_events += 1;
        Ok(())
    }
    /// Ordered output marker used only while the game producer is disabled.
    pub fn routing_fence(&self) -> Result<(), KeydError> {
        if !self.live_enabled {
            return Err(KeydError("live keyd activation disabled".into()));
        }
        let binary = self.runner.keyd_binary().map_err(KeydError)?;
        self.runner
            .run(&[binary.into(), "do".into(), "f24".into()], &[])
            .map_err(|e| KeydError(e.to_string()))?;
        Ok(())
    }
    pub fn sync_focus(&self, profile: &Profile, focus: &FocusState) -> Result<(), KeydError> {
        let marker = self.marker_clear();
        let uncertainty = self.uncertainty_clear();
        let effective = focus.effective_at(Instant::now()) && marker.is_ok() && uncertainty.is_ok();
        let (active, cleanup_failed) = {
            let state = self
                .transition
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (state.active, state.cleanup_failed)
        };
        if cleanup_failed {
            self.clear()
        } else if uncertainty.is_err() {
            if active {
                self.clear()?;
            }
            uncertainty
        } else if effective {
            if active {
                Ok(())
            } else {
                self.activate(profile, focus)
            }
        } else if active || cleanup_failed {
            self.clear()
        } else {
            marker
        }
    }
    pub fn clear(&self) -> Result<(), KeydError> {
        let mut transition = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        transition.clear_events += 1;
        if let Err(error) = crate::paths::private_dir(&self.state) {
            transition.cleanup_failed = true;
            transition.clear_verified = false;
            return Err(KeydError(format!(
                "cannot clear keyd without private recovery state: {error}"
            )));
        }
        let reset = self
            .runner
            .keyd_binary()
            .map_err(CommandFailure::Failed)
            .and_then(|binary| {
                self.runner
                    .run(&[binary.into(), "bind".into(), "reset".into()], &[])
            });
        let clear = (|| -> Result<CommandOutput, KeydError> {
            self.runner.verify_helper().map_err(KeydError)?;
            let created = self.begin_privileged()?;
            let result = self
                .runner
                .run(
                    &[
                        "/usr/bin/sudo".into(),
                        "-n".into(),
                        HELPER.into(),
                        "privileged".into(),
                        "keyd".into(),
                        "clear".into(),
                    ],
                    &[],
                )
                .map_err(|e| KeydError(e.to_string()))?;
            self.finish_privileged(&created)?;
            Ok(result)
        })();
        transition.active = false;
        transition.cleanup_failed = reset.is_err() || clear.is_err();
        transition.clear_verified = !transition.cleanup_failed && self.uncertainty_clear().is_ok();
        if transition.cleanup_failed {
            return Err(KeydError(format!(
                "keyd cleanup failed; input safety unverified: {}; {}",
                reset
                    .err()
                    .map_or_else(|| "reset succeeded".into(), |e| e.to_string()),
                clear
                    .err()
                    .map_or_else(|| "helper clear succeeded".into(), |e| e.to_string())
            )));
        }
        self.uncertainty_clear()
    }
    pub fn input_safety_verified(&self) -> bool {
        let transition = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        !transition.active
            && !transition.cleanup_failed
            && transition.clear_verified
            && self.uncertainty_clear().is_ok()
    }
    pub fn event_counts(&self) -> (u64, u64) {
        let transition = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (transition.activate_events, transition.clear_events)
    }
    fn backups(&self) -> Result<Vec<String>, KeydError> {
        let path = self.state.join("keyd.json");
        let bytes = match safe_record(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(KeydError(format!("unsafe keyd adapter state: {e}"))),
        };
        let value: Backups = serde_json::from_slice(&bytes)
            .map_err(|e| KeydError(format!("invalid keyd adapter state: {e}")))?;
        if value.backups.is_empty()
            || value.backups.iter().any(|id| !valid_id(id))
            || value
                .backups
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != value.backups.len()
        {
            return Err(KeydError("invalid keyd adapter state".into()));
        }
        Ok(value.backups)
    }
    fn save(&self, backups: &[String]) -> Result<(), KeydError> {
        let path = self.state.join("keyd.json");
        if backups.is_empty() {
            remove_record(&path).map_err(|e| KeydError(e.to_string()))
        } else {
            crate::paths::private_dir(&self.state).map_err(|e| KeydError(e.to_string()))?;
            let bytes = serde_json::to_vec(&Backups {
                backups: backups.to_vec(),
            })
            .map_err(|e| KeydError(e.to_string()))?;
            atomic_replace(&path, &bytes, true).map_err(|e| KeydError(e.to_string()))
        }
    }
    fn helper(
        &self,
        action: &str,
        payload: &[u8],
        expected: &str,
        owned_marker: &[u8],
        dispatched: &mut bool,
    ) -> Result<String, KeydError> {
        let preflight = (|| -> Result<Vec<String>, KeydError> {
            crate::paths::private_dir(&self.state).map_err(|e| {
                KeydError(format!(
                    "cannot run keyd helper without private recovery state: {e}"
                ))
            })?;
            self.runner.verify_helper().map_err(KeydError)?;
            let mut args = vec![
                "/usr/bin/pkexec".to_string(),
                HELPER.into(),
                "privileged".into(),
                "keyd".into(),
                action.into(),
            ];
            if action == "restore" {
                args.push(
                    std::str::from_utf8(payload)
                        .map_err(|e| KeydError(e.to_string()))?
                        .into(),
                );
            }
            if safe_record(&self.state.join("keyd-uncertain.json"))
                .map_err(|e| KeydError(format!("unsafe keyd uncertainty marker: {e}")))?
                != owned_marker
            {
                return Err(KeydError(
                    "keyd uncertainty marker changed during privileged operation".into(),
                ));
            }
            Ok(args)
        })();
        let args = match preflight {
            Ok(args) => args,
            Err(error) => {
                if !*dispatched {
                    self.finish_privileged(owned_marker)?;
                }
                return Err(error);
            }
        };
        *dispatched = true;
        let output = self
            .runner
            .run(&args, if action == "install" { payload } else { &[] })
            .map_err(|e| KeydError(e.to_string()))?;
        let value: serde_json::Value = serde_json::from_str(&output.stdout)
            .map_err(|_| KeydError("invalid keyd helper response".into()))?;
        let obj = value
            .as_object()
            .ok_or_else(|| KeydError("invalid keyd helper response".into()))?;
        let id = obj
            .get(expected)
            .and_then(|value| value.as_str())
            .filter(|id| obj.len() == 1 && valid_id(id))
            .ok_or_else(|| KeydError("invalid keyd helper response".into()))?;
        if action == "restore" && id.as_bytes() != payload {
            return Err(KeydError("invalid keyd restore response".into()));
        }
        Ok(id.into())
    }
    fn install_inner(&self, profile: &Profile) -> Result<(), KeydError> {
        let profile = parse_profile(
            &serde_json::to_vec(profile).map_err(|e| KeydError(e.to_string()))?,
            &self.keys,
        )
        .map_err(|e| KeydError(e.to_string()))?;
        self.runner.verify_helper().map_err(KeydError)?;
        let created = self.begin_privileged()?;
        let mut backups = match self.backups() {
            Ok(backups) => backups,
            Err(error) => {
                self.finish_privileged(&created)?;
                return Err(error);
            }
        };
        let mut dispatched = false;
        let id = self.helper(
            "install",
            crate::routing::RoutingPlan::compile(&profile)
                .map_err(KeydError)?
                .fragment
                .as_bytes(),
            "backup_id",
            &created,
            &mut dispatched,
        )?;
        backups.push(id.clone());
        if let Err(error) = self.save(&backups) {
            let _ = self.helper(
                "restore",
                id.as_bytes(),
                "restored",
                &created,
                &mut dispatched,
            );
            return Err(error);
        }
        self.finish_privileged(&created)?;
        Ok(())
    }
    fn restore_inner(&self, checkpoint: &[String], created: &[u8]) -> Result<(), KeydError> {
        let backups = match self.backups() {
            Ok(backups) => backups,
            Err(error) => {
                self.finish_privileged(created)?;
                return Err(error);
            }
        };
        self.restore_with_backups(checkpoint, &backups, created)
    }
    fn restore_with_backups(
        &self,
        checkpoint: &[String],
        backups: &[String],
        created: &[u8],
    ) -> Result<(), KeydError> {
        if checkpoint.len() > backups.len() || backups[..checkpoint.len()] != *checkpoint {
            self.finish_privileged(created)?;
            return Err(KeydError("stale keyd checkpoint".into()));
        }
        if checkpoint.len() < backups.len() {
            let mut dispatched = false;
            self.helper(
                "restore",
                backups[checkpoint.len()].as_bytes(),
                "restored",
                created,
                &mut dispatched,
            )?;
            self.save(checkpoint)?;
        }
        self.finish_privileged(created)
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl KeydAdapter for KeydController {
    fn install(&self, profile: &Profile) -> Result<(), String> {
        let _lock = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.install_inner(profile).map_err(|e| e.to_string())
    }
    fn checkpoint(&self) -> Result<Vec<String>, String> {
        let _lock = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let created = self.begin_privileged().map_err(|e| e.to_string())?;
        let backups = self.backups();
        self.finish_privileged(&created)
            .map_err(|e| e.to_string())?;
        backups.map_err(|e| e.to_string())
    }
    fn restore_to(&self, checkpoint: &[String]) -> Result<(), String> {
        let _lock = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let created = self.begin_privileged().map_err(|e| e.to_string())?;
        self.restore_inner(checkpoint, &created)
            .map_err(|e| e.to_string())
    }
    fn rollback(&self) -> Result<(), String> {
        let _lock = self
            .transition
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let created = self.begin_privileged().map_err(|e| e.to_string())?;
        let mut backups = match self.backups() {
            Ok(backups) => backups,
            Err(error) => {
                self.finish_privileged(&created)
                    .map_err(|e| e.to_string())?;
                return Err(error.to_string());
            }
        };
        if !backups.is_empty() {
            let current = backups.clone();
            backups.pop();
            self.restore_with_backups(&backups, &current, &created)
                .map_err(|e| e.to_string())?;
        } else {
            self.finish_privileged(&created)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::run_command;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    struct NeverDispatch;
    impl super::CommandRunner for NeverDispatch {
        fn run(
            &self,
            _args: &[String],
            _input: &[u8],
        ) -> Result<super::CommandOutput, super::CommandFailure> {
            panic!("privileged command must not run during preflight failure")
        }
        fn verify_helper(&self) -> Result<(), String> {
            Ok(())
        }
        fn keyd_binary(&self) -> Result<&'static str, String> {
            Err("unused".into())
        }
    }

    #[test]
    fn malformed_restore_payload_releases_pre_dispatch_marker() {
        let state = tempfile::tempdir().unwrap();
        std::fs::set_permissions(state.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let controller =
            super::KeydController::new(state.path(), std::sync::Arc::new(NeverDispatch), true);
        let owned = controller.begin_privileged().unwrap();
        let mut dispatched = false;
        assert!(controller
            .helper("restore", b"\xff", "restored", &owned, &mut dispatched)
            .is_err());
        assert!(!dispatched);
        assert!(!state.path().join("keyd-uncertain.json").exists());
    }

    #[test]
    fn private_dir_preflight_failure_releases_owned_marker() {
        let state = tempfile::tempdir().unwrap();
        std::fs::set_permissions(state.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let controller =
            super::KeydController::new(state.path(), std::sync::Arc::new(NeverDispatch), true);
        let owned = controller.begin_privileged().unwrap();
        std::fs::set_permissions(state.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
        let mut dispatched = false;
        assert!(controller
            .helper("restore", b"a", "restored", &owned, &mut dispatched)
            .is_err());
        assert!(!dispatched);
        assert!(!state.path().join("keyd-uncertain.json").exists());
    }

    #[test]
    fn hung_child_is_killed_before_deadline_expires_twice() {
        let now = Instant::now();
        let result = run_command(
            &["/bin/sh".into(), "-c".into(), "sleep 5".into()],
            &[],
            Duration::from_millis(100),
        );
        assert!(matches!(
            result.unwrap_err(),
            super::CommandFailure::TimedOut(_)
        ));
        assert!(now.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn child_that_does_not_read_stdin_is_killed_without_blocking_writer() {
        let now = Instant::now();
        let result = run_command(
            &["/bin/sh".into(), "-c".into(), "sleep 5".into()],
            &vec![b'x'; 1024 * 1024],
            Duration::from_millis(100),
        );
        assert!(matches!(
            result.unwrap_err(),
            super::CommandFailure::TimedOut(_)
        ));
        assert!(now.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn daemon_capability_probe_requires_each_complete_token() {
        assert!(super::verify_managed_capabilities("mcsr-layerk-v1\nmcsr-matchmods-v1\n").is_ok());
        for response in [
            "",
            "mcsr-layerk-v1",
            "mcsr-matchmods-v1",
            "mcsr-layerk-v1-extra mcsr-matchmods-v1",
        ] {
            assert!(
                super::verify_managed_capabilities(response).is_err(),
                "{response}"
            );
        }
    }
}
