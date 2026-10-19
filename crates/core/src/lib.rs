pub mod coregui_loader_trial;
pub mod coregui_probe;
pub mod diagnostics;
pub mod discovery;
pub mod health;
pub mod integration;
pub mod interface_font;
pub mod interface_preferences;
pub mod launch;
pub mod settings;

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
