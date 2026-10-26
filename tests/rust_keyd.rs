use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use mcsr_toolkit::apply::KeydAdapter;
use mcsr_toolkit::focus::{FocusEvent, FocusState};
use mcsr_toolkit::keyd::{CommandFailure, CommandOutput, CommandRunner, KeydController};
use mcsr_toolkit::profile::{parse_profile, KeySet, Profile};

#[derive(Default)]
struct FakeRunner {
    calls: Mutex<Vec<(Vec<String>, Vec<u8>)>>,
    fail: Mutex<Option<String>>,
    timeout: Mutex<Option<String>>,
    next_id: Mutex<char>,
    fail_bind: Mutex<bool>,
    deny_helper: Mutex<bool>,
    deny_verify_at: Mutex<Option<usize>>,
    verify_count: Mutex<usize>,
    invalid_response: Mutex<bool>,
    keyd_fallback: Mutex<bool>,
    pause_on: Mutex<Option<(String, Arc<Pause>)>>,
    pause_verify: Mutex<Option<Arc<Pause>>>,
}

#[derive(Default)]
struct Pause {
    state: Mutex<(bool, bool)>,
    changed: Condvar,
}
impl Pause {
    fn wait_entered(&self) {
        let mut state = self.state.lock().unwrap();
        while !state.0 {
            let (next, timeout) = self
                .changed
                .wait_timeout(state, Duration::from_secs(10))
                .unwrap();
            assert!(!timeout.timed_out(), "fake command did not reach pause");
            state = next;
        }
    }
    fn release(&self) {
        self.state.lock().unwrap().1 = true;
        self.changed.notify_all();
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, args: &[String], input: &[u8]) -> Result<CommandOutput, CommandFailure> {
        self.calls
            .lock()
            .unwrap()
            .push((args.to_vec(), input.to_vec()));
        let pause_request = {
            let mut request = self.pause_on.lock().unwrap();
            if request.as_ref().is_some_and(|(action, _)| {
                args.last().is_some_and(|last| last == action)
                    && !args.iter().any(|arg| arg == "-l")
            }) {
                request.take()
            } else {
                None
            }
        };
        if let Some((action, pause)) = pause_request {
            debug_assert_eq!(args.last(), Some(&action));
            let mut state = pause.state.lock().unwrap();
            state.0 = true;
            pause.changed.notify_all();
            while !state.1 {
                state = pause.changed.wait(state).unwrap();
            }
        }
        if self.timeout.lock().unwrap().as_deref() == Some(args.last().unwrap().as_str()) {
            return Err(CommandFailure::TimedOut(
                "simulated privileged command timed out".into(),
            ));
        }
        if *self.fail_bind.lock().unwrap() && args.len() > 3 && args[0].contains("keyd") {
            return Err(CommandFailure::Failed("bind failed".into()));
        }
        if self.fail.lock().unwrap().as_deref() == Some(args.last().unwrap().as_str()) {
            return Err(CommandFailure::Failed(
                "command failed; stop failed; input safety unverified".into(),
            ));
        }
        let mut id = self.next_id.lock().unwrap();
        let response = if args.contains(&"install".to_string()) {
            let result = format!("{{\"backup_id\":\"{}\"}}", id.to_string().repeat(32));
            *id = char::from_u32(*id as u32 + 1).unwrap();
            result
        } else if args.contains(&"restore".to_string()) {
            format!("{{\"restored\":\"{}\"}}", args.last().unwrap())
        } else {
            String::new()
        };
        Ok(CommandOutput {
            stdout: if *self.invalid_response.lock().unwrap() {
                "invalid response".into()
            } else {
                response
            },
        })
    }
    fn verify_helper(&self) -> Result<(), String> {
        let mut count = self.verify_count.lock().unwrap();
        *count += 1;
        let denied_at = *self.deny_verify_at.lock().unwrap() == Some(*count);
        drop(count);
        if let Some(pause) = self.pause_verify.lock().unwrap().take() {
            let mut state = pause.state.lock().unwrap();
            state.0 = true;
            pause.changed.notify_all();
            while !state.1 {
                state = pause.changed.wait(state).unwrap();
            }
        }
        if denied_at || *self.deny_helper.lock().unwrap() {
            Err("unsafe helper path".into())
        } else {
            Ok(())
        }
    }
    fn keyd_binary(&self) -> Result<&'static str, String> {
        Ok(if *self.keyd_fallback.lock().unwrap() {
            "/usr/bin/keyd"
        } else {
            "/usr/bin/keyd.rvaiya"
        })
    }
}

