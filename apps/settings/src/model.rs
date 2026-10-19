use anyhow::{ensure, Context, Result};
use serde::Serialize;
use solstrap_core::{
    coregui_loader_trial,
    coregui_probe::{self, ProbeStatus},
    crashes, data_dir,
    diagnostics::{self, CoreGuiReport},
    discovery::{self, Installation},
    health::{self, HealthReport},
    integration::{self, IntegrationStatus, NativeRegistry},
    interface_colors, interface_font, interface_preferences,
    settings::Settings,
};
use std::path::PathBuf;

/// An editor keeps its own baseline so refreshes never discard unsaved fields.
pub struct InterfaceDraft {
    pub values: interface_preferences::Values,
    baseline: interface_preferences::Values,
}

impl InterfaceDraft {
    pub fn new(current: interface_preferences::Values) -> Self {
        Self {
            values: current,
            baseline: current,
        }
    }

    pub fn refresh(&mut self, current: interface_preferences::Values) {
        merge_field(
            &mut self.values.text_size,
            &mut self.baseline.text_size,
            current.text_size,
        );
        merge_field(
            &mut self.values.transparency,
            &mut self.baseline.transparency,
            current.transparency,
        );
        merge_field(
            &mut self.values.reduced_motion,
            &mut self.baseline.reduced_motion,
            current.reduced_motion,
        );
    }

    pub fn dirty(&self) -> bool {
        self.values != self.baseline
    }

    pub fn conflict(&self, current: interface_preferences::Values) -> bool {
        field_conflict(
            self.values.text_size,
            self.baseline.text_size,
            current.text_size,
        ) || field_conflict(
            self.values.transparency,
            self.baseline.transparency,
            current.transparency,
        ) || field_conflict(
            self.values.reduced_motion,
            self.baseline.reduced_motion,
            current.reduced_motion,
        )
    }
}

fn merge_field<T: Copy + PartialEq>(draft: &mut T, baseline: &mut T, current: T) {
    if *draft == *baseline || *draft == current {
        *draft = current;
        *baseline = current;
    }
}

fn field_conflict<T: PartialEq>(draft: T, baseline: T, current: T) -> bool {
    draft != baseline && current != baseline && current != draft
}

#[cfg(test)]
mod draft_tests {
    use super::*;

    fn original() -> interface_preferences::Values {
        interface_preferences::Values {
            text_size: Some(1),
            transparency: Some(0.5),
            reduced_motion: None,
        }
    }

    #[test]
    fn focus_refresh_and_failed_apply_keep_unsaved_choices() {
        let mut draft = InterfaceDraft::new(original());
        draft.values.text_size = Some(4);
        draft.values.transparency = Some(0.0);
        let chosen = draft.values;
        for _ in 0..3 {
            draft.refresh(original());
            assert_eq!(draft.values, chosen);
            assert!(draft.dirty());
            assert!(!draft.conflict(original()));
        }
    }

    #[test]
    fn unrelated_external_fields_update_without_losing_edits() {
        let mut draft = InterfaceDraft::new(original());
        draft.values.text_size = Some(4);
        let current = interface_preferences::Values {
            transparency: None,
            reduced_motion: Some(true),
            ..original()
        };
        draft.refresh(current);
        assert_eq!(draft.values.text_size, Some(4));
        assert_eq!(draft.values.transparency, None);
        assert_eq!(draft.values.reduced_motion, Some(true));
        assert!(draft.dirty());
        assert!(!draft.conflict(current));
    }

    #[test]
    fn same_field_external_change_stays_conflicted_until_discard() {
        let mut draft = InterfaceDraft::new(original());
        draft.values.text_size = Some(4);
        let current = interface_preferences::Values {
            text_size: Some(2),
            ..original()
        };
        for _ in 0..3 {
            draft.refresh(current);
            assert_eq!(draft.values.text_size, Some(4));
            assert!(draft.conflict(current));
        }
        draft = InterfaceDraft::new(current);
        assert!(!draft.dirty());
        assert!(!draft.conflict(current));
    }

