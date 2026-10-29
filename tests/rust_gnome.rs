use std::{
    sync::{Arc, Mutex as StdMutex},
    time::{Duration, Instant},
};

use mcsr_toolkit::focus::{FocusEvent, FocusListener, FocusState};
use mcsr_toolkit::gnome::{GnomeBridge, GnomeError, GnomePeer, ResizeError};
use mcsr_toolkit::profile::{Profile, ResizePreset};
async fn wire<P: GnomePeer>(bridge: &GnomeBridge<P>, id: &str) -> String {
    bridge.preset_signal_id(id).await.unwrap()
}

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

#[derive(Default)]
struct FakePeer {
    focus: Mutex<(String, String, u32, u32, bool)>,
    config: Mutex<String>,
    fail_configuration: AtomicBool,
    applied: Mutex<(u32, u32)>,
}

impl GnomePeer for FakePeer {
    async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
        Ok(self.focus.lock().await.clone())
    }
    async fn configure_presets(&self, json: String) -> Result<(), GnomeError> {
        if self.fail_configuration.load(Ordering::SeqCst) {
            return Err(GnomeError("injected configuration failure".into()));
        }
        *self.config.lock().await = json;
        Ok(())
    }
    async fn resize_focused(&self, _width: u32, _height: u32) -> Result<(u32, u32), GnomeError> {
        Ok(*self.applied.lock().await)
    }
}

#[tokio::test]
async fn non_minecraft_resize_is_refused() {
    let peer = Arc::new(FakePeer::default());
    let bridge = GnomeBridge::new(peer.clone(), Arc::new(StdMutex::new(FocusState::default())));
    assert!(matches!(
        bridge.resize_focused(800, 600).await,
        Err(ResizeError::NotFocused)
    ));
    assert_eq!(*peer.applied.lock().await, (0, 0));
}

#[tokio::test]
async fn exact_size_mismatch_is_failure() {
    let peer = Arc::new(FakePeer::default());
    *peer.focus.lock().await = ("Minecraft".into(), "Game".into(), 854, 517, false);
    *peer.applied.lock().await = (801, 600);
    let bridge = GnomeBridge::new(peer, Arc::new(StdMutex::new(FocusState::default())));
    assert!(matches!(
        bridge.resize_focused(800, 600).await,
        Err(ResizeError::Inexact {
            requested: (800, 600),
            actual: (801, 600)
        })
    ));
}

#[tokio::test]
async fn profile_configuration_uses_extension_json_shape() {
    let peer = Arc::new(FakePeer::default());
    let bridge = GnomeBridge::new(peer.clone(), Arc::new(StdMutex::new(FocusState::default())));
    let mut profile = Profile::default();
    profile.presets.push(ResizePreset {
        id: "normal".into(),
        width: 800,
        height: 600,
        shortcut: "leftcontrol+f6".into(),
        eye_zoom: false,
    });
    bridge.configure_presets(&profile).await.unwrap();
    let value: serde_json::Value = serde_json::from_str(&peer.config.lock().await).unwrap();
    assert_eq!(value["protocol"], "mcsr-keyd-v1");
    assert_eq!(
        value["profile_generation"],
        mcsr_toolkit::routing::RoutingPlan::compile(&profile)
            .unwrap()
            .generation
    );
    assert_eq!(value["presets"][0]["accelerator"], "<Control>F13");
}

#[tokio::test]
async fn stale_preset_signal_is_rejected_after_reconfiguration() {
    let peer = Arc::new(FakePeer::default());
    *peer.focus.lock().await = ("Minecraft".into(), "Game".into(), 854, 517, false);
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    focus.lock().unwrap().observe(FocusEvent {
        window_class: "Minecraft".into(),
        title: "Game".into(),
        modal: false,
    });
    let bridge = GnomeBridge::new(peer, focus);
    let mut profile = Profile::default();
    profile.presets.push(ResizePreset {
        id: "old".into(),
        width: 800,
        height: 600,
        shortcut: "f6".into(),
        eye_zoom: false,
    });
    bridge.configure_presets(&profile).await.unwrap();
    let old_wire = wire(&bridge, "old").await;
    profile.presets[0].id = "new".into();
    bridge.configure_presets(&profile).await.unwrap();
    assert!(!bridge.accept_preset_signal(&old_wire, false).await.unwrap());
    assert!(bridge
        .accept_preset_signal(&wire(&bridge, "new").await, false)
        .await
        .unwrap());
}

