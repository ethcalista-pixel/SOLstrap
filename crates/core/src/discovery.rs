use crate::settings::Settings;
use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize)]
pub struct Installation {
    pub root: PathBuf,
    pub detected: bool,
    pub android_guest_library: bool,
    pub player: PathBuf,
    pub player_bytes: Option<u64>,
    pub launchers: Vec<Launcher>,
    pub core_script_files: usize,
}

#[derive(Debug, Serialize)]
pub struct Launcher {
    pub protocol: String,
    pub target: PathBuf,
    pub exists: bool,
}

pub const PROTOCOLS: [(&str, &str); 2] = [
    ("ferns2026", "Player/FernsLauncher.exe"),
    ("ferns-launcher", "Client/FernsLauncher.exe"),
];

/// Evidence of the Android guest library, not a guarantee of host compatibility.
pub fn android_library_present(root: &Path) -> bool {
    let path = root.join("Player/root/data/app/com.ferns.client/lib/x86_64/libroblox.so");
    let mut magic = [0u8; 4];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut magic))
        .is_ok()
        && magic == *b"\x7fELF"
}

pub fn inspect(settings: &Settings) -> Result<Installation> {
    let root = match &settings.client_root {
        Some(root) => root.clone(),
        None => {
            #[cfg(windows)]
            {
                PathBuf::from(
                    std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is unavailable")?,
                )
                .join("Ferns")
            }
            #[cfg(not(windows))]
            {
                anyhow::bail!(
                    "Select a Ferns installation. Linux compatibility is not yet validated."
                )
            }
        }
    };
    let player = if root.join("Player/Ferns.exe").is_file() {
        root.join("Player/Ferns.exe")
    } else {
        root.join("Player/RobloxPlayerBeta.exe")
    };
    let detected = player.is_file();
    let player_bytes = std::fs::metadata(&player).ok().map(|m| m.len());
    let launchers = PROTOCOLS
        .iter()
        .map(|(protocol, relative)| {
            let target = root.join(relative);
            Launcher {
                protocol: (*protocol).into(),
                exists: target.is_file(),
                target,
            }
        })
        .collect();
    let core_script_files = count_lua(&root.join("Player/content/scripts"), 0);
    Ok(Installation {
        android_guest_library: android_library_present(&root),
        root,
        detected,
        player,
        player_bytes,
        launchers,
        core_script_files,
    })
}

fn count_lua(path: &Path, depth: usize) -> usize {
    if depth > 12 {
        return 0;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| {
            let Ok(kind) = entry.file_type() else {
                return 0;
            };
            if kind.is_symlink() {
                return 0;
            }
            if kind.is_dir() {
                count_lua(&entry.path(), depth + 1)
            } else {
                usize::from(entry.path().extension().is_some_and(|ext| ext == "lua"))
            }
        })
        .sum()
}

pub fn fingerprint(path: &Path) -> Result<String> {
    let mut file = File::open(path).context("Cannot open client for fingerprinting")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_revival_player_and_original_player_names() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            client_root: Some(dir.path().to_owned()),
            ..Settings::default()
        };
        assert!(!inspect(&settings).unwrap().detected);
        std::fs::create_dir(dir.path().join("Player")).unwrap();
        let original = dir.path().join("Player/RobloxPlayerBeta.exe");
        std::fs::write(&original, b"fixture").unwrap();
        assert_eq!(inspect(&settings).unwrap().player, original);
        let revival = dir.path().join("Player/Ferns.exe");
        std::fs::write(&revival, b"fixture").unwrap();
        let installation = inspect(&settings).unwrap();
        assert!(installation.detected);
        assert_eq!(installation.player, revival);
        std::fs::remove_file(original).unwrap();
        assert!(inspect(&settings).unwrap().detected);
    }
}
