use mcsr_toolkit::{
    profile::{parse_profile, KeySet},
    routing::RoutingPlan,
};
fn profile(json: &str) -> mcsr_toolkit::profile::Profile {
    parse_profile(json.as_bytes(), &KeySet::fallback()).unwrap()
}
#[test]
fn f3_remap_and_resize_use_separate_outputs() {
    let p = profile(
        r#"{"crafting":{"tab":"f3"},"presets":[{"id":"normal","width":854,"height":480,"shortcut":"b","eye_zoom":false}]}"#,
    );
    let plan = RoutingPlan::compile(&p).unwrap();
    assert!(plan.bindings.contains(&"tab = layerk(mcsr_f3, f3)".into()));
    assert!(plan.bindings.contains(&"f3 = layerk(mcsr_f3, f3)".into()));
    assert!(plan.bindings.contains(&"b = matchmods(f13, b)".into()));
    assert!(plan.fragment.contains("[mcsr_f3]\n"));
    assert!(plan.fragment.contains("b = b\n"));
    assert_eq!(plan.signals[0].accelerator, "F13");
}
#[test]
fn eye_zoom_and_modified_shortcuts_match_exact_modifiers() {
    let p = profile(
        r#"{"presets":[{"id":"eye","width":280,"height":1000,"shortcut":"b","eye_zoom":true},{"id":"wide","width":1200,"height":480,"shortcut":"leftcontrol+b","eye_zoom":false}]}"#,
    );
    let plan = RoutingPlan::compile(&p).unwrap();
    let b = plan
        .bindings
        .iter()
        .find(|s| s.starts_with("b = "))
        .unwrap();
    assert!(b.contains("matchmods(f13,"));
    assert!(b.contains("matchmods(S-f13,"));
    assert!(b.contains("matchmods(C-f14,"));
    assert_eq!(plan.signals[1].accelerator, "<Control>F14");
}
#[test]
fn signals_avoid_profile_keys_and_generation_changes() {
    let p = profile(
        r#"{"crafting":{"a":"f13"},"presets":[{"id":"normal","width":854,"height":480,"shortcut":"b","eye_zoom":false}]}"#,
    );
    let plan = RoutingPlan::compile(&p).unwrap();
    assert_eq!(plan.signals[0].accelerator, "F14");
    let mut changed = p.clone();
    changed.presets[0].width += 1;
    assert_ne!(
        plan.generation,
        RoutingPlan::compile(&changed).unwrap().generation
    );
    assert_eq!(plan, RoutingPlan::compile(&p).unwrap());
}
#[test]
fn chat_restores_resize_and_f3_sources_to_physical_keys() {
    let p = profile(
        r#"{"crafting":{"tab":"f3"},"presets":[{"id":"normal","width":854,"height":480,"shortcut":"b","eye_zoom":false}]}"#,
    );
    let plan = RoutingPlan::compile(&p).unwrap();
    let chat = plan.fragment.split("[mcsr_f3]").next().unwrap();
    assert!(chat.contains("b = b\n"));
    assert!(chat.contains("tab = tab\n"));
    assert!(chat.contains("f3 = f3\n"));
}
#[test]
fn signal_exhaustion_is_reported() {
    let mut p = profile(
        r#"{"presets":[{"id":"normal","width":854,"height":480,"shortcut":"b","eye_zoom":false}]}"#,
    );
    for n in 13..=24 {
        p.controls
            .insert(format!("key_custom.{n}"), format!("f{n}"));
    }
    assert!(RoutingPlan::compile(&p)
        .unwrap_err()
        .contains("internal resize"));
}

