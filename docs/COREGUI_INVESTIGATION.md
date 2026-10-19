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

This can affect interface text and any game-authored text using Builder Sans. It cannot isolate CoreGui from game text and does not implement layout, colors, scale, or a replacement menu. On 19 October, the user confirmed the changed font after restarting and supplied an interface screenshot. This establishes one visually confirmed asset override on this installation; it does not establish the ability to modify scripted layout or colors.

Recovery is stored locally in `interface-font-test.json`. It is integrity-checked, written before modification, and excluded from exported reports. Restoration refuses to overwrite later changes and retains the backup on conflict. A local install/restore round trip matched the original SHA-256; the font experiment was then reinstalled for the next visual test.

## Remaining investigation

The player contains strings naming CoreScript overrides and related internal services. Strings alone do not establish that these interfaces are usable in a release player. No unverified command-line overrides, executable patches, or changes to signature verification were applied.

Full menu and layout customization still needs a demonstrated execution or configuration entry point. The bundled script is the relevant target for further investigation; adding more loose Lua files is not a demonstrated solution.

### Color and layout follow-up

The user's target is the existing menus across all Ferns games. A place-specific replacement is outside this target.

Read-only decoding of instance names and parent references located these actual bundle instances:

- `PatchRoot/DataModelInstances/CoreGui/RobloxGui/Modules/Settings/Theme`
- `PatchRoot/DataModelInstances/CorePackages/Packages/_Index/UIBlox/UIBlox/App/Style/Themes/DarkTheme`
- `PatchRoot/DataModelInstances/CorePackages/Packages/_Index/UIBlox/UIBlox/Core/Style/StyleProvider`
- `PatchRoot/DataModelInstances/CorePackages/Packages/_Index/Foundation/Foundation/Providers/Style/StyleProvider`
- `PatchRoot/DataModelInstances/CorePackages/Packages/_Index/Foundation/Foundation/StyleSheet/updateTokenOverrides`

The Settings Theme module references menu container positioning, dimensions, background/button/selection colors, typography, and scale. Foundation's provider references theme, device, scale, token overrides, and style sheets. These are relevant implementation targets, but finding them does not give an external application access to change their values.

A misleading candidate, `TokenOverridesStore`, is under Foundation's `Utility/Stories` directory: it is a development-story helper, not evidence of a production configuration file. The bundle's eight StringValue entries are an asset map and Luau configuration files, not a menu theme configuration. The installed `AbyssBlue` models are Photobooth scenes containing lighting and geometry, not UI palettes.

The installed user-settings XML exposes text size and preferred transparency, consistent with Roblox's [GuiService accessibility preferences](https://create.roblox.com/docs/reference/engine/classes/GuiService). These preferences do not supply arbitrary menu colors or component positioning.

No usable external color/layout override was found in these paths. No interface bundle, executable, signature check, or runtime memory was modified in this follow-up. The remaining dependency is an actual Ferns-supported route to replace or extend these CoreScripts. The exact maintainer handoff is recorded in [COREGUI_INTEGRATION.md](COREGUI_INTEGRATION.md).

### Read-only startup trace and local-loader trial

A subsequent fresh launch was observed through ordinary Windows process-query/read permissions. The trace inspected readable executable pages only, skipped inaccessible regions, and did not change target memory, page protections, or privileges. It did not inspect heaps or join command lines. The on-disk protected executable could not provide ordinary static code references; readable runtime pages exposed references to `LoadCoreScriptsFromPatchOnly` and the CoreScript module-source error path. This establishes that the loader setting has a real native-code reference in this build, not that it is a functioning customization hook.

Static .NET metadata/IL inspection of FernsLauncher found no script-loader or theme-bridge type. Its command-line builder emits the normal experience-launch command. The player export table did not expose a Lua/CoreGui plugin entry point. These negative results do not rule out internal integration routes.

The next controlled experiment uses the normal `Player/ClientSettings/ClientAppSettings.json` file: set only `FFlagLoadCoreScriptsFromPatchOnly` to the string `False` and reinstall the backed-up loose-script label probe. This local setting may be ignored, or may fail to provide usable source loading. No flag-allowlist bypass, executable patch, signature bypass, or runtime write is used. No layout/color functionality is claimed until actual script execution is demonstrated.

This developer-only trial stores the original settings bytes and hashes in `%LOCALAPPDATA%/SOLstrap/coregui-loader-trial.json` before changing the file. That backup can contain existing client configuration and must remain local; it is not exported or committed. Restore through the updated Interface page's **Restore script / loader trial** action, or `solstrap-launcher.exe --coregui-loader-test-restore`. Restoration preserves later file changes and retains recovery on conflict. Existing font modifications are independent.

## Current log source

This installation's recent sessions write under `%LOCALAPPDATA%/Roblox/logs`; the earlier `Ferns/Roblox/logs` file is stale. SOLstrap now chooses the folder with the newest player log between these two locations and reports the selected path. The shared folder can contain sessions from other Roblox installations, so the app labels that ownership limitation. Log contents and authentication tickets are not exported.
