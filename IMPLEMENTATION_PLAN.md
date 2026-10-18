# SOLstrap implementation plan

Status: proposed implementation baseline, 18 October 2026.

## Product and agreed scope

SOLstrap is an open-source companion for the Ferns Roblox revival. Users install it once, configure it in a polished desktop app, then join from the Ferns website directly into the game with their settings applied. The initial platform is Windows; running the Windows Ferns client on Linux is a separate compatibility milestone.

Settings are global. There are no per-game profiles and no account-switching feature. Ferns remains responsible for website authentication and joining games.

Requested features:

- Customization of Roblox CoreGui: topbar, escape menu, chat, player list, backpack/hotbar, loading screen, notifications, fonts, colors, transparency, scale, and full themes.
- Searchable settings, explanations, per-setting reset, configuration import/export, theme previews, and accessibility presets.
- Performance configuration and measurements, memory monitoring, crash diagnostics, connection troubleshooting, and safe launch.
- Actual RoShade integration if compatible with this client and distributable under its terms.
- Discord presence with privacy controls.
- Screenshot shortcuts and gallery, including hiding interface elements where supported.
- Client verification/repair and a compatibility report.
- Linux compatibility investigation and possible DLSS frame generation investigation.

Users choose their configuration and themes. Support for executable community plugins is a separate capability decision; the initial theme format is declarative.

## Evidence from the installed client

Read-only inspection found:

| Item | Observed value |
| --- | --- |
| Installation root | `%LOCALAPPDATA%\Ferns` |
| Main player | `Player\RobloxPlayerBeta.exe` |
| Player file version | `0.724.0.7240739` |
| Desktop shortcut target | `Player\FernsLauncher.exe` |
| Additional client/launcher | `Client\Ferns.exe`, `Client\FernsLauncher.exe` |
| Website protocol | `ferns2026` invokes `Player\FernsLauncher.exe` |
| Additional protocol | `ferns-launcher` invokes `Client\FernsLauncher.exe` |
| Relevant player folders | `ClientSettings`, `content`, `ExtraContent`, `PlatformContent`, `shaders`, `ssl` |
| Other player files | `RobloxPlayerBeta.dll`, `WebView2Loader.dll`, crash handler |
| Initial development baseline | A Windows laptop; retain machine-specific measurements in local reports |

These observations establish file locations and launch registrations, not compatibility with hooks, settings, shaders, Linux, or custom CoreGui. No launch payloads, executable behavior, CoreScript access, or authoritative repair manifests have been validated yet. The user has permission from the lead developer but does not control the native client or have an official startup hook.

## Recommended architecture

Use Rust for both the core and the desktop settings app, with egui/eframe and its Glow renderer. The user rejected the initial Tauri/Svelte design and requested a native, low-level-language implementation with a warm orange and dark reference aesthetic. SOLstrap has no webview or JavaScript runtime. Keep the launch path independent of the interface and measure actual resource use. See `docs/DESIGN.md` for the reference-derived visual direction.

- `solstrap-launcher`: small native executable for protocol handling, configuration application, launching the original Ferns launcher, and starting selected session services. It must not create the settings window for an ordinary website join.
- `solstrap-app`: desktop settings, compatibility status, theme previews, diagnostics, and gallery.
- `solstrap-core`: typed configuration, client discovery, capability detection, backup/rollback, manifests, and feature orchestration.
- `solstrap-session`: session-lifetime services for optional presence, hotkeys, monitoring, and crash classification. Exit after the client closes unless the user enables a tray preference.
- `solstrap-client-adapter`: build-specific settings and asset transformations. Any necessary in-process extension remains a separate, gated module until a valid integration mechanism is demonstrated.

Do not assume DLL injection is necessary or feasible. Start with protocol routing and documented or observed client capabilities. Protocol routing alone cannot provide CoreGui scripting or engine-level rendering changes, and direct executable launches may bypass SOLstrap.

Proposed repository:

