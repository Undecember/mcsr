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
