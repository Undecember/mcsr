# Graph Report - release-v0.2.0  (2026-10-30)

## Corpus Check
- 149 files · ~175,699 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 172 file(s) not represented in the graph (top: .t 142, .conf 11, (none) 8)

## Summary
- 2709 nodes · 6600 edges · 128 communities (112 shown, 16 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 293 edges (avg confidence: 0.83)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d6df1a7e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- rust_install.rs
- maccel_helper.rs
- rust_keyd_helper.rs
- live-ui-acceptance.mjs
- rust_downloads.rs
- install.rs
- rust_standard_settings.rs
- rust_sensitivity.rs
- setup_helper.rs
- InstallError
- rust_apply.rs
- System
- extension_harness.cjs
- app.js
- RealSystem
- GnomeError
- rust_profile.rs
- runner.py
- install_inner
- api.rs
- rust_keyd.rs
- McsrToolkitExtension
- TrackingRuntime
- keyd.h
- rust_runtime.rs
- Profile
- .apply_inner
- rust_runtime_process.rs
- runtime.rs
- rust_instances.rs
- .start_with_adapters_inner
- Wayland integration acceptance
- rust_gnome.rs
- ServiceRuntime
- with
- apply.rs
- apply
- rust_api.rs
- paths.rs
- keyboard.c
- rust_assets.rs
- File map and task boundaries
- Enter Opens and Sends Chat Implementation Plan
- Review Focus
- ResizeError
- standard_settings.rs
- gnome.rs
- RealRootSetup
- GNOME Wayland focus probe
- keyd.rs
- BlockingDownload
- pathbuf
- bounded
- routing.rs
- Rust single-binary release checklist
- device.c
- Borderless Minecraft Resize Implementation Plan
- duration
- Binding input capture implementation plan
- Minecraft Control Picker Implementation Plan
- maccel.rs
- Key capture for binding fields
- Borderless Minecraft resize design
- MCSR Toolkit
- Review Focus
- config.c
- Uninstall keyd recovery implementation plan
- daemon.c
- uinput.c
- v0.1.0-beta.2 Release Preparation Plan
- Minecraft control picker
- GNOME Wayland resize and sensitivity acceptance
- SensitivityError
- AGENTS.md
- keyd-focus-probe.sh script
- mcsr-toolkit
- uninstall
- keyd_helper.rs
- parse_profile
- .apply_inner
- ipc-reset.c
- ServiceAction
- test-io.c
- Review Focus
- parse_descriptor
- Parallel execution ledger
- Standard Settings Implementation Plan
- Standard Settings integration
- FencedPeer
- HelperFilesystem
- keyd.c
- MCSR Toolkit maccel sensitivity recovery design
- rust_routing.rs
- F3 combinations take priority over resize shortcuts
- usb-gadget.c
- clear
- Review Focus
- Borderless full-size toggle live acceptance, 2026-10-30
- MCSR Toolkit Rust single-binary design
- managed_keyd.rs
- Managed panel visibility live test, 2026-10-30
- RootFile
- HelperSystem
- Borderless Full-Size Toggle Implementation Plan
- usb-gadget.md
- run.sh
- usb-gadget.sh script
- MCSR Toolkit v0.2.0 — release candidate
- Managed keyd live validation, 2026-10-29
- log.c
- Live Keyd Startup Implementation Plan
- Managed stock-panel visibility validation
- MaccelAvailability
- Live keyd startup from an empty baseline
- Borderless full-size alternative to fullscreen
- DirectorySync
- FakeSystem
- Managed keyd and F3 routing implementation verification
- Enter opens and sends chat
- Borderless full-size toggle: local implementation validation
- Managed Minecraft panel visibility
- Toolkit-managed keyd
- _updateVisibility

## God Nodes (most connected - your core abstractions)
1. `InstallError` - 159 edges
2. `System` - 87 edges
3. `Profile` - 72 edges
4. `GnomeError` - 58 edges
5. `install_inner()` - 57 edges
6. `Fake` - 53 edges
7. `RealSystem` - 45 edges
8. `Refused` - 37 edges
9. `profile()` - 37 edges
10. `original()` - 37 edges

## Surprising Connections (you probably didn't know these)
- `Context and goal` --references--> `get()`  [INFERRED]
  docs/superpowers/specs/2026-10-06-maccel-sensitivity-recovery-design.md → integration/live-ui-acceptance.mjs
- `Task 1: Panel ownership, lifecycle, and regressions` --references--> `show()`  [INFERRED]
  docs/superpowers/plans/2026-10-30-managed-panel-visibility.md → tests/extension_harness.cjs
- `Initial state and recovery` --references--> `clear()`  [INFERRED]
  docs/superpowers/evidence/2026-10-29-f3-keyd/live-validation.md → vendor/keyd/src/keyboard.c
- `Global Constraints` --references--> `clear()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-rust-single-binary.md → vendor/keyd/src/keyboard.c
- `Privilege boundary and input safety` --references--> `clear()`  [INFERRED]
  docs/superpowers/specs/2026-10-22-rust-single-binary-design.md → vendor/keyd/src/keyboard.c

## Import Cycles
- 1-file cycle: `src/keyd.rs -> src/keyd.rs`
- 2-file cycle: `src/mapping.rs -> src/options.rs -> src/mapping.rs`

## Communities (128 total, 16 thin omitted)

### Community 0 - "rust_install.rs"
Cohesion: 0.06
Nodes (90): repair(), aborted_root_commit_with_wait_error_still_restores_user_integration(), active_legacy_extension_hands_off_before_root_setup_and_remains_on_disk(), changed_extension_root_failure_preserves_shell_marker_and_old_lineage_for_retry(), changed_owned_extension_with_verified_lineage_waits_for_new_shell(), completed_manifest_write_with_shell_marker_recovers_after_interrupted_root_setup(), conflicting_prior_enabled_uuids_are_not_silently_claimed_restored(), corrupted_staged_helper_never_publishes_passwordless_policy() (+82 more)

### Community 1 - "maccel_helper.rs"
Cohesion: 0.06
Nodes (59): c_name(), dispatch(), EYE_DECIMAL, EYE_RAW, file_name(), FINGERPRINT, held_uninstall_operation_lock_blocks_concurrent_maccel_action(), HelperPaths (+51 more)

### Community 2 - "rust_keyd_helper.rs"
Cohesion: 0.12
Nodes (28): clear_reports_unsafe_when_config_is_untrusted_and_stop_fails(), clear_retries_initial_ipc_errors_then_requires_stable_readiness(), clear_stops_keyd_when_config_is_conflicting_or_writable(), concurrent_helper_invocation_is_refused_before_service_mutation(), DEVICE, existing_config_conflict_and_target_symlink_are_refused(), existing_managed_target_with_special_mode_is_refused(), failed_restart_rolls_back_and_failed_stop_reports_unsafe() (+20 more)

### Community 3 - "live-ui-acceptance.mjs"
Cohesion: 0.07
Nodes (53): allowed, baselineSha, baseUrl, browser(), canonical(), CDP, checkGameClosed(), checkListeningSocket() (+45 more)

### Community 4 - "rust_downloads.rs"
Cohesion: 0.06
Nodes (48): download_release(), DownloadError, Download, InvalidAssetUrl, InvalidDestination, InvalidJar, InvalidMetadata, InvalidRedirect (+40 more)

### Community 5 - "install.rs"
Cohesion: 0.06
Nodes (45): authenticated_status_command_has_a_deadline(), both_protected_recovery_histories_use_one_authenticated_status(), extension_digest(), extension_digest_from_fingerprints(), EXTENSION_UUID, ExtensionManifest, ExtensionStagingMarker, failed_commands_report_exit_status_and_both_output_streams() (+37 more)

### Community 6 - "rust_standard_settings.rs"
Cohesion: 0.28
Nodes (12): prepare_local(), bindings(), legacy_preserves_endings_blank_and_unrelated_values(), missing_and_unchanged_configs_are_handled_without_writes(), put(), rejects_legacy_redirect_chains_without_opening_shared_files(), rejects_malformed_and_duplicate_controls(), rejects_shared_redirects_without_touching_targets() (+4 more)

### Community 7 - "rust_sensitivity.rs"
Cohesion: 0.17
Nodes (21): cancelled_newer_restore_does_not_make_older_token_usable(), EYE_REPLY, eye_resize_failure_propagates_without_sensitivity_write(), f6_restores_exact_baseline_even_when_resize_fails(), f7_changes_sensitivity_only_after_exact_resize(), failed_eye_after_write_restores_while_focus_remains_effective(), failed_eye_and_failed_recovery_report_both_and_remain_pending(), failed_preflight_invalidates_generation_until_restore() (+13 more)

### Community 8 - "setup_helper.rs"
Cohesion: 0.06
Nodes (61): HELPER_PATH, KEYD_POLICY_PATH, MACCEL_POLICY_PATH, SERVICE_PATH, abort_and_eof_roll_back_only_new_files(), check_components(), Decision, Abort (+53 more)

### Community 9 - "InstallError"
Cohesion: 0.09
Nodes (7): InstallError, System, InstallGuard, RootSetup, Fake, FakeSetup, FakeUninstallSetup

### Community 10 - "rust_apply.rs"
Cohesion: 0.13
Nodes (56): applies_two_closed_instances_and_restores_exact_bytes(), apply_publication_runs_under_marker_and_failure_keeps_marker(), backup_is_bound_to_launcher_root(), before_restore_runs_after_validation_and_before_marker(), changed_standard_file_rejects_restore_before_any_mutation(), dangling_recovery_record_symlinks_block_apply(), enter_chat_preserves_hotbar_through_apply_and_restore(), existing_mutation_lock_blocks_another_apply() (+48 more)

### Community 11 - "System"
Cohesion: 0.08
Nodes (10): enabled_catalog_probe_error_is_not_classified_as_disabled(), ExtensionAction, Disable, Enable, install(), marker_parent_sync_failure_is_returned_before_root_setup(), restore_user(), sync_parent() (+2 more)

### Community 12 - "extension_harness.cjs"
Cohesion: 0.03
Nodes (37): assert, beforeDrain, beforeReconfigure, bindings, callbacks, centeredReply, centeredWindow, context (+29 more)

### Community 13 - "app.js"
Cohesion: 0.06
Nodes (48): Task 1: Replace privileged reload with bounded restart and rollback, action(), addMappingRow(), addPresetRow(), api(), attachRecorder(), canApply(), changed() (+40 more)

### Community 14 - "RealSystem"
Cohesion: 0.13
Nodes (8): Artifact, cached_policy_bytes_match(), Refused, RealSystem, start_root_transaction(), toolkit_object_from_introspection(), verified_executable(), verified_helper()

### Community 15 - "GnomeError"
Cohesion: 0.10
Nodes (8): GnomeBridge<SessionPeer>, GnomeError, GnomePeer, NonReplyingPeer, BlockingGnome, FakeGnome, FakeGnome, FakeGnome

### Community 16 - "rust_profile.rs"
Cohesion: 0.16
Nodes (17): decode(), encode(), keyd_to_minecraft(), MappingError, minecraft_to_keyd(), plan_controls(), supported_other_binding(), chat_conflict_is_reported_instead_of_changing_other_control() (+9 more)

### Community 17 - "runner.py"
Cohesion: 0.06
Nodes (11): main(), run(), is_mouse_button(), Key, diff(), KeyStream, run_test(), printerr() (+3 more)

### Community 18 - "install_inner"
Cohesion: 0.15
Nodes (16): enable_extension_checked(), extension_enabled_from_output(), extension_named_in_output(), extension_state_from_info(), ExtensionState, Active, Error, Inactive (+8 more)

### Community 19 - "api.rs"
Cohesion: 0.11
Nodes (15): ApiConfig, ApiResult, ApiState, asset(), asset_at(), constant_time_eq(), gate(), get_profile() (+7 more)

### Community 20 - "rust_keyd.rs"
Cohesion: 0.06
Nodes (37): FocusEvent, FocusListener, FocusState, FocusTransition, Gained, Lost, Unchanged, HEARTBEAT_TIMEOUT (+29 more)

### Community 21 - "McsrToolkitExtension"
Cohesion: 0.08
Nodes (12): Global Constraints, Managed Panel Visibility Implementation Plan, Preparation, Review Focus, Self-review, Task 1: Panel ownership, lifecycle, and regressions, Task 2: Documentation, combined verification, and review, McsrToolkitExtension (+4 more)

### Community 22 - "TrackingRuntime"
Cohesion: 0.06
Nodes (6): BlockingRuntime, FakeDownload, NativeKeyd, RejectPublicationRuntime, Runtime, TrackingRuntime

### Community 24 - "rust_runtime.rs"
Cohesion: 0.16
Nodes (32): absent_optional_maccel_serves_observe_only_without_helper_calls(), clean_shutdown_releases_the_service_lock(), config(), duplicate_runtime_refuses_same_session_lock(), extension_connection_survives_a_non_minecraft_heartbeat(), failed_eye_recovery_still_clears_previous_keyd_bindings(), failed_keyd_discovery_keeps_fallback_vocabulary(), failed_startup_restore_prevents_eye_acceptance_and_http_binding() (+24 more)

### Community 25 - "Profile"
Cohesion: 0.25
Nodes (8): ApiBackend, NativeBackend, ApplyPreview, ApplyReport, InstanceResult, Profile, FakeBackend, report()

### Community 26 - ".apply_inner"
Cohesion: 0.17
Nodes (13): ApplyCoordinator, ApplyError, err(), KeydAdapter, mod_kind(), record_present(), valid_id(), T (+5 more)

### Community 27 - "rust_runtime_process.rs"
Cohesion: 0.17
Nodes (6): active_python_style_service_lock_blocks_rust_start(), assert_restart_with_retained_writer(), fake_service_child(), FakeMaccel, KillOnDrop, signal_restart_keeps_writer_drains_stale_fifo_and_restores_before_new_eye()

### Community 28 - "runtime.rs"
Cohesion: 0.10
Nodes (15): ResizeResult, safe_record(), apply_signal(), configure_routes(), err(), load_profile(), NativeDownloads, process_cwds_under() (+7 more)

### Community 29 - "rust_instances.rs"
Cohesion: 0.11
Nodes (23): discover_instances(), ControlPlan, ManagedState, OptionsDocument, OptionsError, parse_options(), render_options(), render_windowed_options() (+15 more)

### Community 30 - ".start_with_adapters_inner"
Cohesion: 0.20
Nodes (6): DownloadAdapter, CommandRunner, SystemCommandRunner, discover_keys(), RuntimeConfig, ServiceRuntime<P, H>

### Community 31 - "Wayland integration acceptance"
Cohesion: 0.08
Nodes (24): 2026-10-05 browser UI acceptance (Task 9), 2026-10-05 keyd helper recovery check, 2026-10-05 supervised authentication input check, 2026-10-05 supervised live focus and chat check, 2026-10-06 approved helper deployment and current state, 2026-10-06 approved movement-test attempt, 2026-10-06 exact observe-only F6 resize acceptance, 2026-10-06 supervised movement test, second attempt (+16 more)

### Community 32 - "rust_gnome.rs"
Cohesion: 0.10
Nodes (12): bridge_stale_check_leaves_expiry_transition_for_runtime_watch(), captured_special_keys_route_through_internal_gnome_accelerators(), failed_configuration_invalidates_previous_signal_generation(), FakePeer, identical_profile_reconnect_rejects_old_activation_signal(), nonreplying_configuration_times_out(), nonreplying_focus_probe_times_out(), profile_configuration_uses_extension_json_shape() (+4 more)

### Community 33 - "ServiceRuntime"
Cohesion: 0.33
Nodes (4): SharedFocus, GnomeBridge, PresetController, ServiceRuntime

### Community 34 - "with"
Cohesion: 0.10
Nodes (19): jobs, verify, name, on, pull_request, push, workflow_dispatch, permissions (+11 more)

### Community 35 - "apply.rs"
Cohesion: 0.14
Nodes (10): BackupFile, ControlMapping, History, Manifest, MutationLock, new_id(), NEXT_BACKUP, Prepared (+2 more)

### Community 36 - "apply"
Cohesion: 0.14
Nodes (9): ApiError, apply(), download(), Job, JobQueue, QueueInner, report_json(), restore() (+1 more)

### Community 37 - "rust_api.rs"
Cohesion: 0.17
Nodes (18): router(), state_dir(), completed_backup_id(), completed_job(), download_allowlist_and_unknown_job(), embedded_assets_session_and_status_have_ui_contract(), failed_runtime_apply_publication_keeps_marker_and_notifies_runtime(), failed_runtime_restore_publication_keeps_marker_and_notifies_runtime() (+10 more)

### Community 38 - "paths.rs"
Cohesion: 0.18
Nodes (7): directory_fd(), existing_permissive_parent_is_tightened_without_touching_records_or_data_home(), NEXT_TEMP, prepare_private_state(), prepare_private_state_for_uid(), private_child(), private_state_creation_ignores_permissive_umask_and_runtime_accepts_it()

### Community 39 - "keyboard.c"
Cohesion: 0.17
Nodes (29): abort_chord(), activate_layer(), cache_get(), cache_set(), calculate_main_loop_timeout(), check_chord_match(), chord_event_match(), clear_mod() (+21 more)

### Community 40 - "rust_assets.rs"
Cohesion: 0.16
Nodes (5): extension_archive(), web_asset(), app_javascript_is_embedded(), extension_archive_contains_sources_and_compiled_schema(), root_serves_embedded_html()

### Community 41 - "File map and task boundaries"
Cohesion: 0.11
Nodes (18): Execution and approval gates, File map and task boundaries, Global Constraints, Review Focus, Rust Single-Binary Migration Implementation Plan, Task 10: Root-owned maccel helper, Task 11: Unprivileged sensitivity and preset controller, Task 12: Service runtime and lifecycle (+10 more)

### Community 42 - "Enter Opens and Sends Chat Implementation Plan"
Cohesion: 0.22
Nodes (7): Enter Opens and Sends Chat Implementation Plan, Global Constraints, Handoff, Review Focus, Task 1: Profile and mapping contract, Task 2: Browser acceptance contract and documentation, Task 3: Final supervised device check

### Community 43 - "Review Focus"
Cohesion: 0.13
Nodes (13): Global Constraints, Handoff, Review Focus, Rust Source Cutover Implementation Plan, Task 1: Native legacy-lock test fixture, Task 2: Remove Python product and test tree, Task 3: Rust-only active documentation, Task 4: Final deterministic and release-boundary verification (+5 more)

### Community 44 - "ResizeError"
Cohesion: 0.25
Nodes (5): ResizeError, Gnome, Inexact, InvalidSize, NotFocused

### Community 45 - "standard_settings.rs"
Cohesion: 0.15
Nodes (7): fail(), json_edit(), legacy_edit(), local_file(), prepare_local_with_windowed(), StandardSettingsError, Unique

### Community 46 - "gnome.rs"
Cohesion: 0.18
Nodes (10): accelerator(), bus_error(), DESTINATION, FOCUS_TIMEOUT, INTERFACE, is_function_key(), METHOD_TIMEOUT, OBJECT_PATH (+2 more)

### Community 47 - "RealRootSetup"
Cohesion: 0.22
Nodes (4): abandoned_setup_terminates_privileged_process_group(), classify_root_commit_response(), RootAborted, RealRootSetup

### Community 48 - "GNOME Wayland focus probe"
Cohesion: 0.17
Nodes (12): Chat focus risk, Failed chat safety test and cleanup, Failed supervised chat-only probe, 2026-10-04, GNOME Wayland focus probe, Initial environment at first inspection (2026-10-03), Observed after GNOME extension activation, Production observe-only service, 2026-10-04, Restart-helper recovery check, 2026-10-05 (+4 more)

### Community 49 - "keyd.rs"
Cohesion: 0.08
Nodes (20): Backups, child_that_does_not_read_stdin_is_killed_without_blocking_writer(), CommandFailure, Failed, TimedOut, CommandOutput, hung_child_is_killed_before_deadline_expires_twice(), KeydController (+12 more)

### Community 51 - "pathbuf"
Cohesion: 0.10
Nodes (12): build_managed_keyd(), copy_tree(), EXTENSION_FILES, main(), verify_archive(), write_archive(), DiscoveryError, launcher_root() (+4 more)

### Community 52 - "bounded"
Cohesion: 0.15
Nodes (6): is_minecraft_class(), bounded(), canonical_shortcut(), GnomeBridge<P>, Peer, SessionPeer

### Community 53 - "routing.rs"
Cohesion: 0.24
Nodes (6): game_bindings(), render_keyd_fragment(), RoutingPlan, shortcut_parts(), Signal, enter_opens_and_sends_chat()

### Community 54 - "Rust single-binary release checklist"
Cohesion: 0.07
Nodes (27): 2026-10-26 Enter dual-role run, Candidate and reproducible checks, Candidate verification, Candidate verification, Historical v0.1.0-beta.1 preparation record, Live scope and readiness limits, Preparation checklist, Preparation checklist (+19 more)

### Community 55 - "device.c"
Cohesion: 0.12
Nodes (18): device_grab(), device_init(), device_read_event(), device_scan(), device_scan_worker(), device_ungrab(), devmon_create(), devmon_read_device() (+10 more)

### Community 56 - "Borderless Minecraft Resize Implementation Plan"
Cohesion: 0.20
Nodes (8): Approval handoff, Borderless Minecraft Resize Implementation Plan, Global Constraints, Research references, Review Focus, Task 1: Prepare decorated XWayland Minecraft windows asynchronously, Task 2: Apply exact dimensions and close every request lifecycle, Task 3: Verify service integration, packaged assets, and user guidance

### Community 57 - "duration"
Cohesion: 0.15
Nodes (6): blur_and_quick_refocus_cannot_publish_stale_preset(), event(), graceful_restart_preserves_writer_and_discards_downtime_focus(), heartbeat_timeout_clears_focus(), modal_and_disconnect_clear_focus(), preset_signal_requires_live_fifo_heartbeat_even_when_bus_reports_minecraft()

### Community 58 - "Binding input capture implementation plan"
Cohesion: 0.25
Nodes (7): Binding input capture implementation plan, Execution notes, Global constraints, Review focus, Task 1: Supported runtime names and Minecraft mouse mappings, Task 2: Shared recorder and field integration, Task 3: Verification and review

### Community 59 - "Minecraft Control Picker Implementation Plan"
Cohesion: 0.25
Nodes (7): Execution notes, Global Constraints, Integration recheck, Minecraft Control Picker Implementation Plan, Review Focus, Task 1: Searchable picker and profile integration, Verification results

### Community 60 - "maccel.rs"
Cohesion: 0.12
Nodes (9): COMMAND_TIMEOUT, empty_eye_probe_proves_noninteractive_permission_without_a_token(), EYE_REPLY, HELPER, SensitivityController, SensitivityController<H>, State, SUDO (+1 more)

### Community 61 - "Key capture for binding fields"
Cohesion: 0.29
Nodes (6): Approaches, Behavior, Key capture for binding fields, Scope, Validation, Verified runtime constraints

### Community 62 - "Borderless Minecraft resize design"
Cohesion: 0.20
Nodes (10): Approaches, Borderless Minecraft resize design, Corrective fullscreen boundary after live regression, Current behavior, Platform boundary, References, Requested behavior, Resize lifecycle and failures (+2 more)

### Community 63 - "MCSR Toolkit"
Cohesion: 0.22
Nodes (9): Build and deterministic checks, Live keyd mode and recovery, Managed keyd and F3 priority, MCSR Toolkit, Repair and uninstall, Resize presets, Run and use the UI, Rust executable and release status (+1 more)

### Community 65 - "Review Focus"
Cohesion: 0.22
Nodes (9): File map, Global Constraints, Handoff, MCSR Toolkit maccel Sensitivity Recovery Implementation Plan, Review Focus, Task 1: Implement the isolated root helper, Task 2: Add the unprivileged fixed-command adapter, Task 3: Integrate startup, focus, and shutdown recovery (+1 more)

### Community 66 - "config.c"
Cohesion: 0.17
Nodes (22): append_line(), config_add_entry(), config_add_layer(), config_check_match(), config_get_layer_index(), config_init(), config_parse(), config_warn() (+14 more)

### Community 67 - "Uninstall keyd recovery implementation plan"
Cohesion: 0.33
Nodes (5): Review focus, Scope and constraints, Task 1: Uninstall recovery, Task 2: Recovery diagnostics and validation, Uninstall keyd recovery implementation plan

### Community 68 - "daemon.c"
Cohesion: 0.20
Nodes (18): add_listener(), cleanup(), clear_vkbd(), event_handler(), free_configs(), handle_client(), load_configs(), lookup_config_ent() (+10 more)

### Community 69 - "uinput.c"
Cohesion: 0.12
Nodes (9): xwrite(), create_virtual_keyboard(), create_virtual_pointer(), vkbd_init(), vkbd_mouse_move(), vkbd_mouse_move_abs(), vkbd_mouse_scroll(), vkbd_send_key() (+1 more)

### Community 70 - "v0.1.0-beta.2 Release Preparation Plan"
Cohesion: 0.40
Nodes (4): Constraints, Review Focus, Task 1: Candidate and documentation, v0.1.0-beta.2 Release Preparation Plan

### Community 71 - "Minecraft control picker"
Cohesion: 0.40
Nodes (4): Approaches, Behavior and constraints, Minecraft control picker, Scope and validation

### Community 72 - "GNOME Wayland resize and sensitivity acceptance"
Cohesion: 0.22
Nodes (9): Borderless full-size toggle candidate (2026-10-30), Current Rust observation procedure, Current Rust release gates, Fullscreen restoration regression and recovery (2026-10-29), GNOME Wayland resize and sensitivity acceptance, Historical evidence (2026-10-04–06), Historical sensitivity and helper acceptance (2026-10-22), Managed stock-panel visibility candidate (2026-10-30) (+1 more)

### Community 73 - "SensitivityError"
Cohesion: 0.15
Nodes (5): Helper, InstalledMaccelPeer, MaccelPeer, SensitivityError, FakeMaccel

### Community 80 - "uninstall"
Cohesion: 0.14
Nodes (10): archive_files(), check_uninstall_records(), Environment, http_probe(), InstallReport, preflight(), refuse_after_stop(), stage_extension() (+2 more)

### Community 81 - "keyd_helper.rs"
Cohesion: 0.15
Nodes (19): apply_and_restart(), apply_snapshot(), atomic_write(), current(), DEVICE, DEVICE_NAME, FRAGMENT_NAME, Manifest (+11 more)

### Community 82 - "parse_profile"
Cohesion: 0.15
Nodes (13): ChatKeys, deserialize_bool(), deserialize_dimension(), key(), KeySet, modifier_family(), parse_dimension_string(), parse_profile() (+5 more)

### Community 83 - ".apply_inner"
Cohesion: 0.47
Nodes (3): PresetController<P, H>, PresetError, rejected_profile_publication_recovers_before_newer_eye_write()

### Community 84 - "ipc-reset.c"
Cohesion: 0.13
Nodes (7): expect_bind(), expect_bind_bytes(), main(), expect(), main(), new_keyboard(), xread()

### Community 85 - "ServiceAction"
Cohesion: 0.29
Nodes (6): ServiceAction, Disable, Enable, Reload, Start, Stop

### Community 86 - "test-io.c"
Cohesion: 0.31
Nodes (8): cmp_events(), get_time_ns(), lookup_code(), main(), parse_events(), print_diff(), read_file(), run_test()

### Community 87 - "Review Focus"
Cohesion: 0.12
Nodes (18): File map, Global Constraints, MCSR Toolkit Implementation Plan, Review Focus, Source notes for implementers, Task 10: Complete documentation and end-to-end verification, Task 1: Prove the GNOME Wayland integration path, Task 2: Define the profile and validation contract (+10 more)

### Community 88 - "parse_descriptor"
Cohesion: 0.18
Nodes (14): parse_command(), parse_descriptor(), parse_fn(), parse_macro_expression(), input(), parse_key_sequence(), macro_execute(), macro_parse() (+6 more)

### Community 89 - "Parallel execution ledger"
Cohesion: 0.29
Nodes (7): Current-machine inspection (2026-10-29), Execution results and outstanding decision, Fullscreen/focus regression correction (2026-10-29), Merge authorization, Parallel execution ledger, Supervised Jingle thin test and centering amendment (2026-10-29), unmaximize()

### Community 90 - "Standard Settings Implementation Plan"
Cohesion: 0.20
Nodes (9): Execution ledger, Final verification and review, Global Constraints, Handoff, Local integration authorization, Review Focus, Standard Settings Implementation Plan, Task 1: Local config discovery and rendering (+1 more)

### Community 91 - "Standard Settings integration"
Cohesion: 0.29
Nodes (6): Approach, Intent and approved scope, Recovery and compatibility, Standard Settings integration, Upstream evidence, Verification

### Community 93 - "HelperFilesystem"
Cohesion: 0.26
Nodes (8): dispatch(), HelperFilesystem, load_manifest(), recovery_pending(), save_manifest(), select_keyd_client_with(), status_under_lock(), valid_id()

### Community 94 - "keyd.c"
Cohesion: 0.20
Nodes (12): chgid(), ipc_connect(), ipc_create_server(), add_bindings(), cmd_do(), help(), input(), ipc_exec() (+4 more)

### Community 95 - "MCSR Toolkit maccel sensitivity recovery design"
Cohesion: 0.20
Nodes (6): Context and goal, MCSR Toolkit maccel sensitivity recovery design, Privilege boundary, Service behavior and failure recovery, Sources, Verification and acceptance

### Community 96 - "rust_routing.rs"
Cohesion: 0.24
Nodes (13): chat_restores_resize_and_f3_sources_to_physical_keys(), combination_profile(), command_chat_opened_while_f3_is_held_keeps_typed_resize_keys(), engine(), eye_zoom_and_modified_shortcuts_match_exact_modifiers(), f3_remap_and_resize_use_separate_outputs(), generated_engine_handles_native_and_remapped_f3_together(), generated_engine_preserves_all_five_combinations_and_release_orders() (+5 more)

### Community 97 - "F3 combinations take priority over resize shortcuts"
Cohesion: 0.14
Nodes (12): Held-F3 feasibility spike, Reproduce, Result, Activation, failure, and recovery contract, Approaches, Cause, F3 combinations take priority over resize shortcuts, Held-key feasibility evidence and dependency decision (+4 more)

### Community 98 - "usb-gadget.c"
Cohesion: 0.22
Nodes (7): create_virtual_keyboard(), get_modifier(), send_hid_report(), update_key_state(), update_modifier_state(), vkbd_init(), vkbd_send_key()

### Community 99 - "clear"
Cohesion: 0.14
Nodes (18): File map, Global Constraints, Keyd Restart Recovery Implementation Plan, Review Focus, Task 2: Wire the exact clear command and deployment preflight, Task 3: Persist and serialize focus suspension, Task 4: Prove live behavior and empty-config rollback before UI, Task 5: Reconcile documentation, verification, and independent review (+10 more)

### Community 100 - "Review Focus"
Cohesion: 0.22
Nodes (8): Execution ledger, F3 Priority and Managed Keyd Implementation Plan, Global Constraints, Review Focus, Task 1: Reproducible managed keyd engine and embedded asset, Task 2: Fixed-target install, selection, and restoration, Task 3: Compile shared routing and coordinate applied generation, Task 4: Review and user-facing validation record

### Community 101 - "Borderless full-size toggle live acceptance, 2026-10-30"
Cohesion: 0.33
Nodes (6): Borderless full-size toggle live acceptance, 2026-10-30, Chat, release, and ordinary-input results, F3 priority results, Observed windowed transitions and focus recovery, Post-login repair and preparation, Verified cleanup and final scope

### Community 102 - "MCSR Toolkit Rust single-binary design"
Cohesion: 0.29
Nodes (7): Cutover, state, and failure handling, Executable and component boundaries, Goal and release boundary, MCSR Toolkit Rust single-binary design, Migration and verification strategy, Privilege boundary and input safety, Sources

### Community 103 - "managed_keyd.rs"
Cohesion: 0.29
Nodes (5): BINARY_PATH, CAPABILITIES, DROPIN, DROPIN_PATH, UPSTREAM_REVISION

### Community 104 - "Managed panel visibility live test, 2026-10-30"
Cohesion: 0.33
Nodes (6): First panel check, Focus and modal observations, Managed panel visibility live test, 2026-10-30, Post-login preparation, Staged for login, Verified cleanup

### Community 105 - "RootFile"
Cohesion: 0.33
Nodes (3): existing_policy(), policy(), RootFile

### Community 106 - "HelperSystem"
Cohesion: 0.21
Nodes (3): HelperSystem, RealSystem, restart_healthy()

### Community 107 - "Borderless Full-Size Toggle Implementation Plan"
Cohesion: 0.17
Nodes (11): Borderless Full-Size Toggle Implementation Plan, Global Constraints, Parallel execution ledger, Preparation, Review Focus, Self-review, Shared Interfaces and Dependencies, Task 1: Extension geometry toggle (+3 more)

### Community 111 - "MCSR Toolkit v0.2.0 — release candidate"
Cohesion: 0.40
Nodes (5): Candidate identity and publication boundary, Changes since v0.1.0, Download, installation and upgrade, Live evidence and known limitations, MCSR Toolkit v0.2.0 — release candidate

### Community 112 - "Managed keyd live validation, 2026-10-29"
Cohesion: 0.18
Nodes (11): Candidate and fresh automated checks, Confirmed outcome and completed test cleanup, First Minecraft observations and fullscreen recovery failure, Fixed-candidate deployment and input-fence blocker, 2026-10-30, Initial state and recovery, Managed keyd live validation, 2026-10-29, Post-login repair and live startup failure, Pre-login checkpoint (+3 more)

### Community 113 - "log.c"
Cohesion: 0.25
Nodes (6): dbg_print_device_bits(), dbg_print_evdev_details(), colorize(), die(), _keyd_log(), _vkeyd_log()

### Community 114 - "Live Keyd Startup Implementation Plan"
Cohesion: 0.20
Nodes (9): Global Constraints, Live Keyd Startup Implementation Plan, Parallel execution ledger, Plan self-review, Preparation, Review Focus, Task 1: Empty-config reset through the real IPC dispatcher, Task 2: Bounded readiness after restart (+1 more)

### Community 115 - "Managed stock-panel visibility validation"
Cohesion: 0.22
Nodes (9): API evidence, Baseline, Combined checks, Frozen Task 1 candidate, Managed stock-panel visibility validation, Modal review repair, Provenance and execution, Regression-first development (+1 more)

### Community 116 - "MaccelAvailability"
Cohesion: 0.22
Nodes (6): MaccelAvailability, Available, RecoveryUncertain, Unavailable, optional_maccel_availability(), ServiceRuntime<SessionPeer, InstalledMaccelPeer>

### Community 118 - "Live keyd startup from an empty baseline"
Cohesion: 0.29
Nodes (6): Acceptance and verification, Approach and alternatives, Evidence, Intent, Live keyd startup from an empty baseline, Scope and constraints

### Community 119 - "Borderless full-size alternative to fullscreen"
Cohesion: 0.29
Nodes (6): Acceptance, Behavior, Borderless full-size alternative to fullscreen, Implementation boundaries, Intent and selected approach, Windowed game configuration

### Community 120 - "DirectorySync"
Cohesion: 0.29
Nodes (3): DirectorySync, RealDirectorySync, RecordingSync

### Community 122 - "Managed keyd and F3 routing implementation verification"
Cohesion: 0.40
Nodes (4): Automated results, Limits, Managed keyd and F3 routing implementation verification, Review findings and regressions

### Community 124 - "Enter opens and sends chat"
Cohesion: 0.29
Nodes (7): Approaches, Behavior and constraints, Enter opens and sends chat, Intent, Limits, Verification and rollout, cancel()

### Community 126 - "Borderless full-size toggle: local implementation validation"
Cohesion: 0.33
Nodes (6): Borderless full-size toggle: local implementation validation, Desktop boundary, Final combined deterministic checks, Regression and independent review evidence, Resulting behavior, Source and provenance

### Community 127 - "Managed Minecraft panel visibility"
Cohesion: 0.33
Nodes (5): Behavior and ownership, Intent and selected approach, Managed Minecraft panel visibility, Scope and constraints, Verification

### Community 129 - "Toolkit-managed keyd"
Cohesion: 0.67
Nodes (3): Maintained patch, Routing transition protocol, Toolkit-managed keyd

## Knowledge Gaps
- **45 isolated node(s):** `mcsr-toolkit`, `Launcher`, `Ninjabrain`, `InvalidMetadata`, `InvalidAssetUrl` (+40 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 780 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **16 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `clear()` connect `clear` to `MCSR Toolkit Rust single-binary design`, `keyboard.c`, `File map and task boundaries`, `Managed keyd live validation, 2026-10-29`, `GNOME Wayland focus probe`, `Review Focus`, `Wayland integration acceptance`?**
  _High betweenness centrality (0.290) - this node is a cross-community bridge._
- **Are the 7 inferred relationships involving `GnomeError` (e.g. with `.configure_presets()` and `.get_focus()`) actually correct?**
  _`GnomeError` has 7 INFERRED edges - model-reasoned connections that need verification._
- **What connects `mcsr-toolkit`, `Launcher`, `Ninjabrain` to the rest of the system?**
  _45 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `rust_install.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06265143648321218 - nodes in this community are weakly interconnected._
- **Why does `Review Focus` connect `clear` to `app.js`?**
  _High betweenness centrality (0.140) - this node is a cross-community bridge._
- **Should `maccel_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05811027224891329 - nodes in this community are weakly interconnected._
- **Why does `Task 1: Replace privileged reload with bounded restart and rollback` connect `app.js` to `clear`?**
  _High betweenness centrality (0.132) - this node is a cross-community bridge._