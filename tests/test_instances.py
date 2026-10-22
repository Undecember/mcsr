import json
from pathlib import Path

from mcsr_toolkit.instances import ProcessInfo, discover_instances, locate_launcher_root


def make_instance(root: Path, name: str, *, options: bool = True, legacy: bool = False) -> Path:
    instance = root / "instances" / name
    game = instance / (".minecraft" if legacy else "minecraft")
    game.mkdir(parents=True)
    (instance / "instance.json").write_text(json.dumps({"id": name}))
    if options:
        (game / "options.txt").write_text("key_key.back:key.keyboard.s\n")
    return instance


def test_locates_managed_launcher_root(tmp_path: Path) -> None:
    root = tmp_path / "install" / "launcher"
    make_instance(root, "ranked")

    assert locate_launcher_root(tmp_path / "install", None, []) == root.resolve()


def test_locates_running_launcher_working_directory(tmp_path: Path) -> None:
    root = tmp_path / "custom" / "launcher"
    make_instance(root, "ranked")

    assert (
        locate_launcher_root(tmp_path / "missing", None, [ProcessInfo(tmp_path / "custom")])
        == root.resolve()
    )


def test_configured_root_takes_precedence(tmp_path: Path) -> None:
    managed = tmp_path / "managed" / "launcher"
    chosen = tmp_path / "chosen" / "launcher"
    make_instance(managed, "one")
    make_instance(chosen, "two")

    assert locate_launcher_root(tmp_path / "managed", chosen, []) == chosen.resolve()


def test_unknown_launcher_root_is_not_guessed(tmp_path: Path) -> None:
    assert locate_launcher_root(tmp_path / "missing", None, []) is None


def test_discovers_all_managed_instances_and_preserves_missing_options(tmp_path: Path) -> None:
    root = tmp_path / "launcher"
    make_instance(root, "ranked")
    make_instance(root, "practice", legacy=True)
    make_instance(root, "new", options=False)
    unrelated = root / "instances" / "unrelated"
    unrelated.mkdir(parents=True)

    found = discover_instances(root)

    assert [item.id for item in found] == ["new", "practice", "ranked"]
    assert found[0].problem == "options.txt is missing"
    assert found[1].options_path == root / "instances" / "practice" / ".minecraft" / "options.txt"
    assert found[2].problem is None


def test_does_not_follow_instance_symlink_outside_launcher(tmp_path: Path) -> None:
    root = tmp_path / "launcher"
    external = make_instance(tmp_path / "other", "escape")
    (root / "instances").mkdir(parents=True)
    (root / "instances" / "escape").symlink_to(external, target_is_directory=True)

    assert discover_instances(root) == []
