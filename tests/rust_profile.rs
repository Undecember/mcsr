use mcsr_toolkit::mapping::{plan_controls, ManagedState};
use mcsr_toolkit::options::{parse_options, render_options, OptionsDocument};
use mcsr_toolkit::profile::{game_bindings, parse_profile, render_keyd_fragment, KeySet, Profile};
use std::collections::HashSet;

fn parse(json: &str) -> Profile {
    parse_profile(json.as_bytes(), &KeySet::fallback()).unwrap()
}

#[test]
fn enter_opens_and_sends_chat() {
    let profile = parse(r#"{"chat":{"open_chat":"enter","send":"enter"}}"#);
    let game = game_bindings(&profile);
    assert!(game.contains(&"enter = togglem(mcsr_chat, enter)".to_string()));
    assert_eq!(
        game.iter()
            .filter(|line| line.starts_with("enter = "))
            .count(),
        1
    );
    let game_keys: HashSet<_> = game
        .iter()
        .map(|line| line.split_once(" = ").unwrap().0)
        .collect();
    assert_eq!(game_keys.len(), game.len());

    let fragment = render_keyd_fragment(&profile);
    let chat: Vec<_> = fragment.lines().skip(1).collect();
    assert!(chat.contains(&"enter = clearm(enter)"));
    assert_eq!(
        chat.iter()
            .filter(|line| line.starts_with("enter = "))
            .count(),
        1
    );
    let chat_keys: HashSet<_> = chat
        .iter()
        .map(|line| line.split_once(" = ").unwrap().0)
        .collect();
    assert_eq!(chat_keys.len(), chat.len());
}

#[test]
fn other_chat_duplicates_still_fail() {
    for json in [
        r#"{"chat":{"open_chat":"slash"}}"#,
        r#"{"chat":{"open_command":"enter"}}"#,
        r#"{"chat":{"open_chat":"esc"}}"#,
        r#"{"chat":{"send":"esc"}}"#,
        r#"{"chat":{"open_command":"esc"}}"#,
        r#"{"chat":{"open_chat":"j","send":"j"}}"#,
        r#"{"chat":{"open_chat":"enter","send":"enter","open_command":"enter"}}"#,
        r#"{"chat":{"open_chat":"enter","send":"enter","cancel":"enter"}}"#,
        r#"{"chat":{"open_chat":"enter","send":"enter"},"crafting":{"enter":"k"}}"#,
        r#"{"chat":{"open_chat":"enter","send":"enter"},"ninjabrain":{"enter":"leftcontrol+n"}}"#,
        r#"{"chat":{"open_chat":"enter","send":"enter"},"controls":{"key_key.back":"enter"}}"#,
    ] {
        assert!(
            parse_profile(json.as_bytes(), &KeySet::fallback()).is_err(),
            "accepted {json}"
        );
    }
}

#[test]
fn enter_chat_preserves_hotbar_in_real_layout_preview() {
    let baseline = b"gamma:1.0\r\nkey_key.back:key.keyboard.s\r\nkey_key.chat:key.keyboard.enter\r\nkey_key.command:key.keyboard.slash\r\nkey_key.hotbar.7:key.keyboard.t\r\n";
    let options = parse_options(baseline).unwrap();
    let profile = parse(r#"{"crafting":{"s":"k"},"chat":{"open_chat":"enter","send":"enter"}}"#);
    let plan = plan_controls(&options, &profile, None).unwrap();
    assert!(plan.conflicts.is_empty());
    assert_eq!(
        plan.updates
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>(),
        [("key_key.back", "key.keyboard.k")]
    );
    assert_eq!(
        plan.preview,
        [("s".into(), "k".into(), "key_key.back".into())]
    );
    assert_eq!(render_options(&options, &plan), b"gamma:1.0\r\nkey_key.back:key.keyboard.k\r\nkey_key.chat:key.keyboard.enter\r\nkey_key.command:key.keyboard.slash\r\nkey_key.hotbar.7:key.keyboard.t\r\n");
}

#[test]
fn default_profile_matches_python_json_and_chat_fragment() {
    let profile = parse(include_str!("fixtures/profile-default.json"));
    assert_eq!(
        format!("{}\n", serde_json::to_string_pretty(&profile).unwrap()),
        include_str!("fixtures/profile-default.json")
    );
    assert_eq!(
        serde_json::to_value(&profile).unwrap(),
        serde_json::from_str::<serde_json::Value>(include_str!("fixtures/profile-default.json"))
            .unwrap()
    );
    assert_eq!(
        render_keyd_fragment(&profile).as_bytes(),
        include_bytes!("fixtures/keyd-chat.conf")
    );
}

#[test]
fn remap_movement_preserves_physical_key_and_preview_order() {
    let options = OptionsDocument {
        lines: vec![],
        bindings: [
            ("key_key.back".into(), "key.keyboard.s".into()),
            ("key_key.chat".into(), "key.keyboard.t".into()),
        ]
        .into(),
    };
    let profile =
        parse(r#"{"controls":{"key_key.back":"s"},"crafting":{"s":"k"},"snippets":["ka"]}"#);
    let plan = plan_controls(&options, &profile, None).unwrap();
    assert_eq!(
        plan.updates.get("key_key.back").map(String::as_str),
        Some("key.keyboard.k")
    );
    assert_eq!(
        plan.preview[0],
        ("s".into(), "k".into(), "key_key.back".into())
    );
    assert!(plan.conflicts.is_empty());
    assert_eq!(profile.snippets, ["KA"]);
    assert_eq!(
        plan.keyd_bindings,
        [
            "s = k",
            "t = togglem(mcsr_chat, t)",
            "slash = togglem(mcsr_chat, slash)",
        ]
    );
    assert_eq!(
        plan.next_state
            .original
            .get("key_key.back")
            .map(String::as_str),
        Some("s")
    );
    let edited = OptionsDocument {
        lines: vec![],
        bindings: [
            ("key_key.back".into(), "key.keyboard.k".into()),
            ("key_key.chat".into(), "key.keyboard.t".into()),
        ]
        .into(),
    };
    assert!(plan_controls(&edited, &profile, Some(&plan.next_state))
        .unwrap()
        .updates
        .is_empty());
    let _ = ManagedState::default();
}

#[test]
fn chat_conflict_is_reported_instead_of_changing_other_control() {
    let options = OptionsDocument {
        lines: vec![],
        bindings: [
            ("key_key.chat".into(), "key.keyboard.t".into()),
            ("key_key.command".into(), "key.keyboard.slash".into()),
            ("key_key.back".into(), "key.keyboard.y".into()),
        ]
        .into(),
    };
    let plan = plan_controls(
        &options,
        &parse(r#"{"chat":{"open_chat":"y","open_command":"p"}}"#),
        None,
    )
    .unwrap();
    assert_eq!(
        plan.updates.get("key_key.chat").map(String::as_str),
        Some("key.keyboard.y")
    );
    assert!(plan
        .conflicts
        .iter()
        .any(|c| c.contains("key_key.back") && c.contains("key_key.chat")));
}

#[test]
fn invalid_profiles_reject_duplicates_and_unsupported_keys() {
    for json in [
        r#"{"controls":{"key_key.back":"s","key_key.left":"s"}}"#,
        r#"{"presets":[{"id":"a","width":800,"height":600,"shortcut":"f8"},{"id":"b","width":800,"height":600,"shortcut":"f8"}]}"#,
        r#"{"presets":[{"id":"a","width":800,"height":600,"shortcut":"f8"},{"id":"a","width":800,"height":600,"shortcut":"f9"}]}"#,
        r#"{"presets":[{"id":"a","width":99,"height":600,"shortcut":"f8"}]}"#,
        r#"{"presets":[{"id":"a","width":800,"height":16385,"shortcut":"f8"}]}"#,
        r#"{"ninjabrain":{"s":"leftcontrol+rightcontrol+n"}}"#,
        r#"{"crafting":{"not_a_real_key":"k"}}"#,
        r#"{"crafting":{"t":"k"}}"#,
    ] {
        assert!(
            parse_profile(json.as_bytes(), &KeySet::fallback()).is_err(),
            "accepted {json}"
        );
    }
}

#[test]
fn injected_vocabulary_extends_offline_fallback_but_rejects_config_syntax() {
    let keys = KeySet::with_discovered(["kp1", "#", "volumeup"]);
    assert!(parse_profile(br#"{"crafting":{"kp1":"s"}}"#, &keys).is_ok());
    assert!(parse_profile(br##"{"crafting":{"#":"s"}}"##, &keys).is_err());
    let options = OptionsDocument {
        lines: vec![],
        bindings: [("key_key.back".into(), "key.keyboard.s".into())].into(),
    };
    let profile = parse_profile(br#"{"controls":{"key_key.back":"volumeup"}}"#, &keys).unwrap();
    assert!(plan_controls(&options, &profile, None).is_err());
}

#[test]
fn custom_chat_keys_create_missing_options_entries() {
    let options = OptionsDocument {
        lines: vec![],
        bindings: [("key_key.back".into(), "key.keyboard.s".into())].into(),
    };
    let profile = parse(r#"{"chat":{"open_chat":"y","open_command":"p"}}"#);
    let plan = plan_controls(&options, &profile, None).unwrap();
    assert_eq!(
        plan.updates.into_iter().collect::<Vec<_>>(),
        [
            ("key_key.chat".into(), "key.keyboard.y".into()),
            ("key_key.command".into(), "key.keyboard.p".into()),
        ]
    );
}

#[test]
fn existing_options_order_sets_preview_and_duplicate_conflict_order() {
    let options = OptionsDocument {
        lines: vec![],
        bindings: [
            ("key_key.drop".into(), "key.keyboard.k".into()),
            ("key_key.back".into(), "key.keyboard.s".into()),
        ]
        .into(),
    };
    let profile = parse(r#"{"crafting":{"s":"k"}}"#);
    let plan = plan_controls(&options, &profile, None).unwrap();
    assert_eq!(
        plan.preview,
        [("s".into(), "k".into(), "key_key.back".into())]
    );
    assert!(plan
        .conflicts
        .iter()
        .any(|item| { item == "emitted key.keyboard.k conflicts: key_key.drop, key_key.back" }));
}

#[test]
fn preset_numeric_and_boolean_strings_match_python_normalization() {
    let profile = parse(
        r#"{"presets":[{"id":"normal","width":"800","height":"600","shortcut":"f8","eye_zoom":"yes"}]}"#,
    );
    assert_eq!(profile.presets[0].width, 800);
    assert_eq!(profile.presets[0].height, 600);
    assert!(profile.presets[0].eye_zoom);
    assert_eq!(
        serde_json::to_value(&profile).unwrap()["presets"][0]["width"],
        800
    );
    assert_eq!(
        serde_json::to_value(&profile).unwrap()["presets"][0]["eye_zoom"],
        true
    );

    let profile = parse(
        r#"{"presets":[{"id":"normal","width":800.0,"height":600.0,"shortcut":"f8","eye_zoom":1.0}]}"#,
    );
    assert_eq!(
        (
            profile.presets[0].width,
            profile.presets[0].height,
            profile.presets[0].eye_zoom
        ),
        (800, 600, true)
    );

    let profile = parse(
        r#"{"presets":[{"id":"normal","width":"100","height":"16384","shortcut":"f8","eye_zoom":"off"}]}"#,
    );
    assert_eq!(
        (
            profile.presets[0].width,
            profile.presets[0].height,
            profile.presets[0].eye_zoom
        ),
        (100, 16384, false)
    );

    for accepted in ["+800", " 800 ", "800.0", "00800", "0_800"] {
        let json = format!(
            r#"{{"presets":[{{"id":"normal","width":"{accepted}","height":600,"shortcut":"f8"}}]}}"#
        );
        assert_eq!(
            parse_profile(json.as_bytes(), &KeySet::fallback())
                .unwrap()
                .presets[0]
                .width,
            800,
            "rejected width {accepted}"
        );
    }

    for value in ["99", "16385", "800.5", "8__00", "800_"] {
        let json = format!(
            r#"{{"presets":[{{"id":"normal","width":"{value}","height":"600","shortcut":"f8"}}]}}"#
        );
        assert!(
            parse_profile(json.as_bytes(), &KeySet::fallback()).is_err(),
            "accepted width {value}"
        );
    }
    assert!(parse_profile(br#"{"presets":[{"id":"normal","width":"800","height":"600","shortcut":"f8","eye_zoom":"sometimes"}]}"#, &KeySet::fallback()).is_err());
    assert!(parse_profile(
        br#"{"presets":[{"id":"normal","width":800.5,"height":600,"shortcut":"f8"}]}"#,
        &KeySet::fallback()
    )
    .is_err());
}