#[tokio::test]
async fn duplicate_and_shifted_eye_accelerators_are_rejected_before_bus_write() {
    let peer = Arc::new(FakePeer::default());
    let bridge = GnomeBridge::new(peer.clone(), Arc::new(StdMutex::new(FocusState::default())));
    let profile = Profile {
        presets: vec![
            ResizePreset {
                id: "eye".into(),
                width: 800,
                height: 600,
                shortcut: "f7".into(),
                eye_zoom: true,
            },
            ResizePreset {
                id: "other".into(),
                width: 900,
                height: 700,
                shortcut: "leftshift+f7".into(),
                eye_zoom: false,
            },
        ],
        ..Default::default()
    };
    assert!(bridge.configure_presets(&profile).await.is_err());
    assert!(peer.config.lock().await.is_empty());
}

#[tokio::test]
async fn preset_signal_requires_live_fifo_heartbeat_even_when_bus_reports_minecraft() {
    let peer = Arc::new(FakePeer::default());
    *peer.focus.lock().await = ("Minecraft".into(), "Game".into(), 854, 517, false);
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    let bridge = GnomeBridge::new(peer, focus.clone());
    let mut profile = Profile::default();
    profile.presets.push(ResizePreset {
        id: "normal".into(),
        width: 800,
        height: 600,
        shortcut: "f6".into(),
        eye_zoom: false,
    });
    bridge.configure_presets(&profile).await.unwrap();
    let event = || FocusEvent {
        window_class: "Minecraft".into(),
        title: "Game".into(),
        modal: false,
    };
    focus
        .lock()
        .unwrap()
        .observe_at(event(), Instant::now() - Duration::from_secs(3));
    assert!(!bridge
        .accept_preset_signal(&wire(&bridge, "normal").await, false)
        .await
        .unwrap());
    focus.lock().unwrap().observe(event());
    assert!(bridge
        .accept_preset_signal(&wire(&bridge, "normal").await, false)
        .await
        .unwrap());
    focus.lock().unwrap().disconnect();
    assert!(!bridge
        .accept_preset_signal(&wire(&bridge, "normal").await, false)
        .await
        .unwrap());
}

struct NonReplyingPeer;
impl GnomePeer for NonReplyingPeer {
    async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
        std::future::pending().await
    }
    async fn configure_presets(&self, _json: String) -> Result<(), GnomeError> {
        std::future::pending().await
    }
    async fn resize_focused(&self, _width: u32, _height: u32) -> Result<(u32, u32), GnomeError> {
        std::future::pending().await
    }
}

#[tokio::test(start_paused = true)]
async fn nonreplying_focus_probe_times_out() {
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    let bridge = GnomeBridge::new(Arc::new(NonReplyingPeer), focus);
    let result = bridge.resize_focused(800, 600).await;
    assert!(matches!(result, Err(ResizeError::Gnome(error)) if error.0.contains("timed out")));
}

#[tokio::test(start_paused = true)]
async fn nonreplying_configuration_times_out() {
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    let bridge = GnomeBridge::new(Arc::new(NonReplyingPeer), focus);
    let result = bridge.configure_presets(&Profile::default()).await;
    assert!(matches!(result, Err(error) if error.0.contains("timed out")));
}

