use anyhow::{ensure, Context, Result};
use serde::Serialize;
use solstrap_core::{
    coregui_loader_trial,
    coregui_probe::{self, ProbeStatus},
    data_dir,
    diagnostics::{self, CoreGuiReport},
    discovery::{self, Installation},
    health::{self, HealthReport},
    integration::{self, IntegrationStatus, NativeRegistry},
    interface_font,
    settings::Settings,
};
use std::path::PathBuf;

#[derive(Serialize)]
pub struct Snapshot {
    pub settings: Settings,
    pub installation: Installation,
    pub integration: IntegrationStatus,
    pub coregui: CoreGuiReport,
    pub health: HealthReport,
    pub probe: Option<ProbeStatus>,
    pub probe_error: Option<String>,
    pub font_test: Option<interface_font::Status>,
    pub font_test_error: Option<String>,
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
    let (probe, probe_error) = match coregui_probe::status(&dir, &installation.root) {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(format!("{error:#}"))),
    };
    let (font_test, font_test_error) = match interface_font::status(&dir, &installation.root) {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(format!("{error:#}"))),
    };
    Ok(Snapshot {
        installation,
        coregui,
        health,
        probe,
        probe_error,
        font_test,
        font_test_error,
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
        coregui_probe::install(&dir, &installation.root)
    } else {
        coregui_loader_trial::restore(&dir, &installation.root)?;
        coregui_probe::restore(&dir, &installation.root)
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
    Feature { name: "Interface font experiment", category: "Interface", status: "Needs visual test", description: "Reversibly maps Builder Sans to bundled Builder Mono font assets. May also affect game-authored text. Restart the client and verify monospaced interface text; layout and color customization remain unproven." },
    Feature { name: "Player-log evidence", category: "Diagnostics", status: "Available", description: "Bounded scanning of recent Ferns player logs with structured script, test, and exception markers. Raw log text is not exported." },
    Feature { name: "Website launch integration", category: "Client", status: "Available", description: "Forwards website joins to the original Ferns launchers. Requires both original launchers and valid registrations." },
    Feature { name: "Preferences and import/export", category: "Settings", status: "Available", description: "Validated, versioned global settings, native file dialogs, and local storage. No per-game profiles." },
    Feature { name: "Client fingerprint", category: "Diagnostics", status: "Available", description: "On-demand SHA-256 identification. Authenticity verification needs an authoritative manifest." },
    Feature { name: "CoreGui themes and layouts", category: "Interface", status: "Investigating", description: "Topbar, escape menu, chat, player list, backpack, loading screen, notifications, fonts, colors, transparency, and scale. The client integration mechanism needs validation." },
    Feature { name: "Theme preview and accessibility", category: "Interface", status: "Planned", description: "Preview, import/export, per-component reset, larger text, contrast, and reduced motion for the game interface." },
    Feature { name: "Graphics presets and telemetry", category: "Graphics", status: "Investigating", description: "Validated graphics settings, average FPS, frame times, and memory measurements. No unverified client flags are applied." },
    Feature { name: "RoShade", category: "Graphics", status: "Investigating", description: "Compatibility, licensing, performance, and reversible installation of the actual requested package." },
    Feature { name: "DLSS frame generation", category: "Graphics", status: "Research", description: "Needs a viable engine integration and supported hardware. Frame generation is not available in this alpha." },
    Feature { name: "Screenshots and gallery", category: "Capture", status: "Planned", description: "Configurable capture hotkey, save location, gallery, and optional CoreGui hiding where supported." },
    Feature { name: "Discord presence", category: "Presence", status: "Planned", description: "Optional session presence with privacy controls. Requires a project Discord application ID." },
    Feature { name: "Startup and failure clues", category: "Diagnostics", status: "Available", description: "Missing-launcher guidance, matching read-only Defender history, and bounded log clues for access violations, allocation failures, DNS, TLS, and connection failures. Findings do not establish root causes." },
    Feature { name: "Crash reports and safe launch", category: "Diagnostics", status: "Planned", description: "Live session monitoring and recovery after repeated early crashes. Current diagnostics inspect existing log evidence." },
    Feature { name: "Connection troubleshooting", category: "Diagnostics", status: "Partial", description: "Recognized DNS/TLS and connection-failure log clues are available. Endpoint checks, latency measurements, and authentication diagnosis remain planned." },
    Feature { name: "File verification and repair", category: "Diagnostics", status: "Needs upstream data", description: "Authoritative repair needs build-specific manifests and download sources from the Ferns maintainers." },
    Feature { name: "Linux client compatibility", category: "Platform", status: "Investigating", description: "Run the Windows client in an isolated Wine/Proton environment and validate real joins, rendering, input, and reconnects." },
];