#[test]
fn install_releases_own_marker_when_second_helper_preflight_fails() {
    let (dir, fake, controller) = setup();
    *fake.deny_verify_at.lock().unwrap() = Some(2);
    assert!(controller.install(&Profile::default()).is_err());
    assert!(!dir.path().join("keyd-uncertain.json").exists());
    assert!(!fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(args, _)| { args.first().is_some_and(|arg| arg == "/usr/bin/pkexec") }));
    assert!(controller.checkpoint().unwrap().is_empty());
}

#[test]
fn install_keeps_marker_when_helper_response_fails_after_dispatch() {
    let (dir, fake, controller) = setup();
    *fake.invalid_response.lock().unwrap() = true;
    assert!(controller.install(&Profile::default()).is_err());
    assert!(dir.path().join("keyd-uncertain.json").exists());
    assert!(fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(args, _)| { args.first().is_some_and(|arg| arg == "/usr/bin/pkexec") }));
}

#[test]
fn concurrent_controllers_cannot_share_clear_marker_or_activate_early() {
    let (dir, first_fake, first) = setup();
    let pause = Arc::new(Pause::default());
    *first_fake.pause_on.lock().unwrap() = Some(("clear".into(), pause.clone()));
    let first_work = std::thread::spawn(move || first.clear());
    pause.wait_entered();
    let marker = dir.path().join("keyd-uncertain.json");
    let owner_bytes = std::fs::read(&marker).unwrap();

    let second_fake = Arc::new(FakeRunner::default());
    let second = KeydController::new(dir.path(), second_fake.clone(), true);
    assert!(second.clear().is_err());
    assert_eq!(std::fs::read(&marker).unwrap(), owner_bytes);
    assert!(!second_fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(args, _)| { args.first().is_some_and(|arg| arg == "/usr/bin/sudo") }));

    let third_fake = Arc::new(FakeRunner::default());
    let third = KeydController::new(dir.path(), third_fake.clone(), true);
    assert!(third.activate(&Profile::default(), &focus(false)).is_err());
    assert!(third_fake.calls.lock().unwrap().is_empty());

    pause.release();
    first_work.join().unwrap().unwrap();
    assert!(!marker.exists());
    third.activate(&Profile::default(), &focus(false)).unwrap();
}

#[test]
fn in_flight_clear_blocks_empty_lineage_operations() {
    let (dir, first_fake, first) = setup();
    let pause = Arc::new(Pause::default());
    *first_fake.pause_on.lock().unwrap() = Some(("clear".into(), pause.clone()));
    let first_work = std::thread::spawn(move || first.clear());
    pause.wait_entered();

    let second_fake = Arc::new(FakeRunner::default());
    let second = KeydController::new(dir.path(), second_fake.clone(), true);
    let checkpoint = second.checkpoint();
    let no_op_restore = second.restore_to(&[]);
    let no_op_rollback = second.rollback();
    pause.release();
    first_work.join().unwrap().unwrap();

    assert!(checkpoint.is_err());
    assert!(no_op_restore.is_err());
    assert!(no_op_rollback.is_err());
    assert!(second_fake.calls.lock().unwrap().is_empty());
    assert!(second.checkpoint().unwrap().is_empty());
}

