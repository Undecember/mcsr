//! Guided host prerequisites for installation; inspection never changes host state.
use std::{path::Path, time::Duration};

#[derive(Clone, Copy, Debug, Default)]
pub struct InstallOptions {
    pub observe_only: bool,
    pub yes: bool,
}

trait Host {
    fn run(&mut self, program: &str, args: &[&str], deadline: Duration) -> Result<String, String>;
    fn groups(&mut self, pid: Option<u32>) -> Result<Vec<u32>, String>;
    fn uid(&self) -> u32;
    fn confirm(&mut self, prompt: &str, yes: bool) -> Result<bool, String>;
}

const SCHEMA: &str = "org.gnome.desktop.input-sources";
const FKEY_OPTION: &str = "fkeys:basic_13-24";
const BACKUP_NAME: &str = "keyboard-settings-before-install.json";
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

struct RealHost;
impl Host for RealHost {
    fn run(&mut self, program: &str, args: &[&str], deadline: Duration) -> Result<String, String> {
        let mut command = vec![program.to_owned()];
        command.extend(args.iter().map(|arg| (*arg).to_owned()));
        crate::keyd::run_command(&command, &[], deadline)
            .map(|output| output.stdout.trim().to_owned())
            .map_err(|error| error.to_string())
    }
    fn groups(&mut self, pid: Option<u32>) -> Result<Vec<u32>, String> {
        let path = pid.map_or_else(
            || "/proc/self/status".into(),
            |pid| format!("/proc/{pid}/status"),
        );
        let status =
            std::fs::read_to_string(&path).map_err(|error| format!("read {path}: {error}"))?;
        process_groups(&status)
    }
    fn uid(&self) -> u32 {
        unsafe { libc::geteuid() }
    }
    fn confirm(&mut self, prompt: &str, yes: bool) -> Result<bool, String> {
        use std::io::{IsTerminal, Write};
        eprintln!("{prompt}");
        if yes {
            return Ok(true);
        }
        if !std::io::stdin().is_terminal() {
            eprintln!("No terminal for confirmation; rerun interactively or pass --yes.");
            return Ok(false);
        }
        eprint!("Continue? [y/N] ");
        std::io::stderr()
            .flush()
            .map_err(|error| error.to_string())?;
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .map_err(|error| error.to_string())?;
        Ok(matches!(
            answer.trim().to_ascii_lowercase().as_str(),
            "y" | "yes"
        ))
    }
}

