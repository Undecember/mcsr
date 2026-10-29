use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use mcsr_toolkit::{
    focus::{FocusEvent, FocusState, SharedFocus},
    gnome::{GnomeBridge, GnomeError, GnomePeer, PresetController},
    maccel::{MaccelPeer, SensitivityController, SensitivityError},
    profile::{Profile, ResizePreset},
};
use tokio::sync::{Mutex as AsyncMutex, Notify};

const EYE_REPLY: &str = "0.02083000005222856998443603515625";

#[derive(Default)]
struct FakeHelper {
    state: AsyncMutex<HelperState>,
    eye_started: Notify,
    eye_release: Notify,
    restore_started: Notify,
    restore_release: Notify,
}
#[derive(Default)]
struct HelperState {
    raw: i64,
    baseline: Option<i64>,
    generation: u64,
    available: bool,
    fail_eye: bool,
    fail_eye_after_write: bool,
    invalid_eye_reply: bool,
    fail_restore: bool,
    hold_eye: bool,
    hold_restore: bool,
    calls: Vec<&'static str>,
}
impl FakeHelper {
    async fn ready(raw: i64) -> Arc<Self> {
        let helper = Arc::new(Self::default());
        let mut state = helper.state.lock().await;
        state.raw = raw;
        state.available = true;
        drop(state);
        helper
    }
}
impl MaccelPeer for FakeHelper {
    async fn preflight(&self) -> Result<(), SensitivityError> {
        if self.state.lock().await.available {
            Ok(())
        } else {
            Err(SensitivityError("helper unavailable".into()))
        }
    }
    async fn eye(&self, token: &str) -> Result<String, SensitivityError> {
        let hold = self.state.lock().await.hold_eye;
        if hold {
            self.eye_started.notify_one();
            self.eye_release.notified().await;
        }
        let mut state = self.state.lock().await;
        state.calls.push("eye");
        if state.fail_eye || token != format!("{:064x}", state.generation) {
            return Err(SensitivityError("eye rejected".into()));
        }
        if state.baseline.is_none() {
            state.baseline = Some(state.raw);
        }
        state.raw = 89_464_169;
        if state.fail_eye_after_write {
            return Err(SensitivityError("eye readback failed".into()));
        }
        Ok(if state.invalid_eye_reply {
            "unverified".into()
        } else {
            EYE_REPLY.into()
        })
    }
    async fn restore(&self) -> Result<String, SensitivityError> {
        let hold = self.state.lock().await.hold_restore;
        if hold {
            self.restore_started.notify_one();
            self.restore_release.notified().await;
        }
        let mut state = self.state.lock().await;
        state.calls.push("restore");
        if state.fail_restore {
            return Err(SensitivityError("restore failed".into()));
        }
        if let Some(raw) = state.baseline.take() {
            state.raw = raw;
        }
        state.generation += 1;
        Ok(format!("{:064x}", state.generation))
    }
}

#[derive(Default)]
struct FakeGnome {
    actual: AsyncMutex<Option<(u32, u32)>>,
    fail: AsyncMutex<bool>,
    toggle: AsyncMutex<Option<(u32, u32, bool)>>,
}
impl GnomePeer for FakeGnome {
    async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
        Ok(("Minecraft".into(), "Game".into(), 800, 600, false))
    }
    async fn configure_presets(&self, _: String) -> Result<(), GnomeError> {
        Ok(())
    }
    async fn resize_focused(&self, width: u32, height: u32) -> Result<(u32, u32), GnomeError> {
        if *self.fail.lock().await {
            return Err(GnomeError("resize failed".into()));
        }
        Ok(self.actual.lock().await.unwrap_or((width, height)))
    }
    async fn toggle_resize_focused(
        &self,
        width: u32,
        height: u32,
    ) -> Result<(u32, u32, bool), GnomeError> {
        if *self.fail.lock().await {
            return Err(GnomeError("resize failed".into()));
        }
        Ok(self.toggle.lock().await.unwrap_or((width, height, false)))
    }
}

