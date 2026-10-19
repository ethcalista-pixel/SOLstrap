# SOLstrap

A native Rust desktop companion for the Ferns revival. Configure SOLstrap once, then join from the Ferns website through a small native launcher. The desktop interface uses egui/eframe; there is no Electron, webview, HTML, or JavaScript runtime.

**Development alpha · Windows first · 0.1.0**

## Implemented

- Local Ferns installation discovery and manual installation path.
- Native website-link forwarding, separate from the settings app.
- Optional integration for `ferns2026` and `ferns-launcher`, with original command backups, partial-install recovery, and restoration that preserves third-party changes.
- Versioned global settings, validation, import/export, and atomic file replacement.
- Solar terminal interface: monospace typography, amber accents, numbered modules, a sun mark, and a dense dot grid over the entire interface with a fluid cursor wake. The low-opacity overlay covers navigation and panels without capturing input. Dark, light, and system appearance remain available. Cursor effects fade to idle, stop when unfocused, and respect reduced motion. Grid density is bounded on large displays.
- Read-only installation details, on-demand SHA-256 fingerprint, and local compatibility report export.
- A searchable feature roadmap that identifies unavailable integrations.
- Bounded scanning of the actual Ferns player-log location, with structured CoreGui and exception-marker evidence rather than exported log text.
- Startup findings distinguish absent launchers from historical Defender detections. Native, read-only history inspection matches only this installation's player launcher and update staging file.
- Session clues identify explicit access-violation, allocation-failure, DNS, certificate-validation, and connection-failure log markers without claiming a confirmed root cause.
- An optional, reversible CoreGui compatibility test that backs up the existing loose script and attempts to display a small label on the next join.
- An optional interface-font experiment that maps Builder Sans to bundled Builder Mono faces, with exact-byte restoration and conflict protection. The user confirmed the font change on Ferns 0.724; it can also affect game text using that family.
- Native controls for saved player text size, transparency, and reduced motion, with recoverable, targeted XML changes. The user confirmed Largest text and that transparency zero makes the Escape-menu panel opaque on the tested build; reduced motion remains unverified. These preferences are shared with other Windows Roblox players.
- Experimental native GUI palette controls for the pinned Ferns D3D11 build, with locally generated shader changes and verified original-file recovery. This is a rendering remap of dark neutral surfaces and blue accents, including matching game UI and image pixels; it does not edit CoreGui objects or layout.

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

**Interface → Player accessibility preferences** edits only `PreferredTextSize`, `PreferredTransparency`, and `ReducedMotion` in `%LOCALAPPDATA%/Roblox/GlobalBasicSettings_13.xml`. Missing entries remain unset unless explicitly selected; **Player default** preserves that absence. All Roblox players must be fully closed first. Join again after applying. Unsaved choices survive Refresh, focus changes, and failed Apply attempts while the window is open; Discard explicitly reloads saved values. **Restore previous preferences** restores the original values or their absence while preserving unrelated settings; external changes to the same preferences are preserved and reported as conflicts. Recovery is local at `%LOCALAPPDATA%/SOLstrap/interface-preferences-recovery.json`. These controls do not supply arbitrary menu colors or positions.

**Interface → GUI palette · Experimental** provides dark-background, lighter-surface, and blue-accent replacement colors. Each role has a native color picker and hex input, solid/linear/radial fills, secondary color, gradient angle/spread, and optional color fade, gradient flow, or rainbow animation with a 0.5–60 second cycle. A pausable preview shows edits before applying; app reduced-motion preferences pause that preview. Incomplete hex edits and other palette drafts survive refresh/focus changes; Apply is disabled for invalid input. Gradients span the viewport rather than each panel. Solid custom colors have been verified in Ferns; gradients and animation pass offline shader compilation, reflection, and Direct3D validation but still require in-game testing, especially whether the client updates its shader clock on this UI path.

The palette supports the exact tested player and original D3D11 shader pack and requires a fully closed player. Restart to apply changes. Matching colors elsewhere in the GUI, including images, can change; white text and other colors are generally retained. This is not a per-element CoreGui theme or a world post-processing filter. Use **Restore original GUI colors**, or `solstrap-launcher.exe --gui-colors-restore`, before removing SOLstrap. Recovery uses `%LOCALAPPDATA%/SOLstrap/interface-colors.json` and `interface-colors-original.pack`. Outside file changes are preserved and reported rather than overwritten. Older solid-color recovery records remain compatible. Do not distribute local shader recovery files.

If the verified warm pack remains installed but its recovery record is missing, **Recover existing SOLstrap palette from original backup** can recreate metadata using the original pack. It verifies the original hash and compares a complete rebuilt palette pack byte-for-byte with the installed file before writing recovery data. It never changes client files or replaces conflicting backups. A private local `palette-recovery-original.pack` beside the settings executable permits the same repair at startup; this seed is a user-owned client backup, not a distributable application asset. Other palettes and unknown packs are not adopted automatically. **Recheck palette files** preserves unsaved edits, and **Palette file details** shows the actual paths, hash, and current verification state.

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
