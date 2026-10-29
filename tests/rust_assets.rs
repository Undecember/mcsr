use std::io::{Cursor, Read};

use mcsr_toolkit::assets::{extension_archive, web_asset};
use zip::ZipArchive;

#[test]
fn root_serves_embedded_html() {
    let (bytes, content_type) = web_asset("/").expect("root asset");
    assert_eq!(content_type, "text/html; charset=utf-8");
    assert!(std::str::from_utf8(bytes).unwrap().contains("<html"));
}

#[test]
fn app_javascript_is_embedded() {
    let (bytes, content_type) = web_asset("/app.js").expect("app script");
    assert_eq!(content_type, "text/javascript; charset=utf-8");
    assert!(std::str::from_utf8(bytes).unwrap().contains("fetch("));
}

#[test]
fn unknown_web_path_is_not_an_asset() {
    assert!(web_asset("/does-not-exist").is_none());
}

#[test]
fn extension_archive_contains_sources_and_compiled_schema() {
    let mut archive = ZipArchive::new(Cursor::new(extension_archive())).expect("valid zip");
    for path in [
        "metadata.json",
        "extension.js",
        "schemas/org.mcsr.toolkit.gschema.xml",
        "schemas/gschemas.compiled",
    ] {
        let mut file = archive.by_name(path).expect("required extension entry");
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert!(!bytes.is_empty(), "empty entry: {path}");
        if path == "metadata.json" {
            let metadata: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(metadata["uuid"], "mcsr-toolkit@undecember.github.io");
            assert_eq!(metadata["url"], "https://github.com/Undecember/mcsr");
        }
    }
}

#[test]
fn executable_contains_web_and_extension_assets() {
    let bytes = std::fs::read(env!("CARGO_BIN_EXE_mcsr-toolkit")).expect("read executable");
    for marker in [b"<!doctype html>".as_slice(), b"schemas/gschemas.compiled"] {
        assert!(
            bytes.windows(marker.len()).any(|window| window == marker),
            "executable lacks embedded asset marker: {}",
            String::from_utf8_lossy(marker)
        );
    }
}

#[test]
fn embedded_managed_keyd_executes_capability_probe_and_checks_routing() {
    use mcsr_toolkit::managed_keyd;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let bytes = managed_keyd::binary();
    assert_eq!(&bytes[..4], b"\x7fELF");
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("mcsr-keyd");
    std::fs::write(&binary, bytes).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(&binary)
        .arg("mcsr-capabilities")
        .output()
        .unwrap();
    assert!(output.status.success());
    let capabilities = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        capabilities.split_whitespace().collect::<Vec<_>>(),
        managed_keyd::CAPABILITIES
    );
    let license = Command::new(&binary).arg("mcsr-license").output().unwrap();
    assert!(license.status.success());
    assert_eq!(
        String::from_utf8(license.stdout).unwrap(),
        include_str!("../vendor/keyd/LICENSE")
    );
    let config = dir.path().join("routing.conf");
    std::fs::write(
        &config,
        "[ids]\n*\n[main]\ntab = layerk(mcsr_debug, f3)\nb = matchmods(f13, b)\n[mcsr_debug]\nb = b\n",
    )
    .unwrap();
    let output = Command::new(&binary)
        .arg("check")
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(managed_keyd::DROPIN.contains(managed_keyd::BINARY_PATH));
    assert!(!managed_keyd::DROPIN.contains("/usr/bin/keyd"));
}
