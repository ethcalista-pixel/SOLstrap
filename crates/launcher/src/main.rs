#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use anyhow::{ensure, Context, Result};
use solstrap_core::{
    coregui_loader_trial, coregui_probe, data_dir, diagnostics, discovery, health,
    integration::{self, NativeRegistry},
    interface_colors, interface_font, launch,
    settings::Settings,
};
use std::{path::PathBuf, process::Command};

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = data_dir()?;
    match args.first().map(String::as_str) {
        Some("--inspect") if args.len() == 1 => {
            let settings = Settings::load(&dir)?;
            let installation = discovery::inspect(&settings)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "probe": coregui_probe::status(&dir, &installation.root)?,
                    "loader_trial": coregui_loader_trial::status(&dir, &installation.root)?,
                    "interface_font": interface_font::status(&dir, &installation.root)?,
                    "installation": installation,
                    "integration": integration::status(&dir, &NativeRegistry)?,
                    "data_dir": dir,
                }))?
            );
            Ok(())
        }
        Some("--restore") if args.len() == 1 => integration::disable(&dir, &NativeRegistry),
        Some("--diagnose") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            let shared_logs =
                std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Roblox/logs"));
            let logs = diagnostics::inspect_coregui_with_shared(
                &installation.root,
                shared_logs.as_deref(),
            );
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"coregui":logs,"health":health::inspect(&installation,&logs)})
                )?
            );
            Ok(())
        }
        Some("--probe-install") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            ensure!(
                installation.detected,
                "Select a valid Ferns installation first"
            );
            coregui_probe::install(&dir, &installation.root)
        }
        Some("--probe-restore") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            coregui_probe::restore(&dir, &installation.root)
        }
        Some("--font-test-install") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            interface_font::install(&dir, &installation.root)
        }
        Some("--coregui-loader-test-install") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            coregui_loader_trial::install_with_probe(&dir, &installation.root)
        }
        Some("--coregui-loader-test-restore") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            coregui_loader_trial::restore_with_probe(&dir, &installation.root)
        }
        Some("--font-test-restore") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            interface_font::restore(&dir, &installation.root)
        }
        Some("--gui-colors-restore") if args.len() == 1 => {
            let installation = discovery::inspect(&Settings::load(&dir)?)?;
            interface_colors::restore(&dir, &installation.root)
        }
        Some("--help") | None => {
            println!("--gui-colors-restore  Restore the original GUI shader pack from verified local recovery");
            println!("--coregui-loader-test-install  Developer trial: normal local loader setting plus label test\n--coregui-loader-test-restore  Restore trial settings and original script");
            println!("--font-test-install  Back up Builder Sans and map it to bundled Builder Mono\n--font-test-restore  Restore the original font definition");
            println!("SOLstrap launcher 0.1.0\n--inspect  Read-only client detection\n--diagnose  Read-only CoreGui/log evidence\n--dry-run <ferns-link>  Validate forwarding without launching\n--restore  Restore original website launch handlers\n--probe-install  Back up the loose script and add a temporary CoreGui test\n--probe-restore  Restore the original test script\n<ferns-link>  Forward a website join after integration is enabled");
            Ok(())
        }
        Some("--dry-run") if args.len() == 2 => {
            let target = target_for(&args[1], &dir, false)?;
            println!(
                "{}",
                serde_json::json!({ "protocol": launch::protocol(&args[1])?, "target": target, "payload": "[redacted]", "launched": false })
            );
            Ok(())
        }
        Some(_) if args.len() == 1 => {
            let target = target_for(&args[0], &dir, true)?;
            Command::new(&target)
                .arg(&args[0])
                .current_dir(
                    target
                        .parent()
                        .context("Launcher has no parent directory")?,
                )
                .spawn()
                .context("Ferns could not be started. Open SOLstrap to inspect the installation")?;
            Ok(())
        }
        _ => anyhow::bail!("Invalid launcher arguments. Use --help for available commands"),
    }
}

fn target_for(uri: &str, dir: &std::path::Path, require_backup: bool) -> Result<PathBuf> {
    let protocol = launch::protocol(uri)?;
    let installation = discovery::inspect(&Settings::load(dir)?)?;
    let expected = installation
        .launchers
        .iter()
        .find(|entry| entry.protocol == protocol)
        .context("Unsupported launcher")?;
    ensure!(expected.exists, "The original Ferns launcher for {protocol} is missing. Repair or reinstall Ferns before enabling website integration");
    let target = match integration::load_backup(dir)? {
        Some(backup) => {
            backup
                .entries
                .into_iter()
                .find(|entry| entry.protocol == protocol)
                .context("Integration backup is incomplete")?
                .original_target
        }
        None => {
            ensure!(!require_backup, "SOLstrap integration is not enabled. Open the SOLstrap app or run --restore if needed");
            expected.target.clone()
        }
    };
    ensure!(launch::same_file_path(&target, &expected.target), "The saved Ferns launcher no longer matches the selected installation; restore integration and configure it again");
    ensure!(
        !launch::same_file_path(&target, &std::env::current_exe()?),
        "Refusing a launcher forwarding loop"
    );
    Ok(target)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("SOLstrap: {error:#}");
        #[cfg(windows)]
        if std::env::args()
            .nth(1)
            .is_some_and(|arg| !arg.starts_with("--"))
        {
            let message: Vec<u16> = format!("{error:#}\0").encode_utf16().collect();
            let title: Vec<u16> = "SOLstrap — launch failed\0".encode_utf16().collect();
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    std::ptr::null_mut(),
                    message.as_ptr(),
                    title.as_ptr(),
                    0x10,
                );
            }
        }
        std::process::exit(1);
    }
}
