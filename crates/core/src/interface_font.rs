//! A reversible asset experiment. This does not execute scripts or grant CoreGui access.
use crate::{read_json, write_json};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

const BACKUP: &str = "interface-font-test.json";
const LIMIT: usize = 64 * 1024;

#[derive(Serialize, Deserialize)]
struct Backup {
    version: u32,
    path: PathBuf,
    original: Vec<u8>,
    patched: Vec<u8>,
    original_hash: String,
    patched_hash: String,
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub installed: bool,
    pub recovery_available: bool,
    pub conflict: bool,
}

fn target(root: &Path) -> PathBuf {
    root.join("Player/content/fonts/families/BuilderSans.json")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_regular(path: &Path) -> Result<Vec<u8>> {
    let info = fs::symlink_metadata(path).context("Font definition is missing")?;
    ensure!(
        info.is_file() && !info.file_type().is_symlink() && info.len() <= LIMIT as u64,
        "Expected a small, regular font-definition file"
    );
    Ok(fs::read(path)?)
}
fn load(dir: &Path, root: &Path) -> Result<Option<Backup>> {
    let path = dir.join(BACKUP);
    if !path
        .try_exists()
        .context("Cannot inspect font recovery file")?
    {
        return Ok(None);
    }
    ensure!(
        fs::metadata(&path)?.len() <= 1024 * 1024,
        "Font recovery file is too large"
    );
    let backup: Backup = read_json(&path)?;
    ensure!(
        backup.version == 1 && backup.path == target(root),
        "Font recovery belongs to another installation or version"
    );
    ensure!(
        backup.original.len() <= LIMIT
            && backup.patched.len() <= LIMIT
            && hash(&backup.original) == backup.original_hash
            && hash(&backup.patched) == backup.patched_hash,
        "Font recovery integrity check failed"
    );
    Ok(Some(backup))
}
pub fn status(dir: &Path, root: &Path) -> Result<Status> {
    let Some(backup) = load(dir, root)? else {
        return Ok(Status {
            installed: false,
            recovery_available: false,
            conflict: false,
        });
    };
    let current = hash(&read_regular(&backup.path)?);
    Ok(Status {
        installed: current == backup.patched_hash,
        recovery_available: true,
        conflict: current != backup.patched_hash && current != backup.original_hash,
    })
}
fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().context("Missing font folder")?)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|e| e.error)
        .context("Cannot replace font definition")?;
    Ok(())
}
fn lock(dir: &Path) -> Result<fs::File> {
    fs::create_dir_all(dir)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("interface-font-test.lock"))?;
    file.try_lock_exclusive()
        .context("Another process is changing the font test")?;
    Ok(file)
}

fn patch(original: &[u8], source: &[u8], fonts: &Path) -> Result<Vec<u8>> {
    let mut family: Value = serde_json::from_slice(original)?;
    let source: Value = serde_json::from_slice(source)?;
    let candidates = source["faces"]
        .as_array()
        .context("Source font has no faces")?;
    ensure!(
        !candidates.is_empty() && candidates.len() <= 64,
        "Invalid source font faces"
    );
    for face in candidates {
        let uri = face["assetId"].as_str().context("Missing font asset")?;
        let file = uri
            .strip_prefix("rbxasset://fonts/")
            .context("The test requires bundled font assets")?;
        ensure!(
            !file.is_empty()
                && !file.contains(['/', '\\', ':'])
                && (file.ends_with(".otf") || file.ends_with(".ttf")),
            "Invalid bundled font path"
        );
        let info =
            fs::symlink_metadata(fonts.join(file)).context("A bundled font asset is missing")?;
        ensure!(
            info.is_file() && !info.file_type().is_symlink(),
            "The font asset must be a regular file"
        );
        ensure!(
            face["weight"]
                .as_u64()
                .is_some_and(|w| (100..=900).contains(&w)),
            "Invalid source font weight"
        );
        ensure!(
            face["style"].as_str().is_some(),
            "Missing source font style"
        );
    }
    let faces = family["faces"]
        .as_array_mut()
        .context("Target font has no faces")?;
    ensure!(
        !faces.is_empty() && faces.len() <= 64,
        "Invalid target font faces"
    );
    for face in faces {
        let weight = face["weight"]
            .as_u64()
            .context("Missing target font weight")?;
        let style = face["style"]
            .as_str()
            .context("Missing target font style")?;
        let nearest = candidates
            .iter()
            .filter(|c| c["style"].as_str() == Some(style))
            .min_by_key(|c| c["weight"].as_u64().unwrap().abs_diff(weight))
            .context("No matching font style is bundled")?;
        face["assetId"] = nearest["assetId"].clone();
    }
    let mut bytes = serde_json::to_vec_pretty(&family)?;
    bytes.push(b'\n');
    ensure!(bytes.len() <= LIMIT, "Patched font definition is too large");
    Ok(bytes)
}

