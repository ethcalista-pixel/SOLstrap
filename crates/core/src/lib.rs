pub mod coregui_loader_trial;
pub mod coregui_probe;
pub mod crashes;
pub mod diagnostics;
pub mod discovery;
pub mod health;
pub mod integration;
pub mod interface_colors;
pub mod interface_font;
pub mod interface_preferences;
pub mod launch;
pub mod settings;
pub mod themes;

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn data_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is unavailable")?;
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
        .context("User data directory is unavailable")?;
    Ok(PathBuf::from(root).join("SOLstrap"))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("Cannot read {}", path.display()))?;
    serde_json::from_slice(&bytes).context("The saved file is not a valid SOLstrap document")
}

/// A packaged development host can virtualize per-user registry and AppData.
#[cfg(windows)]
pub fn execution_package() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentPackageFullName(length: *mut u32, name: *mut u16) -> i32;
    }
    let mut length = 0;
    unsafe {
        if GetCurrentPackageFullName(&mut length, std::ptr::null_mut()) != 122 || length > 4096 {
            return None;
        }
        let mut name = vec![0u16; length as usize];
        if GetCurrentPackageFullName(&mut length, name.as_mut_ptr()) != 0 {
            return None;
        }
        Some(String::from_utf16_lossy(
            &name[..name.iter().position(|c| *c == 0).unwrap_or(name.len())],
        ))
    }
}
#[cfg(not(windows))]
pub fn execution_package() -> Option<String> {
    None
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().context("Document needs a parent directory")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temp, value)?;
    temp.write_all(b"\n")?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|error| error.error)
        .context("Cannot replace saved document")?;
    Ok(())
}