#[tokio::test(start_paused = true)]
async fn bridge_stale_check_leaves_expiry_transition_for_runtime_watch() {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    use tokio::sync::watch;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("focus.fifo");
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    let listener = FocusListener::open(&path, focus.clone()).unwrap();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (effective_tx, mut effective_rx) = watch::channel(false);
    let task = tokio::spawn(listener.run(cancel_rx, effective_tx));
    let mut writer = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)
        .unwrap();
    writer
        .write_all(b"{\"class\":\"Minecraft\",\"title\":\"Game\",\"modal\":false}\n")
        .unwrap();
    effective_rx.changed().await.unwrap();
    assert!(*effective_rx.borrow());

    let peer = Arc::new(FakePeer::default());
    *peer.focus.lock().await = ("Minecraft".into(), "Game".into(), 854, 517, false);
    let bridge = GnomeBridge::new(peer, focus.clone());
    let mut profile = Profile::default();
    profile.presets.push(ResizePreset {
        id: "normal".into(),
        width: 800,
        height: 600,
        shortcut: "f6".into(),
        eye_zoom: false,
    });
    bridge.configure_presets(&profile).await.unwrap();
    focus.lock().unwrap().observe_at(
        FocusEvent {
            window_class: "Minecraft".into(),
            title: "Game".into(),
            modal: false,
        },
        Instant::now() - Duration::from_secs(3),
    );
    assert!(!bridge
        .accept_preset_signal(&wire(&bridge, "normal").await, false)
        .await
        .unwrap());
    assert!(
        *effective_rx.borrow(),
        "listener has not run its expiry tick yet"
    );
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::time::timeout(Duration::from_millis(50), effective_rx.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(!*effective_rx.borrow());
    cancel_tx.send(true).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn captured_special_keys_route_through_internal_gnome_accelerators() {
    let peer = Arc::new(FakePeer::default());
    let bridge = GnomeBridge::new(peer.clone(), Arc::new(StdMutex::new(FocusState::default())));
    for (key, _symbol) in [
        ("capslock", "Caps_Lock"),
        ("numlock", "Num_Lock"),
        ("scrolllock", "Scroll_Lock"),
        ("sysrq", "Print"),
        ("pause", "Pause"),
        ("menu", "Menu"),
        ("kp0", "KP_0"),
        ("kp1", "KP_1"),
        ("kp2", "KP_2"),
        ("kp3", "KP_3"),
        ("kp4", "KP_4"),
        ("kp5", "KP_5"),
        ("kp6", "KP_6"),
        ("kp7", "KP_7"),
        ("kp8", "KP_8"),
        ("kp9", "KP_9"),
        ("kpplus", "KP_Add"),
        ("kpminus", "KP_Subtract"),
        ("kpasterisk", "KP_Multiply"),
        ("kpslash", "KP_Divide"),
        ("kpdot", "KP_Decimal"),
        ("kpenter", "KP_Enter"),
        ("kpequal", "KP_Equal"),
    ] {
        let mut profile = Profile::default();
        profile.presets.push(ResizePreset {
            id: "normal".into(),
            width: 800,
            height: 600,
            shortcut: format!("rightcontrol+{key}"),
            eye_zoom: false,
        });
        bridge.configure_presets(&profile).await.unwrap();
        let value: serde_json::Value = serde_json::from_str(&peer.config.lock().await).unwrap();
        assert_eq!(value["presets"][0]["accelerator"], "<Control>F13", "{key}");
    }
}

#[tokio::test]
async fn reused_preset_id_rejects_previous_generation() {
    let peer = Arc::new(FakePeer::default());
    *peer.focus.lock().await = ("Minecraft".into(), "Game".into(), 854, 517, false);
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    focus.lock().unwrap().observe(FocusEvent {
        window_class: "Minecraft".into(),
        title: "Game".into(),
        modal: false,
    });
    let bridge = GnomeBridge::new(peer, focus);
    let mut profile = Profile::default();
    profile.presets.push(ResizePreset {
        id: "normal".into(),
        width: 800,
        height: 600,
        shortcut: "b".into(),
        eye_zoom: false,
    });
    bridge.configure_presets(&profile).await.unwrap();
    let old = wire(&bridge, "normal").await;
    assert!(bridge.accept_preset_signal(&old, false).await.unwrap());
    profile.presets[0].width = 900;
    bridge.configure_presets(&profile).await.unwrap();
    assert!(!bridge.accept_preset_signal(&old, false).await.unwrap());
    assert!(!bridge.accept_preset_signal("normal", false).await.unwrap());
    assert!(bridge
        .accept_preset_signal(&wire(&bridge, "normal").await, false)
        .await
        .unwrap());
}

#[tokio::test]
async fn failed_configuration_invalidates_previous_signal_generation() {
    let peer = Arc::new(FakePeer::default());
    *peer.focus.lock().await = ("Minecraft".into(), "Game".into(), 854, 517, false);
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    focus.lock().unwrap().observe(FocusEvent {
        window_class: "Minecraft".into(),
        title: "Game".into(),
        modal: false,
    });
    let bridge = GnomeBridge::new(peer.clone(), focus);
    let mut profile = Profile::default();
    profile.presets.push(ResizePreset {
        id: "normal".into(),
        width: 800,
        height: 600,
        shortcut: "b".into(),
        eye_zoom: false,
    });
    bridge.configure_presets(&profile).await.unwrap();
    let old = wire(&bridge, "normal").await;
    assert!(bridge.accept_preset_signal(&old, false).await.unwrap());
    peer.fail_configuration.store(true, Ordering::SeqCst);
    profile.presets[0].width = 900;
    assert!(bridge.configure_presets(&profile).await.is_err());
    assert!(!bridge.accept_preset_signal(&old, false).await.unwrap());
}

#[tokio::test]
async fn identical_profile_reconnect_rejects_old_activation_signal() {
    let peer = Arc::new(FakePeer::default());
    *peer.focus.lock().await = ("Minecraft".into(), "Game".into(), 854, 480, false);
    let focus = Arc::new(StdMutex::new(FocusState::default()));
    focus.lock().unwrap().observe(FocusEvent {
        window_class: "Minecraft".into(),
        title: "Game".into(),
        modal: false,
    });
    let bridge = GnomeBridge::new(peer, focus);
    let mut profile = Profile::default();
    profile.presets.push(ResizePreset {
        id: "normal".into(),
        width: 854,
        height: 480,
        shortcut: "b".into(),
        eye_zoom: false,
    });
    bridge.configure_presets(&profile).await.unwrap();
    let old = wire(&bridge, "normal").await;
    bridge.configure_presets(&profile).await.unwrap();
    assert!(!bridge.accept_preset_signal(&old, false).await.unwrap());
}

struct FencedPeer {
    events: Arc<StdMutex<Vec<&'static str>>>,
    attempts: std::sync::atomic::AtomicUsize,
}
impl GnomePeer for FencedPeer {
    async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
        Ok(("Minecraft".into(), "Game".into(), 854, 480, false))
    }
    async fn prepare_routing(&self, generation: String) -> Result<(), GnomeError> {
        self.events.lock().unwrap().push(if generation.is_empty() {
            "abort"
        } else {
            "prepare"
        });
        Ok(())
    }
    async fn configure_presets(&self, json: String) -> Result<(), GnomeError> {
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["drain"], true);
        self.events.lock().unwrap().push("configure");
        if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(GnomeError("routing input drain pending".into()))
        } else {
            Ok(())
        }
    }
    async fn resize_focused(&self, w: u32, h: u32) -> Result<(u32, u32), GnomeError> {
        Ok((w, h))
    }
}
#[tokio::test]
async fn routing_prepares_then_emits_ordered_marker_then_waits_for_consumer() {
    let events = Arc::new(StdMutex::new(vec![]));
    let bridge = GnomeBridge::new(
        Arc::new(FencedPeer {
            events: events.clone(),
            attempts: 0.into(),
        }),
        Arc::new(StdMutex::new(FocusState::default())),
    );
    bridge
        .configure_presets_with_fence(&Profile::default(), || {
            events.lock().unwrap().push("marker");
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        *events.lock().unwrap(),
        ["prepare", "marker", "configure", "configure"]
    );
    events.lock().unwrap().clear();
    assert!(bridge
        .configure_presets_with_fence(&Profile::default(), || {
            events.lock().unwrap().push("marker-failed");
            Err("keyd unavailable".into())
        })
        .await
        .is_err());
    assert_eq!(
        *events.lock().unwrap(),
        ["prepare", "marker-failed", "abort"]
    );
}
