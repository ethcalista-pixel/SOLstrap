# Alpha validation

This document separates implemented behavior from real-client compatibility claims.

## Automated checks

- Settings roundtrip and atomic replacement.
- Rejection of unknown fields and future schema versions.
- Malformed saved settings are reported rather than silently replaced.
- Launch envelope validation, length limits, and credential-free error messages.
- Acceptance of encoded payloads without decoding or rewriting them.
- Strict parsing of original executable/argument shape on Windows.
- Reversible and idempotent registration using an in-memory registry.
- Rollback if a later handler write fails.
- Restoration preserves externally modified registrations and retains recovery data.
- Rejection of incomplete/duplicate integration backup entries.

## Local validation to record

Verified on the development Windows machine on 18 October 2026:

- `cargo test -p solstrap-core -p solstrap-launcher --locked --quiet`: 11 passed.
- `cargo clippy --workspace --all-targets --locked --quiet -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo build -p solstrap -p solstrap-launcher --release --locked`: passed.
- Native settings executable: 5,012,992 bytes; native launcher: 580,608 bytes. Sizes are specific to this build/toolchain.
- Native window rendering and navigation to settings were inspected. Palette, headings, panel shadows, and missing-launcher messaging were visible.
- One five-second idle sample: approximately 61.9 MiB private memory, 100.5 MiB working set, and 0.0 accumulated CPU seconds. This is a short local sample, not a general benchmark or an estimate of in-game improvement.
- The dependency lockfile contains egui/eframe and contains neither Tauri nor Wry. SOLstrap does not use a browser runtime.
- `--dry-run` selects the original `ferns-launcher` executable and redacts the link payload without starting a client.
- `--dry-run` for `ferns2026` reports that the original player launcher is currently missing. Its registration exists, but its target file disappeared after the initial inspection. Integration remains disabled; Ferns must be repaired before validating that route.

The initial automated checks performed no real protocol changes, Ferns client file changes, or client joins. Native theme persistence and file-dialog flows have not been manually verified. Desktop automation reported a window-bounds error during appearance interaction and later detected user activity; further automated input was stopped to leave the app available for review.

## First live website-join test

After the user reinstalled Ferns, the user enabled integration and reported that the client launched normally from the website. Read-only follow-up inspection confirmed:

- Both original Ferns launcher files are present.
- Both `ferns2026` and `ferns-launcher` handler commands are managed by SOLstrap.
- Integration recovery information is present.
- The running `Player/RobloxPlayerBeta.exe` process was started by `Player/FernsLauncher.exe`.

This establishes one user-confirmed successful website launch with integration enabled. It does not establish repeated joins, rejoining an active session, every launch route, restoration behavior on this machine, or long-session stability. Those remain separate checks.

## CoreGui investigation

The installed player has one loose Lua file, `content/scripts/CoreScripts/PerformanceStatsManagerScript.lua`. Its existing diagnostic code attempts to require Chrome/CorePackages services and emits `[COREDBG]` markers. This is evidence of a possible script integration point, not proof of execution or successful customization. No such markers were found in the recent logs inspected so far, and those inspected player logs predate the latest successful join. The next investigation is to locate current session logging and establish whether this script executes before choosing a reversible CoreGui proof of concept. SOLstrap has not modified this script.

### Log-location discovery and compatibility test

Current player logs were subsequently located at `Ferns/Roblox/logs`, inside the Ferns installation root. The latest available join log also contained no existing `[COREDBG]` markers. This does not establish whether source loading or output logging is responsible.

The native Interface page now exposes a reversible compatibility test. It preserves the original loose script in a local integrity-checked backup and prepends a self-contained task that attempts to create a small CoreGui label. The test does not touch the executable or join payload. Restoration requires the expected patched hash or the unchanged original hash; later script changes are preserved and the backup retained. A new join and visual confirmation remain necessary to establish that the test executes.

The Diagnostics page scans up to four recent player logs, with at most 512 KiB per log. Large logs are sampled at the head and tail without merging boundary fragments. Reports contain only log metadata, counters, and booleans; raw log text and source backups are excluded.

Updated automated checks: 20 tests pass, including original-byte restoration, idempotent test installation, interrupted-install recovery, UTF-8 BOM preservation, backup integrity, external-change conflicts, bounded log sampling, and structured evidence extraction. Strict workspace Clippy and formatting checks pass. Native visual review of this theme revision and real-session CoreGui test results are recorded separately after the build is checked.

The revised native executables built successfully. On the actual installed loose script, installing the test, restoring it, and comparing SHA-256 confirmed exact original-byte restoration. Explicit child-process exit codes were zero for both restore and install. The test was then reinstalled for the user's next join, with its local recovery backup present. No in-game result is claimed yet. Read-only diagnostics located one player log with zero scan errors and no previous SOLstrap test result.

The revised app was opened for review. It uses a warm-charcoal work surface, a burnt-orange header, aligned vector-icon navigation, Segoe UI typography, quieter section headings, and corrected form-control widths. Desktop activity overlapped the observation, so automated checks of every page and visual state remain pending.

## Manual acceptance checks before a community alpha

### Startup and failure diagnostics revision

The CoreGui test was restored to its original bytes during the launcher investigation; no test is currently installed. The original website handlers are restored. After the user's latest reinstall, both original launchers and the player are present. A `ferns2026` dry run selects the player launcher and redacts the payload without executing it.

The native Diagnostics page now reads matching Defender detection/action events through the Windows event API and shows startup findings and explicit session-failure markers. On this machine it successfully read eight historical matching events, reported zero missing-file findings after reinstall, and found no recognized failure markers in the inspected player-log sections. Historical events remain visible without implying that a reinstalled executable is identical or safe. Inspection errors and event limits are represented explicitly.

25 isolated tests pass, including exact detection-path matching, exclusion of unrelated event data, malformed-history handling, and distinguishing a reinstalled launcher from one missing after a recorded quarantine. Workspace Clippy with warnings denied and the release build pass. The current executable sizes are 5,220,352 bytes for settings and 778,240 bytes for forwarding; these are build-specific.

The Client and Diagnostics screens were inspected in the native app. A dark-theme heading contrast defect was corrected, and detection details are now collapsed by default to keep session findings accessible. No Defender settings were changed. A fresh live join, complete appearance coverage, and CoreGui execution confirmation remain pending.

- In a desktop session, confirm navigation, app appearance persistence, search, settings import/export, and readable error states.
- In a controlled installation, enable integration, join through the Ferns website, and verify both original launch routes still work.
- Confirm repeat joins, joining with an existing client session, and launching from a browser while the settings app is closed.
- Restore integration and verify the original registry commands are identical to their backups.
- Test interruption during installation and conflicts caused by Ferns re-registering its launch handlers.
- Confirm settings/report exports and imports through native file dialogs.
- Test portable cleanup after restoration and paths containing spaces/non-ASCII characters.

These checks remain pending until explicitly recorded. CoreGui and experimental features require separate real-client proofs of concept.
