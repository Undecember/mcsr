from pathlib import Path


def test_python_policy_templates_match_retained_helpers_and_guidance() -> None:
    keyd = Path("packaging/mcsr-toolkit-keyd.sudoers")
    maccel = Path("packaging/mcsr-toolkit-maccel.sudoers")
    assert keyd.read_text().splitlines() == [
        "<desktop-user> ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-keyd clear"
    ]
    assert maccel.read_text().splitlines() == [
        "# Replace @DESKTOP_USER@ with the reviewed desktop login before validation.",
        "@DESKTOP_USER@ ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit-maccel eye, /usr/local/libexec/mcsr-toolkit-maccel restore",
    ]
    assert "[sudoers template](packaging/mcsr-toolkit-keyd.sudoers)" in Path(
        "README.md"
    ).read_text()
    assert "[exact sudoers template](packaging/mcsr-toolkit-maccel.sudoers)" in Path(
        "README.md"
    ).read_text()
    assert "[sudoers template](../packaging/mcsr-toolkit-maccel.sudoers)" in Path(
        "integration/resize-wayland.md"
    ).read_text()


def test_rust_policy_templates_match_installer_commands() -> None:
    keyd = Path("packaging/mcsr-toolkit-rust-keyd.sudoers")
    maccel = Path("packaging/mcsr-toolkit-rust-maccel.sudoers")
    assert keyd.read_text().splitlines() == [
        "@DESKTOP_USER@ ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit privileged keyd clear"
    ]
    assert maccel.read_text().splitlines() == [
        "@DESKTOP_USER@ ALL=(root) NOPASSWD: /usr/local/libexec/mcsr-toolkit privileged maccel eye, /usr/local/libexec/mcsr-toolkit privileged maccel restore"
    ]
    installer = Path("src/install.rs").read_text()
    assert 'include_str!("../packaging/mcsr-toolkit-rust-keyd.sudoers")' in installer
    assert 'include_str!("../packaging/mcsr-toolkit-rust-maccel.sudoers")' in installer
    readme = Path("README.md").read_text()
    assert "[keyd](packaging/mcsr-toolkit-rust-keyd.sudoers)" in readme
    assert "[maccel](packaging/mcsr-toolkit-rust-maccel.sudoers)" in readme
