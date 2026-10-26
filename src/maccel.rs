//! Unprivileged, serialized client for the fixed root-owned maccel helper.

use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex as StdMutex,
    },
    time::Duration,
};

use tokio::{io::AsyncWriteExt, process::Command, sync::Mutex};

use crate::maccel_helper::verify_executable;

const HELPER: &str = "/usr/local/libexec/mcsr-toolkit";
const SUDO: &str = "/usr/bin/sudo";
const EYE_REPLY: &str = "0.02083000005222856998443603515625";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitivityError(pub String);
impl fmt::Display for SensitivityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for SensitivityError {}

/// The privileged boundary is injectable so controllers can be tested without a device.
pub trait MaccelPeer: Send + Sync {
    fn preflight(&self) -> impl std::future::Future<Output = Result<(), SensitivityError>> + Send;
    fn eye(
        &self,
        generation: &str,
    ) -> impl std::future::Future<Output = Result<String, SensitivityError>> + Send;
    fn restore(&self)
        -> impl std::future::Future<Output = Result<String, SensitivityError>> + Send;
}

pub struct InstalledMaccelPeer {
    maccel: PathBuf,
}
impl InstalledMaccelPeer {
    pub fn new(maccel: PathBuf) -> Self {
        Self { maccel }
    }

    async fn run_output(
        program: &Path,
        args: &[&str],
        input: Option<&str>,
    ) -> Result<std::process::Output, SensitivityError> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(if input.is_some() {
                std::process::Stdio::piped()
            } else {
                std::process::Stdio::null()
            })
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| SensitivityError(e.to_string()))?;
        if let Some(input) = input {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| SensitivityError("helper stdin unavailable".into()))?;
            stdin
                .write_all(input.as_bytes())
                .await
                .map_err(|e| SensitivityError(e.to_string()))?;
        }
        tokio::time::timeout(COMMAND_TIMEOUT, child.wait_with_output())
            .await
            .map_err(|_| SensitivityError("maccel command timed out".into()))?
            .map_err(|e| SensitivityError(e.to_string()))
    }

    async fn run(
        program: &Path,
        args: &[&str],
        input: Option<&str>,
    ) -> Result<String, SensitivityError> {
        let result = Self::run_output(program, args, input).await?;
        if !result.status.success() {
            return Err(SensitivityError(
                String::from_utf8_lossy(&result.stderr).trim().to_owned(),
            ));
        }
        String::from_utf8(result.stdout).map_err(|e| SensitivityError(e.to_string()))
    }

    async fn permission_probe_with(program: &Path, helper: &str) -> Result<(), SensitivityError> {
        // Invalid stdin is rejected by the root helper before any runtime or sysfs access.
        let result = Self::run_output(
            program,
            &["-n", helper, "privileged", "maccel", "eye"],
            Some(""),
        )
        .await?;
        if result.status.code() == Some(1)
            && result.stdout.is_empty()
            && result.stderr == b"mcsr-toolkit: missing or malformed generation\n"
        {
            Ok(())
        } else {
            Err(SensitivityError(format!(
                "eye permission unconfirmed: {}",
                String::from_utf8_lossy(&result.stderr).trim()
            )))
        }
    }
}
impl MaccelPeer for InstalledMaccelPeer {
    async fn preflight(&self) -> Result<(), SensitivityError> {
        verify_executable(Path::new(HELPER), Path::new("/"), 0).map_err(SensitivityError)?;
        let value = Self::run(&self.maccel, &["get", "param", "sens-mult"], None).await?;
        if !value
            .trim()
            .parse::<f64>()
            .is_ok_and(|n| n.is_finite() && n > 0.0)
        {
            return Err(SensitivityError("invalid maccel multiplier".into()));
        }
        for command in ["eye", "restore"] {
            Self::run(
                Path::new(SUDO),
                &["-n", "-l", "--", HELPER, "privileged", "maccel", command],
                None,
            )
            .await?;
        }
        Self::permission_probe_with(Path::new(SUDO), HELPER).await?;
        Ok(())
    }
    async fn eye(&self, generation: &str) -> Result<String, SensitivityError> {
        Self::run(
            Path::new(SUDO),
            &["-n", HELPER, "privileged", "maccel", "eye"],
            Some(&format!("{generation}\n")),
        )
        .await
    }
    async fn restore(&self) -> Result<String, SensitivityError> {
        Self::run(
            Path::new(SUDO),
            &["-n", HELPER, "privileged", "maccel", "restore"],
            None,
        )
        .await
    }
}

