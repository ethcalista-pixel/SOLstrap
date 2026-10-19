# Ferns 0.724 CoreGui investigation

19 October 2026. These findings concern this installation and build; they are not a compatibility claim for other Roblox clients.

## Why the loose-script test failed

The test was present before the new player process started and its bytes matched SOLstrap's recovery record. The user saw no label. The current player logs contain neither the SOLstrap start marker nor the existing loose script's COREDBG markers. Absence alone does not prove a particular loader behavior.

There is stronger evidence of the active path: the session log records retrieval of `rbxasset://models/InExperience/InExperience.rbxm`, matching `content/configs/InExperiencePatchConfig/InExperiencePatchConfig.json` (asset version 8610, maximum app version 724). The bundle contains 21,599 ModuleScript instances, including `CoreScripts/PerformanceStatsManagerScript`. Its Source property uses type `0x1d` (compiled bytecode). HASH, SIGU, and SIGN chunks are present. The binary format and bytecode distinction are described by the [rbx-dom format documentation](https://github.com/rojo-rbx/rbx-dom/blob/master/docs/binary.md).

The active bundle and its pre-existing `.orig` file are identical, with SHA-256 `c243f7ada11fa55e0e35127fe6a4004b948face93b85f82d1baee723f09266e3`. SOLstrap has not changed either file. The loose-script test was restored byte-for-byte after this investigation.

The player EXE and DLL pass local Authenticode validation with Roblox Corporation certificates. This identifies those files; it does not characterize changes performed by the separate Ferns launcher at runtime.

## Asset-based font experiment

Font families are separate JSON assets. Roblox's [Font documentation](https://github.com/Roblox/creator-docs/blob/main/content/en-us/reference/engine/datatypes/Font.yaml) maps Builder Sans to `rbxasset://fonts/families/BuilderSans.json`. The installed family refers to bundled font files and some asset IDs.

SOLstrap now offers a narrow experiment: replace only Builder Sans face references with the nearest matching bundled Builder Mono weight/style. The original family name, weights, and styles remain. Source assets must be local, present, regular font files. No download or script execution is involved.

This can affect interface text and any game-authored text using Builder Sans. It cannot isolate CoreGui from game text and does not implement layout, colors, scale, or a replacement menu. A full client restart and visual confirmation of monospaced menu/interface text are required before claiming this build loads the modified definition. Font caching or alternate families may prevent a visible result.

Recovery is stored locally in `interface-font-test.json`. It is integrity-checked, written before modification, and excluded from exported reports. Restoration refuses to overwrite later changes and retains the backup on conflict. A local install/restore round trip matched the original SHA-256; the font experiment was then reinstalled for the next visual test.

## Remaining investigation

The player contains strings naming CoreScript overrides and related internal services. Strings alone do not establish that these interfaces are usable in a release player. No unverified command-line overrides, executable patches, or changes to signature verification were applied.

Full menu and layout customization still needs a demonstrated execution or configuration entry point. The bundled script is the relevant target for further investigation; adding more loose Lua files is not a demonstrated solution.

## Current log source

This installation's recent sessions write under `%LOCALAPPDATA%/Roblox/logs`; the earlier `Ferns/Roblox/logs` file is stale. SOLstrap now chooses the folder with the newest player log between these two locations and reports the selected path. The shared folder can contain sessions from other Roblox installations, so the app labels that ownership limitation. Log contents and authentication tickets are not exported.
