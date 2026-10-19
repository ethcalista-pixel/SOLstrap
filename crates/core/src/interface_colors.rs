//! Experimental D3D11 GUI palette remapping for one verified Ferns build.
//! This changes rendering assets, including matching game UI colors. It does
//! not edit scripts, executable code, signature checks, or GUI object layouts.
use anyhow::{ensure, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

mod dxbc;

const ORIGINAL: &str = "63a233028114abdd3e554c12283b0d94c28d7528fe91f77870ac9fdc7597e71f";
const EXE: &str = "ac4f4613a30012cc209b3358b815fcac9a3f47dea9860ebb5f74519ed0fe2958";
const RECORD: &str = "interface-colors.json";
const BACKUP: &str = "interface-colors-original.pack";
const LIMIT: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gradient {
    Solid,
    Linear,
    Radial,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Animation {
    None,
    ColorFade,
    Flow,
    Rainbow,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Effect {
    pub gradient: Gradient,
    pub secondary: [u8; 3],
    pub angle_degrees: u16,
    pub spread_percent: u16,
    pub animation: Animation,
    pub period_ms: u32,
}
impl Default for Effect {
    fn default() -> Self {
        Self {
            gradient: Gradient::Solid,
            secondary: [255, 190, 90],
            angle_degrees: 0,
            spread_percent: 100,
            animation: Animation::None,
            period_ms: 6000,
        }
    }
}
impl Effect {
    pub fn active(self) -> bool {
        self.gradient != Gradient::Solid || self.animation != Animation::None
    }
    pub fn animated(self) -> bool {
        self.animation != Animation::None
    }
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.angle_degrees < 360,
            "Gradient angle must be between 0 and 359 degrees"
        );
        ensure!(
            (25..=400).contains(&self.spread_percent),
            "Gradient spread must be between 25 and 400 percent"
        );
        ensure!(
            (500..=60000).contains(&self.period_ms),
            "Animation cycle must be between 0.5 and 60 seconds"
        );
        ensure!(
            self.animation != Animation::Flow || self.gradient != Gradient::Solid,
            "Choose a linear or radial gradient before enabling flow"
        );
        Ok(())
    }
    /// Native editor preview. Game rendering uses the generated shader equivalent.
    pub fn sample(self, primary: [u8; 3], uv: [f32; 2], seconds: f32) -> [u8; 3] {
        let angle = (self.angle_degrees as f32).to_radians();
        let spread = 100.0 / self.spread_percent.max(1) as f32;
        let coord = match self.gradient {
            Gradient::Solid => 0.0,
            Gradient::Linear => {
                0.5 + ((uv[0] - 0.5) * angle.cos() + (uv[1] - 0.5) * angle.sin()) * spread
            }
            Gradient::Radial => {
                ((uv[0] - 0.5).powi(2) + (uv[1] - 0.5).powi(2)).sqrt() * 2.0 * spread
            }
        };
        let phase = seconds * 1000.0 / self.period_ms.max(1) as f32;
        let cycle = 0.5 - 0.5 * (phase * std::f32::consts::TAU).cos();
        let mut t = coord.clamp(0.0, 1.0);
        match self.animation {
            Animation::None | Animation::Rainbow => {}
            Animation::ColorFade => {
                t = if self.gradient == Gradient::Solid {
                    cycle
                } else {
                    t + (1.0 - 2.0 * t) * cycle
                }
            }
            Animation::Flow => {
                t = 0.5 - 0.5 * ((coord * 0.5 - phase) * std::f32::consts::TAU).cos()
            }
        }
        let color = std::array::from_fn::<_, 3, _>(|i| {
            primary[i] as f32 * (1.0 - t) + self.secondary[i] as f32 * t
        });
        if self.animation == Animation::Rainbow {
            let value = color.into_iter().fold(0.0_f32, f32::max);
            return std::array::from_fn(|i| {
                let offset = [0.0, 2.0 / 3.0, 1.0 / 3.0][i];
                let hue = (phase + coord * 0.25 + offset).rem_euclid(1.0);
                (((hue * 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0) * value)
                    .round()
                    .clamp(0.0, 255.0) as u8
            });
        }
        color.map(|v| v.round().clamp(0.0, 255.0) as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Palette {
    pub background: [u8; 3],
    pub surface: [u8; 3],
    pub accent: [u8; 3],
    pub background_effect: Effect,
    pub surface_effect: Effect,
    pub accent_effect: Effect,
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            background: [33, 17, 9],
            surface: [115, 63, 29],
            accent: [224, 92, 23],
            background_effect: Effect {
                secondary: [65, 30, 16],
                ..Effect::default()
            },
            surface_effect: Effect {
                secondary: [155, 76, 31],
                ..Effect::default()
            },
            accent_effect: Effect::default(),
        }
    }
}
impl Palette {
    pub fn validate(self) -> Result<()> {
        for effect in [
            self.background_effect,
            self.surface_effect,
            self.accent_effect,
        ] {
            effect.validate()?;
        }
        Ok(())
    }
    pub fn animated(self) -> bool {
        [
            self.background_effect,
            self.surface_effect,
            self.accent_effect,
        ]
        .iter()
        .any(|e| e.animated())
    }
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub supported: bool,
    pub installed: bool,
    pub recovery_available: bool,
    pub conflict: bool,
    pub palette: Palette,
}
#[derive(Serialize, Deserialize)]
struct Recovery {
    version: u32,
    path: PathBuf,
    applied: String,
    pending: Option<String>,
    palette: Palette,
}
fn target(root: &Path) -> PathBuf {
    root.join("Player/shaders/shaders_d3d11.pack")
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let info = fs::symlink_metadata(path).context("GUI shader file is missing")?;
    ensure!(
        info.is_file() && !info.file_type().is_symlink() && info.len() <= LIMIT,
        "Expected a regular GUI shader file within the size limit"
    );
    Ok(fs::read(path)?)
}
fn recovery(dir: &Path, root: &Path) -> Result<Option<Recovery>> {
    let file = dir.join(RECORD);
    if !file.try_exists()? {
        return Ok(None);
    }
    ensure!(
        fs::metadata(&file)?.len() < 16384,
        "Color recovery record is too large"
    );
    let r: Recovery = crate::read_json(&file)?;
    ensure!(
        r.version == 1 && r.path == target(root),
        "Color recovery belongs to another installation"
    );
    ensure!(
        hash(&read(&dir.join(BACKUP))?) == ORIGINAL,
        "Color recovery backup failed verification"
    );
    Ok(Some(r))
}
fn managed(r: &Recovery, current: &str) -> bool {
    current == ORIGINAL || current == r.applied || r.pending.as_deref() == Some(current)
}
pub fn status(dir: &Path, root: &Path) -> Result<Status> {
    let current = hash(&read(&target(root))?);
    let r = recovery(dir, root)?;
    let supported = current == ORIGINAL || r.as_ref().is_some_and(|r| managed(r, &current));
    Ok(Status {
        supported,
        installed: supported && current != ORIGINAL,
        recovery_available: r.is_some(),
        conflict: r.as_ref().is_some_and(|r| !managed(r, &current)),
        palette: r.map(|r| r.palette).unwrap_or_default(),
    })
}
fn replace(path: &Path, b: &[u8], expected: &str) -> Result<()> {
    crate::interface_preferences::require_closed()?;
    ensure!(
        hash(&read(path)?) == expected,
        "GUI shaders changed externally; keeping the file and recovery backup"
    );
    let mut temp =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing shader folder")?)?;
    temp.write_all(b)?;
    temp.as_file().sync_all()?;
    crate::interface_preferences::require_closed()?;
    ensure!(
        hash(&read(path)?) == expected,
        "GUI shaders changed while preparing the palette"
    );
    temp.persist(path)
        .map_err(|e| e.error)
        .context("Cannot replace GUI shader pack")?;
    ensure!(
        hash(&read(path)?) == hash(b),
        "GUI shader write did not verify"
    );
    Ok(())
}
fn lock(dir: &Path) -> Result<fs::File> {
    fs::create_dir_all(dir)?;
    let f = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("interface-colors.lock"))?;
    f.try_lock_exclusive()
        .context("Another palette operation is running")?;
    Ok(f)
}
pub fn apply(dir: &Path, root: &Path, palette: Palette) -> Result<()> {
    palette.validate()?;
    let _lock = lock(dir)?;
    crate::interface_preferences::require_closed()?;
    ensure!(
        hash(&fs::read(root.join("Player/RobloxPlayerBeta.exe"))?) == EXE,
        "This color experiment supports only the verified Ferns 2026 build"
    );
    let current = read(&target(root))?;
    let expected = hash(&current);
    let previous = recovery(dir, root)?;
    if let Some(r) = &previous {
        ensure!(
            managed(r, &expected),
            "GUI shaders changed externally; restore or repair the client before applying"
        );
    } else {
        ensure!(
            expected == ORIGINAL,
            "Shader pack does not match the supported original"
        );
    }
    let original = if previous.is_some() {
        read(&dir.join(BACKUP))?
    } else {
        current
    };
    ensure!(
        hash(&original) == ORIGINAL,
        "Original shader pack verification failed"
    );
    let patched = build(&original, palette)?;
    let backup = dir.join(BACKUP);
    if backup.try_exists()? {
        ensure!(
            hash(&read(&backup)?) == ORIGINAL,
            "Existing color backup has changed"
        );
    } else {
        let mut f = tempfile::NamedTempFile::new_in(dir)?;
        f.write_all(&original)?;
        f.as_file().sync_all()?;
        f.persist_noclobber(&backup)
            .map_err(|e| e.error)
            .context("Cannot save original GUI shader backup")?;
    }
    let mut r = Recovery {
        version: 1,
        path: target(root),
        applied: expected.clone(),
        pending: Some(hash(&patched)),
        palette,
    };
    crate::write_json(&dir.join(RECORD), &r)?;
    replace(&r.path, &patched, &expected)?;
    r.applied = hash(&patched);
    r.pending = None;
    crate::write_json(&dir.join(RECORD), &r)
}
pub fn restore(dir: &Path, root: &Path) -> Result<()> {
    let _lock = lock(dir)?;
    crate::interface_preferences::require_closed()?;
    let mut r = recovery(dir, root)?.context("No color recovery backup exists")?;
    let expected = hash(&read(&r.path)?);
    ensure!(
        managed(&r, &expected),
        "GUI shaders changed externally; preserving the current file and recovery backup"
    );
    let original = read(&dir.join(BACKUP))?;
    if expected != ORIGINAL {
        replace(&r.path, &original, &expected)?;
    }
    r.applied = ORIGINAL.into();
    r.pending = None;
    crate::write_json(&dir.join(RECORD), &r)
}

fn u16_at(b: &[u8], p: usize) -> Result<usize> {
    Ok(u16::from_le_bytes(
        b.get(p..p + 2)
            .context("Truncated shader header")?
            .try_into()?,
    ) as usize)
}
fn u32_at(b: &[u8], p: usize) -> Result<usize> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4)
            .context("Truncated shader record")?
            .try_into()?,
    ) as usize)
}
fn build(original: &[u8], palette: Palette) -> Result<Vec<u8>> {
    ensure!(hash(original) == ORIGINAL, "Unsupported shader pack");
    ensure!(
        original.get(..4) == Some(b"RBXS")
            && u16_at(original, 4)? == 11
            && u16_at(original, 6)? == 1,
        "Unsupported shader directory"
    );
    let flags = u16_at(original, 8)?;
    let caps = u16_at(original, 10)?;
    let count = u16_at(original, 12)?;
    let records = u16_at(original, 14)?;
    ensure!(
        (flags, caps, count, records) == (17, 7, 428, 7935),
        "Shader directory counts changed"
    );
    let names = 84;
    let table = names + count * 68 + flags * 64 + caps * 65;
    let data_start = table + records * 64;
    let mapper = dxbc::compile(palette)?;
    let mut result = original.to_vec();
    let mut changed = 0;
    for i in 0..records {
        let rec = table + i * 64;
        let offset = u32_at(original, rec + 16)?;
        let size = u32_at(original, rec + 20)?;
        let name = u16_at(original, rec + 30)?;
        ensure!(
            name < count
                && offset >= data_start
                && size >= 44
                && offset
                    .checked_add(size)
                    .is_some_and(|n| n <= original.len()),
            "Invalid shader record bounds"
        );
        let raw = &original[offset..offset + size];
        ensure!(
            &raw[12..16] == b"DXBC" && u32_at(raw, 36)? == size - 12,
            "Invalid shader payload"
        );
        let name = &original[names + name * 68..names + name * 68 + 64];
        let end = name
            .iter()
            .position(|&c| c == 0)
            .context("Invalid shader name")?;
        if ![b"UIFS".as_slice(), b"UIFSMdrSDBox", b"UIFSAdvSDBox"].contains(&&name[..end]) {
            continue;
        }
        let shader = dxbc::patch(&raw[12..], &mapper)?;
        let new_offset = result.len();
        let mut payload = raw[..12].to_vec();
        payload.extend(shader);
        result[rec..rec + 16].copy_from_slice(&Sha256::digest(&payload)[..16]);
        result[rec + 16..rec + 20].copy_from_slice(&(new_offset as u32).to_le_bytes());
        result[rec + 20..rec + 24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        result.extend(payload);
        changed += 1;
    }
    ensure!(
        changed == 8 && result.len() <= LIMIT as usize,
        "Unexpected GUI shader variants"
    );
    ensure!(
        result[data_start..original.len()] == original[data_start..],
        "An original shader payload changed"
    );
    Ok(result)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn old_palette_json_and_effect_bounds() {
        let p: Palette =
            serde_json::from_str(r#"{"background":[1,2,3],"surface":[4,5,6],"accent":[7,8,9]}"#)
                .unwrap();
        assert!(!p.animated());
        assert!(!p.accent_effect.active());
        assert_eq!(p.accent, [7, 8, 9]);
        for e in [
            Effect {
                period_ms: 0,
                ..Effect::default()
            },
            Effect {
                angle_degrees: 360,
                ..Effect::default()
            },
            Effect {
                spread_percent: 401,
                ..Effect::default()
            },
            Effect {
                animation: Animation::Flow,
                ..Effect::default()
            },
        ] {
            assert!(e.validate().is_err());
        }
        let e = Effect {
            gradient: Gradient::Linear,
            secondary: [0, 0, 255],
            ..Effect::default()
        };
        assert_eq!(e.sample([255, 0, 0], [0.0, 0.5], 0.0), [255, 0, 0]);
        assert_eq!(e.sample([255, 0, 0], [1.0, 0.5], 0.0), [0, 0, 255]);
        let e = Effect {
            animation: Animation::ColorFade,
            ..e
        };
        assert_eq!(e.sample([255, 0, 0], [0.0, 0.5], 3.0), [0, 0, 255]);
        assert_eq!(e.sample([255, 0, 0], [0.0, 0.5], 6.0), [255, 0, 0]);
    }
    #[test]
    #[ignore = "read-only validation using explicitly supplied original private shader pack"]
    fn pinned_pack_effects_compile_read_only() {
        let original = fs::read(
            std::env::var_os("SOLSTRAP_TEST_SHADER_BACKUP").expect("explicit original pack path"),
        )
        .unwrap();
        assert_eq!(
            hash(&build(&original, Palette::default()).unwrap()),
            "41f39f4f721e54fbc48334c2b9ac458a71f5bc1b4c9ef0b1b6b779179c458d09"
        );
        for gradient in [Gradient::Solid, Gradient::Linear, Gradient::Radial] {
            for animation in [
                Animation::None,
                Animation::ColorFade,
                Animation::Flow,
                Animation::Rainbow,
            ] {
                if gradient == Gradient::Solid && animation == Animation::Flow {
                    continue;
                }
                let e = Effect {
                    gradient,
                    animation,
                    ..Effect::default()
                };
                let p = Palette {
                    background_effect: e,
                    surface_effect: e,
                    accent_effect: e,
                    ..Palette::default()
                };
                build(&original, p)
                    .unwrap_or_else(|error| panic!("{gradient:?}/{animation:?}: {error:#}"));
                // Independently enabled roles exercise compiler optimization:
                // a solid animated role can remove position/viewport inputs.
                for role in 0..3 {
                    let mut p = Palette::default();
                    match role {
                        0 => p.background_effect = e,
                        1 => p.surface_effect = e,
                        _ => p.accent_effect = e,
                    }
                    build(&original, p).unwrap_or_else(|error| {
                        panic!("role {role} {gradient:?}/{animation:?}: {error:#}")
                    });
                }
            }
        }
    }
    // Private installed bytes are supplied explicitly and never bundled in Git.
    #[test]
    #[ignore = "requires the pinned local Ferns player, original pack, and closed players"]
    fn pinned_pack_install_update_restore_and_conflict() {
        let player =
            PathBuf::from(std::env::var_os("SOLSTRAP_TEST_PLAYER").expect("explicit player path"));
        let pack = PathBuf::from(
            std::env::var_os("SOLSTRAP_TEST_SHADER_BACKUP").expect("explicit original pack path"),
        );
        let original = fs::read(pack).unwrap();
        assert_eq!(hash(&original), ORIGINAL);
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("client");
        let dir = temp.path().join("recovery");
        fs::create_dir_all(root.join("Player/shaders")).unwrap();
        fs::hard_link(player, root.join("Player/RobloxPlayerBeta.exe")).unwrap();
        fs::write(target(&root), &original).unwrap();
        assert!(status(&dir, &root).unwrap().supported);
        apply(&dir, &root, Palette::default()).unwrap();
        let first = fs::read(target(&root)).unwrap();
        assert_ne!(first, original);
        assert!(status(&dir, &root).unwrap().installed);
        let second = Palette {
            accent: [28, 190, 160],
            ..Palette::default()
        };
        apply(&dir, &root, second).unwrap();
        assert_ne!(fs::read(target(&root)).unwrap(), first);
        assert_eq!(status(&dir, &root).unwrap().palette, second);
        restore(&dir, &root).unwrap();
        assert_eq!(fs::read(target(&root)).unwrap(), original);
        let mut conflict = original.clone();
        conflict.push(42);
        fs::write(target(&root), &conflict).unwrap();
        assert!(status(&dir, &root).unwrap().conflict);
        assert!(apply(&dir, &root, Palette::default()).is_err());
        assert!(restore(&dir, &root).is_err());
        assert_eq!(fs::read(target(&root)).unwrap(), conflict);
        assert_eq!(fs::read(dir.join(BACKUP)).unwrap(), original);
        // Interrupted pending write is recoverable even before applied is committed.
        fs::write(target(&root), &first).unwrap();
        let mut record = recovery(&dir, &root).unwrap().unwrap();
        record.applied = ORIGINAL.into();
        record.pending = Some(hash(&first));
        crate::write_json(&dir.join(RECORD), &record).unwrap();
        restore(&dir, &root).unwrap();
        assert_eq!(fs::read(target(&root)).unwrap(), original);
        // Corrupt backup must prevent both apply and restore.
        fs::write(dir.join(BACKUP), b"invalid backup").unwrap();
        assert!(apply(&dir, &root, Palette::default()).is_err());
        assert!(restore(&dir, &root).is_err());
        assert_eq!(fs::read(target(&root)).unwrap(), original);
    }
}