    #[test]
    fn successful_apply_marks_clean_and_later_external_values_load() {
        let mut draft = InterfaceDraft::new(original());
        draft.values.text_size = Some(4);
        draft.values.transparency = None;
        draft.refresh(draft.values);
        assert!(!draft.dirty());
        draft.refresh(original());
        assert_eq!(draft.values, original());
        assert!(!draft.dirty());
    }

    #[test]
    fn reverting_a_conflicted_edit_loads_current_external_value() {
        let mut draft = InterfaceDraft::new(original());
        draft.values.transparency = None;
        let current = interface_preferences::Values {
            transparency: Some(0.9),
            ..original()
        };
        draft.refresh(current);
        assert!(draft.conflict(current));
        draft.values.transparency = original().transparency;
        draft.refresh(current);
        assert_eq!(draft.values, current);
        assert!(!draft.dirty());
    }
}

#[derive(Serialize)]
pub struct Snapshot {
    pub data_directory: PathBuf,
    pub settings: Settings,
    pub installation: Installation,
    pub integration: IntegrationStatus,
    pub coregui: CoreGuiReport,
    pub health: HealthReport,
    pub crash_sessions: Vec<crashes::Session>,
    pub crash_sessions_error: Option<String>,
    pub probe: Option<ProbeStatus>,
    pub probe_error: Option<String>,
    pub loader_trial: Option<coregui_loader_trial::Status>,
    pub loader_trial_error: Option<String>,
    pub font_test: Option<interface_font::Status>,
    pub font_test_error: Option<String>,
    pub interface_preferences: Option<interface_preferences::Status>,
    pub interface_preferences_error: Option<String>,
    pub interface_colors: Option<interface_colors::Status>,
    pub interface_colors_error: Option<String>,
    pub launcher_available: bool,
    pub version: &'static str,
    pub platform: &'static str,
}

pub fn launcher_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    Ok(exe
        .parent()
        .context("Cannot locate the executable folder")?
        .join(if cfg!(windows) {
            "solstrap-launcher.exe"
        } else {
            "solstrap-launcher"
        }))
}

pub fn snapshot() -> Result<Snapshot> {
    let dir = data_dir()?;
    let settings = Settings::load(&dir)?;
    let installation = discovery::inspect(&settings)?;
    let shared_logs =
        std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("Roblox/logs"));
    let coregui =
        diagnostics::inspect_coregui_with_shared(&installation.root, shared_logs.as_deref());
    let health = health::inspect(&installation, &coregui);
    let (crash_sessions, crash_sessions_error) = match crashes::history(&dir) {
        Ok(s) => (s, None),
        Err(e) => (vec![], Some(format!("{e:#}"))),
    };
    let (probe, probe_error) = match coregui_probe::status(&dir, &installation.root) {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(format!("{error:#}"))),
    };
    let (font_test, font_test_error) = match interface_font::status(&dir, &installation.root) {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(format!("{error:#}"))),
    };
    let (interface_colors, interface_colors_error) =
        match interface_colors::status(&dir, &installation.root) {
            Ok(status) => (Some(status), None),
            Err(error) => (None, Some(format!("{error:#}"))),
        };
    let (loader_trial, loader_trial_error) =
        match coregui_loader_trial::status(&dir, &installation.root) {
            Ok(status) => (Some(status), None),
            Err(error) => (None, Some(format!("{error:#}"))),
        };
    let (interface_preferences, interface_preferences_error) =
        match interface_preferences::profile_path()
            .and_then(|path| interface_preferences::status(&dir, &path))
        {
            Ok(status) => (Some(status), None),
            Err(error) => (None, Some(format!("{error:#}"))),
        };
    Ok(Snapshot {
        data_directory: dir.clone(),
        installation,
        coregui,
        health,
        crash_sessions,
        crash_sessions_error,
        probe,
        probe_error,
        loader_trial,
        loader_trial_error,
        font_test,
        font_test_error,
        interface_preferences,
        interface_preferences_error,
        interface_colors,
        interface_colors_error,
        integration: integration::status(&dir, &NativeRegistry)?,
        launcher_available: launcher_path()?.is_file(),
        settings,
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
    })
}

