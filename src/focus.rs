use std::{
    io,
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
        },
    },
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::Deserialize;
use tokio::{io::unix::AsyncFd, sync::watch};

pub const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FocusEvent {
    #[serde(rename = "class")]
    pub window_class: String,
    pub title: String,
    pub modal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTransition {
    Gained,
    Lost,
    Unchanged,
}

#[derive(Debug, Default, Clone)]
pub struct FocusState {
    effective: bool,
    last_seen: Option<Instant>,
    generation: u64,
}

pub type SharedFocus = Arc<Mutex<FocusState>>;

impl FocusState {
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn effective(&self) -> bool {
        self.effective
    }

    pub fn effective_at(&self, now: Instant) -> bool {
        self.effective
            && self
                .last_seen
                .is_some_and(|seen| now.saturating_duration_since(seen) < HEARTBEAT_TIMEOUT)
    }

    pub fn connected_at(&self, now: Instant) -> bool {
        self.last_seen
            .is_some_and(|seen| now.saturating_duration_since(seen) < HEARTBEAT_TIMEOUT)
    }

    pub fn observe(&mut self, event: FocusEvent) -> FocusTransition {
        self.observe_at(event, Instant::now())
    }

    pub fn observe_at(&mut self, event: FocusEvent, now: Instant) -> FocusTransition {
        self.last_seen = Some(now);
        self.set_effective(!event.modal && is_minecraft_class(&event.window_class))
    }

    pub fn expire(&mut self, now: Instant) -> FocusTransition {
        if self
            .last_seen
            .is_some_and(|seen| now.saturating_duration_since(seen) >= HEARTBEAT_TIMEOUT)
        {
            self.disconnect()
        } else {
            FocusTransition::Unchanged
        }
    }

    pub fn disconnect(&mut self) -> FocusTransition {
        self.last_seen = None;
        self.set_effective(false)
    }

    fn set_effective(&mut self, effective: bool) -> FocusTransition {
        let previous = std::mem::replace(&mut self.effective, effective);
        if previous != effective {
            self.generation = self.generation.wrapping_add(1);
        }
        match (previous, effective) {
            (false, true) => FocusTransition::Gained,
            (true, false) => FocusTransition::Lost,
            _ => FocusTransition::Unchanged,
        }
    }
}

pub fn is_minecraft_class(class: &str) -> bool {
    class
        .get(..9)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("minecraft"))
}

pub fn parse_focus_event(raw: &[u8]) -> Result<FocusEvent, String> {
    if raw.len() > 4096 {
        return Err("invalid focus event size".into());
    }
    let event: FocusEvent =
        serde_json::from_slice(raw).map_err(|e| format!("invalid focus event: {e}"))?;
    if event.window_class.chars().count() > 512 || event.title.chars().count() > 1024 {
        return Err("invalid focus event values".into());
    }
    Ok(event)
}

pub struct FocusListener {
    fd: AsyncFd<std::fs::File>,
    state: SharedFocus,
}

impl FocusListener {
    pub fn open(path: &Path, state: SharedFocus) -> io::Result<Self> {
        state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .disconnect();
        let name = std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in FIFO path"))?;
        let result = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
        if result < 0 && io::Error::last_os_error().kind() != io::ErrorKind::AlreadyExists {
            return Err(io::Error::last_os_error());
        }
        let metadata = std::fs::symlink_metadata(path)?;
        validate_fifo(&metadata)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)?;
        let opened = file.metadata()?;
        validate_fifo(&opened)?;
        if opened.ino() != metadata.ino() || opened.dev() != metadata.dev() {
            return Err(io::Error::other("FIFO changed during open"));
        }
        let mut buffer = [0u8; 8192];
        loop {
            let read =
                unsafe { libc::read(file.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len()) };
            if read > 0 {
                continue;
            }
            if read == 0 || io::Error::last_os_error().kind() == io::ErrorKind::WouldBlock {
                break;
            }
            return Err(io::Error::last_os_error());
        }
        // File owns a stable descriptor for the entire AsyncFd lifetime.
        let fd = unsafe { AsyncFd::register(file) }.map_err(|error| error.into_parts().1)?;
        Ok(Self { fd, state })
    }

    pub async fn run(
        self,
        mut cancel: watch::Receiver<bool>,
        effective: watch::Sender<bool>,
    ) -> io::Result<()> {
        if *cancel.borrow() {
            effective.send_replace(false);
            return Ok(());
        }
        let mut buffer = Vec::new();
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let result = loop {
            tokio::select! {
                biased;
                changed = cancel.changed() => {
                    if changed.is_err() || *cancel.borrow() { break Ok(()); }
                }
                _ = interval.tick() => {
                    let transition = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).expire(Instant::now());
                    self.publish(transition, &effective);
                }
                readable = self.fd.readable() => {
                    let mut guard = match readable { Ok(guard) => guard, Err(error) => break Err(error) };
                    let mut chunk = [0u8; 8192];
                    match guard.try_io(|inner| {
                        let size = unsafe { libc::read(inner.get_ref().as_raw_fd(), chunk.as_mut_ptr().cast(), chunk.len()) };
                        if size < 0 { Err(io::Error::last_os_error()) } else { Ok(size as usize) }
                    }) {
                        Ok(Ok(size)) => {
                            buffer.extend_from_slice(&chunk[..size]);
                            while let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
                                let line: Vec<u8> = buffer.drain(..=end).collect();
                                let transition = match parse_focus_event(&line[..line.len() - 1]) {
                                    Ok(event) => self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).observe(event),
                                    Err(_) => self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).disconnect(),
                                };
                                self.publish(transition, &effective);
                            }
                            if buffer.len() > 8192 {
                                buffer.clear();
                                let transition = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).disconnect();
                                self.publish(transition, &effective);
                            }
                        }
                        Ok(Err(error)) => break Err(error),
                        Err(_) => {},
                    }
                }
            }
        };
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .disconnect();
        effective.send_replace(false);
        result
    }

    fn publish(&self, transition: FocusTransition, effective: &watch::Sender<bool>) {
        if transition != FocusTransition::Unchanged {
            effective.send_replace(
                self.state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .effective(),
            );
        }
    }
}

impl Drop for FocusListener {
    fn drop(&mut self) {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .disconnect();
        // GNOME retains an RDWR writer across service restarts. Keep its FIFO inode
        // reachable; the next listener validates it and drains queued stale events.
    }
}

fn validate_fifo(metadata: &std::fs::Metadata) -> io::Result<()> {
    if !metadata.file_type().is_fifo()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unsafe focus FIFO",
        ));
    }
    Ok(())
}
