use std::{
    env,
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};
use zip::{write::FileOptions, CompressionMethod, DateTime, ZipArchive, ZipWriter};

const EXTENSION_FILES: &[(&str, &str)] = &[
    ("extension/extension.js", "extension.js"),
    ("extension/metadata.json", "metadata.json"),
    (
        "extension/schemas/org.mcsr.toolkit.gschema.xml",
        "schemas/org.mcsr.toolkit.gschema.xml",
    ),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    for path in [
        "static/index.html",
        "static/app.js",
        "static/style.css",
        "extension/extension.js",
        "extension/metadata.json",
        "extension/schemas/org.mcsr.toolkit.gschema.xml",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"));
    build_managed_keyd(&out_dir);
    let schema_out = out_dir.join("schemas");
    fs::create_dir_all(&schema_out).expect("create schema output directory");
    let output = Command::new("glib-compile-schemas")
        .arg("--strict")
        .arg(format!("--targetdir={}", schema_out.display()))
        .arg("extension/schemas")
        .output()
        .expect("glib-compile-schemas is required to build the extension archive");
    if !output.status.success() {
        panic!(
            "schema compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let mut entries: Vec<(&str, PathBuf)> = EXTENSION_FILES
        .iter()
        .map(|(source, name)| (*name, PathBuf::from(source)))
        .collect();
    entries.push((
        "schemas/gschemas.compiled",
        schema_out.join("gschemas.compiled"),
    ));
    entries.sort_by_key(|(name, _)| *name);

    let archive_path = out_dir.join("extension.zip");
    write_archive(&archive_path, &entries);
    verify_archive(&archive_path);
}

fn build_managed_keyd(out_dir: &Path) {
    println!("cargo:rerun-if-changed=vendor/keyd");
    println!("cargo:rerun-if-changed=packaging/keyd/mcsr.patch");
    println!("cargo:rerun-if-env-changed=CC");
    assert_eq!(
        env::var("HOST").unwrap(),
        env::var("TARGET").unwrap(),
        "managed keyd currently requires a native Linux build"
    );
    assert_eq!(env::var("CARGO_CFG_TARGET_OS").unwrap(), "linux");
    let stage = out_dir.join("keyd-source");
    if stage.exists() {
        fs::remove_dir_all(&stage).expect("remove previous keyd build staging");
    }
    copy_tree(Path::new("vendor/keyd/src"), &stage.join("src"));
    fs::create_dir_all(stage.join("t")).expect("create keyd test staging");
    fs::copy("vendor/keyd/t/test-io.c", stage.join("t/test-io.c"))
        .expect("copy keyd event test harness");
    let patch = fs::canonicalize("packaging/keyd/mcsr.patch").expect("keyd patch exists");
    let output = Command::new("patch")
        .args(["--batch", "--forward", "-p1", "-i"])
        .arg(patch)
        .current_dir(&stage)
        .output()
        .expect("patch is required to build managed keyd");
    assert!(
        output.status.success(),
        "managed keyd patch failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let mut sources: Vec<_> = fs::read_dir(stage.join("src"))
        .expect("read keyd sources")
        .map(|entry| entry.expect("keyd source entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "c"))
        .map(|path| path.strip_prefix(&stage).unwrap().to_path_buf())
        .collect();
    sources.sort();
    sources.push(PathBuf::from("src/vkbd/uinput.c"));
    let output = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()))
        .args([
            "-O2",
            "-std=c11",
            "-D_DEFAULT_SOURCE",
            "-D_FORTIFY_SOURCE=2",
            "-DVERSION=\"v2.6.0 (mcsr-f564288-layerk-matchmods-v1)\"",
            "-DCONFIG_DIR=\"/etc/keyd\"",
            "-DSOCKET_PATH=\"/var/run/keyd.socket\"",
            "-DDATA_DIR=\"/usr/local/share/mcsr-toolkit/keyd\"",
        ])
        .args(sources)
        .args(["-pthread", "-o"])
        .arg(out_dir.join("mcsr-keyd"))
        .current_dir(&stage)
        .output()
        .expect("a C compiler is required to build managed keyd");
    assert!(
        output.status.success(),
        "managed keyd compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let engine = out_dir.join("mcsr-keyd-test-io");
    let mut command = Command::new(env::var_os("CC").unwrap_or_else(|| "cc".into()));
    command
        .arg("-DDATA_DIR=\"\"")
        .arg(stage.join("t/test-io.c"));
    for name in [
        "keyboard", "string", "macro", "config", "log", "ini", "keys", "unicode",
    ] {
        command.arg(stage.join(format!("src/{name}.c")));
    }
    let output = command
        .arg("-o")
        .arg(&engine)
        .output()
        .expect("compile keyd event test harness");
    assert!(
        output.status.success(),
        "keyd event harness compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!("cargo:rustc-env=MCSR_KEYD_TEST_ENGINE={}", engine.display());
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create keyd staging directory");
    for entry in fs::read_dir(source).expect("read vendored keyd directory") {
        let entry = entry.expect("vendored keyd entry");
        let kind = entry.file_type().expect("keyd source file type");
        assert!(
            !kind.is_symlink(),
            "vendored keyd must not contain symlinks"
        );
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy keyd source");
        }
    }
}

fn write_archive(path: &Path, entries: &[(&str, PathBuf)]) {
    let file = File::create(path).expect("create extension archive");
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::default())
        .unix_permissions(0o644);
    for (name, source) in entries {
        let bytes =
            fs::read(source).unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        assert!(!bytes.is_empty(), "extension entry {name} is empty");
        zip.start_file(*name, options)
            .unwrap_or_else(|error| panic!("add {name} to extension archive: {error}"));
        zip.write_all(&bytes)
            .unwrap_or_else(|error| panic!("write {name} to extension archive: {error}"));
    }
    zip.finish().expect("finish extension archive");
}

fn verify_archive(path: &Path) {
    let bytes = fs::read(path).expect("read extension archive");
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("extension archive is valid ZIP");
    for name in [
        "extension.js",
        "metadata.json",
        "schemas/org.mcsr.toolkit.gschema.xml",
        "schemas/gschemas.compiled",
    ] {
        let mut entry = zip
            .by_name(name)
            .unwrap_or_else(|error| panic!("required extension entry {name} missing: {error}"));
        let mut first_byte = [0];
        assert_eq!(
            entry.read(&mut first_byte).unwrap(),
            1,
            "empty entry: {name}"
        );
    }
}