pub fn save(settings: &Settings) -> Result<()> {
    let dir = data_dir()?;
    let previous = Settings::load(&dir)?;
    if previous.client_root != settings.client_root {
        ensure!(
            integration::load_backup(&dir)?.is_none(),
            "Restore website integration before changing the client location"
        );
        let current = discovery::inspect(&previous)?;
        if dir.join("interface-colors.json").try_exists()? {
            let colors = interface_colors::status(&dir, &current.root)?;
            ensure!(
                !colors.installed && !colors.conflict,
                "Restore GUI colors before changing the client location"
            );
        }
        ensure!(
            !coregui_probe::status(&dir, &current.root)?.recovery_available,
            "Restore the CoreGui test script before changing the client location"
        );
    }
    settings.validate()?;
    if settings.client_root.is_some() {
        ensure!(
            discovery::inspect(settings)?.detected,
            "This folder does not contain Player/RobloxPlayerBeta.exe"
        );
    }
    settings.save(&dir)
}

pub fn apply_interface_colors(palette: interface_colors::Palette) -> Result<()> {
    let dir = data_dir()?;
    let installation = discovery::inspect(&Settings::load(&dir)?)?;
    interface_colors::apply(&dir, &installation.root, palette)
}
pub fn restore_interface_colors() -> Result<()> {
    let dir = data_dir()?;
    let installation = discovery::inspect(&Settings::load(&dir)?)?;
    interface_colors::restore(&dir, &installation.root)
}

pub fn apply_interface_preferences(
    values: interface_preferences::Values,
    expected: interface_preferences::Values,
) -> Result<()> {
    interface_preferences::apply_if_current(
        &data_dir()?,
        &interface_preferences::profile_path()?,
        values,
        expected,
    )
}

pub fn restore_interface_preferences() -> Result<()> {
    interface_preferences::restore(&data_dir()?, &interface_preferences::profile_path()?)
}

pub fn recover_warm_palette(original: &std::path::Path) -> Result<()> {
    let dir = data_dir()?;
    let installation = discovery::inspect(&Settings::load(&dir)?)?;
    interface_colors::recover_warm(&dir, &installation.root, original)
}

pub fn set_integration(enabled: bool) -> Result<()> {
    let dir = data_dir()?;
    if enabled {
        let installation = discovery::inspect(&Settings::load(&dir)?)?;
        integration::enable(&dir, &installation, &launcher_path()?, &NativeRegistry)
    } else {
        integration::disable(&dir, &NativeRegistry)
    }
}

pub fn set_probe(enabled: bool) -> Result<()> {
    let dir = data_dir()?;
    let installation = discovery::inspect(&Settings::load(&dir)?)?;
    if enabled {
        ensure!(
            installation.detected,
            "Select a valid Ferns installation first"
        );
        coregui_loader_trial::install_with_probe(&dir, &installation.root)
    } else {
        coregui_loader_trial::restore_with_probe(&dir, &installation.root)
    }
}

pub fn set_font_test(enabled: bool) -> Result<()> {
    let dir = data_dir()?;
    let installation = discovery::inspect(&Settings::load(&dir)?)?;
    if enabled {
        interface_font::install(&dir, &installation.root)
    } else {
        interface_font::restore(&dir, &installation.root)
    }
}

#[derive(Serialize)]
pub struct Feature {
    pub name: &'static str,
    pub category: &'static str,
    pub status: &'static str,
    pub description: &'static str,
}

