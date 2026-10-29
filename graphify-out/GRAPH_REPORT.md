# Graph Report - feat-standard-settings  (2026-10-29)

## Corpus Check
- 83 files · ~113,009 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 11 file(s) not represented in the graph (top: (none) 3, .conf 2, .sudoers 2)

## Summary
- 2077 nodes · 5328 edges · 96 communities (78 shown, 18 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 194 edges (avg confidence: 0.82)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c54ae8f1`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- rust_install.rs
- maccel_helper.rs
- keyd_helper.rs
- live-ui-acceptance.mjs
- rust_downloads.rs
- install.rs
- KeydController
- rust_sensitivity.rs
- setup_helper.rs
- InstallError
- rust_apply.rs
- System
- extension_harness.cjs
- app.js
- Refused
- GnomeError
- parse_profile
- MaccelAvailability
- install_inner
- api.rs
- rust_keyd.rs
- McsrToolkitExtension
- TrackingRuntime
- SensitivityError
- rust_runtime.rs
- Profile
- .apply_inner
- rust_runtime_process.rs
- State
- rust_instances.rs
- .start_with_adapters_inner
- Wayland integration acceptance
- rust_gnome.rs
- runtime.rs
- with
- apply.rs
- apply
- rust_api.rs
- paths.rs
- Environment
- rust_assets.rs
- File map and task boundaries
- Enter Opens and Sends Chat Implementation Plan
- Review Focus
- ResizeError
- standard_settings.rs
- gnome.rs
- RealRootSetup
- GNOME Wayland focus probe
- main
- Node
- pathbuf
- RuntimeAdapter
- FocusState
- Rust single-binary release checklist
- fail
- Borderless Minecraft Resize Implementation Plan
- SensitivityController<H>
- Binding input capture implementation plan
- Minecraft Control Picker Implementation Plan
- ui_harness.cjs
- Key capture for binding fields
- Borderless Minecraft resize design
- MCSR Toolkit
- MCSR Toolkit v0.1.0-beta.3
- CDP
- Uninstall keyd recovery implementation plan
- BlockingDownload
- keyd.rs
- v0.1.0-beta.2 Release Preparation Plan
- Minecraft control picker
- GNOME Wayland resize and sensitivity acceptance
- maccel.rs
- AGENTS.md
- keyd-focus-probe.sh script
- mcsr-toolkit
- http_probe
- Review Focus
- parse_profile
- Enter opens and sends chat
- CommandRunner
- ServiceAction
- cleanupProfile
- cleanLineage
- Parallel execution ledger
- Standard Settings Implementation Plan
- Standard Settings integration
- .listen_presets
- DownloadAdapter

## God Nodes (most connected - your core abstractions)
1. `InstallError` - 158 edges
2. `System` - 86 edges
3. `Profile` - 65 edges
4. `install_inner()` - 56 edges
5. `Fake` - 53 edges
6. `RealSystem` - 45 edges
7. `GnomeError` - 38 edges
8. `original()` - 37 edges
9. `Refused` - 36 edges
10. `uninstall()` - 35 edges

## Surprising Connections (you probably didn't know these)
- `Release decision` --references--> `main()`  [INFERRED]
  docs/rust-release-checklist.md → integration/live-ui-acceptance.mjs
- `Task 1: Implement the isolated root helper` --references--> `main()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-maccel-sensitivity-recovery.md → integration/live-ui-acceptance.mjs
- `Task 1: Rust artifact and embedded assets` --references--> `main()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-rust-single-binary.md → integration/live-ui-acceptance.mjs
- `Context and goal` --references--> `get()`  [INFERRED]
  docs/superpowers/specs/2026-10-06-maccel-sensitivity-recovery-design.md → integration/live-ui-acceptance.mjs
- `Preparation checklist` --references--> `main()`  [INFERRED]
  docs/rust-release-checklist.md → integration/live-ui-acceptance.mjs

## Import Cycles
- 1-file cycle: `src/keyd.rs -> src/keyd.rs`
- 2-file cycle: `src/mapping.rs -> src/options.rs -> src/mapping.rs`

## Communities (96 total, 18 thin omitted)

### Community 0 - "rust_install.rs"
Cohesion: 0.06
Nodes (88): repair(), aborted_root_commit_with_wait_error_still_restores_user_integration(), active_legacy_extension_hands_off_before_root_setup_and_remains_on_disk(), changed_extension_root_failure_preserves_shell_marker_and_old_lineage_for_retry(), changed_owned_extension_with_verified_lineage_waits_for_new_shell(), completed_manifest_write_with_shell_marker_recovers_after_interrupted_root_setup(), conflicting_prior_enabled_uuids_are_not_silently_claimed_restored(), corrupted_staged_helper_never_publishes_passwordless_policy() (+80 more)

### Community 1 - "maccel_helper.rs"
Cohesion: 0.06
Nodes (59): c_name(), dispatch(), EYE_DECIMAL, EYE_RAW, file_name(), FINGERPRINT, held_uninstall_operation_lock_blocks_concurrent_maccel_action(), HelperPaths (+51 more)

### Community 2 - "keyd_helper.rs"
Cohesion: 0.05
Nodes (53): apply_and_restart(), apply_snapshot(), atomic_write(), current(), DEVICE, DEVICE_NAME, DirectorySync, dispatch() (+45 more)

### Community 3 - "live-ui-acceptance.mjs"
Cohesion: 0.11
Nodes (19): allowed, baselineSha, baseUrl, browser(), childExited(), closeTemporaryBrowser(), flags, instanceId (+11 more)

### Community 4 - "rust_downloads.rs"
Cohesion: 0.06
Nodes (48): download_release(), DownloadError, Download, InvalidAssetUrl, InvalidDestination, InvalidJar, InvalidMetadata, InvalidRedirect (+40 more)

### Community 5 - "install.rs"
Cohesion: 0.06
Nodes (43): authenticated_status_command_has_a_deadline(), both_protected_recovery_histories_use_one_authenticated_status(), extension_digest(), extension_digest_from_fingerprints(), EXTENSION_UUID, ExtensionManifest, ExtensionStagingMarker, failed_commands_report_exit_status_and_both_output_streams() (+35 more)

### Community 6 - "KeydController"
Cohesion: 0.22
Nodes (4): KeydController, KeydError, NeverDispatch, valid_id()

### Community 7 - "rust_sensitivity.rs"
Cohesion: 0.18
Nodes (19): cancelled_newer_restore_does_not_make_older_token_usable(), EYE_REPLY, eye_resize_failure_propagates_without_sensitivity_write(), f6_restores_exact_baseline_even_when_resize_fails(), f7_changes_sensitivity_only_after_exact_resize(), failed_eye_after_write_restores_while_focus_remains_effective(), failed_eye_and_failed_recovery_report_both_and_remain_pending(), failed_preflight_invalidates_generation_until_restore() (+11 more)

### Community 8 - "setup_helper.rs"
Cohesion: 0.10
Nodes (39): HELPER_PATH, KEYD_POLICY_PATH, MACCEL_POLICY_PATH, SERVICE_PATH, abort_and_eof_roll_back_only_new_files(), check_components(), Decision, Abort (+31 more)

### Community 9 - "InstallError"
Cohesion: 0.08
Nodes (10): ExtensionAction, Disable, Enable, InstallError, System, InstallGuard, RootSetup, Fake (+2 more)

### Community 10 - "rust_apply.rs"
Cohesion: 0.14
Nodes (48): applies_two_closed_instances_and_restores_exact_bytes(), apply_publication_runs_under_marker_and_failure_keeps_marker(), backup_is_bound_to_launcher_root(), before_restore_runs_after_validation_and_before_marker(), changed_standard_file_rejects_restore_before_any_mutation(), dangling_recovery_record_symlinks_block_apply(), enter_chat_preserves_hotbar_through_apply_and_restore(), existing_mutation_lock_blocks_another_apply() (+40 more)

### Community 11 - "System"
Cohesion: 0.08
Nodes (10): enabled_catalog_probe_error_is_not_classified_as_disabled(), ExtensionState, Active, Error, Inactive, marker_parent_sync_failure_is_returned_before_root_setup(), restore_user(), sync_parent() (+2 more)

### Community 12 - "extension_harness.cjs"
Cohesion: 0.05
Nodes (28): assert, bindings, callbacks, centeredReply, centeredWindow, context, disabling, emitted (+20 more)

### Community 13 - "app.js"
Cohesion: 0.05
Nodes (63): File map, Global Constraints, MCSR Toolkit Implementation Plan, Review Focus, Source notes for implementers, Task 10: Complete documentation and end-to-end verification, Task 1: Prove the GNOME Wayland integration path, Task 2: Define the profile and validation contract (+55 more)

### Community 14 - "Refused"
Cohesion: 0.12
Nodes (15): Artifact, cached_policy_bytes_match(), check_uninstall_records(), existing_policy(), file_state_from_open(), Refused, marker_read_uses_opened_inode_when_path_is_replaced(), open_file_nofollow() (+7 more)

### Community 15 - "GnomeError"
Cohesion: 0.11
Nodes (10): is_minecraft_class(), bounded(), GnomeBridge<P>, GnomeError, GnomePeer, SessionPeer, BlockingGnome, FakeGnome (+2 more)

### Community 16 - "parse_profile"
Cohesion: 0.12
Nodes (24): ControlPlan, decode(), encode(), keyd_to_minecraft(), ManagedState, MappingError, minecraft_to_keyd(), plan_controls() (+16 more)

### Community 17 - "MaccelAvailability"
Cohesion: 0.29
Nodes (6): MaccelAvailability, Available, RecoveryUncertain, Unavailable, optional_maccel_availability(), ServiceRuntime<SessionPeer, InstalledMaccelPeer>

### Community 18 - "install_inner"
Cohesion: 0.11
Nodes (17): enable_extension_checked(), extension_enabled_from_output(), extension_named_in_output(), extension_state_from_info(), install_inner(), InstallReport, policy(), preflight() (+9 more)

### Community 19 - "api.rs"
Cohesion: 0.10
Nodes (9): asset(), asset_at(), constant_time_eq(), gate(), index(), Job, JobQueue, QueueInner (+1 more)

### Community 20 - "rust_keyd.rs"
Cohesion: 0.14
Nodes (27): activation_requires_effective_focus_and_no_mutation(), clear_refuses_privileged_call_without_private_durable_state(), clear_refuses_untrusted_helper_after_reset(), clear_resets_before_helper_and_blocks_activation_after_failure(), concurrent_controllers_cannot_share_clear_marker_or_activate_early(), concurrent_install_cannot_start_a_second_privileged_install(), concurrent_restore_blocks_other_controller_clear_and_activation(), delayed_install_keeps_both_snapshot_ids_and_restores_latest() (+19 more)

### Community 21 - "McsrToolkitExtension"
Cohesion: 0.12
Nodes (5): McsrToolkitExtension, minecraftWindow(), focusedWindow(), isMinecraft(), ProbeExtension

### Community 22 - "TrackingRuntime"
Cohesion: 0.08
Nodes (4): BlockingRuntime, NativeKeyd, RejectPublicationRuntime, TrackingRuntime

### Community 23 - "SensitivityError"
Cohesion: 0.18
Nodes (4): InstalledMaccelPeer, MaccelPeer, SensitivityError, FakeMaccel

### Community 24 - "rust_runtime.rs"
Cohesion: 0.18
Nodes (29): absent_optional_maccel_serves_observe_only_without_helper_calls(), clean_shutdown_releases_the_service_lock(), config(), duplicate_runtime_refuses_same_session_lock(), extension_connection_survives_a_non_minecraft_heartbeat(), failed_eye_recovery_still_clears_previous_keyd_bindings(), failed_keyd_discovery_keeps_fallback_vocabulary(), failed_startup_restore_prevents_eye_acceptance_and_http_binding() (+21 more)

### Community 25 - "Profile"
Cohesion: 0.25
Nodes (8): ApiBackend, NativeBackend, ApplyPreview, ApplyReport, InstanceResult, Profile, FakeBackend, report()

### Community 26 - ".apply_inner"
Cohesion: 0.21
Nodes (11): ApplyCoordinator, ApplyError, err(), KeydAdapter, mod_kind(), record_present(), valid_id(), T (+3 more)

### Community 27 - "rust_runtime_process.rs"
Cohesion: 0.18
Nodes (6): active_python_style_service_lock_blocks_rust_start(), assert_restart_with_retained_writer(), fake_service_child(), FakeMaccel, KillOnDrop, signal_restart_keeps_writer_drains_stale_fifo_and_restores_before_new_eye()

### Community 28 - "State"
Cohesion: 0.11
Nodes (7): NativeDownloads, process_cwds_under(), publish_active_preset(), reconcile_cycle(), reconciliation_unsafe(), State, visible_active_preset()

### Community 29 - "rust_instances.rs"
Cohesion: 0.15
Nodes (17): discover_instances(), OptionsError, parse_options(), discovery_excludes_missing_malformed_and_mismatched_instance_json(), discovery_finds_every_valid_instance_and_legacy_game_dir(), discovery_rejects_symlink_escape_and_unsafe_launcher_root(), discovery_reports_dangling_game_symlink_as_unsafe(), instance() (+9 more)

### Community 30 - ".start_with_adapters_inner"
Cohesion: 0.23
Nodes (9): ResizeResult, apply_signal(), err(), load_profile(), RuntimeConfig, RuntimeError, ServiceLock, ServiceRuntime (+1 more)

### Community 31 - "Wayland integration acceptance"
Cohesion: 0.08
Nodes (24): 2026-10-05 browser UI acceptance (Task 9), 2026-10-05 keyd helper recovery check, 2026-10-05 supervised authentication input check, 2026-10-05 supervised live focus and chat check, 2026-10-06 approved helper deployment and current state, 2026-10-06 approved movement-test attempt, 2026-10-06 exact observe-only F6 resize acceptance, 2026-10-06 supervised movement test, second attempt (+16 more)

### Community 32 - "rust_gnome.rs"
Cohesion: 0.07
Nodes (14): blur_and_quick_refocus_cannot_publish_stale_preset(), event(), graceful_restart_preserves_writer_and_discards_downtime_focus(), heartbeat_timeout_clears_focus(), modal_and_disconnect_clear_focus(), bridge_stale_check_leaves_expiry_transition_for_runtime_watch(), captured_special_keys_become_gnome_accelerators(), FakePeer (+6 more)

### Community 33 - "runtime.rs"
Cohesion: 0.10
Nodes (3): FocusListener, HEARTBEAT_TIMEOUT, validate_fifo()

### Community 34 - "with"
Cohesion: 0.10
Nodes (19): jobs, verify, name, on, pull_request, push, workflow_dispatch, permissions (+11 more)

### Community 35 - "apply.rs"
Cohesion: 0.12
Nodes (13): BackupFile, ControlMapping, History, Manifest, MutationLock, new_id(), NEXT_BACKUP, Prepared (+5 more)

### Community 36 - "apply"
Cohesion: 0.20
Nodes (14): ApiError, ApiResult, ApiState, apply(), download(), get_profile(), job(), preview() (+6 more)

### Community 37 - "rust_api.rs"
Cohesion: 0.18
Nodes (17): router(), state_dir(), completed_backup_id(), completed_job(), download_allowlist_and_unknown_job(), embedded_assets_session_and_status_have_ui_contract(), failed_runtime_apply_publication_keeps_marker_and_notifies_runtime(), failed_runtime_restore_publication_keeps_marker_and_notifies_runtime() (+9 more)

### Community 38 - "paths.rs"
Cohesion: 0.21
Nodes (7): directory_fd(), existing_permissive_parent_is_tightened_without_touching_records_or_data_home(), NEXT_TEMP, prepare_private_state(), prepare_private_state_for_uid(), private_child(), private_state_creation_ignores_permissive_umask_and_runtime_accepts_it()

### Community 40 - "rust_assets.rs"
Cohesion: 0.09
Nodes (9): EXTENSION_FILES, main(), verify_archive(), write_archive(), extension_archive(), web_asset(), app_javascript_is_embedded(), extension_archive_contains_sources_and_compiled_schema() (+1 more)

### Community 41 - "File map and task boundaries"
Cohesion: 0.04
Nodes (44): File map, Global Constraints, Handoff, MCSR Toolkit maccel Sensitivity Recovery Implementation Plan, Execution and approval gates, File map and task boundaries, Global Constraints, Review Focus (+36 more)

### Community 42 - "Enter Opens and Sends Chat Implementation Plan"
Cohesion: 0.22
Nodes (7): Enter Opens and Sends Chat Implementation Plan, Global Constraints, Handoff, Review Focus, Task 1: Profile and mapping contract, Task 2: Browser acceptance contract and documentation, Task 3: Final supervised device check

### Community 43 - "Review Focus"
Cohesion: 0.13
Nodes (13): Global Constraints, Handoff, Review Focus, Rust Source Cutover Implementation Plan, Task 1: Native legacy-lock test fixture, Task 2: Remove Python product and test tree, Task 3: Rust-only active documentation, Task 4: Final deterministic and release-boundary verification (+5 more)

### Community 44 - "ResizeError"
Cohesion: 0.18
Nodes (7): PresetController<P, H>, PresetError, ResizeError, Gnome, Inexact, InvalidSize, NotFocused

### Community 45 - "standard_settings.rs"
Cohesion: 0.12
Nodes (16): fail(), json_edit(), legacy_edit(), local_file(), prepare_local(), StandardSettingsError, Unique, bindings() (+8 more)

### Community 46 - "gnome.rs"
Cohesion: 0.15
Nodes (13): SharedFocus, accelerator(), bus_error(), canonical_shortcut(), DESTINATION, FOCUS_TIMEOUT, GnomeBridge, INTERFACE (+5 more)

### Community 47 - "RealRootSetup"
Cohesion: 0.22
Nodes (4): abandoned_setup_terminates_privileged_process_group(), classify_root_commit_response(), RootAborted, RealRootSetup

### Community 48 - "GNOME Wayland focus probe"
Cohesion: 0.17
Nodes (12): Chat focus risk, Failed chat safety test and cleanup, Failed supervised chat-only probe, 2026-10-04, GNOME Wayland focus probe, Initial environment at first inspection (2026-10-03), Observed after GNOME extension activation, Production observe-only service, 2026-10-04, Restart-helper recovery check, 2026-10-05 (+4 more)

### Community 49 - "main"
Cohesion: 0.22
Nodes (13): Candidate verification, Preparation checklist, Publication verification, Remaining live gates, Scope and evidence, v0.1.0-beta.3 candidate — 2026-10-29, expectedKeydSnapshot(), expectedProfile() (+5 more)

### Community 51 - "pathbuf"
Cohesion: 0.16
Nodes (5): DiscoveryError, launcher_root(), maccel_binary(), main(), run()

### Community 53 - "FocusState"
Cohesion: 0.20
Nodes (7): FocusEvent, FocusState, FocusTransition, Gained, Lost, Unchanged, parse_focus_event()

### Community 54 - "Rust single-binary release checklist"
Cohesion: 0.18
Nodes (11): 2026-10-26 Enter dual-role run, Candidate and reproducible checks, Historical v0.1.0-beta.1 preparation record, Publication verification, Release decision, Remaining live gates, Rust single-binary release checklist, Source cutover (+3 more)

### Community 55 - "fail"
Cohesion: 0.22
Nodes (14): canonical(), checkGameClosed(), checkListeningSocket(), checkServiceIdentity(), checkStateDirectory(), checkStatus(), checkSystem(), clickResponse() (+6 more)

### Community 56 - "Borderless Minecraft Resize Implementation Plan"
Cohesion: 0.20
Nodes (8): Approval handoff, Borderless Minecraft Resize Implementation Plan, Global Constraints, Research references, Review Focus, Task 1: Prepare decorated XWayland Minecraft windows asynchronously, Task 2: Apply exact dimensions and close every request lifecycle, Task 3: Verify service integration, packaged assets, and user guidance

### Community 58 - "Binding input capture implementation plan"
Cohesion: 0.25
Nodes (7): Binding input capture implementation plan, Execution notes, Global constraints, Review focus, Task 1: Supported runtime names and Minecraft mouse mappings, Task 2: Shared recorder and field integration, Task 3: Verification and review

### Community 59 - "Minecraft Control Picker Implementation Plan"
Cohesion: 0.25
Nodes (7): Execution notes, Global Constraints, Integration recheck, Minecraft Control Picker Implementation Plan, Review Focus, Task 1: Searchable picker and profile integration, Verification results

### Community 60 - "ui_harness.cjs"
Cohesion: 0.24
Nodes (7): assert, browser(), fs, profile(), source, tick(), vm

### Community 61 - "Key capture for binding fields"
Cohesion: 0.29
Nodes (6): Approaches, Behavior, Key capture for binding fields, Scope, Validation, Verified runtime constraints

### Community 62 - "Borderless Minecraft resize design"
Cohesion: 0.20
Nodes (10): Approaches, Borderless Minecraft resize design, Corrective fullscreen boundary after live regression, Current behavior, Platform boundary, References, Requested behavior, Resize lifecycle and failures (+2 more)

### Community 63 - "MCSR Toolkit"
Cohesion: 0.25
Nodes (8): Build and deterministic checks, Live keyd mode and recovery, MCSR Toolkit, Repair and uninstall, Resize presets, Run and use the UI, Rust executable and beta status, Verify and install

### Community 65 - "MCSR Toolkit v0.1.0-beta.3"
Cohesion: 0.33
Nodes (6): Beta status and known limitations, Candidate verification and artifact identity, Download and install, Highlights, MCSR Toolkit v0.1.0-beta.3, New in beta.3

### Community 67 - "Uninstall keyd recovery implementation plan"
Cohesion: 0.33
Nodes (5): Review focus, Scope and constraints, Task 1: Uninstall recovery, Task 2: Recovery diagnostics and validation, Uninstall keyd recovery implementation plan

### Community 69 - "keyd.rs"
Cohesion: 0.11
Nodes (13): Backups, child_that_does_not_read_stdin_is_killed_without_blocking_writer(), CommandFailure, Failed, TimedOut, CommandOutput, hung_child_is_killed_before_deadline_expires_twice(), KEYD_FALLBACK (+5 more)

### Community 70 - "v0.1.0-beta.2 Release Preparation Plan"
Cohesion: 0.40
Nodes (4): Constraints, Review Focus, Task 1: Candidate and documentation, v0.1.0-beta.2 Release Preparation Plan

### Community 71 - "Minecraft control picker"
Cohesion: 0.40
Nodes (4): Approaches, Behavior and constraints, Minecraft control picker, Scope and validation

### Community 72 - "GNOME Wayland resize and sensitivity acceptance"
Cohesion: 0.29
Nodes (7): Current Rust observation procedure, Current Rust release gates, Fullscreen restoration regression and recovery (2026-10-29), GNOME Wayland resize and sensitivity acceptance, Historical evidence (2026-10-04–06), Historical sensitivity and helper acceptance (2026-10-22), Supervised borderless primitive check (2026-10-29)

### Community 73 - "maccel.rs"
Cohesion: 0.18
Nodes (7): COMMAND_TIMEOUT, empty_eye_probe_proves_noninteractive_permission_without_a_token(), EYE_REPLY, HELPER, SensitivityController, State, SUDO

### Community 81 - "Review Focus"
Cohesion: 0.29
Nodes (7): Review Focus, Task 1: Implement the isolated root helper, Task 2: Add the unprivileged fixed-command adapter, Task 3: Integrate startup, focus, and shutdown recovery, Task 4: Document, deploy by separate approval, and run live acceptance, Context and goal, get()

### Community 82 - "parse_profile"
Cohesion: 0.14
Nodes (13): ChatKeys, deserialize_bool(), deserialize_dimension(), key(), KeySet, modifier_family(), parse_dimension_string(), parse_profile() (+5 more)

### Community 83 - "Enter opens and sends chat"
Cohesion: 0.29
Nodes (7): Approaches, Behavior and constraints, Enter opens and sends chat, Intent, Limits, Verification and rollout, cancel()

### Community 84 - "CommandRunner"
Cohesion: 0.18
Nodes (5): CommandRunner, malformed_restore_payload_releases_pre_dispatch_marker(), private_dir_preflight_failure_releases_owned_marker(), discover_keys(), FakeKeyd

### Community 85 - "ServiceAction"
Cohesion: 0.29
Nodes (6): ServiceAction, Disable, Enable, Reload, Start, Stop

### Community 86 - "cleanupProfile"
Cohesion: 0.40
Nodes (6): cleanupProfile(), same(), stable(), terminal(), uiApi(), validPreview()

### Community 87 - "cleanLineage"
Cohesion: 0.40
Nodes (5): cleanLineage(), readState(), regularFile(), syncStateDirectory(), writeState()

### Community 89 - "Parallel execution ledger"
Cohesion: 0.29
Nodes (7): Current-machine inspection (2026-10-29), Execution results and outstanding decision, Fullscreen/focus regression correction (2026-10-29), Merge authorization, Parallel execution ledger, Supervised Jingle thin test and centering amendment (2026-10-29), unmaximize()

### Community 90 - "Standard Settings Implementation Plan"
Cohesion: 0.25
Nodes (7): Execution ledger, Global Constraints, Handoff, Review Focus, Standard Settings Implementation Plan, Task 1: Local config discovery and rendering, Task 2: Transactional Apply and Restore integration

### Community 91 - "Standard Settings integration"
Cohesion: 0.29
Nodes (6): Approach, Intent and approved scope, Recovery and compatibility, Standard Settings integration, Upstream evidence, Verification

### Community 93 - "DownloadAdapter"
Cohesion: 0.25
Nodes (3): ApiConfig, DownloadAdapter, FakeDownload

## Knowledge Gaps
- **45 isolated node(s):** `mcsr-toolkit`, `Launcher`, `Ninjabrain`, `InvalidMetadata`, `InvalidAssetUrl` (+40 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 575 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **18 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Profile` connect `Profile` to `runtime.rs`, `apply.rs`, `apply`, `keyd.rs`, `KeydController`, `rust_api.rs`, `rust_apply.rs`, `ResizeError`, `gnome.rs`, `GnomeError`, `parse_profile`, `parse_profile`, `api.rs`, `RuntimeAdapter`, `TrackingRuntime`, `.apply_inner`, `State`, `.start_with_adapters_inner`?**
  _High betweenness centrality (0.055) - this node is a cross-community bridge._
- **What connects `mcsr-toolkit`, `Launcher`, `Ninjabrain` to the rest of the system?**
  _45 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `rust_install.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06161301488404292 - nodes in this community are weakly interconnected._
- **Why does `InstallError` connect `InstallError` to `rust_install.rs`, `install.rs`, `Environment`, `System`, `Refused`, `RealRootSetup`, `http_probe`, `install_inner`, `ServiceAction`?**
  _High betweenness centrality (0.039) - this node is a cross-community bridge._
- **Should `maccel_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05811027224891329 - nodes in this community are weakly interconnected._
- **Why does `parse_profile()` connect `parse_profile` to `runtime.rs`, `apply`, `keyd.rs`, `KeydController`, `rust_apply.rs`, `parse_profile`, `api.rs`, `rust_keyd.rs`, `Profile`, `State`, `.start_with_adapters_inner`?**
  _High betweenness centrality (0.024) - this node is a cross-community bridge._
- **Should `keyd_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05329153605015674 - nodes in this community are weakly interconnected._