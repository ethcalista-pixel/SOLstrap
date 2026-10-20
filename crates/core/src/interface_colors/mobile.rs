//! Independently recoverable palette support for the pinned Android Vulkan pack.
use super::*;
const ORIGINAL: &str = "3d240d4050394ce1661caee12a5dd8ccb6d2f46fd06fd8bb2008cd0aea0b6b9b";
const LIBRARY: &str = "77eb1fb8d6386139745fe58fb942d44e093dcfa72b27f952dbc85c55a63b0779";
const RECORD: &str = "interface-colors-mobile-vulkan.json";
const BACKUP: &str = "interface-colors-mobile-vulkan-original.pack";
fn target(root: &Path) -> PathBuf {
    root.join(
        "Player/root/data/user/0/com.ferns.client/app_assets/shaders/shaders_vulkan_mobile.pack",
    )
}
fn recovery(dir: &Path, root: &Path) -> Result<Option<Recovery>> {
    let file = dir.join(RECORD);
    if !file.try_exists()? {
        return Ok(None);
    }
    ensure!(
        fs::metadata(&file)?.len() < 16384,
        "Mobile color recovery record is too large"
    );
    let r: Recovery = crate::read_json(&file)?;
    ensure!(
        r.version == 1 && r.path == target(root),
        "Mobile color recovery belongs to another installation"
    );
    ensure!(
        hash(&read(&dir.join(BACKUP))?) == ORIGINAL,
        "Mobile shader backup failed verification"
    );
    Ok(Some(r))
}
fn managed(r: &Recovery, current: &str) -> bool {
    current == ORIGINAL || current == r.applied || r.pending.as_deref() == Some(current)
}
pub(super) fn status(dir: &Path, root: &Path) -> Result<Status> {
    let current = hash(&read(&target(root))?);
    let r = recovery(dir, root)?;
    let supported = current == ORIGINAL || r.as_ref().is_some_and(|r| managed(r, &current));
    Ok(Status {
        target: target(root),
        backend: "Android Vulkan",
        current_hash: current.clone(),
        supported,
        installed: supported && current != ORIGINAL,
        recovery_available: r.is_some(),
        conflict: r.as_ref().is_some_and(|r| !managed(r, &current)),
        needs_reapply: current == ORIGINAL && r.as_ref().is_some_and(|r| r.applied != ORIGINAL),
        palette: r.map(|r| r.palette).unwrap_or_default(),
    })
}
pub(super) fn apply(dir: &Path, root: &Path, palette: Palette) -> Result<()> {
    palette.validate()?;
    let _lock = lock(dir)?;
    crate::interface_preferences::require_closed()?;
    ensure!(
        crate::discovery::fingerprint(
            &root.join("Player/root/data/app/com.ferns.client/lib/x86_64/libroblox.so")
        )? == LIBRARY,
        "This palette supports only the verified mobile client build"
    );
    let current = read(&target(root))?;
    let expected = hash(&current);
    let previous = recovery(dir, root)?;
    if let Some(r) = &previous {
        ensure!(
            managed(r, &expected),
            "Mobile shader pack changed externally; preserving current files"
        );
    } else {
        ensure!(expected == ORIGINAL, "Unsupported mobile shader pack");
    }
    let original = if previous.is_some() {
        read(&dir.join(BACKUP))?
    } else {
        current
    };
    ensure!(
        hash(&original) == ORIGINAL,
        "Mobile shader original failed verification"
    );
    let patched = build(&original, palette)?;
    let backup = dir.join(BACKUP);
    if backup.try_exists()? {
        ensure!(
            hash(&read(&backup)?) == ORIGINAL,
            "Existing mobile shader backup changed"
        );
    } else {
        let mut f = tempfile::NamedTempFile::new_in(dir)?;
        f.write_all(&original)?;
        f.as_file().sync_all()?;
        f.persist_noclobber(&backup)
            .map_err(|e| e.error)
            .context("Cannot save mobile shader backup")?;
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
pub(super) fn restore(dir: &Path, root: &Path) -> Result<()> {
    let _lock = lock(dir)?;
    crate::interface_preferences::require_closed()?;
    let mut r = recovery(dir, root)?.context("No mobile palette recovery exists")?;
    let expected = hash(&read(&r.path)?);
    ensure!(
        managed(&r, &expected),
        "Mobile shader pack changed externally; preserving current files"
    );
    let original = read(&dir.join(BACKUP))?;
    if expected != ORIGINAL {
        replace(&r.path, &original, &expected)?;
    }
    r.applied = ORIGINAL.into();
    r.pending = None;
    crate::write_json(&dir.join(RECORD), &r)
}
fn build(original: &[u8], palette: Palette) -> Result<Vec<u8>> {
    ensure!(hash(original) == ORIGINAL, "Unsupported mobile shader pack");
    palette.validate()?;
    ensure!(
        original.get(..4) == Some(b"RBXS")
            && (
                u16_at(original, 4)?,
                u16_at(original, 6)?,
                u16_at(original, 8)?,
                u16_at(original, 10)?,
                u16_at(original, 12)?,
                u16_at(original, 14)?
            ) == (11, 1, 18, 7, 433, 4720),
        "Mobile shader directory changed"
    );
    let names = 84;
    let table = names + 433 * 68 + 18 * 64 + 7 * 65;
    let data_start = table + 4720 * 64;
    let mut result = original.to_vec();
    let mut changed = 0;
    for i in 0..4720 {
        let rec = table + i * 64;
        let offset = u32_at(original, rec + 16)?;
        let size = u32_at(original, rec + 20)?;
        let name = u16_at(original, rec + 30)?;
        ensure!(
            name < 433
                && offset >= data_start
                && size >= 24
                && offset
                    .checked_add(size)
                    .is_some_and(|n| n <= original.len()),
            "Invalid mobile shader bounds"
        );
        let name = &original[names + name * 68..names + name * 68 + 64];
        let end = name
            .iter()
            .position(|c| *c == 0)
            .context("Invalid mobile shader name")?;
        if ![b"UIFS".as_slice(), b"UIFSMdrSDBox", b"UIFSAdvSDBox"].contains(&&name[..end]) {
            continue;
        }
        let raw = &original[offset..offset + size];
        ensure!(
            [2, 3, 6, 7].contains(&u32_at(raw, 0)?),
            "Unexpected GUI constant buffer mask"
        );
        let shader = super::spirv::patch(&raw[24..], palette)?;
        let mut payload = raw[..24].to_vec();
        let buffers = (u32_at(raw, 0)? as u32) | 1;
        payload[..4].copy_from_slice(&buffers.to_le_bytes());
        payload.extend(shader);
        let new_offset = result.len();
        result[rec..rec + 16].copy_from_slice(&Sha256::digest(&payload)[..16]);
        result[rec + 16..rec + 20].copy_from_slice(&(new_offset as u32).to_le_bytes());
        result[rec + 20..rec + 24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        result.extend(payload);
        changed += 1;
    }
    ensure!(
        changed == 8 && result.len() <= LIMIT as usize,
        "Unexpected mobile GUI variants"
    );
    ensure!(
        result[data_start..original.len()] == original[data_start..],
        "Original mobile payload changed"
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires explicit private mobile pack and Khronos validation executable"]
    fn pinned_mobile_variants_validate() {
        let source = PathBuf::from(
            std::env::var_os("SOLSTRAP_TEST_MOBILE_PACK").expect("explicit mobile pack"),
        );
        let validator =
            PathBuf::from(std::env::var_os("SOLSTRAP_TEST_SPIRV_VAL").expect("explicit validator"));
        let original = fs::read(source).unwrap();
        let mut palettes = vec![Palette::default()];
        for gradient in [Gradient::Linear, Gradient::Radial] {
            for animation in [
                Animation::None,
                Animation::ColorFade,
                Animation::Flow,
                Animation::Rainbow,
            ] {
                let mut p = Palette::default();
                for e in [
                    &mut p.background_effect,
                    &mut p.surface_effect,
                    &mut p.accent_effect,
                ] {
                    e.gradient = gradient;
                    e.animation = animation;
                }
                palettes.push(p);
            }
        }
        let temp = tempfile::tempdir().unwrap();
        for p in palettes {
            let patched = build(&original, p).unwrap();
            let table = 84 + 433 * 68 + 18 * 64 + 7 * 65;
            let mut count = 0;
            for i in 0..4720 {
                let rec = table + i * 64;
                let offset = u32_at(&patched, rec + 16).unwrap();
                if offset < original.len() {
                    continue;
                }
                let size = u32_at(&patched, rec + 20).unwrap();
                let file = temp.path().join(format!("{i}.spv"));
                fs::write(&file, &patched[offset + 24..offset + size]).unwrap();
                let out = std::process::Command::new(&validator)
                    .args(["--target-env", "vulkan1.0"])
                    .arg(&file)
                    .output()
                    .unwrap();
                assert!(
                    out.status.success(),
                    "variant {i}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                count += 1;
            }
            assert_eq!(count, 8);
        }
        // Recovery ownership must follow committed or interrupted writes, and
        // must never adopt a pack changed by another program.
        let root = temp.path().join("client");
        let state = temp.path().join("state");
        fs::create_dir_all(target(&root).parent().unwrap()).unwrap();
        fs::create_dir(&state).unwrap();
        fs::write(target(&root), &original).unwrap();
        fs::write(state.join(BACKUP), &original).unwrap();
        let patched = build(&original, Palette::default()).unwrap();
        let mut record = Recovery {
            version: 1,
            path: target(&root),
            applied: ORIGINAL.into(),
            pending: Some(hash(&patched)),
            palette: Palette::default(),
        };
        crate::write_json(&state.join(RECORD), &record).unwrap();
        assert!(!status(&state, &root).unwrap().installed);
        fs::write(target(&root), &patched).unwrap();
        let pending = status(&state, &root).unwrap();
        assert!(pending.supported && pending.installed && !pending.conflict);
        record.applied = hash(&patched);
        record.pending = None;
        crate::write_json(&state.join(RECORD), &record).unwrap();
        assert!(status(&state, &root).unwrap().installed);
        fs::write(target(&root), &original).unwrap();
        assert!(status(&state, &root).unwrap().needs_reapply);
        fs::write(target(&root), b"external change").unwrap();
        assert!(status(&state, &root).unwrap().conflict);
        assert_eq!(fs::read(target(&root)).unwrap(), b"external change");
        assert_eq!(fs::read(state.join(BACKUP)).unwrap(), original);
        fs::write(state.join(BACKUP), b"corrupt backup").unwrap();
        assert!(status(&state, &root).is_err());
        assert!(build(b"unsupported", Palette::default()).is_err());
    }
}
