//! Player accessibility preferences. These are shared with other Windows Roblox clients.
//! Does not provide arbitrary CoreGui colors/positions or change signed resources.
use crate::{read_json, write_json};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const RECOVERY: &str = "interface-preferences-recovery.json";
const KEYS: [(&str, &str); 3] = [
    ("PreferredTextSize", "token"),
    ("PreferredTransparency", "float"),
    ("ReducedMotion", "bool"),
];

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Values {
    pub text_size: Option<u8>,
    pub transparency: Option<f32>,
    pub reduced_motion: Option<bool>,
}

impl Values {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.text_size.is_none_or(|value| (1..=4).contains(&value)),
            "Unsupported player text-size preference"
        );
        ensure!(
            self.transparency
                .is_none_or(|value| value.is_finite() && (0.0..=1.0).contains(&value)),
            "Panel transparency must be between zero and one"
        );
        Ok(())
    }
    fn strings(self) -> [Option<String>; 3] {
        [
            self.text_size.map(|value| value.to_string()),
            self.transparency.map(|value| value.to_string()),
            self.reduced_motion.map(|value| value.to_string()),
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub path: PathBuf,
    pub current: Values,
    pub recovery_available: bool,
    pub conflict: bool,
}

#[derive(Serialize, Deserialize)]
struct Backup {
    version: u32,
    path: PathBuf,
    original: Values,
    applied: Values,
    pending: Option<Values>,
}

impl Backup {
    fn accepts(&self, current: Values) -> bool {
        field_accepts(
            self.original.text_size,
            self.applied.text_size,
            self.pending.map(|v| v.text_size),
            current.text_size,
        ) && field_accepts(
            self.original.transparency,
            self.applied.transparency,
            self.pending.map(|v| v.transparency),
            current.transparency,
        ) && field_accepts(
            self.original.reduced_motion,
            self.applied.reduced_motion,
            self.pending.map(|v| v.reduced_motion),
            current.reduced_motion,
        )
    }

    fn original_for(&self, current: Values) -> Values {
        Values {
            text_size: field_original(
                current.text_size,
                self.original.text_size,
                self.applied.text_size,
                self.pending.map(|v| v.text_size),
            ),
            transparency: field_original(
                current.transparency,
                self.original.transparency,
                self.applied.transparency,
                self.pending.map(|v| v.transparency),
            ),
            reduced_motion: field_original(
                current.reduced_motion,
                self.original.reduced_motion,
                self.applied.reduced_motion,
                self.pending.map(|v| v.reduced_motion),
            ),
        }
    }
}

fn field_owned<T: Copy + PartialEq>(original: T, applied: T, pending: Option<T>) -> bool {
    original != applied || pending.is_some_and(|value| value != original)
}

fn field_accepts<T: Copy + PartialEq>(
    original: T,
    applied: T,
    pending: Option<T>,
    current: T,
) -> bool {
    !field_owned(original, applied, pending)
        || current == original
        || current == applied
        || pending == Some(current)
}

fn field_original<T: Copy + PartialEq>(
    current: T,
    original: T,
    applied: T,
    pending: Option<T>,
) -> T {
    if field_owned(original, applied, pending) {
        original
    } else {
        current
    }
}

pub fn profile_path() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("LOCALAPPDATA").context("Windows player preferences are unavailable")?,
    )
    .join("Roblox/GlobalBasicSettings_13.xml"))
}

fn read(path: &Path) -> Result<String> {
    let info = fs::symlink_metadata(path).context("Player preferences file is missing")?;
    ensure!(
        info.is_file() && !info.file_type().is_symlink() && info.len() <= 2 * 1024 * 1024,
        "Expected a small regular player-preferences file"
    );
    fs::read_to_string(path).context("Player preferences are not UTF-8 XML")
}

type FieldRanges = [Option<(std::ops::Range<usize>, std::ops::Range<usize>)>; 3];

