use mcsr_toolkit::instances::{discover_instances, locate_launcher_root};
use mcsr_toolkit::mapping::{ControlPlan, ManagedState};
use mcsr_toolkit::options::{parse_options, render_options};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct TempRoot(PathBuf);
impl TempRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "mcsr-rust-instances-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn instance(root: &Path, name: &str, game: &str, options: bool) {
    let dir = root.join("instances").join(name);
    fs::create_dir_all(dir.join(game)).unwrap();
    fs::write(dir.join("instance.json"), format!(r#"{{"id":"{name}"}}"#)).unwrap();
    if options {
        fs::write(
            dir.join(game).join("options.txt"),
            b"key_key.back:key.keyboard.s\n",
        )
        .unwrap();
    }
}

fn plan(updates: &[(&str, &str)]) -> ControlPlan {
    ControlPlan {
        updates: updates
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        preview: vec![],
        conflicts: vec![],
        next_state: ManagedState::default(),
        keyd_bindings: vec![],
    }
}

#[test]
fn options_preserve_unrelated_lines_and_crlf() {
    let input = b"version:2586\r\nkey_key.back:key.keyboard.s\r\ngamma:1.0\r\n";
    let doc = parse_options(input).unwrap();
    assert_eq!(doc.bindings["key_key.back"], "key.keyboard.s");
    assert_eq!(
        render_options(&doc, &plan(&[("key_key.back", "key.keyboard.k")])),
        b"version:2586\r\nkey_key.back:key.keyboard.k\r\ngamma:1.0\r\n"
    );
}

#[test]
fn options_preserve_unrelated_lines_with_bare_cr_endings() {
    let doc = parse_options(b"key_key.back:a\rgamma:1").unwrap();
    assert_eq!(doc.bindings["key_key.back"], "a");
    assert_eq!(
        render_options(&doc, &plan(&[("key_key.back", "b")])),
        b"key_key.back:b\rgamma:1"
    );
    let terminated = parse_options(b"key_key.back:a\rgamma:1\r").unwrap();
    assert_eq!(
        render_options(&terminated, &plan(&[("key_key.chat", "t")])),
        b"key_key.back:a\rgamma:1\rkey_key.chat:t\r"
    );
}

#[test]
fn representative_116_options_change_only_requested_binding() {
    let input = include_bytes!("fixtures/options-1.16.txt");
    let doc = parse_options(input).unwrap();
    let output = render_options(&doc, &plan(&[("key_key.back", "key.keyboard.k")]));
    let expected = String::from_utf8(input.to_vec())
        .unwrap()
        .replace("key_key.back:key.keyboard.s", "key_key.back:key.keyboard.k");
    assert_eq!(output, expected.as_bytes());
    assert_eq!(doc.bindings["key_key.forward"], "key.keyboard.w");
}

#[test]
fn options_preserve_missing_final_newline_and_append_using_existing_ending() {
    let doc = parse_options(b"version:2586\r\nkey_key.back:key.keyboard.s").unwrap();
    assert_eq!(
        render_options(&doc, &plan(&[("key_key.back", "key.keyboard.k")])),
        b"version:2586\r\nkey_key.back:key.keyboard.k"
    );
    assert_eq!(
        render_options(&doc, &plan(&[("key_key.chat", "key.keyboard.t")])),
        b"version:2586\r\nkey_key.back:key.keyboard.s\r\nkey_key.chat:key.keyboard.t\r\n"
    );
}

#[test]
fn options_accept_legacy_116_alias_values_and_reject_duplicate_controls() {
    let doc = parse_options(b"key_key.chat:key.keyboard.t\nkey_key.command:key.keyboard.slash\nkey_key.back:key.keyboard.left.shift\n").unwrap();
    assert_eq!(doc.bindings["key_key.back"], "key.keyboard.left.shift");
    assert_eq!(doc.bindings["key_key.command"], "key.keyboard.slash");
    assert!(parse_options(b"key_key.back:a\nkey_key.back:b\n")
        .unwrap_err()
        .to_string()
        .contains("key_key.back"));
}

#[test]
fn invalid_utf8_is_rejected() {
    assert!(parse_options(b"key_key.back:\xff\n").is_err());
}

#[test]
fn discovery_finds_every_valid_instance_and_legacy_game_dir() {
    let tmp = TempRoot::new();
    let root = tmp.0.join("launcher");
    instance(&root, "ranked", "minecraft", true);
    instance(&root, "practice", ".minecraft", true);
    instance(&root, "new", "minecraft", false);
    let found = discover_instances(&root).unwrap();
    assert_eq!(
        found.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        vec!["new", "practice", "ranked"]
    );
    assert_eq!(found[0].problem.as_deref(), Some("options.txt is missing"));
    assert_eq!(
        found[1].options_path,
        root.join("instances/practice/.minecraft/options.txt")
    );
    assert_eq!(found[2].problem, None);
}

#[test]
fn discovery_excludes_missing_malformed_and_mismatched_instance_json() {
    let tmp = TempRoot::new();
    let root = tmp.0.join("launcher");
    instance(&root, "good", "minecraft", true);
    let missing = root.join("instances/missing");
    fs::create_dir(&missing).unwrap();
    let malformed = root.join("instances/malformed");
    fs::create_dir(&malformed).unwrap();
    fs::write(malformed.join("instance.json"), b"{").unwrap();
    let mismatch = root.join("instances/mismatch");
    fs::create_dir(&mismatch).unwrap();
    fs::write(mismatch.join("instance.json"), br#"{"id":"other"}"#).unwrap();
    let found = discover_instances(&root).unwrap();
    assert_eq!(
        found.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        vec!["good"]
    );
}

#[test]
fn launcher_location_uses_managed_and_process_roots_with_override_precedence() {
    let tmp = TempRoot::new();
    let managed = tmp.0.join("managed");
    let chosen = tmp.0.join("chosen/launcher");
    let process = tmp.0.join("process/launcher");
    for root in [&managed.join("launcher"), &chosen, &process] {
        fs::create_dir_all(root.join("instances")).unwrap();
    }
    assert_eq!(
        locate_launcher_root(&managed, None, &[]),
        Some(managed.join("launcher").canonicalize().unwrap())
    );
    assert_eq!(
        locate_launcher_root(&managed, Some(&chosen), &[]),
        Some(chosen.canonicalize().unwrap())
    );
    assert_eq!(
        locate_launcher_root(&tmp.0.join("absent"), None, &[tmp.0.join("process")]),
        Some(process.canonicalize().unwrap())
    );
}

#[cfg(unix)]
#[test]
fn discovery_rejects_symlink_escape_and_unsafe_launcher_root() {
    use std::os::unix::fs::symlink;
    let tmp = TempRoot::new();
    let root = tmp.0.join("launcher");
    let external = tmp.0.join("external");
    instance(&external, "escape", "minecraft", true);
    fs::create_dir_all(root.join("instances")).unwrap();
    symlink(
        external.join("instances/escape"),
        root.join("instances/escape"),
    )
    .unwrap();
    assert!(discover_instances(&root).unwrap().is_empty());
    let linked = tmp.0.join("linked");
    symlink(&root, &linked).unwrap();
    assert!(discover_instances(&linked).is_err());
    assert_eq!(locate_launcher_root(&tmp.0, Some(&linked), &[]), None);
}

#[cfg(unix)]
#[test]
fn discovery_reports_dangling_game_symlink_as_unsafe() {
    use std::os::unix::fs::symlink;
    let tmp = TempRoot::new();
    let root = tmp.0.join("launcher");
    let dir = root.join("instances/ranked");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("instance.json"), br#"{"id":"ranked"}"#).unwrap();
    symlink(tmp.0.join("missing-game"), dir.join("minecraft")).unwrap();
    fs::create_dir(dir.join(".minecraft")).unwrap();
    fs::write(
        dir.join(".minecraft/options.txt"),
        b"key_key.back:key.keyboard.s\n",
    )
    .unwrap();
    let found = discover_instances(&root).unwrap();
    assert_eq!(found[0].problem.as_deref(), Some("unsafe game path"));
}