#[test]
fn concurrent_install_cannot_start_a_second_privileged_install() {
    let (dir, first_fake, first) = setup();
    let first_command = Arc::new(Pause::default());
    *first_fake.pause_on.lock().unwrap() = Some(("install".into(), first_command.clone()));
    let first_work = std::thread::spawn(move || first.install(&Profile::default()));
    first_command.wait_entered();

    let second_fake = Arc::new(FakeRunner::default());
    let second = KeydController::new(dir.path(), second_fake.clone(), true);
    let second_result = second.install(&Profile::default());
    let marker = dir.path().join("keyd-uncertain.json");
    let owner_bytes = std::fs::read(&marker).unwrap();

    let second_called_pkexec = second_fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(args, _)| args.first().is_some_and(|arg| arg == "/usr/bin/pkexec"));
    let marker_unchanged = std::fs::read(&marker).unwrap() == owner_bytes;

    first_command.release();
    first_work.join().unwrap().unwrap();
    assert!(second_result.is_err());
    assert!(!second_called_pkexec);
    assert!(marker_unchanged);
    assert!(!marker.exists());
    let restarted = KeydController::new(dir.path(), second_fake, true);
    assert_eq!(restarted.checkpoint().unwrap(), vec!["a".repeat(32)]);
}

#[test]
fn delayed_install_keeps_both_snapshot_ids_and_restores_latest() {
    let (dir, first_fake, first) = setup();
    let first_command = Arc::new(Pause::default());
    *first_fake.pause_on.lock().unwrap() = Some(("install".into(), first_command.clone()));
    let first_work = std::thread::spawn(move || first.install(&Profile::default()));
    first_command.wait_entered();

    let second_fake = Arc::new(FakeRunner::default());
    *second_fake.next_id.lock().unwrap() = 'b';
    let second_verified = Arc::new(Pause::default());
    *second_fake.pause_verify.lock().unwrap() = Some(second_verified.clone());
    let second = KeydController::new(dir.path(), second_fake.clone(), true);
    let second_work = std::thread::spawn(move || second.install(&Profile::default()));
    second_verified.wait_entered();

    first_command.release();
    first_work.join().unwrap().unwrap();
    second_verified.release();
    second_work.join().unwrap().unwrap();

    let first = KeydController::new(dir.path(), first_fake.clone(), true);
    let expected = vec!["a".repeat(32), "b".repeat(32)];
    assert_eq!(first.checkpoint().unwrap(), expected);
    first.restore_to(&expected[..1]).unwrap();
    assert_eq!(first.checkpoint().unwrap(), vec!["a".repeat(32)]);
    assert!(first_fake.calls.lock().unwrap().iter().any(|(args, _)| {
        args.contains(&"restore".to_string()) && args.last() == Some(&"b".repeat(32))
    }));
}

#[test]
fn concurrent_restore_blocks_other_controller_clear_and_activation() {
    let (dir, first_fake, first) = setup();
    first.install(&Profile::default()).unwrap();
    let pause = Arc::new(Pause::default());
    *first_fake.pause_on.lock().unwrap() = Some(("a".repeat(32), pause.clone()));
    let first_work = std::thread::spawn(move || first.restore_to(&[]));
    pause.wait_entered();
    let marker = dir.path().join("keyd-uncertain.json");
    let owner_bytes = std::fs::read(&marker).unwrap();

    let second_fake = Arc::new(FakeRunner::default());
    let second = KeydController::new(dir.path(), second_fake.clone(), true);
    let second_result = second.clear();
    let second_called_sudo = second_fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(args, _)| args.first().is_some_and(|arg| arg == "/usr/bin/sudo"));
    let third_fake = Arc::new(FakeRunner::default());
    let third = KeydController::new(dir.path(), third_fake.clone(), true);
    let third_result = third.activate(&Profile::default(), &focus(false));
    let third_called_command = !third_fake.calls.lock().unwrap().is_empty();
    let marker_unchanged = std::fs::read(&marker).unwrap() == owner_bytes;

    pause.release();
    first_work.join().unwrap().unwrap();
    assert!(second_result.is_err());
    assert!(!second_called_sudo);
    assert!(third_result.is_err());
    assert!(!third_called_command);
    assert!(marker_unchanged);
    assert!(!marker.exists());
    assert!(second.checkpoint().unwrap().is_empty());
}

