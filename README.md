# SOLstrap

A native Rust desktop companion for the Ferns revival. Configure SOLstrap once, then join from the Ferns website through a small native launcher. The desktop interface uses egui/eframe; there is no Electron, webview, HTML, or JavaScript runtime.

**Development alpha · Windows first · 0.1.0**

## Implemented

- Local Ferns installation discovery and manual installation path.
- Native website-link forwarding, separate from the settings app.
- Optional integration for `ferns2026` and `ferns-launcher`, with original command backups, partial-install recovery, and restoration that preserves third-party changes.
- Versioned global settings, validation, import/export, and atomic file replacement.
- Warm orange, espresso, and cream app design with dark, light, and system appearance and reduced-motion support.
- Read-only installation details, on-demand SHA-256 fingerprint, and local compatibility report export.
- A searchable feature roadmap that identifies unavailable integrations.

**This alpha does not modify Roblox game files.** CoreGui customization, crash monitoring/safe launch, FPS/memory tuning, screenshot capture, Discord presence, repair downloads, RoShade, Linux client support, and DLSS frame generation are not implemented yet. App appearance changes SOLstrap, not the in-game interface.

## Build and run

Requirements: Windows and a stable Rust toolchain with a working native linker. The MSVC toolchain requires Visual Studio C++ build tools; GNU requires its corresponding linker. Node.js and WebView2 are not required by SOLstrap. Ferns is installed separately; this repository contains no Ferns binaries.

```powershell
git clone https://github.com/ethcalista-pixel/SOLstrap.git
cd SOLstrap
cargo build -p solstrap-launcher
cargo run -p solstrap
```

Build portable release executables:

```powershell
cargo build -p solstrap -p solstrap-launcher --release --locked
```

The workspace output is `target/release/solstrap.exe` and `target/release/solstrap-launcher.exe`. Keep both files together in a stable folder. Open `solstrap.exe`, inspect the detected installation, and enable website integration in **Settings** when ready. Do not move the launcher while integration is enabled.

The current release build is portable; a packaged installer, automatic updater, and signing are future work.

## Website integration and recovery

SOLstrap only changes existing per-user Ferns handler commands. It does not change the official Roblox protocols, website authentication, Ferns executables, or startup tasks. Unsupported original command formats are rejected before modification. Links are passed as a single process argument, not interpreted by a shell, and join tickets are excluded from diagnostics.

Original registrations are saved before any change under `%LOCALAPPDATA%\SOLstrap\integration.json`. Do not delete this backup while integration is enabled.

Restore from the app, or use:

```powershell
.\solstrap-launcher.exe --restore
```

Restore before removing the portable application. If Ferns or another app has changed a handler after installation, SOLstrap leaves that handler alone and retains its backup for review.

Read-only CLI operations:

```powershell
.\solstrap-launcher.exe --inspect
.\solstrap-launcher.exe --dry-run "ferns2026:diagnostic-placeholder"
```

The dry run validates the link envelope and selects a launcher but does not start Ferns or verify that Ferns accepts the placeholder. Do not paste real authentication links into issue reports.

## Data and privacy

App preferences and integration recovery information are local files under `%LOCALAPPDATA%\SOLstrap`. There is no telemetry or diagnostic upload endpoint in this alpha. Exports contain local paths and configuration; review them before sharing. A locally calculated hash identifies a file and is not an authenticity guarantee.

## Checks

```powershell
cargo test -p solstrap-core -p solstrap-launcher
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p solstrap -p solstrap-launcher --release --locked
```

Rust tests use isolated temporary directories and an in-memory protocol registry. They do not change the machine's website handlers. A successful compile or test run does not prove a live Ferns join, in-game compatibility, or performance improvements.

See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for architecture, release milestones, and feature gates. See [docs/ALPHA_VALIDATION.md](docs/ALPHA_VALIDATION.md) for validation status and remaining real-client checks.

A Windows CI template is provided at [docs/ci/windows-check.yml](docs/ci/windows-check.yml). It is not active: the GitHub credential available for the initial push lacks workflow scope. A maintainer can enable it by placing it at `.github/workflows/check.yml` using a credential authorized to create workflows.

## Contributing

Keep platform-independent logic in `crates/core`, native forwarding in `crates/launcher`, and the native egui interface in `apps/settings/src`. Test rollback and error behavior when changing configuration or protocol handling. Do not add client binaries, credentials, join links, crash dumps, or generated diagnostics to Git.

A project license has not been selected yet. Third-party dependencies retain their own licenses; inclusion of a roadmap item does not imply redistribution rights for that package.
