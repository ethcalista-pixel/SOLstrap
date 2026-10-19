//! Named, local palette documents. Saving/importing never applies client changes.
use crate::{interface_colors::Palette, read_json, write_json};
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
const LIMIT: u64 = 512 * 1024;
const MAX: usize = 64;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub schema_version: u32,
    pub name: String,
    pub palette: Palette,
}
impl Theme {
    pub fn new(name: &str, palette: Palette) -> Result<Self> {
        let theme = Self {
            schema_version: 1,
            name: name.trim().into(),
            palette,
        };
        theme.validate()?;
        Ok(theme)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == 1, "Unsupported theme version");
        ensure!(!self.name.is_empty() && self.name.trim()==self.name && self.name.chars().count()<=48 && !self.name.chars().any(char::is_control),"Theme name must contain 1–48 characters without leading/trailing spaces or control characters");
        self.palette.validate()
    }
    pub fn read(path: &Path) -> Result<Self> {
        ensure!(
            fs::metadata(path)?.len() <= 64 * 1024,
            "Theme files must be smaller than 64 KiB"
        );
        let theme: Self = read_json(path)?;
        theme.validate()?;
        Ok(theme)
    }
    pub fn export(&self, path: &Path) -> Result<()> {
        self.validate()?;
        write_json(path, self)
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Library {
    schema_version: u32,
    themes: Vec<Theme>,
}
pub fn load(dir: &Path) -> Result<Vec<Theme>> {
    let path = dir.join("themes.json");
    if !path.try_exists()? {
        return Ok(Vec::new());
    }
    ensure!(
        fs::metadata(&path)?.len() <= LIMIT,
        "Theme library exceeds size limit"
    );
    let library: Library = read_json(&path)?;
    ensure!(
        library.schema_version == 1 && library.themes.len() <= MAX,
        "Unsupported theme library version or size"
    );
    for (i, theme) in library.themes.iter().enumerate() {
        theme.validate()?;
        ensure!(
            !library.themes[..i]
                .iter()
                .any(|t| key(&t.name) == key(&theme.name)),
            "Theme library contains duplicate names"
        );
    }
    Ok(library.themes)
}
fn key(name: &str) -> String {
    name.to_lowercase()
}
fn edit<T>(dir: &Path, change: impl FnOnce(&mut Vec<Theme>) -> Result<T>) -> Result<T> {
    fs::create_dir_all(dir)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("themes.lock"))?;
    lock.try_lock_exclusive()
        .context("Another theme operation is running")?;
    let mut themes = load(dir)?;
    let value = change(&mut themes)?;
    themes.sort_by_key(|t| key(&t.name));
    write_json(
        &dir.join("themes.json"),
        &Library {
            schema_version: 1,
            themes,
        },
    )?;
    Ok(value)
}
pub fn save_new(dir: &Path, theme: Theme) -> Result<()> {
    theme.validate()?;
    edit(dir, |themes| {
        ensure!(themes.len() < MAX, "Theme library is full (64 themes)");
        ensure!(
            !themes.iter().any(|t| key(&t.name) == key(&theme.name)),
            "That theme name already exists; choose another name or Update selected"
        );
        themes.push(theme);
        Ok(())
    })
}
pub fn update(dir: &Path, selected: &Theme, replacement: Theme) -> Result<()> {
    replacement.validate()?;
    edit(dir, |themes| {
        let at = themes
            .iter()
            .position(|t| key(&t.name) == key(&selected.name))
            .context("Selected theme no longer exists; reload themes")?;
        ensure!(
            themes[at] == *selected,
            "Selected theme changed in another window; reload themes before updating"
        );
        ensure!(
            !themes
                .iter()
                .enumerate()
                .any(|(i, t)| i != at && key(&t.name) == key(&replacement.name)),
            "Another theme already uses that name"
        );
        themes[at] = replacement;
        Ok(())
    })
}
pub fn import(dir: &Path, mut theme: Theme) -> Result<Theme> {
    theme.validate()?;
    edit(dir, |themes| {
        ensure!(themes.len() < MAX, "Theme library is full (64 themes)");
        let base = theme.name.clone();
        let mut n = 2;
        while themes.iter().any(|t| key(&t.name) == key(&theme.name)) {
            let suffix = format!(" ({n})");
            theme.name = format!(
                "{}{}",
                base.chars()
                    .take(48 - suffix.chars().count())
                    .collect::<String>(),
                suffix
            );
            n += 1;
        }
        theme.validate()?;
        themes.push(theme.clone());
        Ok(theme)
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface_colors::{Animation, Effect, Gradient};
    #[test]
    fn full_effect_roundtrip_and_duplicate_import() {
        let dir = tempfile::tempdir().unwrap();
        let p = Palette {
            accent_effect: Effect {
                gradient: Gradient::Radial,
                animation: Animation::Flow,
                secondary: [1, 2, 3],
                angle_degrees: 271,
                spread_percent: 260,
                period_ms: 3750,
            },
            ..Palette::default()
        };
        let t = Theme::new("Solar flux", p).unwrap();
        save_new(dir.path(), t.clone()).unwrap();
        assert!(save_new(dir.path(), t.clone()).is_err());
        let file = dir.path().join("export.json");
        t.export(&file).unwrap();
        assert_eq!(Theme::read(&file).unwrap(), t);
        let imported = import(dir.path(), Theme::read(&file).unwrap()).unwrap();
        assert_eq!(imported.name, "Solar flux (2)");
        assert_eq!(imported.palette, p);
        assert_eq!(load(dir.path()).unwrap().len(), 2);
        let renamed = Theme::new("Corona", Palette::default()).unwrap();
        update(dir.path(), &t, renamed.clone()).unwrap();
        assert!(update(dir.path(), &t, t.clone()).is_err());
        assert!(load(dir.path()).unwrap().contains(&renamed));
    }
    #[test]
    fn corrupt_library_and_stale_edits_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let t = Theme::new("Sun", Palette::default()).unwrap();
        save_new(dir.path(), t.clone()).unwrap();
        let mut changed = t.clone();
        changed.palette.accent = [1, 2, 3];
        update(dir.path(), &t, changed.clone()).unwrap();
        assert!(update(dir.path(), &t, t.clone()).is_err());
        assert_eq!(load(dir.path()).unwrap(), vec![changed]);
        let bad = b"broken library";
        fs::write(dir.path().join("themes.json"), bad).unwrap();
        assert!(save_new(dir.path(), t).is_err());
        assert_eq!(fs::read(dir.path().join("themes.json")).unwrap(), bad);
    }
    #[test]
    fn reject_invalid_documents_and_bound_names() {
        assert!(Theme::new(" ", Palette::default()).is_err());
        assert!(Theme::new(&"x".repeat(49), Palette::default()).is_err());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        for value in [
            serde_json::json!({"schema_version":2,"name":"Sun","palette":Palette::default()}),
            serde_json::json!({"schema_version":1,"name":"Sun","palette":Palette::default(),"run":"ignored.exe"}),
        ] {
            fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(Theme::read(&path).is_err());
        }
        let mut t = Theme::new("Sun", Palette::default()).unwrap();
        t.palette.accent_effect.period_ms = 0;
        assert!(save_new(dir.path(), t).is_err());
        assert!(!dir.path().join("themes.json").exists());
    }
}
