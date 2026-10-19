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
- Bounded scanning of the actual Ferns player-log location, with structured CoreGui and exception-marker evidence rather than exported log text.
- Startup findings distinguish absent launchers from historical Defender detections. Native, read-only history inspection matches only this installation's player launcher and update staging file.
- Session clues identify explicit access-violation, allocation-failure, DNS, certificate-validation, and connection-failure log markers without claiming a confirmed root cause.
- An optional, reversible CoreGui compatibility test that backs up the existing loose script and attempts to display a small label on the next join.
- An optional interface-font experiment that maps Builder Sans to bundled Builder Mono faces, with exact-byte restoration and conflict protection. It can also affect game text using that family; live compatibility is not yet confirmed.

Normal launch routing does not modify game files. The optional **CoreGui compatibility test** temporarily prepends a test to one existing loose client script, preserves its original bytes, and offers restoration. Installing the test does not prove that Ferns loads it; confirm its label in a new session. Full CoreGui customization, crash monitoring/safe launch, FPS/memory tuning, screenshot capture, Discord presence, repair downloads, RoShade, Linux client support, and DLSS frame generation are not implemented yet. App appearance changes SOLstrap, not the in-game interface.

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

The workspace output is `target/release/solstrap.exe` and `target/release/solstrap-launcher.exe`. Keep both files together in a stable folder. Open `solstrap.exe`, inspect the detected installation, and enable website integration in **Client** when ready. Do not move the launcher while integration is enabled.

The current release build is portable; a packaged installer, automatic updater, and signing are future work.

## Website integration and recovery

SOLstrap only changes existing per-user Ferns handler commands. It does not change the official Roblox protocols, website authentication, Ferns executables, or startup tasks. Unsupported original command formats are rejected before modification. Links are passed as a single process argument, not interpreted by a shell, and join tickets are excluded from diagnostics.

Original registrations are saved before any change under `%LOCALAPPDATA%\SOLstrap\integration.json`. Do not delete this backup while integration is enabled.

Restore from the app, or use:

```powershell
.\solstrap-launcher.exe --restore
```

Restore before removing the portable application. If Ferns or another app has changed a handler after installation, SOLstrap leaves that handler alone and retains its backup for review.

If you installed the CoreGui test, restore its script from **Interface** before removing SOLstrap. The matching recovery command is `solstrap-launcher.exe --probe-restore`. Script restoration preserves later changes made by Ferns or another tool and retains the backup if there is a conflict. Rejoin to clear a test label already loaded into a session.

The loose-script test did not execute in the tested 0.724 installation. Investigation located the compiled InExperience interface bundle; see [CoreGui findings](docs/COREGUI_INVESTIGATION.md). The separate **Interface font experiment** uses a font-family asset and needs a full restart for visual testing. Restore it through **Interface → Restore original font** or `solstrap-launcher.exe --font-test-restore` before removing SOLstrap. Its recovery file is `%LOCALAPPDATA%/SOLstrap/interface-font-test.json`.

A developer loader trial is available through **Interface → Install loader trial** or `--coregui-loader-test-install`. It changes one normal local loader setting and installs the label probe; the build may ignore the setting or fail to load the interface. This is not a theme feature or a demonstrated hook. Restore both files with `--coregui-loader-test-restore` or **Interface → Restore script / loader trial**. Its settings backup is local and excluded from reports. The page shows the recovery folder and separate loader status, and rechecks state when the app regains focus. See the investigation document before using the trial.

Read-only CLI operations:

```powershell
.\solstrap-launcher.exe --inspect
.\solstrap-launcher.exe --diagnose
.\solstrap-launcher.exe --dry-run "ferns2026:diagnostic-placeholder"
```

The dry run validates the link envelope and selects a launcher but does not start Ferns or verify that Ferns accepts the placeholder. Do not paste real authentication links into issue reports.

`--diagnose` returns `coregui` log evidence and a `health` section containing startup findings, session clues, and matching Defender events. Defender inspection covers up to 128 detection/action events from the last seven days; access errors and inspection limits are reported explicitly. A past detection does not establish the safety or identity of a reinstalled file. SOLstrap does not restore quarantined files or change protection settings.

## Data and privacy

App preferences and integration recovery information are local files under `%LOCALAPPDATA%\SOLstrap`. There is no telemetry or diagnostic upload endpoint in this alpha. Exports contain local paths and configuration; review them before sharing. A locally calculated hash identifies a file and is not an authenticity guarantee.

The CoreGui test backup contains the original local script and is never included in report exports. Log inspection checks at most four Ferns player logs, up to 512 KiB per file, and exports only filenames, timestamps, sizes, marker counts, and diagnostic booleans. Large logs are sampled at the head and tail; absent markers are not proof that a script never ran.

Log discovery compares the Ferns log folder with the shared `%LOCALAPPDATA%/Roblox/logs` folder and selects the one with the newest player log. The shared folder may contain other Roblox installations' sessions; its origin is shown explicitly. Font-test backups are also excluded from reports.

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
