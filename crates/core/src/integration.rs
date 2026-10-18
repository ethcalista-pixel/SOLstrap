use crate::{
    discovery::{Installation, PROTOCOLS},
    launch::{handler_command, parse_original, same_file_path},
    read_json, write_json,
};
use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

const STATE_FILE: &str = "integration.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BackupEntry {
    pub protocol: String,
    pub original_command: String,
    pub original_target: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IntegrationBackup {
    pub schema_version: u32,
    pub owned_command: String,
    pub entries: Vec<BackupEntry>,
}

#[derive(Debug, Serialize)]
pub struct ProtocolStatus {
    pub protocol: String,
    pub registered: bool,
    pub managed: bool,
    pub original_available: bool,
}

#[derive(Debug, Serialize)]
pub struct IntegrationStatus {
    pub recovery_available: bool,
    pub enabled: bool,
    pub protocols: Vec<ProtocolStatus>,
}

pub trait ProtocolRegistry {
    fn read(&self, protocol: &str) -> Result<Option<String>>;
    fn write(&self, protocol: &str, command: &str) -> Result<()>;
}

pub struct NativeRegistry;

fn known_protocol(protocol: &str) -> Result<()> {
    ensure!(
        PROTOCOLS.iter().any(|(name, _)| *name == protocol),
        "Unknown Ferns protocol"
    );
    Ok(())
}