#[test]
fn restart_during_in_flight_clear_remains_suspended() {
    let (dir, fake, controller) = setup();
    let pause = Arc::new(Pause::default());
    *fake.pause_on.lock().unwrap() = Some(("clear".into(), pause.clone()));
    let work = std::thread::spawn(move || controller.clear());
    pause.wait_entered();
    let restarted = KeydController::new(dir.path(), fake.clone(), true);
    assert!(!restarted.input_safety_verified());
    assert!(restarted
        .activate(&Profile::default(), &focus(false))
        .is_err());
    pause.release();
    work.join().unwrap().unwrap();
    assert!(!dir.path().join("keyd-uncertain.json").exists());
}

#[test]
fn restart_during_in_flight_install_remains_suspended() {
    let (dir, fake, controller) = setup();
    let pause = Arc::new(Pause::default());
    *fake.pause_on.lock().unwrap() = Some(("install".into(), pause.clone()));
    let work = std::thread::spawn(move || controller.install(&Profile::default()));
    pause.wait_entered();
    let restarted = KeydController::new(dir.path(), fake.clone(), true);
    assert!(!restarted.input_safety_verified());
    let checkpoint = restarted.checkpoint();
    assert!(restarted
        .activate(&Profile::default(), &focus(false))
        .is_err());
    pause.release();
    work.join().unwrap().unwrap();
    assert!(checkpoint.is_err(), "in-flight lineage was exposed");
    assert!(!dir.path().join("keyd-uncertain.json").exists());
}

#[test]
fn timed_out_privileged_clear_stays_uncertain_after_success_and_restart() {
    let (dir, fake, controller) = setup();
    *fake.timeout.lock().unwrap() = Some("clear".into());
    assert!(controller
        .clear()
        .unwrap_err()
        .to_string()
        .contains("timed out"));
    assert!(!controller.input_safety_verified());
    let marker = dir.path().join("keyd-uncertain.json");
    let owner_bytes = std::fs::read(&marker).unwrap();
    *fake.timeout.lock().unwrap() = None;
    assert!(controller.clear().is_err());
    assert_eq!(std::fs::read(&marker).unwrap(), owner_bytes);
    assert!(!controller.input_safety_verified());
    assert!(controller.preflight_clear().is_err());
    let restarted = KeydController::new(dir.path(), fake.clone(), true);
    assert!(!restarted.input_safety_verified());
    assert!(restarted
        .activate(&Profile::default(), &focus(false))
        .is_err());
    assert!(restarted.clear().is_err());
    assert!(!restarted.input_safety_verified());
    assert!(restarted.install(&Profile::default()).is_err());
    assert_eq!(std::fs::read(&marker).unwrap(), owner_bytes);
    assert_eq!(
        fake.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(args, _)| args.first().is_some_and(|arg| arg == "/usr/bin/sudo"))
            .count(),
        1
    );
}

#[test]
fn clear_refuses_privileged_call_without_private_durable_state() {
    let (dir, fake, _controller) = setup();
    let controller = KeydController::new(dir.path().join("missing"), fake.clone(), true);
    assert!(controller.clear().is_err());
    assert!(!controller.input_safety_verified());
    assert!(!fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(args, _)| args.first().is_some_and(|arg| arg == "/usr/bin/sudo")));
}

#[test]
fn timed_out_install_suspends_existing_active_bindings() {
    let (_dir, fake, controller) = setup();
    let state = focus(false);
    controller.activate(&Profile::default(), &state).unwrap();
    *fake.timeout.lock().unwrap() = Some("install".into());
    assert!(controller.install(&Profile::default()).is_err());
    *fake.timeout.lock().unwrap() = None;
    assert!(controller.sync_focus(&Profile::default(), &state).is_err());
    assert!(!controller.input_safety_verified());
    assert_eq!(
        fake.calls.lock().unwrap().last().unwrap().0.last().unwrap(),
        "reset"
    );
    assert!(!fake.calls.lock().unwrap().iter().any(|(args, _)| {
        args.first().is_some_and(|arg| arg == "/usr/bin/sudo")
            && args.last().is_some_and(|arg| arg == "clear")
            && !args.iter().any(|arg| arg == "-l")
    }));
}

