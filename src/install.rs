//! Guided, recoverable installation. Every external operation crosses `System`.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

pub const SERVICE_PATH: &str = "/usr/local/bin/mcsr-toolkit";
pub const HELPER_PATH: &str = "/usr/local/libexec/mcsr-toolkit";
pub const KEYD_POLICY_PATH: &str = "/etc/sudoers.d/mcsr-toolkit-rust-keyd";
pub const MACCEL_POLICY_PATH: &str = "/etc/sudoers.d/mcsr-toolkit-rust-maccel";
const EXTENSION_UUID: &str = "mcsr-toolkit@undecember.github.io";
const LEGACY_EXTENSION_UUID: &str = "mcsr-toolkit@local";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionState {
    Active,
    Inactive,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallError {
    Refused(String),
    System(String),
    RootAborted(String),
    Staged(String),
}
impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(s) | Self::System(s) | Self::RootAborted(s) | Self::Staged(s) => {
                f.write_str(s)
            }
        }
    }
}
impl std::error::Error for InstallError {}

#[derive(Clone, Debug)]
pub struct Environment {
    pub login: String,
    pub home: PathBuf,
    pub data_home: PathBuf,
    pub architecture: String,
    pub gnome_major: u32,
    pub wayland: bool,
    pub python_service_active: bool,
    pub other_toolkit_active: bool,
    pub keyd_recovery_pending: bool,
    pub maccel_recovery_pending: bool,
    pub missing_commands: Vec<String>,
}
impl Environment {
    pub fn supported(login: &str, home: PathBuf) -> Self {
        Self {
            login: login.into(),
            data_home: home.join(".local/share"),
            home,
            architecture: "x86_64".into(),
            gnome_major: 50,
            wayland: true,
            python_service_active: false,
            other_toolkit_active: false,
            keyd_recovery_pending: false,
            maccel_recovery_pending: false,
            missing_commands: vec![],
        }
    }
    fn validate_login(&self) -> Result<(), InstallError> {
        if self.login.is_empty()
            || self.login == "ALL"
            || !self
                .login
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || self.login.starts_with('-')
        {
            return Err(InstallError::Refused(
                "invalid desktop login for sudoers".into(),
            ));
        }
        Ok(())
    }
    fn unit_path(&self) -> PathBuf {
        self.home.join(".config/systemd/user/mcsr-toolkit.service")
    }
    fn marker_path(&self) -> PathBuf {
        self.data_home
            .join("mcsr-toolkit/rust-v1/install-pending.json")
    }
    fn extension_dir(&self) -> PathBuf {
        self.home
            .join(".local/share/gnome-shell/extensions")
            .join(EXTENSION_UUID)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileState {
    pub bytes: Vec<u8>,
    pub uid: u32,
    pub mode: u32,
}
#[derive(Clone, Debug)]
pub struct RootFile {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub mode: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceAction {
    Stop,
    Reload,
    Enable,
    Start,
    Disable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionAction {
    Enable,
    Disable,
}
pub trait InstallGuard {}
impl<T> InstallGuard for T {}
pub trait RootSetup {
    fn commit(self: Box<Self>) -> Result<(), InstallError>;
    fn abort(self: Box<Self>) -> Result<(), InstallError>;
}
/// Classifies the helper's authenticated terminal response separately from child reaping.
pub fn classify_root_commit_response(
    response: &str,
    waited: Result<bool, InstallError>,
) -> Result<(), InstallError> {
    match response {
        "ABORTED\n" => {
            let detail = match waited {
                Ok(_) => "authenticated root transaction aborted and restored its targets".into(),
                Err(error) => format!("authenticated root transaction aborted and restored its targets; child wait failed: {error}"),
            };
            Err(InstallError::RootAborted(detail))
        }
        "COMMITTED\n" => match waited {
            Ok(true) => Ok(()),
            Ok(false) => Err(InstallError::System(
                "authenticated setup exited unsuccessfully after commit response".into(),
            )),
            Err(error) => Err(error),
        },
        _ => Err(InstallError::System(
            "invalid authenticated root transaction response".into(),
        )),
    }
}
pub trait System {
    fn lock_install(&self) -> Result<Box<dyn InstallGuard + '_>, InstallError>;
    fn environment(&self) -> Result<Environment, InstallError>;
    fn read(&self, path: &Path) -> Result<Option<FileState>, InstallError>;
    fn write_user(&self, path: &Path, bytes: &[u8], mode: u32) -> Result<(), InstallError>;
    fn remove_user(&self, path: &Path) -> Result<(), InstallError>;
    fn write_root(&self, files: &[RootFile]) -> Result<(), InstallError>;
    fn remove_root(&self, paths: &[PathBuf]) -> Result<(), InstallError>;
    fn validate_sudoers(&self, rule: &[u8]) -> Result<(), InstallError>;
    fn policies_match(&self, policy: &[RootFile; 2]) -> Result<bool, InstallError> {
        existing_policy(self, policy)
    }
    fn begin_root_setup(
        &self,
        artifact: &Artifact,
    ) -> Result<Box<dyn RootSetup + '_>, InstallError>;
    fn begin_root_uninstall(&self) -> Result<Box<dyn RootSetup + '_>, InstallError>;
    fn service(&self, action: ServiceAction) -> Result<(), InstallError>;
    fn extension(&self, action: ExtensionAction) -> Result<(), InstallError>;
    fn service_active(&self) -> Result<bool, InstallError>;
    fn service_dropins(&self) -> Result<Vec<PathBuf>, InstallError>;
    fn service_command(&self) -> Result<String, InstallError>;
    fn service_ready(&self) -> Result<bool, InstallError>;
    fn wait_readiness_poll(&self) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    fn service_enabled(&self) -> Result<bool, InstallError>;
    fn extension_enabled(&self) -> Result<bool, InstallError>;
    fn extension_known(&self) -> Result<bool, InstallError>;
    fn extension_state(&self) -> Result<ExtensionState, InstallError>;
    fn legacy_extension_known(&self) -> Result<bool, InstallError>;
    fn legacy_extension_enabled(&self) -> Result<bool, InstallError>;
    fn legacy_extension_state(&self) -> Result<ExtensionState, InstallError>;
    fn legacy_extension(&self, action: ExtensionAction) -> Result<(), InstallError>;
    fn toolkit_object_exported(&self) -> Result<bool, InstallError>;
    fn ensure_private_state(&self, data_home: &Path) -> Result<(), InstallError>;
    fn shell_instance(&self) -> Result<String, InstallError>;
    fn recover_input(&self) -> Result<(), InstallError>;
}

#[derive(Clone, Debug)]
pub struct Artifact {
    bytes: Vec<u8>,
    digest: [u8; 32],
}
impl Artifact {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, InstallError> {
        if bytes.is_empty() || bytes.len() > 100_000_000 {
            return Err(InstallError::Refused("invalid executable size".into()));
        }
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        Ok(Self { bytes, digest })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug)]
pub struct InstallReport {
    pub problems: Vec<String>,
    pub observations: Vec<String>,
    pub repair_pending: bool,
}
impl InstallReport {
    pub fn ready(&self) -> bool {
        self.problems.is_empty()
    }
}

fn validate_marker_state(state: &FileState) -> Result<(), InstallError> {
    if state.uid != unsafe { libc::geteuid() } || state.mode != 0o600 {
        return Err(InstallError::Refused(
            "unsafe install marker ownership or mode; manual review required".into(),
        ));
    }
    Ok(())
}

pub fn preflight(system: &dyn System) -> InstallReport {
    let mut report = InstallReport {
        problems: vec![],
        observations: vec![],
        repair_pending: false,
    };
    let env = match system.environment() {
        Ok(env) => env,
        Err(e) => {
            report.problems.push(e.to_string());
            return report;
        }
    };
    if let Err(e) = env.validate_login() {
        report.problems.push(e.to_string());
    }
    if env.architecture != "x86_64" {
        report.problems.push("requires x86_64".into());
    }
    if env.gnome_major != 50 || !env.wayland {
        report
            .problems
            .push("requires GNOME Shell 50 on Wayland".into());
    }
    if env.python_service_active {
        report
            .problems
            .push("active Python service must be stopped".into());
    }
    if env.other_toolkit_active {
        report
            .problems
            .push("another toolkit process holds the service lock".into());
    }
    match system.service_dropins() {
        Ok(dropins) if !dropins.is_empty() => report.problems.push(format!(
            "user service drop-in overrides must be reviewed before Rust cutover: {}",
            dropins
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Ok(_) => {}
        Err(e) => report.problems.push(e.to_string()),
    }
    if env.keyd_recovery_pending {
        report.problems.push("keyd recovery pending".into());
    }
    if env.maccel_recovery_pending {
        report
            .problems
            .push("maccel baseline recovery pending".into());
    }
    for name in &env.missing_commands {
        report
            .problems
            .push(format!("required command missing: {name}"));
    }
    for path in [
        env.data_home.join("mcsr-toolkit/state/mutation.json"),
        env.data_home.join("mcsr-toolkit/state/keyd-adapter.json"),
        env.data_home.join("mcsr-toolkit/rust-v1/mutation.json"),
        env.data_home
            .join("mcsr-toolkit/rust-v1/keyd-uncertain.json"),
    ] {
        match system.read(&path) {
            Ok(Some(_)) => report
                .problems
                .push(format!("unresolved recovery record: {}", path.display())),
            Ok(None) => {}
            Err(e) => report.problems.push(e.to_string()),
        }
    }
    match system.read(&env.unit_path()) {
        Ok(Some(_)) => report
            .observations
            .push("existing user service unit".into()),
        Ok(None) => {}
        Err(e) => report.problems.push(e.to_string()),
    }
    match system.read(&env.extension_dir().join("metadata.json")) {
        Ok(Some(_)) => report.observations.push("existing GNOME extension".into()),
        Ok(None) => {}
        Err(e) => report.problems.push(e.to_string()),
    }
    match system.legacy_extension_known() {
        Ok(true) => match (
            system.legacy_extension_enabled(),
            system.legacy_extension_state(),
        ) {
            (Ok(enabled), Ok(state)) => report.observations.push(format!(
                "legacy GNOME extension @local: enabled={enabled}, state={state:?}"
            )),
            (Err(error), _) | (_, Err(error)) => report.problems.push(error.to_string()),
        },
        Ok(false) => {}
        Err(error) => report.problems.push(error.to_string()),
    }
    match system.service_active() {
        Ok(true) => report
            .observations
            .push("toolkit user service is active".into()),
        Ok(false) => {}
        Err(e) => report.problems.push(e.to_string()),
    }
    match system.read(&env.marker_path()) {
        Ok(Some(state)) => {
            if let Err(error) = validate_marker_state(&state) {
                report.problems.push(error.to_string());
                return report;
            }
            report.repair_pending = true;
            if let Ok(marker) = serde_json::from_slice::<ExtensionStagingMarker>(&state.bytes) {
                if marker.phase == "shell-restart" {
                    report.observations.push("GNOME extension staged; installation is incomplete. Log out, log back in, then run mcsr-toolkit repair".into());
                } else if marker.phase == "handoff-uncertain" {
                    report.problems.push("GNOME extension handoff rollback is uncertain; manual recovery required before repair; retain the marker and old extension files".into());
                }
            }
        }
        Ok(None) => {}
        Err(e) => report.problems.push(e.to_string()),
    }
    report
}

fn policy(login: &str) -> [RootFile; 2] {
    [
        RootFile {
            path: KEYD_POLICY_PATH.into(),
            bytes: include_str!("../packaging/mcsr-toolkit-rust-keyd.sudoers")
                .replace("@DESKTOP_USER@", login)
                .into_bytes(),
            mode: 0o440,
        },
        RootFile {
            path: MACCEL_POLICY_PATH.into(),
            bytes: include_str!("../packaging/mcsr-toolkit-rust-maccel.sudoers")
                .replace("@DESKTOP_USER@", login)
                .into_bytes(),
            mode: 0o440,
        },
    ]
}
fn unit() -> &'static [u8] {
    include_bytes!("../packaging/mcsr-toolkit.service")
}
fn archive_files(env: &Environment) -> Result<Vec<(PathBuf, Vec<u8>)>, InstallError> {
    let bytes = crate::assets::extension_archive();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| InstallError::System(e.to_string()))?;
    let names = [
        "extension.js",
        "metadata.json",
        "schemas/org.mcsr.toolkit.gschema.xml",
        "schemas/gschemas.compiled",
    ];
    if zip.len() != names.len() {
        return Err(InstallError::Refused(
            "unexpected extension archive entries".into(),
        ));
    }
    let mut files = Vec::new();
    for name in names {
        let mut entry = zip
            .by_name(name)
            .map_err(|e| InstallError::System(e.to_string()))?;
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|e| InstallError::System(e.to_string()))?;
        if content.is_empty() || content.len() > 2_000_000 {
            return Err(InstallError::Refused(
                "invalid extension archive entry".into(),
            ));
        }
        files.push((env.extension_dir().join(name), content));
    }
    Ok(files)
}
fn verified_helper(system: &dyn System, artifact: &Artifact) -> Result<bool, InstallError> {
    verified_executable(system, Path::new(HELPER_PATH), artifact)
}
fn verified_executable(
    system: &dyn System,
    path: &Path,
    artifact: &Artifact,
) -> Result<bool, InstallError> {
    match system.read(path)? {
        None => Ok(false),
        Some(state)
            if state.uid == 0
                && state.mode == 0o755
                && Sha256::digest(&state.bytes).as_slice() == artifact.digest =>
        {
            Ok(true)
        }
        Some(_) => Err(InstallError::Refused(format!(
            "installed executable {} ownership, mode, or hash differs; manual review required",
            path.display()
        ))),
    }
}
fn existing_policy<S: System + ?Sized>(
    system: &S,
    policy: &[RootFile; 2],
) -> Result<bool, InstallError> {
    let mut all = true;
    for file in policy {
        match system.read(&file.path)? {
            None => all = false,
            Some(state)
                if state.uid == 0 && state.mode == file.mode && state.bytes == file.bytes => {}
            Some(_) => {
                return Err(InstallError::Refused(format!(
                    "existing policy {} differs; manual review required",
                    file.path.display()
                )))
            }
        }
    }
    Ok(all)
}
fn restore_user(
    system: &dyn System,
    prior: &[(PathBuf, Option<FileState>)],
) -> Result<(), InstallError> {
    for (path, state) in prior.iter().rev() {
        match state {
            Some(state) => system.write_user(path, &state.bytes, state.mode)?,
            None => system.remove_user(path)?,
        }
    }
    Ok(())
}

fn verify_resolved_command(system: &dyn System) -> Result<(), InstallError> {
    let resolved = system.service_command()?;
    let expected = "/usr/local/bin/mcsr-toolkit serve";
    let exact = resolved.trim() == expected || {
        let path = resolved
            .split_once("path=")
            .and_then(|(_, tail)| tail.split_once(" ;"));
        let argv = resolved
            .split_once("argv[]=")
            .and_then(|(_, tail)| tail.split_once(" ;"));
        resolved.matches("path=").count() == 1
            && resolved.matches("argv[]=").count() == 1
            && path.is_some_and(|(value, _)| value == "/usr/local/bin/mcsr-toolkit")
            && argv.is_some_and(|(value, _)| value == expected)
    };
    if exact {
        Ok(())
    } else {
        Err(InstallError::Refused(format!(
            "resolved ExecStart is not the observe-only Rust service: {resolved}"
        )))
    }
}

fn verify_running_service(system: &dyn System) -> Result<(), InstallError> {
    for attempt in 0..40 {
        if !system.service_active()? {
            return Err(InstallError::System(
                "user service did not remain active after start".into(),
            ));
        }
        verify_resolved_command(system)?;
        if system.service_ready()? {
            if !system.service_active()? {
                return Err(InstallError::System(
                    "user service did not remain active after readiness".into(),
                ));
            }
            verify_resolved_command(system)?;
            return Ok(());
        }
        if attempt < 39 {
            system.wait_readiness_poll();
        }
    }
    Err(InstallError::System(
        "user service did not become HTTP ready".into(),
    ))
}

fn enable_extension_checked(system: &dyn System, was_enabled: bool) -> Result<bool, InstallError> {
    let command = system.extension(ExtensionAction::Enable);
    let now_enabled = match system.extension_enabled() {
        Ok(enabled) => enabled,
        Err(probe) => {
            let mut detail = command.as_ref().err().map_or_else(
                || "GNOME extension enable returned without a verifiable state".to_string(),
                ToString::to_string,
            );
            detail.push_str(&format!(
                "; GNOME extension enabled state could not be checked: {probe}"
            ));
            match system.extension_known() {
                Ok(true) => {
                    if let Err(restore) = system.extension(if was_enabled {
                        ExtensionAction::Enable
                    } else {
                        ExtensionAction::Disable
                    }) {
                        detail.push_str(&format!("; GNOME extension rollback failed: {restore}"));
                    } else {
                        detail.push_str(
                            "; prior GNOME extension state restoration requested but not verified",
                        );
                    }
                }
                Ok(false) => detail.push_str("; unknown GNOME extension was not disabled"),
                Err(known) => {
                    detail.push_str(&format!("; GNOME extension catalog check failed: {known}"))
                }
            }
            return Err(InstallError::System(format!("{detail}; repair required")));
        }
    };
    let error = match command {
        Ok(()) if now_enabled => match wait_new_active(system) {
            Ok(()) => return Ok(!was_enabled),
            Err(error) => error,
        },
        Ok(()) => InstallError::System("GNOME extension did not become enabled".into()),
        Err(error) => error,
    };
    if now_enabled != was_enabled {
        if let Err(restore) = system.extension(if was_enabled {
            ExtensionAction::Enable
        } else {
            ExtensionAction::Disable
        }) {
            return Err(InstallError::System(format!(
                "{error}; GNOME extension rollback failed: {restore}; repair required"
            )));
        }
    }
    Err(error)
}

fn wait_new_active(system: &dyn System) -> Result<(), InstallError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    for attempt in 0..40 {
        match system.extension_state()? {
            ExtensionState::Active => return Ok(()),
            ExtensionState::Error => {
                return Err(InstallError::System(
                    "GNOME extension entered ERROR; root setup was not started".into(),
                ))
            }
            ExtensionState::Inactive => {}
        }
        if attempt < 39 && std::time::Instant::now() < deadline {
            system.wait_readiness_poll();
        } else {
            break;
        }
    }
    Err(InstallError::System(
        "GNOME extension did not become ACTIVE".into(),
    ))
}

fn wait_legacy_released(system: &dyn System) -> Result<(), InstallError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    for attempt in 0..40 {
        if system.legacy_extension_state()? == ExtensionState::Inactive
            && !system.toolkit_object_exported()?
        {
            return Ok(());
        }
        if attempt < 39 && std::time::Instant::now() < deadline {
            system.wait_readiness_poll();
        } else {
            break;
        }
    }
    Err(InstallError::System(
        "legacy GNOME extension did not become INACTIVE and release org.mcsr.Toolkit".into(),
    ))
}

fn restore_extension_configuration(
    system: &dyn System,
    old_was_enabled: bool,
    new_was_enabled: bool,
) -> Result<(), InstallError> {
    // Never configure both UUIDs to load in the next Shell session.
    if old_was_enabled {
        if system.extension_enabled()? {
            if !system.extension_known()? {
                return Err(InstallError::System(
                    "new extension is enabled but not catalog-known during rollback".into(),
                ));
            }
            system.extension(ExtensionAction::Disable)?;
        }
        if system.extension_enabled()? {
            return Err(InstallError::System("new extension remained enabled after rollback disable; prior @local state cannot be restored safely".into()));
        }
        if system.legacy_extension_known()? {
            system.legacy_extension(ExtensionAction::Enable)?;
            if !system.legacy_extension_enabled()?
                || system.legacy_extension_state()? != ExtensionState::Active
            {
                return Err(InstallError::System("legacy extension restoration could not be verified; keep the marker and use gnome-extensions to recover @local".into()));
            }
        } else {
            return Err(InstallError::System("legacy extension disappeared during rollback; keep the marker and recover @local manually".into()));
        }
        if system.extension_enabled()? {
            return Err(InstallError::System(
                "new extension became enabled during legacy restoration; manual recovery required"
                    .into(),
            ));
        }
        if new_was_enabled {
            return Err(InstallError::System("both UUIDs were previously enabled; @local was restored and the new UUID was left disabled to avoid another collision; keep the marker for review".into()));
        }
    } else {
        if system.legacy_extension_known()? && system.legacy_extension_enabled()? {
            system.legacy_extension(ExtensionAction::Disable)?;
        }
        if system.extension_known()? && system.extension_enabled()? != new_was_enabled {
            system.extension(if new_was_enabled {
                ExtensionAction::Enable
            } else {
                ExtensionAction::Disable
            })?;
        }
        if system.legacy_extension_enabled()? || system.extension_enabled()? != new_was_enabled {
            return Err(InstallError::System(
                "extension enabled states differ after rollback; manual recovery required".into(),
            ));
        }
    }
    Ok(())
}

fn extension_enabled_from_output(
    output: Result<String, InstallError>,
) -> Result<bool, InstallError> {
    extension_named_in_output(output, EXTENSION_UUID)
}
fn extension_named_in_output(
    output: Result<String, InstallError>,
    uuid: &str,
) -> Result<bool, InstallError> {
    Ok(output?.lines().any(|line| line.trim() == uuid))
}
fn extension_state_from_info(output: String) -> Result<ExtensionState, InstallError> {
    match output
        .lines()
        .find_map(|line| line.trim().strip_prefix("State:").map(str::trim))
    {
        Some("ACTIVE") => Ok(ExtensionState::Active),
        Some("INACTIVE" | "INITIALIZED" | "DISABLED") => Ok(ExtensionState::Inactive),
        Some("ERROR") => Ok(ExtensionState::Error),
        _ => Err(InstallError::Refused(
            "GNOME extension state is unavailable or unexpected; manual review required".into(),
        )),
    }
}

fn toolkit_object_from_introspection(
    result: Result<String, crate::keyd::CommandFailure>,
) -> Result<bool, InstallError> {
    match result {
        Ok(output) if output.lines().any(|line| line.trim() == "interface org.mcsr.Toolkit {") => Ok(true),
        // GNOME Shell can introspect an unexported path as an empty node.
        Ok(output) if output.split_whitespace().eq(["node", "/org/mcsr/Toolkit", "{", "};"]) => Ok(false),
        Ok(_) => Err(InstallError::Refused("GNOME Shell toolkit object did not expose the expected interface; manual review required".into())),
        Err(crate::keyd::CommandFailure::Failed(detail)) if detail.contains("UnknownObject") || detail.contains("Object does not exist at path") => Ok(false),
        Err(error) => Err(InstallError::System(format!("GNOME Shell toolkit object probe failed: {error}"))),
    }
}

#[derive(Serialize, Deserialize)]
struct ExtensionManifest {
    digest: String,
    files: Vec<FileFingerprint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileFingerprint {
    digest: String,
    mode: u32,
    size: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtensionStagingMarker {
    phase: String,
    shell_instance: String,
    desired_digest: String,
    prior_files: Vec<Option<FileFingerprint>>,
    prior_manifest: Option<FileFingerprint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy_was_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    new_was_enabled: Option<bool>,
}

fn retry_marker_bytes(staged: &ExtensionStagingMarker) -> Result<Vec<u8>, InstallError> {
    let mut retry = staged.clone();
    retry.phase = "shell-restart".into();
    marker_bytes(&retry)
}

fn marker_bytes(marker: &ExtensionStagingMarker) -> Result<Vec<u8>, InstallError> {
    let mut bytes = serde_json::to_vec(marker).map_err(|e| InstallError::System(e.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn uncertain_marker_bytes(staged: &ExtensionStagingMarker) -> Result<Vec<u8>, InstallError> {
    let mut uncertain = staged.clone();
    uncertain.phase = "handoff-uncertain".into();
    marker_bytes(&uncertain)
}

fn file_fingerprint(state: &FileState) -> FileFingerprint {
    fingerprint_bytes(&state.bytes, state.mode)
}

fn fingerprint_bytes(bytes: &[u8], mode: u32) -> FileFingerprint {
    FileFingerprint {
        digest: format!("{:x}", Sha256::digest(bytes)),
        mode,
        size: bytes.len() as u64,
    }
}

fn matches_fingerprint(state: Option<&FileState>, prior: Option<&FileFingerprint>) -> bool {
    match (state, prior) {
        (None, None) => true,
        (Some(state), Some(prior)) => file_fingerprint(state) == *prior,
        _ => false,
    }
}

struct ShellProcessSnapshot {
    uid: u32,
    executable: PathBuf,
    starttime: u64,
}

fn shell_identity_from_bus_owner_with(
    response: &str,
    expected_uid: u32,
    read_process: impl FnOnce(u32) -> Option<ShellProcessSnapshot>,
) -> Result<String, InstallError> {
    let pid = response
        .strip_prefix("(uint32 ")
        .and_then(|value| value.strip_suffix(",)"))
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|pid| *pid > 0)
        .ok_or_else(|| InstallError::Refused("invalid GNOME Shell bus owner PID".into()))?;
    let process = read_process(pid).ok_or_else(|| {
        InstallError::Refused("GNOME Shell bus owner process changed or is unavailable".into())
    })?;
    if process.uid != expected_uid
        || process.executable != Path::new("/usr/bin/gnome-shell")
        || process.starttime == 0
    {
        return Err(InstallError::Refused(
            "GNOME Shell bus owner identity differs; manual review required".into(),
        ));
    }
    Ok(format!("{pid}:{}", process.starttime))
}

fn read_shell_process_snapshot(pid: u32) -> Option<ShellProcessSnapshot> {
    use std::os::unix::fs::MetadataExt;
    let path = PathBuf::from(format!("/proc/{pid}"));
    let uid = std::fs::metadata(&path).ok()?.uid();
    let executable = std::fs::read_link(path.join("exe")).ok()?;
    let stat = std::fs::read_to_string(path.join("stat")).ok()?;
    if stat.split_once(' ')?.0.parse::<u32>().ok()? != pid {
        return None;
    }
    let starttime = stat
        .rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(19)?
        .parse::<u64>()
        .ok()?;
    Some(ShellProcessSnapshot {
        uid,
        executable,
        starttime,
    })
}

fn stable_shell_identity_with(
    mut owner: impl FnMut() -> Result<String, InstallError>,
    expected_uid: u32,
    mut read_process: impl FnMut(u32) -> Option<ShellProcessSnapshot>,
) -> Result<String, InstallError> {
    let first = shell_identity_from_bus_owner_with(&owner()?, expected_uid, &mut read_process)?;
    let second = shell_identity_from_bus_owner_with(&owner()?, expected_uid, &mut read_process)?;
    if first != second {
        return Err(InstallError::Refused(
            "GNOME Shell bus owner changed during inspection; rerun repair".into(),
        ));
    }
    Ok(first)
}

fn extension_digest_from_fingerprints(
    paths: impl Iterator<Item = PathBuf>,
    fingerprints: &[FileFingerprint],
) -> String {
    let mut hash = Sha256::new();
    for (path, fingerprint) in paths.zip(fingerprints) {
        hash.update(path.as_os_str().as_encoded_bytes());
        hash.update(fingerprint.size.to_le_bytes());
        hash.update(fingerprint.mode.to_le_bytes());
        hash.update(fingerprint.digest.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn extension_digest(files: &[(PathBuf, Vec<u8>)]) -> String {
    let fingerprints: Vec<_> = files
        .iter()
        .map(|(_, bytes)| fingerprint_bytes(bytes, 0o644))
        .collect();
    extension_digest_from_fingerprints(files.iter().map(|(path, _)| path.clone()), &fingerprints)
}

fn manifest_matches_fingerprints(
    record: &FileState,
    files: &[(PathBuf, Vec<u8>)],
    fingerprints: &[FileFingerprint],
) -> bool {
    if record.mode != 0o600 || fingerprints.len() != files.len() {
        return false;
    }
    serde_json::from_slice::<ExtensionManifest>(&record.bytes).is_ok_and(|manifest| {
        manifest.files == fingerprints
            && manifest.digest
                == extension_digest_from_fingerprints(
                    files.iter().map(|(path, _)| path.clone()),
                    fingerprints,
                )
    })
}

fn manifest_bytes(files: &[(PathBuf, Vec<u8>)]) -> Vec<u8> {
    let manifest = ExtensionManifest {
        digest: extension_digest(files),
        files: files
            .iter()
            .map(|(_, bytes)| fingerprint_bytes(bytes, 0o644))
            .collect(),
    };
    let mut bytes = serde_json::to_vec(&manifest).expect("fixed extension manifest serializes");
    bytes.push(b'\n');
    bytes
}

fn shell_restart_error() -> InstallError {
    InstallError::Staged(
        "GNOME extension staged; installation is incomplete. Log out of the GNOME Wayland session, log back in, then run mcsr-toolkit repair.".into(),
    )
}

fn stage_extension(
    system: &dyn System,
    env: &Environment,
    files: &[(PathBuf, Vec<u8>)],
    prior: &[(PathBuf, Option<FileState>)],
    prior_manifest: Option<FileState>,
    staging_marker: Option<ExtensionStagingMarker>,
) -> Result<(), InstallError> {
    let marker = env.marker_path();
    let shell_instance = system.shell_instance()?;
    if shell_instance.is_empty() {
        return Err(InstallError::Refused(
            "cannot identify current GNOME Shell instance for safe staged install".into(),
        ));
    }
    let mut phase = match staging_marker {
        Some(phase) => phase,
        None => {
            let phase = ExtensionStagingMarker {
                phase: "extension-staging".into(),
                shell_instance,
                desired_digest: extension_digest(files),
                prior_files: if prior_manifest.is_none() {
                    vec![None; files.len()]
                } else {
                    prior
                        .iter()
                        .map(|(_, state)| state.as_ref().map(file_fingerprint))
                        .collect()
                },
                prior_manifest: prior_manifest.as_ref().map(file_fingerprint),
                legacy_was_enabled: None,
                new_was_enabled: None,
            };
            let mut serialized =
                serde_json::to_vec(&phase).map_err(|e| InstallError::System(e.to_string()))?;
            serialized.push(b'\n');
            system.write_user(&marker, &serialized, 0o600)?;
            phase
        }
    };
    let result = (|| {
        for ((path, bytes), (_, state)) in files.iter().zip(prior) {
            if state
                .as_ref()
                .is_some_and(|state| state.bytes == *bytes && state.mode == 0o644)
            {
                continue;
            }
            system.write_user(path, bytes, 0o644)?;
        }
        // A Shell restart during file replacement cannot establish that it loaded the final set.
        phase.phase = "shell-restart".into();
        phase.shell_instance = system.shell_instance()?;
        let mut serialized =
            serde_json::to_vec(&phase).map_err(|e| InstallError::System(e.to_string()))?;
        serialized.push(b'\n');
        system.write_user(&marker, &serialized, 0o600)
    })();
    if let Err(error) = result {
        return Err(InstallError::System(format!(
            "{error}; extension staging incomplete; run repair"
        )));
    }
    Err(shell_restart_error())
}

pub fn install(system: &dyn System, artifact: &Artifact) -> Result<(), InstallError> {
    install_inner(system, artifact, false)
}
pub fn repair(system: &dyn System, artifact: &Artifact) -> Result<(), InstallError> {
    install_inner(system, artifact, true)
}
fn install_inner(
    system: &dyn System,
    artifact: &Artifact,
    repairing: bool,
) -> Result<(), InstallError> {
    let _install_guard = system.lock_install()?;
    if repairing {
        let env = system.environment()?;
        let marker = env.marker_path();
        if let Some(state) = system.read(&marker)? {
            validate_marker_state(&state)?;
            if state.bytes == b"{\"phase\":\"uninstall\"}\n" {
                let mut root_remains = false;
                for path in [
                    KEYD_POLICY_PATH,
                    MACCEL_POLICY_PATH,
                    SERVICE_PATH,
                    HELPER_PATH,
                ] {
                    root_remains |= system.read(Path::new(path))?.is_some();
                }
                if root_remains
                    || system.read(&env.unit_path())?.is_some()
                    || system.service_active()?
                    || system.service_enabled()?
                    || system.extension_enabled()?
                {
                    return Err(InstallError::Refused(
                        "uninstall incomplete; retain marker and recovery tools for review".into(),
                    ));
                }
                system.remove_user(&marker)?;
                return Ok(());
            }
        }
    }
    let report = preflight(system);
    if !report.ready() {
        return Err(InstallError::Refused(report.problems.join("; ")));
    }
    if report.repair_pending && !repairing {
        return Err(InstallError::Refused(
            "interrupted installation; run repair".into(),
        ));
    }
    let env = system.environment()?;
    system.ensure_private_state(&env.data_home)?;
    let rules = policy(&env.login);
    let helper_exists = verified_helper(system, artifact)?;
    let policies_exist = system.policies_match(&rules)?;
    let extension = archive_files(&env)?;
    let expected_manifest = manifest_bytes(&extension);
    let manifest = env
        .data_home
        .join("mcsr-toolkit/rust-v1/extension-manifest.json");
    let prior_manifest = system.read(&manifest)?;
    let extension_prior: Vec<_> = extension
        .iter()
        .map(|(path, _)| Ok((path.clone(), system.read(path)?)))
        .collect::<Result<_, InstallError>>()?;
    let desktop_uid = unsafe { libc::geteuid() };
    if extension_prior
        .iter()
        .any(|(_, state)| state.as_ref().is_some_and(|state| state.uid != desktop_uid))
        || prior_manifest
            .as_ref()
            .is_some_and(|state| state.uid != desktop_uid)
    {
        return Err(InstallError::Refused(
            "GNOME extension files or lineage manifest have a foreign owner; manual review required"
                .into(),
        ));
    }
    let extension_exact = extension
        .iter()
        .zip(&extension_prior)
        .all(|((_, bytes), (_, state))| {
            state
                .as_ref()
                .is_some_and(|state| state.bytes == *bytes && state.mode == 0o644)
        });
    let prior_fingerprints: Option<Vec<_>> = extension_prior
        .iter()
        .map(|(_, state)| state.as_ref().map(file_fingerprint))
        .collect();
    let marker = env.marker_path();
    let prior_marker = system.read(&marker)?;
    if let Some(state) = &prior_marker {
        validate_marker_state(state)?;
    }
    let stage_marker = match prior_marker.as_ref() {
        None => None,
        Some(state)
            if state.bytes == b"{\"phase\":\"repair\"}\n"
                || state.bytes == b"{\"phase\":\"staging\"}\n" =>
        {
            None
        }
        Some(state) => {
            let staged =
                serde_json::from_slice::<ExtensionStagingMarker>(&state.bytes).map_err(|_| {
                    InstallError::Refused("invalid install marker; manual review required".into())
                })?;
            if ![
                "extension-staging",
                "shell-restart",
                "legacy-handoff-restart",
                "extension-handoff",
            ]
            .contains(&staged.phase.as_str())
                || staged.shell_instance.is_empty()
                || staged.desired_digest != extension_digest(&extension)
                || staged.prior_files.len() != extension.len()
            {
                return Err(InstallError::Refused(
                    "invalid install marker; manual review required".into(),
                ));
            }
            Some(staged)
        }
    };
    if let Some(staged) = &stage_marker {
        let marker_prior: Option<Vec<_>> = staged.prior_files.iter().cloned().collect();
        let provenance = match (
            prior_manifest.as_ref(),
            staged.prior_manifest.as_ref(),
            marker_prior.as_ref(),
        ) {
            (None, None, None) => true,
            (Some(record), Some(prior_record), Some(fingerprints)) => {
                matches_fingerprint(Some(record), Some(prior_record))
                    && manifest_matches_fingerprints(record, &extension, fingerprints)
            }
            _ => false,
        };
        let completed_manifest = [
            "shell-restart",
            "extension-handoff",
            "legacy-handoff-restart",
        ]
        .contains(&staged.phase.as_str())
            && extension_exact
            && prior_manifest
                .as_ref()
                .is_some_and(|record| record.bytes == expected_manifest && record.mode == 0o600);
        let valid_files = extension
            .iter()
            .zip(&extension_prior)
            .zip(&staged.prior_files)
            .all(|(((_, desired), (_, state)), prior)| {
                matches_fingerprint(state.as_ref(), prior.as_ref())
                    || state
                        .as_ref()
                        .is_some_and(|state| state.bytes == *desired && state.mode == 0o644)
            });
        if (!provenance && !completed_manifest) || !valid_files {
            return Err(InstallError::Refused(
                "interrupted GNOME extension staging contains unrelated changes; manual review required"
                    .into(),
            ));
        }
    }
    let lineage_matches = match (&prior_manifest, &prior_fingerprints) {
        (Some(record), Some(fingerprints)) => {
            manifest_matches_fingerprints(record, &extension, fingerprints)
        }
        _ => false,
    };
    if stage_marker.is_none() && prior_manifest.is_some() && !lineage_matches {
        return Err(InstallError::Refused(
            "GNOME extension lineage manifest differs from installed files; manual review required"
                .into(),
        ));
    }
    if extension_prior
        .iter()
        .zip(&extension)
        .any(|((_, old), (_, bytes))| {
            old.as_ref()
                .is_some_and(|old| old.bytes != *bytes || old.mode != 0o644)
        })
        && !lineage_matches
        && stage_marker.is_none()
    {
        return Err(InstallError::Refused(
            "existing GNOME extension differs without verified installer lineage; manual review required".into(),
        ));
    }
    if stage_marker
        .as_ref()
        .is_some_and(|marker| marker.phase == "extension-staging")
    {
        return stage_extension(
            system,
            &env,
            &extension,
            &extension_prior,
            prior_manifest,
            stage_marker,
        );
    }
    let current_shell = if stage_marker.is_some() {
        Some(system.shell_instance()?)
    } else {
        None
    };
    let known = system.extension_known()?;
    if let Some(staged) = &stage_marker {
        let after_catalog = system.shell_instance()?;
        if current_shell.as_deref() != Some(&after_catalog) {
            return Err(InstallError::Refused(
                "GNOME Shell changed during extension catalog check; rerun repair".into(),
            ));
        }
        if !extension_exact {
            return Err(InstallError::Refused(
                "staged GNOME extension files or lineage changed; manual review required".into(),
            ));
        }
        if !known || (staged.phase != "extension-handoff" && after_catalog == staged.shell_instance)
        {
            return Err(shell_restart_error());
        }
    } else if !extension_exact || !known {
        return stage_extension(
            system,
            &env,
            &extension,
            &extension_prior,
            prior_manifest,
            None,
        );
    }
    let desired_extension_digest = extension_digest(&extension);
    let mut desired: Vec<(PathBuf, Vec<u8>, u32)> = vec![(env.unit_path(), unit().to_vec(), 0o644)];
    desired.extend(extension.into_iter().map(|(p, b)| (p, b, 0o644)));
    desired.push((manifest, expected_manifest, 0o600));
    let mut prior = Vec::new();
    let service_exists = verified_executable(system, Path::new(SERVICE_PATH), artifact)?;
    let mut identical = service_exists && helper_exists && policies_exist && !report.repair_pending;
    let legacy_known = system.legacy_extension_known()?;
    let legacy_was_enabled = if legacy_known {
        system.legacy_extension_enabled()?
    } else {
        false
    };
    let legacy_was_active =
        legacy_known && system.legacy_extension_state()? == ExtensionState::Active;
    let new_was_enabled = system.extension_enabled()?;
    if legacy_was_enabled || legacy_was_active {
        identical = false;
    }
    for (path, bytes, mode) in &desired {
        let old = system.read(path)?;
        if old
            .as_ref()
            .is_none_or(|s| s.bytes != *bytes || s.mode != *mode)
        {
            identical = false;
        }
        prior.push((path.clone(), old));
    }
    if identical {
        let prior_service_enabled = system.service_enabled()?;
        let prior_service_active = system.service_active()?;
        let result = (|| {
            if system.extension_state()? != ExtensionState::Active {
                enable_extension_checked(system, new_was_enabled)?;
            }
            if !system.service_enabled()? {
                system.service(ServiceAction::Enable)?;
            }
            if !system.service_active()? {
                system.service(ServiceAction::Reload)?;
                verify_resolved_command(system)?;
                system.service(ServiceAction::Start)?;
            }
            verify_running_service(system)
        })();
        if let Err(error) = result {
            let mut rollback_errors = Vec::new();
            if !prior_service_active {
                if let Err(stop) = system.service(ServiceAction::Stop) {
                    rollback_errors.push(stop.to_string());
                }
            }
            if let Err(restore) = restore_extension_configuration(system, false, new_was_enabled) {
                rollback_errors.push(restore.to_string());
            }
            if let Err(restore) = system.service(if prior_service_enabled {
                ServiceAction::Enable
            } else {
                ServiceAction::Disable
            }) {
                rollback_errors.push(restore.to_string());
            }
            match system.service_enabled() {
                Ok(enabled) if enabled == prior_service_enabled => {}
                Ok(_) => {
                    rollback_errors.push("prior service enabled state could not be restored".into())
                }
                Err(probe) => rollback_errors.push(probe.to_string()),
            }
            if !rollback_errors.is_empty() {
                let marker_result = (|| {
                    let uncertain = ExtensionStagingMarker {
                        phase: "handoff-uncertain".into(),
                        shell_instance: system.shell_instance()?,
                        desired_digest: desired_extension_digest.clone(),
                        prior_files: if prior_manifest.is_some() {
                            extension_prior
                                .iter()
                                .map(|(_, state)| state.as_ref().map(file_fingerprint))
                                .collect()
                        } else {
                            vec![None; extension_prior.len()]
                        },
                        prior_manifest: prior_manifest.as_ref().map(file_fingerprint),
                        legacy_was_enabled: Some(false),
                        new_was_enabled: Some(new_was_enabled),
                    };
                    system.write_user(&env.marker_path(), &marker_bytes(&uncertain)?, 0o600)
                })();
                if let Err(marker_error) = marker_result {
                    rollback_errors.push(format!("repair marker failed: {marker_error}"));
                }
                return Err(InstallError::System(format!(
                    "{error}; rollback failed: {}; manual recovery required",
                    rollback_errors.join("; ")
                )));
            }
            return Err(error);
        }
        return Ok(());
    }
    for rule in &rules {
        system.validate_sudoers(&rule.bytes)?;
    }
    let was_active = system.service_active()?;
    let was_enabled = system.service_enabled()?;
    let extension_was_enabled = stage_marker
        .as_ref()
        .and_then(|marker| marker.new_was_enabled)
        .unwrap_or(new_was_enabled);
    let legacy_restore_enabled = stage_marker
        .as_ref()
        .and_then(|marker| marker.legacy_was_enabled)
        .unwrap_or(legacy_was_enabled || legacy_was_active);
    let marker = env.marker_path();
    if stage_marker
        .as_ref()
        .is_some_and(|stage| stage.phase == "legacy-handoff-restart")
    {
        if let Err(error) = wait_new_active(system) {
            let restore = restore_extension_configuration(
                system,
                legacy_restore_enabled,
                extension_was_enabled,
            );
            let bytes = if restore.is_err() {
                uncertain_marker_bytes(stage_marker.as_ref().expect("sticky marker"))?
            } else {
                retry_marker_bytes(stage_marker.as_ref().expect("sticky marker"))?
            };
            system.write_user(&marker, &bytes, 0o600)?;
            return Err(match restore {
                Ok(()) => InstallError::System(format!("{error}; prior @local state restored; review the GNOME extension error before retrying repair")),
                Err(restore) => InstallError::System(format!("{error}; rollback uncertain: {restore}; retain the marker and recover @local manually")),
            });
        }
    } else if system.extension_state()? == ExtensionState::Error {
        let mut staged = stage_marker
            .clone()
            .unwrap_or_else(|| ExtensionStagingMarker {
                phase: "legacy-handoff-restart".into(),
                shell_instance: String::new(),
                desired_digest: desired_extension_digest,
                prior_files: if prior_manifest.is_some() {
                    extension_prior
                        .iter()
                        .map(|(_, state)| state.as_ref().map(file_fingerprint))
                        .collect()
                } else {
                    vec![None; extension_prior.len()]
                },
                prior_manifest: prior_manifest.as_ref().map(file_fingerprint),
                legacy_was_enabled: None,
                new_was_enabled: None,
            });
        staged.phase = "legacy-handoff-restart".into();
        staged.shell_instance = system.shell_instance()?;
        staged.legacy_was_enabled = Some(legacy_restore_enabled);
        staged.new_was_enabled = Some(extension_was_enabled);
        let mut bytes =
            serde_json::to_vec(&staged).map_err(|e| InstallError::System(e.to_string()))?;
        bytes.push(b'\n');
        system.write_user(&marker, &bytes, 0o600)?;
        let staged_result = (|| {
            if legacy_known && (legacy_was_enabled || legacy_was_active) {
                system.legacy_extension(ExtensionAction::Disable)?;
                wait_legacy_released(system)?;
            }
            let enable_result = if !system.extension_enabled()? {
                Some(system.extension(ExtensionAction::Enable))
            } else {
                None
            };
            if !system.extension_enabled()? {
                return Err(enable_result.and_then(Result::err).unwrap_or_else(|| {
                    InstallError::System(
                        "new GNOME extension could not be configured for next login".into(),
                    )
                }));
            }
            Ok(())
        })();
        if let Err(error) = staged_result {
            let restore = restore_extension_configuration(
                system,
                legacy_restore_enabled,
                extension_was_enabled,
            );
            if restore.is_err() {
                system.write_user(&marker, &uncertain_marker_bytes(&staged)?, 0o600)?;
            } else if let Some(previous) = &prior_marker {
                system.write_user(&marker, &previous.bytes, previous.mode)?;
            } else {
                system.write_user(&marker, b"{\"phase\":\"repair\"}\n", 0o600)?;
            }
            return Err(match restore {
                Ok(()) => error,
                Err(restore) => InstallError::System(format!(
                    "{error}; rollback uncertain: {restore}; keep recovery tools"
                )),
            });
        }
        return Err(InstallError::Staged("GNOME extension ERROR is sticky in this Shell session. The old @local UUID is disabled and the new UUID alone is configured to load. Log out, log back in, then run mcsr-toolkit repair; root setup has not started.".into()));
    }
    let mut handoff_marker = stage_marker
        .clone()
        .unwrap_or_else(|| ExtensionStagingMarker {
            phase: "extension-handoff".into(),
            shell_instance: String::new(),
            desired_digest: desired_extension_digest,
            prior_files: if prior_manifest.is_some() {
                extension_prior
                    .iter()
                    .map(|(_, state)| state.as_ref().map(file_fingerprint))
                    .collect()
            } else {
                vec![None; extension_prior.len()]
            },
            prior_manifest: prior_manifest.as_ref().map(file_fingerprint),
            legacy_was_enabled: None,
            new_was_enabled: None,
        });
    handoff_marker.phase = "extension-handoff".into();
    handoff_marker.shell_instance = system.shell_instance()?;
    handoff_marker.legacy_was_enabled = Some(legacy_restore_enabled);
    handoff_marker.new_was_enabled = Some(extension_was_enabled);
    system.write_user(&marker, &marker_bytes(&handoff_marker)?, 0o600)?;
    let mut root_setup: Option<Box<dyn RootSetup + '_>> = None;
    let mut switching_service = false;
    let mut extension_handoff_started = stage_marker.as_ref().is_some_and(|stage| {
        ["legacy-handoff-restart", "extension-handoff"].contains(&stage.phase.as_str())
    });
    let result = (|| {
        if legacy_known && (legacy_was_enabled || legacy_was_active) {
            extension_handoff_started = true;
            system.legacy_extension(ExtensionAction::Disable)?;
            wait_legacy_released(system)?;
        }
        if system.extension_state()? != ExtensionState::Active {
            extension_handoff_started = true;
            enable_extension_checked(system, new_was_enabled)?;
        } else {
            wait_new_active(system)?;
        }
        for (path, bytes, mode) in &desired {
            system.write_user(path, bytes, *mode)?;
        }
        root_setup = Some(system.begin_root_setup(artifact)?);
        if !verified_executable(system, Path::new(SERVICE_PATH), artifact)?
            || !verified_helper(system, artifact)?
        {
            return Err(InstallError::System(
                "root executable verification failed".into(),
            ));
        }
        switching_service = true;
        system.service(ServiceAction::Stop)?;
        system.service(ServiceAction::Reload)?;
        verify_resolved_command(system)?;
        system.service(ServiceAction::Enable)?;
        system.service(ServiceAction::Start)?;
        verify_running_service(system)?;
        root_setup.take().expect("root setup started").commit()?;
        system.remove_user(&marker)?;
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_errors = Vec::new();
        let mut attempt = |result: Result<(), InstallError>| {
            if let Err(e) = result {
                rollback_errors.push(e.to_string());
            }
        };
        if switching_service {
            attempt(system.service(ServiceAction::Stop));
            if !was_enabled {
                attempt(system.service(ServiceAction::Disable));
            }
        }
        if extension_handoff_started {
            attempt(restore_extension_configuration(
                system,
                legacy_restore_enabled,
                extension_was_enabled,
            ));
        }
        attempt(restore_user(system, &prior));
        if switching_service {
            attempt(system.service(ServiceAction::Reload));
            if was_enabled {
                attempt(system.service(ServiceAction::Enable));
            }
            attempt(system.service_enabled().and_then(|enabled| {
                if enabled == was_enabled {
                    Ok(())
                } else {
                    Err(InstallError::System(
                        "prior service enabled state could not be restored".into(),
                    ))
                }
            }));
        }
        if let Some(setup) = root_setup.take() {
            attempt(setup.abort());
        }
        if switching_service && was_active {
            attempt(system.service(ServiceAction::Start));
        }
        let marker_result = if !rollback_errors.is_empty() {
            uncertain_marker_bytes(&handoff_marker)
        } else if let Some(staged) = &stage_marker {
            if staged.phase == "legacy-handoff-restart" {
                retry_marker_bytes(staged)
            } else {
                Ok(prior_marker
                    .as_ref()
                    .expect("staged marker was read")
                    .bytes
                    .clone())
            }
        } else {
            Ok(b"{\"phase\":\"repair\"}\n".to_vec())
        };
        if let Err(marker_error) =
            marker_result.and_then(|bytes| system.write_user(&marker, &bytes, 0o600))
        {
            rollback_errors.push(format!("repair marker failed: {marker_error}"));
        }
        return if rollback_errors.is_empty() {
            Err(error)
        } else {
            Err(InstallError::System(format!(
                "{error}; rollback failed: {}; repair required",
                rollback_errors.join("; ")
            )))
        };
    }
    Ok(())
}

fn check_uninstall_records(system: &dyn System, env: &Environment) -> Result<(), InstallError> {
    if system
        .read(&env.data_home.join("mcsr-toolkit/rust-v1/keyd.json"))?
        .is_some()
    {
        return Err(InstallError::Refused(
            "keyd backup lineage remains; use the toolkit Restore workflow and verify original /etc/keyd before uninstall".into(),
        ));
    }
    for path in [
        env.data_home.join("mcsr-toolkit/rust-v1/mutation.json"),
        env.data_home
            .join("mcsr-toolkit/rust-v1/keyd-uncertain.json"),
    ] {
        if system.read(&path)?.is_some() {
            return Err(InstallError::Refused(format!(
                "recovery record pending: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn refuse_after_stop(system: &dyn System, was_active: bool, refusal: InstallError) -> InstallError {
    if !was_active {
        return refusal;
    }
    let restart = system.service(ServiceAction::Start).and_then(|()| {
        if system.service_active()? {
            Ok(())
        } else {
            Err(InstallError::System(
                "prior user service did not become active".into(),
            ))
        }
    });
    match restart {
        Ok(()) => refusal,
        Err(error) => InstallError::System(format!(
            "{refusal}; prior service restart failed: {error}; retain the helper and policy for recovery"
        )),
    }
}

pub fn uninstall(system: &dyn System) -> Result<(), InstallError> {
    let _install_guard = system.lock_install()?;
    let env = system.environment()?;
    check_uninstall_records(system, &env)?;
    let marker = env.marker_path();
    if system.read(&marker)?.is_some() {
        return Err(InstallError::Refused(
            "interrupted installation; run repair before uninstall".into(),
        ));
    }
    if env.keyd_recovery_pending {
        return Err(InstallError::Refused(
            "keyd recovery pending; keep the service and helper available for Restore".into(),
        ));
    }
    let was_active = system.service_active()?;
    let was_enabled = system.service_enabled()?;
    let extension_was_enabled = system.extension_enabled()?;
    let prior_unit = system.read(&env.unit_path())?;
    system.service(ServiceAction::Stop)?;
    let validation = (|| {
        system.recover_input()?;
        let env = system.environment()?;
        check_uninstall_records(system, &env)?;
        if env.keyd_recovery_pending || env.maccel_recovery_pending {
            return Err(InstallError::Refused(
                "input recovery pending; keep installed helpers".into(),
            ));
        }
        let service = system.read(Path::new(SERVICE_PATH))?;
        let helper = system.read(Path::new(HELPER_PATH))?;
        for file in [&service, &helper].into_iter().flatten() {
            if file.uid != 0 || file.mode != 0o755 {
                return Err(InstallError::Refused(
                    "installed executable ownership or mode differs; manual review required".into(),
                ));
            }
        }
        if let (Some(service), Some(helper)) = (&service, &helper) {
            if Sha256::digest(&service.bytes) != Sha256::digest(&helper.bytes) {
                return Err(InstallError::Refused(
                    "installed executable copies differ; manual review required".into(),
                ));
            }
        }
        let rules = policy(&env.login);
        system.policies_match(&rules)?;
        Ok(env)
    })();
    let env = match validation {
        Ok(env) => env,
        Err(error) => return Err(refuse_after_stop(system, was_active, error)),
    };
    if let Err(error) = system.write_user(&marker, b"{\"phase\":\"uninstall\"}\n", 0o600) {
        return Err(refuse_after_stop(system, was_active, error));
    }
    let mut root_removal: Option<Box<dyn RootSetup + '_>> = None;
    let mut committing = false;
    let result = (|| {
        root_removal = Some(system.begin_root_uninstall()?);
        system.extension(ExtensionAction::Disable)?;
        system.service(ServiceAction::Disable)?;
        system.remove_user(&env.unit_path())?;
        system.service(ServiceAction::Reload)?;
        committing = true;
        root_removal
            .take()
            .expect("root uninstall started")
            .commit()?;
        system.remove_user(&marker)?;
        Ok(())
    })();
    if let Err(error) = result {
        if committing && !matches!(error, InstallError::RootAborted(_)) {
            return Err(InstallError::System(format!(
                "{error}; uninstall may be partially committed; repair required"
            )));
        }
        let mut errors = Vec::new();
        let mut attempt = |result: Result<(), InstallError>| {
            if let Err(e) = result {
                errors.push(e.to_string());
            }
        };
        if let Some(removal) = root_removal.take() {
            attempt(removal.abort());
        }
        if let Some(unit) = prior_unit {
            attempt(system.write_user(&env.unit_path(), &unit.bytes, unit.mode));
        }
        attempt(system.service(ServiceAction::Reload));
        attempt(system.service(if was_enabled {
            ServiceAction::Enable
        } else {
            ServiceAction::Disable
        }));
        attempt(system.extension(if extension_was_enabled {
            ExtensionAction::Enable
        } else {
            ExtensionAction::Disable
        }));
        if was_active {
            attempt(system.service(ServiceAction::Start));
        }
        return if errors.is_empty() {
            Err(error)
        } else {
            Err(InstallError::System(format!(
                "{error}; uninstall rollback failed: {}; repair required",
                errors.join("; ")
            )))
        };
    }
    Ok(())
}

/// Production adapter. Tests use a temporary-root `System`; constructing this does not mutate.
pub struct RealSystem {
    env: std::sync::Mutex<Environment>,
    policy_status: PolicyStatus,
}
fn sync_parent_with(
    path: &Path,
    mut sync: impl FnMut(&Path) -> std::io::Result<()>,
) -> Result<(), InstallError> {
    let parent = path
        .parent()
        .ok_or_else(|| InstallError::Refused("missing parent".into()))?;
    // A newly created marker directory also needs its entry persisted in each ancestor.
    for directory in parent.ancestors() {
        sync(directory)
            .map_err(|e| InstallError::System(format!("sync {}: {e}", directory.display())))?;
    }
    Ok(())
}
fn sync_parent(path: &Path) -> Result<(), InstallError> {
    sync_parent_with(path, |parent| std::fs::File::open(parent)?.sync_all())
}
struct RealRootSetup {
    child: std::process::Child,
    input: Option<std::process::ChildStdin>,
    output: std::process::ChildStdout,
    finished: bool,
}
impl RealRootSetup {
    fn line(&mut self) -> Result<String, InstallError> {
        use std::os::fd::AsRawFd;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let mut line = Vec::new();
        while line.len() < 32 {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(InstallError::System("authenticated setup timed out".into()));
            }
            let mut descriptor = libc::pollfd {
                fd: self.output.as_raw_fd(),
                events: libc::POLLIN | libc::POLLHUP,
                revents: 0,
            };
            let ready = unsafe {
                libc::poll(
                    &mut descriptor,
                    1,
                    remaining.as_millis().min(i32::MAX as u128) as i32,
                )
            };
            if ready == 0 {
                return Err(InstallError::System("authenticated setup timed out".into()));
            }
            if ready < 0 {
                return Err(InstallError::System(
                    std::io::Error::last_os_error().to_string(),
                ));
            }
            let mut byte = [0];
            let count = self
                .output
                .read(&mut byte)
                .map_err(|e| InstallError::System(e.to_string()))?;
            if count == 0 {
                return Err(InstallError::System(
                    "authenticated setup closed before response".into(),
                ));
            }
            line.push(byte[0]);
            if byte[0] == b'\n' {
                return String::from_utf8(line).map_err(|e| InstallError::System(e.to_string()));
            }
        }
        Err(InstallError::Refused(
            "invalid authenticated setup response".into(),
        ))
    }
    fn send(&mut self, decision: &str) -> Result<(), InstallError> {
        use std::io::Write;
        self.input
            .as_mut()
            .ok_or_else(|| InstallError::System("setup input closed".into()))?
            .write_all(decision.as_bytes())
            .map_err(|e| InstallError::System(e.to_string()))?;
        self.input.take();
        Ok(())
    }
    fn wait(&mut self) -> Result<std::process::ExitStatus, InstallError> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|e| InstallError::System(e.to_string()))?
            {
                return Ok(status);
            }
            if std::time::Instant::now() >= deadline {
                return Err(InstallError::System(
                    "authenticated setup exit timed out".into(),
                ));
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
}
impl Drop for RealRootSetup {
    fn drop(&mut self) {
        self.input.take();
        if !self.finished {
            let group = self.child.id() as i32;
            if group > 0 && group != unsafe { libc::getpgrp() } {
                unsafe { libc::kill(-group, libc::SIGKILL) };
            }
            let _ = self.child.wait();
        }
    }
}
impl RootSetup for RealRootSetup {
    fn commit(mut self: Box<Self>) -> Result<(), InstallError> {
        self.send("COMMIT\n")?;
        let response = self.line()?;
        let waited = self.wait().map(|status| status.success());
        self.finished = waited.is_ok();
        classify_root_commit_response(&response, waited)
    }
    fn abort(mut self: Box<Self>) -> Result<(), InstallError> {
        self.send("ABORT\n")?;
        if self.line()?.as_str() != "ABORTED\n" {
            return Err(InstallError::System(
                "authenticated setup did not abort".into(),
            ));
        }
        self.wait()?;
        self.finished = true;
        Ok(())
    }
}
fn start_root_transaction(uninstall: bool) -> Result<Box<dyn RootSetup>, InstallError> {
    use std::os::unix::process::CommandExt;
    let mut command = std::process::Command::new("/usr/bin/pkexec");
    command.args([HELPER_PATH, "privileged", "setup"]);
    if uninstall {
        command.arg("uninstall");
    }
    let mut child = command
        .process_group(0)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| InstallError::System(format!("authenticated root transaction: {e}")))?;
    let input = child
        .stdin
        .take()
        .ok_or_else(|| InstallError::System("root transaction stdin unavailable".into()))?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| InstallError::System("root transaction stdout unavailable".into()))?;
    let mut transaction = RealRootSetup {
        child,
        input: Some(input),
        output,
        finished: false,
    };
    if transaction.line()?.as_str() != "READY\n" {
        return Err(InstallError::System(
            "authenticated root transaction did not become ready".into(),
        ));
    }
    Ok(Box::new(transaction))
}
fn run_status_command(
    program: &Path,
    args: &[&str],
    deadline: std::time::Duration,
) -> Result<String, InstallError> {
    let mut command = vec![program.to_string_lossy().into_owned()];
    command.extend(args.iter().map(|arg| (*arg).to_owned()));
    let output = crate::keyd::run_command(&command, &[], deadline)
        .map_err(|error| InstallError::System(error.to_string()))?;
    Ok(output.stdout.trim().to_owned())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PolicyStatus {
    Match,
    Missing,
    Mismatch,
    Unknown,
}
fn cached_policy_bytes_match(
    read: Result<Vec<u8>, std::io::Error>,
    expected: &[u8],
    status: PolicyStatus,
) -> Result<bool, InstallError> {
    match read {
        Ok(bytes) if bytes == expected => Ok(true),
        Ok(_) => Err(InstallError::Refused(
            "existing policy differs; manual review required".into(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            Ok(status == PolicyStatus::Match)
        }
        Err(error) => Err(InstallError::System(error.to_string())),
    }
}
struct RootStatusSnapshot {
    keyd_pending: bool,
    maccel_pending: bool,
    policy: PolicyStatus,
}
fn root_status_snapshot(
    keyd_probe: Result<(), std::io::Error>,
    maccel_probe: Result<(), std::io::Error>,
    policy_probe: Result<(), std::io::Error>,
    authenticated: impl FnOnce() -> Result<String, InstallError>,
) -> RootStatusSnapshot {
    let present = |probe: &Result<(), std::io::Error>| !matches!(probe, Err(error) if error.kind() == std::io::ErrorKind::NotFound);
    let keyd = present(&keyd_probe);
    let maccel = present(&maccel_probe);
    let policy = present(&policy_probe);
    let status = if keyd || maccel || policy {
        authenticated().ok()
    } else {
        None
    };
    let fields = status.as_deref().and_then(|s| {
        let (keyd, rest) = s.split_once(" maccel=")?;
        let (maccel, policy) = rest.split_once(" policies=")?;
        Some((keyd, maccel, policy))
    });
    RootStatusSnapshot {
        keyd_pending: keyd && fields.is_none_or(|(value, _, _)| value != "keyd=clear"),
        maccel_pending: maccel && fields.is_none_or(|(_, value, _)| value != "clear"),
        policy: if !policy {
            PolicyStatus::Missing
        } else {
            match fields.map(|(_, _, value)| value) {
                Some("match") => PolicyStatus::Match,
                Some("missing") => PolicyStatus::Missing,
                Some("mismatch") => PolicyStatus::Mismatch,
                _ => PolicyStatus::Unknown,
            }
        },
    }
}
#[cfg(test)]
fn root_recovery_snapshot(
    keyd_probe: Result<(), std::io::Error>,
    maccel_probe: Result<(), std::io::Error>,
    authenticated: impl FnOnce() -> Result<String, InstallError>,
) -> (bool, bool) {
    let result = root_status_snapshot(
        keyd_probe,
        maccel_probe,
        Err(std::io::ErrorKind::NotFound.into()),
        authenticated,
    );
    (result.keyd_pending, result.maccel_pending)
}
fn http_probe(address: std::net::SocketAddr, timeout: std::time::Duration) -> bool {
    use std::io::{Read as _, Write as _};
    let started = std::time::Instant::now();
    let Ok(mut stream) = std::net::TcpStream::connect_timeout(&address, timeout) else {
        return false;
    };
    let request =
        format!("GET /api/session HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    if stream.set_write_timeout(Some(timeout)).is_err()
        || stream.write_all(request.as_bytes()).is_err()
    {
        return false;
    }
    let mut bytes = [0u8; 128];
    let mut used = 0;
    while used < bytes.len() {
        let Some(remaining) = timeout.checked_sub(started.elapsed()) else {
            return false;
        };
        if remaining.is_zero() || stream.set_read_timeout(Some(remaining)).is_err() {
            return false;
        }
        let Ok(count) = stream.read(&mut bytes[used..]) else {
            return false;
        };
        if count == 0 {
            return false;
        }
        used += count;
        if let Some(end) = bytes[..used].windows(2).position(|pair| pair == b"\r\n") {
            return bytes[..end].starts_with(b"HTTP/1.1 200 ");
        }
    }
    false
}
impl RealSystem {
    fn gnome_output(args: &[&str]) -> Result<String, InstallError> {
        run_status_command(
            Path::new("/usr/bin/gnome-extensions"),
            args,
            std::time::Duration::from_secs(5),
        )
    }
    fn trusted_root_status() -> Result<String, InstallError> {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        Self::checked_root_path(Path::new(HELPER_PATH))?;
        let meta = std::fs::symlink_metadata(HELPER_PATH)
            .map_err(|e| InstallError::Refused(format!("root status helper unavailable: {e}")))?;
        if !meta.file_type().is_file()
            || meta.uid() != 0
            || meta.permissions().mode() & 0o7777 != 0o755
        {
            return Err(InstallError::Refused("unsafe root status helper".into()));
        }
        let current = std::fs::read(
            std::env::current_exe().map_err(|e| InstallError::System(e.to_string()))?,
        )
        .map_err(|e| InstallError::System(e.to_string()))?;
        let installed =
            std::fs::read(HELPER_PATH).map_err(|e| InstallError::System(e.to_string()))?;
        if Sha256::digest(current) != Sha256::digest(installed) {
            return Err(InstallError::Refused(
                "root status helper hash differs; manual review required".into(),
            ));
        }
        run_status_command(
            Path::new("/usr/bin/pkexec"),
            &[HELPER_PATH, "privileged", "setup", "status"],
            std::time::Duration::from_secs(60),
        )
    }
    #[cfg(test)]
    fn root_recovery_pending_with(
        probe: Result<(), std::io::Error>,
        authenticated: impl FnOnce() -> Result<String, InstallError>,
    ) -> bool {
        match probe {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            _ => authenticated().as_deref() != Ok("clear"),
        }
    }
    pub fn discover() -> Result<Self, InstallError> {
        let login =
            std::env::var("USER").map_err(|e| InstallError::Refused(format!("USER: {e}")))?;
        let actual_login = crate::setup_helper::desktop_login_for_uid(unsafe { libc::geteuid() })
            .map_err(InstallError::Refused)?;
        if login != actual_login {
            return Err(InstallError::Refused(
                "USER does not match invoking account".into(),
            ));
        }
        let home = PathBuf::from(
            std::env::var_os("HOME")
                .ok_or_else(|| InstallError::Refused("HOME is required".into()))?,
        );
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        let gnome = Self::output("gnome-shell", &["--version"]);
        let gnome_major = gnome
            .ok()
            .and_then(|s| {
                s.split_whitespace()
                    .last()
                    .and_then(|v| v.split('.').next())
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(0);
        let mut missing_commands = Vec::new();
        for name in ["pkexec", "systemctl", "gnome-extensions", "gdbus", "visudo"] {
            if !Self::on_path(name) {
                missing_commands.push(name.into());
            }
        }
        let unit = home.join(".config/systemd/user/mcsr-toolkit.service");
        let service_active = Self::output(
            "systemctl",
            &["--user", "is-active", "mcsr-toolkit.service"],
        )
        .ok()
        .as_deref()
            == Some("active");
        let launch = Self::output(
            "systemctl",
            &["--user", "show", "-P", "ExecStart", "mcsr-toolkit.service"],
        )
        .ok()
        .or_else(|| std::fs::read_to_string(unit).ok())
        .unwrap_or_default();
        let python_service_active =
            service_active && !launch.contains("/usr/local/bin/mcsr-toolkit serve");
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("/run/user/{}", unsafe { libc::geteuid() })));
        let other_toolkit_active =
            !service_active && Self::lock_held(&runtime_dir.join("mcsr-toolkit-service.lock"));
        let root_status = root_status_snapshot(
            std::fs::symlink_metadata("/var/lib/mcsr-toolkit/keyd").map(|_| ()),
            std::fs::symlink_metadata("/run/mcsr-toolkit-maccel").map(|_| ()),
            std::fs::symlink_metadata(KEYD_POLICY_PATH)
                .or_else(|error| {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        std::fs::symlink_metadata(MACCEL_POLICY_PATH)
                    } else {
                        Err(error)
                    }
                })
                .map(|_| ()),
            Self::trusted_root_status,
        );
        Ok(Self {
            policy_status: root_status.policy,
            env: std::sync::Mutex::new(Environment {
                login,
                home,
                data_home,
                architecture: std::env::consts::ARCH.into(),
                gnome_major,
                wayland: std::env::var("XDG_SESSION_TYPE").ok().as_deref() == Some("wayland"),
                python_service_active,
                other_toolkit_active,
                keyd_recovery_pending: root_status.keyd_pending,
                maccel_recovery_pending: root_status.maccel_pending,
                missing_commands,
            }),
        })
    }
    fn acquire_install_lock(&self) -> Result<std::fs::File, InstallError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{MetadataExt, PermissionsExt},
        };
        const LOCK_DIRECTORY: &str = "/usr/local/bin";
        Self::checked_root_path(Path::new(LOCK_DIRECTORY))?;
        // The root-owned directory is a stable inode even when the desktop user can unlink their own files.
        let fd = unsafe {
            libc::open(
                c"/usr/local/bin".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if fd < 0 {
            return Err(InstallError::System(format!(
                "install lock open: {}",
                std::io::Error::last_os_error()
            )));
        }
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        let metadata = file
            .metadata()
            .map_err(|e| InstallError::System(e.to_string()))?;
        if !metadata.file_type().is_dir()
            || metadata.uid() != 0
            || metadata.permissions().mode() & 0o022 != 0
        {
            return Err(InstallError::Refused(
                "unsafe install lock directory".into(),
            ));
        }
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(InstallError::System(format!(
                "install lock: {}",
                std::io::Error::last_os_error()
            )));
        }
        let current = std::fs::symlink_metadata(LOCK_DIRECTORY)
            .map_err(|e| InstallError::System(e.to_string()))?;
        if current.ino() != metadata.ino() || current.dev() != metadata.dev() {
            return Err(InstallError::Refused(
                "install lock directory changed".into(),
            ));
        }
        Ok(file)
    }
    fn lock_held(path: &Path) -> bool {
        use std::os::fd::AsRawFd;
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
            Err(_) => return true,
        };
        if !metadata.file_type().is_file() {
            return true;
        }
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(_) => return true,
        };
        let status = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if status == 0 {
            unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
            false
        } else {
            true
        }
    }
    fn on_path(name: &str) -> bool {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
            .any(|dir| dir.join(name).is_file())
    }
    fn output(command: &str, args: &[&str]) -> Result<String, InstallError> {
        let out = std::process::Command::new(command)
            .args(args)
            .output()
            .map_err(|e| InstallError::System(format!("{command}: {e}")))?;
        if !out.status.success() {
            return Err(InstallError::System(format!(
                "{command} {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().into())
    }
    fn run(command: &str, args: &[&str]) -> Result<(), InstallError> {
        Self::output(command, args).map(|_| ())
    }
    fn checked_user_path(&self, path: &Path) -> Result<(), InstallError> {
        let env = self.environment()?;
        if !path.starts_with(&env.home) && !path.starts_with(&env.data_home) {
            return Err(InstallError::Refused("user write outside home".into()));
        }
        for ancestor in path.ancestors() {
            if ancestor == env.home.parent().unwrap_or(Path::new("/")) {
                break;
            }
            match std::fs::symlink_metadata(ancestor) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    return Err(InstallError::Refused(format!(
                        "symlink in install path: {}",
                        ancestor.display()
                    )))
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(InstallError::System(e.to_string())),
            }
        }
        Ok(())
    }
    fn checked_root_path(path: &Path) -> Result<(), InstallError> {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        for part in path.ancestors() {
            let meta = match std::fs::symlink_metadata(part) {
                Ok(meta) => meta,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(InstallError::System(e.to_string())),
            };
            if meta.file_type().is_symlink()
                || meta.uid() != 0
                || meta.permissions().mode() & 0o022 != 0
            {
                return Err(InstallError::Refused(format!(
                    "unsafe root path component: {}",
                    part.display()
                )));
            }
        }
        Ok(())
    }
}

fn open_file_nofollow(path: &Path) -> Result<Option<std::fs::File>, InstallError> {
    use std::os::unix::fs::OpenOptionsExt;
    match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
    {
        Ok(file) => Ok(Some(file)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(InstallError::System(error.to_string())),
    }
}

fn file_state_from_open(mut file: std::fs::File, path: &Path) -> Result<FileState, InstallError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let meta = file
        .metadata()
        .map_err(|e| InstallError::System(e.to_string()))?;
    if !meta.file_type().is_file() {
        return Err(InstallError::Refused(format!(
            "unsafe file type: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| InstallError::System(e.to_string()))?;
    Ok(FileState {
        bytes,
        uid: meta.uid(),
        mode: meta.permissions().mode() & 0o7777,
    })
}

fn read_file_state(path: &Path) -> Result<Option<FileState>, InstallError> {
    open_file_nofollow(path)?
        .map(|file| file_state_from_open(file, path))
        .transpose()
}

impl System for RealSystem {
    fn lock_install(&self) -> Result<Box<dyn InstallGuard + '_>, InstallError> {
        let guard = self.acquire_install_lock()?;
        Ok(Box::new(guard))
    }
    fn environment(&self) -> Result<Environment, InstallError> {
        let env = self
            .env
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        Ok(env)
    }
    fn read(&self, path: &Path) -> Result<Option<FileState>, InstallError> {
        if [
            SERVICE_PATH,
            HELPER_PATH,
            KEYD_POLICY_PATH,
            MACCEL_POLICY_PATH,
        ]
        .iter()
        .any(|p| path == Path::new(p))
        {
            Self::checked_root_path(path)?;
        }
        read_file_state(path)
    }
    fn write_user(&self, path: &Path, bytes: &[u8], mode: u32) -> Result<(), InstallError> {
        use std::os::unix::fs::PermissionsExt;
        self.checked_user_path(path)?;
        let parent = path
            .parent()
            .ok_or_else(|| InstallError::Refused("missing parent".into()))?;
        std::fs::create_dir_all(parent).map_err(|e| InstallError::System(e.to_string()))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)
            .map_err(|e| InstallError::System(e.to_string()))?;
        std::io::Write::write_all(&mut file, bytes)
            .map_err(|e| InstallError::System(e.to_string()))?;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|e| InstallError::System(e.to_string()))?;
        file.as_file()
            .sync_all()
            .map_err(|e| InstallError::System(e.to_string()))?;
        file.persist(path)
            .map_err(|e| InstallError::System(e.to_string()))?;
        sync_parent(path)?;
        Ok(())
    }
    fn remove_user(&self, path: &Path) -> Result<(), InstallError> {
        self.checked_user_path(path)?;
        match std::fs::remove_file(path) {
            Ok(()) => sync_parent(path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(InstallError::System(e.to_string())),
        }
    }
    fn write_root(&self, _files: &[RootFile]) -> Result<(), InstallError> {
        Err(InstallError::Refused(
            "legacy root staging is disabled".into(),
        ))
    }
    fn remove_root(&self, _paths: &[PathBuf]) -> Result<(), InstallError> {
        Err(InstallError::Refused(
            "legacy root removal is disabled".into(),
        ))
    }
    fn validate_sudoers(&self, rule: &[u8]) -> Result<(), InstallError> {
        let mut temp =
            tempfile::NamedTempFile::new().map_err(|e| InstallError::System(e.to_string()))?;
        std::io::Write::write_all(&mut temp, rule)
            .map_err(|e| InstallError::System(e.to_string()))?;
        Self::run(
            "/usr/sbin/visudo",
            &[
                "-cf",
                temp.path()
                    .to_str()
                    .ok_or_else(|| InstallError::Refused("non-UTF8 temp path".into()))?,
            ],
        )
    }
    fn policies_match(&self, rules: &[RootFile; 2]) -> Result<bool, InstallError> {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if matches!(
            self.policy_status,
            PolicyStatus::Mismatch | PolicyStatus::Unknown
        ) {
            return Err(InstallError::Refused(
                "existing root policy could not be verified; manual review required".into(),
            ));
        }
        let mut all = true;
        for rule in rules {
            Self::checked_root_path(&rule.path)?;
            let meta = match std::fs::symlink_metadata(&rule.path) {
                Ok(meta) => meta,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    all = false;
                    continue;
                }
                Err(e) => return Err(InstallError::System(e.to_string())),
            };
            if !meta.file_type().is_file()
                || meta.uid() != 0
                || meta.permissions().mode() & 0o7777 != rule.mode
            {
                return Err(InstallError::Refused(format!(
                    "existing policy {} differs; manual review required",
                    rule.path.display()
                )));
            }
            if !cached_policy_bytes_match(
                std::fs::read(&rule.path),
                &rule.bytes,
                self.policy_status,
            )? {
                all = false;
            }
        }
        Ok(all)
    }
    fn begin_root_setup(
        &self,
        artifact: &Artifact,
    ) -> Result<Box<dyn RootSetup + '_>, InstallError> {
        use std::io::Write;
        if !verified_helper(self, artifact)? {
            Self::checked_root_path(Path::new(HELPER_PATH))?;
            let mut source =
                tempfile::NamedTempFile::new().map_err(|e| InstallError::System(e.to_string()))?;
            source
                .write_all(artifact.bytes())
                .map_err(|e| InstallError::System(e.to_string()))?;
            source
                .as_file()
                .sync_all()
                .map_err(|e| InstallError::System(e.to_string()))?;
            Self::run(
                "/usr/bin/pkexec",
                &[
                    "/usr/bin/install",
                    "-D",
                    "-o",
                    "root",
                    "-g",
                    "root",
                    "-m",
                    "0755",
                    source
                        .path()
                        .to_str()
                        .ok_or_else(|| InstallError::Refused("non-UTF8 bootstrap source".into()))?,
                    HELPER_PATH,
                ],
            )?;
            if !verified_helper(self, artifact)? {
                return Err(InstallError::Refused(
                    "bootstrap helper ownership, mode, path, or hash differs".into(),
                ));
            }
        }
        start_root_transaction(false)
    }
    fn begin_root_uninstall(&self) -> Result<Box<dyn RootSetup + '_>, InstallError> {
        let bytes = std::fs::read(
            std::env::current_exe().map_err(|e| InstallError::System(e.to_string()))?,
        )
        .map_err(|e| InstallError::System(e.to_string()))?;
        let artifact = Artifact::from_bytes(bytes)?;
        if !verified_helper(self, &artifact)? {
            return Err(InstallError::Refused(
                "verified installed helper required for uninstall".into(),
            ));
        }
        Self::checked_root_path(Path::new(HELPER_PATH))?;
        let meta = std::fs::symlink_metadata(HELPER_PATH).map_err(|e| {
            InstallError::Refused(format!("verified root helper required for uninstall: {e}"))
        })?;
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if !meta.file_type().is_file()
            || meta.uid() != 0
            || meta.permissions().mode() & 0o7777 != 0o755
        {
            return Err(InstallError::Refused(
                "verified root helper required for uninstall".into(),
            ));
        }
        start_root_transaction(true)
    }
    fn service(&self, action: ServiceAction) -> Result<(), InstallError> {
        let arg = match action {
            ServiceAction::Stop => "stop",
            ServiceAction::Reload => "daemon-reload",
            ServiceAction::Enable => "enable",
            ServiceAction::Start => "start",
            ServiceAction::Disable => "disable",
        };
        if action == ServiceAction::Reload {
            Self::run("/usr/bin/systemctl", &["--user", arg])
        } else {
            Self::run(
                "/usr/bin/systemctl",
                &["--user", arg, "mcsr-toolkit.service"],
            )
        }
    }
    fn extension(&self, action: ExtensionAction) -> Result<(), InstallError> {
        Self::gnome_output(&[
            if action == ExtensionAction::Enable {
                "enable"
            } else {
                "disable"
            },
            EXTENSION_UUID,
        ])
        .map(|_| ())
    }
    fn service_active(&self) -> Result<bool, InstallError> {
        Ok(Self::output(
            "/usr/bin/systemctl",
            &["--user", "is-active", "mcsr-toolkit.service"],
        )
        .ok()
        .as_deref()
            == Some("active"))
    }
    fn service_dropins(&self) -> Result<Vec<PathBuf>, InstallError> {
        let home = self
            .env
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .home
            .clone();
        let unit_name = "mcsr-toolkit.service";
        let unit_files = [
            home.join(".config/systemd/user").join(unit_name),
            PathBuf::from("/etc/systemd/user").join(unit_name),
            PathBuf::from("/usr/lib/systemd/user").join(unit_name),
            PathBuf::from("/lib/systemd/user").join(unit_name),
        ];
        let loaded = Self::output(
            "/usr/bin/systemctl",
            &["--user", "show", "-P", "DropInPaths", unit_name],
        );
        let mut paths: Vec<PathBuf> = match loaded {
            Ok(text) => text.split_whitespace().map(PathBuf::from).collect(),
            Err(error) => {
                for path in &unit_files {
                    match std::fs::symlink_metadata(path) {
                        Ok(_) => return Err(error),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(InstallError::System(e.to_string())),
                    }
                }
                Vec::new()
            }
        };
        for base in [
            home.join(".config/systemd/user"),
            PathBuf::from("/etc/systemd/user"),
            PathBuf::from("/usr/lib/systemd/user"),
            PathBuf::from("/lib/systemd/user"),
        ] {
            let directory = base.join("mcsr-toolkit.service.d");
            match std::fs::read_dir(&directory) {
                Ok(entries) => {
                    for entry in entries {
                        let path = entry
                            .map_err(|e| InstallError::System(e.to_string()))?
                            .path();
                        if path.extension().is_some_and(|ext| ext == "conf")
                            && !paths.contains(&path)
                        {
                            paths.push(path);
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(InstallError::System(error.to_string())),
            }
        }
        Ok(paths)
    }
    fn service_command(&self) -> Result<String, InstallError> {
        Self::output(
            "/usr/bin/systemctl",
            &["--user", "show", "-P", "ExecStart", "mcsr-toolkit.service"],
        )
    }
    fn service_ready(&self) -> Result<bool, InstallError> {
        let address = std::net::SocketAddr::from(([127, 0, 0, 1], 8765));
        Ok(http_probe(address, std::time::Duration::from_millis(250)))
    }
    fn service_enabled(&self) -> Result<bool, InstallError> {
        Ok(Self::output(
            "/usr/bin/systemctl",
            &["--user", "is-enabled", "mcsr-toolkit.service"],
        )
        .ok()
        .as_deref()
            == Some("enabled"))
    }
    fn extension_enabled(&self) -> Result<bool, InstallError> {
        extension_enabled_from_output(Self::gnome_output(&["list", "--enabled"]))
    }
    fn extension_known(&self) -> Result<bool, InstallError> {
        extension_named_in_output(Self::gnome_output(&["list", "--user"]), EXTENSION_UUID)
    }
    fn extension_state(&self) -> Result<ExtensionState, InstallError> {
        extension_state_from_info(Self::gnome_output(&["info", EXTENSION_UUID])?)
    }
    fn legacy_extension_known(&self) -> Result<bool, InstallError> {
        extension_named_in_output(
            Self::gnome_output(&["list", "--user"]),
            LEGACY_EXTENSION_UUID,
        )
    }
    fn legacy_extension_enabled(&self) -> Result<bool, InstallError> {
        extension_named_in_output(
            Self::gnome_output(&["list", "--enabled"]),
            LEGACY_EXTENSION_UUID,
        )
    }
    fn legacy_extension_state(&self) -> Result<ExtensionState, InstallError> {
        extension_state_from_info(Self::gnome_output(&["info", LEGACY_EXTENSION_UUID])?)
    }
    fn legacy_extension(&self, action: ExtensionAction) -> Result<(), InstallError> {
        Self::gnome_output(&[
            if action == ExtensionAction::Enable {
                "enable"
            } else {
                "disable"
            },
            LEGACY_EXTENSION_UUID,
        ])
        .map(|_| ())
    }
    fn toolkit_object_exported(&self) -> Result<bool, InstallError> {
        let args = [
            "/usr/bin/gdbus",
            "introspect",
            "--session",
            "--dest",
            "org.gnome.Shell",
            "--object-path",
            "/org/mcsr/Toolkit",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        toolkit_object_from_introspection(
            crate::keyd::run_command(&args, &[], std::time::Duration::from_secs(5))
                .map(|out| out.stdout),
        )
    }
    fn ensure_private_state(&self, data_home: &Path) -> Result<(), InstallError> {
        crate::paths::prepare_private_state(data_home)
            .map(|_| ())
            .map_err(|e| InstallError::Refused(format!("unsafe state directory: {e}")))
    }
    fn shell_instance(&self) -> Result<String, InstallError> {
        let owner = || {
            Self::output(
                "/usr/bin/gdbus",
                &[
                    "call",
                    "--session",
                    "--dest",
                    "org.freedesktop.DBus",
                    "--object-path",
                    "/org/freedesktop/DBus",
                    "--method",
                    "org.freedesktop.DBus.GetConnectionUnixProcessID",
                    "org.gnome.Shell",
                ],
            )
        };
        stable_shell_identity_with(
            owner,
            unsafe { libc::geteuid() },
            read_shell_process_snapshot,
        )
    }
    fn recover_input(&self) -> Result<(), InstallError> {
        // Do not remove the helper until both recovery paths are verified clear.
        if Self::output("/usr/bin/systemctl", &["is-active", "keyd.service"])
            .ok()
            .as_deref()
            == Some("active")
        {
            Self::run("/usr/bin/keyd", &["bind", "reset"])?;
            Self::run(
                "/usr/bin/sudo",
                &["-n", HELPER_PATH, "privileged", "keyd", "clear"],
            )?;
            Self::run("/usr/bin/systemctl", &["is-active", "keyd.service"])?;
        }
        if Path::new("/run/mcsr-toolkit-maccel").exists() {
            Self::run(
                "/usr/bin/sudo",
                &["-n", HELPER_PATH, "privileged", "maccel", "restore"],
            )?;
            if Path::new("/run/mcsr-toolkit-maccel/baseline").exists() {
                return Err(InstallError::Refused(
                    "maccel baseline still pending".into(),
                ));
            }
            self.env
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .maccel_recovery_pending = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    #[test]
    fn shell_object_probe_distinguishes_exported_interface_absence_and_bus_failure() {
        use crate::keyd::CommandFailure;
        let exported =
            "node /org/mcsr/Toolkit {\n  interface org.mcsr.Toolkit {\n    methods:\n  };\n};";
        assert!(super::toolkit_object_from_introspection(Ok(exported.into())).unwrap());
        assert!(!super::toolkit_object_from_introspection(Err(CommandFailure::Failed("command failed: GDBus.Error:org.freedesktop.DBus.Error.UnknownObject: Object does not exist at path /org/mcsr/Toolkit".into()))).unwrap());
        assert!(!super::toolkit_object_from_introspection(Err(CommandFailure::Failed("command failed: GDBus.Error:org.freedesktop.DBus.Error.UnknownMethod: Object does not exist at path “/org/mcsr/Toolkit”".into()))).unwrap());
        assert!(
            super::toolkit_object_from_introspection(Err(CommandFailure::Failed(
                "session bus unavailable".into()
            )))
            .is_err()
        );
    }

    #[test]
    fn shell_object_probe_recognizes_empty_node_as_absent() {
        for output in [
            "node /org/mcsr/Toolkit {\n};\n",
            "node /org/mcsr/Toolkit { };",
        ] {
            assert!(!super::toolkit_object_from_introspection(Ok(output.into())).unwrap());
        }
    }

    #[test]
    fn shell_object_probe_refuses_unexpected_successful_output() {
        for output in [
            "node /org/mcsr/Toolkit {\n  interface org.example.Other {\n  };\n};\n",
            "node /org/mcsr/Toolkit {\n  node child {\n  };\n};\n",
            "node /org/example/Other {\n};\n",
            "node /org/mcsr/Toolkit {\n",
            "",
        ] {
            assert!(matches!(
                super::toolkit_object_from_introspection(Ok(output.into())),
                Err(super::InstallError::Refused(_))
            ));
        }
    }

    #[test]
    fn real_temp_marker_requires_owner_and_mode_0600() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("mcsr-toolkit/rust-v1");
        std::fs::create_dir_all(&parent).unwrap();
        std::fs::set_permissions(
            parent.parent().unwrap(),
            std::fs::Permissions::from_mode(0o775),
        )
        .unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o775)).unwrap();
        let marker = parent.join("install-pending.json");
        std::fs::write(&marker, b"{\"phase\":\"repair\"}\n").unwrap();
        let inspect = || super::read_file_state(&marker).unwrap().unwrap();
        std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o664)).unwrap();
        assert!(super::validate_marker_state(&inspect()).is_err());
        std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(super::validate_marker_state(&inspect()).is_ok());
        let mut foreign = inspect();
        foreign.uid += 1;
        assert!(super::validate_marker_state(&foreign).is_err());
    }

    #[test]
    fn marker_read_uses_opened_inode_when_path_is_replaced() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("install-pending.json");
        let saved = temp.path().join("saved-marker");
        let attacker = temp.path().join("other");
        std::fs::write(&marker, b"trusted marker").unwrap();
        std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&attacker, b"foreign marker").unwrap();
        let file = super::open_file_nofollow(&marker).unwrap().unwrap();
        std::fs::rename(&marker, &saved).unwrap();
        symlink(&attacker, &marker).unwrap();
        let state = super::file_state_from_open(file, &marker).unwrap();
        assert_eq!(state.bytes, b"trusted marker");
        assert!(super::validate_marker_state(&state).is_ok());
    }

    #[test]
    fn enabled_catalog_probe_error_is_not_classified_as_disabled() {
        let error = super::InstallError::System("session bus unavailable".into());
        assert!(super::extension_enabled_from_output(Err(error)).is_err());
        assert!(super::extension_enabled_from_output(Ok(
            "other@extension\nmcsr-toolkit@undecember.github.io".into(),
        ))
        .unwrap());
    }

    #[test]
    fn shell_identity_uses_session_bus_owner_among_multiple_shells() {
        let process = |pid| match pid {
            12 => Some(super::ShellProcessSnapshot {
                uid: 1000,
                executable: "/usr/bin/gnome-shell".into(),
                starttime: 100,
            }),
            45 => Some(super::ShellProcessSnapshot {
                uid: 1000,
                executable: "/usr/bin/gnome-shell".into(),
                starttime: 200,
            }),
            46 => Some(super::ShellProcessSnapshot {
                uid: 1000,
                executable: "/usr/bin/gnome-shell".into(),
                starttime: 300,
            }),
            _ => None,
        };
        assert_eq!(
            super::shell_identity_from_bus_owner_with("(uint32 45,)", 1000, process).unwrap(),
            "45:200"
        );
        // The older Shell exits; the bus owner is still the same process.
        assert_eq!(
            super::shell_identity_from_bus_owner_with("(uint32 45,)", 1000, |pid| {
                if pid == 12 {
                    None
                } else {
                    process(pid)
                }
            })
            .unwrap(),
            "45:200"
        );
        assert_eq!(
            super::shell_identity_from_bus_owner_with("(uint32 46,)", 1000, process).unwrap(),
            "46:300"
        );
        // Reusing the owner's PID after a restart still changes the process identity.
        assert_eq!(
            super::shell_identity_from_bus_owner_with("(uint32 45,)", 1000, |_| {
                Some(super::ShellProcessSnapshot {
                    uid: 1000,
                    executable: "/usr/bin/gnome-shell".into(),
                    starttime: 301,
                })
            })
            .unwrap(),
            "45:301"
        );
    }

    #[test]
    fn shell_identity_refuses_stale_or_ambiguous_bus_owner() {
        let snapshot = |_| {
            Some(super::ShellProcessSnapshot {
                uid: 1000,
                executable: "/usr/bin/gnome-shell".into(),
                starttime: 200,
            })
        };
        for response in ["(uint32 45,)(uint32 46,)", "(uint32 0,)", "(uint32 nope,)"] {
            assert!(super::shell_identity_from_bus_owner_with(response, 1000, snapshot).is_err());
        }
        assert!(super::shell_identity_from_bus_owner_with("(uint32 45,)", 1000, |_| None).is_err());
        assert!(
            super::shell_identity_from_bus_owner_with("(uint32 45,)", 1000, |_| {
                Some(super::ShellProcessSnapshot {
                    uid: 1001,
                    executable: "/usr/bin/gnome-shell".into(),
                    starttime: 200,
                })
            })
            .is_err()
        );
        assert!(
            super::shell_identity_from_bus_owner_with("(uint32 45,)", 1000, |_| {
                Some(super::ShellProcessSnapshot {
                    uid: 1000,
                    executable: "/usr/bin/other-shell".into(),
                    starttime: 200,
                })
            })
            .is_err()
        );
    }

    #[test]
    fn shell_identity_refuses_owner_change_during_inspection() {
        let mut owners = ["(uint32 45,)", "(uint32 46,)"].into_iter();
        let result = super::stable_shell_identity_with(
            || Ok(owners.next().unwrap().into()),
            1000,
            |pid| {
                Some(super::ShellProcessSnapshot {
                    uid: 1000,
                    executable: "/usr/bin/gnome-shell".into(),
                    starttime: pid as u64,
                })
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn readiness_probe_waits_for_complete_http_status_line() {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0u8; 128];
            assert!(socket.read(&mut request).unwrap() > 0);
            socket.write_all(b"HTTP/1.1 2").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
            socket
                .write_all(b"00 OK\r\ncontent-length: 0\r\n\r\n")
                .unwrap();
        });
        assert!(super::http_probe(
            address,
            std::time::Duration::from_millis(250)
        ));
        server.join().unwrap();
    }

    #[test]
    fn inaccessible_private_root_is_classified_by_authenticated_probe() {
        let denied = || Err(io::Error::from(io::ErrorKind::PermissionDenied));
        assert!(!super::RealSystem::root_recovery_pending_with(
            denied(),
            || Ok("clear".into())
        ));
        assert!(super::RealSystem::root_recovery_pending_with(
            denied(),
            || Ok("active".into())
        ));
        assert!(super::RealSystem::root_recovery_pending_with(
            denied(),
            || Err(super::InstallError::System("denied".into()))
        ));
        assert!(!super::RealSystem::root_recovery_pending_with(
            Err(io::Error::from(io::ErrorKind::NotFound)),
            || panic!("missing root must not invoke helper")
        ));
    }

    #[test]
    fn authenticated_status_command_has_a_deadline() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("fake-stalled-status");
        std::fs::write(&fake, b"#!/bin/sh\nwhile :; do :; done\n").unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        let started = std::time::Instant::now();
        let error = super::run_status_command(&fake, &[], std::time::Duration::from_millis(75))
            .unwrap_err()
            .to_string();
        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }

    #[test]
    fn both_protected_recovery_histories_use_one_authenticated_status() {
        let calls = std::cell::Cell::new(0);
        let statuses = super::root_recovery_snapshot(Ok(()), Ok(()), || {
            calls.set(calls.get() + 1);
            Ok("keyd=clear maccel=clear policies=missing".into())
        });
        assert_eq!(statuses, (false, false));
        assert_eq!(calls.get(), 1);
        let missing = super::root_recovery_snapshot(
            Err(io::ErrorKind::NotFound.into()),
            Err(io::ErrorKind::NotFound.into()),
            || panic!("missing records must not authorize"),
        );
        assert_eq!(missing, (false, false));
    }

    #[test]
    fn unreadable_exact_policies_share_cached_authenticated_status() {
        let calls = std::cell::Cell::new(0);
        let snapshot = super::root_status_snapshot(
            Err(io::ErrorKind::NotFound.into()),
            Err(io::ErrorKind::NotFound.into()),
            Ok(()),
            || {
                calls.set(calls.get() + 1);
                Ok("keyd=clear maccel=clear policies=match".into())
            },
        );
        assert_eq!(snapshot.policy, super::PolicyStatus::Match);
        assert_eq!(calls.get(), 1);
        assert!(super::cached_policy_bytes_match(
            Err(io::ErrorKind::PermissionDenied.into()),
            b"exact sudoers",
            snapshot.policy,
        )
        .unwrap());
        assert!(!super::cached_policy_bytes_match(
            Err(io::ErrorKind::PermissionDenied.into()),
            b"exact sudoers",
            super::PolicyStatus::Missing,
        )
        .unwrap());
    }

    #[test]
    fn marker_parent_sync_failure_is_returned_before_root_setup() {
        let fixture = tempfile::tempdir().unwrap();
        let marker = fixture.path().join("install-pending.json");
        std::fs::write(&marker, b"pending").unwrap();
        let called = std::cell::Cell::new(false);
        let failure = super::sync_parent_with(&marker, |_| {
            called.set(true);
            Err(io::ErrorKind::Other.into())
        });
        assert!(called.get());
        assert!(failure.is_err());
        let mut synced = Vec::new();
        super::sync_parent_with(&marker, |directory| {
            synced.push(directory.to_path_buf());
            Ok(())
        })
        .unwrap();
        assert_eq!(synced.first().unwrap(), marker.parent().unwrap());
        assert_eq!(synced.last().unwrap(), std::path::Path::new("/"));
    }

    #[test]
    fn abandoned_setup_terminates_privileged_process_group() {
        use std::os::unix::process::CommandExt;
        let fixture = tempfile::tempdir().unwrap();
        let marker = fixture.path().join("survivor");
        let mut child = std::process::Command::new("/bin/sh")
            .args([
                "-c",
                "(sleep 0.3; : > \"$1\") & echo READY; wait",
                "sh",
                marker.to_str().unwrap(),
            ])
            .process_group(0)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let setup = super::RealRootSetup {
            input: child.stdin.take(),
            output: child.stdout.take().unwrap(),
            child,
            finished: false,
        };
        drop(setup);
        std::thread::sleep(std::time::Duration::from_millis(450));
        assert!(
            !marker.exists(),
            "privileged descendant survived setup cancellation"
        );
    }

    #[test]
    fn readonly_directory_flock_excludes_a_second_opener() {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        };
        let fixture = tempfile::tempdir().unwrap();
        let name = std::ffi::CString::new(fixture.path().as_os_str().as_bytes()).unwrap();
        let open = || {
            let fd = unsafe {
                libc::open(
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
                )
            };
            assert!(fd >= 0, "{}", io::Error::last_os_error());
            unsafe { std::fs::File::from_raw_fd(fd) }
        };
        let first = open();
        let second = open();
        assert_eq!(
            unsafe { libc::flock(first.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        assert_eq!(
            unsafe { libc::flock(second.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(unsafe { libc::flock(first.as_raw_fd(), libc::LOCK_UN) }, 0);
        assert_eq!(
            unsafe { libc::flock(second.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
    }
}
