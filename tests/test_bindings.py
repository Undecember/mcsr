from pathlib import Path

from mcsr_toolkit.bindings import plan_controls
from mcsr_toolkit.models import Profile
from mcsr_toolkit.options import parse_options, render_options

FIXTURE = Path(__file__).parent / "fixtures" / "options-1.16.txt"


def test_crafting_target_becomes_minecraft_movement_binding() -> None:
    document = parse_options(FIXTURE.read_text())
    plan = plan_controls(
        document,
        Profile(controls={"key_key.back": "s"}, crafting={"s": "k"}),
        None,
    )

    assert plan.updates == {"key_key.back": "key.keyboard.k"}
    assert ("s", "k", "key_key.back") in plan.preview
    assert plan.conflicts == []


def test_other_controls_affected_by_remap_are_updated() -> None:
    document = parse_options(FIXTURE.read_text())
    plan = plan_controls(document, Profile(crafting={"s": "k", "e": "p"}), None)

    assert plan.updates == {
        "key_key.back": "key.keyboard.k",
        "key_key.inventory": "key.keyboard.p",
    }


def test_new_duplicate_emitted_control_key_is_reported() -> None:
    document = parse_options("key_key.back:key.keyboard.s\nkey_key.drop:key.keyboard.k\n")
    plan = plan_controls(document, Profile(crafting={"s": "k"}), None)

    assert any("key_key.back" in item and "key_key.drop" in item for item in plan.conflicts)


def test_repeat_apply_and_target_change_use_original_physical_key() -> None:
    original = parse_options(FIXTURE.read_text())
    first = plan_controls(original, Profile(crafting={"s": "k"}), None)
    edited = parse_options(render_options(original, first))

    second = plan_controls(edited, Profile(crafting={"s": "k"}), first.next_state)
    changed = plan_controls(edited, Profile(crafting={"s": "j"}), first.next_state)

    assert second.updates == {}
    assert render_options(edited, second) == render_options(original, first)
    assert changed.updates == {"key_key.back": "key.keyboard.j"}


def test_custom_chat_entry_controls_follow_physical_events_and_detect_conflicts() -> None:
    document = parse_options(
        'key_key.chat:key.keyboard.t\nkey_key.command:key.keyboard.slash\n'
        'key_key.back:key.keyboard.y\n'
    )
    profile = Profile(chat={'open_chat': 'y', 'open_command': 'p'})
    plan = plan_controls(document, profile, None)
    assert plan.updates['key_key.chat'] == 'key.keyboard.y'
    assert plan.updates['key_key.command'] == 'key.keyboard.p'
    assert any('key_key.back' in conflict and 'key_key.chat' in conflict
               for conflict in plan.conflicts)


def test_missing_custom_chat_entry_controls_are_added() -> None:
    document = parse_options('key_key.back:key.keyboard.s\n')
    plan = plan_controls(document, Profile(chat={'open_chat': 'y', 'open_command': 'p'}), None)
    rendered = render_options(document, plan)
    assert 'key_key.chat:key.keyboard.y\n' in rendered
    assert 'key_key.command:key.keyboard.p\n' in rendered


def test_keyd_names_roundtrip_canonical_minecraft_names() -> None:
    aliases = {
        'dot': 'period', 'leftbrace': 'left.bracket', 'rightbrace': 'right.bracket',
        'grave': 'grave.accent', 'pageup': 'page.up', 'pagedown': 'page.down',
        'capslock': 'caps.lock', 'kp1': 'keypad.1', 'kpdot': 'keypad.decimal',
        'leftmeta': 'left.win', 'sysrq': 'print.screen',
    }
    for keyd_key, minecraft_key in aliases.items():
        document = parse_options('key_key.back:key.keyboard.s\n')
        first = plan_controls(document, Profile(crafting={'s': keyd_key}), None)
        assert first.updates == {'key_key.back': f'key.keyboard.{minecraft_key}'}
        reverse = plan_controls(
            parse_options(f'key_key.back:key.keyboard.{minecraft_key}\n'),
            Profile(crafting={keyd_key: 'h'}), None,
        )
        assert reverse.updates == {'key_key.back': 'key.keyboard.h'}


def test_keyd_key_without_minecraft_equivalent_is_rejected() -> None:
    document = parse_options('key_key.back:key.keyboard.s\n')
    import pytest
    with pytest.raises(ValueError, match='unsupported Minecraft key'):
        plan_controls(document, Profile(controls={'key_key.back': 'volumeup'}), None)


def test_unchanged_chat_binding_still_conflicts_with_another_control() -> None:
    document = parse_options(
        'key_key.chat:key.keyboard.y\nkey_key.command:key.keyboard.p\n'
        'key_key.back:key.keyboard.y\n'
    )
    plan = plan_controls(document, Profile(chat={'open_chat': 'y', 'open_command': 'p'}), None)
    assert plan.updates == {}
    assert any('key_key.back' in conflict and 'key_key.chat' in conflict
               for conflict in plan.conflicts)