```text
apps/settings/             # Native Rust egui/eframe desktop UI
crates/core/               # Configuration, transactions, capabilities
crates/launcher/           # Native protocol entry point
crates/session/            # Optional session services
crates/ferns-adapter/      # Version-specific integration
schemas/                   # Settings, themes, compatibility manifests
themes/                    # Bundled declarative themes
docs/                      # Architecture, compatibility, contributor guides
tests/fixtures/            # Synthetic launch/log/client-layout fixtures
.github/workflows/         # Checks and release packaging
```

Use GitHub for source, issues, CI, and SOLstrap releases after the user creates the repository. Client redistribution and repair downloads need a separate authoritative source from the Ferns team.

## Phase 0 — Establish what the client allows

1. Record player and launcher hashes, architecture, metadata, configuration layout, and known build identity. Store no authentication information.
2. Trace a normal website join through the two Ferns protocols, distinguishing which handles current 2026 games. Inspect the two launchers' roles and update behavior.
3. Determine whether settings, packaged scripts, or assets can change CoreGui. Make one reversible proof of concept, such as a visible CoreGui color or scale adjustment, if an accessible mechanism exists.
4. Determine available graphics APIs, configurable graphics settings, logging, crash reporting, and whether the installed client validates modified files.
5. Reproduce at least one baseline launch and session. Measure startup, memory, average FPS and frame-time percentiles with a repeatable scene and external measurement if necessary.
6. Run an early Linux feasibility spike using a separate Wine/Proton environment on an available Linux machine. Record startup failures, rendering, authentication, joining, and input behavior. Windows inspection alone cannot pass this gate.

Deliverables: compatibility matrix, launch sequence, baseline measurements, and a concrete integration decision. If CoreGui access is unavailable, mark affected controls unavailable and document the exact client capability needed; do not build a working-looking settings panel around a missing backend.

Gate: demonstrate launch forwarding and a supported customization mechanism, or explicitly separate the launch/diagnostics release from blocked CoreGui features. Linux is experimental until tested end to end.

## Phase 1 — Reliable installation, launching, and settings foundation

1. Build client discovery with manual path selection, build fingerprinting, and clear unsupported-build status.
2. Add versioned global settings, validation, migrations, reset, import/export, and backups.
3. Provide an installer that can register SOLstrap for the relevant Ferns protocols after saving their original registrations. Preserve separate handling for each protocol; leave standard Roblox protocols unchanged.
4. Forward the website payload to the original launcher using native process arguments rather than a shell command. Validate protocol and payload shape, avoid forwarding loops, and redact tickets from logs. Do not copy tokens into a settings database.
5. Handle repeat joins, simultaneous invocations, launching while a session is active, missing files, and a Ferns updater that replaces protocol registrations.
6. Apply approved settings through transactional writes with backups; recover interrupted changes. Restore only SOLstrap-owned changes during uninstall, checking for intervening user or Ferns changes before replacing anything.
7. Build the app shell: General, Interface, Graphics, Screenshots, Presence, Diagnostics, Compatibility, and About. Add settings search, descriptions, restart indicators, and capability-aware controls.

Gate: website Play launches the intended game without a SOLstrap window; uninstall restores previous launch handling; failure never leaves settings half-written.

## Phase 2 — CoreGui customization

Implement through the mechanism proven in Phase 0, with a separate capability entry for each component:

| Feature group | Implementation approach | Validation |
| --- | --- | --- |
| Fonts, colors, transparency, scale | Typed theme tokens mapped to supported CoreGui settings/assets/scripts | Readable text and correct scaling across resolutions |
| Topbar and notifications | Component-level theme/layout adapters | Menus, notifications, and input still work |
| Chat and player list | Supported CoreGui component adapters | Chat input, scrolling, teams, and visibility work |
| Backpack/hotbar | Layout, sizing, and appearance adapter | Equip and keyboard/controller interactions work |
| Escape menu and loading screen | Supported templates or targeted replacements | Settings, leaving, loading progress, and error paths remain usable |
| Complete themes | Versioned composition of supported components | Missing components fall back predictably |
| Accessibility presets | Larger text, stronger contrast, reduced motion | Keyboard navigation and text fit remain usable |

Add a desktop preview labeled as an approximation until validated against the client. Initial changes may require restart; only label a control live-applying when it actually is.

