//! Toolkit-owned, reproducibly built keyd dependency. Distribution files are untouched.

pub const BINARY_PATH: &str = "/usr/local/libexec/mcsr-keyd";
pub const DROPIN_PATH: &str = "/etc/systemd/system/keyd.service.d/90-mcsr-toolkit.conf";
pub const DROPIN: &str = "# MCSR Toolkit managed keyd engine\n[Service]\nExecStart=\nExecStart=/usr/local/libexec/mcsr-keyd\n";
pub const CAPABILITIES: &[&str] = &["mcsr-layerk-v1", "mcsr-matchmods-v1"];
pub const UPSTREAM_REVISION: &str = "f564288ac2b19d2305a5b39023c474805ff8fce5";

pub fn binary() -> &'static [u8] {
    include_bytes!(concat!(env!("OUT_DIR"), "/mcsr-keyd"))
}