fn fields(xml: &str) -> Result<(Values, FieldRanges, usize)> {
    let doc = roxmltree::Document::parse(xml).context("Cannot parse player preferences")?;
    ensure!(
        doc.root_element().has_tag_name("roblox"),
        "Unexpected preferences document"
    );
    let items: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("Item") && n.attribute("class") == Some("UserGameSettings"))
        .collect();
    ensure!(items.len() == 1, "Expected one UserGameSettings item");
    let properties: Vec<_> = items[0]
        .children()
        .filter(|n| n.has_tag_name("Properties"))
        .collect();
    ensure!(
        properties.len() == 1,
        "Expected one preferences property group"
    );
    let property_range = properties[0].range();
    let closing = "</Properties>";
    ensure!(
        xml[property_range.clone()].ends_with(closing),
        "Expected an expanded Properties element"
    );
    let insertion = property_range.end - closing.len();
    let mut values = Vec::new();
    let mut ranges = Vec::new();
    for (name, tag) in KEYS {
        let matches: Vec<_> = properties[0]
            .children()
            .filter(|n| n.is_element() && n.attribute("name") == Some(name))
            .collect();
        ensure!(matches.len() <= 1, "Ambiguous {name} preference");
        if matches.is_empty() {
            values.push(None);
            ranges.push(None);
            continue;
        }
        ensure!(
            matches[0].has_tag_name(tag),
            "Incorrectly typed {name} preference"
        );
        let text: Vec<_> = matches[0].children().collect();
        ensure!(
            text.len() == 1 && text[0].is_text(),
            "Expected scalar {name} preference"
        );
        values.push(Some(text[0].text().unwrap_or_default().trim()));
        ranges.push(Some((matches[0].range(), text[0].range())));
    }
    let current = Values {
        text_size: values[0]
            .map(str::parse)
            .transpose()
            .context("Invalid text-size preference")?,
        transparency: values[1]
            .map(str::parse)
            .transpose()
            .context("Invalid transparency preference")?,
        reduced_motion: values[2]
            .map(str::parse)
            .transpose()
            .context("Invalid reduced-motion preference")?,
    };
    current.validate()?;
    Ok((
        current,
        ranges
            .try_into()
            .map_err(|_| anyhow::anyhow!("Missing preference ranges"))?,
        insertion,
    ))
}

fn patch(xml: &str, values: Values) -> Result<String> {
    values.validate()?;
    let (_, ranges, insertion) = fields(xml)?;
    let mut edits = Vec::new();
    let mut additions = String::new();
    for ((ranges, value), (name, tag)) in ranges.into_iter().zip(values.strings()).zip(KEYS) {
        match (ranges, value) {
            (Some((_, text)), Some(value)) => edits.push((text, value)),
            (Some((whole, _)), None) => edits.push((whole, String::new())),
            (None, Some(value)) => {
                additions.push_str(&format!("<{tag} name=\"{name}\">{value}</{tag}>"))
            }
            (None, None) => {}
        }
    }
    if !additions.is_empty() {
        edits.push((insertion..insertion, additions));
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut output = xml.to_owned();
    for (range, value) in edits {
        output.replace_range(range, &value);
    }
    ensure!(
        fields(&output)?.0 == values,
        "Preference update failed verification"
    );
    Ok(output)
}

fn backup(dir: &Path, path: &Path) -> Result<Option<Backup>> {
    let file = dir.join(RECOVERY);
    if !file.try_exists()? {
        return Ok(None);
    }
    ensure!(
        fs::metadata(&file)?.len() <= 16 * 1024,
        "Preference recovery document is too large"
    );
    let value: Backup = read_json(&file)?;
    ensure!(
        value.version == 1 && value.path == path,
        "Preference recovery belongs to another profile"
    );
    value.original.validate()?;
    value.applied.validate()?;
    if let Some(pending) = value.pending {
        pending.validate()?;
    }
    Ok(Some(value))
}

pub fn status(dir: &Path, path: &Path) -> Result<Status> {
    let current = fields(&read(path)?)?.0;
    let saved = backup(dir, path)?;
    Ok(Status {
        path: path.into(),
        current,
        recovery_available: saved.is_some(),
        conflict: saved.is_some_and(|b| !b.accepts(current)),
    })
}

fn lock(dir: &Path) -> Result<fs::File> {
    fs::create_dir_all(dir)?;
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("interface-preferences.lock"))?;
    file.try_lock_exclusive()
        .context("Another process is editing interface preferences")?;
    Ok(file)
}

fn replace(
    path: &Path,
    before: &str,
    after: &str,
    check_closed: &impl Fn() -> Result<()>,
) -> Result<()> {
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing profile directory")?)?;
    temporary.write_all(after.as_bytes())?;
    temporary.as_file().sync_all()?;
    check_closed()?;
    ensure!(
        read(path)? == before,
        "Preferences changed during preparation; recovery preserved"
    );
    temporary
        .persist(path)
        .map_err(|e| e.error)
        .context("Cannot replace player preferences")?;
    ensure!(
        read(path)? == after,
        "Preference verification failed; recovery preserved"
    );
    Ok(())
}

