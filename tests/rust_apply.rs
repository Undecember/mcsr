use mcsr_toolkit::apply::{ApplyCoordinator, KeydAdapter};
use mcsr_toolkit::profile::{parse_profile, KeySet, Profile};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    base: PathBuf,
    root: PathBuf,
    data: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "mcsr-apply-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = base.join("launcher");
        let data = base.join("data");
        fs::create_dir_all(root.join("instances")).unwrap();
        Self { base, root, data }
    }
    fn instance(&self, id: &str, bytes: &[u8]) -> PathBuf {
        let dir = self.root.join("instances").join(id);
        fs::create_dir_all(dir.join("minecraft")).unwrap();
        fs::write(dir.join("instance.json"), format!(r#"{{"id":"{id}"}}"#)).unwrap();
        let path = dir.join("minecraft/options.txt");
        fs::write(&path, bytes).unwrap();
        path
    }
    fn coordinator(&self, running: Vec<PathBuf>) -> ApplyCoordinator {
        ApplyCoordinator::new(&self.root, &self.data, running).unwrap()
    }
    fn state(&self) -> PathBuf {
        self.data.join("mcsr-toolkit/rust-v1")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}

#[derive(Default)]
struct FakeKeyd {
    snapshots: Mutex<Vec<String>>,
    installs: AtomicU64,
}
impl KeydAdapter for FakeKeyd {
    fn install(&self, _: &Profile) -> Result<(), String> {
        self.installs.fetch_add(1, Ordering::Relaxed);
        self.snapshots.lock().unwrap().push(format!(
            "snapshot-{}",
            self.installs.load(Ordering::Relaxed)
        ));
        Ok(())
    }
    fn checkpoint(&self) -> Result<Vec<String>, String> {
        Ok(self.snapshots.lock().unwrap().clone())
    }
    fn restore_to(&self, to: &[String]) -> Result<(), String> {
        *self.snapshots.lock().unwrap() = to.to_vec();
        Ok(())
    }
    fn rollback(&self) -> Result<(), String> {
        self.snapshots.lock().unwrap().pop();
        Ok(())
    }
}
fn profile(target: &str) -> Profile {
    let mut p = Profile::default();
    p.crafting.insert("s".into(), target.into());
    p
}
fn original() -> &'static [u8] {
    b"gamma:1.0\r\nkey_key.back:key.keyboard.s\r\n"
}
fn status(report: &mcsr_toolkit::apply::ApplyReport, id: &str) -> String {
    report
        .instances
        .iter()
        .find(|x| x.instance_id == id)
        .unwrap()
        .status
        .clone()
}

#[test]
fn enter_chat_preserves_hotbar_through_apply_and_restore() {
    let baseline = b"gamma:1.0\r\nkey_key.back:key.keyboard.s\r\nkey_key.chat:key.keyboard.enter\r\nkey_key.command:key.keyboard.slash\r\nkey_key.hotbar.7:key.keyboard.t\r\n";
    let applied = b"gamma:1.0\r\nkey_key.back:key.keyboard.k\r\nkey_key.chat:key.keyboard.enter\r\nkey_key.command:key.keyboard.slash\r\nkey_key.hotbar.7:key.keyboard.t\r\n";
    let profile = parse_profile(
        br#"{"crafting":{"s":"k"},"chat":{"open_chat":"enter","send":"enter"}}"#,
        &KeySet::fallback(),
    )
    .unwrap();
    let fixture = Fixture::new();
    let options = fixture.instance("one", baseline);
    let keyd = FakeKeyd::default();
    let coordinator = fixture.coordinator(vec![]);

    let report = coordinator.apply(&profile, &keyd).unwrap();
    assert_eq!(status(&report, "one"), "updated");
    assert_eq!(fs::read(&options).unwrap(), applied);
    let backup_id = report.backup_id.as_deref().unwrap();
    let restored = coordinator.restore(backup_id, &keyd).unwrap();
    assert_eq!(status(&restored, "one"), "restored");
    assert_eq!(restored.restored_profile, None);
    assert_eq!(fs::read(&options).unwrap(), baseline);
    let lineage: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.state().join("lineage.json")).unwrap()).unwrap();
    assert!(lineage["head"].is_null());
    assert!(lineage["profile"].is_null());
    assert!(keyd.checkpoint().unwrap().is_empty());
}

#[test]
fn applies_two_closed_instances_and_restores_exact_bytes() {
    let f = Fixture::new();
    let one = f.instance("one", original());
    let two = f.instance("two", original());
    let keyd = FakeKeyd::default();
    let coordinator = f.coordinator(vec![]);
    let report = coordinator.apply(&profile("k"), &keyd).unwrap();
    assert_eq!(status(&report, "one"), "updated");
    assert_eq!(status(&report, "two"), "updated");
    assert!(fs::read(&one)
        .unwrap()
        .ends_with(b"key_key.back:key.keyboard.k\r\n"));
    coordinator
        .restore(report.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(one).unwrap(), original());
    assert_eq!(fs::read(two).unwrap(), original());
}

#[test]
fn reconciliation_publication_failure_keeps_mutation_marker() {
    let f = Fixture::new();
    f.instance("one", original());
    let keyd = FakeKeyd::default();
    let coordinator = f.coordinator(vec![]);
    coordinator.apply(&profile("k"), &keyd).unwrap();
    f.instance("two", original());
    let error = coordinator
        .reconcile_new_instances_with_publish(&profile("k"), &keyd, &|_| {
            Err(mcsr_toolkit::apply::ApplyError("publish failed".into()))
        })
        .unwrap_err();
    assert!(error.to_string().contains("publish failed"));
    assert!(f.state().join("mutation.json").exists());
}

