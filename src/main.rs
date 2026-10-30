use std::{
    env,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
};

fn maccel_binary() -> PathBuf {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join("maccel"))
        .find(|path| {
            std::fs::metadata(path).is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
        .unwrap_or_else(|| PathBuf::from("/usr/bin/maccel"))
}

async fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [mode, rest @ ..] if mode == "serve" => {
            let mut port = 8765;
            let mut enable_live_keyd = false;
            let mut index = 0;
            while index < rest.len() {
                match rest[index].as_str() {
                    "--enable-live-keyd" if !enable_live_keyd => enable_live_keyd = true,
                    "--port" if index + 1 < rest.len() => {
                        port = rest[index + 1].parse::<u16>().map_err(|_| "invalid port")?;
                        index += 1;
                    }
                    _ => return Err("unexpected serve argument".into()),
                }
                index += 1;
            }
            let home = env::var_os("HOME").ok_or("HOME is required")?;
            let data_home = env::var_os("XDG_DATA_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| Path::new(&home).join(".local/share"));
            let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    std::path::PathBuf::from(format!("/run/user/{}", unsafe { libc::geteuid() }))
                });
            let config = mcsr_toolkit::runtime::RuntimeConfig {
                runtime_dir,
                managed_dir: data_home.join("mcsr-toolkit"),
                data_home,
                port,
                enable_live_keyd,
                maccel_binary: maccel_binary(),
            };
            let mut runtime = mcsr_toolkit::runtime::ServiceRuntime::start(config)
                .await
                .map_err(|e| e.to_string())?;
            runtime.run().await.map_err(|e| e.to_string())
        }
        [mode] if mode == "restore-keyboard-settings" => {
            let home = env::var_os("HOME").ok_or("HOME is required")?;
            let data_home = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| Path::new(&home).join(".local/share"));
            mcsr_toolkit::install_prerequisites::restore_keyboard_settings(&data_home)?;
            println!("Original extended function-key settings restored. Live routing may require setup again.");
            Ok(())
        }
        [mode, rest @ ..]
            if ["install", "repair", "uninstall", "doctor"].contains(&mode.as_str()) =>
        {
            use mcsr_toolkit::install::{self, Artifact, RealSystem};
            use mcsr_toolkit::install_prerequisites::InstallOptions;
            let mut options = InstallOptions::default();
            if ["doctor", "uninstall"].contains(&mode.as_str()) && !rest.is_empty() {
                return Err(format!("{mode} takes no setup flags"));
            }
            for arg in rest {
                match arg.as_str() {
                    "--observe-only" if !options.observe_only => options.observe_only = true,
                    "--yes" if !options.yes => options.yes = true,
                    _ => return Err(format!("unexpected {mode} argument: {arg}")),
                }
            }
            let system = RealSystem::discover()
                .map_err(|e| e.to_string())?
                .with_install_options(options);
            match mode.as_str() {
                "doctor" => {
                    use install::System;
                    let report = install::preflight(&system);
                    let live_status = report
                        .live_problems
                        .is_empty()
                        .then(|| system.live_service_ready());
                    // An active managed keyd config is expected while live mode is healthy.
                    let problems: Vec<_> = report
                        .problems
                        .iter()
                        .filter(|problem| {
                            !(problem.as_str() == "keyd recovery pending"
                                && matches!(live_status.as_ref(), Some(Ok(true))))
                        })
                        .collect();
                    if problems.is_empty() {
                        println!("Installation safety checks passed; prerequisites and live readiness follow");
                    } else {
                        for problem in problems {
                            println!("Preflight: {problem}");
                        }
                    }
                    for observation in &report.observations {
                        println!("Observed: {observation}");
                    }
                    if let Some(live_status) = live_status {
                        match live_status {
                            Ok(true) => println!("Live mode ready: extension connected, keyd available, recovery clear"),
                            Ok(false) => println!("Live prerequisites satisfied; live startup has not been verified"),
                            Err(error) => println!("Live mode pending: {error}"),
                        }
                    } else {
                        for problem in &report.live_problems {
                            println!("Live mode pending: {problem}");
                        }
                    }
                    if report.repair_pending
                        && !report
                            .observations
                            .iter()
                            .any(|observation| observation.contains("GNOME extension staged"))
                    {
                        println!("Interrupted installation: run repair");
                    }
                    Ok(())
                }
                "uninstall" => install::uninstall(&system).map_err(|e| e.to_string()),
                "install" | "repair" => {
                    let bytes = std::fs::read(env::current_exe().map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                    let artifact = Artifact::from_bytes(bytes).map_err(|e| e.to_string())?;
                    if mode == "repair" {
                        install::repair(&system, &artifact)
                    } else {
                        install::install(&system, &artifact)
                    }
                    .map_err(|e| e.to_string())
                }
                _ => unreachable!(),
            }
        }
        [privileged, keyd, ..] if privileged == "privileged" && keyd == "keyd" => {
            const HELPER: &str = "/usr/local/libexec/mcsr-toolkit";
            if unsafe { libc::geteuid() } != 0
                || env::current_exe().map_err(|e| e.to_string())? != Path::new(HELPER)
            {
                return Err(
                    "privileged keyd mode requires the installed root-owned executable".into(),
                );
            }
            let mut input = Vec::new();
            if args.as_slice() != ["privileged", "keyd", "status"] {
                std::io::stdin()
                    .take(mcsr_toolkit::keyd_helper::MAX_REQUEST + 1)
                    .read_to_end(&mut input)
                    .map_err(|e| e.to_string())?;
            }
            if input.len() as u64 > mcsr_toolkit::keyd_helper::MAX_REQUEST {
                return Err("invalid keyd payload size".into());
            }
            let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
            if let Some(response) = mcsr_toolkit::keyd_helper::dispatch(
                &borrowed,
                &input,
                &mcsr_toolkit::keyd_helper::HelperFilesystem::production(),
                &mut mcsr_toolkit::keyd_helper::RealSystem::default(),
            )? {
                println!("{response}");
            }
            Ok(())
        }
        [privileged, setup] if privileged == "privileged" && setup == "setup" => {
            const HELPER: &str = "/usr/local/libexec/mcsr-toolkit";
            if unsafe { libc::geteuid() } != 0
                || env::current_exe().map_err(|e| e.to_string())? != Path::new(HELPER)
            {
                return Err("privileged setup requires the installed root-owned executable".into());
            }
            mcsr_toolkit::setup_helper::run_stdio()
        }
        [privileged, setup, status]
            if privileged == "privileged" && setup == "setup" && status == "status" =>
        {
            const HELPER: &str = "/usr/local/libexec/mcsr-toolkit";
            if unsafe { libc::geteuid() } != 0
                || env::current_exe().map_err(|e| e.to_string())? != Path::new(HELPER)
            {
                return Err(
                    "privileged setup status requires the installed root-owned executable".into(),
                );
            }
            println!("{}", mcsr_toolkit::setup_helper::recovery_status()?);
            Ok(())
        }
        [privileged, setup, uninstall]
            if privileged == "privileged" && setup == "setup" && uninstall == "uninstall" =>
        {
            const HELPER: &str = "/usr/local/libexec/mcsr-toolkit";
            if unsafe { libc::geteuid() } != 0
                || env::current_exe().map_err(|e| e.to_string())? != Path::new(HELPER)
            {
                return Err(
                    "privileged uninstall requires the installed root-owned executable".into(),
                );
            }
            mcsr_toolkit::setup_helper::uninstall_stdio()
        }
        [privileged, maccel, ..] if privileged == "privileged" && maccel == "maccel" => {
            const HELPER: &str = "/usr/local/libexec/mcsr-toolkit";
            if unsafe { libc::geteuid() } != 0
                || env::current_exe().map_err(|e| e.to_string())? != Path::new(HELPER)
            {
                return Err(
                    "privileged maccel mode requires the installed root-owned executable".into(),
                );
            }
            mcsr_toolkit::maccel_helper::verify_executable(Path::new(HELPER), Path::new("/"), 0)?;
            let input = if args.as_slice() == ["privileged", "maccel", "eye"] {
                let mut input = Vec::new();
                std::io::stdin()
                    .take(67)
                    .read_to_end(&mut input)
                    .map_err(|e| e.to_string())?;
                input
            } else if args.as_slice() == ["privileged", "maccel", "restore"] {
                mcsr_toolkit::maccel_helper::read_restore_input(libc::STDIN_FILENO)?
            } else {
                Vec::new()
            };
            let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
            let response = mcsr_toolkit::maccel_helper::dispatch(
                &borrowed,
                &input,
                &mcsr_toolkit::maccel_helper::HelperPaths::production(),
                &mut mcsr_toolkit::maccel_helper::RealParameter,
            )?;
            println!("{response}");
            Ok(())
        }
        [mode] => Err(format!("unknown command mode: {mode}")),
        [] => Err("expected a command mode".into()),
        _ => Err("unexpected command arguments".into()),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    // Keep assets in the executable until the service and installer consume them.
    for path in ["/", "/app.js", "/style.css"] {
        std::hint::black_box(mcsr_toolkit::assets::web_asset(path));
    }
    std::hint::black_box(mcsr_toolkit::assets::extension_archive());

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mcsr-toolkit: {error}");
            ExitCode::FAILURE
        }
    }
}