fn apply_with(
    dir: &Path,
    path: &Path,
    values: Values,
    check_closed: impl Fn() -> Result<()>,
) -> Result<()> {
    let _lock = lock(dir)?;
    values.validate()?;
    check_closed()?;
    let xml = read(path)?;
    let current = fields(&xml)?.0;
    let previous = backup(dir, path)?;
    if let Some(saved) = &previous {
        ensure!(
            saved.accepts(current),
            "Interface preferences changed outside SOLstrap; preserving those changes and recovery"
        );
    }
    let patched = patch(&xml, values)?;
    let mut staged = Backup {
        version: 1,
        path: path.into(),
        original: previous.map_or(current, |b| b.original_for(current)),
        applied: current,
        pending: Some(values),
    };
    write_json(&dir.join(RECOVERY), &staged)?;
    replace(path, &xml, &patched, &check_closed)?;
    staged.applied = values;
    staged.pending = None;
    write_json(&dir.join(RECOVERY), &staged)
}

fn restore_with(dir: &Path, path: &Path, check_closed: impl Fn() -> Result<()>) -> Result<()> {
    let _lock = lock(dir)?;
    check_closed()?;
    let Some(saved) = backup(dir, path)? else {
        return Ok(());
    };
    let xml = read(path)?;
    let current = fields(&xml)?.0;
    ensure!(
        saved.accepts(current),
        "Interface preferences changed outside SOLstrap; preserving those changes and recovery"
    );
    let restored = saved.original_for(current);
    if current != restored {
        replace(path, &xml, &patch(&xml, restored)?, &check_closed)?;
    }
    fs::remove_file(dir.join(RECOVERY))
        .context("Preferences restored; could not clear recovery")?;
    Ok(())
}

pub fn apply(dir: &Path, path: &Path, values: Values) -> Result<()> {
    apply_with(dir, path, values, require_closed)
}

pub fn restore(dir: &Path, path: &Path) -> Result<()> {
    restore_with(dir, path, require_closed)
}

#[cfg(not(windows))]
fn require_closed() -> Result<()> {
    anyhow::bail!("Editing Windows player preferences requires Windows")
}

