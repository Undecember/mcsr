use indexmap::IndexMap;
use mcsr_toolkit::standard_settings::prepare_local;
use std::fs;
use tempfile::tempdir;

fn bindings() -> IndexMap<String, String> {
    [
        ("key_key.forward", "key.keyboard.w"),
        ("key_key.attack", "key.mouse.left"),
        ("key_key.jump", "key.keyboard.unknown"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}
fn put(root: &std::path::Path, path: &str, data: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, data).unwrap();
}
#[test]
fn updates_enabled_json_controls_and_preserves_disabled_and_metadata() {
    let dir = tempdir().unwrap();
    let original = r#"{"toggleStandardSettings":false,"key_key.forward":"key.keyboard.a","key_key.attack":{"enabled":true,"value":"key.mouse.right","extra":42},"key_key.jump":{"enabled":false,"value":"key.keyboard.space"},"unknown":{"x":1}}"#;
    put(dir.path(), "config/mcsr/standardsettings.json", original);
    let edits = prepare_local(dir.path(), &bindings()).unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].before, original.as_bytes());
    let result: serde_json::Value = serde_json::from_slice(&edits[0].after).unwrap();
    assert_eq!(result["key_key.forward"], "key.keyboard.w");
    assert_eq!(result["key_key.attack"]["value"], "key.mouse.left");
    assert_eq!(result["key_key.attack"]["extra"], 42);
    assert_eq!(result["key_key.jump"]["value"], "key.keyboard.space");
    assert_eq!(result["toggleStandardSettings"], false);
    assert_eq!(result["unknown"]["x"], 1);
    assert_eq!(fs::read(&edits[0].path).unwrap(), original.as_bytes());
}
#[test]
fn legacy_preserves_endings_blank_and_unrelated_values() {
    let dir = tempdir().unwrap();
    let original = "key_key.forward:key.keyboard.a\r\nkey_key.attack:key.mouse.right\rkey_key.jump:\nother:yes";
    put(dir.path(), "config/standardoptions.txt", original);
    let edits = prepare_local(dir.path(), &bindings()).unwrap();
    assert_eq!(edits[0].after, b"key_key.forward:key.keyboard.w\r\nkey_key.attack:key.mouse.left\rkey_key.jump:\nother:yes");
}
#[test]
fn missing_and_unchanged_configs_are_handled_without_writes() {
    let dir = tempdir().unwrap();
    assert!(prepare_local(dir.path(), &bindings()).unwrap().is_empty());
    let original = "{ \"key_key.forward\": \"key.keyboard.w\" }\n";
    put(dir.path(), "config/mcsr/standardsettings.json", original);
    let edits = prepare_local(dir.path(), &bindings()).unwrap();
    assert_eq!(edits[0].after, original.as_bytes());
}
#[test]
fn rejects_malformed_and_duplicate_controls() {
    for json in [
        r#"{"key_key.forward":4}"#,
        r#"{"key_key.forward":{"enabled":false}}"#,
        r#"{"key_key.forward":"a","key_key.forward":"b"}"#,
    ] {
        let dir = tempdir().unwrap();
        put(dir.path(), "config/mcsr/standardsettings.json", json);
        assert!(prepare_local(dir.path(), &bindings()).is_err(), "{json}");
    }
    let dir = tempdir().unwrap();
    put(
        dir.path(),
        "config/standardoptions.txt",
        "key_key.forward:a\nkey_key.forward:b\n",
    );
    assert!(prepare_local(dir.path(), &bindings()).is_err());
}
#[test]
fn rejects_shared_redirects_without_touching_targets() {
    let dir = tempdir().unwrap();
    let target = dir.path().join("shared.txt");
    fs::write(&target, "key_key.forward:old\n").unwrap();
    put(
        dir.path(),
        "config/standardoptions.txt",
        target.to_str().unwrap(),
    );
    assert!(prepare_local(dir.path(), &bindings()).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"key_key.forward:old\n");
    fs::remove_file(dir.path().join("config/standardoptions.txt")).unwrap();
    put(
        dir.path(),
        "config/mcsr/standardsettings.global",
        "shared.json",
    );
    assert!(prepare_local(dir.path(), &bindings()).is_err());
}
#[cfg(unix)]
#[test]
fn rejects_symlinks_and_nonregular_files() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    fs::create_dir(dir.path().join("real")).unwrap();
    symlink(dir.path().join("real"), dir.path().join("config")).unwrap();
    assert!(prepare_local(dir.path(), &bindings()).is_err());
    fs::remove_file(dir.path().join("config")).unwrap();
    fs::create_dir_all(dir.path().join("config/standardoptions.txt")).unwrap();
    assert!(prepare_local(dir.path(), &bindings()).is_err());
}

#[test]
fn returns_both_existing_configs_and_syncs_unbound_controls() {
    let dir = tempdir().unwrap();
    put(
        dir.path(),
        "config/mcsr/standardsettings.json",
        r#"{".dataVersion":1,"key_key.jump":"key.keyboard.space"}"#,
    );
    put(
        dir.path(),
        "config/standardoptions.txt",
        "key_key.jump:key.keyboard.space\n",
    );
    let edits = prepare_local(dir.path(), &bindings()).unwrap();
    assert_eq!(edits.len(), 2);
    let json: serde_json::Value = serde_json::from_slice(&edits[0].after).unwrap();
    assert_eq!(json["key_key.jump"], "key.keyboard.unknown");
    assert_eq!(json[".dataVersion"], 1);
    assert_eq!(edits[1].after, b"key_key.jump:key.keyboard.unknown\n");
}

#[test]
fn rejects_legacy_redirect_chains_without_opening_shared_files() {
    let dir = tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    fs::write(&first, second.to_str().unwrap()).unwrap();
    fs::write(&second, "key_key.forward:old\n").unwrap();
    put(
        dir.path(),
        "config/standardoptions.txt",
        first.to_str().unwrap(),
    );
    assert!(prepare_local(dir.path(), &bindings()).is_err());
    assert_eq!(fs::read(&second).unwrap(), b"key_key.forward:old\n");
}