#[test]
fn running_instance_defers_all_activation() {
    let f = Fixture::new();
    let one = f.instance("one", original());
    let two = f.instance("two", original());
    let keyd = FakeKeyd::default();
    let running = two.parent().unwrap().to_path_buf();
    let report = f
        .coordinator(vec![running])
        .apply(&profile("k"), &keyd)
        .unwrap();
    assert_eq!(status(&report, "one"), "deferred");
    assert_eq!(status(&report, "two"), "deferred");
    assert_eq!(fs::read(one).unwrap(), original());
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 0);
}

#[test]
fn idempotent_apply_does_not_add_backup_or_keyd_snapshot() {
    let f = Fixture::new();
    f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let first = c.apply(&profile("k"), &keyd).unwrap();
    let second = c.apply(&profile("k"), &keyd).unwrap();
    assert!(first.backup_id.is_some());
    assert!(second.backup_id.is_none());
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 1);
}

#[test]
fn reconciliation_updates_only_new_instance() {
    let f = Fixture::new();
    let one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    c.apply(&profile("k"), &keyd).unwrap();
    let prior = fs::read(&one).unwrap();
    let two = f.instance("two", original());
    let report = c.reconcile_new_instances(&profile("k"), &keyd).unwrap();
    assert_eq!(status(&report, "two"), "updated");
    assert_eq!(fs::read(one).unwrap(), prior);
    assert!(fs::read(two)
        .unwrap()
        .ends_with(b"key_key.back:key.keyboard.k\r\n"));
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 1);
}

#[test]
fn restores_reconciled_new_instance_before_original_generation() {
    let f = Fixture::new();
    let one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let first = c.apply(&profile("k"), &keyd).unwrap();
    let applied_one = fs::read(&one).unwrap();
    let applied_keyd = keyd.checkpoint().unwrap();
    let new_original = b"gamma:0.5\nkey_key.back:key.keyboard.s\n";
    let two = f.instance("two", new_original);
    let second = c.reconcile_new_instances(&profile("k"), &keyd).unwrap();
    let applied_two = fs::read(&two).unwrap();
    let first_id = first.backup_id.as_deref().unwrap();
    let second_id = second.backup_id.as_deref().unwrap();

    assert!(c.restore(first_id, &keyd).is_err());
    assert_eq!(fs::read(&one).unwrap(), applied_one);
    assert_eq!(fs::read(&two).unwrap(), applied_two);
    assert_eq!(keyd.checkpoint().unwrap(), applied_keyd);

    let restored = c.restore(second_id, &keyd).unwrap();
    assert_eq!(restored.restored_profile, Some(profile("k")));
    assert_eq!(fs::read(&one).unwrap(), applied_one);
    assert_eq!(fs::read(&two).unwrap(), new_original);
    assert_eq!(keyd.checkpoint().unwrap(), applied_keyd);
    assert!(!f.state().join("mutation.json").exists());
    assert!(c.restore(second_id, &keyd).is_err());

    // The older generation must still require its exact instance set.
    assert!(c.restore(first_id, &keyd).is_err());
    assert_eq!(fs::read(&one).unwrap(), applied_one);
    assert_eq!(fs::read(&two).unwrap(), new_original);
    assert_eq!(keyd.checkpoint().unwrap(), applied_keyd);
    fs::remove_dir_all(f.root.join("instances/two")).unwrap();
    let restored = c.restore(first_id, &keyd).unwrap();
    assert_eq!(restored.restored_profile, None);
    assert_eq!(fs::read(&one).unwrap(), original());
    assert!(keyd.checkpoint().unwrap().is_empty());
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 1);
    assert!(!f.state().join("mutation.json").exists());
}

#[test]
fn restore_does_not_reclassify_existing_instance_from_changed_parent_files() {
    use sha2::{Digest, Sha256};

    for remove_parent_file in [false, true] {
        let f = Fixture::new();
        let one = f.instance("one", original());
        let keyd = FakeKeyd::default();
        let c = f.coordinator(vec![]);
        let first = c.apply(&profile("k"), &keyd).unwrap();
        let second = c.apply(&profile("j"), &keyd).unwrap();
        let current = fs::read(&one).unwrap();
        let checkpoint = keyd.checkpoint().unwrap();
        let parent = f.state().join("backups").join(first.backup_id.unwrap());
        let id = second.backup_id.unwrap();
        let backup = f.state().join("backups").join(&id);
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(backup.join("manifest.json")).unwrap()).unwrap();
        let mut parent_manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(parent.join("manifest.json")).unwrap()).unwrap();
        let forged_hash = format!("{:x}", Sha256::digest(original()));
        fs::write(backup.join("one.txt"), original()).unwrap();
        manifest["files"]["one"]["before_hash"] = forged_hash.clone().into();
        if remove_parent_file {
            parent_manifest["files"]
                .as_object_mut()
                .unwrap()
                .remove("one");
        } else {
            parent_manifest["files"]["one"]["after_hash"] = forged_hash.into();
        }
        fs::write(
            backup.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(
            parent.join("manifest.json"),
            serde_json::to_vec(&parent_manifest).unwrap(),
        )
        .unwrap();

        let error = c.restore(&id, &keyd).unwrap_err();
        assert!(error
            .0
            .contains("backup options do not match prior profile"));
        assert_eq!(fs::read(&one).unwrap(), current);
        assert_eq!(keyd.checkpoint().unwrap(), checkpoint);
        assert!(!f.state().join("mutation.json").exists());
    }
}