pub fn install(dir: &Path, root: &Path) -> Result<()> {
    let _lock = lock(dir)?;
    if load(dir, root)?.is_some() {
        ensure!(
            status(dir, root)?.installed,
            "Restore the existing font test before installing again"
        );
        return Ok(());
    }
    let path = target(root);
    let original = read_regular(&path)?;
    let fonts = root.join("Player/content/fonts");
    let source = read_regular(&fonts.join("families/BuilderMono.json"))?;
    let patched = patch(&original, &source, &fonts)?;
    ensure!(
        patched != original,
        "This font definition already matches the test"
    );
    let backup = Backup {
        version: 1,
        path: path.clone(),
        original_hash: hash(&original),
        patched_hash: hash(&patched),
        original,
        patched,
    };
    write_json(&dir.join(BACKUP), &backup)?;
    replace(&path, &backup.patched)
}

pub fn restore(dir: &Path, root: &Path) -> Result<()> {
    let _lock = lock(dir)?;
    let Some(backup) = load(dir, root)? else {
        return Ok(());
    };
    let current = hash(&read_regular(&backup.path)?);
    ensure!(current == backup.patched_hash || current == backup.original_hash, "The font changed after installation. It was left unchanged and the recovery backup retained");
    if current == backup.patched_hash {
        replace(&backup.path, &backup.original)?;
    }
    fs::remove_file(dir.join(BACKUP))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Vec<u8>) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Ferns");
        let data = temp.path().join("data");
        let font = root.join("Player/content/fonts");
        fs::create_dir_all(font.join("families")).unwrap();
        let original = br#"{"name":"Builder Sans","faces":[{"weight":500,"style":"normal","assetId":"rbxasset://fonts/original.otf"}]}"#.to_vec();
        fs::write(target(&root), &original).unwrap();
        fs::write(
            font.join("families/BuilderMono.json"),
            br#"{"faces":[{"weight":400,"style":"normal","assetId":"rbxasset://fonts/mono.otf"}]}"#,
        )
        .unwrap();
        fs::write(font.join("mono.otf"), b"test fixture").unwrap();
        (temp, root, data, original)
    }
    #[test]
    fn installation_is_idempotent_and_restore_is_byte_exact() {
        let (_temp, root, data, original) = fixture();
        install(&data, &root).unwrap();
        install(&data, &root).unwrap();
        assert!(status(&data, &root).unwrap().installed);
        let value: Value = serde_json::from_slice(&fs::read(target(&root)).unwrap()).unwrap();
        assert_eq!(value["faces"][0]["weight"], 500);
        assert_eq!(value["faces"][0]["assetId"], "rbxasset://fonts/mono.otf");
        restore(&data, &root).unwrap();
        assert_eq!(fs::read(target(&root)).unwrap(), original);
        assert!(!data.join(BACKUP).exists());
    }
    #[test]
    fn later_changes_and_backup_are_preserved() {
        let (_temp, root, data, _) = fixture();
        install(&data, &root).unwrap();
        fs::write(target(&root), b"later changes").unwrap();
        assert!(status(&data, &root).unwrap().conflict);
        assert!(restore(&data, &root).is_err());
        assert_eq!(fs::read(target(&root)).unwrap(), b"later changes");
        assert!(data.join(BACKUP).exists());
    }
    #[test]
    fn missing_assets_and_external_paths_are_rejected_without_changes() {
        let (_temp, root, data, original) = fixture();
        let fonts = root.join("Player/content/fonts");
        fs::remove_file(fonts.join("mono.otf")).unwrap();
        assert!(install(&data, &root).is_err());
        assert_eq!(fs::read(target(&root)).unwrap(), original);
        assert!(!data.join(BACKUP).exists());
        for asset in [
            "rbxasset://fonts/../other.otf",
            "https://example.com/font.otf",
            "rbxassetid://1",
        ] {
            let source = serde_json::to_vec(
                &serde_json::json!({"faces":[{"weight":400,"style":"normal","assetId":asset}]}),
            )
            .unwrap();
            assert!(patch(&original, &source, &fonts).is_err());
        }
    }
    #[test]
    fn tampered_backup_cannot_overwrite_current_font() {
        let (_temp, root, data, _) = fixture();
        install(&data, &root).unwrap();
        let current = fs::read(target(&root)).unwrap();
        let mut backup: Backup = read_json(&data.join(BACKUP)).unwrap();
        backup.original.push(b'x');
        write_json(&data.join(BACKUP), &backup).unwrap();
        assert!(restore(&data, &root).is_err());
        assert_eq!(fs::read(target(&root)).unwrap(), current);
    }

    #[test]
    fn staged_backup_recovers_when_replacement_was_interrupted() {
        let (_temp, root, data, original) = fixture();
        install(&data, &root).unwrap();
        fs::write(target(&root), &original).unwrap();
        assert!(!status(&data, &root).unwrap().installed);
        assert!(status(&data, &root).unwrap().recovery_available);
        restore(&data, &root).unwrap();
        assert_eq!(fs::read(target(&root)).unwrap(), original);
        assert!(!data.join(BACKUP).exists());
    }
}