Support theme import/export, reset by component, and an original-Ferns appearance preset. Apply changes to Roblox interface components, not experience-authored UI. Detect and report components an experience replaces or disables.

Full replacement themes come after individual components work. Community script plugins, if pursued, require a separate API and an explicit user-visible execution model; theme import must not silently execute code.

Gate: each exposed control changes the real client, survives a join/rejoin, and can be rolled back. All requested components remain on the roadmap, but unsupported components are reported honestly.

## Phase 3 — Performance, crashes, and connection diagnostics

- Create measured graphics presets using only settings validated for this client build. Keep a record of previous values and provide one-click restoration.
- Measure average FPS and frame-time percentiles where a supported data source exists. Report unavailable measurements explicitly; avoid inventing FPS from CPU usage.
- Track client memory and session trends with low-frequency sampling. Identify suspected growth without labeling it a confirmed leak from a single session.
- Distinguish clean exit, requested termination, crash, and unknown exit using process state and available logs. Do not call every nonzero exit a diagnosed crash.
- Record feature/configuration changes around failures. After two suspected early crashes within ten minutes, offer a safe launch that disables SOLstrap changes. Treat that threshold as tunable after real use.
- Present readable diagnostic summaries with evidence, confidence, and suggested next steps. Keep unknown failures unknown.
- Separate join/authentication failures, DNS/TLS/connectivity failures, disconnects, and observed latency. Probe only appropriate Ferns endpoints; avoid collecting raw join URLs or credentials.
- Generate previewable diagnostic bundles with system/build metadata, settings, logs, and optional crash dumps. Keep collection local by default; offer explicit upload once a destination and retention policy exist. Explain that dumps can contain sensitive process memory.
- Add a compatibility report showing available, experimental, unsupported, and failed features with reasons.

An overlay depends on a reliable rendering/telemetry integration. A settings-app performance view can ship first. Client instability, memory leaks, and server problems may require fixes from the Ferns team; SOLstrap must distinguish diagnosis from a demonstrated fix.

Gate: baseline and modified runs can be compared reproducibly; safe launch restores a known configuration; diagnostic exports redact secrets in fixtures and actual sampled logs.

## Phase 4 — Screenshots and Discord presence

- Configurable screenshot shortcut, configurable save location, gallery thumbnails, sorting, preview, deletion, and open-in-folder.
- Capture the game window rather than the whole desktop by default. Verify behavior in windowed, fullscreen, and minimized states.
- Optional capture without CoreGui only if supported; restore interface state even if capture fails. Explain whether game-authored UI remains visible.
- Optional Discord presence, disabled by default, with controls for experience name and session details. Require the project Discord application ID before integration testing.
- Run services only when enabled. Handle Discord absence and disconnects without affecting joining or gameplay.

Gate: hotkeys do not swallow normal input, screenshots have correct framing, and presence respects privacy settings immediately.

## Phase 5 — File verification and repair

- Obtain an authoritative manifest and lawful download locations from Ferns maintainers. GitHub hosting of SOLstrap alone does not supply this.
- Compare immutable files against build-specific hashes. Distinguish original files, intentional SOLstrap transformations, and user-modified files.
- Show a repair preview. Back up affected local modifications before restoring originals, then reapply compatible SOLstrap changes.
- Download to staging, validate sizes/hashes, use atomic replacement, and recover from network interruption. Refuse repair while affected client files are in use.
- Without upstream downloads, offer verification against an explicitly labeled local baseline and restoration from backups; do not represent that as authoritative clean-client repair.

Gate: synthetic corruption, interrupted download, locked files, and unknown builds do not damage the installation.

## Phase 6 — Graphics integrations and Linux support

### RoShade

Identify the exact requested RoShade release and its license/dependencies. Validate renderer compatibility and the actual package on this Ferns build. Prefer installation/import from a user-supplied package until redistribution is established. Make enable/disable/uninstall reversible, detect conflicts, and measure the performance cost. Run an early compatibility experiment once baseline launching works; release only after stability checks.

### Linux