#[test]
fn new_instance_restore_preserves_preflight_guards() {
    use std::os::unix::fs::symlink;

    for guard in ["backup", "options", "instances", "running", "symlink"] {
        let f = Fixture::new();
        let one = f.instance("one", original());
        let keyd = FakeKeyd::default();
        let c = f.coordinator(vec![]);
        c.apply(&profile("k"), &keyd).unwrap();
        let two = f.instance("two", original());
        let report = c.reconcile_new_instances(&profile("k"), &keyd).unwrap();
        let id = report.backup_id.unwrap();
        let mut running = vec![];
        match guard {
            "backup" => {
                fs::write(
                    f.state().join("backups").join(&id).join("two.txt"),
                    b"changed",
                )
                .unwrap();
            }
            "options" => fs::write(&two, b"changed").unwrap(),
            "instances" => {
                f.instance("three", original());
            }
            "running" => running.push(two.parent().unwrap().to_path_buf()),
            "symlink" => {
                let moved = f.base.join("moved-options.txt");
                fs::rename(&two, &moved).unwrap();
                symlink(moved, &two).unwrap();
            }
            _ => unreachable!(),
        }
        let current_one = fs::read(&one).unwrap();
        let current_two = fs::read(&two).unwrap();
        let checkpoint = keyd.checkpoint().unwrap();
        let lineage = fs::read(f.state().join("lineage.json")).unwrap();
        let managed = fs::read(f.state().join("managed.json")).unwrap();
        let before_called = std::cell::Cell::new(false);
        assert!(
            f.coordinator(running)
                .restore_with_hooks(
                    &id,
                    &keyd,
                    &|| {
                        before_called.set(true);
                        Ok(())
                    },
                    &|_| Ok(())
                )
                .is_err(),
            "{guard}"
        );
        assert!(!before_called.get(), "{guard}");
        assert_eq!(fs::read(&one).unwrap(), current_one, "{guard}");
        assert_eq!(fs::read(&two).unwrap(), current_two, "{guard}");
        assert_eq!(keyd.checkpoint().unwrap(), checkpoint, "{guard}");
        assert_eq!(fs::read(f.state().join("lineage.json")).unwrap(), lineage);
        assert_eq!(fs::read(f.state().join("managed.json")).unwrap(), managed);
        assert!(!f.state().join("mutation.json").exists(), "{guard}");
    }
}

