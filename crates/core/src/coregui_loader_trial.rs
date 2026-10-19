//! Reversible developer trial of a normal local setting, not a protection bypass.
use crate::{read_json, write_json};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};
const BACKUP: &str = "coregui-loader-trial.json";
const FLAG: &str = "FFlagLoadCoreScriptsFromPatchOnly";
const MAX: usize = 4 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
struct Backup {
    version: u32,
    path: PathBuf,
    original: Vec<u8>,
    original_hash: String,
    patched_hash: String,
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub installed: bool,
    pub recovery_available: bool,
    pub conflict: bool,
}
fn path(root: &Path) -> PathBuf {
    root.join("Player/ClientSettings/ClientAppSettings.json")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn patch(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
    value
        .as_object_mut()
        .context("Client settings must be an object")?
        .insert(FLAG.into(), serde_json::Value::String("False".into()));
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    ensure!(
        bytes.len() <= MAX,
        "Client settings exceed the trial size limit"
    );
    Ok(bytes)
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let info = fs::symlink_metadata(path).context("Existing client settings are required")?;
    ensure!(
        info.is_file() && !info.file_type().is_symlink() && info.len() <= MAX as u64,
        "Expected a small regular client-settings file"
    );
    Ok(fs::read(path)?)
}
fn load(dir: &Path, root: &Path) -> Result<Option<Backup>> {
    let file = dir.join(BACKUP);
    if !file
        .try_exists()
        .context("Cannot inspect loader recovery file")?
    {
        return Ok(None);
    }
    ensure!(
        fs::metadata(&file)?.len() <= 64 * 1024 * 1024,
        "Trial recovery file is too large"
    );
    let backup: Backup = read_json(&file)?;
    ensure!(
        backup.version == 1 && backup.path == path(root) && backup.original.len() <= MAX,
        "Trial recovery belongs to another installation or version"
    );
    ensure!(
        hash(&backup.original) == backup.original_hash
            && hash(&patch(&backup.original)?) == backup.patched_hash,
        "Trial recovery integrity check failed"
    );
    Ok(Some(backup))
}
fn lock(dir: &Path) -> Result<fs::File> {
    fs::create_dir_all(dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("coregui-loader-trial.lock"))?;
    file.try_lock_exclusive()
        .context("Another process is changing the loader trial")?;
    Ok(file)
}
fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing client-settings folder")?)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| e.error)
        .context("Cannot replace trial settings")?;
    Ok(())
}
pub fn status(dir: &Path, root: &Path) -> Result<Status> {
    let Some(backup) = load(dir, root)? else {
        return Ok(Status {
            installed: false,
            recovery_available: false,
            conflict: false,
        });
    };
    let current = hash(&read(&backup.path)?);
    Ok(Status {
        installed: current == backup.patched_hash,
        recovery_available: true,
        conflict: current != backup.patched_hash && current != backup.original_hash,
    })
}
pub fn install(dir: &Path, root: &Path) -> Result<()> {
    let _lock = lock(dir)?;
    if load(dir, root)?.is_some() {
        ensure!(
            status(dir, root)?.installed,
            "Restore the existing trial first"
        );
        return Ok(());
    }
    let path = path(root);
    let original = read(&path)?;
    let patched = patch(&original)?;
    let backup = Backup {
        version: 1,
        path: path.clone(),
        original_hash: hash(&original),
        patched_hash: hash(&patched),
        original,
    };
    write_json(&dir.join(BACKUP), &backup)?;
    replace(&path, &patched)
}
pub fn restore(dir: &Path, root: &Path) -> Result<()> {
    let _lock = lock(dir)?;
    let Some(backup) = load(dir, root)? else {
        return Ok(());
    };
    let current = hash(&read(&backup.path)?);
    ensure!(
        current == backup.patched_hash || current == backup.original_hash,
        "Client settings changed after the trial. They were preserved and recovery retained"
    );
    if current == backup.patched_hash {
        replace(&backup.path, &backup.original)?;
    }
    fs::remove_file(dir.join(BACKUP))?;
    Ok(())
}
pub fn install_with_probe(dir: &Path, root: &Path) -> Result<()> {
    let existing = status(dir, root)?.recovery_available;
    install(dir, root)?;
    if let Err(error) = crate::coregui_probe::install(dir, root) {
        if !existing {
            restore(dir, root)
                .context("Script trial failed and settings rollback failed; recovery retained")?;
        }
        return Err(error);
    }
    Ok(())
}

pub fn restore_with_probe(dir: &Path, root: &Path) -> Result<()> {
    restore(dir, root)?;
    crate::coregui_probe::restore(dir, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Vec<u8>) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Ferns");
        let dir = temp.path().join("data");
        fs::create_dir_all(path(&root).parent().unwrap()).unwrap();
        let original = br#"{ "ExistingFlag": "True", "OtherValue": 42 }"#.to_vec();
        fs::write(path(&root), &original).unwrap();
        (temp, root, dir, original)
    }
    #[test]
    fn changes_only_one_setting_and_restores_exact_bytes() {
        let (_temp, root, dir, original) = fixture();
        install(&dir, &root).unwrap();
        install(&dir, &root).unwrap();
        assert!(status(&dir, &root).unwrap().installed);
        let mut result: serde_json::Value = read_json(&path(&root)).unwrap();
        assert_eq!(
            result.as_object_mut().unwrap().remove(FLAG).unwrap(),
            "False"
        );
        assert_eq!(
            result,
            serde_json::from_slice::<serde_json::Value>(&original).unwrap()
        );
        restore(&dir, &root).unwrap();
        assert_eq!(fs::read(path(&root)).unwrap(), original);
    }
    #[test]
    fn external_changes_are_preserved_with_recovery() {
        let (_temp, root, dir, _) = fixture();
        install(&dir, &root).unwrap();
        fs::write(path(&root), b"external changes").unwrap();
        assert!(status(&dir, &root).unwrap().conflict);
        assert!(restore(&dir, &root).is_err());
        assert_eq!(fs::read(path(&root)).unwrap(), b"external changes");
        assert!(dir.join(BACKUP).exists());
    }
    #[test]
    fn invalid_input_does_not_modify_the_client() {
        let (_temp, root, dir, _) = fixture();
        fs::write(path(&root), b"[]").unwrap();
        assert!(install(&dir, &root).is_err());
        assert!(!dir.join(BACKUP).exists());
        assert_eq!(fs::read(path(&root)).unwrap(), b"[]");
    }

    #[test]
    fn missing_script_rolls_back_new_settings_trial() {
        let (_temp, root, dir, original) = fixture();
        assert!(install_with_probe(&dir, &root).is_err());
        assert_eq!(fs::read(path(&root)).unwrap(), original);
        assert!(!dir.join(BACKUP).exists());
    }
    #[test]
    fn interrupted_replacement_and_tampered_backup_are_safe() {
        let (_temp, root, dir, original) = fixture();
        install(&dir, &root).unwrap();
        fs::write(path(&root), &original).unwrap();
        restore(&dir, &root).unwrap();
        assert!(!dir.join(BACKUP).exists());
        install(&dir, &root).unwrap();
        let mut backup: Backup = read_json(&dir.join(BACKUP)).unwrap();
        backup.original.push(b'x');
        write_json(&dir.join(BACKUP), &backup).unwrap();
        assert!(restore(&dir, &root).is_err());
    }
}
