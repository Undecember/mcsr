//! Resolve official releases and replace downloaded JARs only after validation.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use url::Url;
use zip::{CompressionMethod, ZipArchive};

const MAX_BYTES: u64 = 256 * 1024 * 1024;
const MAX_METADATA_BYTES: u64 = 1024 * 1024;
const MAX_REDIRECTS: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Product {
    Launcher,
    Ninjabrain,
}

impl Product {
    fn repository(self) -> &'static str {
        match self {
            Self::Launcher => "MCSRLauncher/Launcher",
            Self::Ninjabrain => "Ninjabrain1/Ninjabrain-Bot",
        }
    }

    fn source_url(self) -> &'static str {
        match self {
            Self::Launcher => "https://mcsrlauncher.github.io/download",
            Self::Ninjabrain => "https://github.com/Ninjabrain1/Ninjabrain-Bot/releases",
        }
    }

    fn valid_filename(self, name: &str) -> bool {
        match self {
            Self::Launcher => name == "MCSRLauncher.jar",
            Self::Ninjabrain => {
                name.starts_with("Ninjabrain-Bot-")
                    && name.ends_with(".jar")
                    && name.len() > "Ninjabrain-Bot-.jar".len()
                    && name["Ninjabrain-Bot-".len()..name.len() - 4]
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseAsset {
    pub product: Product,
    pub version: String,
    pub source_url: String,
    pub asset_url: String,
    pub filename: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadResult {
    pub version: String,
    pub path: PathBuf,
    pub byte_count: u64,
}

#[derive(Debug)]
pub enum DownloadError {
    Lookup(String),
    InvalidMetadata,
    InvalidAssetUrl,
    InvalidDestination,
    InvalidRedirect,
    TooManyRedirects,
    Download(String),
    TooLarge,
    InvalidJar,
    Io(io::Error),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DownloadError {}

pub struct HttpResponse {
    pub status: u16,
    pub location: Option<String>,
    pub body: Box<dyn Read + Send>,
}

pub trait HttpClient {
    fn get(&self, url: &str) -> Result<HttpResponse, String>;
}

/// Production transport. Redirects remain disabled so every hop is validated here.
pub struct ReqwestClient(reqwest::blocking::Client);

impl ReqwestClient {
    pub fn new() -> Result<Self, DownloadError> {
        reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .user_agent("mcsr-toolkit")
            .build()
            .map(Self)
            .map_err(|error| DownloadError::Download(error.to_string()))
    }
}

impl HttpClient for ReqwestClient {
    fn get(&self, url: &str) -> Result<HttpResponse, String> {
        let response = self.0.get(url).send().map_err(|error| error.to_string())?;
        Ok(HttpResponse {
            status: response.status().as_u16(),
            location: response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            body: Box::new(response),
        })
    }
}

fn secure_url(url: &Url, hosts: &[&str]) -> bool {
    url.scheme() == "https"
        && url.host_str().is_some_and(|host| hosts.contains(&host))
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none_or(|port| port == 443)
        && url.fragment().is_none()
}

fn valid_asset_url(product: Product, filename: &str, address: &str) -> bool {
    if !product.valid_filename(filename) {
        return false;
    }
    let Ok(url) = Url::parse(address) else {
        return false;
    };
    if !secure_url(&url, &["github.com"]) || url.query().is_some() {
        return false;
    }
    let expected_prefix = format!("/{}/releases/download/", product.repository());
    let path = url.path();
    let Some(suffix) = path.strip_prefix(&expected_prefix) else {
        return false;
    };
    let Some((tag, name)) = suffix.split_once('/') else {
        return false;
    };
    !tag.is_empty()
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        && name == filename
}

pub fn resolve_release(
    product: Product,
    client: &dyn HttpClient,
) -> Result<ReleaseAsset, DownloadError> {
    let api_url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        product.repository()
    );
    let response = client.get(&api_url).map_err(DownloadError::Lookup)?;
    if response.status != 200 {
        return Err(DownloadError::Lookup(format!("HTTP {}", response.status)));
    }
    let mut bytes = Vec::new();
    response
        .body
        .take(MAX_METADATA_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| DownloadError::Lookup(error.to_string()))?;
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err(DownloadError::TooLarge);
    }
    let metadata: Value =
        serde_json::from_slice(&bytes).map_err(|_| DownloadError::InvalidMetadata)?;
    let version = metadata
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or(DownloadError::InvalidMetadata)?;
    let assets = metadata
        .get("assets")
        .and_then(Value::as_array)
        .ok_or(DownloadError::InvalidMetadata)?;
    for asset in assets {
        let Some(filename) = asset.get("name").and_then(Value::as_str) else {
            continue;
        };
        if !product.valid_filename(filename) {
            continue;
        }
        let address = asset
            .get("browser_download_url")
            .and_then(Value::as_str)
            .ok_or(DownloadError::InvalidAssetUrl)?;
        if !valid_asset_url(product, filename, address) {
            return Err(DownloadError::InvalidAssetUrl);
        }
        return Ok(ReleaseAsset {
            product,
            version: version.to_owned(),
            source_url: product.source_url().to_owned(),
            asset_url: address.to_owned(),
            filename: filename.to_owned(),
        });
    }
    Err(DownloadError::InvalidMetadata)
}

fn valid_jar(path: &Path) -> Result<(), DownloadError> {
    let file = File::open(path).map_err(DownloadError::Io)?;
    let mut archive = ZipArchive::new(file).map_err(|_| DownloadError::InvalidJar)?;
    let mut total = 0_u64;
    let mut manifest_found = false;
    for index in 0..archive.len() {
        // Inspect the raw entry first: zip 0.6 may panic when reading a recognized
        // compression method whose decoder was excluded at build time.
        let method = archive
            .by_index_raw(index)
            .map_err(|_| DownloadError::InvalidJar)?
            .compression();
        if !matches!(
            method,
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(DownloadError::InvalidJar);
        }
        let mut entry = archive
            .by_index(index)
            .map_err(|_| DownloadError::InvalidJar)?;
        if entry.name() == "META-INF/MANIFEST.MF" {
            if entry.size() == 0 {
                return Err(DownloadError::InvalidJar);
            }
            manifest_found = true;
        }
        total = total
            .checked_add(entry.size())
            .ok_or(DownloadError::TooLarge)?;
        if total > MAX_BYTES {
            return Err(DownloadError::TooLarge);
        }
        let copied =
            io::copy(&mut entry, &mut io::sink()).map_err(|_| DownloadError::InvalidJar)?;
        if copied != entry.size() {
            return Err(DownloadError::InvalidJar);
        }
    }
    if !manifest_found {
        return Err(DownloadError::InvalidJar);
    }
    Ok(())
}

pub fn download_release(
    asset: &ReleaseAsset,
    destination: &Path,
    client: &dyn HttpClient,
) -> Result<DownloadResult, DownloadError> {
    if !valid_asset_url(asset.product, &asset.filename, &asset.asset_url) {
        return Err(DownloadError::InvalidAssetUrl);
    }
    if destination
        .extension()
        .and_then(|extension| extension.to_str())
        .is_none_or(|extension| !extension.eq_ignore_ascii_case("jar"))
    {
        return Err(DownloadError::InvalidDestination);
    }
    let parent = destination
        .parent()
        .ok_or(DownloadError::InvalidDestination)?;
    let mut current = Url::parse(&asset.asset_url).map_err(|_| DownloadError::InvalidAssetUrl)?;
    for redirects in 0..=MAX_REDIRECTS {
        let mut response = client
            .get(current.as_str())
            .map_err(DownloadError::Download)?;
        if matches!(response.status, 301 | 302 | 303 | 307 | 308) {
            if redirects == MAX_REDIRECTS {
                return Err(DownloadError::TooManyRedirects);
            }
            let location = response.location.ok_or(DownloadError::InvalidRedirect)?;
            let next = current
                .join(&location)
                .map_err(|_| DownloadError::InvalidRedirect)?;
            if !secure_url(
                &next,
                &["github.com", "release-assets.githubusercontent.com"],
            ) {
                return Err(DownloadError::InvalidRedirect);
            }
            current = next;
            continue;
        }
        if response.status != 200 {
            return Err(DownloadError::Download(format!("HTTP {}", response.status)));
        }
        std::fs::create_dir_all(parent).map_err(DownloadError::Io)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(DownloadError::Io)?;
        let mut count = 0_u64;
        let mut chunk = [0_u8; 64 * 1024];
        loop {
            let read = response
                .body
                .read(&mut chunk)
                .map_err(|error| DownloadError::Download(error.to_string()))?;
            if read == 0 {
                break;
            }
            count += read as u64;
            if count > MAX_BYTES {
                return Err(DownloadError::TooLarge);
            }
            temp.write_all(&chunk[..read]).map_err(DownloadError::Io)?;
        }
        temp.flush().map_err(DownloadError::Io)?;
        temp.as_file().sync_all().map_err(DownloadError::Io)?;
        valid_jar(temp.path())?;
        temp.persist(destination)
            .map_err(|error| DownloadError::Io(error.error))?;
        return Ok(DownloadResult {
            version: asset.version.clone(),
            path: destination.to_owned(),
            byte_count: count,
        });
    }
    Err(DownloadError::TooManyRedirects)
}