fn profile() -> Profile {
    Profile {
        presets: vec![
            ResizePreset {
                id: "normal".into(),
                width: 800,
                height: 600,
                shortcut: "f6".into(),
                eye_zoom: false,
            },
            ResizePreset {
                id: "eye".into(),
                width: 400,
                height: 300,
                shortcut: "f7".into(),
                eye_zoom: true,
            },
        ],
        ..Profile::default()
    }
}
fn focused() -> SharedFocus {
    let focus = Arc::new(Mutex::new(FocusState::default()));
    focus.lock().unwrap().observe(FocusEvent {
        window_class: "Minecraft".into(),
        title: "Game".into(),
        modal: false,
    });
    focus
}

#[tokio::test]
async fn f7_changes_sensitivity_only_after_exact_resize() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    let gnome = Arc::new(FakeGnome::default());
    let focus = focused();
    let bridge = Arc::new(GnomeBridge::new(gnome.clone(), focus.clone()));
    let controller = PresetController::new(bridge, sensitivity.clone(), focus.clone());
    *gnome.actual.lock().await = Some((401, 300));
    let snapshot = focus.lock().unwrap().clone();
    assert!(controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .is_err());
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    *gnome.actual.lock().await = None;
    let result = controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap();
    assert_eq!(
        (result.width, result.height, result.sensitivity.as_deref()),
        (400, 300, Some("0.02083"))
    );
    assert_eq!(helper.state.lock().await.raw, 89_464_169);
}

#[tokio::test]
async fn eye_resize_failure_propagates_without_sensitivity_write() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    let gnome = Arc::new(FakeGnome::default());
    *gnome.fail.lock().await = true;
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(gnome, focus.clone())),
        sensitivity,
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    let error = controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("resize failed"));
    let state = helper.state.lock().await;
    assert_eq!(state.raw, 3_221_225_472);
    assert_eq!(state.calls.as_slice(), &["restore"]);
}

#[tokio::test]
async fn shifted_f7_preserves_existing_multiplier() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(
            Arc::new(FakeGnome::default()),
            focus.clone(),
        )),
        sensitivity,
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    let result = controller
        .apply(&profile(), "eye", true, &snapshot)
        .await
        .unwrap();
    assert_eq!(result.sensitivity, None);
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap();
    let calls_before = helper.state.lock().await.calls.len();
    controller
        .apply(&profile(), "eye", true, &snapshot)
        .await
        .unwrap();
    assert_eq!(helper.state.lock().await.raw, 89_464_169);
    assert_eq!(helper.state.lock().await.calls.len(), calls_before);
}

#[tokio::test]
async fn f6_restores_exact_baseline_even_when_resize_fails() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    let gnome = Arc::new(FakeGnome::default());
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(gnome.clone(), focus.clone())),
        sensitivity.clone(),
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap();
    *gnome.fail.lock().await = true;
    assert!(controller
        .apply(&profile(), "normal", false, &snapshot)
        .await
        .is_err());
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    assert!(!sensitivity.needs_restore());
}

#[tokio::test]
async fn invalid_normal_size_still_restores_exact_baseline() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(
            Arc::new(FakeGnome::default()),
            focus.clone(),
        )),
        sensitivity.clone(),
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap();
    let mut invalid = profile();
    invalid.presets[0].width = 99;
    assert!(controller
        .apply(&invalid, "normal", false, &snapshot)
        .await
        .is_err());
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    assert!(!sensitivity.needs_restore());
}

#[tokio::test]
async fn normal_preset_restores_eye_even_after_focus_is_lost() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(
            Arc::new(FakeGnome::default()),
            focus.clone(),
        )),
        sensitivity.clone(),
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap();
    focus.lock().unwrap().disconnect();
    assert!(controller
        .apply(&profile(), "normal", false, &snapshot)
        .await
        .is_err());
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    assert!(!sensitivity.needs_restore());
}