pub const FEATURES: &[Feature] = &[
    Feature { name: "CoreGui compatibility test", category: "Interface", status: "No result on tested build", description: "The loose-script test produced no label or execution marker on Ferns 0.724. The active interface uses the compiled InExperience bundle. Restore controls remain available." },
    Feature { name: "Interface font experiment", category: "Interface", status: "Confirmed on tested build", description: "The user confirmed monospaced interface text on Ferns 0.724. Reversibly maps Builder Sans to bundled Builder Mono; game text using the same font may also change." },
    Feature { name: "Player accessibility preferences", category: "Interface", status: "Text size and transparency confirmed", description: "The user confirmed Largest text and an opaque Escape-menu panel on the tested build. Reduced motion remains unverified. Changes have recovery and conflict checks and are shared with other Windows Roblox players; close the player before applying." },
    Feature { name: "GUI palette", category: "Interface", status: "Experimental · user-tested gradients and animation", description: "Native color pickers and hex input recolor dark GUI surfaces and blue accents. Linear/radial gradients, color fade, gradient flow, and rainbow effects have offline Direct3D validation; the user confirmed their gradient and animation checks work in Ferns. Gradients span the viewport. Matching game UI and image colors can change. Requires a closed player, restart, and the pinned Ferns build; exact original-file recovery is available." },
    Feature { name: "Saved palette themes", category: "Interface", status: "Implemented", description: "Save named color/gradient/animation themes, switch the editor between them, update or rename a selection, and import/export versioned JSON. Imports preserve existing names with a numbered suffix. Saving, selecting, and importing do not change Ferns; Apply GUI palette is separate." },
    Feature { name: "Player-log evidence", category: "Diagnostics", status: "Available", description: "Bounded scanning of recent Ferns player logs with structured script, test, and exception markers. Raw log text is not exported." },
    Feature { name: "Website launch integration", category: "Client", status: "Available", description: "Forwards website joins to the original Ferns launchers. Requires both original launchers and valid registrations." },
    Feature { name: "Preferences and import/export", category: "Settings", status: "Available", description: "Validated, versioned global settings, native file dialogs, and local storage. No per-game profiles." },
    Feature { name: "Client fingerprint", category: "Diagnostics", status: "Available", description: "On-demand SHA-256 identification. Authenticity verification needs an authoritative manifest." },
    Feature { name: "CoreGui themes and layouts", category: "Interface", status: "Object-level integration unresolved", description: "The experimental GUI palette changes shared rendering. Per-component CoreGui properties and free positioning still need an integration point. Signed-bundle edits caused fatal verification errors and were restored; menu-selector flag trials had no demonstrated effect." },
    Feature { name: "Theme preview and accessibility", category: "Interface", status: "Planned", description: "Preview, import/export, per-component reset, larger text, contrast, and reduced motion for the game interface." },
    Feature { name: "Graphics presets and telemetry", category: "Graphics", status: "Investigating", description: "Validated graphics settings, average FPS, frame times, and memory measurements. No unverified client flags are applied." },
    Feature { name: "RoShade", category: "Graphics", status: "Investigating", description: "Compatibility, licensing, performance, and reversible installation of the actual requested package." },
    Feature { name: "DLSS frame generation", category: "Graphics", status: "Research", description: "Needs a viable engine integration and supported hardware. Frame generation is not available in this alpha." },
    Feature { name: "Screenshots and gallery", category: "Capture", status: "Planned", description: "Configurable capture hotkey, save location, gallery, and optional CoreGui hiding where supported." },
    Feature { name: "Discord presence", category: "Presence", status: "Planned", description: "Optional session presence with privacy controls. Requires a project Discord application ID." },
    Feature { name: "Startup and failure clues", category: "Diagnostics", status: "Available", description: "Missing-launcher guidance, matching read-only Defender history, and bounded log clues for access violations, allocation failures, DNS, TLS, and connection failures. Findings do not establish root causes." },
    Feature { name: "Crash diagnostics", category: "Diagnostics", status: "Implemented · Windows website joins", description: "A native observer records matching new Ferns player processes and final exit codes, even with the settings window closed. Diagnostics shows retained sessions, readable status explanations, possible new-log clues, and a local summary export without raw logs or join links. Existing sessions, missed processes, stale observers, and interrupted monitoring keep an unknown outcome." },
    Feature { name: "Safe launch", category: "Diagnostics", status: "Planned", description: "Automatic recovery after repeated early exits is not implemented. Current crash diagnostics reports evidence and supports manual use of existing restoration controls." },
    Feature { name: "Connection troubleshooting", category: "Diagnostics", status: "Partial", description: "Recognized DNS/TLS and connection-failure log clues are available. Endpoint checks, latency measurements, and authentication diagnosis remain planned." },
    Feature { name: "File verification and repair", category: "Diagnostics", status: "Needs upstream data", description: "Authoritative repair needs build-specific manifests and download sources from the Ferns maintainers." },
    Feature { name: "Linux client compatibility", category: "Platform", status: "Investigating", description: "Run the Windows client in an isolated Wine/Proton environment and validate real joins, rendering, input, and reconnects." },
];
