from pathlib import Path

import pytest

from mcsr_toolkit.bindings import plan_controls
from mcsr_toolkit.models import Profile
from mcsr_toolkit.options import parse_options, render_options

FIXTURE = Path(__file__).parent / "fixtures" / "options-1.16.txt"


def test_render_changes_only_affected_control_line() -> None:
    original = FIXTURE.read_text()
    document = parse_options(original)
    plan = plan_controls(document, Profile(crafting={"s": "k"}), None)

    assert render_options(document, plan) == original.replace(
        "key_key.back:key.keyboard.s", "key_key.back:key.keyboard.k"
    )


def test_render_preserves_crlf_line_endings() -> None:
    original = "version:2586\r\nkey_key.back:key.keyboard.s\r\ngamma:1.0\r\n"
    document = parse_options(original)

    assert render_options(
        document, plan_controls(document, Profile(crafting={"s": "k"}), None)
    ) == ("version:2586\r\nkey_key.back:key.keyboard.k\r\ngamma:1.0\r\n")


def test_duplicate_control_entry_is_rejected() -> None:
    with pytest.raises(ValueError, match="duplicate.*key_key.back"):
        parse_options("key_key.back:key.keyboard.s\nkey_key.back:key.keyboard.w\n")
