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

### Interface bundle and font-asset investigation

Developer loader trial: a fresh-launch read-only trace captured native references to `LoadCoreScriptsFromPatchOnly` and the CoreScript module-source path. A controlled normal-configuration trial was then implemented; it does not bypass configuration restrictions or modify executable memory. 35 isolated tests pass, including preservation of other settings, byte-exact restoration, interrupted installation, external-change conflicts, invalid input, and recovery tampering. Strict Clippy passes. On the real installation, trial install/restore matched both the original client-settings SHA-256 and script SHA-256; the trial was subsequently reinstalled for the next join. Script execution, layout control, and color control remain unconfirmed.

GUI recovery follow-up: the user reported an ineffective Install action and disabled Restore. Inspection found that the GUI still installed the earlier script-only test and gated recovery only on the script backup. The updated GUI and CLI now share the complete loader-plus-script operation, with rollback when script installation fails. Recovery is enabled for either valid backup, loader state and its recovery folder are visible, file-query failures are propagated, and status reloads when the window regains focus. The old open app also disagreed with CLI recovery status even after Refresh; after rebuilding/reopening, their states agree. No unverified cause is assigned to that earlier discrepancy.

36 tests and strict workspace Clippy pass. In the actual native UI, **Restore script / loader trial** successfully restored both components while retaining the independent font test. After the final rebuild, clicking **Install loader trial** created both valid recoveries, displayed its completion message, disabled Install, and enabled Restore. Independent CLI inspection agreed. The trial is installed for the next client restart; in-game script execution remains unconfirmed.

Follow-up: the user confirmed a visible font change after restarting and supplied a screenshot. Font-family asset overriding is therefore visually validated once on this installation. The user explicitly requires existing-menu colors and layout across all games. Further inspection located the relevant compiled Settings Theme, UIBlox, and Foundation providers, but did not identify a usable external theme input. [The integration dependency](COREGUI_INTEGRATION.md) is documented; color/layout modification is not implemented or claimed.

The user installed the loose-script test, restarted the client, and reported no visible label. Read-only follow-up confirmed the expected patched bytes were present before process creation. Current session logs were found in the shared Roblox log folder and contain no SOLstrap probe markers. The log identifies InExperience bundle retrieval; metadata inspection found the corresponding compiled PerformanceStatsManagerScript module inside that bundle. See [CoreGui investigation](COREGUI_INVESTIGATION.md).

The loose-script test has been restored. The separate Builder Sans → Builder Mono asset experiment was installed, restored, verified against the original SHA-256, and reinstalled for the next visual check. The current game process was not restarted by SOLstrap. Visual font compatibility remains unconfirmed. Font changes also affect game-authored text using Builder Sans.

31 isolated tests pass. New cases cover font restoration, idempotency, interrupted installation, later-change conflicts, backup tampering, missing assets, invalid source paths, and selection of newer shared player logs with explicit source information. Strict workspace Clippy passes. The new Interface controls compile; interactive visual verification of this revision remains pending because desktop activity overlapped the attempted observation.

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

## 2026-10-19 GUI palette and editor validation

### Native player-exit diagnostics

Website forwarding now attempts to record a session and starts a separate native launcher observer. A failure to initialize monitoring does not prevent the original launcher from running. The observer excludes baseline processes, matches the configured player executable path and process creation time, holds only limited-query/wait handles, records final exit codes, and exits after completion. It does not collect join arguments, raw log text, process memory, or dumps. Waiting times out at 90 seconds; observation stops after 24 hours; missing heartbeats produce an unknown outcome. Up to 32 sessions are retained, evicting completed or stale records before active observers.

Diagnostics displays the newest 12 sessions, durations, readable explanations, next steps, optional newly seen log-marker clues, and a local redacted summary export. Log ownership remains explicitly unverified. The view refreshes the history every three seconds while focused without rerunning the broader installation scan. Safe launch and automatic restoration remain unimplemented.

Controlled Windows tests used a locally compiled, non-faulting fixture that exits with requested statuses. The observer captured 0, 0xC0000005, and 259, matched PID/creation identity, and ignored an identically named executable in another directory. A separate release-launcher `--watch-session` process also captured the expected status/PID with no settings window involved. No Ferns client was launched or terminated for these tests. Unit tests cover classification without overclaiming, PID reuse/baselines, duplicate-observer rejection, bounded retention preserving an active observer, corrupt-history preservation, stale monitoring, export redaction, and exclusion of pre-existing logs. A real website join and ordinary player exit remain the final manual integration check.

Follow-up investigation found the development host's MSIX registry/AppData view differed from the ordinary browser's view. The browser's real handlers still pointed directly to Ferns, explaining the missing session records. After the user enabled integration from an Explorer-launched settings app, both real-user handlers were independently verified to point to SOLstrap and its integration backup was present. A fresh join/exit check is pending. Package identity warnings and an integration-enable guard now expose this distinction; a bounded local launcher receipt records initialization or observer-start failures without storing join arguments.

### Gradient and animation preview build

The native palette editor now includes picker/hex input, two-color linear and radial fills, angle/spread controls, color fade, gradient flow, rainbow, cycle duration, and a pausable preview. These effects span the viewport; they do not know individual CoreGui element bounds. Preview motion honors the app's reduced-motion setting. Older palette records deserialize as solid colors. Palette drafts and incomplete hex edits remain in memory across refresh/focus changes until applied or explicitly discarded.