Continue the Phase 0 feasibility spike into reproducible Wine/Proton runtime configuration, isolated prefix management, required dependencies, browser protocol registration, logging, and uninstall. Test authentication, joins, reconnects, graphics, input, window behavior, and screenshots. The SOLstrap app running on Linux does not prove Ferns works there. Publish a tested distro/runtime/GPU matrix; track RoShade-on-Linux separately from basic client support.

### DLSS frame generation

Keep behind a research milestone. Check the exact GPU, driver, rendering API, SDK requirements, required engine buffers, and viable integration path. The development machine is not a validated frame-generation target. Do not advertise DLSS support or silently substitute a different interpolation technique. If no viable integration exists, publish that result and keep the rest of the graphics roadmap independent.

## Phase 7 — Public release

- GitHub CI for formatting, static checks, focused unit/integration tests, and packaging.
- Installer/uninstaller tests in a clean Windows environment, including upgrades and coexistence with official Roblox.
- Signed SOLstrap updater artifacts and verified release metadata; plan Windows executable signing separately. Keep updates out of the immediate website join path.
- Open-source license selected by the user, contribution guide, issue templates, third-party notices, support policy, and compatibility documentation.
- Alpha release to the user's machine, then a small community test group, then public Windows release. Linux remains experimental until its separate acceptance checks pass.

## Release slices and completion criteria

| Release | Included | Completion evidence |
| --- | --- | --- |
| Technical prototype | Discovery, native launch forwarding, capability report, one real CoreGui proof of concept if available | Website join works and integration limits are recorded |
| Windows alpha | Settings app, reversible installation, supported CoreGui controls, settings export/import, local diagnostics, safe launch | Real-client feature checks and uninstall recovery pass |
| Windows beta | Expanded CoreGui, screenshots/gallery, presence, performance reporting, repair where upstream data exists | Small-group testing, measured overhead, recovery checks |
| Public Windows v1 | Stable verified subset, compatibility report, documentation, release/update pipeline | Fresh install, upgrade, rollback, and uninstall verified |
| Experimental tracks | RoShade, Linux, overlay integration, frame-generation research | Each has independent compatibility and stability evidence |

Do not require every experimental feature to ship before the stable launcher becomes useful.

## Performance goals and testing approach

Initial engineering targets, subject to baseline measurement:

- Website launch path must not create the settings window or block on update checks.
- No always-on service when all optional session features are disabled.
- Target less than 1% average CPU use for idle enabled session services and less than 50 MiB private memory for the native helper; measure separately from the settings app and client.
- Target less than 250 ms median SOLstrap-only launch overhead on the development machine after warmup; report first-run overhead separately.
- Use repeated runs of the same scene/settings to compare FPS, frame times, memory, and launch time. Document measurement noise before claiming improvements.
- Focus automated tests on argument forwarding/redaction, settings migrations, capability decisions, rollback, manifest verification, and crash classification. Use real-client/manual checks for CoreGui, rendering, input, and capture.
- Benchmarks and reports must name the client build and enabled features. FPS, memory, or connection improvements are outcomes to demonstrate, not installation promises.

## Inputs needed during implementation

- GitHub repository URL when created; no repository creation or publishing is part of this planning step.
- A normal website join for local tracing, without posting authentication tickets in chat or GitHub.
- Any available lead-developer notes about customization, protected files, launch behavior, and approved integration methods.
- Exact RoShade package/version.
- Authoritative Ferns hashes/downloads for repair and a diagnostic upload destination if desired.
- Discord application ID before presence testing and access to a Linux test environment before claiming Linux compatibility.

These inputs gate their dependent features, not the entire project.

## Reference documentation

- egui/eframe: https://github.com/emilk/egui/tree/main/crates/eframe
- eframe native app lifecycle: https://docs.rs/eframe/0.33.3/eframe/trait.App.html
- Reference-derived styling guidance: https://github.com/anthropics/skills/blob/main/skills/frontend-design/SKILL.md
- RoShade upstream repository: https://github.com/bituq/Roshade
- Wine project: https://www.winehq.org/
- NVIDIA DLSS integration overview: https://developer.nvidia.com/rtx/dlss

The architecture and release sequence above are SOLstrap design recommendations. External framework documentation establishes available framework mechanisms, not Ferns compatibility.
