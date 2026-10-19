use crate::{read_json, write_json};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const BACKUP_FILE: &str = "coregui-probe.json";
const MAX_SCRIPT_BYTES: usize = 256 * 1024;
const MARKER: &str = "SOLSTRAP_PROBE_V1";
const PROBE: &str = r#"-- SOLSTRAP_PROBE_V1: temporary compatibility test; remove through SOLstrap.
do
    task.defer(function()
        warn("[SOLSTRAP_PROBE_V1] started")
        local ok = pcall(function()
            local coreGui = game:GetService("CoreGui")
            local previous = coreGui:FindFirstChild("SOLstrapCompatibilityProbe")
            if previous then previous:Destroy() end
            local screen = Instance.new("ScreenGui")
            screen.Name = "SOLstrapCompatibilityProbe"
            screen.ResetOnSpawn = false
            screen.DisplayOrder = 9999
            local label = Instance.new("TextLabel")
            label.Name = "CompatibilityLabel"
            label.AnchorPoint = Vector2.new(0.5, 0)
            label.Position = UDim2.new(0.5, 0, 0, 8)
            label.Size = UDim2.new(0, 250, 0, 30)
            label.BackgroundColor3 = Color3.fromRGB(31, 27, 24)
            label.BorderSizePixel = 0
            label.TextColor3 = Color3.fromRGB(239, 154, 80)
            label.Font = Enum.Font.SourceSansSemibold
            label.TextSize = 17
            label.Text = "SOLstrap - CoreGui test"
            label.Parent = screen
            screen.Parent = coreGui
        end)
        warn("[SOLSTRAP_PROBE_V1] CoreGui write ok=", ok)
    end)
end
-- Original client script continues unchanged below.
"#;

#[derive(Deserialize, Serialize)]
struct Backup {
    schema_version: u32,
    script_path: PathBuf,
    original: Vec<u8>,
    original_hash: String,
    patched_hash: String,
}

#[derive(Debug, Serialize)]
pub struct ProbeStatus {
    pub installed: bool,
    pub recovery_available: bool,
    pub conflict: bool,
}

fn script_path(root: &Path) -> PathBuf {
    root.join("Player/content/scripts/CoreScripts/PerformanceStatsManagerScript.lua")
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn patched(original: &[u8]) -> Vec<u8> {
    let (prefix, source) = if let Some(source) = original.strip_prefix(b"\xef\xbb\xbf") {
        (&b"\xef\xbb\xbf"[..], source)
    } else {
        (&b""[..], original)
    };
    let mut bytes = prefix.to_vec();
    bytes.extend_from_slice(PROBE.as_bytes());
    bytes.extend_from_slice(source);
    bytes
}

fn load_backup(dir: &Path, root: &Path) -> Result<Option<Backup>> {
    let path = dir.join(BACKUP_FILE);
    if !path
        .try_exists()
        .context("Cannot inspect CoreGui recovery file")?
    {
        return Ok(None);
    }
    ensure!(
        fs::metadata(&path)?.len() < 2 * 1024 * 1024,
        "CoreGui probe backup is too large"
    );
    let backup: Backup = read_json(&path)?;
    ensure!(
        backup.schema_version == 1 && backup.original.len() <= MAX_SCRIPT_BYTES,
        "Unsupported CoreGui probe backup"
    );
    let expected = script_path(root);
    ensure!(
        backup.script_path == expected,
        "The CoreGui probe belongs to a different client location"
    );
    ensure!(
        hash(&backup.original) == backup.original_hash
            && hash(&patched(&backup.original)) == backup.patched_hash,
        "CoreGui probe backup integrity check failed"
    );
    Ok(Some(backup))
}

pub fn status(dir: &Path, root: &Path) -> Result<ProbeStatus> {
    let Some(backup) = load_backup(dir, root)? else {
        return Ok(ProbeStatus {
            installed: false,
            recovery_available: false,
            conflict: false,
        });
    };
    let current = fs::read(script_path(root))
        .context("The CoreGui probe script is missing; recovery information was retained")?;
    let current_hash = hash(&current);
    Ok(ProbeStatus {
        installed: current_hash == backup.patched_hash,
        recovery_available: true,
        conflict: current_hash != backup.patched_hash && current_hash != backup.original_hash,
    })
}

fn write_script(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().context("Script has no parent folder")?)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|error| error.error)
        .context("Cannot replace the CoreGui test script")?;
    Ok(())
}

