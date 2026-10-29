# Graph Report - fix-uninstall-keyd  (2026-10-29)

## Corpus Check
- 77 files · ~100,308 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 11 file(s) not represented in the graph (top: (none) 3, .conf 2, .sudoers 2)

## Summary
- 1948 nodes · 5033 edges · 75 communities (65 shown, 10 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 177 edges (avg confidence: 0.82)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `f094e3ba`
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
- Fake
- rust_apply.rs
- System
- extension_harness.cjs
- app.js
- Refused
- GnomeError
- rust_profile.rs
- downloads.rs
- install_inner
- api.rs
- focus.rs
- McsrToolkitExtension
- DownloadError
- rust_api.rs
- rust_runtime.rs
- Profile
- .restore_with_hooks
- Product
- runtime.rs
- rust_instances.rs
- .start_with_adapters_inner
- Wayland integration acceptance
- rust_gnome.rs
- rust_runtime_process.rs
- with
- apply.rs
- duration
- ServiceAction
- paths.rs
- Environment
- read
- File map and task boundaries
- Enter opens and sends chat
- Review Focus
- ResizeError
- Profile
- gnome.rs
- RealRootSetup
- GNOME Wayland focus probe
- http_probe
- runtime.rs
- pathbuf
- MaccelAvailability
- MCSR Toolkit Design
- DownloadAdapter
- MCSR Toolkit maccel Sensitivity Recovery Implementation Plan
- Binding input capture implementation plan
- Minecraft Control Picker Implementation Plan
- Key capture for binding fields
- MCSR Toolkit
- MCSR Toolkit v0.1.0-beta.2
- Uninstall keyd recovery implementation plan
- MCSR Toolkit maccel sensitivity recovery design
- Rust Single-Binary Migration Implementation Plan
- v0.1.0-beta.2 Release Preparation Plan
- Minecraft control picker
- GNOME Wayland resize and sensitivity acceptance
- .listen_presets
- AGENTS.md
- keyd-focus-probe.sh script
- mcsr-toolkit

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
- `Task 1: Rust artifact and embedded assets` --references--> `main()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-rust-single-binary.md → integration/live-ui-acceptance.mjs
- `Release decision` --references--> `main()`  [INFERRED]
  docs/rust-release-checklist.md → integration/live-ui-acceptance.mjs
- `Task 1: Implement the isolated root helper` --references--> `main()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-maccel-sensitivity-recovery.md → integration/live-ui-acceptance.mjs
- `Review Focus` --references--> `get()`  [INFERRED]
  docs/superpowers/plans/2026-10-22-maccel-sensitivity-recovery.md → integration/live-ui-acceptance.mjs

## Import Cycles
- 1-file cycle: `src/keyd.rs -> src/keyd.rs`
- 2-file cycle: `src/mapping.rs -> src/options.rs -> src/mapping.rs`

## Communities (75 total, 10 thin omitted)

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
Cohesion: 0.05
Nodes (69): 2026-10-26 Enter dual-role run, Candidate and reproducible checks, Historical v0.1.0-beta.1 preparation record, Publication verification, Release decision, Remaining live gates, Rust single-binary release checklist, Source cutover (+61 more)

### Community 4 - "rust_downloads.rs"
Cohesion: 0.19
Nodes (22): ReleaseAsset, assert_preserved(), asset(), BrokenReader, corrupt_entry_data_preserves_previous_file(), deflated_jar(), destination_must_be_a_jar(), empty_manifest_is_not_a_valid_jar() (+14 more)

### Community 5 - "install.rs"
Cohesion: 0.06
Nodes (43): authenticated_status_command_has_a_deadline(), both_protected_recovery_histories_use_one_authenticated_status(), extension_digest(), extension_digest_from_fingerprints(), EXTENSION_UUID, ExtensionManifest, ExtensionStagingMarker, failed_commands_report_exit_status_and_both_output_streams() (+35 more)

### Community 6 - "keyd.rs"
Cohesion: 0.05
Nodes (47): Backups, child_that_does_not_read_stdin_is_killed_without_blocking_writer(), CommandFailure, Failed, TimedOut, CommandOutput, CommandRunner, hung_child_is_killed_before_deadline_expires_twice() (+39 more)

### Community 7 - "rust_sensitivity.rs"
Cohesion: 0.06
Nodes (31): COMMAND_TIMEOUT, empty_eye_probe_proves_noninteractive_permission_without_a_token(), EYE_REPLY, InstalledMaccelPeer, MaccelPeer, HELPER, SensitivityController, SensitivityController<H> (+23 more)

### Community 8 - "setup_helper.rs"
Cohesion: 0.10
Nodes (39): HELPER_PATH, KEYD_POLICY_PATH, MACCEL_POLICY_PATH, SERVICE_PATH, abort_and_eof_roll_back_only_new_files(), check_components(), Decision, Abort (+31 more)

### Community 9 - "Fake"
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
Nodes (26): assert, bindings, callbacks, context, disabling, emitted, extension, fs (+18 more)

### Community 13 - "app.js"
Cohesion: 0.05
Nodes (63): File map, Global Constraints, MCSR Toolkit Implementation Plan, Review Focus, Source notes for implementers, Task 10: Complete documentation and end-to-end verification, Task 1: Prove the GNOME Wayland integration path, Task 2: Define the profile and validation contract (+55 more)

### Community 14 - "Refused"
Cohesion: 0.12
Nodes (15): Artifact, cached_policy_bytes_match(), check_uninstall_records(), existing_policy(), file_state_from_open(), Refused, marker_read_uses_opened_inode_when_path_is_replaced(), open_file_nofollow() (+7 more)

### Community 15 - "GnomeError"
Cohesion: 0.11
Nodes (10): is_minecraft_class(), bounded(), GnomeBridge<P>, GnomeError, GnomePeer, SessionPeer, BlockingGnome, FakeGnome (+2 more)

### Community 16 - "rust_profile.rs"
Cohesion: 0.13
Nodes (21): ControlPlan, decode(), encode(), keyd_to_minecraft(), ManagedState, MappingError, minecraft_to_keyd(), plan_controls() (+13 more)

### Community 17 - "downloads.rs"
Cohesion: 0.15
Nodes (11): download_release(), Io, DownloadResult, HttpClient, HttpResponse, MAX_BYTES, MAX_METADATA_BYTES, MAX_REDIRECTS (+3 more)

### Community 18 - "install_inner"
Cohesion: 0.11
Nodes (17): enable_extension_checked(), extension_enabled_from_output(), extension_named_in_output(), extension_state_from_info(), install_inner(), InstallReport, policy(), preflight() (+9 more)

### Community 19 - "api.rs"
Cohesion: 0.08
Nodes (25): ApiConfig, ApiError, ApiResult, ApiState, apply(), asset(), asset_at(), constant_time_eq() (+17 more)

### Community 20 - "focus.rs"
Cohesion: 0.10
Nodes (10): FocusEvent, FocusListener, FocusState, FocusTransition, Gained, Lost, Unchanged, HEARTBEAT_TIMEOUT (+2 more)

### Community 21 - "McsrToolkitExtension"
Cohesion: 0.12
Nodes (5): McsrToolkitExtension, minecraftWindow(), focusedWindow(), isMinecraft(), ProbeExtension

### Community 22 - "DownloadError"
Cohesion: 0.18
Nodes (9): DownloadError, Download, InvalidAssetUrl, InvalidDestination, InvalidJar, InvalidMetadata, InvalidRedirect, TooLarge (+1 more)

### Community 23 - "rust_api.rs"
Cohesion: 0.06
Nodes (23): router(), state_dir(), BlockingRuntime, completed_backup_id(), completed_job(), download_allowlist_and_unknown_job(), embedded_assets_session_and_status_have_ui_contract(), failed_runtime_apply_publication_keeps_marker_and_notifies_runtime() (+15 more)

### Community 24 - "rust_runtime.rs"
Cohesion: 0.18
Nodes (29): absent_optional_maccel_serves_observe_only_without_helper_calls(), clean_shutdown_releases_the_service_lock(), config(), duplicate_runtime_refuses_same_session_lock(), extension_connection_survives_a_non_minecraft_heartbeat(), failed_eye_recovery_still_clears_previous_keyd_bindings(), failed_keyd_discovery_keeps_fallback_vocabulary(), failed_startup_restore_prevents_eye_acceptance_and_http_binding() (+21 more)

### Community 25 - "Profile"
Cohesion: 0.20
Nodes (8): ApiBackend, NativeBackend, ApplyPreview, ApplyReport, ControlMapping, InstanceResult, FakeBackend, report()

### Community 26 - ".restore_with_hooks"
Cohesion: 0.20
Nodes (9): ApplyCoordinator, ApplyError, err(), KeydAdapter, record_present(), valid_id(), T, ensure_state() (+1 more)

### Community 27 - "Product"
Cohesion: 0.33
Nodes (6): Lookup, Product, Launcher, Ninjabrain, resolve_release(), valid_asset_url()

### Community 28 - "runtime.rs"
Cohesion: 0.13
Nodes (8): safe_record(), err(), load_profile(), process_cwds_under(), reconcile_cycle(), reconciliation_unsafe(), State, visible_active_preset()

### Community 29 - "rust_instances.rs"
Cohesion: 0.10
Nodes (21): discover_instances(), DiscoveryError, launcher_root(), OptionsDocument, OptionsError, parse_options(), render_options(), discovery_excludes_missing_malformed_and_mismatched_instance_json() (+13 more)

### Community 30 - ".start_with_adapters_inner"
Cohesion: 0.26
Nodes (4): discover_keys(), RuntimeConfig, RuntimeError, ServiceRuntime<P, H>

### Community 31 - "Wayland integration acceptance"
Cohesion: 0.08
Nodes (24): 2026-10-05 browser UI acceptance (Task 9), 2026-10-05 keyd helper recovery check, 2026-10-05 supervised authentication input check, 2026-10-05 supervised live focus and chat check, 2026-10-06 approved helper deployment and current state, 2026-10-06 approved movement-test attempt, 2026-10-06 exact observe-only F6 resize acceptance, 2026-10-06 supervised movement test, second attempt (+16 more)

### Community 32 - "rust_gnome.rs"
Cohesion: 0.13
Nodes (9): bridge_stale_check_leaves_expiry_transition_for_runtime_watch(), captured_special_keys_become_gnome_accelerators(), FakePeer, nonreplying_configuration_times_out(), nonreplying_focus_probe_times_out(), NonReplyingPeer, preset_signal_requires_live_fifo_heartbeat_even_when_bus_reports_minecraft(), profile_configuration_uses_extension_json_shape() (+1 more)

### Community 33 - "rust_runtime_process.rs"
Cohesion: 0.15
Nodes (6): active_python_style_service_lock_blocks_rust_start(), assert_restart_with_retained_writer(), fake_service_child(), FakeMaccel, KillOnDrop, signal_restart_keeps_writer_drains_stale_fifo_and_restores_before_new_eye()

### Community 34 - "with"
Cohesion: 0.10
Nodes (19): jobs, verify, name, on, pull_request, push, workflow_dispatch, permissions (+11 more)

### Community 35 - "apply.rs"
Cohesion: 0.14
Nodes (11): BackupFile, History, Manifest, MutationLock, new_id(), NEXT_BACKUP, Prepared, Instance (+3 more)

### Community 36 - "duration"
Cohesion: 0.18
Nodes (5): blur_and_quick_refocus_cannot_publish_stale_preset(), event(), graceful_restart_preserves_writer_and_discards_downtime_focus(), heartbeat_timeout_clears_focus(), modal_and_disconnect_clear_focus()

### Community 37 - "ServiceAction"
Cohesion: 0.29
Nodes (6): ServiceAction, Disable, Enable, Reload, Start, Stop

### Community 38 - "paths.rs"
Cohesion: 0.18
Nodes (7): directory_fd(), existing_permissive_parent_is_tightened_without_touching_records_or_data_home(), NEXT_TEMP, prepare_private_state(), prepare_private_state_for_uid(), private_child(), private_state_creation_ignores_permissive_umask_and_runtime_accepts_it()

### Community 40 - "read"
Cohesion: 0.16
Nodes (5): extension_archive(), web_asset(), app_javascript_is_embedded(), extension_archive_contains_sources_and_compiled_schema(), root_serves_embedded_html()

### Community 41 - "File map and task boundaries"
Cohesion: 0.13
Nodes (15): File map and task boundaries, Task 10: Root-owned maccel helper, Task 11: Unprivileged sensitivity and preset controller, Task 12: Service runtime and lifecycle, Task 13: Guided install, repair, and uninstall, Task 14: Release artifact, docs, and supervised acceptance, Task 1: Rust artifact and embedded assets, Task 2: Profile validation and mapping contract (+7 more)

### Community 42 - "Enter opens and sends chat"
Cohesion: 0.13
Nodes (13): Enter Opens and Sends Chat Implementation Plan, Global Constraints, Handoff, Review Focus, Task 1: Profile and mapping contract, Task 2: Browser acceptance contract and documentation, Task 3: Final supervised device check, Approaches (+5 more)

### Community 43 - "Review Focus"
Cohesion: 0.13
Nodes (13): Global Constraints, Handoff, Review Focus, Rust Source Cutover Implementation Plan, Task 1: Native legacy-lock test fixture, Task 2: Remove Python product and test tree, Task 3: Rust-only active documentation, Task 4: Final deterministic and release-boundary verification (+5 more)

### Community 44 - "ResizeError"
Cohesion: 0.18
Nodes (7): PresetController<P, H>, PresetError, ResizeError, Gnome, Inexact, InvalidSize, NotFocused

### Community 45 - "Profile"
Cohesion: 0.14
Nodes (15): ChatKeys, deserialize_bool(), deserialize_dimension(), key(), KeySet, modifier_family(), parse_dimension_string(), parse_profile() (+7 more)

### Community 46 - "gnome.rs"
Cohesion: 0.17
Nodes (10): accelerator(), bus_error(), canonical_shortcut(), DESTINATION, FOCUS_TIMEOUT, INTERFACE, is_function_key(), METHOD_TIMEOUT (+2 more)

### Community 47 - "RealRootSetup"
Cohesion: 0.22
Nodes (4): abandoned_setup_terminates_privileged_process_group(), classify_root_commit_response(), RootAborted, RealRootSetup

### Community 48 - "GNOME Wayland focus probe"
Cohesion: 0.17
Nodes (12): Chat focus risk, Failed chat safety test and cleanup, Failed supervised chat-only probe, 2026-10-04, GNOME Wayland focus probe, Initial environment at first inspection (2026-10-03), Observed after GNOME extension activation, Production observe-only service, 2026-10-04, Restart-helper recovery check, 2026-10-05 (+4 more)

### Community 50 - "runtime.rs"
Cohesion: 0.17
Nodes (8): SharedFocus, GnomeBridge, PresetController, ResizeResult, apply_signal(), publish_active_preset(), ServiceLock, ServiceRuntime

### Community 51 - "pathbuf"
Cohesion: 0.15
Nodes (7): EXTENSION_FILES, main(), verify_archive(), write_archive(), maccel_binary(), main(), run()

### Community 52 - "MaccelAvailability"
Cohesion: 0.22
Nodes (6): MaccelAvailability, Available, RecoveryUncertain, Unavailable, optional_maccel_availability(), ServiceRuntime<SessionPeer, InstalledMaccelPeer>

### Community 53 - "MCSR Toolkit Design"
Cohesion: 0.22
Nodes (9): Applying and recovery, Mapping rules, MCSR Toolkit Design, Platform and components, Purpose and scope, References, Resize and sensitivity, User workflow (+1 more)

### Community 55 - "DownloadAdapter"
Cohesion: 0.15
Nodes (4): DownloadAdapter, NativeDownloads, BlockingDownload, FakeKeyd

### Community 57 - "MCSR Toolkit maccel Sensitivity Recovery Implementation Plan"
Cohesion: 0.15
Nodes (11): File map, Global Constraints, Handoff, MCSR Toolkit maccel Sensitivity Recovery Implementation Plan, Cutover, state, and failure handling, Executable and component boundaries, Goal and release boundary, MCSR Toolkit Rust single-binary design (+3 more)

### Community 58 - "Binding input capture implementation plan"
Cohesion: 0.25
Nodes (7): Binding input capture implementation plan, Execution notes, Global constraints, Review focus, Task 1: Supported runtime names and Minecraft mouse mappings, Task 2: Shared recorder and field integration, Task 3: Verification and review

### Community 59 - "Minecraft Control Picker Implementation Plan"
Cohesion: 0.25
Nodes (7): Execution notes, Global Constraints, Integration recheck, Minecraft Control Picker Implementation Plan, Review Focus, Task 1: Searchable picker and profile integration, Verification results

### Community 61 - "Key capture for binding fields"
Cohesion: 0.29
Nodes (6): Approaches, Behavior, Key capture for binding fields, Scope, Validation, Verified runtime constraints

### Community 63 - "MCSR Toolkit"
Cohesion: 0.29
Nodes (7): Build and deterministic checks, Live keyd mode and recovery, MCSR Toolkit, Repair and uninstall, Run and use the UI, Rust executable and beta status, Verify and install

### Community 65 - "MCSR Toolkit v0.1.0-beta.2"
Cohesion: 0.33
Nodes (6): Beta status and known limitations, Candidate verification and artifact identity, Download and install, Highlights, MCSR Toolkit v0.1.0-beta.2, New in beta.2

### Community 67 - "Uninstall keyd recovery implementation plan"
Cohesion: 0.33
Nodes (5): Review focus, Scope and constraints, Task 1: Uninstall recovery, Task 2: Recovery diagnostics and validation, Uninstall keyd recovery implementation plan

### Community 68 - "MCSR Toolkit maccel sensitivity recovery design"
Cohesion: 0.33
Nodes (6): Context and goal, MCSR Toolkit maccel sensitivity recovery design, Privilege boundary, Service behavior and failure recovery, Sources, Verification and acceptance

### Community 69 - "Rust Single-Binary Migration Implementation Plan"
Cohesion: 0.40
Nodes (4): Execution and approval gates, Global Constraints, Review Focus, Rust Single-Binary Migration Implementation Plan

### Community 70 - "v0.1.0-beta.2 Release Preparation Plan"
Cohesion: 0.40
Nodes (4): Constraints, Review Focus, Task 1: Candidate and documentation, v0.1.0-beta.2 Release Preparation Plan

### Community 71 - "Minecraft control picker"
Cohesion: 0.40
Nodes (4): Approaches, Behavior and constraints, Minecraft control picker, Scope and validation

### Community 72 - "GNOME Wayland resize and sensitivity acceptance"
Cohesion: 0.40
Nodes (5): Current Rust observation procedure, Current Rust release gates, GNOME Wayland resize and sensitivity acceptance, Historical evidence (2026-10-04–06), Historical sensitivity and helper acceptance (2026-10-22)

## Knowledge Gaps
- **45 isolated node(s):** `mcsr-toolkit`, `Launcher`, `Ninjabrain`, `InvalidMetadata`, `InvalidAssetUrl` (+40 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 513 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `InstallError` connect `Fake` to `rust_install.rs`, `install.rs`, `ServiceAction`, `Environment`, `System`, `Refused`, `RealRootSetup`, `http_probe`, `install_inner`?**
  _High betweenness centrality (0.054) - this node is a cross-community bridge._
- **What connects `mcsr-toolkit`, `Launcher`, `Ninjabrain` to the rest of the system?**
  _45 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `rust_install.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06161301488404292 - nodes in this community are weakly interconnected._
- **Why does `Profile` connect `Profile` to `apply.rs`, `keyd.rs`, `rust_apply.rs`, `ResizeError`, `gnome.rs`, `GnomeError`, `rust_profile.rs`, `runtime.rs`, `api.rs`, `rust_api.rs`, `Profile`, `.restore_with_hooks`, `runtime.rs`?**
  _High betweenness centrality (0.042) - this node is a cross-community bridge._
- **Should `maccel_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05811027224891329 - nodes in this community are weakly interconnected._
- **Why does `SensitivityController` connect `rust_sensitivity.rs` to `runtime.rs`, `gnome.rs`?**
  _High betweenness centrality (0.018) - this node is a cross-community bridge._
- **Should `keyd_helper.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05329153605015674 - nodes in this community are weakly interconnected._