#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use anyhow::{ensure, Context, Result};
use solstrap_core::{
    data_dir, discovery,
    integration::{self, NativeRegistry},
    launch,
    settings::Settings,
};
use std::{path::PathBuf, process::Command};

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = data_dir()?;
    match args.first().map(String::as_str) {
        Some("--inspect") if args.len() == 1 => {
            let settings = Settings::load(&dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "installation": discovery::inspect(&settings)?,
                    "integration": integration::status(&dir, &NativeRegistry)?,
                    "data_dir": dir,
                }))?
            );
            Ok(())
        }
        Some("--restore") if args.len() == 1 => integration::disable(&dir, &NativeRegistry),
        Some("--help") | None => {
            println!("SOLstrap launcher 0.1.0\n--inspect  Read-only client detection\n--dry-run <ferns-link>  Validate forwarding without launching\n--restore  Restore original website launch handlers\n<ferns-link>  Forward a website join after integration is enabled");
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
