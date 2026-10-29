# Graph Report - fix-borderless-resize  (2026-10-29)

## Corpus Check
- 79 files · ~108,308 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 11 file(s) not represented in the graph (top: (none) 3, .conf 2, .sudoers 2)

## Summary
- 2008 nodes · 5112 edges · 94 communities (81 shown, 13 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 183 edges (avg confidence: 0.82)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `700bff9b`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- rust_install.rs
- maccel_helper.rs
- keyd_helper.rs
- live-ui-acceptance.mjs
- rust_downloads.rs
- install.rs
- keyd.rs
- rust_sensitivity.rs
- setup_helper.rs
- InstallError
- rust_apply.rs
- System
- extension_harness.cjs
- app.js
- Refused
- GnomeError
- rust_profile.rs
- runtime.rs
- install_inner
- api.rs
- rust_keyd.rs
- McsrToolkitExtension
- rust_api.rs
- rust_focus.rs
- rust_runtime.rs
- Profile
- .restore_with_hooks
- Review Focus
- State
- rust_instances.rs
- .start_with_adapters_inner
- Wayland integration acceptance
- rust_gnome.rs
- rust_runtime_process.rs
- with
- apply.rs
- MCSR Toolkit Design
- browser
- paths.rs
- Environment
- build.rs
- File map and task boundaries
- Enter Opens and Sends Chat Implementation Plan
- Review Focus
- ResizeError
- profile.rs
- gnome.rs
- RealRootSetup
- GNOME Wayland focus probe
- main
- Node
- pathbuf
- Review Focus
- focus.rs
- Rust single-binary release checklist
- SensitivityError
- Borderless Minecraft Resize Implementation Plan
- bounded
- Binding input capture implementation plan
- Minecraft Control Picker Implementation Plan
- ui_harness.cjs
- Key capture for binding fields
- Borderless Minecraft resize design
- MCSR Toolkit
- MCSR Toolkit v0.1.0-beta.3
- CDP
- Uninstall keyd recovery implementation plan
- MCSR Toolkit maccel Sensitivity Recovery Implementation Plan
- keyd.rs
- v0.1.0-beta.2 Release Preparation Plan
- Minecraft control picker
- GNOME Wayland resize and sensitivity acceptance
- maccel.rs
- AGENTS.md
- keyd-focus-probe.sh script
- mcsr-toolkit
- SensitivityController<H>
- Review Focus
- parse_profile
- Enter opens and sends chat
- MCSR Toolkit Rust single-binary design
- ServiceAction
- cleanupProfile
- cleanLineage
- MCSR Toolkit Implementation Plan
- Parallel execution ledger
- MCSR Toolkit maccel sensitivity recovery design
- Rust Single-Binary Migration Implementation Plan
- .listen_presets
- http_probe

## God Nodes (most connected - your core abstractions)
1. `InstallError` - 158 edges
2. `System` - 86 edges
3. `Profile` - 65 edges
4. `install_inner()` - 56 edges
5. `Fake` - 53 edges
6. `RealSystem` - 45 edges
7. `GnomeError` - 38 edges
8. `Refused` - 36 edges
9. `uninstall()` - 35 edges
10. `KeydController` - 34 edges

## Surprising Connections (you probably didn't know these)
- `Context and goal` --references--> `get()`  [INFERRED]
  docs/superpowers/specs/2026-10-06-maccel-sensitivity-recovery-design.md → integration/live-ui-acceptance.mjs
- `Release decision` --references--> `main()`  [INFERRED]
  docs/rust-release-checklist.md → integration/live-ui-acceptance.mjs
- `Task 1: Implement the isolated root helper` --references--> `main()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-maccel-sensitivity-recovery.md → integration/live-ui-acceptance.mjs
- `Task 1: Rust artifact and embedded assets` --references--> `main()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-rust-single-binary.md → integration/live-ui-acceptance.mjs
- `Preparation checklist` --references--> `main()`  [INFERRED]
  docs/rust-release-checklist.md → integration/live-ui-acceptance.mjs

## Import Cycles
- 1-file cycle: `src/keyd.rs -> src/keyd.rs`
- 2-file cycle: `src/mapping.rs -> src/options.rs -> src/mapping.rs`

## Communities (94 total, 13 thin omitted)

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
Nodes (20): allowed, baselineSha, baseUrl, canonical(), checkGameClosed(), checkListeningSocket(), checkServiceIdentity(), checkStatus() (+12 more)

### Community 4 - "rust_downloads.rs"
Cohesion: 0.06
Nodes (48): download_release(), DownloadError, Download, InvalidAssetUrl, InvalidDestination, InvalidJar, InvalidMetadata, InvalidRedirect (+40 more)

### Community 5 - "install.rs"
Cohesion: 0.06
Nodes (43): authenticated_status_command_has_a_deadline(), both_protected_recovery_histories_use_one_authenticated_status(), extension_digest(), extension_digest_from_fingerprints(), EXTENSION_UUID, ExtensionManifest, ExtensionStagingMarker, failed_commands_report_exit_status_and_both_output_streams() (+35 more)

### Community 6 - "keyd.rs"
Cohesion: 0.22
Nodes (4): KeydController, KeydError, NeverDispatch, valid_id()

### Community 7 - "rust_sensitivity.rs"
Cohesion: 0.19
Nodes (19): cancelled_newer_restore_does_not_make_older_token_usable(), EYE_REPLY, eye_resize_failure_propagates_without_sensitivity_write(), f6_restores_exact_baseline_even_when_resize_fails(), f7_changes_sensitivity_only_after_exact_resize(), failed_eye_after_write_restores_while_focus_remains_effective(), failed_eye_and_failed_recovery_report_both_and_remain_pending(), failed_preflight_invalidates_generation_until_restore() (+11 more)

### Community 8 - "setup_helper.rs"
Cohesion: 0.10
Nodes (39): HELPER_PATH, KEYD_POLICY_PATH, MACCEL_POLICY_PATH, SERVICE_PATH, abort_and_eof_roll_back_only_new_files(), check_components(), Decision, Abort (+31 more)

### Community 9 - "InstallError"
Cohesion: 0.08
Nodes (10): ExtensionAction, Disable, Enable, InstallError, System, InstallGuard, RootSetup, Fake (+2 more)

### Community 10 - "rust_apply.rs"
Cohesion: 0.15
Nodes (33): applies_two_closed_instances_and_restores_exact_bytes(), apply_publication_runs_under_marker_and_failure_keeps_marker(), backup_is_bound_to_launcher_root(), before_restore_runs_after_validation_and_before_marker(), dangling_recovery_record_symlinks_block_apply(), enter_chat_preserves_hotbar_through_apply_and_restore(), existing_mutation_lock_blocks_another_apply(), failed_keyd_rollback_keeps_marker_and_reports_both_failures() (+25 more)

### Community 11 - "System"
Cohesion: 0.08
Nodes (10): enabled_catalog_probe_error_is_not_classified_as_disabled(), ExtensionState, Active, Error, Inactive, marker_parent_sync_failure_is_returned_before_root_setup(), restore_user(), sync_parent() (+2 more)

### Community 12 - "extension_harness.cjs"
Cohesion: 0.05
Nodes (28): assert, bindings, callbacks, centeredReply, centeredWindow, context, disabling, emitted (+20 more)

### Community 13 - "app.js"
Cohesion: 0.11
Nodes (40): Task 1: Replace privileged reload with bounded restart and rollback, action(), addMappingRow(), addPresetRow(), api(), attachRecorder(), canApply(), changed() (+32 more)

### Community 14 - "Refused"
Cohesion: 0.12
Nodes (15): Artifact, cached_policy_bytes_match(), check_uninstall_records(), existing_policy(), file_state_from_open(), Refused, marker_read_uses_opened_inode_when_path_is_replaced(), open_file_nofollow() (+7 more)

### Community 15 - "GnomeError"
Cohesion: 0.12
Nodes (7): GnomeError, GnomePeer, FakePeer, BlockingGnome, FakeGnome, FakeGnome, FakeGnome

### Community 16 - "rust_profile.rs"
Cohesion: 0.14
Nodes (21): decode(), encode(), keyd_to_minecraft(), MappingError, minecraft_to_keyd(), plan_controls(), supported_other_binding(), game_bindings() (+13 more)

### Community 17 - "runtime.rs"
Cohesion: 0.14
Nodes (13): SharedFocus, GnomeBridge, PresetController, ResizeResult, apply_signal(), MaccelAvailability, Available, RecoveryUncertain (+5 more)

### Community 18 - "install_inner"
Cohesion: 0.11
Nodes (17): enable_extension_checked(), extension_enabled_from_output(), extension_named_in_output(), extension_state_from_info(), install_inner(), InstallReport, policy(), preflight() (+9 more)

### Community 19 - "api.rs"
Cohesion: 0.08
Nodes (24): ApiError, ApiResult, ApiState, apply(), asset(), asset_at(), constant_time_eq(), download() (+16 more)

### Community 20 - "rust_keyd.rs"
Cohesion: 0.14
Nodes (27): activation_requires_effective_focus_and_no_mutation(), clear_refuses_privileged_call_without_private_durable_state(), clear_refuses_untrusted_helper_after_reset(), clear_resets_before_helper_and_blocks_activation_after_failure(), concurrent_controllers_cannot_share_clear_marker_or_activate_early(), concurrent_install_cannot_start_a_second_privileged_install(), concurrent_restore_blocks_other_controller_clear_and_activation(), delayed_install_keeps_both_snapshot_ids_and_restores_latest() (+19 more)

### Community 21 - "McsrToolkitExtension"
Cohesion: 0.12
Nodes (5): McsrToolkitExtension, minecraftWindow(), focusedWindow(), isMinecraft(), ProbeExtension

### Community 22 - "rust_api.rs"
Cohesion: 0.06
Nodes (23): router(), state_dir(), BlockingRuntime, completed_backup_id(), completed_job(), download_allowlist_and_unknown_job(), embedded_assets_session_and_status_have_ui_contract(), failed_runtime_apply_publication_keeps_marker_and_notifies_runtime() (+15 more)

### Community 23 - "rust_focus.rs"
Cohesion: 0.19
Nodes (5): blur_and_quick_refocus_cannot_publish_stale_preset(), event(), graceful_restart_preserves_writer_and_discards_downtime_focus(), heartbeat_timeout_clears_focus(), modal_and_disconnect_clear_focus()

### Community 24 - "rust_runtime.rs"
Cohesion: 0.12
Nodes (31): absent_optional_maccel_serves_observe_only_without_helper_calls(), BlockingDownload, clean_shutdown_releases_the_service_lock(), config(), duplicate_runtime_refuses_same_session_lock(), extension_connection_survives_a_non_minecraft_heartbeat(), failed_eye_recovery_still_clears_previous_keyd_bindings(), failed_keyd_discovery_keeps_fallback_vocabulary() (+23 more)

### Community 25 - "Profile"
Cohesion: 0.28
Nodes (6): ApiBackend, NativeBackend, ApplyReport, Profile, FakeBackend, report()

### Community 26 - ".restore_with_hooks"
Cohesion: 0.18
Nodes (12): ApplyCoordinator, ApplyError, err(), KeydAdapter, record_present(), valid_id(), T, atomic_replace() (+4 more)

### Community 27 - "Review Focus"
Cohesion: 0.18
Nodes (11): Review Focus, Task 10: Complete documentation and end-to-end verification, Task 1: Prove the GNOME Wayland integration path, Task 2: Define the profile and validation contract, Task 3: Discover instances and compose Minecraft controls, Task 4: Generate and install focus-aware keyd rules, Task 5: Resize the focused window and manage eye zoom sensitivity, Task 6: Download the two official JARs (+3 more)

### Community 28 - "State"
Cohesion: 0.10
Nodes (10): safe_record(), err(), load_profile(), NativeDownloads, process_cwds_under(), reconcile_cycle(), reconciliation_unsafe(), ServiceRuntime<SessionPeer, InstalledMaccelPeer> (+2 more)

### Community 29 - "rust_instances.rs"
Cohesion: 0.09
Nodes (25): Prepared, discover_instances(), DiscoveryError, Instance, launcher_root(), ControlPlan, ManagedState, OptionsDocument (+17 more)

### Community 30 - ".start_with_adapters_inner"
Cohesion: 0.23
Nodes (5): DownloadAdapter, discover_keys(), RuntimeConfig, RuntimeError, ServiceRuntime<P, H>

### Community 31 - "Wayland integration acceptance"
Cohesion: 0.08
Nodes (24): 2026-10-05 browser UI acceptance (Task 9), 2026-10-05 keyd helper recovery check, 2026-10-05 supervised authentication input check, 2026-10-05 supervised live focus and chat check, 2026-10-06 approved helper deployment and current state, 2026-10-06 approved movement-test attempt, 2026-10-06 exact observe-only F6 resize acceptance, 2026-10-06 supervised movement test, second attempt (+16 more)

### Community 32 - "rust_gnome.rs"
Cohesion: 0.15
Nodes (8): bridge_stale_check_leaves_expiry_transition_for_runtime_watch(), captured_special_keys_become_gnome_accelerators(), nonreplying_configuration_times_out(), nonreplying_focus_probe_times_out(), NonReplyingPeer, preset_signal_requires_live_fifo_heartbeat_even_when_bus_reports_minecraft(), profile_configuration_uses_extension_json_shape(), stale_preset_signal_is_rejected_after_reconfiguration()

### Community 33 - "rust_runtime_process.rs"
Cohesion: 0.15
Nodes (6): active_python_style_service_lock_blocks_rust_start(), assert_restart_with_retained_writer(), fake_service_child(), FakeMaccel, KillOnDrop, signal_restart_keeps_writer_drains_stale_fifo_and_restores_before_new_eye()

### Community 34 - "with"
Cohesion: 0.10
Nodes (19): jobs, verify, name, on, pull_request, push, workflow_dispatch, permissions (+11 more)

### Community 35 - "apply.rs"
Cohesion: 0.17
Nodes (9): ApplyPreview, BackupFile, ControlMapping, History, InstanceResult, Manifest, MutationLock, new_id() (+1 more)

### Community 36 - "MCSR Toolkit Design"
Cohesion: 0.22
Nodes (9): Applying and recovery, Mapping rules, MCSR Toolkit Design, Platform and components, Purpose and scope, References, Resize and sensitivity, User workflow (+1 more)

### Community 37 - "browser"
Cohesion: 0.28
Nodes (9): browser(), childExited(), closeTemporaryBrowser(), removeTemporaryProfile(), sleep(), spawnChrome(), stopChrome(), waitForChildExit() (+1 more)

### Community 38 - "paths.rs"
Cohesion: 0.18
Nodes (7): directory_fd(), existing_permissive_parent_is_tightened_without_touching_records_or_data_home(), NEXT_TEMP, prepare_private_state(), prepare_private_state_for_uid(), private_child(), private_state_creation_ignores_permissive_umask_and_runtime_accepts_it()

### Community 40 - "build.rs"
Cohesion: 0.10
Nodes (9): EXTENSION_FILES, main(), verify_archive(), write_archive(), extension_archive(), web_asset(), app_javascript_is_embedded(), extension_archive_contains_sources_and_compiled_schema() (+1 more)

### Community 41 - "File map and task boundaries"
Cohesion: 0.13
Nodes (15): File map and task boundaries, Task 10: Root-owned maccel helper, Task 11: Unprivileged sensitivity and preset controller, Task 12: Service runtime and lifecycle, Task 13: Guided install, repair, and uninstall, Task 14: Release artifact, docs, and supervised acceptance, Task 1: Rust artifact and embedded assets, Task 2: Profile validation and mapping contract (+7 more)

### Community 42 - "Enter Opens and Sends Chat Implementation Plan"
Cohesion: 0.22
Nodes (7): Enter Opens and Sends Chat Implementation Plan, Global Constraints, Handoff, Review Focus, Task 1: Profile and mapping contract, Task 2: Browser acceptance contract and documentation, Task 3: Final supervised device check

### Community 43 - "Review Focus"
Cohesion: 0.13
Nodes (13): Global Constraints, Handoff, Review Focus, Rust Source Cutover Implementation Plan, Task 1: Native legacy-lock test fixture, Task 2: Remove Python product and test tree, Task 3: Rust-only active documentation, Task 4: Final deterministic and release-boundary verification (+5 more)

### Community 44 - "ResizeError"
Cohesion: 0.18
Nodes (7): PresetController<P, H>, PresetError, ResizeError, Gnome, Inexact, InvalidSize, NotFocused

### Community 45 - "profile.rs"
Cohesion: 0.15
Nodes (6): ChatKeys, deserialize_bool(), deserialize_dimension(), modifier_family(), parse_dimension_string(), ResizePreset

### Community 46 - "gnome.rs"
Cohesion: 0.17
Nodes (9): accelerator(), bus_error(), DESTINATION, FOCUS_TIMEOUT, INTERFACE, is_function_key(), METHOD_TIMEOUT, OBJECT_PATH (+1 more)

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
Cohesion: 0.36
Nodes (3): maccel_binary(), main(), run()

### Community 52 - "Review Focus"
Cohesion: 0.25
Nodes (8): File map, Global Constraints, Keyd Restart Recovery Implementation Plan, Review Focus, Task 2: Wire the exact clear command and deployment preflight, Task 3: Persist and serialize focus suspension, Task 4: Prove live behavior and empty-config rollback before UI, Task 5: Reconcile documentation, verification, and independent review

### Community 53 - "focus.rs"
Cohesion: 0.09
Nodes (10): FocusEvent, FocusListener, FocusState, FocusTransition, Gained, Lost, Unchanged, HEARTBEAT_TIMEOUT (+2 more)

### Community 54 - "Rust single-binary release checklist"
Cohesion: 0.18
Nodes (11): 2026-10-26 Enter dual-role run, Candidate and reproducible checks, Historical v0.1.0-beta.1 preparation record, Publication verification, Release decision, Remaining live gates, Rust single-binary release checklist, Source cutover (+3 more)

### Community 55 - "SensitivityError"
Cohesion: 0.25
Nodes (3): InstalledMaccelPeer, MaccelPeer, SensitivityError

### Community 56 - "Borderless Minecraft Resize Implementation Plan"
Cohesion: 0.20
Nodes (8): Approval handoff, Borderless Minecraft Resize Implementation Plan, Global Constraints, Research references, Review Focus, Task 1: Prepare decorated XWayland Minecraft windows asynchronously, Task 2: Apply exact dimensions and close every request lifecycle, Task 3: Verify service integration, packaged assets, and user guidance

### Community 57 - "bounded"
Cohesion: 0.25
Nodes (5): is_minecraft_class(), bounded(), canonical_shortcut(), GnomeBridge<P>, SessionPeer

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

### Community 68 - "MCSR Toolkit maccel Sensitivity Recovery Implementation Plan"
Cohesion: 0.32
Nodes (4): File map, Global Constraints, Handoff, MCSR Toolkit maccel Sensitivity Recovery Implementation Plan

### Community 69 - "keyd.rs"
Cohesion: 0.08
Nodes (17): Backups, child_that_does_not_read_stdin_is_killed_without_blocking_writer(), CommandFailure, Failed, TimedOut, CommandOutput, CommandRunner, hung_child_is_killed_before_deadline_expires_twice() (+9 more)

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
Cohesion: 0.19
Nodes (7): COMMAND_TIMEOUT, empty_eye_probe_proves_noninteractive_permission_without_a_token(), EYE_REPLY, HELPER, SensitivityController, State, SUDO

### Community 81 - "Review Focus"
Cohesion: 0.33
Nodes (6): Review Focus, Task 1: Implement the isolated root helper, Task 2: Add the unprivileged fixed-command adapter, Task 3: Integrate startup, focus, and shutdown recovery, Task 4: Document, deploy by separate approval, and run live acceptance, get()

### Community 82 - "parse_profile"
Cohesion: 0.25
Nodes (7): ApiConfig, key(), KeySet, parse_profile(), ProfileError, shortcut(), valid_key_syntax()

### Community 83 - "Enter opens and sends chat"
Cohesion: 0.29
Nodes (7): Approaches, Behavior and constraints, Enter opens and sends chat, Intent, Limits, Verification and rollout, cancel()

### Community 84 - "MCSR Toolkit Rust single-binary design"
Cohesion: 0.29
Nodes (7): Cutover, state, and failure handling, Executable and component boundaries, Goal and release boundary, MCSR Toolkit Rust single-binary design, Migration and verification strategy, Privilege boundary and input safety, Sources

### Community 85 - "ServiceAction"
Cohesion: 0.29
Nodes (6): ServiceAction, Disable, Enable, Reload, Start, Stop

### Community 86 - "cleanupProfile"
Cohesion: 0.40
Nodes (6): cleanupProfile(), same(), stable(), terminal(), uiApi(), validPreview()

### Community 87 - "cleanLineage"
Cohesion: 0.28
Nodes (9): checkStateDirectory(), cleanLineage(), clickResponse(), expectedAppliedOptions(), fail(), readState(), regularFile(), syncStateDirectory() (+1 more)

### Community 88 - "MCSR Toolkit Implementation Plan"
Cohesion: 0.33
Nodes (4): File map, Global Constraints, MCSR Toolkit Implementation Plan, Source notes for implementers

### Community 89 - "Parallel execution ledger"
Cohesion: 0.33
Nodes (6): Current-machine inspection (2026-10-29), Execution results and outstanding decision, Fullscreen/focus regression correction (2026-10-29), Parallel execution ledger, Supervised Jingle thin test and centering amendment (2026-10-29), unmaximize()

### Community 90 - "MCSR Toolkit maccel sensitivity recovery design"
Cohesion: 0.33
Nodes (6): Context and goal, MCSR Toolkit maccel sensitivity recovery design, Privilege boundary, Service behavior and failure recovery, Sources, Verification and acceptance

### Community 91 - "Rust Single-Binary Migration Implementation Plan"
Cohesion: 0.40
Nodes (4): Execution and approval gates, Global Constraints, Review Focus, Rust Single-Binary Migration Implementation Plan

## Knowledge Gaps
- **45 isolated node(s):** `mcsr-toolkit`, `Launcher`, `Ninjabrain`, `InvalidMetadata`, `InvalidAssetUrl` (+40 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 553 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **13 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `InstallError` connect `InstallError` to `rust_install.rs`, `install.rs`, `Environment`, `System`, `Refused`, `RealRootSetup`, `install_inner`, `ServiceAction`, `http_probe`?**
  _High betweenness centrality (0.055) - this node is a cross-community bridge._
- **What connects `mcsr-toolkit`, `Launcher`, `Ninjabrain` to the rest of the system?**
  _45 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `rust_install.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06161301488404292 - nodes in this community are weakly interconnected._
- **Why does `Profile` connect `Profile` to `apply.rs`, `keyd.rs`, `keyd.rs`, `rust_apply.rs`, `ResizeError`, `profile.rs`, `gnome.rs`, `rust_profile.rs`, `runtime.rs`, `parse_profile`, `api.rs`, `rust_api.rs`, `bounded`, `.restore_with_hooks`, `State`, `rust_instances.rs`?**
  _High betweenness centrality (0.039) - this node is a cross-community bridge._
- **Should `maccel_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05811027224891329 - nodes in this community are weakly interconnected._
- **Why does `GnomeError` connect `GnomeError` to `rust_gnome.rs`, `rust_runtime_process.rs`, `rust_sensitivity.rs`, `ResizeError`, `gnome.rs`, `bounded`, `.listen_presets`?**
  _High betweenness centrality (0.029) - this node is a cross-community bridge._
- **Should `keyd_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05329153605015674 - nodes in this community are weakly interconnected._