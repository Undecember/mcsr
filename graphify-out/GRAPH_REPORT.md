# Graph Report - mcsr  (2026-10-30)

## Corpus Check
- 154 files · ~184,965 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 171 file(s) not represented in the graph (top: .t 142, .conf 11, (none) 7)

## Summary
- 2873 nodes · 7049 edges · 130 communities (114 shown, 16 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 306 edges (avg confidence: 0.83)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `498c2a43`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- rust_install.rs
- maccel_helper.rs
- rust_keyd_helper.rs
- live-ui-acceptance.mjs
- rust_downloads.rs
- install.rs
- KeydController
- rust_sensitivity.rs
- setup_helper.rs
- InstallError
- rust_apply.rs
- install_prerequisites.rs
- extension_harness.cjs
- app.js
- RealSystem
- GnomeError
- rust_profile.rs
- runner.py
- install_inner
- api.rs
- focus.rs
- McsrToolkitExtension
- RuntimeAdapter
- keyd.h
- rust_runtime.rs
- downloads.rs
- .apply_inner
- DownloadError
- runtime.rs
- rust_instances.rs
- CommandRunner
- Wayland integration acceptance
- rust_gnome.rs
- Product
- with
- apply.rs
- ApiState
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
- rust_keyd.rs
- keyd.rs
- build.rs
- Review Focus
- .start_with_adapters_inner
- Rust single-binary release checklist
- device.c
- Borderless Minecraft Resize Implementation Plan
- rust_standard_settings.rs
- Binding input capture implementation plan
- Minecraft Control Picker Implementation Plan
- maccel.rs
- Key capture for binding fields
- Borderless Minecraft resize design
- MCSR Toolkit
- resize-wayland.md
- v0.1.0 stable candidate — 2026-10-29
- config.c
- Uninstall keyd recovery implementation plan
- daemon.c
- uinput.c
- v0.1.0-beta.2 Release Preparation Plan
- Minecraft control picker
- GNOME Wayland resize and sensitivity acceptance
- rust_runtime_process.rs
- AGENTS.md
- keyd-focus-probe.sh script
- mcsr-toolkit
- v0.1.0-beta.3 candidate — 2026-10-29
- keyd_helper.rs
- parse_profile
- Guided installation with live keyd by default
- ipc-reset.c
- ServiceAction
- test-io.c
- Review Focus
- parse_descriptor
- Parallel execution ledger
- Standard Settings Implementation Plan
- Standard Settings integration
- .listen_presets
- HelperFilesystem
- keyd.c
- MCSR Toolkit Rust single-binary design
- rust_routing.rs
- F3 combinations take priority over resize shortcuts
- usb-gadget.c
- clear
- Review Focus
- Borderless full-size toggle live acceptance, 2026-10-30
- installed_live_mode
- managed_keyd.rs
- Managed panel visibility live test, 2026-10-30
- Profile
- HelperSystem
- Borderless Full-Size Toggle Implementation Plan
- usb-gadget.md
- run.sh
- usb-gadget.sh script
- MCSR Toolkit v0.2.0
- Managed keyd live validation, 2026-10-29
- log.c
- Live Keyd Startup Implementation Plan
- Managed stock-panel visibility validation
- ServiceRuntime
- Environment
- Live keyd startup from an empty baseline
- Borderless full-size alternative to fullscreen
- DirectorySync
- FakeSystem
- Managed keyd and F3 routing implementation verification
- MaccelAvailability
- Enter opens and sends chat
- BlockingDownload
- Borderless full-size toggle: local implementation validation
- Managed Minecraft panel visibility
- Toolkit-managed keyd
- _updateVisibility

## God Nodes (most connected - your core abstractions)
1. `InstallError` - 180 edges
2. `System` - 100 edges
3. `Profile` - 72 edges
4. `install_inner()` - 63 edges
5. `Fake` - 60 edges
6. `GnomeError` - 58 edges
7. `RealSystem` - 55 edges
8. `Refused` - 42 edges
9. `profile()` - 37 edges
10. `original()` - 37 edges

## Surprising Connections (you probably didn't know these)
- `Context and goal` --references--> `get()`  [INFERRED]
  docs/superpowers/specs/2026-10-06-maccel-sensitivity-recovery-design.md → integration/live-ui-acceptance.mjs
- `Initial state and recovery` --references--> `clear()`  [INFERRED]
  docs/superpowers/evidence/2026-10-29-f3-keyd/live-validation.md → vendor/keyd/src/keyboard.c
- `Global Constraints` --references--> `clear()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-rust-single-binary.md → vendor/keyd/src/keyboard.c
- `Privilege boundary and input safety` --references--> `clear()`  [INFERRED]
  docs/superpowers/specs/2026-10-22-rust-single-binary-design.md → vendor/keyd/src/keyboard.c
- `Earlier automated recovery verification, 2026-10-05 (203-test run)` --references--> `clear()`  [INFERRED]
  integration/acceptance-wayland.md → vendor/keyd/src/keyboard.c

## Import Cycles
- 1-file cycle: `src/keyd.rs -> src/keyd.rs`
- 2-file cycle: `src/mapping.rs -> src/options.rs -> src/mapping.rs`

## Communities (130 total, 16 thin omitted)

### Community 0 - "rust_install.rs"
Cohesion: 0.05
Nodes (109): repair(), aborted_root_commit_with_wait_error_still_restores_user_integration(), active_legacy_extension_hands_off_before_root_setup_and_remains_on_disk(), changed_extension_root_failure_preserves_shell_marker_and_old_lineage_for_retry(), changed_owned_extension_with_verified_lineage_waits_for_new_shell(), completed_manifest_write_with_shell_marker_recovers_after_interrupted_root_setup(), conflicting_prior_enabled_uuids_are_not_silently_claimed_restored(), corrupted_staged_helper_never_publishes_passwordless_policy() (+101 more)

### Community 1 - "maccel_helper.rs"
Cohesion: 0.06
Nodes (59): c_name(), dispatch(), EYE_DECIMAL, EYE_RAW, file_name(), FINGERPRINT, held_uninstall_operation_lock_blocks_concurrent_maccel_action(), HelperPaths (+51 more)

### Community 2 - "rust_keyd_helper.rs"
Cohesion: 0.12
Nodes (33): clear_reports_unsafe_when_config_is_untrusted_and_stop_fails(), clear_retries_initial_ipc_errors_then_requires_stable_readiness(), clear_stops_keyd_when_config_is_conflicting_or_writable(), concurrent_helper_invocation_is_refused_before_service_mutation(), DEVICE, existing_config_conflict_and_target_symlink_are_refused(), existing_managed_target_with_special_mode_is_refused(), failed_restart_rolls_back_and_failed_stop_reports_unsafe() (+25 more)

### Community 3 - "live-ui-acceptance.mjs"
Cohesion: 0.06
Nodes (62): File map, Global Constraints, Handoff, MCSR Toolkit maccel Sensitivity Recovery Implementation Plan, Review Focus, Task 1: Implement the isolated root helper, Task 2: Add the unprivileged fixed-command adapter, Task 3: Integrate startup, focus, and shutdown recovery (+54 more)

### Community 4 - "rust_downloads.rs"
Cohesion: 0.19
Nodes (21): assert_preserved(), asset(), BrokenReader, corrupt_entry_data_preserves_previous_file(), deflated_jar(), destination_must_be_a_jar(), empty_manifest_is_not_a_valid_jar(), interrupted_stream_preserves_previous_file() (+13 more)

### Community 5 - "install.rs"
Cohesion: 0.05
Nodes (55): both_protected_recovery_histories_use_one_authenticated_status(), cached_policy_bytes_match(), enabled_catalog_probe_error_is_not_classified_as_disabled(), extension_digest(), extension_digest_from_fingerprints(), EXTENSION_UUID, ExtensionManifest, ExtensionStagingMarker (+47 more)

### Community 6 - "KeydController"
Cohesion: 0.22
Nodes (4): KeydController, KeydError, NeverDispatch, valid_id()

### Community 7 - "rust_sensitivity.rs"
Cohesion: 0.17
Nodes (21): cancelled_newer_restore_does_not_make_older_token_usable(), EYE_REPLY, eye_resize_failure_propagates_without_sensitivity_write(), f6_restores_exact_baseline_even_when_resize_fails(), f7_changes_sensitivity_only_after_exact_resize(), failed_eye_after_write_restores_while_focus_remains_effective(), failed_eye_and_failed_recovery_report_both_and_remain_pending(), failed_preflight_invalidates_generation_until_restore() (+13 more)

### Community 8 - "setup_helper.rs"
Cohesion: 0.06
Nodes (61): HELPER_PATH, KEYD_POLICY_PATH, MACCEL_POLICY_PATH, SERVICE_PATH, abort_and_eof_roll_back_only_new_files(), check_components(), Decision, Abort (+53 more)

### Community 9 - "InstallError"
Cohesion: 0.05
Nodes (14): ExtensionAction, Disable, Enable, InstallError, System, InstallGuard, RootSetup, System (+6 more)

### Community 10 - "rust_apply.rs"
Cohesion: 0.13
Nodes (56): applies_two_closed_instances_and_restores_exact_bytes(), apply_publication_runs_under_marker_and_failure_keeps_marker(), backup_is_bound_to_launcher_root(), before_restore_runs_after_validation_and_before_marker(), changed_standard_file_rejects_restore_before_any_mutation(), dangling_recovery_record_symlinks_block_apply(), enter_chat_preserves_hotbar_through_apply_and_restore(), existing_mutation_lock_blocks_another_apply() (+48 more)

### Community 11 - "install_prerequisites.rs"
Cohesion: 0.06
Nodes (54): array_entries(), BACKUP_NAME, change_options(), declined_membership_keeps_observe_only_available(), declined_package_is_actionable_and_does_not_mutate(), doctor_reports_distinct_live_issues_without_mutation(), existing_fkey_option_refresh_failure_restores_current_settings(), existing_fkey_option_refreshes_current_sources_without_backup() (+46 more)

### Community 12 - "extension_harness.cjs"
Cohesion: 0.03
Nodes (36): assert, beforeDrain, beforeReconfigure, bindings, callbacks, centeredReply, centeredWindow, context (+28 more)

### Community 13 - "app.js"
Cohesion: 0.06
Nodes (47): action(), addMappingRow(), addPresetRow(), api(), attachRecorder(), canApply(), changed(), controlPicker() (+39 more)

### Community 14 - "RealSystem"
Cohesion: 0.09
Nodes (24): Artifact, authenticated_status_command_has_a_deadline(), check_uninstall_records(), existing_policy(), failed_commands_report_exit_status_and_both_output_streams(), install(), install_inner(), Refused (+16 more)

### Community 15 - "GnomeError"
Cohesion: 0.06
Nodes (17): is_minecraft_class(), bounded(), bus_error(), canonical_shortcut(), GnomeBridge<P>, GnomeError, GnomePeer, Peer (+9 more)

### Community 16 - "rust_profile.rs"
Cohesion: 0.10
Nodes (27): ControlPlan, decode(), encode(), keyd_to_minecraft(), ManagedState, MappingError, minecraft_to_keyd(), plan_controls() (+19 more)

### Community 17 - "runner.py"
Cohesion: 0.06
Nodes (11): main(), run(), is_mouse_button(), Key, diff(), KeyStream, run_test(), printerr() (+3 more)

### Community 18 - "install_inner"
Cohesion: 0.15
Nodes (11): enable_extension_checked(), extension_enabled_from_output(), extension_named_in_output(), extension_state_from_info(), ExtensionState, Active, Error, Inactive (+3 more)

### Community 19 - "api.rs"
Cohesion: 0.11
Nodes (16): ApiConfig, ApiResult, ApiState, asset(), asset_at(), constant_time_eq(), gate(), get_profile() (+8 more)

### Community 20 - "focus.rs"
Cohesion: 0.06
Nodes (15): FocusEvent, FocusListener, FocusState, FocusTransition, Gained, Lost, Unchanged, HEARTBEAT_TIMEOUT (+7 more)

### Community 21 - "McsrToolkitExtension"
Cohesion: 0.08
Nodes (13): Global Constraints, Managed Panel Visibility Implementation Plan, Preparation, Review Focus, Self-review, Task 1: Panel ownership, lifecycle, and regressions, Task 2: Documentation, combined verification, and review, McsrToolkitExtension (+5 more)

### Community 22 - "RuntimeAdapter"
Cohesion: 0.06
Nodes (6): BlockingRuntime, FakeDownload, NativeKeyd, RejectPublicationRuntime, Runtime, TrackingRuntime

### Community 24 - "rust_runtime.rs"
Cohesion: 0.16
Nodes (33): absent_optional_maccel_serves_observe_only_without_helper_calls(), clean_shutdown_releases_the_service_lock(), config(), duplicate_runtime_refuses_same_session_lock(), extension_connection_survives_a_non_minecraft_heartbeat(), failed_eye_recovery_still_clears_previous_keyd_bindings(), failed_keyd_discovery_keeps_fallback_vocabulary(), failed_startup_restore_prevents_eye_acceptance_and_http_binding() (+25 more)

### Community 25 - "downloads.rs"
Cohesion: 0.16
Nodes (11): download_release(), Io, DownloadResult, HttpClient, HttpResponse, MAX_BYTES, MAX_METADATA_BYTES, MAX_REDIRECTS (+3 more)

### Community 26 - ".apply_inner"
Cohesion: 0.20
Nodes (10): ApplyCoordinator, ApplyError, err(), KeydAdapter, mod_kind(), record_present(), T, ensure_state() (+2 more)

### Community 27 - "DownloadError"
Cohesion: 0.18
Nodes (9): DownloadError, Download, InvalidAssetUrl, InvalidDestination, InvalidJar, InvalidMetadata, InvalidRedirect, TooLarge (+1 more)

### Community 28 - "runtime.rs"
Cohesion: 0.10
Nodes (8): NativeDownloads, process_cwds_under(), publish_active_preset(), reconcile_cycle(), reconciliation_unsafe(), ServiceLock, State, visible_active_preset()

### Community 29 - "rust_instances.rs"
Cohesion: 0.09
Nodes (22): discover_instances(), DiscoveryError, Instance, launcher_root(), OptionsError, parse_options(), render_windowed_options(), windowed_preserves_lines_and_never_enters_bindings() (+14 more)

### Community 30 - "CommandRunner"
Cohesion: 0.29
Nodes (3): CommandRunner, malformed_restore_payload_releases_pre_dispatch_marker(), private_dir_preflight_failure_releases_owned_marker()

### Community 31 - "Wayland integration acceptance"
Cohesion: 0.08
Nodes (25): 2026-10-05 browser UI acceptance (Task 9), 2026-10-05 keyd helper recovery check, 2026-10-05 supervised authentication input check, 2026-10-05 supervised live focus and chat check, 2026-10-06 approved helper deployment and current state, 2026-10-06 approved movement-test attempt, 2026-10-06 exact observe-only F6 resize acceptance, 2026-10-06 supervised movement test, second attempt (+17 more)

### Community 32 - "rust_gnome.rs"
Cohesion: 0.12
Nodes (11): bridge_stale_check_leaves_expiry_transition_for_runtime_watch(), captured_special_keys_route_through_internal_gnome_accelerators(), failed_configuration_invalidates_previous_signal_generation(), FakePeer, identical_profile_reconnect_rejects_old_activation_signal(), nonreplying_configuration_times_out(), preset_signal_requires_live_fifo_heartbeat_even_when_bus_reports_minecraft(), profile_configuration_uses_extension_json_shape() (+3 more)

### Community 33 - "Product"
Cohesion: 0.24
Nodes (7): Lookup, Product, Launcher, Ninjabrain, ReleaseAsset, resolve_release(), valid_asset_url()

### Community 34 - "with"
Cohesion: 0.10
Nodes (19): jobs, verify, name, on, pull_request, push, workflow_dispatch, permissions (+11 more)

### Community 35 - "apply.rs"
Cohesion: 0.15
Nodes (10): BackupFile, ControlMapping, History, Manifest, MutationLock, new_id(), NEXT_BACKUP, Prepared (+2 more)

### Community 36 - "ApiState"
Cohesion: 0.12
Nodes (9): ApiError, apply(), download(), Job, JobQueue, QueueInner, report_json(), restore() (+1 more)

### Community 37 - "rust_api.rs"
Cohesion: 0.17
Nodes (18): router(), state_dir(), completed_backup_id(), completed_job(), download_allowlist_and_unknown_job(), embedded_assets_session_and_status_have_ui_contract(), failed_runtime_apply_publication_keeps_marker_and_notifies_runtime(), failed_runtime_restore_publication_keeps_marker_and_notifies_runtime() (+10 more)

### Community 38 - "paths.rs"
Cohesion: 0.11
Nodes (13): maccel_binary(), main(), run(), atomic_replace(), directory_fd(), existing_permissive_parent_is_tightened_without_touching_records_or_data_home(), NEXT_TEMP, prepare_private_state() (+5 more)

### Community 39 - "keyboard.c"
Cohesion: 0.17
Nodes (29): abort_chord(), activate_layer(), cache_get(), cache_set(), calculate_main_loop_timeout(), check_chord_match(), chord_event_match(), clear_mod() (+21 more)

### Community 40 - "rust_assets.rs"
Cohesion: 0.15
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
Cohesion: 0.12
Nodes (15): accelerator(), DESTINATION, FOCUS_TIMEOUT, Helper, INTERFACE, is_function_key(), METHOD_TIMEOUT, OBJECT_PATH (+7 more)

### Community 47 - "RealRootSetup"
Cohesion: 0.19
Nodes (7): abandoned_setup_terminates_privileged_process_group(), authenticated_setup_eof_reports_helper_stderr_and_exit_status(), authenticated_setup_stderr_is_truncated_without_pipe_blocking(), classify_root_commit_response(), RootAborted, RealRootSetup, start_root_transaction()

### Community 48 - "GNOME Wayland focus probe"
Cohesion: 0.17
Nodes (12): Chat focus risk, Failed chat safety test and cleanup, Failed supervised chat-only probe, 2026-10-04, GNOME Wayland focus probe, Initial environment at first inspection (2026-10-03), Observed after GNOME extension activation, Production observe-only service, 2026-10-04, Restart-helper recovery check, 2026-10-05 (+4 more)

### Community 49 - "rust_keyd.rs"
Cohesion: 0.13
Nodes (27): activation_requires_effective_focus_and_no_mutation(), clear_refuses_privileged_call_without_private_durable_state(), clear_refuses_untrusted_helper_after_reset(), clear_resets_before_helper_and_blocks_activation_after_failure(), concurrent_controllers_cannot_share_clear_marker_or_activate_early(), concurrent_install_cannot_start_a_second_privileged_install(), concurrent_restore_blocks_other_controller_clear_and_activation(), delayed_install_keeps_both_snapshot_ids_and_restores_latest() (+19 more)

### Community 50 - "keyd.rs"
Cohesion: 0.11
Nodes (16): Backups, child_that_does_not_read_stdin_is_killed_without_blocking_writer(), CommandFailure, Failed, TimedOut, CommandOutput, hung_child_is_killed_before_deadline_expires_twice(), HELPER (+8 more)

### Community 51 - "build.rs"
Cohesion: 0.21
Nodes (6): build_managed_keyd(), copy_tree(), EXTENSION_FILES, main(), verify_archive(), write_archive()

### Community 52 - "Review Focus"
Cohesion: 0.25
Nodes (7): Global Constraints, Guided Live Installer Implementation Plan, Review Focus, Task 1: Authenticated helper diagnostics, Task 2: Guided prerequisites and read-only live checks, Task 3: Live-default transaction and fallback, Verification record

### Community 53 - ".start_with_adapters_inner"
Cohesion: 0.19
Nodes (10): DownloadAdapter, apply_signal(), configure_routes(), discover_keys(), err(), load_profile(), RuntimeConfig, RuntimeError (+2 more)

### Community 54 - "Rust single-binary release checklist"
Cohesion: 0.13
Nodes (15): 2026-10-26 Enter dual-role run, Candidate and reproducible checks, Historical v0.1.0-beta.1 preparation record, Live scope and readiness limits, Publication verification, Release decision, Remaining live gates, Rust single-binary release checklist (+7 more)

### Community 55 - "device.c"
Cohesion: 0.12
Nodes (18): device_grab(), device_init(), device_read_event(), device_scan(), device_scan_worker(), device_ungrab(), devmon_create(), devmon_read_device() (+10 more)

### Community 56 - "Borderless Minecraft Resize Implementation Plan"
Cohesion: 0.20
Nodes (8): Approval handoff, Borderless Minecraft Resize Implementation Plan, Global Constraints, Research references, Review Focus, Task 1: Prepare decorated XWayland Minecraft windows asynchronously, Task 2: Apply exact dimensions and close every request lifecycle, Task 3: Verify service integration, packaged assets, and user guidance

### Community 57 - "rust_standard_settings.rs"
Cohesion: 0.28
Nodes (12): prepare_local(), bindings(), legacy_preserves_endings_blank_and_unrelated_values(), missing_and_unchanged_configs_are_handled_without_writes(), put(), rejects_legacy_redirect_chains_without_opening_shared_files(), rejects_malformed_and_duplicate_controls(), rejects_shared_redirects_without_touching_targets() (+4 more)

### Community 58 - "Binding input capture implementation plan"
Cohesion: 0.25
Nodes (7): Binding input capture implementation plan, Execution notes, Global constraints, Review focus, Task 1: Supported runtime names and Minecraft mouse mappings, Task 2: Shared recorder and field integration, Task 3: Verification and review

### Community 59 - "Minecraft Control Picker Implementation Plan"
Cohesion: 0.25
Nodes (7): Execution notes, Global Constraints, Integration recheck, Minecraft Control Picker Implementation Plan, Review Focus, Task 1: Searchable picker and profile integration, Verification results

### Community 60 - "maccel.rs"
Cohesion: 0.08
Nodes (12): COMMAND_TIMEOUT, empty_eye_probe_proves_noninteractive_permission_without_a_token(), EYE_REPLY, InstalledMaccelPeer, MaccelPeer, HELPER, SensitivityController<H>, SensitivityError (+4 more)

### Community 61 - "Key capture for binding fields"
Cohesion: 0.29
Nodes (6): Approaches, Behavior, Key capture for binding fields, Scope, Validation, Verified runtime constraints

### Community 62 - "Borderless Minecraft resize design"
Cohesion: 0.20
Nodes (10): Approaches, Borderless Minecraft resize design, Corrective fullscreen boundary after live regression, Current behavior, Platform boundary, References, Requested behavior, Resize lifecycle and failures (+2 more)

### Community 63 - "MCSR Toolkit"
Cohesion: 0.22
Nodes (9): Build and deterministic checks, Live keyd mode and recovery, Managed keyd and F3 priority, MCSR Toolkit, Repair and uninstall, Resize presets, Run and use the UI, Rust executable and release status (+1 more)

### Community 65 - "v0.1.0 stable candidate — 2026-10-29"
Cohesion: 0.33
Nodes (6): Candidate verification, Preparation checklist, Publication verification, Remaining live limitations, Scope and source identity, v0.1.0 stable candidate — 2026-10-29

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

### Community 73 - "rust_runtime_process.rs"
Cohesion: 0.11
Nodes (6): active_python_style_service_lock_blocks_rust_start(), assert_restart_with_retained_writer(), fake_service_child(), FakeMaccel, KillOnDrop, signal_restart_keeps_writer_drains_stale_fifo_and_restores_before_new_eye()

### Community 80 - "v0.1.0-beta.3 candidate — 2026-10-29"
Cohesion: 0.33
Nodes (6): Candidate verification, Preparation checklist, Publication verification, Remaining live gates, Scope and evidence, v0.1.0-beta.3 candidate — 2026-10-29

### Community 81 - "keyd_helper.rs"
Cohesion: 0.15
Nodes (17): apply_and_restart(), apply_snapshot(), atomic_write(), DEVICE, DEVICE_NAME, FencedRequest, FRAGMENT_NAME, Manifest (+9 more)

### Community 82 - "parse_profile"
Cohesion: 0.16
Nodes (12): ChatKeys, deserialize_bool(), deserialize_dimension(), key(), KeySet, modifier_family(), parse_dimension_string(), parse_profile() (+4 more)

### Community 83 - "Guided installation with live keyd by default"
Cohesion: 0.40
Nodes (4): Approaches, Behavior, Boundaries, Guided installation with live keyd by default

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
Cohesion: 0.23
Nodes (9): current(), dispatch(), HelperFilesystem, load_manifest(), recovery_pending(), save_manifest(), select_keyd_client_with(), status_under_lock() (+1 more)

### Community 94 - "keyd.c"
Cohesion: 0.20
Nodes (12): chgid(), ipc_connect(), ipc_create_server(), add_bindings(), cmd_do(), help(), input(), ipc_exec() (+4 more)

### Community 95 - "MCSR Toolkit Rust single-binary design"
Cohesion: 0.12
Nodes (13): Context and goal, MCSR Toolkit maccel sensitivity recovery design, Privilege boundary, Service behavior and failure recovery, Sources, Verification and acceptance, Cutover, state, and failure handling, Executable and component boundaries (+5 more)

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
Cohesion: 0.11
Nodes (21): File map, Global Constraints, Keyd Restart Recovery Implementation Plan, Review Focus, Task 1: Replace privileged reload with bounded restart and rollback, Task 2: Wire the exact clear command and deployment preflight, Task 3: Persist and serialize focus suspension, Task 4: Prove live behavior and empty-config rollback before UI (+13 more)

### Community 100 - "Review Focus"
Cohesion: 0.22
Nodes (8): Execution ledger, F3 Priority and Managed Keyd Implementation Plan, Global Constraints, Review Focus, Task 1: Reproducible managed keyd engine and embedded asset, Task 2: Fixed-target install, selection, and restoration, Task 3: Compile shared routing and coordinate applied generation, Task 4: Review and user-facing validation record

### Community 101 - "Borderless full-size toggle live acceptance, 2026-10-30"
Cohesion: 0.33
Nodes (6): Borderless full-size toggle live acceptance, 2026-10-30, Chat, release, and ordinary-input results, F3 priority results, Observed windowed transitions and focus recovery, Post-login repair and preparation, Verified cleanup and final scope

### Community 102 - "installed_live_mode"
Cohesion: 0.47
Nodes (5): installed_live_mode(), observe_unit(), unit(), verify_resolved_command(), verify_running_service()

### Community 103 - "managed_keyd.rs"
Cohesion: 0.29
Nodes (5): BINARY_PATH, CAPABILITIES, DROPIN, DROPIN_PATH, UPSTREAM_REVISION

### Community 104 - "Managed panel visibility live test, 2026-10-30"
Cohesion: 0.33
Nodes (6): First panel check, Focus and modal observations, Managed panel visibility live test, 2026-10-30, Post-login preparation, Staged for login, Verified cleanup

### Community 105 - "Profile"
Cohesion: 0.22
Nodes (8): ApiBackend, NativeBackend, ApplyPreview, ApplyReport, InstanceResult, Profile, FakeBackend, report()

### Community 106 - "HelperSystem"
Cohesion: 0.16
Nodes (6): HelperSystem, production_readiness_requires_active_service_and_complete_ipc_capabilities(), production_readiness_shares_command_budget_and_rejects_overrun(), readiness_with(), RealSystem, restart_healthy()

### Community 107 - "Borderless Full-Size Toggle Implementation Plan"
Cohesion: 0.17
Nodes (11): Borderless Full-Size Toggle Implementation Plan, Global Constraints, Parallel execution ledger, Preparation, Review Focus, Self-review, Shared Interfaces and Dependencies, Task 1: Extension geometry toggle (+3 more)

### Community 111 - "MCSR Toolkit v0.2.0"
Cohesion: 0.40
Nodes (5): Candidate identity and publication boundary, Changes since v0.1.0, Download, installation and upgrade, Live evidence and known limitations, MCSR Toolkit v0.2.0

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

### Community 116 - "ServiceRuntime"
Cohesion: 0.28
Nodes (5): SharedFocus, GnomeBridge, PresetController, SensitivityController, ServiceRuntime

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

### Community 123 - "MaccelAvailability"
Cohesion: 0.40
Nodes (5): MaccelAvailability, Available, RecoveryUncertain, Unavailable, optional_maccel_availability()

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
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 817 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **16 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `clear()` connect `clear` to `keyboard.c`, `File map and task boundaries`, `Managed keyd live validation, 2026-10-29`, `GNOME Wayland focus probe`, `Review Focus`, `MCSR Toolkit Rust single-binary design`, `Wayland integration acceptance`?**
  _High betweenness centrality (0.243) - this node is a cross-community bridge._
- **What connects `mcsr-toolkit`, `Launcher`, `Ninjabrain` to the rest of the system?**
  _45 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `rust_install.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05411823522059742 - nodes in this community are weakly interconnected._
- **Why does `Task 1: Replace privileged reload with bounded restart and rollback` connect `clear` to `app.js`?**
  _High betweenness centrality (0.111) - this node is a cross-community bridge._
- **Should `maccel_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.055412371134020616 - nodes in this community are weakly interconnected._
- **Should `rust_keyd_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.11948790896159317 - nodes in this community are weakly interconnected._
- **Should `live-ui-acceptance.mjs` be split into smaller, more focused modules?**
  _Cohesion score 0.056842105263157895 - nodes in this community are weakly interconnected._