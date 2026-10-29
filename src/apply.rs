use crate::instances::{discover_instances, Instance};
use crate::mapping::{plan_controls, ManagedState};
use crate::options::{parse_options, render_options};
use crate::paths::{
    atomic_replace, ensure_state, private_dir, remove_record, safe_record, state_dir, sync_dir,
};
use crate::profile::Profile;
use crate::standard_settings::{prepare_local, ConfigEdit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::fs::OpenOptions;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_BACKUP: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct ApplyError(pub String);
impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ApplyError {}
fn err(e: impl fmt::Display) -> ApplyError {
    ApplyError(e.to_string())
}

pub trait KeydAdapter {
    fn install(&self, profile: &Profile) -> Result<(), String>;
    fn checkpoint(&self) -> Result<Vec<String>, String>;
    fn restore_to(&self, checkpoint: &[String]) -> Result<(), String>;
    fn rollback(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceResult {
    pub instance_id: String,
    pub status: String,
    pub detail: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlMapping {
    pub instance_id: String,
    pub physical: String,
    pub emitted: String,
    pub action: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPreview {
    pub instances: Vec<InstanceResult>,
    pub mappings: Vec<ControlMapping>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    pub instances: Vec<InstanceResult>,
    pub backup_id: Option<String>,
    pub keyd_error: Option<String>,
    pub restored_profile: Option<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct History {
    head: Option<String>,
    profile: Option<Profile>,
    keyd: Vec<String>,
    launcher_root: PathBuf,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupFile {
    relative: PathBuf,
    before_hash: String,
    after_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    launcher_root: PathBuf,
    files: BTreeMap<String, BackupFile>,
    prior_state: BTreeMap<String, ManagedState>,
    post_state: BTreeMap<String, ManagedState>,
    prior_history: History,
    profile: Profile,
    keyd_after: Vec<String>,
    // None identifies an older backup that did not manage mod configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mod_files: Option<BTreeMap<String, BTreeMap<String, BackupFile>>>,
}
struct Prepared {
    instance: Instance,
    old: Vec<u8>,
    new: Vec<u8>,
    state: ManagedState,
    configs: Vec<ConfigEdit>,
}
impl Prepared {
    fn changed(&self) -> bool {
        self.old != self.new || self.configs.iter().any(|edit| edit.before != edit.after)
    }
}

struct MutationLock(PathBuf);
impl MutationLock {
    fn acquire(state: &Path) -> Result<Self, ApplyError> {
        let path = state.join("mutation.lock");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| err(format!("mutation locked: {e}")))?
            .sync_all()
            .map_err(err)?;
        sync_dir(state).map_err(err)?;
        Ok(Self(path))
    }
}
impl Drop for MutationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
        if let Some(parent) = self.0.parent() {
            let _ = sync_dir(parent);
        }
    }
}

pub struct ApplyCoordinator {
    root: PathBuf,
    state: PathBuf,
    process_cwds: Vec<PathBuf>,
}
impl ApplyCoordinator {
    pub fn new(
        root: impl AsRef<Path>,
        data_home: impl AsRef<Path>,
        process_cwds: Vec<PathBuf>,
    ) -> Result<Self, ApplyError> {
        if root
            .as_ref()
            .symlink_metadata()
            .map_err(err)?
            .file_type()
            .is_symlink()
        {
            return Err(err("unsafe launcher root symlink"));
        }
        let root = root.as_ref().canonicalize().map_err(err)?;
        if !root.join("instances").is_dir() {
            return Err(err("not a launcher root"));
        }
        Ok(Self {
            root,
            state: state_dir(data_home.as_ref()),
            process_cwds,
        })
    }
    fn guard(&self) -> Result<(), ApplyError> {
        if self.state.parent().is_some_and(|parent| parent.exists()) {
            private_dir(self.state.parent().unwrap()).map_err(err)?;
        }
        if self.state.symlink_metadata().is_ok() {
            private_dir(&self.state).map_err(err)?;
        }
        if self.state.join("mutation.json").symlink_metadata().is_ok() {
            return Err(err("recovery required: unfinished mutation"));
        }
        Ok(())
    }
    fn record<T: for<'de> Deserialize<'de>>(&self, path: &Path) -> Result<T, ApplyError> {
        serde_json::from_slice(&safe_record(path).map_err(err)?).map_err(err)
    }
    fn write<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), ApplyError> {
        atomic_replace(path, &serde_json::to_vec(value).map_err(err)?, true).map_err(err)
    }
    fn history(&self, keyd: &dyn KeydAdapter) -> Result<History, ApplyError> {
        let checkpoint = keyd.checkpoint().map_err(err)?;
        let path = self.state.join("lineage.json");
        if !record_present(&path)? {
            if record_present(&self.state.join("managed.json"))? || !checkpoint.is_empty() {
                return Err(err("legacy or missing recovery lineage"));
            }
            return Ok(History {
                head: None,
                profile: None,
                keyd: vec![],
                launcher_root: self.root.clone(),
            });
        }
        let history: History = self.record(&path)?;
        if history.launcher_root != self.root {
            return Err(err("history belongs to a different launcher root"));
        }
        if history.keyd != checkpoint
            || history.head.is_some() != history.profile.is_some()
            || history.head.is_none() && !history.keyd.is_empty()
        {
            return Err(err("stale or ambiguous recovery history"));
        }
        if history.head.as_deref().is_some_and(|id| !valid_id(id)) {
            return Err(err("invalid recovery head"));
        }
        Ok(history)
    }
    fn managed(&self) -> Result<BTreeMap<String, ManagedState>, ApplyError> {
        let path = self.state.join("managed.json");
        if !record_present(&path)? {
            return Ok(BTreeMap::new());
        }
        self.record(&path)
    }
    fn running(&self, instance: &Instance) -> bool {
        self.process_cwds.iter().any(|cwd| {
            cwd.canonicalize()
                .is_ok_and(|cwd| cwd.starts_with(&instance.path))
        })
    }
    fn prepare(
        &self,
        profile: &Profile,
        state: &BTreeMap<String, ManagedState>,
    ) -> Result<(Vec<Prepared>, ApplyPreview), ApplyError> {
        let mut prepared = vec![];
        let mut results = vec![];
        let mut mappings = vec![];
        for instance in discover_instances(&self.root).map_err(err)? {
            let mut failure = instance.problem.clone();
            let mut maybe = None;
            if failure.is_none() {
                let result = (|| {
                    let old = fs::read(&instance.options_path).map_err(err)?;
                    let document = parse_options(&old).map_err(err)?;
                    let plan =
                        plan_controls(&document, profile, state.get(&instance.id)).map_err(err)?;
                    for (physical, emitted, action) in &plan.preview {
                        mappings.push(ControlMapping {
                            instance_id: instance.id.clone(),
                            physical: physical.clone(),
                            emitted: emitted.clone(),
                            action: action.clone(),
                        });
                    }
                    if !plan.conflicts.is_empty() {
                        return Err(err(plan.conflicts.join("; ")));
                    }
                    let new = render_options(&document, &plan);
                    let effective = parse_options(&new).map_err(err)?;
                    let configs =
                        prepare_local(instance.options_path.parent().unwrap(), &effective.bindings)
                            .map_err(err)?;
                    Ok::<_, ApplyError>((old, new, plan.next_state, configs))
                })();
                match result {
                    Ok(value) => maybe = Some(value),
                    Err(e) => failure = Some(e.0),
                }
            }
            if let Some(detail) = failure {
                results.push(InstanceResult {
                    instance_id: instance.id,
                    status: "failed".into(),
                    detail,
                });
                continue;
            }
            let (old, new, next_state, configs) = maybe.unwrap();
            let changed = old != new || configs.iter().any(|edit| edit.before != edit.after);
            let status = if !changed {
                "unchanged"
            } else if self.running(&instance) {
                "deferred"
            } else {
                "ready"
            };
            results.push(InstanceResult {
                instance_id: instance.id.clone(),
                status: status.into(),
                detail: String::new(),
            });
            if status != "deferred" {
                prepared.push(Prepared {
                    instance,
                    old,
                    new,
                    state: next_state,
                    configs,
                });
            }
        }
        Ok((
            prepared,
            ApplyPreview {
                instances: results,
                mappings,
            },
        ))
    }
    pub fn preview(&self, profile: &Profile) -> Result<ApplyPreview, ApplyError> {
        self.guard()?;
        Ok(self.prepare(profile, &self.managed()?)?.1)
    }
    fn validate_prepared(&self, prepared: &[Prepared], applied: bool) -> Result<(), ApplyError> {
        let instances = discover_instances(&self.root).map_err(err)?;
        if instances.len() != prepared.len() {
            return Err(err("launcher instances changed during apply"));
        }
        for instance in instances {
            let item = prepared
                .iter()
                .find(|item| item.instance.id == instance.id)
                .ok_or_else(|| err("launcher instances changed during apply"))?;
            if instance.problem.is_some() || instance.options_path != item.instance.options_path {
                return Err(err("unsafe or changed instance during apply"));
            }
            let expected = if applied { &item.new } else { &item.old };
            if fs::read(&instance.options_path).map_err(err)? != *expected {
                return Err(err("options changed during apply"));
            }
            let configs =
                prepare_local(instance.options_path.parent().unwrap(), &Default::default())
                    .map_err(err)?;
            if configs.len() != item.configs.len() {
                return Err(err("Standard Settings files changed during apply"));
            }
            for config in configs {
                let edit = item
                    .configs
                    .iter()
                    .find(|edit| edit.path == config.path)
                    .ok_or_else(|| err("Standard Settings files changed during apply"))?;
                let expected = if applied { &edit.after } else { &edit.before };
                if config.before != *expected {
                    return Err(err("Standard Settings contents changed during apply"));
                }
            }
        }
        Ok(())
    }
    pub fn apply(
        &self,
        profile: &Profile,
        keyd: &dyn KeydAdapter,
    ) -> Result<ApplyReport, ApplyError> {
        self.apply_with_publish(profile, keyd, &|_| Ok(()))
    }
    pub fn apply_with_publish(
        &self,
        profile: &Profile,
        keyd: &dyn KeydAdapter,
        publish: &dyn Fn(&Profile) -> Result<(), ApplyError>,
    ) -> Result<ApplyReport, ApplyError> {
        ensure_state(self.state.parent().unwrap().parent().unwrap()).map_err(err)?;
        let _lock = MutationLock::acquire(&self.state)?;
        self.apply_inner(profile, keyd, true, publish)
    }
    pub fn reconcile_new_instances(
        &self,
        profile: &Profile,
        keyd: &dyn KeydAdapter,
    ) -> Result<ApplyReport, ApplyError> {
        self.reconcile_new_instances_with_publish(profile, keyd, &|_| Ok(()))
    }
    pub fn reconcile_new_instances_with_publish(
        &self,
        profile: &Profile,
        keyd: &dyn KeydAdapter,
        publish: &dyn Fn(&Profile) -> Result<(), ApplyError>,
    ) -> Result<ApplyReport, ApplyError> {
        ensure_state(self.state.parent().unwrap().parent().unwrap()).map_err(err)?;
        let _lock = MutationLock::acquire(&self.state)?;
        self.guard()?;
        let pending = self.state.join("pending.json");
        if pending.exists() {
            let p: Profile = self.record(&pending)?;
            return self.apply_inner(&p, keyd, true, publish);
        }
        self.apply_inner(profile, keyd, false, publish)
    }
    fn apply_inner(
        &self,
        profile: &Profile,
        keyd: &dyn KeydAdapter,
        install: bool,
        publish: &dyn Fn(&Profile) -> Result<(), ApplyError>,
    ) -> Result<ApplyReport, ApplyError> {
        self.guard()?;
        let history = self.history(keyd)?;
        let state = self.managed()?;
        let (prepared, mut preview) = self.prepare(profile, &state)?;
        if preview.instances.iter().any(|x| x.status == "failed") {
            for item in &mut preview.instances {
                if item.status == "ready" {
                    item.status = "blocked".into();
                    item.detail = "another instance failed validation".into();
                }
            }
            return Ok(ApplyReport {
                instances: preview.instances,
                backup_id: None,
                keyd_error: None,
                restored_profile: None,
            });
        }
        if preview.instances.iter().any(|x| x.status == "deferred") {
            if install {
                ensure_state(self.state.parent().unwrap().parent().unwrap()).map_err(err)?;
                self.write(&self.state.join("pending.json"), profile)?;
            }
            for item in &mut preview.instances {
                if item.status == "ready" {
                    item.status = "deferred".into();
                    item.detail = "waiting for Minecraft to close".into();
                }
            }
            return Ok(ApplyReport {
                instances: preview.instances,
                backup_id: None,
                keyd_error: None,
                restored_profile: None,
            });
        }
        let install = install && history.profile.as_ref() != Some(profile);
        if !install && history.profile.as_ref() != Some(profile) {
            return Err(err("reconciliation profile differs from installed profile"));
        }
        if !install
            && prepared
                .iter()
                .all(|item| !item.changed() && state.get(&item.instance.id) == Some(&item.state))
        {
            self.validate_prepared(&prepared, false)?;
            self.write(
                &self.state.join("mutation.json"),
                &serde_json::json!({"operation":"apply","backup_id":history.head}),
            )?;
            let operation = (|| {
                remove_record(&self.state.join("pending.json")).map_err(err)?;
                publish(profile)?;
                sync_dir(&self.state).map_err(err)?;
                fs::remove_file(self.state.join("mutation.json")).map_err(err)
            })();
            if let Err(error) = operation {
                let _ = self.write(
                    &self.state.join("mutation.json"),
                    &serde_json::json!({"operation":"apply","backup_id":history.head,"error":error.0}),
                );
                return Err(error);
            }
            return Ok(ApplyReport {
                instances: preview.instances,
                backup_id: None,
                keyd_error: None,
                restored_profile: None,
            });
        }
        ensure_state(self.state.parent().unwrap().parent().unwrap()).map_err(err)?;
        let backups = self.state.join("backups");
        if !backups.exists() {
            fs::create_dir(&backups).map_err(err)?;
            fs::set_permissions(&backups, fs::Permissions::from_mode(0o700)).map_err(err)?;
        }
        private_dir(&backups).map_err(err)?;
        let id = new_id();
        let backup_dir = backups.join(&id);
        fs::create_dir(&backup_dir).map_err(err)?;
        fs::set_permissions(&backup_dir, fs::Permissions::from_mode(0o700)).map_err(err)?;
        sync_dir(&backups).map_err(err)?;
        let mut post_state = state.clone();
        for item in &prepared {
            post_state.insert(item.instance.id.clone(), item.state.clone());
        }
        self.validate_prepared(&prepared, false)?;
        let mut files = BTreeMap::new();
        let mut mod_files = BTreeMap::new();
        let mod_dir = backup_dir.join("mod-configs");
        fs::create_dir(&mod_dir).map_err(err)?;
        fs::set_permissions(&mod_dir, fs::Permissions::from_mode(0o700)).map_err(err)?;
        for item in &prepared {
            let instance = &item.instance;
            atomic_replace(
                &backup_dir.join(format!("{}.txt", instance.id)),
                &item.old,
                true,
            )
            .map_err(err)?;
            let mut configs = BTreeMap::new();
            for edit in &item.configs {
                let kind = mod_kind(instance.options_path.parent().unwrap(), &edit.path)?;
                atomic_replace(
                    &mod_dir.join(format!("{}.{kind}.txt", instance.id)),
                    &edit.before,
                    true,
                )
                .map_err(err)?;
                configs.insert(
                    kind.to_string(),
                    BackupFile {
                        relative: edit
                            .path
                            .strip_prefix(&self.root)
                            .map_err(err)?
                            .to_path_buf(),
                        before_hash: hash(&edit.before),
                        after_hash: hash(&edit.after),
                    },
                );
            }
            mod_files.insert(instance.id.clone(), configs);
            files.insert(
                instance.id.clone(),
                BackupFile {
                    relative: instance
                        .options_path
                        .strip_prefix(&self.root)
                        .map_err(err)?
                        .to_path_buf(),
                    before_hash: hash(&item.old),
                    after_hash: hash(&item.new),
                },
            );
        }
        let mut manifest = Manifest {
            launcher_root: self.root.clone(),
            files,
            prior_state: state,
            post_state,
            prior_history: history.clone(),
            profile: profile.clone(),
            keyd_after: vec![],
            mod_files: Some(mod_files),
        };
        sync_dir(&mod_dir).map_err(err)?;
        self.write(&backup_dir.join("manifest.json"), &manifest)?;
        sync_dir(&backup_dir).map_err(err)?;
        sync_dir(&backups).map_err(err)?;
        self.write(
            &self.state.join("mutation.json"),
            &serde_json::json!({"operation":"apply","backup_id":id}),
        )?;
        let had_managed = record_present(&self.state.join("managed.json"))?;
        let had_lineage = record_present(&self.state.join("lineage.json"))?;
        let mut installed = false;
        let mut touched: Vec<(&Path, &[u8], &[u8])> = Vec::new();
        let operation = (|| {
            self.validate_prepared(&prepared, false)?;
            if install {
                keyd.install(profile).map_err(err)?;
                installed = true;
            }
            self.validate_prepared(&prepared, false)?;
            for item in &prepared {
                if item.old != item.new {
                    if fs::read(&item.instance.options_path).map_err(err)? != item.old {
                        return Err(err("options changed during apply"));
                    }
                    // A failed directory sync can occur after the rename, so include
                    // the attempted write in rollback before making the call.
                    touched.push((&item.instance.options_path, &item.old, &item.new));
                    atomic_replace(&item.instance.options_path, &item.new, false).map_err(err)?;
                }
                for edit in &item.configs {
                    if edit.before != edit.after {
                        let current = prepare_local(
                            item.instance.options_path.parent().unwrap(),
                            &Default::default(),
                        )
                        .map_err(err)?;
                        if !current
                            .iter()
                            .any(|config| config.path == edit.path && config.before == edit.before)
                        {
                            return Err(err("Standard Settings changed during apply"));
                        }
                        touched.push((&edit.path, &edit.before, &edit.after));
                        atomic_replace(&edit.path, &edit.after, false).map_err(err)?;
                    }
                }
            }
            self.validate_prepared(&prepared, true)?;
            self.write(&self.state.join("managed.json"), &manifest.post_state)?;
            manifest.keyd_after = keyd.checkpoint().map_err(err)?;
            self.write(&backup_dir.join("manifest.json"), &manifest)?;
            self.write(
                &self.state.join("lineage.json"),
                &History {
                    head: Some(id.clone()),
                    profile: Some(profile.clone()),
                    keyd: manifest.keyd_after.clone(),
                    launcher_root: self.root.clone(),
                },
            )?;
            remove_record(&self.state.join("pending.json")).map_err(err)?;
            publish(profile)?;
            self.validate_prepared(&prepared, true)?;
            sync_dir(&self.state).map_err(err)?;
            // Unlink last. A crash can resurrect a marker, but cannot erase it before
            // every published record and directory is durable.
            fs::remove_file(self.state.join("mutation.json")).map_err(err)
        })();
        if let Err(e) = operation {
            let mut rollback_errors = Vec::new();
            for (path, old, new) in touched.into_iter().rev() {
                let result = (|| {
                    // Recheck config ancestors before rollback as well as forward writes.
                    if let Some(item) = prepared
                        .iter()
                        .find(|item| item.configs.iter().any(|edit| edit.path == path))
                    {
                        prepare_local(
                            item.instance.options_path.parent().unwrap(),
                            &Default::default(),
                        )
                        .map_err(err)?;
                    }
                    let current = fs::read(path).map_err(err)?;
                    if current == old {
                        return Ok(());
                    }
                    if current != new {
                        return Err(err("changed managed file during apply rollback"));
                    }
                    atomic_replace(path, old, false).map_err(err)
                })();
                if let Err(error) = result {
                    rollback_errors.push(error.0);
                }
            }
            if installed {
                if let Err(error) = keyd.rollback() {
                    rollback_errors.push(error);
                }
            }
            let managed_path = self.state.join("managed.json");
            let managed_result = if had_managed {
                self.write(&managed_path, &manifest.prior_state)
            } else {
                remove_record(&managed_path).map_err(err)
            };
            if let Err(error) = managed_result {
                rollback_errors.push(error.0);
            }
            let lineage_path = self.state.join("lineage.json");
            let lineage_result = if had_lineage {
                self.write(&lineage_path, &history)
            } else {
                remove_record(&lineage_path).map_err(err)
            };
            if let Err(error) = lineage_result {
                rollback_errors.push(error.0);
            }
            let failure = if rollback_errors.is_empty() {
                e.0
            } else {
                format!("{}; rollback failed: {}", e.0, rollback_errors.join("; "))
            };
            let _ = self.write(
                &self.state.join("mutation.json"),
                &serde_json::json!({"operation":"apply","backup_id":id,"error":failure}),
            );
            return Err(err(failure));
        }
        for item in &mut preview.instances {
            if item.status == "ready" {
                item.status = "updated".into();
            }
        }
        Ok(ApplyReport {
            instances: preview.instances,
            backup_id: Some(id),
            keyd_error: None,
            restored_profile: None,
        })
    }
    pub fn restore(
        &self,
        backup_id: &str,
        keyd: &dyn KeydAdapter,
    ) -> Result<ApplyReport, ApplyError> {
        self.restore_with_publish(backup_id, keyd, &|_| Ok(()))
    }
    pub fn restore_with_publish(
        &self,
        backup_id: &str,
        keyd: &dyn KeydAdapter,
        publish: &dyn Fn(Option<&Profile>) -> Result<(), ApplyError>,
    ) -> Result<ApplyReport, ApplyError> {
        self.restore_with_hooks(backup_id, keyd, &|| Ok(()), publish)
    }
    pub fn restore_with_hooks(
        &self,
        backup_id: &str,
        keyd: &dyn KeydAdapter,
        before: &dyn Fn() -> Result<(), ApplyError>,
        publish: &dyn Fn(Option<&Profile>) -> Result<(), ApplyError>,
    ) -> Result<ApplyReport, ApplyError> {
        ensure_state(self.state.parent().unwrap().parent().unwrap()).map_err(err)?;
        let _lock = MutationLock::acquire(&self.state)?;
        self.guard()?;
        if !valid_id(backup_id) {
            return Err(err("unknown backup"));
        }
        let history = self.history(keyd)?;
        if history.head.as_deref() != Some(backup_id) {
            return Err(err("stale backup is not the current recovery head"));
        }
        let backup_dir = self.state.join("backups").join(backup_id);
        private_dir(&self.state.join("backups")).map_err(err)?;
        private_dir(&backup_dir).map_err(err)?;
        let manifest: Manifest = self.record(&backup_dir.join("manifest.json"))?;
        if manifest.launcher_root != self.root
            || manifest.profile
                != history
                    .profile
                    .clone()
                    .ok_or_else(|| err("missing profile"))?
            || manifest.keyd_after != history.keyd
            || manifest.post_state != self.managed()?
        {
            return Err(err("changed or ambiguous backup state"));
        }
        let prior = &manifest.prior_history;
        if prior.launcher_root != self.root
            || prior.head.is_some() != prior.profile.is_some()
            || !history.keyd.starts_with(&prior.keyd)
        {
            return Err(err("ambiguous backup lineage"));
        }
        let prior_files = if let Some(parent_id) = &prior.head {
            if !valid_id(parent_id) {
                return Err(err("invalid parent backup"));
            }
            private_dir(&self.state.join("backups").join(parent_id)).map_err(err)?;
            let parent: Manifest = self.record(
                &self
                    .state
                    .join("backups")
                    .join(parent_id)
                    .join("manifest.json"),
            )?;
            if parent.launcher_root != self.root
                || parent.profile != prior.profile.clone().unwrap()
                || parent.keyd_after != prior.keyd
                || parent.post_state != manifest.prior_state
            {
                return Err(err("changed parent backup lineage"));
            }
            for (id, file) in &parent.files {
                if manifest
                    .files
                    .get(id)
                    .is_none_or(|current| current.before_hash != file.after_hash)
                {
                    return Err(err("changed parent backup lineage"));
                }
            }
            // Mod configs can drift independently between applies. Validate each
            // generation against its own snapshot so a repair remains reversible.
            // Restoring the parent still checks its own after hashes below.
            parent.files
        } else {
            if !prior.keyd.is_empty() || !manifest.prior_state.is_empty() {
                return Err(err("ambiguous baseline lineage"));
            }
            BTreeMap::new()
        };
        let instances = discover_instances(&self.root).map_err(err)?;
        if instances.len() != manifest.files.len() {
            return Err(err("changed launcher instances"));
        }
        let mod_restores = self.restore_mods(&manifest, &backup_dir, &instances)?;
        let mut restores = vec![];
        for instance in &instances {
            let file = manifest
                .files
                .get(&instance.id)
                .ok_or_else(|| err("changed launcher instances"))?;
            let target = self.root.join(&file.relative);
            if instance.problem.is_some()
                || target != instance.options_path
                || self.running(&instance)
            {
                return Err(err("unsafe or running restore target"));
            }
            let before =
                safe_record(&backup_dir.join(format!("{}.txt", instance.id))).map_err(err)?;
            if hash(&before) != file.before_hash
                || hash(&fs::read(&target).map_err(err)?) != file.after_hash
            {
                return Err(err("changed options or backup contents"));
            }
            // A newly discovered instance predates its first managed profile.
            // Keep the prior-state check as well so removing a parent file entry
            // cannot reclassify an existing managed instance as new.
            if let Some(profile) = prior.profile.as_ref().filter(|_| {
                prior_files.contains_key(&instance.id)
                    || manifest.prior_state.contains_key(&instance.id)
            }) {
                let doc = parse_options(&before).map_err(err)?;
                let plan = plan_controls(&doc, profile, manifest.prior_state.get(&instance.id))
                    .map_err(err)?;
                if !plan.conflicts.is_empty() || render_options(&doc, &plan) != before {
                    return Err(err("backup options do not match prior profile"));
                }
            }
            restores.push((instance.id.clone(), target, before));
        }
        before()?;
        // Hooks can have side effects; do not overwrite files changed since preflight.
        let validate = || {
            if discover_instances(&self.root).map_err(err)? != instances {
                return Err(err("changed launcher instances during restore"));
            }
            for (id, target, _) in &restores {
                if hash(&fs::read(target).map_err(err)?) != manifest.files[id].after_hash {
                    return Err(err("changed options during restore"));
                }
            }
            if self.restore_mods(&manifest, &backup_dir, &instances)? != mod_restores {
                return Err(err("changed Standard Settings during restore"));
            }
            Ok(())
        };
        validate()?;
        self.write(
            &self.state.join("mutation.json"),
            &serde_json::json!({"operation":"restore","backup_id":backup_id}),
        )?;
        let operation = (|| {
            keyd.restore_to(&prior.keyd).map_err(err)?;
            validate()?;
            for (_, target, before) in &restores {
                atomic_replace(target, before, false).map_err(err)?;
            }
            for edit in &mod_restores {
                atomic_replace(&edit.path, &edit.after, false).map_err(err)?;
            }
            self.write(&self.state.join("managed.json"), &manifest.prior_state)?;
            self.write(&self.state.join("lineage.json"), prior)?;
            remove_record(&self.state.join("pending.json")).map_err(err)?;
            publish(prior.profile.as_ref())?;
            sync_dir(&self.state).map_err(err)?;
            fs::remove_file(self.state.join("mutation.json")).map_err(err)
        })();
        if let Err(e) = operation {
            let _ = self.write(
                &self.state.join("mutation.json"),
                &serde_json::json!({"operation":"restore","backup_id":backup_id,"error":e.0}),
            );
            return Err(e);
        }
        Ok(ApplyReport {
            instances: restores
                .into_iter()
                .map(|(instance_id, _, _)| InstanceResult {
                    instance_id,
                    status: "restored".into(),
                    detail: String::new(),
                })
                .collect(),
            backup_id: Some(backup_id.into()),
            keyd_error: None,
            restored_profile: prior.profile.clone(),
        })
    }
    fn restore_mods(
        &self,
        manifest: &Manifest,
        backup_dir: &Path,
        instances: &[Instance],
    ) -> Result<Vec<ConfigEdit>, ApplyError> {
        let Some(mod_files) = &manifest.mod_files else {
            return Ok(vec![]);
        };
        if !mod_files.keys().eq(manifest.files.keys()) {
            return Err(err("changed Standard Settings backup instance set"));
        }
        let mut restores = vec![];
        for instance in instances {
            let files = mod_files
                .get(&instance.id)
                .ok_or_else(|| err("missing Standard Settings backup instance"))?;
            let game = instance.options_path.parent().unwrap();
            let configs = prepare_local(game, &Default::default()).map_err(err)?;
            if configs.len() != files.len() {
                return Err(err("changed Standard Settings file set"));
            }
            if !configs.is_empty() {
                private_dir(&backup_dir.join("mod-configs")).map_err(err)?;
            }
            for config in configs {
                let kind = mod_kind(game, &config.path)?;
                let file = files
                    .get(kind)
                    .ok_or_else(|| err("changed Standard Settings file set"))?;
                if self.root.join(&file.relative) != config.path {
                    return Err(err("unsafe Standard Settings restore path"));
                }
                let before = safe_record(
                    &backup_dir
                        .join("mod-configs")
                        .join(format!("{}.{kind}.txt", instance.id)),
                )
                .map_err(err)?;
                if hash(&before) != file.before_hash || hash(&config.before) != file.after_hash {
                    return Err(err("changed Standard Settings or backup contents"));
                }
                restores.push(ConfigEdit {
                    path: config.path,
                    before: config.before,
                    after: before,
                });
            }
        }
        Ok(restores)
    }
}
fn mod_kind(game: &Path, path: &Path) -> Result<&'static str, ApplyError> {
    if path == game.join("config/mcsr/standardsettings.json") {
        Ok("json")
    } else if path == game.join("config/standardoptions.txt") {
        Ok("legacy")
    } else {
        Err(err("unknown Standard Settings target"))
    }
}
fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn record_present(path: &Path) -> Result<bool, ApplyError> {
    match path.symlink_metadata() {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(err(e)),
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn new_id() -> String {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{:032x}",
        ns ^ u128::from(NEXT_BACKUP.fetch_add(1, Ordering::Relaxed))
    )
}
