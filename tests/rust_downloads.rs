use std::io::{self, Cursor, Read};
use std::path::Path;

use mcsr_toolkit::downloads::{
    download_release, resolve_release, HttpClient, HttpResponse, Product, ReleaseAsset,
};
use zip::write::FileOptions;

type ResponseFactory = dyn Fn(&str) -> Result<HttpResponse, String>;
struct MockClient(Box<ResponseFactory>);

impl HttpClient for MockClient {
    fn get(&self, url: &str) -> Result<HttpResponse, String> {
        (self.0)(url)
    }
}

fn response(status: u16, location: Option<&str>, body: impl Read + Send + 'static) -> HttpResponse {
    HttpResponse {
        status,
        location: location.map(str::to_owned),
        body: Box::new(body),
    }
}

fn jar() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "META-INF/MANIFEST.MF",
            FileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    std::io::Write::write_all(&mut writer, b"Manifest-Version: 1.0\n").unwrap();
    writer
        .start_file(
            "Main.class",
            FileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
    std::io::Write::write_all(&mut writer, b"class bytes").unwrap();
    writer.finish().unwrap().into_inner()
}

fn deflated_jar() -> Vec<u8> {
    const HEX: &str = concat!(
        "504b030414000000080046bd565d7adf5a201800000016000000140000004d4554412d494e462f4d414e49464553542e4d46",
        "f34dcccb4c4b2d2ed10d4b2d2acecccfb35230d433e00200504b030414000000080046bd565decdd3aad0d0000000b000000",
        "0a0000004d61696e2e636c6173734bce492c2e5648aa2c492d0600504b0102140314000000080046bd565d7adf5a20",
        "18000000160000001400000000000000000000008001000000004d4554412d494e462f4d414e49464553542e4d46504b0102",
        "140314000000080046bd565decdd3aad0d0000000b0000000a000000000000000000000080014a0000004d61696e2e636c61",
        "7373504b050600000000020002007a0000007f0000000000"
    );
    HEX.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn asset() -> ReleaseAsset {
    ReleaseAsset {
        product: Product::Launcher,
        version: "0.8.9-beta".into(),
        source_url: "https://mcsrlauncher.github.io/download".into(),
        asset_url:
            "https://github.com/MCSRLauncher/Launcher/releases/download/0.8.9-beta/MCSRLauncher.jar"
                .into(),
        filename: "MCSRLauncher.jar".into(),
    }
}

#[test]
fn resolves_both_official_repositories() {
    for (product, repo, name) in [
        (
            Product::Launcher,
            "MCSRLauncher/Launcher",
            "MCSRLauncher.jar",
        ),
        (
            Product::Ninjabrain,
            "Ninjabrain1/Ninjabrain-Bot",
            "Ninjabrain-Bot-1.5.2.jar",
        ),
    ] {
        let expected = format!("https://api.github.com/repos/{repo}/releases/latest");
        let payload = format!("{{\"tag_name\":\"1.5.2\",\"assets\":[{{\"name\":\"{name}\",\"browser_download_url\":\"https://github.com/{repo}/releases/download/1.5.2/{name}\"}}]}}");
        let client = MockClient(Box::new(move |url| {
            assert_eq!(url, expected);
            Ok(response(
                200,
                None,
                Cursor::new(payload.clone().into_bytes()),
            ))
        }));
        let resolved = resolve_release(product, &client).unwrap();
        assert_eq!(resolved.filename, name);
        assert_eq!(resolved.version, "1.5.2");
    }
}

#[test]
fn rejects_forged_release_urls() {
    for url in [
        "http://github.com/MCSRLauncher/Launcher/releases/download/1/MCSRLauncher.jar",
        "https://github.com.evil.example/MCSRLauncher/Launcher/releases/download/1/MCSRLauncher.jar",
        "https://github.com/Ninjabrain1/Ninjabrain-Bot/releases/download/1/MCSRLauncher.jar",
        "https://github.com/MCSRLauncher/Launcher/releases/download/1/other.jar",
        "https://github.com/MCSRLauncher/Launcher/releases/download/1/MCSRLauncher.jar?x=1",
    ] {
        let payload = format!("{{\"tag_name\":\"1\",\"assets\":[{{\"name\":\"MCSRLauncher.jar\",\"browser_download_url\":\"{url}\"}}]}}");
        let client = MockClient(Box::new(move |_| Ok(response(200, None, Cursor::new(payload.clone().into_bytes())))));
        assert!(resolve_release(Product::Launcher, &client).is_err(), "accepted {url}");
    }
}

#[test]
fn valid_cdn_redirect_atomically_replaces_previous_jar() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("MCSRLauncher.jar");
    std::fs::write(&path, b"previous").unwrap();
    let payload = jar();
    let expected = payload.clone();
    let client = MockClient(Box::new(move |url| {
        if url.starts_with("https://github.com/") {
            Ok(response(
                302,
                Some("https://release-assets.githubusercontent.com/asset"),
                io::empty(),
            ))
        } else {
            assert_eq!(url, "https://release-assets.githubusercontent.com/asset");
            Ok(response(200, None, Cursor::new(payload.clone())))
        }
    }));
    let result = download_release(&asset(), &path, &client).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    assert_eq!(result.byte_count, expected.len() as u64);
    assert_eq!(result.path, path);
}