#[tokio::test]
async fn focus_loss_during_eye_restores_before_reporting_success() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    helper.state.lock().await.hold_eye = true;
    let focus = focused();
    let controller = Arc::new(PresetController::new(
        Arc::new(GnomeBridge::new(
            Arc::new(FakeGnome::default()),
            focus.clone(),
        )),
        sensitivity.clone(),
        focus.clone(),
    ));
    let started = helper.eye_started.notified();
    let task = {
        let controller = controller.clone();
        let focus = focus.clone();
        tokio::spawn(async move {
            let snapshot = { focus.lock().unwrap().clone() };
            controller.apply(&profile(), "eye", false, &snapshot).await
        })
    };
    tokio::time::timeout(Duration::from_secs(1), started)
        .await
        .unwrap();
    focus.lock().unwrap().disconnect();
    controller.invalidate();
    helper.eye_release.notify_one();
    assert!(task.await.unwrap().is_err());
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    assert!(!sensitivity.needs_restore());
}

#[tokio::test]
async fn focus_loss_during_failed_eye_attempts_recovery() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    {
        let mut state = helper.state.lock().await;
        state.hold_eye = true;
        state.fail_eye_after_write = true;
    }
    let focus = focused();
    let controller = Arc::new(PresetController::new(
        Arc::new(GnomeBridge::new(
            Arc::new(FakeGnome::default()),
            focus.clone(),
        )),
        sensitivity.clone(),
        focus.clone(),
    ));
    let started = helper.eye_started.notified();
    let task = {
        let controller = controller.clone();
        let snapshot = focus.lock().unwrap().clone();
        tokio::spawn(async move { controller.apply(&profile(), "eye", false, &snapshot).await })
    };
    tokio::time::timeout(Duration::from_secs(1), started)
        .await
        .unwrap();
    focus.lock().unwrap().disconnect();
    helper.eye_release.notify_one();
    assert!(task.await.unwrap().is_err());
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    assert!(!sensitivity.needs_restore());
}

#[tokio::test]
async fn failed_eye_after_write_restores_while_focus_remains_effective() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    helper.state.lock().await.fail_eye_after_write = true;
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(
            Arc::new(FakeGnome::default()),
            focus.clone(),
        )),
        sensitivity.clone(),
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    let error = controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("eye readback failed"));
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    assert!(!sensitivity.needs_restore());
}

#[tokio::test]
async fn failed_eye_and_failed_recovery_report_both_and_remain_pending() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    {
        let mut state = helper.state.lock().await;
        state.fail_eye_after_write = true;
        state.fail_restore = true;
    }
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(
            Arc::new(FakeGnome::default()),
            focus.clone(),
        )),
        sensitivity.clone(),
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    let error = controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("eye readback failed"));
    assert!(error.to_string().contains("restore failed"));
    assert!(sensitivity.needs_restore());
    assert_eq!(helper.state.lock().await.raw, 89_464_169);
}

#[tokio::test]
async fn failed_preflight_invalidates_generation_until_restore() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = SensitivityController::new(helper.clone());
    sensitivity.startup_restore().await.unwrap();
    helper.state.lock().await.available = false;
    assert!(sensitivity.eye().await.is_err());
    helper.state.lock().await.available = true;
    assert!(sensitivity.needs_restore());
    assert!(sensitivity.eye().await.is_err());
    assert_eq!(helper.state.lock().await.calls.as_slice(), &["restore"]);
    sensitivity.restore().await.unwrap();
    sensitivity.eye().await.unwrap();
}

#[tokio::test]
async fn unavailable_or_failed_helper_blocks_new_eye_writes_until_recovery() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = SensitivityController::new(helper.clone());
    assert!(sensitivity.eye().await.is_err());
    assert!(helper.state.lock().await.calls.is_empty());
    sensitivity.startup_restore().await.unwrap();
    helper.state.lock().await.available = false;
    assert!(sensitivity.eye().await.is_err());
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    helper.state.lock().await.available = true;
    sensitivity.restore().await.unwrap();
    helper.state.lock().await.fail_eye = true;
    assert!(sensitivity.eye().await.is_err());
    helper.state.lock().await.fail_eye = false;
    assert!(sensitivity.eye().await.is_err());
    assert!(sensitivity.needs_restore());
    sensitivity.restore().await.unwrap();
    sensitivity.eye().await.unwrap();
    assert_eq!(helper.state.lock().await.raw, 89_464_169);
}