#[cfg(windows)]
fn require_closed() -> Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };
    // Snapshot enumeration only; no process writes or privilege changes.
    unsafe {
        let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        ensure!(
            handle != INVALID_HANDLE_VALUE,
            "Cannot check whether the player is closed"
        );
        let result = (|| {
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            ensure!(
                Process32FirstW(handle, &mut entry) != 0,
                "Cannot enumerate player processes"
            );
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                ensure!(!String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case("RobloxPlayerBeta.exe"),
                    "Fully close Ferns and other Roblox players before changing shared interface preferences");
                if Process32NextW(handle, &mut entry) == 0 {
                    ensure!(
                        GetLastError() == ERROR_NO_MORE_FILES,
                        "Player process inventory failed"
                    );
                    break;
                }
            }
            Ok(())
        })();
        CloseHandle(handle);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const XML: &str = "<roblox version=\"4\"><Item class=\"UserGameSettings\"><Properties>\n<token name=\"PreferredTextSize\">1</token><float name=\"PreferredTransparency\">0.5</float><bool name=\"ReducedMotion\">false</bool><float name=\"MasterVolume\">0.7</float><!-- preserve -->\n</Properties></Item></roblox>";
    fn chosen() -> Values {
        Values {
            text_size: Some(3),
            transparency: Some(0.0),
            reduced_motion: Some(true),
        }
    }

    #[test]
    fn changes_only_three_scalar_values_and_rejects_ambiguous_documents() {
        assert_eq!(
            patch(XML, chosen()).unwrap(),
            XML.replace(">1</token>", ">3</token>")
                .replace(">0.5</float>", ">0</float>")
                .replace(">false</bool>", ">true</bool>")
        );
        let duplicate = XML.replace(
            "<!-- preserve -->",
            "<token name=\"PreferredTextSize\">1</token>",
        );
        assert!(patch(&duplicate, chosen()).is_err());
        assert!(patch(
            &XML.replace("<float name=\"PreferredTransparency\">0.5</float>", ""),
            chosen()
        )
        .is_ok());
        assert!(patch(&XML.replace("0.5", "NaN"), chosen()).is_err());
        assert!(patch(
            XML,
            Values {
                text_size: Some(0),
                ..chosen()
            }
        )
        .is_err());
    }

    #[test]
    fn recovery_preserves_later_unrelated_settings_changes() {
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("profile.xml");
        let recovery = dir.path().join("recovery");
        fs::write(&profile, XML).unwrap();
        apply_with(&recovery, &profile, chosen(), || Ok(())).unwrap();
        let later = read(&profile)
            .unwrap()
            .replace(">0.7</float>", ">0.2</float>");
        fs::write(&profile, later).unwrap();
        restore_with(&recovery, &profile, || Ok(())).unwrap();
        assert_eq!(
            read(&profile).unwrap(),
            XML.replace(">0.7</float>", ">0.2</float>")
        );
        assert!(!recovery.join(RECOVERY).exists());
    }

    #[test]
    fn missing_preferences_stay_absent_except_the_selected_value_and_restore_exactly() {
        let xml = "<roblox><Item class=\"UserGameSettings\"><Properties><float name=\"MasterVolume\">0.7</float></Properties></Item></roblox>";
        let values = Values {
            text_size: None,
            transparency: Some(0.0),
            reduced_motion: None,
        };
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("profile.xml");
        let recovery = dir.path().join("recovery");
        fs::write(&profile, xml).unwrap();
        apply_with(&recovery, &profile, values, || Ok(())).unwrap();
        assert_eq!(
            read(&profile).unwrap(),
            xml.replace(
                "</Properties>",
                "<float name=\"PreferredTransparency\">0</float></Properties>"
            )
        );
        assert_eq!(fields(&read(&profile).unwrap()).unwrap().0, values);
        restore_with(&recovery, &profile, || Ok(())).unwrap();
        assert_eq!(read(&profile).unwrap(), xml);
    }

    #[test]
    fn player_serializing_untouched_defaults_does_not_conflict_or_lose_them() {
        let xml = "<roblox><Item class=\"UserGameSettings\"><Properties><float name=\"MasterVolume\">0.7</float></Properties></Item></roblox>";
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("profile.xml");
        let recovery = dir.path().join("recovery");
        fs::write(&profile, xml).unwrap();
        let opacity = Values {
            text_size: None,
            transparency: Some(0.0),
            reduced_motion: None,
        };
        apply_with(&recovery, &profile, opacity, || Ok(())).unwrap();
        let populated = patch(
            &read(&profile).unwrap(),
            Values {
                text_size: Some(1),
                reduced_motion: Some(false),
                ..opacity
            },
        )
        .unwrap();
        fs::write(&profile, &populated).unwrap();
        assert!(!status(&recovery, &profile).unwrap().conflict);
        let large = Values {
            text_size: Some(4),
            transparency: Some(0.0),
            reduced_motion: Some(false),
        };
        apply_with(&recovery, &profile, large, || Ok(())).unwrap();
        let saved = backup(&recovery, &profile).unwrap().unwrap();
        assert_eq!(saved.original.text_size, Some(1));
        assert_eq!(saved.original.transparency, None);
        restore_with(&recovery, &profile, || Ok(())).unwrap();
        assert_eq!(
            fields(&read(&profile).unwrap()).unwrap().0,
            Values {
                text_size: Some(1),
                transparency: None,
                reduced_motion: Some(false)
            }
        );
        assert!(read(&profile)
            .unwrap()
            .contains("<float name=\"MasterVolume\">0.7</float>"));
    }

    #[test]
    fn owned_setting_conflicts_and_running_player_checks_preserve_files() {
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("profile.xml");
        let recovery = dir.path().join("recovery");
        fs::write(&profile, XML).unwrap();
        assert!(apply_with(&recovery, &profile, chosen(), || anyhow::bail!(
            "Player running"
        ))
        .is_err());
        assert_eq!(read(&profile).unwrap(), XML);
        apply_with(&recovery, &profile, chosen(), || Ok(())).unwrap();
        let changed = read(&profile).unwrap().replace(">3</token>", ">4</token>");
        fs::write(&profile, &changed).unwrap();
        assert!(restore_with(&recovery, &profile, || Ok(())).is_err());
        assert_eq!(read(&profile).unwrap(), changed);
        assert!(recovery.join(RECOVERY).exists());
    }

    #[test]
    fn staged_recovery_handles_failed_reapply_and_interrupted_commit() {
        let dir = tempfile::tempdir().unwrap();
        let profile = dir.path().join("profile.xml");
        let recovery = dir.path().join("recovery");
        fs::write(&profile, XML).unwrap();
        apply_with(&recovery, &profile, chosen(), || Ok(())).unwrap();
        let next = Values {
            text_size: Some(4),
            ..chosen()
        };
        let checks = std::cell::Cell::new(0);
        assert!(apply_with(&recovery, &profile, next, || {
            checks.set(checks.get() + 1);
            ensure!(checks.get() < 2, "Player started before replacement");
            Ok(())
        })
        .is_err());
        assert_eq!(fields(&read(&profile).unwrap()).unwrap().0, chosen());
        restore_with(&recovery, &profile, || Ok(())).unwrap();
        assert_eq!(read(&profile).unwrap(), XML);
        // Simulate a completed profile write with the staged record still on disk.
        let original = fields(XML).unwrap().0;
        write_json(
            &recovery.join(RECOVERY),
            &Backup {
                version: 1,
                path: profile.clone(),
                original,
                applied: chosen(),
                pending: Some(next),
            },
        )
        .unwrap();
        fs::write(&profile, patch(XML, next).unwrap()).unwrap();
        restore_with(&recovery, &profile, || Ok(())).unwrap();
        assert_eq!(read(&profile).unwrap(), XML);
    }
}
