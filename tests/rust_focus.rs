use std::time::{Duration, Instant};

use mcsr_toolkit::focus::{parse_focus_event, FocusEvent, FocusState, FocusTransition};

fn event(class: &str, modal: bool) -> FocusEvent {
    FocusEvent {
        window_class: class.into(),
        title: "Minecraft wiki".into(),
        modal,
    }
}

#[test]
fn class_not_title_controls_effective_focus() {
    let mut state = FocusState::default();
    assert_eq!(
        state.observe(event("org.mozilla.firefox", false)),
        FocusTransition::Unchanged
    );
    assert_eq!(
        state.observe(event("Minecraft* 1.16", false)),
        FocusTransition::Gained
    );
    assert!(state.effective());
    assert_eq!(
        state.observe(event("MINECRAFT", false)),
        FocusTransition::Unchanged
    );
}

#[test]
fn heartbeat_timeout_clears_focus() {
    let mut state = FocusState::default();
    let now = Instant::now();
    state.observe_at(event("minecraft", false), now);
    assert_eq!(
        state.expire(now + Duration::from_millis(1999)),
        FocusTransition::Unchanged
    );
    assert_eq!(
        state.expire(now + Duration::from_secs(2)),
        FocusTransition::Lost
    );
    assert!(!state.effective());
}

#[test]
fn modal_and_disconnect_clear_focus() {
    let mut state = FocusState::default();
    state.observe(event("minecraft", false));
    assert_eq!(
        state.observe(event("minecraft", true)),
        FocusTransition::Lost
    );
    assert_eq!(
        state.observe(event("minecraft", false)),
        FocusTransition::Gained
    );
    assert_eq!(state.disconnect(), FocusTransition::Lost);
    assert_eq!(state.disconnect(), FocusTransition::Unchanged);
}

#[test]
fn fifo_json_has_exact_bounded_shape() {
    assert_eq!(
        parse_focus_event(br#"{"class":"minecraft","title":"Game","modal":false}"#)
            .unwrap()
            .window_class,
        "minecraft"
    );
    assert!(parse_focus_event(br#"{"class":"minecraft","title":"Game","modal":0}"#).is_err());
    assert!(
        parse_focus_event(br#"{"class":"minecraft","title":"Game","modal":false,"extra":1}"#)
            .is_err()
    );
    assert!(parse_focus_event(&vec![b'x'; 4097]).is_err());
}

#[tokio::test]
async fn fifo_listener_emits_focus_and_stops_on_cancellation() {
    use mcsr_toolkit::focus::FocusListener;
    use std::os::unix::fs::OpenOptionsExt;
    use tokio::sync::watch;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("focus.fifo");
    let listener = FocusListener::open(
        &path,
        std::sync::Arc::new(std::sync::Mutex::new(FocusState::default())),
    )
    .unwrap();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (focus_tx, mut focus_rx) = watch::channel(false);
    let task = tokio::spawn(listener.run(cancel_rx, focus_tx));
    let mut writer = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
        .unwrap();
    use std::io::Write;
    writer
        .write_all(b"{\"class\":\"minecraft\",\"title\":\"Game\",\"modal\":false}\n")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), focus_rx.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(*focus_rx.borrow());
    cancel_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
    assert!(!*focus_rx.borrow());
}

#[tokio::test]
async fn fifo_restart_discards_old_messages_and_refuses_regular_files() {
    use mcsr_toolkit::focus::FocusListener;
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    use tokio::sync::watch;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("focus.fifo");
    std::fs::write(&path, b"untouched").unwrap();
    assert!(FocusListener::open(
        &path,
        std::sync::Arc::new(std::sync::Mutex::new(FocusState::default()))
    )
    .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"untouched");
    std::fs::remove_file(&path).unwrap();
    let name = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let mut writer = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
        .unwrap();
    writer
        .write_all(b"{\"class\":\"minecraft\",\"title\":\"Old\",\"modal\":false}\n")
        .unwrap();
    // The extension may retain its writer while the service starts again.
    let listener = FocusListener::open(
        &path,
        std::sync::Arc::new(std::sync::Mutex::new(FocusState::default())),
    )
    .unwrap();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (focus_tx, mut focus_rx) = watch::channel(false);
    let task = tokio::spawn(listener.run(cancel_rx, focus_tx));
    assert!(
        tokio::time::timeout(Duration::from_millis(50), focus_rx.changed())
            .await
            .is_err()
    );
    writer
        .write_all(b"{\"class\":\"minecraft\",\"title\":\"New\",\"modal\":false}\n")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), focus_rx.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(*focus_rx.borrow());
    cancel_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn graceful_restart_preserves_writer_and_discards_downtime_focus() {
    use mcsr_toolkit::focus::FocusListener;
    use std::{
        io::Write,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
        sync::{Arc, Mutex},
    };
    use tokio::sync::watch;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("focus.fifo");
    let state = Arc::new(Mutex::new(FocusState::default()));
    let listener = FocusListener::open(&path, state.clone()).unwrap();
    // GNOME keeps this RDWR descriptor open across service restarts.
    let mut writer = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
        .unwrap();
    let original = writer.metadata().unwrap();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (focus_tx, mut focus_rx) = watch::channel(false);
    let task = tokio::spawn(listener.run(cancel_rx, focus_tx));
    writer
        .write_all(b"{\"class\":\"minecraft\",\"title\":\"Before restart\",\"modal\":false}\n")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), focus_rx.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(*focus_rx.borrow_and_update());
    cancel_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
    assert!(!*focus_rx.borrow());
    assert!(!state.lock().unwrap().connected_at(Instant::now()));
    let retained = std::fs::symlink_metadata(&path).expect("restart must retain the FIFO pathname");
    assert_eq!(
        (retained.dev(), retained.ino()),
        (original.dev(), original.ino())
    );

    writer
        .write_all(b"{\"class\":\"minecraft\",\"title\":\"During downtime\",\"modal\":false}\n")
        .unwrap();
    // Startup must also clear an inherited focus state before draining the FIFO.
    state.lock().unwrap().observe(event("minecraft", false));
    let listener = FocusListener::open(&path, state.clone()).unwrap();
    assert!(!state.lock().unwrap().effective());
    assert!(!state.lock().unwrap().connected_at(Instant::now()));
    let reopened = std::fs::symlink_metadata(&path).unwrap();
    assert_eq!(
        (reopened.dev(), reopened.ino()),
        (original.dev(), original.ino())
    );
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (focus_tx, mut focus_rx) = watch::channel(false);
    let task = tokio::spawn(listener.run(cancel_rx, focus_tx));
    assert!(
        tokio::time::timeout(Duration::from_millis(50), focus_rx.changed())
            .await
            .is_err()
    );
    assert!(!state.lock().unwrap().connected_at(Instant::now()));
    writer
        .write_all(b"{\"class\":\"minecraft\",\"title\":\"Fresh\",\"modal\":false}\n")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), focus_rx.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(*focus_rx.borrow());
    cancel_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
}