fn engine(profile: &mcsr_toolkit::profile::Profile, input: &str, expected: &str) {
    let plan = RoutingPlan::compile(profile).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("generated.conf");
    let case = temp.path().join("events.t");
    std::fs::write(
        &config,
        format!(
            "[ids]\n*\n\n[main]\n{}\n\n{}",
            plan.bindings.join("\n"),
            plan.fragment
        ),
    )
    .unwrap();
    std::fs::write(&case, format!("{}\n\n{}\n", input.trim(), expected.trim())).unwrap();
    let output = std::process::Command::new(env!("MCSR_KEYD_TEST_ENGINE"))
        .args([&config, &case])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}\nconfig:{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        std::fs::read_to_string(config).unwrap()
    );
}
fn combination_profile() -> mcsr_toolkit::profile::Profile {
    profile(
        r#"{"crafting":{"tab":"f3"},"presets":[{"id":"normal","width":854,"height":480,"shortcut":"b","eye_zoom":false},{"id":"thin","width":280,"height":480,"shortcut":"g","eye_zoom":false},{"id":"eye","width":280,"height":1000,"shortcut":"f","eye_zoom":true}]}"#,
    )
}
#[test]
fn generated_engine_preserves_all_five_combinations_and_release_orders() {
    let p = combination_profile();
    for source in ["tab", "f3"] {
        for key in ["b", "g", "f"] {
            engine(
                &p,
                &format!("{source} down\n100ms\n{key} down\n{key} up\n{source} up"),
                &format!("f3 down\n{key} down\n{key} up\nf3 up"),
            );
            engine(
                &p,
                &format!("{source} down\n{key} down\n{source} up\n{key} up"),
                &format!("f3 down\n{key} down\nf3 up\n{key} up"),
            );
        }
        engine(
            &p,
            &format!("shift down\n{source} down\nf down\nf up\n{source} up\nshift up"),
            "shift down\nf3 down\nf down\nf up\nf3 up\nshift up",
        );
        engine(
            &p,
            &format!("shift down\n{source} down\n{source} up\nshift up"),
            "shift down\nf3 down\nf3 up\nshift up",
        );
        engine(
            &p,
            &format!(
                "{source} down\nshift down\nf down\nf up\nshift up\nf down\nf up\n{source} up"
            ),
            "f3 down\nshift down\nf down\nf up\nshift up\nf down\nf up\nf3 up",
        );
        engine(
            &p,
            &format!("{source} down\n{source} up\nb down\nb up"),
            "f3 down\nf3 up\nf13 down\nf13 up",
        );
    }
}
#[test]
fn generated_engine_handles_native_and_remapped_f3_together() {
    engine(
        &combination_profile(),
        "tab down\nf3 down\ntab up\nb down\nb up\nf3 up\nb down\nb up",
        "f3 down\nb down\nb up\nf3 up\nf13 down\nf13 up",
    );
}
#[test]
fn generated_engine_preserves_chat_and_exact_modified_shortcuts() {
    let p = combination_profile();
    engine(
        &p,
        "shift down\nb down\nb up\nshift up",
        "shift down\nb down\nb up\nshift up",
    );
    engine(
        &p,
        "shift down\nf down\nf up\nshift up",
        "shift down\nf15 down\nf15 up\nshift up",
    );
    engine(
        &p,
        "t down\nt up\ntab down\ntab up\nb down\nb up\nenter down\nenter up\nb down\nb up",
        "t down\nt up\ntab down\ntab up\nb down\nb up\nenter down\nenter up\nf13 down\nf13 up",
    );
}

#[test]
fn observed_minecraft_bindings_cannot_be_consumed_as_internal_signals() {
    let p = combination_profile();
    let options =
        mcsr_toolkit::options::parse_options(b"key_key.custom:key.keyboard.f13\n").unwrap();
    let plan = mcsr_toolkit::mapping::plan_controls(&options, &p, None).unwrap();
    assert!(plan
        .conflicts
        .iter()
        .any(|s| s.contains("internal resize signal")));
}
#[test]
fn unsupported_altgr_resize_is_rejected_instead_of_misrouted() {
    assert!(parse_profile(br#"{"presets":[{"id":"normal","width":854,"height":480,"shortcut":"rightalt+b","eye_zoom":false}]}"#,&KeySet::fallback()).is_err());
}

#[test]
fn command_chat_opened_while_f3_is_held_keeps_typed_resize_keys() {
    engine(
        &combination_profile(),
        "tab down\n/ down\n/ up\ntab up\nb down\nb up\nenter down\nenter up\nb down\nb up",
        "f3 down\n/ down\n/ up\nf3 up\nb down\nb up\nenter down\nenter up\nf13 down\nf13 up",
    );
}