#[test]
fn restore_requires_lifo_and_rejects_changed_file() {
    let f = Fixture::new();
    let one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let first = c.apply(&profile("k"), &keyd).unwrap();
    let second = c.apply(&profile("j"), &keyd).unwrap();
    assert!(c
        .restore(first.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
    fs::write(&one, b"changed").unwrap();
    assert!(c
        .restore(second.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
    assert_eq!(fs::read(&one).unwrap(), b"changed");
}

#[test]
fn restores_two_generations_in_lifo_order() {
    let f = Fixture::new();
    let one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let first = c.apply(&profile("k"), &keyd).unwrap();
    let second = c.apply(&profile("j"), &keyd).unwrap();
    c.restore(second.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert!(fs::read(&one)
        .unwrap()
        .ends_with(b"key_key.back:key.keyboard.k\r\n"));
    c.restore(first.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(one).unwrap(), original());
    assert!(keyd.checkpoint().unwrap().is_empty());
}

#[test]
fn pending_apply_reconciles_after_running_instance_closes() {
    let f = Fixture::new();
    let one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    f.coordinator(vec![one.parent().unwrap().to_path_buf()])
        .apply(&profile("k"), &keyd)
        .unwrap();
    assert!(f.state().join("pending.json").is_file());
    let report = f
        .coordinator(vec![])
        .reconcile_new_instances(&Profile::default(), &keyd)
        .unwrap();
    assert_eq!(status(&report, "one"), "updated");
    assert!(!f.state().join("pending.json").exists());
}

#[test]
fn state_directory_symlink_blocks_preview_and_apply() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    c.apply(&profile("k"), &keyd).unwrap();
    let actual = f.data.join("mcsr-toolkit/moved");
    fs::rename(f.state(), &actual).unwrap();
    symlink(&actual, f.state()).unwrap();
    assert!(c.preview(&profile("k")).is_err());
    assert!(c.apply(&profile("k"), &keyd).is_err());
}

#[test]
fn existing_mutation_lock_blocks_another_apply() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.instance("one", original());
    fs::create_dir_all(f.state()).unwrap();
    fs::set_permissions(
        f.data.join("mcsr-toolkit"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    fs::set_permissions(f.state(), fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(f.state().join("mutation.lock"), b"").unwrap();
    let keyd = FakeKeyd::default();
    assert!(f.coordinator(vec![]).apply(&profile("k"), &keyd).is_err());
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 0);
}

#[test]
fn backup_is_bound_to_launcher_root() {
    let f = Fixture::new();
    f.instance("one", original());
    let keyd = FakeKeyd::default();
    let report = f.coordinator(vec![]).apply(&profile("k"), &keyd).unwrap();
    let other = f.base.join("other");
    fs::rename(&f.root, &other).unwrap();
    let c = ApplyCoordinator::new(&other, &f.data, vec![]).unwrap();
    assert!(c
        .restore(report.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
}

#[test]
fn symlink_launcher_root_is_rejected_at_construction() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    f.instance("one", original());
    let linked = f.base.join("linked-launcher");
    symlink(&f.root, &linked).unwrap();
    assert!(ApplyCoordinator::new(&linked, &f.data, vec![]).is_err());
}

#[test]
fn dangling_recovery_record_symlinks_block_apply() {
    use std::os::unix::fs::symlink;
    for record in ["lineage.json", "managed.json"] {
        let f = Fixture::new();
        f.instance("one", original());
        fs::create_dir_all(f.state()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            f.data.join("mcsr-toolkit"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        fs::set_permissions(f.state(), fs::Permissions::from_mode(0o700)).unwrap();
        symlink(f.state().join("missing"), f.state().join(record)).unwrap();
        let keyd = FakeKeyd::default();
        assert!(
            f.coordinator(vec![]).apply(&profile("k"), &keyd).is_err(),
            "{record}"
        );
        assert_eq!(keyd.installs.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn symlink_backups_ancestor_blocks_restore() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let options = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let report = c.apply(&profile("k"), &keyd).unwrap();
    let before = fs::read(&options).unwrap();
    let moved = f.state().join("moved-backups");
    fs::rename(f.state().join("backups"), &moved).unwrap();
    symlink(&moved, f.state().join("backups")).unwrap();
    assert!(c
        .restore(report.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
    assert_eq!(fs::read(options).unwrap(), before);
}

#[test]
fn mapping_conflict_returns_failure_without_panic_or_keyd_install() {
    let f = Fixture::new();
    f.instance(
        "one",
        b"key_key.back:key.keyboard.s\nkey_key.forward:key.keyboard.w\n",
    );
    let keyd = FakeKeyd::default();
    let mut p = profile("k");
    p.controls.insert("key_key.forward".into(), "s".into());
    let report = f.coordinator(vec![]).apply(&p, &keyd).unwrap();
    assert_eq!(status(&report, "one"), "failed");
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 0);
}

#[test]
fn uncertain_failure_keeps_durable_marker_and_blocks_restarted_apply() {
    struct Sabotage {
        target: PathBuf,
        snapshots: Mutex<Vec<String>>,
    }
    impl KeydAdapter for Sabotage {
        fn install(&self, _: &Profile) -> Result<(), String> {
            self.snapshots.lock().unwrap().push("installed".into());
            fs::remove_file(&self.target).unwrap();
            fs::create_dir(&self.target).unwrap();
            Ok(())
        }
        fn checkpoint(&self) -> Result<Vec<String>, String> {
            Ok(self.snapshots.lock().unwrap().clone())
        }
        fn restore_to(&self, _: &[String]) -> Result<(), String> {
            Ok(())
        }
        fn rollback(&self) -> Result<(), String> {
            self.snapshots.lock().unwrap().pop();
            Ok(())
        }
    }
    let f = Fixture::new();
    let one = f.instance("one", original());
    let two = f.instance("two", original());
    let keyd = Sabotage {
        target: two,
        snapshots: Mutex::new(vec![]),
    };
    let c = f.coordinator(vec![]);
    assert!(c.apply(&profile("k"), &keyd).is_err());
    assert_eq!(fs::read(one).unwrap(), original());
    assert!(keyd.snapshots.lock().unwrap().is_empty());
    assert!(f.state().join("mutation.json").is_file());
    assert!(f
        .coordinator(vec![])
        .apply(&profile("k"), &FakeKeyd::default())
        .is_err());
}

struct FailingCheckpointKeyd {
    snapshots: Mutex<Vec<String>>,
    calls: AtomicU64,
    rollback_fails: bool,
}
impl KeydAdapter for FailingCheckpointKeyd {
    fn install(&self, _: &Profile) -> Result<(), String> {
        self.snapshots.lock().unwrap().push("installed".into());
        Ok(())
    }
    fn checkpoint(&self) -> Result<Vec<String>, String> {
        if self.calls.fetch_add(1, Ordering::Relaxed) > 0 {
            Err("checkpoint failed".into())
        } else {
            Ok(self.snapshots.lock().unwrap().clone())
        }
    }
    fn restore_to(&self, _: &[String]) -> Result<(), String> {
        Ok(())
    }
    fn rollback(&self) -> Result<(), String> {
        if self.rollback_fails {
            Err("rollback failed".into())
        } else {
            self.snapshots.lock().unwrap().pop();
            Ok(())
        }
    }
}

#[test]
fn publication_failure_rolls_back_options_and_keyd_but_retains_marker() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let keyd = FailingCheckpointKeyd {
        snapshots: Mutex::new(vec![]),
        calls: AtomicU64::new(0),
        rollback_fails: false,
    };
    assert!(f.coordinator(vec![]).apply(&profile("k"), &keyd).is_err());
    assert_eq!(fs::read(options).unwrap(), original());
    assert!(keyd.snapshots.lock().unwrap().is_empty());
    assert!(f.state().join("mutation.json").is_file());
}

#[test]
fn failed_keyd_rollback_keeps_marker_and_reports_both_failures() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let keyd = FailingCheckpointKeyd {
        snapshots: Mutex::new(vec![]),
        calls: AtomicU64::new(0),
        rollback_fails: true,
    };
    let error = f
        .coordinator(vec![])
        .apply(&profile("k"), &keyd)
        .unwrap_err();
    assert!(error.0.contains("checkpoint failed"));
    assert!(error.0.contains("rollback failed"));
    assert_eq!(fs::read(options).unwrap(), original());
    assert_eq!(keyd.snapshots.lock().unwrap().as_slice(), ["installed"]);
    assert!(f.state().join("mutation.json").is_file());
}

#[test]
fn restore_publication_failure_keeps_mutation_marker() {
    let f = Fixture::new();
    let _one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let report = c.apply(&profile("k"), &keyd).unwrap();
    let id = report.backup_id.as_deref().unwrap();
    let failed = c.restore_with_publish(id, &keyd, &|restored| {
        assert!(restored.is_none());
        assert!(f.state().join("mutation.json").exists());
        Err(mcsr_toolkit::apply::ApplyError(
            "profile directory sync failed".into(),
        ))
    });
    assert!(failed
        .unwrap_err()
        .to_string()
        .contains("profile directory sync failed"));
    assert!(f.state().join("mutation.json").exists());
}

#[test]
fn apply_publication_runs_under_marker_and_failure_keeps_marker() {
    let f = Fixture::new();
    let _one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let failure = c.apply_with_publish(&profile("k"), &keyd, &|_| {
        assert!(f.state().join("mutation.json").exists());
        Err(mcsr_toolkit::apply::ApplyError(
            "runtime publish failed".into(),
        ))
    });
    assert!(failure
        .unwrap_err()
        .to_string()
        .contains("runtime publish failed"));
    assert!(f.state().join("mutation.json").exists());
}

#[test]
fn before_restore_runs_after_validation_and_before_marker() {
    let f = Fixture::new();
    let _one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let report = c.apply(&profile("k"), &keyd).unwrap();
    let observed = std::cell::Cell::new(false);
    c.restore_with_hooks(
        report.backup_id.as_deref().unwrap(),
        &keyd,
        &|| {
            assert!(!f.state().join("mutation.json").exists());
            observed.set(true);
            Ok(())
        },
        &|_| Ok(()),
    )
    .unwrap();
    assert!(observed.get());
}

#[test]
fn idempotent_apply_still_publishes_runtime_state_under_marker() {
    let f = Fixture::new();
    let _one = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let p = profile("k");
    c.apply(&p, &keyd).unwrap();
    let called = std::cell::Cell::new(false);
    c.apply_with_publish(&p, &keyd, &|_| {
        assert!(f.state().join("mutation.json").exists());
        called.set(true);
        Ok(())
    })
    .unwrap();
    assert!(called.get());
    assert!(!f.state().join("mutation.json").exists());
}

fn standard_file(options: &std::path::Path, relative: &str, bytes: &[u8]) -> PathBuf {
    let path = options.parent().unwrap().join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn standard_settings_sync_effective_controls_and_restore_exact_bytes() {
    let f = Fixture::new();
    let options = f.instance("one", b"key_key.back:key.keyboard.s\r\nkey_key.forward:key.keyboard.w\r\nkey_key.chat:key.keyboard.t\r\n");
    let original_json = br#"{ "key_key.back":"key.keyboard.s", "key_key.forward":{"enabled":true,"value":"key.keyboard.w","extra":42}, "key_key.chat":"key.keyboard.t", "key_key.jump":{"enabled":false,"value":"key.keyboard.space"}, "resetOnWorldCreation":false, "other":17 }"#;
    let json = standard_file(&options, "config/mcsr/standardsettings.json", original_json);
    let p = parse_profile(br#"{"crafting":{"s":"k"},"controls":{"key_key.forward":"u"},"chat":{"open_chat":"enter","send":"enter"}}"#, &KeySet::fallback()).unwrap();
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let report = c.apply(&p, &keyd).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
    assert_eq!(value["key_key.back"], "key.keyboard.k");
    assert_eq!(value["key_key.forward"]["value"], "key.keyboard.u");
    assert_eq!(value["key_key.forward"]["extra"], 42);
    assert_eq!(value["key_key.chat"], "key.keyboard.enter");
    assert_eq!(
        value["key_key.jump"],
        serde_json::json!({"enabled":false,"value":"key.keyboard.space"})
    );
    assert_eq!(value["resetOnWorldCreation"], false);
    assert_eq!(value["other"], 17);
    c.restore(report.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(json).unwrap(), original_json);
    assert_eq!(fs::read(options).unwrap(), b"key_key.back:key.keyboard.s\r\nkey_key.forward:key.keyboard.w\r\nkey_key.chat:key.keyboard.t\r\n");
}

#[test]
fn standard_legacy_preserves_crlf_disabled_and_restores_original() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let before = b"# keep\r\nkey_key.back:key.keyboard.s\r\nkey_key.jump:\r\ngamma:0.7\r\n";
    let legacy = standard_file(&options, "config/standardoptions.txt", before);
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let report = c.apply(&profile("k"), &keyd).unwrap();
    assert_eq!(
        fs::read(&legacy).unwrap(),
        b"# keep\r\nkey_key.back:key.keyboard.k\r\nkey_key.jump:\r\ngamma:0.7\r\n"
    );
    c.restore(report.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(legacy).unwrap(), before);
}

#[test]
fn stale_standard_settings_are_repaired_even_when_options_are_current() {
    let f = Fixture::new();
    let options = f.instance("one", b"key_key.back:key.keyboard.k\n");
    let json = standard_file(
        &options,
        "config/mcsr/standardsettings.json",
        br#"{"key_key.back":"key.keyboard.s"}"#,
    );
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let first = c.apply(&profile("k"), &keyd).unwrap();
    assert_eq!(status(&first, "one"), "updated");
    assert!(first.backup_id.is_some());
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
    assert_eq!(value["key_key.back"], "key.keyboard.k");
    let second = c.apply(&profile("k"), &keyd).unwrap();
    assert!(second.backup_id.is_none());
}

#[test]
fn missing_standard_configs_stay_absent() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    f.coordinator(vec![])
        .apply(&profile("k"), &FakeKeyd::default())
        .unwrap();
    assert!(!options
        .parent()
        .unwrap()
        .join("config/mcsr/standardsettings.json")
        .exists());
    assert!(!options
        .parent()
        .unwrap()
        .join("config/standardoptions.txt")
        .exists());
}

#[test]
fn running_instance_defers_mod_only_change_for_all_instances() {
    let f = Fixture::new();
    let one = f.instance("one", b"key_key.back:key.keyboard.k\n");
    let two = f.instance("two", original());
    let json = standard_file(
        &one,
        "config/mcsr/standardsettings.json",
        br#"{"key_key.back":"key.keyboard.s"}"#,
    );
    let keyd = FakeKeyd::default();
    let report = f
        .coordinator(vec![one.parent().unwrap().to_path_buf()])
        .apply(&profile("k"), &keyd)
        .unwrap();
    assert_eq!(status(&report, "one"), "deferred");
    assert_eq!(status(&report, "two"), "deferred");
    assert_eq!(fs::read(two).unwrap(), original());
    assert_eq!(
        fs::read(json).unwrap(),
        br#"{"key_key.back":"key.keyboard.s"}"#
    );
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 0);
}

#[test]
fn invalid_or_shared_standard_config_blocks_entire_apply() {
    use std::os::unix::fs::symlink;
    for kind in ["malformed", "symlink", "modern_redirect", "legacy_redirect"] {
        let f = Fixture::new();
        let one = f.instance("one", original());
        let two = f.instance("two", original());
        let shared = f.base.join("shared.txt");
        fs::write(&shared, b"shared sentinel").unwrap();
        match kind {
            "malformed" => {
                standard_file(&two, "config/mcsr/standardsettings.json", b"{");
            }
            "symlink" => {
                let path = two
                    .parent()
                    .unwrap()
                    .join("config/mcsr/standardsettings.json");
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                symlink(&shared, path).unwrap();
            }
            "modern_redirect" => {
                standard_file(
                    &two,
                    "config/mcsr/standardsettings.global",
                    shared.to_str().unwrap().as_bytes(),
                );
            }
            "legacy_redirect" => {
                standard_file(
                    &two,
                    "config/standardoptions.txt",
                    shared.to_str().unwrap().as_bytes(),
                );
            }
            _ => unreachable!(),
        }
        let keyd = FakeKeyd::default();
        let result = f.coordinator(vec![]).apply(&profile("k"), &keyd);
        assert!(
            result.is_err() || status(result.as_ref().unwrap(), "two") == "failed",
            "{kind}"
        );
        assert_eq!(fs::read(one).unwrap(), original(), "{kind}");
        assert_eq!(fs::read(two).unwrap(), original(), "{kind}");
        assert_eq!(fs::read(shared).unwrap(), b"shared sentinel", "{kind}");
        assert_eq!(keyd.installs.load(Ordering::Relaxed), 0, "{kind}");
    }
}

#[test]
fn changed_standard_file_rejects_restore_before_any_mutation() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let json = standard_file(
        &options,
        "config/mcsr/standardsettings.json",
        br#"{"key_key.back":"key.keyboard.s"}"#,
    );
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let report = c.apply(&profile("k"), &keyd).unwrap();
    fs::write(&json, br#"{"key_key.back":"key.keyboard.x"}"#).unwrap();
    let options_before = fs::read(&options).unwrap();
    let checkpoint = keyd.checkpoint().unwrap();
    assert!(c
        .restore(report.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
    assert_eq!(fs::read(options).unwrap(), options_before);
    assert_eq!(
        fs::read(json).unwrap(),
        br#"{"key_key.back":"key.keyboard.x"}"#
    );
    assert_eq!(keyd.checkpoint().unwrap(), checkpoint);
    assert!(!f.state().join("mutation.json").exists());
}

#[test]
fn standard_settings_restore_two_generations_in_lifo_order() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let before = br#"{"key_key.back":"key.keyboard.s"}"#;
    let json = standard_file(&options, "config/mcsr/standardsettings.json", before);
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let first = c.apply(&profile("k"), &keyd).unwrap();
    let after_first = fs::read(&json).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&after_first).unwrap();
    assert_eq!(value["key_key.back"], "key.keyboard.k");
    let second = c.apply(&profile("j"), &keyd).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
    assert_eq!(value["key_key.back"], "key.keyboard.j");
    assert!(c
        .restore(first.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
    c.restore(second.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(&json).unwrap(), after_first);
    c.restore(first.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(json).unwrap(), before);
    assert_eq!(fs::read(options).unwrap(), original());
}

#[test]
fn reconciliation_syncs_and_restores_new_instance_standard_settings() {
    let f = Fixture::new();
    f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    c.apply(&profile("k"), &keyd).unwrap();
    let two = f.instance("two", original());
    let before = br#"{"key_key.back":"key.keyboard.s"}"#;
    let json = standard_file(&two, "config/mcsr/standardsettings.json", before);
    let report = c.reconcile_new_instances(&profile("k"), &keyd).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
    assert_eq!(value["key_key.back"], "key.keyboard.k");
    c.restore(report.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(json).unwrap(), before);
    assert_eq!(fs::read(two).unwrap(), original());
}

#[test]
fn standard_file_rolls_back_with_options_when_publication_fails() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let before = br#"{"key_key.back":"key.keyboard.s"}"#;
    let json = standard_file(&options, "config/mcsr/standardsettings.json", before);
    let keyd = FakeKeyd::default();
    let seen = std::cell::Cell::new(false);
    let result = f
        .coordinator(vec![])
        .apply_with_publish(&profile("k"), &keyd, &|_| {
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
            seen.set(value["key_key.back"] == "key.keyboard.k");
            Err(mcsr_toolkit::apply::ApplyError("publication failed".into()))
        });
    assert!(result.is_err());
    assert!(
        seen.get(),
        "standard config must be staged before publication"
    );
    assert_eq!(fs::read(options).unwrap(), original());
    assert_eq!(fs::read(json).unwrap(), before);
}

#[test]
fn restore_accepts_options_only_manifest_and_leaves_later_config_alone() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let report = c.apply(&profile("k"), &keyd).unwrap();
    let id = report.backup_id.as_deref().unwrap();
    let manifest_path = f.state().join("backups").join(id).join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.as_object_mut().unwrap().retain(|key, _| {
        [
            "launcher_root",
            "files",
            "prior_state",
            "post_state",
            "prior_history",
            "profile",
            "keyd_after",
        ]
        .contains(&key.as_str())
    });
    fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let added = br#"{"key_key.back":"key.keyboard.z"}"#;
    let json = standard_file(&options, "config/mcsr/standardsettings.json", added);
    c.restore(id, &keyd).unwrap();
    assert_eq!(fs::read(options).unwrap(), original());
    assert_eq!(fs::read(json).unwrap(), added);
}

#[test]
fn restore_rejects_added_or_removed_standard_config_before_mutation() {
    for added in [false, true] {
        let f = Fixture::new();
        let options = f.instance("one", original());
        let config = options
            .parent()
            .unwrap()
            .join("config/mcsr/standardsettings.json");
        if !added {
            standard_file(
                &options,
                "config/mcsr/standardsettings.json",
                br#"{"key_key.back":"key.keyboard.s"}"#,
            );
        }
        let keyd = FakeKeyd::default();
        let c = f.coordinator(vec![]);
        let report = c.apply(&profile("k"), &keyd).unwrap();
        if added {
            standard_file(
                &options,
                "config/mcsr/standardsettings.json",
                br#"{"key_key.back":"key.keyboard.z"}"#,
            );
        } else {
            fs::remove_file(&config).unwrap();
        }
        let applied_options = fs::read(&options).unwrap();
        let checkpoint = keyd.checkpoint().unwrap();
        assert!(
            c.restore(report.backup_id.as_deref().unwrap(), &keyd)
                .is_err(),
            "added={added}"
        );
        assert_eq!(fs::read(&options).unwrap(), applied_options);
        assert_eq!(keyd.checkpoint().unwrap(), checkpoint);
        assert_eq!(config.exists(), added);
        assert!(!f.state().join("mutation.json").exists());
    }
}

#[test]
fn restore_preserves_config_edited_during_keyd_restore() {
    struct EditingRestore<'a> {
        keyd: &'a FakeKeyd,
        config: PathBuf,
    }
    impl KeydAdapter for EditingRestore<'_> {
        fn install(&self, profile: &Profile) -> Result<(), String> {
            self.keyd.install(profile)
        }
        fn checkpoint(&self) -> Result<Vec<String>, String> {
            self.keyd.checkpoint()
        }
        fn restore_to(&self, checkpoint: &[String]) -> Result<(), String> {
            fs::write(&self.config, br#"{"key_key.back":"key.keyboard.x"}"#)
                .map_err(|error| error.to_string())?;
            self.keyd.restore_to(checkpoint)
        }
        fn rollback(&self) -> Result<(), String> {
            self.keyd.rollback()
        }
    }
    let f = Fixture::new();
    let options = f.instance("one", original());
    let config = standard_file(
        &options,
        "config/mcsr/standardsettings.json",
        br#"{"key_key.back":"key.keyboard.s"}"#,
    );
    let keyd = FakeKeyd::default();
    let coordinator = f.coordinator(vec![]);
    let report = coordinator.apply(&profile("k"), &keyd).unwrap();
    let applied_options = fs::read(&options).unwrap();
    let editing = EditingRestore {
        keyd: &keyd,
        config: config.clone(),
    };
    assert!(coordinator
        .restore(report.backup_id.as_deref().unwrap(), &editing)
        .is_err());
    assert_eq!(fs::read(options).unwrap(), applied_options);
    assert_eq!(
        fs::read(config).unwrap(),
        br#"{"key_key.back":"key.keyboard.x"}"#
    );
    assert!(f.state().join("mutation.json").exists());
}

#[test]
fn restoring_repair_backup_preserves_mod_edits_between_applies() {
    let f = Fixture::new();
    let options = f.instance("one", original());
    let config = standard_file(
        &options,
        "config/mcsr/standardsettings.json",
        br#"{"key_key.back":"key.keyboard.s"}"#,
    );
    let keyd = FakeKeyd::default();
    let coordinator = f.coordinator(vec![]);
    let first = coordinator.apply(&profile("k"), &keyd).unwrap();
    let external = br#"{ "key_key.back": "key.keyboard.x", "gamma": 0.7 }"#;
    fs::write(&config, external).unwrap();
    let repair = coordinator
        .reconcile_new_instances(&profile("k"), &keyd)
        .unwrap();
    assert_eq!(status(&repair, "one"), "updated");
    let repaired: serde_json::Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(repaired["key_key.back"], "key.keyboard.k");
    coordinator
        .restore(repair.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(&config).unwrap(), external);
    assert_eq!(keyd.installs.load(Ordering::Relaxed), 1);
    assert!(!f.state().join("mutation.json").exists());
    // The older snapshot still cannot overwrite the independently edited contents.
    assert!(coordinator
        .restore(first.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
    assert_eq!(fs::read(&config).unwrap(), external);
}

fn resize_profile() -> Profile {
    parse_profile(
        br#"{"presets":[{"id":"thin","width":300,"height":900,"shortcut":"b"}]}"#,
        &KeySet::fallback(),
    )
    .unwrap()
}

#[test]
fn resize_only_windowed_changes_preview_apply_restore_and_reconcile() {
    let f = Fixture::new();
    let options = f.instance("one", b"fullscreen:true\r\n");
    let json_before = br#"{ "fullscreen":{"enabled":true,"value":true}, "unrelated":42 }"#;
    let json = standard_file(&options, "config/mcsr/standardsettings.json", json_before);
    let legacy = standard_file(&options, "config/standardoptions.txt", b"fullscreen:true\r");
    let c = f.coordinator(vec![]);
    let keyd = FakeKeyd::default();
    let p = resize_profile();
    let preview = c.preview(&p).unwrap();
    assert_eq!(preview.instances[0].status, "ready");
    assert!(preview.instances[0].detail.contains("windowed"));
    assert!(preview.mappings.iter().all(|m| m.action != "fullscreen"));
    let report = c.apply(&p, &keyd).unwrap();
    assert_eq!(fs::read(&options).unwrap(), b"fullscreen:false\r\n");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&json).unwrap()).unwrap();
    assert_eq!(value["fullscreen"]["value"], false);
    assert_eq!(fs::read(&legacy).unwrap(), b"fullscreen:false\r");
    assert!(c
        .reconcile_new_instances(&p, &keyd)
        .unwrap()
        .backup_id
        .is_none());
    c.restore(report.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(fs::read(options).unwrap(), b"fullscreen:true\r\n");
    assert_eq!(fs::read(json).unwrap(), json_before);
    assert_eq!(fs::read(legacy).unwrap(), b"fullscreen:true\r");
}

#[test]
fn windowed_changes_defer_running_instances_and_refuse_invalid_peer_atomically() {
    let f = Fixture::new();
    let one = f.instance("one", b"fullscreen:true\n");
    let two = f.instance("two", b"fullscreen:true\n");
    let p = resize_profile();
    let keyd = FakeKeyd::default();
    let report = f
        .coordinator(vec![one.parent().unwrap().to_path_buf()])
        .apply(&p, &keyd)
        .unwrap();
    assert_eq!(status(&report, "one"), "deferred");
    assert_eq!(fs::read(&two).unwrap(), b"fullscreen:true\n");
    fs::write(&two, b"fullscreen:bad\n").unwrap();
    let report = f.coordinator(vec![]).apply(&p, &keyd).unwrap();
    assert_eq!(status(&report, "one"), "blocked");
    assert_eq!(status(&report, "two"), "failed");
    assert_eq!(fs::read(one).unwrap(), b"fullscreen:true\n");
}

#[test]
fn no_presets_preserves_fullscreen_and_removing_presets_does_not_enable_it() {
    let f = Fixture::new();
    let options = f.instance("one", b"fullscreen:true\n");
    let c = f.coordinator(vec![]);
    let keyd = FakeKeyd::default();
    c.apply(&Profile::default(), &keyd).unwrap();
    assert_eq!(fs::read(&options).unwrap(), b"fullscreen:true\n");
    c.apply(&resize_profile(), &keyd).unwrap();
    c.apply(&Profile::default(), &keyd).unwrap();
    assert_eq!(fs::read(options).unwrap(), b"fullscreen:false\n");
}

#[test]
fn historical_resize_backups_restore_fullscreen_true_without_replaying_new_policy() {
    let f = Fixture::new();
    let options = f.instance("one", b"fullscreen:true\nkey_key.back:key.keyboard.s\n");
    let config = standard_file(
        &options,
        "config/mcsr/standardsettings.json",
        br#"{"fullscreen":true}"#,
    );
    let keyd = FakeKeyd::default();
    let c = f.coordinator(vec![]);
    let first = c.apply(&profile("k"), &keyd).unwrap();
    let second = c.apply(&profile("j"), &keyd).unwrap();
    // These generations predate windowed management: add their resize metadata
    // while retaining every exact content hash and control-state snapshot.
    let presets = serde_json::to_value(resize_profile().presets).unwrap();
    let first_path = f
        .state()
        .join("backups")
        .join(first.backup_id.as_deref().unwrap())
        .join("manifest.json");
    let second_path = f
        .state()
        .join("backups")
        .join(second.backup_id.as_deref().unwrap())
        .join("manifest.json");
    let lineage_path = f.state().join("lineage.json");
    for (path, prior) in [
        (&first_path, false),
        (&second_path, true),
        (&lineage_path, false),
    ] {
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        value["profile"]["presets"] = presets.clone();
        if prior {
            value["prior_history"]["profile"]["presets"] = presets.clone();
        }
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }
    let restored = c
        .restore(second.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert!(!restored.restored_profile.unwrap().presets.is_empty());
    assert_eq!(
        fs::read(&options).unwrap(),
        b"fullscreen:true\nkey_key.back:key.keyboard.k\n"
    );
    assert_eq!(fs::read(&config).unwrap(), br#"{"fullscreen":true}"#);
    c.restore(first.backup_id.as_deref().unwrap(), &keyd)
        .unwrap();
    assert_eq!(
        fs::read(options).unwrap(),
        b"fullscreen:true\nkey_key.back:key.keyboard.s\n"
    );
}

#[test]
fn windowed_publication_failure_rolls_back_exactly() {
    let f = Fixture::new();
    let options = f.instance("one", b"fullscreen:true\n");
    let before = br#"{ "fullscreen":true }"#;
    let config = standard_file(&options, "config/mcsr/standardsettings.json", before);
    let c = f.coordinator(vec![]);
    let keyd = FakeKeyd::default();
    assert!(c
        .apply_with_publish(&resize_profile(), &keyd, &|_| Err(
            mcsr_toolkit::apply::ApplyError("publication failed".into())
        ))
        .is_err());
    assert_eq!(fs::read(&options).unwrap(), b"fullscreen:true\n");
    assert_eq!(fs::read(&config).unwrap(), before);
    assert!(f.state().join("mutation.json").exists());
}

#[test]
fn windowed_restore_refuses_mod_drift() {
    let f = Fixture::new();
    let options = f.instance("one", b"fullscreen:true\n");
    let config = standard_file(
        &options,
        "config/mcsr/standardsettings.json",
        br#"{"fullscreen":true}"#,
    );
    let c = f.coordinator(vec![]);
    let keyd = FakeKeyd::default();
    let report = c.apply(&resize_profile(), &keyd).unwrap();
    fs::write(&config, br#"{"fullscreen":true}"#).unwrap();
    assert!(c
        .restore(report.backup_id.as_deref().unwrap(), &keyd)
        .is_err());
    assert_eq!(fs::read(options).unwrap(), b"fullscreen:false\n");
}

#[test]
fn fullscreen_mod_only_change_is_visible_and_deferred_when_running() {
    for running in [false, true] {
        let f = Fixture::new();
        let options = f.instance("one", b"fullscreen:false\n");
        let config = standard_file(&options, "config/standardoptions.txt", b"fullscreen:true\n");
        let cwds = if running {
            vec![options.parent().unwrap().to_path_buf()]
        } else {
            vec![]
        };
        let c = f.coordinator(cwds);
        let p = resize_profile();
        let preview = c.preview(&p).unwrap();
        assert_eq!(
            preview.instances[0].status,
            if running { "deferred" } else { "ready" }
        );
        assert!(preview.instances[0]
            .detail
            .contains("config/standardoptions.txt"));
        assert!(!preview.instances[0].detail.contains("options.txt,"));
        let keyd = FakeKeyd::default();
        let report = c.apply(&p, &keyd).unwrap();
        assert_eq!(fs::read(options).unwrap(), b"fullscreen:false\n");
        if running {
            assert!(report.backup_id.is_none());
            assert_eq!(fs::read(config).unwrap(), b"fullscreen:true\n");
        } else {
            assert_eq!(fs::read(&config).unwrap(), b"fullscreen:false\n");
            c.restore(report.backup_id.as_deref().unwrap(), &keyd)
                .unwrap();
            assert_eq!(fs::read(config).unwrap(), b"fullscreen:true\n");
        }
    }
}