#[test]
fn normal_deflated_jar_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("MCSRLauncher.jar");
    let payload = deflated_jar();
    let client = MockClient(Box::new(move |_| {
        Ok(response(200, None, Cursor::new(payload.clone())))
    }));
    let result = download_release(&asset(), &path, &client).unwrap();
    assert_eq!(result.byte_count, deflated_jar().len() as u64);
    assert_eq!(std::fs::read(path).unwrap(), deflated_jar());
}

#[test]
fn recognized_but_unsupported_compression_returns_an_error() {
    let mut payload = deflated_jar();
    for index in 0..payload.len() - 10 {
        if payload[index..].starts_with(b"PK\x03\x04") {
            payload[index + 8..index + 10].copy_from_slice(&12_u16.to_le_bytes());
        } else if payload[index..].starts_with(b"PK\x01\x02") {
            payload[index + 10..index + 12].copy_from_slice(&12_u16.to_le_bytes());
        }
    }
    let client = MockClient(Box::new(move |_| {
        Ok(response(200, None, Cursor::new(payload.clone())))
    }));
    assert_preserved(&client);
}

fn assert_preserved(client: &dyn HttpClient) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("MCSRLauncher.jar");
    std::fs::write(&path, b"previous").unwrap();
    assert!(download_release(&asset(), &path, client).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"previous");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn redirects_to_unapproved_hosts_or_protocols_are_rejected() {
    for location in [
        "https://evil.example/file.jar",
        "http://release-assets.githubusercontent.com/file.jar",
        "https://github.com.evil.example/file",
    ] {
        let client = MockClient(Box::new(move |_| {
            Ok(response(302, Some(location), io::empty()))
        }));
        assert_preserved(&client);
    }
}

#[test]
fn malformed_jar_and_missing_manifest_preserve_previous_file() {
    assert_preserved(&MockClient(Box::new(|_| {
        Ok(response(
            200,
            None,
            Cursor::new(b"<html>oops</html>".to_vec()),
        ))
    })));
    let mut no_manifest = zip::ZipWriter::new(Cursor::new(Vec::new()));
    no_manifest
        .start_file("Main.class", FileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut no_manifest, b"class bytes").unwrap();
    let bytes = no_manifest.finish().unwrap().into_inner();
    assert_preserved(&MockClient(Box::new(move |_| {
        Ok(response(200, None, Cursor::new(bytes.clone())))
    })));
}

#[test]
fn empty_manifest_is_not_a_valid_jar() {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("META-INF/MANIFEST.MF", FileOptions::default())
        .unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    assert_preserved(&MockClient(Box::new(move |_| {
        Ok(response(200, None, Cursor::new(bytes.clone())))
    })));
}

#[test]
fn corrupt_entry_data_preserves_previous_file() {
    let mut bytes = jar();
    let position = bytes
        .windows(b"class bytes".len())
        .position(|window| window == b"class bytes")
        .unwrap();
    bytes[position] ^= 0xff;
    assert_preserved(&MockClient(Box::new(move |_| {
        Ok(response(200, None, Cursor::new(bytes.clone())))
    })));
}

#[test]
fn redirect_chain_is_bounded() {
    let client = MockClient(Box::new(|_| {
        Ok(response(
            302,
            Some("https://release-assets.githubusercontent.com/next"),
            io::empty(),
        ))
    }));
    assert_preserved(&client);
}

#[test]
fn release_metadata_has_a_size_limit() {
    let client = MockClient(Box::new(|_| {
        Ok(response(
            200,
            None,
            Cursor::new(vec![b' '; 1024 * 1024 + 1]),
        ))
    }));
    assert!(resolve_release(Product::Launcher, &client).is_err());
}

struct BrokenReader(bool);
impl Read for BrokenReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.0 {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "truncated"))
        } else {
            self.0 = true;
            out[..4].copy_from_slice(b"PK\x03\x04");
            Ok(4)
        }
    }
}

#[test]
fn interrupted_stream_preserves_previous_file() {
    assert_preserved(&MockClient(Box::new(|_| {
        Ok(response(200, None, BrokenReader(false)))
    })));
}

#[test]
fn destination_must_be_a_jar() {
    let client = MockClient(Box::new(|_| panic!("network must not be called")));
    assert!(download_release(&asset(), Path::new("/tmp/wrong.zip"), &client).is_err());
}