pub fn install(dir: &Path, root: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("coregui-probe.lock"))?;
    lock.try_lock_exclusive()
        .context("Another process is changing the CoreGui test")?;
    if load_backup(dir, root)?.is_some() {
        ensure!(
            status(dir, root)?.installed,
            "Restore the existing CoreGui probe backup before installing again"
        );
        return Ok(());
    }
    let path = script_path(root);
    let metadata = fs::symlink_metadata(&path)
        .context("This client does not have the expected loose CoreScript")?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "The CoreScript must be a regular file"
    );
    ensure!(
        metadata.len() <= MAX_SCRIPT_BYTES as u64,
        "The CoreScript is too large for this compatibility test"
    );
    let original = fs::read(&path)?;
    let source = std::str::from_utf8(&original).context("The CoreScript is not UTF-8 source")?;
    ensure!(
        !source.contains(MARKER),
        "An existing SOLstrap probe has no matching recovery backup; the script was left unchanged"
    );
    let bytes = patched(&original);
    let backup = Backup {
        schema_version: 1,
        script_path: path.clone(),
        original_hash: hash(&original),
        patched_hash: hash(&bytes),
        original,
    };
    // Back up first. If the process stops before replacement, restore safely clears the staged backup.
    write_json(&dir.join(BACKUP_FILE), &backup)?;
    write_script(&path, &bytes)
}

pub fn restore(dir: &Path, root: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("coregui-probe.lock"))?;
    lock.try_lock_exclusive()
        .context("Another process is changing the CoreGui test")?;
    let Some(backup) = load_backup(dir, root)? else {
        return Ok(());
    };
    let current = fs::read(&backup.script_path)
        .context("The script is missing; recovery backup was retained")?;
    let current_hash = hash(&current);
    if current_hash == backup.patched_hash {
        write_script(&backup.script_path, &backup.original)?;
    } else {
        ensure!(current_hash == backup.original_hash, "Ferns or another tool changed this script. It was left unchanged and the recovery backup was retained");
    }
    fs::remove_file(dir.join(BACKUP_FILE))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Vec<u8>) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Ferns");
        let data = temp.path().join("SOLstrap");
        let path = script_path(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = b"-- existing client customization\r\nwarn(\"original\")\r\n".to_vec();
        fs::write(path, &original).unwrap();
        (temp, root, data, original)
    }

    #[test]
    fn installs_idempotently_and_restores_original_bytes() {
        let (_temp, root, data, original) = fixture();
        install(&data, &root).unwrap();
        install(&data, &root).unwrap();
        assert!(status(&data, &root).unwrap().installed);
        assert!(fs::read(script_path(&root)).unwrap().ends_with(&original));
        restore(&data, &root).unwrap();
        assert_eq!(fs::read(script_path(&root)).unwrap(), original);
        assert!(!status(&data, &root).unwrap().recovery_available);
    }

    #[test]
    fn preserves_later_client_updates_and_recovery_backup() {
        let (_temp, root, data, _original) = fixture();
        install(&data, &root).unwrap();
        fs::write(script_path(&root), b"updated by Ferns").unwrap();
        assert!(status(&data, &root).unwrap().conflict);
        assert!(restore(&data, &root).is_err());
        assert_eq!(fs::read(script_path(&root)).unwrap(), b"updated by Ferns");
        assert!(data.join(BACKUP_FILE).is_file());
    }

    #[test]
    fn rejects_corrupt_backup_and_different_installation() {
        let (_temp, root, data, _original) = fixture();
        install(&data, &root).unwrap();
        assert!(status(&data, &root.join("other")).is_err());
        let path = data.join(BACKUP_FILE);
        let mut backup: Backup = read_json(&path).unwrap();
        backup.original.push(b'x');
        write_json(&path, &backup).unwrap();
        assert!(restore(&data, &root).is_err());
    }

    #[test]
    fn recovers_a_staged_backup_without_rewriting_original() {
        let (_temp, root, data, original) = fixture();
        install(&data, &root).unwrap();
        fs::write(script_path(&root), &original).unwrap();
        let staged = status(&data, &root).unwrap();
        assert!(staged.recovery_available && !staged.installed && !staged.conflict);
        restore(&data, &root).unwrap();
        assert_eq!(fs::read(script_path(&root)).unwrap(), original);
    }

    #[test]
    fn preserves_utf8_bom_at_start_of_patched_source() {
        let original = b"\xef\xbb\xbf-- original\r\n";
        let bytes = patched(original);
        assert!(bytes.starts_with(b"\xef\xbb\xbf-- SOLSTRAP_PROBE_V1"));
        assert!(bytes.ends_with(b"-- original\r\n"));
        assert_eq!(
            bytes
                .windows(3)
                .filter(|window| *window == b"\xef\xbb\xbf")
                .count(),
            1
        );
    }
}