Offline checks pass: 52 workspace tests, strict clippy, formatting, and optimized builds. An additional opt-in read-only test builds all valid gradient/animation combinations for each palette role independently and all roles together against the private original pack. Every affected variant passes DXBC checksum, Direct3D reflection/global-buffer extent checks, and D3D11 WARP shader creation. The all-effects-off warm pack retains the exact hash of the previously verified live build. No installed Ferns assets were changed for these checks.

The user subsequently reported that their gradient and animation checks work. This is user-reported live confirmation; it does not establish exhaustive coverage of every mode, renderer, or client build. Detailed per-mode persistence and restore checks remain available for further testing.

### Saved themes

Named palette libraries, switching into the draft editor, explicit updates/renaming, and versioned JSON import/export are implemented. Every palette role retains primary/secondary colors, gradient mode/angle/spread, animation mode, and period. Switching or importing never installs shaders. Duplicate imports receive a numbered name, and writes are atomic and locked. Tests verify complete effect round-trips, non-destructive duplicate import, rename, stale-update rejection, invalid documents, and preservation of a corrupt library. Native file-dialog interaction remains a manual check; no Ferns join is required to save or switch editor themes.

### Missing palette recovery record

A report showed the exact previously verified warm pack with recovery unavailable and Apply disabled. Added strict recovery for that specific generated pack: hash-check the original, rebuild the default palette, compare every byte with the installed pack, then publish the original backup and record without replacing client files or conflicting recovery. A private seed beside the local settings executable can perform this repair at startup; it is excluded from Git and must not be distributed. The manual recovery button accepts an original pack when that seed is absent.

The regression test reproduces a missing-record install in isolated temporary directories, confirms enabled status after recovery, and checks rejection of wrong originals, corrupt existing backups, and non-warm packs. A release-app startup test in an isolated profile recreated both recovery files and left the live Ferns pack hash unchanged. The new `target/palette-recovery/release/solstrap.exe` was also opened on the desktop: Interface exposed enabled Apply and Restore buttons. No live palette application or Ferns join was performed for this repair. Snapshot checks now refresh when entering Interface, include a local recheck control, and expose actual paths and hash; launcher `--inspect` includes palette status.

Native Rust GUI palette controls now generate and validate changes to eight GUI pixel-shader variants in the pinned Ferns D3D11 pack. Fresh website joins visually confirmed warm People-menu rows and Resume changing from orange to a picker-selected teal. This is shared GUI rendering: matching game UI and image pixels can also change. Per-component CoreGui properties and layout remain unresolved. The original signed InExperience bundle was reverified unchanged.

The native recovery command restored the exact original shader-pack hash. An isolated test using copies of the installed files passed palette install, palette update, exact restore, interrupted-write restore, outside-change rejection, and corrupt-backup rejection. Recovery backups and player bytes are private local files, excluded from Git. This opt-in installed-file test passed again after atomic backup publication was added.

The accessibility editor now retains dirty fields across Refresh, focus changes, and failed Apply attempts. Same-field outside changes require review or explicit Discard, and stale writes are rejected under the lock. A native visual check confirmed Refresh retaining an unsaved text-size choice; that temporary draft was discarded.

Validation: 45 core tests and five settings-editor tests pass in the normal workspace suite; the additional local installed-file recovery test passes separately. Strict workspace Clippy, formatting, and release builds pass. No client-authentication or signature-enforcement settings were changed.

The fresh browser request created a real-user session record, but the observer did not attach: the current installation runs Player/Ferns.exe rather than Player/RobloxPlayerBeta.exe. The user supplied a client HTTP 529 join-error screenshot; this is not evidence of a process crash. Discovery and process observation now recognize both exact executable paths. The controlled observer test passed for both names, including rejection of the same name outside the configured installation. All 64 regular workspace tests and strict Clippy checks passed. Real-client observation with this correction remains pending.

The user reported a developer update using an Android client through a Windows compatibility host. Local inspection found an x86-64 ELF libroblox.so under Player/root/data/app/com.ferns.client/lib/x86_64, an Android app-assets directory, and a current Ferns/logs/client.log containing Vulkan activity, explicit HttpError: SslConnectFail, and HttpError: Timedout markers. This does not establish which failure caused error 529. Diagnostics now detects the ELF library, selects the host log instead of stale shared Windows-player logs, reports structured TLS/timeout evidence, and explains that Windows host exits do not establish Android guest crashes. A fixture verifies log selection, invalid-library rejection, marker extraction, and raw-text exclusion; all 65 regular workspace tests passed. No client files were changed for this investigation.

Unattended follow-up: the user authorized testing. A normal close and fresh website join again produced the HTTP 529 screen while the real ferns2026 handler pointed directly to FernsLauncher.exe, bypassing SOLstrap. The ferns-launcher handler remained owned by SOLstrap. The exact cause/timing of registration drift is not established. The updated settings app visibly displayed the Android-runtime finding and detected player; read-only release diagnostics selected Ferns/logs/client.log and reported two TLS markers and three connection/timeout markers at inspection time. Subsequent desktop input failed twice with 'foreground window did not report a process id', including a refreshed target retry; UI input stopped. Real-client observer capture and integration restoration remain unverified. The test-opened settings app was cleaned up without changing its settings, and no Ferns files or protection settings were modified. Website integration now explains handler drift, and the palette screen warns that a retained Windows shader pack does not prove compatibility with the Android host.