#[tokio::test]
async fn malformed_eye_reply_keeps_recovery_pending() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = SensitivityController::new(helper.clone());
    sensitivity.startup_restore().await.unwrap();
    helper.state.lock().await.invalid_eye_reply = true;
    assert!(sensitivity.eye().await.is_err());
    assert!(sensitivity.needs_restore());
    assert!(sensitivity.eye().await.is_err());
    sensitivity.restore().await.unwrap();
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
}

#[tokio::test]
async fn cancelled_newer_restore_does_not_make_older_token_usable() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    helper.state.lock().await.hold_restore = true;
    let started = helper.restore_started.notified();
    let first = {
        let sensitivity = sensitivity.clone();
        tokio::spawn(async move { sensitivity.restore().await })
    };
    tokio::time::timeout(Duration::from_secs(1), started)
        .await
        .unwrap();
    let second = {
        let sensitivity = sensitivity.clone();
        tokio::spawn(async move { sensitivity.restore().await })
    };
    tokio::task::yield_now().await;
    second.abort();
    let _ = second.await;
    helper.restore_release.notify_one();
    assert!(first.await.unwrap().is_err());
    assert!(sensitivity.needs_restore());
    assert!(sensitivity.eye().await.is_err());
}

#[tokio::test]
async fn normal_preset_requires_toggle_support_instead_of_fixed_size_fallback() {
    struct OldPeer;
    impl GnomePeer for OldPeer {
        async fn get_focus(&self) -> Result<(String, String, u32, u32, bool), GnomeError> {
            Ok(("Minecraft".into(), "Game".into(), 800, 600, false))
        }
        async fn configure_presets(&self, _: String) -> Result<(), GnomeError> {
            Ok(())
        }
        async fn resize_focused(&self, w: u32, h: u32) -> Result<(u32, u32), GnomeError> {
            Ok((w, h))
        }
    }
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper));
    sensitivity.startup_restore().await.unwrap();
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(Arc::new(OldPeer), focus.clone())),
        sensitivity,
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    assert!(controller
        .apply(&profile(), "normal", false, &snapshot)
        .await
        .is_err());
}

#[tokio::test]
async fn normal_toggle_restores_baseline_for_full_and_preset_while_eye_stays_fixed() {
    let helper = FakeHelper::ready(3_221_225_472).await;
    let sensitivity = Arc::new(SensitivityController::new(helper.clone()));
    sensitivity.startup_restore().await.unwrap();
    let gnome = Arc::new(FakeGnome::default());
    let focus = focused();
    let controller = PresetController::new(
        Arc::new(GnomeBridge::new(gnome.clone(), focus.clone())),
        sensitivity,
        focus.clone(),
    );
    let snapshot = focus.lock().unwrap().clone();
    let eye = controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap();
    assert!(!eye.full_size);
    *gnome.toggle.lock().await = Some((1920, 1080, true));
    let full = controller
        .apply(&profile(), "normal", false, &snapshot)
        .await
        .unwrap();
    assert_eq!(
        (full.width, full.height, full.full_size),
        (1920, 1080, true)
    );
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    controller
        .apply(&profile(), "eye", false, &snapshot)
        .await
        .unwrap();
    *gnome.toggle.lock().await = Some((800, 600, false));
    let preset = controller
        .apply(&profile(), "normal", false, &snapshot)
        .await
        .unwrap();
    assert!(!preset.full_size);
    assert_eq!(helper.state.lock().await.raw, 3_221_225_472);
    *gnome.toggle.lock().await = Some((1, 1, true));
    assert!(
        !controller
            .apply(&profile(), "eye", true, &snapshot)
            .await
            .unwrap()
            .full_size
    );
}