#[cfg(windows)]
impl ProtocolRegistry for NativeRegistry {
    fn read(&self, protocol: &str) -> Result<Option<String>> {
        known_protocol(protocol)?;
        let root = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let path = format!("Software\\Classes\\{protocol}\\shell\\open\\command");
        let key = match root.open_subkey(path) {
            Ok(key) => key,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        match key.get_value("") {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write(&self, protocol: &str, command: &str) -> Result<()> {
        known_protocol(protocol)?;
        // Only update an existing Ferns handler. Do not change protocol metadata.
        let root = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let key = root.open_subkey_with_flags(
            format!("Software\\Classes\\{protocol}\\shell\\open\\command"),
            winreg::enums::KEY_SET_VALUE,
        )?;
        key.set_value("", &command)?;
        Ok(())
    }
}

#[cfg(not(windows))]
impl ProtocolRegistry for NativeRegistry {
    fn read(&self, protocol: &str) -> Result<Option<String>> {
        known_protocol(protocol)?;
        Ok(None)
    }
    fn write(&self, _: &str, _: &str) -> Result<()> {
        bail!("Linux integration is not implemented; no protocol handlers were changed")
    }
}

pub fn load_backup(dir: &Path) -> Result<Option<IntegrationBackup>> {
    let path = dir.join(STATE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let backup: IntegrationBackup = read_json(&path)?;
    ensure!(
        backup.schema_version == 1,
        "Unsupported integration backup version"
    );
    ensure!(
        backup.entries.len() == PROTOCOLS.len(),
        "Integration backup is incomplete"
    );
    for (expected, _) in PROTOCOLS {
        ensure!(
            backup
                .entries
                .iter()
                .filter(|entry| entry.protocol == expected)
                .count()
                == 1,
            "Integration backup has invalid protocols"
        );
    }
    Ok(Some(backup))
}

pub fn status(dir: &Path, registry: &impl ProtocolRegistry) -> Result<IntegrationStatus> {
    let backup = load_backup(dir)?;
    let mut protocols = Vec::new();
    for (name, _) in PROTOCOLS {
        let command = registry.read(name)?;
        let managed = backup
            .as_ref()
            .is_some_and(|backup| command.as_deref() == Some(&backup.owned_command));
        let original_available = backup
            .as_ref()
            .and_then(|backup| backup.entries.iter().find(|entry| entry.protocol == name))
            .is_some_and(|entry| entry.original_target.is_file());
        protocols.push(ProtocolStatus {
            protocol: name.into(),
            registered: command.is_some(),
            managed,
            original_available,
        });
    }
    Ok(IntegrationStatus {
        recovery_available: backup.is_some(),
        enabled: protocols.iter().all(|protocol| protocol.managed),
        protocols,
    })
}

fn lock(dir: &Path) -> Result<File> {
    fs::create_dir_all(dir)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("integration.lock"))?;
    file.try_lock_exclusive()
        .context("Another SOLstrap process is changing launch integration")?;
    Ok(file)
}

pub fn enable(
    dir: &Path,
    installation: &Installation,
    executable: &Path,
    registry: &impl ProtocolRegistry,
) -> Result<()> {
    let _lock = lock(dir)?;
    ensure!(
        installation.detected,
        "Select a valid Ferns installation first"
    );
    ensure!(
        executable.is_file(),
        "The native launcher is missing; build or extract both SOLstrap executables together"
    );
    if load_backup(dir)?.is_some() {
        ensure!(
            status(dir, registry)?.enabled,
            "An earlier integration needs restoring before enabling again"
        );
        return Ok(());
    }
    let owned_command = handler_command(executable)?;
    let mut entries = Vec::new();
    for launcher in &installation.launchers {
        let original_command = registry
            .read(&launcher.protocol)?
            .context("A Ferns website protocol is missing; repair Ferns registration first")?;
        let original_target = parse_original(&original_command)?;
        ensure!(
            same_file_path(&original_target, &launcher.target),
            "A Ferns protocol points to an unexpected launcher; integration was not changed"
        );
        ensure!(
            !same_file_path(&original_target, executable),
            "Refusing a launcher forwarding loop"
        );
        entries.push(BackupEntry {
            protocol: launcher.protocol.clone(),
            original_command,
            original_target,
        });
    }
    let backup = IntegrationBackup {
        schema_version: 1,
        owned_command,
        entries,
    };
    // Save recovery information before the first registry write, including partial installations.
    write_json(&dir.join(STATE_FILE), &backup)?;
    for entry in &backup.entries {
        if let Err(error) = registry.write(&entry.protocol, &backup.owned_command) {
            let recovery = restore_unlocked(dir, registry);
            bail!(
                "Could not enable website integration: {error}. Restore result: {}",
                if recovery.is_ok() {
                    "restored"
                } else {
                    "recovery remains available in SOLstrap"
                }
            );
        }
    }
    Ok(())
}

pub fn disable(dir: &Path, registry: &impl ProtocolRegistry) -> Result<()> {
    let _lock = lock(dir)?;
    restore_unlocked(dir, registry)
}

fn restore_unlocked(dir: &Path, registry: &impl ProtocolRegistry) -> Result<()> {
    let Some(backup) = load_backup(dir)? else {
        return Ok(());
    };
    let mut conflicts = Vec::new();
    for entry in &backup.entries {
        let current = registry.read(&entry.protocol)?;
        if current.as_deref() == Some(&backup.owned_command) {
            registry.write(&entry.protocol, &entry.original_command)?;
        } else if current.as_deref() != Some(&entry.original_command) {
            conflicts.push(entry.protocol.as_str());
        }
    }
    ensure!(conflicts.is_empty(), "Another app changed these handlers: {}. They were left unchanged; the recovery backup was retained", conflicts.join(", "));
    fs::remove_file(dir.join(STATE_FILE))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::Launcher;
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
    };

    struct FakeRegistry {
        values: RefCell<HashMap<String, String>>,
        fail_once: Cell<bool>,
    }
    impl ProtocolRegistry for FakeRegistry {
        fn read(&self, name: &str) -> Result<Option<String>> {
            Ok(self.values.borrow().get(name).cloned())
        }
        fn write(&self, name: &str, command: &str) -> Result<()> {
            if name == "ferns-launcher" && self.fail_once.replace(false) {
                bail!("simulated write failure");
            }
            self.values.borrow_mut().insert(name.into(), command.into());
            Ok(())
        }
    }

    fn fixture() -> (tempfile::TempDir, Installation, PathBuf, FakeRegistry) {
        let temp = tempfile::tempdir().unwrap();
        let mut values = HashMap::new();
        let launchers = PROTOCOLS
            .iter()
            .map(|(name, relative)| {
                let target = temp.path().join(relative);
                fs::create_dir_all(target.parent().unwrap()).unwrap();
                fs::write(&target, b"fixture").unwrap();
                values.insert((*name).into(), handler_command(&target).unwrap());
                Launcher {
                    protocol: (*name).into(),
                    target,
                    exists: true,
                }
            })
            .collect();
        let exe = temp.path().join("solstrap-launcher.exe");
        fs::write(&exe, b"fixture").unwrap();
        let installation = Installation {
            root: temp.path().into(),
            detected: true,
            player: temp.path().join("player.exe"),
            player_bytes: None,
            launchers,
            core_script_files: 0,
        };
        (
            temp,
            installation,
            exe,
            FakeRegistry {
                values: RefCell::new(values),
                fail_once: Cell::new(false),
            },
        )
    }

    #[cfg(windows)]
    #[test]
    fn installation_is_reversible_and_idempotent() {
        let (temp, installation, exe, registry) = fixture();
        let original = registry.values.borrow().clone();
        enable(temp.path(), &installation, &exe, &registry).unwrap();
        enable(temp.path(), &installation, &exe, &registry).unwrap();
        assert!(status(temp.path(), &registry).unwrap().enabled);
        disable(temp.path(), &registry).unwrap();
        assert_eq!(*registry.values.borrow(), original);
        assert!(!temp.path().join(STATE_FILE).exists());
    }

    #[cfg(windows)]
    #[test]
    fn failed_second_write_rolls_back_first() {
        let (temp, installation, exe, registry) = fixture();
        let original = registry.values.borrow().clone();
        registry.fail_once.set(true);
        assert!(enable(temp.path(), &installation, &exe, &registry).is_err());
        assert_eq!(*registry.values.borrow(), original);
        assert!(!temp.path().join(STATE_FILE).exists());
    }

    #[cfg(windows)]
    #[test]
    fn restore_preserves_third_party_changes_and_backup() {
        let (temp, installation, exe, registry) = fixture();
        enable(temp.path(), &installation, &exe, &registry).unwrap();
        registry
            .values
            .borrow_mut()
            .insert("ferns2026".into(), "changed externally".into());
        assert!(disable(temp.path(), &registry).is_err());
        assert_eq!(
            registry.read("ferns2026").unwrap().unwrap(),
            "changed externally"
        );
        assert!(temp.path().join(STATE_FILE).exists());
    }

    #[test]
    fn rejects_backup_with_duplicate_protocols() {
        let temp = tempfile::tempdir().unwrap();
        let entry = BackupEntry {
            protocol: "ferns2026".into(),
            original_command: String::new(),
            original_target: PathBuf::new(),
        };
        write_json(
            &temp.path().join(STATE_FILE),
            &IntegrationBackup {
                schema_version: 1,
                owned_command: String::new(),
                entries: vec![entry.clone(), entry],
            },
        )
        .unwrap();
        assert!(load_backup(temp.path()).is_err());
    }
}