#[derive(Default)]
struct State {
    generation: Option<String>,
    pending: bool,
    eye_active: bool,
}

pub struct SensitivityController<H: MaccelPeer> {
    peer: Arc<H>,
    operation: Mutex<()>,
    state: StdMutex<State>,
    epoch: AtomicU64,
}
impl<H: MaccelPeer> SensitivityController<H> {
    pub fn new(peer: Arc<H>) -> Self {
        Self {
            peer,
            operation: Mutex::new(()),
            state: StdMutex::new(State {
                pending: true,
                ..State::default()
            }),
            epoch: AtomicU64::new(0),
        }
    }
    pub fn unavailable(peer: Arc<H>) -> Self {
        Self {
            peer,
            operation: Mutex::new(()),
            state: StdMutex::new(State::default()),
            epoch: AtomicU64::new(0),
        }
    }
    pub fn needs_restore(&self) -> bool {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.pending || state.eye_active
    }
    pub fn recovery_pending(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pending
    }
    pub async fn startup_restore(&self) -> Result<(), SensitivityError> {
        self.restore().await
    }
    pub async fn restore(&self) -> Result<(), SensitivityError> {
        let epoch = self.epoch.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.generation = None;
            state.eye_active = false;
            state.pending = true;
        }
        let _guard = self.operation.lock().await;
        let generation = self.peer.restore().await?;
        if !valid_generation(&generation) {
            return Err(SensitivityError("invalid restore generation".into()));
        }
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.epoch.load(Ordering::SeqCst) != epoch {
            return Err(SensitivityError("restore operation invalidated".into()));
        }
        state.generation = Some(generation.trim_end_matches('\n').to_owned());
        state.pending = false;
        Ok(())
    }
    pub async fn eye(&self) -> Result<String, SensitivityError> {
        let _guard = self.operation.lock().await;
        let epoch = self.epoch.load(Ordering::SeqCst);
        let generation = {
            let state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.pending {
                return Err(SensitivityError("sensitivity recovery pending".into()));
            }
            state
                .generation
                .clone()
                .ok_or_else(|| SensitivityError("startup restore required".into()))?
        };
        if let Err(error) = self.peer.preflight().await {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.generation = None;
            state.eye_active = false;
            state.pending = true;
            return Err(error);
        }
        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if self.epoch.load(Ordering::SeqCst) != epoch {
                return Err(SensitivityError("eye operation invalidated".into()));
            }
            state.pending = true;
            state.eye_active = false;
        }
        let result = self.peer.eye(&generation).await;
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.epoch.load(Ordering::SeqCst) != epoch {
            return Err(SensitivityError("eye operation invalidated".into()));
        }
        let value = match result {
            Ok(value) => value,
            Err(error) => {
                state.generation = None;
                return Err(error);
            }
        };
        if value.trim() != EYE_REPLY {
            state.generation = None;
            return Err(SensitivityError("invalid eye readback".into()));
        }
        state.pending = false;
        state.eye_active = true;
        Ok("0.02083".into())
    }
}

fn valid_generation(value: &str) -> bool {
    let token = value.strip_suffix('\n').unwrap_or(value);
    token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    #[tokio::test]
    async fn empty_eye_probe_proves_noninteractive_permission_without_a_token() {
        let directory = tempfile::tempdir().unwrap();
        let fake_sudo = directory.path().join("fake-sudo");
        fs::write(
            &fake_sudo,
            "#!/bin/sh\n[ \"$#\" -eq 5 ] && [ \"$1\" = '-n' ] && [ \"$2\" = '/fixture/helper' ] && [ \"$3\" = 'privileged' ] && [ \"$4\" = 'maccel' ] && [ \"$5\" = 'eye' ] || exit 7\nif IFS= read -r line; then exit 8; fi\nprintf 'mcsr-toolkit: missing or malformed generation\\n' >&2\nexit 1\n",
        ).unwrap();
        fs::set_permissions(&fake_sudo, fs::Permissions::from_mode(0o700)).unwrap();
        InstalledMaccelPeer::permission_probe_with(&fake_sudo, "/fixture/helper")
            .await
            .unwrap();

        fs::write(
            &fake_sudo,
            "#!/bin/sh\nprintf 'sudo: a password is required\\n' >&2\nexit 1\n",
        )
        .unwrap();
        assert!(
            InstalledMaccelPeer::permission_probe_with(&fake_sudo, "/fixture/helper")
                .await
                .is_err()
        );
    }
}