#[test]
fn event_counts_reflect_attempted_clear_and_successful_activation() {
    let (_dir, _fake, controller) = setup();
    assert_eq!(controller.event_counts(), (0, 0));
    controller
        .activate(&Profile::default(), &focus(false))
        .unwrap();
    assert_eq!(controller.event_counts(), (1, 0));
    controller.clear().unwrap();
    assert_eq!(controller.event_counts(), (1, 1));
}

#[test]
fn failed_activation_attempts_cleanup_while_focus_remains_effective() {
    let (_dir, fake, controller) = setup();
    *fake.fail_bind.lock().unwrap() = true;
    let state = focus(false);
    assert!(controller.activate(&Profile::default(), &state).is_err());
    assert!(!controller.input_safety_verified());
    controller.sync_focus(&Profile::default(), &state).unwrap();
    assert!(controller.input_safety_verified());
    assert_eq!(
        fake.calls.lock().unwrap().last().unwrap().0.last().unwrap(),
        "clear"
    );
}

#[test]
fn clear_refuses_untrusted_helper_after_reset() {
    let (_dir, fake, controller) = setup();
    *fake.deny_helper.lock().unwrap() = true;
    assert!(controller
        .clear()
        .unwrap_err()
        .to_string()
        .contains("unsafe helper"));
    assert!(!controller.input_safety_verified());
    assert_eq!(fake.calls.lock().unwrap().len(), 1);
}

#[test]
fn discovered_key_survives_install_and_activation() {
    let (dir, fake, _) = setup();
    let keys = KeySet::with_discovered(["mouseback"]);
    let profile = parse_profile(br#"{"crafting":{"mouseback":"k"}}"#, &keys).unwrap();
    let controller = KeydController::with_keys(dir.path(), fake.clone(), true, keys);
    controller.install(&profile).unwrap();
    controller.activate(&profile, &focus(false)).unwrap();
    assert!(fake
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|(args, _)| args.iter().any(|arg| arg == "mouseback = k")));
}

#[test]
fn missing_rvaiya_client_uses_fixed_keyd_fallback() {
    let (_dir, fake, controller) = setup();
    *fake.keyd_fallback.lock().unwrap() = true;
    controller
        .activate(&Profile::default(), &focus(false))
        .unwrap();
    controller.clear().unwrap();
    let calls = fake.calls.lock().unwrap();
    assert_eq!(calls[1].0[0], "/usr/bin/keyd");
    assert_eq!(calls[2].0[0], "/usr/bin/keyd");
}

fn setup() -> (tempfile::TempDir, Arc<FakeRunner>, KeydController) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        dir.path(),
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    let fake = Arc::new(FakeRunner::default());
    *fake.next_id.lock().unwrap() = 'a';
    let controller = KeydController::new(dir.path(), fake.clone(), true);
    (dir, fake, controller)
}

fn focus(modal: bool) -> FocusState {
    let mut state = FocusState::default();
    state.observe(FocusEvent {
        window_class: "Minecraft".into(),
        title: "Game".into(),
        modal,
    });
    state
}

#[test]
fn install_sends_restricted_chat_fragment_and_persists_snapshot() {
    let (_dir, fake, controller) = setup();
    let mut profile = Profile::default();
    profile.crafting.insert("s".into(), "k".into());
    controller.install(&profile).unwrap();
    assert_eq!(controller.checkpoint().unwrap(), vec!["a".repeat(32)]);
    let calls = fake.calls.lock().unwrap();
    assert_eq!(
        calls[0].0,
        [
            "/usr/bin/pkexec",
            "/usr/local/libexec/mcsr-toolkit",
            "privileged",
            "keyd",
            "install"
        ]
    );
    let fragment = String::from_utf8(calls[0].1.clone()).unwrap();
    assert_eq!(
        fragment,
        "[mcsr_chat]\ns = s\nslash = slash\nt = t\nenter = clearm(enter)\nesc = clearm(esc)\n"
    );
}