fn valid_login(login: &str) -> bool {
    !login.is_empty()
        && login != "ALL"
        && !login.starts_with('-')
        && login
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
fn number(raw: &str, name: &str) -> Result<u32, String> {
    let value = raw.parse::<u32>().map_err(|_| format!("invalid {name}"))?;
    if raw != value.to_string() {
        return Err(format!("invalid {name}"));
    }
    Ok(value)
}
fn numeric_groups(raw: &str) -> Result<Vec<u32>, String> {
    let groups = raw
        .split_whitespace()
        .map(|raw| number(raw, "group ID"))
        .collect::<Result<Vec<_>, _>>()?;
    if groups.is_empty() {
        return Err("empty group list".into());
    }
    Ok(groups)
}
fn process_groups(status: &str) -> Result<Vec<u32>, String> {
    let groups = status
        .lines()
        .find_map(|line| line.strip_prefix("Groups:"))
        .ok_or("process group list unavailable")?;
    // Linux permits an empty supplementary group list.
    let mut groups = if groups.trim().is_empty() {
        vec![]
    } else {
        numeric_groups(groups)?
    };
    // Socket access uses the filesystem GID as well as supplementary groups.
    if let Some(gids) = status.lines().find_map(|line| line.strip_prefix("Gid:")) {
        let gids = numeric_groups(gids)?;
        if gids.len() != 4 {
            return Err("invalid process GID list".into());
        }
        if !groups.contains(&gids[3]) {
            groups.push(gids[3]);
        }
    }
    Ok(groups)
}
fn load_state(host: &mut dyn Host) -> Result<String, String> {
    host.run(
        "/usr/bin/systemctl",
        &["show", "keyd.service", "--property=LoadState", "--value"],
        PROBE_TIMEOUT,
    )
}
fn service_problem(state: &str) -> Option<String> {
    match state {
        "loaded" => None,
        "not-found" => Some(
            "keyd.service is missing; install the distribution package with sudo apt install keyd"
                .into(),
        ),
        "masked" => Some("keyd.service is masked; review and unmask it before installation".into()),
        other => Some(format!(
            "keyd.service is unavailable (LoadState={other:?}); review the system service"
        )),
    }
}
struct Membership {
    account: bool,
    installer: bool,
    manager: bool,
}
fn membership(host: &mut dyn Host, login: &str) -> Result<Membership, String> {
    let group = host.run("/usr/bin/getent", &["group", "keyd"], PROBE_TIMEOUT)?;
    let fields: Vec<_> = group.split(':').collect();
    if fields.len() != 4 || fields[0] != "keyd" {
        return Err("invalid or missing keyd group record; reinstall the keyd package".into());
    }
    let gid = number(fields[2], "keyd group ID")?;
    let account =
        numeric_groups(&host.run("/usr/bin/id", &["-G", login], PROBE_TIMEOUT)?)?.contains(&gid);
    let installer = host.groups(None)?.contains(&gid);
    let user_unit = format!("user@{}.service", host.uid());
    let pid = host.run(
        "/usr/bin/systemctl",
        &["show", &user_unit, "--property=MainPID", "--value"],
        PROBE_TIMEOUT,
    )?;
    let pid = number(&pid, "user service manager PID")?;
    if pid == 0 {
        return Err(
            "user service manager is unavailable; log into GNOME before installation".into(),
        );
    }
    let manager = host.groups(Some(pid))?.contains(&gid);
    Ok(Membership {
        account,
        installer,
        manager,
    })
}
fn get_setting(host: &mut dyn Host, key: &str) -> Result<String, String> {
    host.run("/usr/bin/gsettings", &["get", SCHEMA, key], PROBE_TIMEOUT)
}
fn set_setting(host: &mut dyn Host, key: &str, value: &str) -> Result<(), String> {
    host.run(
        "/usr/bin/gsettings",
        &["set", SCHEMA, key, value],
        PROBE_TIMEOUT,
    )
    .map(|_| ())
}
// Work with complete serialized array entries, preserving GVariant quoting and
// all unrelated options. Command arguments never pass through a shell.
fn array_entries(raw: &str, annotation: &str) -> Result<Vec<String>, String> {
    let raw = raw
        .trim()
        .strip_prefix(annotation)
        .unwrap_or(raw.trim())
        .trim();
    if raw.len() > 16384 {
        return Err("keyboard settings value is too large".into());
    }
    let content = raw
        .strip_prefix('[')
        .and_then(|raw| raw.strip_suffix(']'))
        .ok_or("invalid keyboard settings array")?;
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0i32;
    let mut start = 0;
    let mut entries = Vec::new();
    for (index, ch) in content.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(delimiter) = quote {
            if ch == '\\' {
                escaped = true;
            } else if ch == delimiter {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err("invalid keyboard settings tuple".into());
                }
            }
            ',' if depth == 0 => {
                let value = content[start..index].trim();
                if value.is_empty() {
                    return Err("invalid keyboard settings entry".into());
                }
                entries.push(value.to_owned());
                start = index + 1;
            }
            _ => {}
        }
    }
    if quote.is_some() || escaped || depth != 0 {
        return Err("invalid keyboard settings quoting".into());
    }
    let last = content[start..].trim();
    if !last.is_empty() {
        entries.push(last.to_owned());
    }
    Ok(entries)
}
fn has_fkeys(raw: &str) -> Result<bool, String> {
    Ok(array_entries(raw, "@as")?
        .iter()
        .any(|value| value == "'fkeys:basic_13-24'" || value == "\"fkeys:basic_13-24\""))
}
fn with_fkeys(raw: &str) -> Result<String, String> {
    let mut entries = array_entries(raw, "@as")?;
    if !has_fkeys(raw)? {
        entries.push(format!("'{FKEY_OPTION}'"));
    }
    Ok(format!("[{}]", entries.join(", ")))
}
fn temporary_sources(raw: &str) -> Result<String, String> {
    let mut entries = array_entries(raw, "@a(ss)")?;
    let normalized: Vec<_> = entries
        .iter()
        .map(|entry| entry.replace(' ', "").replace('"', "'"))
        .collect();
    if let Some(candidate) = ["us", "gb", "de"]
        .into_iter()
        .find(|layout| !normalized.contains(&format!("('xkb','{layout}')")))
    {
        entries.push(format!("('xkb', '{candidate}')"));
    } else {
        // All candidates already exist, so rotate valid current entries instead
        // of inventing a layout or introducing duplicate input sources. The
        // refresh restores the exact original source list and MRU immediately.
        entries.rotate_left(1);
    }
    Ok(format!("[{}]", entries.join(", ")))
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyboardSettings {
    xkb_options: String,
    sources: String,
    mru_sources: String,
    updated_options: String,
}
fn keyboard_snapshot(host: &mut dyn Host) -> Result<KeyboardSettings, String> {
    let xkb_options = get_setting(host, "xkb-options")?;
    let updated_options = with_fkeys(&xkb_options)?;
    let sources = get_setting(host, "sources")?;
    let mru_sources = get_setting(host, "mru-sources")?;
    array_entries(&sources, "@a(ss)")?;
    array_entries(&mru_sources, "@a(ss)")?;
    Ok(KeyboardSettings {
        xkb_options,
        sources,
        mru_sources,
        updated_options,
    })
}
fn read_backup(path: &Path) -> Result<KeyboardSettings, String> {
    let bytes = crate::paths::safe_record(path)
        .map_err(|error| format!("keyboard settings backup: {error}"))?;
    if bytes.len() > 65536 {
        return Err("keyboard settings backup is too large".into());
    }
    let saved: KeyboardSettings = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid keyboard settings backup: {error}"))?;
    if with_fkeys(&saved.xkb_options)? != saved.updated_options || has_fkeys(&saved.xkb_options)? {
        return Err("invalid keyboard settings backup options".into());
    }
    array_entries(&saved.sources, "@a(ss)")?;
    array_entries(&saved.mru_sources, "@a(ss)")?;
    Ok(saved)
}
fn save_backup(data_home: &Path, snapshot: &KeyboardSettings) -> Result<(), String> {
    use std::io::Write;
    let dir = crate::paths::ensure_state(data_home)
        .map_err(|error| format!("private keyboard backup directory: {error}"))?;
    let path = dir.join(BACKUP_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            let prior = read_backup(&path)?;
            if prior.updated_options != snapshot.updated_options {
                return Err("keyboard settings backup belongs to earlier options; restore or review that backup before changing the layout again".into());
            }
            return Ok(());
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let mut file = tempfile::NamedTempFile::new_in(&dir).map_err(|error| error.to_string())?;
    serde_json::to_writer_pretty(&mut file, snapshot).map_err(|error| error.to_string())?;
    file.write_all(b"\n")
        .and_then(|_| file.as_file().sync_all())
        .map_err(|error| error.to_string())?;
    file.persist_noclobber(&path)
        .map_err(|error| format!("save original keyboard settings: {}", error.error))?;
    crate::paths::sync_dir(&dir).map_err(|error| error.to_string())
}
fn refresh_sources(host: &mut dyn Host, sources: &str, mru: &str) -> Result<(), String> {
    let temporary = temporary_sources(sources)?;
    set_setting(host, "sources", &temporary)?;
    // Separate synchronous gsettings processes allow Shell to observe the source
    // list change. A successful live handshake remains the authoritative check.
    std::thread::sleep(Duration::from_millis(100));
    set_setting(host, "sources", sources)?;
    set_setting(host, "mru-sources", mru)
}
fn change_options(
    host: &mut dyn Host,
    options: &str,
    snapshot: &KeyboardSettings,
) -> Result<(), String> {
    let changed = (|| {
        set_setting(host, "xkb-options", options)?;
        refresh_sources(host, &snapshot.sources, &snapshot.mru_sources)
    })();
    if let Err(error) = changed {
        let mut rollback = Vec::new();
        for (key, value) in [
            ("xkb-options", &snapshot.xkb_options),
            ("sources", &snapshot.sources),
            ("mru-sources", &snapshot.mru_sources),
        ] {
            if let Err(problem) = set_setting(host, key, value) {
                rollback.push(format!("{key}: {problem}"));
            }
        }
        if let Err(problem) = refresh_sources(host, &snapshot.sources, &snapshot.mru_sources) {
            rollback.push(problem);
        }
        if rollback.is_empty() {
            return Err(format!(
                "keyboard settings change failed and original settings were restored: {error}"
            ));
        }
        return Err(format!(
            "keyboard settings change failed: {error}; restoration failed: {}",
            rollback.join("; ")
        ));
    }
    Ok(())
}

/// Reports live-mode prerequisites without changing package, account or settings state.
pub fn inspect(login: &str) -> Vec<String> {
    inspect_with(&mut RealHost, login)
}
fn inspect_with(host: &mut dyn Host, login: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if !valid_login(login) {
        return vec!["invalid desktop login".into()];
    }
    match load_state(host) {
        Ok(state) => {
            if let Some(problem) = service_problem(&state) {
                issues.push(problem);
            }
        }
        Err(error) => issues.push(format!("cannot inspect keyd.service: {error}")),
    }
    match membership(host, login) {
        Ok(member) => {
            if !member.account {
                issues.push("desktop account is not a member of the keyd group; run install/repair for guided setup".into());
            }
            if !member.installer {
                issues.push("current session lacks the keyd group; reboot after adding membership, then run repair".into());
            }
            if !member.manager {
                issues.push(
                    "user service manager lacks the keyd group; reboot, then run repair".into(),
                );
            }
        }
        Err(error) => issues.push(format!("cannot verify keyd socket permissions: {error}")),
    }
    match get_setting(host, "xkb-options").and_then(|raw| has_fkeys(&raw)) {
        Ok(true) => {}
        Ok(false) => issues.push(
            "F24 routing mapping is not configured; run install/repair to offer fkeys:basic_13-24"
                .into(),
        ),
        Err(error) => issues.push(format!("cannot inspect F24 keyboard mapping: {error}")),
    }
    issues
}
/// Offers the fixed host prerequisite changes. Some(reason) requests a resumable
/// observe-only fallback rather than claiming live setup completed.
pub fn prepare(
    login: &str,
    data_home: &Path,
    options: InstallOptions,
) -> Result<Option<String>, String> {
    let actual = crate::setup_helper::desktop_login_for_uid(unsafe { libc::geteuid() })?;
    if login != actual {
        return Err("desktop login differs from invoking account".into());
    }
    prepare_with(&mut RealHost, login, data_home, options)
}
fn prepare_with(
    host: &mut dyn Host,
    login: &str,
    data_home: &Path,
    options: InstallOptions,
) -> Result<Option<String>, String> {
    if !valid_login(login) {
        return Err("invalid desktop login".into());
    }
    let state = load_state(host)?;
    if state == "not-found" {
        if !host.confirm("The distribution keyd service is required. Install the keyd package through authenticated apt?",options.yes)? {
            return Err("keyd.service is missing; run sudo apt install keyd, then rerun repair".into());
        }
        host.run(
            "/usr/bin/pkexec",
            &["/usr/bin/apt-get", "install", "--yes", "keyd"],
            Duration::from_secs(600),
        )?;
        if let Some(problem) = service_problem(&load_state(host)?) {
            return Err(problem);
        }
    } else if let Some(problem) = service_problem(&state) {
        return Err(problem);
    }
    if options.observe_only {
        return Ok(None);
    }
    let mut pending = Vec::new();
    let mut member = membership(host, login)?;
    if !member.account {
        if host.confirm("Live keyd needs access to its privileged input socket. Add your account to the keyd group? Membership persists after toolkit uninstall.",options.yes)? {
            host.run("/usr/bin/pkexec", &["/usr/sbin/usermod","-aG","keyd",login],Duration::from_secs(120))?;
            member=membership(host,login)?;
            if !member.account { return Err("keyd group membership was not saved; review the account before retrying repair".into()); }
        } else {
            pending.push("live keyd needs account membership in the keyd group; rerun repair to accept guided setup".into());
        }
    }
    if member.account && (!member.installer || !member.manager) {
        let owner = if !member.manager {
            "user service manager"
        } else {
            "current session"
        };
        pending.push(format!(
            "{owner} still lacks the keyd group; reboot required, then run repair"
        ));
    }
    let current = get_setting(host, "xkb-options")?;
    if !has_fkeys(&current)? {
        if host.confirm("Live routing needs F13–F24 key names. Enable fkeys:basic_13-24? This changes the extended function-key layout; original settings will be backed up and can be restored with mcsr-toolkit restore-keyboard-settings.",options.yes)? {
            let snapshot=keyboard_snapshot(host)?;
            save_backup(data_home,&snapshot)?;
            change_options(host,&snapshot.updated_options,&snapshot)?;
        } else {
            pending.push("F24 routing mapping was declined; rerun repair to enable fkeys:basic_13-24".into());
        }
    } else if pending.is_empty() {
        // Shell can retain an old keymap even when dconf already contains the
        // option. The caller has stopped the input producer before this point.
        // Refresh current source choices without claiming or replacing an
        // original-settings backup for an option we did not enable.
        let snapshot = keyboard_snapshot(host)?;
        change_options(host, &snapshot.xkb_options, &snapshot)?;
    }
    if pending.is_empty() {
        Ok(None)
    } else {
        Ok(Some(pending.join("; ")))
    }
}
/// Restores only our known keyboard option change, preserving input sources and
/// refusing to overwrite options the user edited after installation.
pub fn restore_keyboard_settings(data_home: &Path) -> Result<(), String> {
    restore_with(&mut RealHost, data_home)
}
fn restore_with(host: &mut dyn Host, data_home: &Path) -> Result<(), String> {
    let command = host.run(
        "/usr/bin/systemctl",
        &["--user", "show", "-P", "ExecStart", "mcsr-toolkit.service"],
        PROBE_TIMEOUT,
    ).map_err(|error| format!("cannot verify live routing is disabled before restoring keyboard settings: {error}"))?;
    if command.contains("--enable-live-keyd") {
        return Err("live routing is still configured; run mcsr-toolkit repair --observe-only before restore-keyboard-settings".into());
    }
    let path = crate::paths::state_dir(data_home).join(BACKUP_NAME);
    let saved = read_backup(&path)?;
    let current = keyboard_snapshot(host)?;
    if array_entries(&current.xkb_options, "@as")? == array_entries(&saved.xkb_options, "@as")? {
        return Ok(());
    }
    if array_entries(&current.xkb_options, "@as")? != array_entries(&saved.updated_options, "@as")?
    {
        return Err("keyboard options changed after installation; review the backup before restoring to avoid discarding your changes".into());
    }
    change_options(host, &saved.xkb_options, &current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeMap, fs};

    struct Fake {
        load: String,
        account: bool,
        installer: bool,
        manager: bool,
        accept: bool,
        settings: BTreeMap<String, String>,
        commands: Vec<Vec<String>>,
        fail_source_once: bool,
        service_command: String,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                load: "loaded".into(),
                account: true,
                installer: true,
                manager: true,
                accept: true,
                commands: vec![],
                fail_source_once: false,
                service_command: "/usr/local/bin/mcsr-toolkit serve".into(),
                settings: [
                    ("xkb-options", "['fkeys:basic_13-24']"),
                    ("sources", "[('xkb', 'us')]"),
                    ("mru-sources", "@a(ss) []"),
                ]
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
            }
        }
    }
    impl Host for Fake {
        fn run(
            &mut self,
            program: &str,
            args: &[&str],
            _deadline: Duration,
        ) -> Result<String, String> {
            let mut command = vec![program.to_owned()];
            command.extend(args.iter().map(|s| (*s).to_owned()));
            self.commands.push(command);
            match program {
                "/usr/bin/systemctl" if args.iter().any(|s| s.contains("LoadState")) => {
                    Ok(self.load.clone())
                }
                "/usr/bin/systemctl" if args.contains(&"--user") => {
                    Ok(self.service_command.clone())
                }
                "/usr/bin/systemctl" => Ok("1234".into()),
                "/usr/bin/getent" => Ok("keyd:x:973:".into()),
                "/usr/bin/id" => Ok(if self.account { "1000 973" } else { "1000" }.into()),
                "/usr/bin/pkexec" if args.first() == Some(&"/usr/bin/apt-get") => {
                    self.load = "loaded".into();
                    Ok(String::new())
                }
                "/usr/bin/pkexec" if args.first() == Some(&"/usr/sbin/usermod") => {
                    self.account = true;
                    Ok(String::new())
                }
                "/usr/bin/gsettings" if args[0] == "get" => Ok(self.settings[args[2]].clone()),
                "/usr/bin/gsettings" if args[0] == "set" => {
                    if args[2] == "sources" && self.fail_source_once {
                        self.fail_source_once = false;
                        return Err("injected sources failure".into());
                    }
                    self.settings.insert(args[2].into(), args[3].into());
                    Ok(String::new())
                }
                _ => panic!("unexpected command {program} {args:?}"),
            }
        }
        fn groups(&mut self, pid: Option<u32>) -> Result<Vec<u32>, String> {
            let member = if pid.is_none() {
                self.installer
            } else {
                self.manager
            };
            Ok(if member { vec![1000, 973] } else { vec![1000] })
        }
        fn uid(&self) -> u32 {
            1000
        }
        fn confirm(&mut self, _prompt: &str, yes: bool) -> Result<bool, String> {
            Ok(yes || self.accept)
        }
    }
    fn mutates(fake: &Fake) -> bool {
        fake.commands
            .iter()
            .any(|c| c[0] == "/usr/bin/pkexec" || (c[0] == "/usr/bin/gsettings" && c[1] == "set"))
    }
    fn prepare_fake(fake: &mut Fake, root: &Path) -> Result<Option<String>, String> {
        prepare_with(fake, "alice", root, InstallOptions::default())
    }

    #[test]
    fn effective_primary_group_counts_for_socket_access() {
        assert_eq!(
            process_groups("Gid:\t1000 973 973 973\nGroups:\t1000\n").unwrap(),
            vec![1000, 973]
        );
    }
    #[test]
    fn malformed_numeric_ids_are_rejected() {
        assert!(number("0973", "group ID").is_err());
        assert!(numeric_groups("1000 invalid").is_err());
        assert_eq!(
            process_groups("Name: systemd\nGroups:\t1000 973\n").unwrap(),
            vec![1000, 973]
        );
        assert!(process_groups("Name: systemd\n").is_err());
    }
    #[test]
    fn invalid_login_never_reaches_commands() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake::default();
        assert!(prepare_with(&mut fake, "-root", root.path(), InstallOptions::default()).is_err());
        assert!(fake.commands.is_empty());
    }
    #[test]
    fn unsafe_keyboard_backup_blocks_layout_mutation() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let dir = crate::paths::ensure_state(root.path()).unwrap();
        let foreign = root.path().join("foreign.json");
        fs::write(&foreign, b"untouched").unwrap();
        symlink(&foreign, dir.join(BACKUP_NAME)).unwrap();
        let mut fake = Fake::default();
        fake.settings.insert("xkb-options".into(), "@as []".into());
        assert!(prepare_fake(&mut fake, root.path())
            .unwrap_err()
            .contains("unsafe"));
        assert!(!mutates(&fake));
        assert_eq!(fs::read(foreign).unwrap(), b"untouched");
    }
    #[test]
    fn old_backup_is_preserved_when_new_options_conflict() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake::default();
        fake.settings.insert("xkb-options".into(), "@as []".into());
        prepare_fake(&mut fake, root.path()).unwrap();
        let backup = root.path().join("mcsr-toolkit/rust-v1").join(BACKUP_NAME);
        let original = fs::read(&backup).unwrap();
        fake.settings
            .insert("xkb-options".into(), "['caps:escape']".into());
        fake.commands.clear();
        assert!(prepare_fake(&mut fake, root.path())
            .unwrap_err()
            .contains("earlier options"));
        assert!(!mutates(&fake));
        assert_eq!(fs::read(backup).unwrap(), original);
    }
    #[test]
    fn existing_fkey_option_refreshes_current_sources_without_backup() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake::default();
        let original = fake.settings.clone();
        assert_eq!(prepare_fake(&mut fake, root.path()).unwrap(), None);
        assert_eq!(fake.settings, original);
        assert!(fake
            .commands
            .iter()
            .any(|command| command[0] == "/usr/bin/gsettings"
                && command[1] == "set"
                && command[3] == "sources"));
        assert!(!fake
            .commands
            .iter()
            .any(|command| command[0] == "/usr/bin/pkexec"));
        assert!(!root
            .path()
            .join("mcsr-toolkit/rust-v1")
            .join(BACKUP_NAME)
            .exists());
    }
    #[test]
    fn existing_fkeys_refreshes_when_all_temporary_layouts_already_exist() {
        let root = tempfile::tempdir().unwrap();
        let dir = crate::paths::ensure_state(root.path()).unwrap();
        let backup = dir.join(BACKUP_NAME);
        fs::write(&backup, b"existing backup must be preserved").unwrap();
        let mut fake = Fake::default();
        let sources = "[('xkb', 'us'), ('xkb', 'gb'), ('xkb', 'de')]";
        fake.settings.insert("sources".into(), sources.into());
        fake.settings.insert(
            "mru-sources".into(),
            "[('xkb', 'de'), ('xkb', 'us'), ('xkb', 'gb')]".into(),
        );
        let original = fake.settings.clone();
        assert_eq!(prepare_fake(&mut fake, root.path()).unwrap(), None);
        assert_eq!(fake.settings, original);
        assert_eq!(
            fs::read(backup).unwrap(),
            b"existing backup must be preserved"
        );
        assert!(fake
            .commands
            .iter()
            .any(|command| command[0] == "/usr/bin/gsettings"
                && command[1] == "set"
                && command[3] == "sources"
                && command[4] != sources));
    }
    #[test]
    fn existing_fkey_option_refresh_failure_restores_current_settings() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            fail_source_once: true,
            ..Fake::default()
        };
        let original = fake.settings.clone();
        assert!(prepare_fake(&mut fake, root.path())
            .unwrap_err()
            .contains("injected sources failure"));
        assert_eq!(fake.settings, original);
        assert!(!root
            .path()
            .join("mcsr-toolkit/rust-v1")
            .join(BACKUP_NAME)
            .exists());
    }
    #[test]
    fn missing_package_is_offered_and_verified_before_observe_install() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            load: "not-found".into(),
            ..Fake::default()
        };
        assert_eq!(
            prepare_with(
                &mut fake,
                "alice",
                root.path(),
                InstallOptions {
                    observe_only: true,
                    yes: false
                }
            )
            .unwrap(),
            None
        );
        assert!(fake.commands.iter().any(|c| c
            == &[
                "/usr/bin/pkexec",
                "/usr/bin/apt-get",
                "install",
                "--yes",
                "keyd"
            ]));
        assert!(!fake.commands.iter().any(|c| c[0] == "/usr/bin/gsettings"));
    }
    #[test]
    fn masked_service_is_never_unmasked_or_reinstalled() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            load: "masked".into(),
            ..Fake::default()
        };
        assert!(prepare_fake(&mut fake, root.path())
            .unwrap_err()
            .contains("masked"));
        assert!(!mutates(&fake));
    }
    #[test]
    fn declined_package_is_actionable_and_does_not_mutate() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            load: "not-found".into(),
            accept: false,
            ..Fake::default()
        };
        assert!(prepare_fake(&mut fake, root.path())
            .unwrap_err()
            .contains("apt install keyd"));
        assert!(!mutates(&fake));
    }
    #[test]
    fn group_addition_leaves_stale_session_pending() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            account: false,
            installer: false,
            manager: false,
            ..Fake::default()
        };
        let reason = prepare_fake(&mut fake, root.path()).unwrap().unwrap();
        assert!(reason.contains("reboot"));
        assert!(reason.contains("repair"));
        assert!(fake.commands.iter().any(|c| c
            == &[
                "/usr/bin/pkexec",
                "/usr/sbin/usermod",
                "-aG",
                "keyd",
                "alice"
            ]));
    }
    #[test]
    fn refreshed_shell_does_not_hide_stale_user_manager() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            manager: false,
            ..Fake::default()
        };
        let reason = prepare_fake(&mut fake, root.path()).unwrap().unwrap();
        assert!(reason.contains("user service manager"));
        assert!(reason.contains("reboot"));
        assert!(!mutates(&fake));
    }
    #[test]
    fn declined_membership_keeps_observe_only_available() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            account: false,
            accept: false,
            ..Fake::default()
        };
        let reason = prepare_fake(&mut fake, root.path()).unwrap().unwrap();
        assert!(reason.contains("keyd group"));
        assert!(!mutates(&fake));
    }
    #[test]
    fn doctor_reports_distinct_live_issues_without_mutation() {
        let mut fake = Fake {
            load: "not-found".into(),
            account: false,
            installer: false,
            manager: false,
            ..Fake::default()
        };
        fake.settings.insert("xkb-options".into(), "@as []".into());
        let issues = inspect_with(&mut fake, "alice");
        assert!(issues.iter().any(|s| s.contains("keyd.service")));
        assert!(issues.iter().any(|s| s.contains("account")));
        assert!(issues.iter().any(|s| s.contains("user service manager")));
        assert!(issues.iter().any(|s| s.contains("F24")));
        assert!(!mutates(&fake));
    }
    #[test]
    fn keyboard_offer_preserves_options_sources_and_original_backup() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake::default();
        fake.settings
            .insert("xkb-options".into(), "['caps:escape']".into());
        let sources = fake.settings["sources"].clone();
        let mru = fake.settings["mru-sources"].clone();
        assert_eq!(prepare_fake(&mut fake, root.path()).unwrap(), None);
        assert!(fake.settings["xkb-options"].contains("caps:escape"));
        assert!(fake.settings["xkb-options"].contains("fkeys:basic_13-24"));
        assert_eq!(fake.settings["sources"], sources);
        assert_eq!(fake.settings["mru-sources"], mru);
        let backup = root
            .path()
            .join("mcsr-toolkit/rust-v1/keyboard-settings-before-install.json");
        let original = fs::read(&backup).unwrap();
        assert!(String::from_utf8_lossy(&original).contains("caps:escape"));
        assert_eq!(prepare_fake(&mut fake, root.path()).unwrap(), None);
        assert_eq!(fs::read(backup).unwrap(), original);
    }
    #[test]
    fn keyboard_refresh_failure_restores_options_and_sources() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake {
            fail_source_once: true,
            ..Fake::default()
        };
        fake.settings.insert("xkb-options".into(), "@as []".into());
        let original = fake.settings.clone();
        let error = prepare_fake(&mut fake, root.path()).unwrap_err();
        assert!(error.contains("injected sources failure"));
        assert_eq!(fake.settings, original);
    }
    #[test]
    fn restoration_requires_live_service_configuration_disabled() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake::default();
        fake.settings.insert("xkb-options".into(), "@as []".into());
        prepare_fake(&mut fake, root.path()).unwrap();
        fake.service_command = "{ path=/usr/local/bin/mcsr-toolkit ; argv[]=/usr/local/bin/mcsr-toolkit serve --enable-live-keyd ; }".into();
        fake.commands.clear();
        let original = fake.settings.clone();
        let error = restore_with(&mut fake, root.path()).unwrap_err();
        assert!(error.contains("repair --observe-only"));
        assert_eq!(fake.settings, original);
        assert!(!mutates(&fake));
    }
    #[test]
    fn restoration_refuses_later_user_options_changes() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake::default();
        fake.settings.insert("xkb-options".into(), "@as []".into());
        prepare_fake(&mut fake, root.path()).unwrap();
        fake.settings.insert(
            "xkb-options".into(),
            "['caps:escape','fkeys:basic_13-24']".into(),
        );
        fake.commands.clear();
        assert!(restore_with(&mut fake, root.path())
            .unwrap_err()
            .contains("changed"));
        assert!(!mutates(&fake));
    }
    #[test]
    fn restoration_preserves_current_input_source_choices() {
        let root = tempfile::tempdir().unwrap();
        let mut fake = Fake::default();
        fake.settings.insert("xkb-options".into(), "@as []".into());
        prepare_fake(&mut fake, root.path()).unwrap();
        fake.settings
            .insert("sources".into(), "[('ibus', 'hangul')]".into());
        fake.settings
            .insert("mru-sources".into(), "[('ibus', 'hangul')]".into());
        restore_with(&mut fake, root.path()).unwrap();
        assert_eq!(fake.settings["xkb-options"], "@as []");
        assert_eq!(fake.settings["sources"], "[('ibus', 'hangul')]");
        assert_eq!(fake.settings["mru-sources"], "[('ibus', 'hangul')]");
    }
}
