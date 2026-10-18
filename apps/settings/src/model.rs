use anyhow::{ensure, Context, Result};
use serde::Serialize;
use solstrap_core::{
    data_dir,
    discovery::{self, Installation},
    integration::{self, IntegrationStatus, NativeRegistry},
    settings::Settings,
};
use std::path::PathBuf;

#[derive(Serialize)]
pub struct Snapshot {
    pub settings: Settings,
    pub installation: Installation,
    pub integration: IntegrationStatus,
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
    Ok(Snapshot {
        installation: discovery::inspect(&settings)?,
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

#[derive(Serialize)]
pub struct Feature {
    pub name: &'static str,
    pub category: &'static str,
    pub status: &'static str,
    pub description: &'static str,
}

pub const FEATURES: &[Feature] = &[
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
    Feature { name: "Crash reports and safe launch", category: "Diagnostics", status: "Planned", description: "Session monitoring, evidence-based failure summaries, and recovery after repeated early crashes." },
    Feature { name: "Connection troubleshooting", category: "Diagnostics", status: "Planned", description: "Separate joining, authentication, DNS/TLS, latency, and disconnect failures using validated Ferns endpoints and logs." },
    Feature { name: "File verification and repair", category: "Diagnostics", status: "Needs upstream data", description: "Authoritative repair needs build-specific manifests and download sources from the Ferns maintainers." },
    Feature { name: "Linux client compatibility", category: "Platform", status: "Investigating", description: "Run the Windows client in an isolated Wine/Proton environment and validate real joins, rendering, input, and reconnects." },
];