#[test]
fn activation_requires_effective_focus_and_no_mutation() {
    let (dir, fake, controller) = setup();
    assert!(!controller.input_safety_verified());
    assert!(controller
        .activate(&Profile::default(), &FocusState::default())
        .is_err());
    assert!(controller
        .activate(&Profile::default(), &focus(true))
        .is_err());
    std::fs::write(dir.path().join("mutation.json"), b"{}").unwrap();
    assert!(controller
        .activate(&Profile::default(), &focus(false))
        .is_err());
    assert!(fake.calls.lock().unwrap().is_empty());
    std::fs::remove_file(dir.path().join("mutation.json")).unwrap();
    controller
        .activate(&Profile::default(), &focus(false))
        .unwrap();
    assert_eq!(fake.calls.lock().unwrap().len(), 2);
}

#[test]
fn clear_resets_before_helper_and_blocks_activation_after_failure() {
    let (_dir, fake, controller) = setup();
    controller
        .activate(&Profile::default(), &focus(false))
        .unwrap();
    *fake.fail.lock().unwrap() = Some("clear".into());
    let error = controller.clear().unwrap_err();
    assert!(error.to_string().contains("input safety unverified"));
    assert!(!controller.input_safety_verified());
    assert!(controller
        .activate(&Profile::default(), &focus(false))
        .is_err());
    let calls = fake.calls.lock().unwrap();
    assert_eq!(calls[2].0, ["/usr/bin/keyd.rvaiya", "bind", "reset"]);
    assert_eq!(
        calls[3].0,
        [
            "/usr/bin/sudo",
            "-n",
            "/usr/local/libexec/mcsr-toolkit",
            "privileged",
            "keyd",
            "clear"
        ]
    );
}

#[test]
fn unfinished_mutation_clears_existing_activation() {
    let (dir, fake, controller) = setup();
    let state = focus(false);
    controller.sync_focus(&Profile::default(), &state).unwrap();
    std::fs::write(dir.path().join("mutation.json"), b"{}").unwrap();
    controller.sync_focus(&Profile::default(), &state).unwrap();
    assert!(controller.input_safety_verified());
    assert_eq!(
        fake.calls.lock().unwrap().last().unwrap().0.last().unwrap(),
        "clear"
    );
}

#[test]
fn restore_to_rejects_stale_checkpoint_and_restores_snapshot() {
    let (_dir, fake, controller) = setup();
    controller.install(&Profile::default()).unwrap();
    let checkpoint = controller.checkpoint().unwrap();
    controller.install(&Profile::default()).unwrap();
    assert!(controller.restore_to(&["f".repeat(32)]).is_err());
    controller.restore_to(&checkpoint).unwrap();
    assert_eq!(controller.checkpoint().unwrap(), checkpoint);
    assert_eq!(
        fake.calls.lock().unwrap().last().unwrap().0.last().unwrap(),
        &"b".repeat(32)
    );
}

#[test]
fn focus_loss_modal_and_disconnect_clear_active_bindings() {
    for changed in [
        Some(FocusEvent {
            window_class: "Terminal".into(),
            title: "Shell".into(),
            modal: false,
        }),
        Some(FocusEvent {
            window_class: "Minecraft".into(),
            title: "Game".into(),
            modal: true,
        }),
        None,
    ] {
        let (_dir, fake, controller) = setup();
        let mut state = focus(false);
        controller.sync_focus(&Profile::default(), &state).unwrap();
        if let Some(event) = changed {
            state.observe(event);
        } else {
            state.disconnect();
        }
        controller.sync_focus(&Profile::default(), &state).unwrap();
        assert!(controller.input_safety_verified());
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls[2].0, ["/usr/bin/keyd.rvaiya", "bind", "reset"]);
        assert_eq!(calls[3].0.last().unwrap(), "clear");
    }
}
