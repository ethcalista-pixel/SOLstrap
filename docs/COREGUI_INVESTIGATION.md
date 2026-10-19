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

## Local-loader trial result

The user restarted after installing the complete loader-plus-probe trial and again reported no label. Follow-up inspection confirmed the expected trial settings and probe were still installed, with no conflict. The new player process started at 08:15 BST on 19 October 2026. A full scan of that session's 739,369-byte log found zero `SOLSTRAP_PROBE_V1` markers and two references to the InExperience bundle path. No module-source-empty or cannot-compile marker was found. This trial did not demonstrate source execution. These observations do not distinguish an ignored setting from another loader path taking precedence.

The trial setting and probe were subsequently restored through the verified recovery operation. The independent font modification remains installed. A native reference to a loader flag is not a sufficient basis to expose it as a working customization mechanism; another identical restart test is not justified by this result.

## Historical source comparison (2016)

The user supplied [Artifaqt/ROBLOX2016](https://github.com/Artifaqt/ROBLOX2016), whose description identifies it as leaked historical source. Read-only comparison used commit `e0cfac59fea3a5b986843e65b0fda286e439f9fc`; its provenance and correspondence to the installed 2026 client have not been independently established. No code from this repository was copied into SOLstrap.

In that historical implementation, `App/script/CoreScript.cpp` selects loose source files only when `LuaVM::canCompileScripts()` is true. Otherwise, it requests embedded CoreScript bytecode. `App/script/LuaVMClient.cpp` returns false for this capability, while the server implementation returns true. A replacement-directory setting therefore does not, by itself, establish that a release player can compile loose scripts.

This supplies a hypothesis consistent with the failed loose-script trial, not proof of the modern implementation. The installed 2026 player loads a separately verified InExperience bundle, which is a material difference. Further native inspection should distinguish configuration selection, source-compiler availability, and bundle loading instead of treating an override-directory string as a working extension API.

## REA / Ghidra result

Local REA 6.7.0 and Ghidra 12.1.4 passed the Windows provider prerequisite checks using the existing Node.js 24.19.0 and JDK 21 installation. No Codex configuration or client files were changed for this setup. Ghidra's official release archive digest was verified before extraction.

The initial default-analysis query spent several minutes in `Windows x86 PE RTTI Analyzer` and was stopped without findings. A subsequent completed trace disabled that optional analyser and searched `LoadCoreScriptsFromPatchOnly`. It recovered the literal at `0x145cec618`, with no analysed references and no truncated result. This is a result about this static analysis profile, not proof that no native references exist: the earlier read-only runtime trace had already observed them. The protected on-disk executable remains a limitation, and this query did not recover a CoreGui customization hook. No additional join test is justified by this result alone.

Raw REA evidence and local tooling stay outside the public repository under `SOLstrap-analysis`. No historical source, client binaries, or runtime memory was added to SOLstrap.

### Observed loader-code follow-up

A later focused capture used five selected executable-address pages around the observed loader references. The first capture incorrectly excluded mapped pages, retaining only the original encrypted image pages. After correcting that filter, a user-performed restart yielded four readable mapped code pages; the page at RVA `0x2345000` was not observed readable. No inaccessible page was read, and no memory, protection, or privilege was changed. The capture is specific to executable SHA-256 `ac4f4613a30012cc209b3358b815fcac9a3f47dea9860ebb5f74519ed0fe2958`.

An offline, nonfunctional analysis fixture combined those observed pages with a known metadata-string page. Its missing code page was explicitly filled with synthetic breakpoint padding. It was never executed or installed. Ghidra recovered pseudocode for the observed function containing the module-source-empty warning and a caller that gathers paths before invoking it. This gives inspectable loader implementation evidence, but neither identifies a working extension hook nor resolves the inaccessible compiler-entry code. Pages were sampled at different times, external callees remain absent, and inferred types are not original symbols.

Two records referenced by flag-registration code were also inspected read-only. They contain name/function registration metadata; their structure has not established a live Boolean value. They must not be used to claim that the local loader flag was accepted or ignored.

Read-only Luau inspection additionally confirmed that Foundation's production token-override routine accepts a caller-provided table and is guarded by `FoundationTokenOverrides`. Finding this routine does not supply an external configuration input. The observed v6 carrier also has a 24-byte trailing region whose semantics have not been established; no script bytes were edited.

### Metadata-only theme experiment result

A separate candidate swaps only the two UIBlox `DarkTheme`/`LightTheme` module names. Re-parsing confirms that all 21,599 module Source values, object identities, other properties, and HASH/SIGU/SIGN payloads remain unchanged. This tests whether existing trusted themes can be selected through model metadata; it is not arbitrary palette or layout customization, and preserved signature bytes do not imply that verification will accept the modified model.

The private trial helper requires a closed player, verifies original/candidate/recovery hashes, uses atomic replacement, and refuses to overwrite later external changes. Its install/restore roundtrip and conflict handling passed against an isolated temporary copy. An initial live install was refused because the player was still running. After that process exited, the user-approved candidate was installed and both the installed bytes and recovery backup were verified. The candidate SHA-256 is `7e3e387648ff8eef4b1df7ee6bb9f5976bed238019b16d557da1e2616257172d`.

The user joined and supplied a screenshot showing light backgrounds in the Escape/People menu. Some text remained white, producing poor contrast. This demonstrates a visible CoreGui effect from the metadata change in this session; it does not establish consistent palette control, arbitrary colors, layout changes, or compatibility across games.

The modified-bundle sessions also introduced four `Signature verification failed with error: VerifyFinal` messages and one `No signatures in the model could be verified` message per inspected session. The inspected baseline sessions did not contain these messages. Follow-up after the styling probe showed that these earlier palette sessions also logged `RBXCRASH: FatalRuntimeError` immediately after the verification failures, about 16 seconds after process start. The visible color change was therefore a failed integration trial, not a usable partial success. Preserving signature payloads did not preserve successful model verification.

After the user closed Ferns and approved restoration, the original bundle was restored and verified against SHA-256 `c243f7ada11fa55e0e35127fe6a4004b948face93b85f82d1baee723f09266e3`. The separate font modification remains installed. Further candidates are prepared offline.

Following the user's explicit testing preference, further launches, joins, and restarts must be requested from the user rather than triggered by the agent.

### Offline native styling probe

The next candidate adds a `StyleSheet`, one `StyleRule`, and a `StyleLink` beneath the existing `CoreGui/RobloxGui` template. The rule selects `#MenuContainer::UIStroke` and specifies an orange, three-pixel outline. The menu container name appears in the installed SettingsHub module. This tests native data-driven styling without adding or editing scripts; it is not yet a functioning customization hook.

[Roblox's styling documentation](https://create.roblox.com/docs/ui/styling) describes name selectors, phantom UI components, and a StyleLink applying a sheet to a GUI tree. It also documents that direct property assignments override styling and only one sheet applies to a tree. Whether this player's patch merge retains the new objects, whether the link reaches the live ScreenGui, and whether existing links take precedence remain unknown.

The fragment was generated and round-tripped with upstream rbx-dom at commit `dbec31dbe6de9bc7d2b6dd78c35bb67b9236ebda`. Only its new class/property records were grafted into the original bundle. Offline checks verified all original objects and Sources, byte-for-byte preservation of every original encoded chunk except the extended parent array, the added reference/attribute values, and an isolated install/restore/conflict test. The header counts and parent array include the three new objects. Candidate SHA-256: `2fc105a1f9d5c0f103e6362d6da2a088df69a427811d521386eb10d924f3a61a`.

After the user confirmed Ferns was closed and requested the next test, the candidate was installed. The closed-player check, original/candidate hashes, recovery backup hash, and installed-file verification passed. The user reported no outline and crashes approximately ten seconds after joining.

Both recent test logs contain `Multiple StyleLinks under RobloxGui may result in undefined behavior`. This indicates that the new link reached RobloxGui, but conflicts with an existing link; the absence of an outline cannot be treated as proof that native styling is unavailable. Both logs also contain four signature verification failures, the no-verified-signatures message, and immediately afterward `RBXCRASH: FatalRuntimeError`, approximately 16 seconds after process start. The duplicate link is a separate styling issue; the temporal association with signature rejection is stronger evidence for the fatal exit, without independently proving causality.

The probe was restored immediately after the report, with original bundle SHA-256 `c243f7ada11fa55e0e35127fe6a4004b948face93b85f82d1baee723f09266e3` verified. Both modified-bundle experiments are rejected; neither should be reinstalled. No signature enforcement changes were made. Further investigation must find an integration route that preserves successful verification rather than continue modifying this signed bundle.

The user then launched a recovery session. The original hash was reverified; at 76 seconds of process uptime, the player was still running and responding. Its new log contained no signature failures, fatal-runtime-error marker, or duplicate-StyleLink warning. This confirms recovery beyond the approximately 16-second failure window of the modified-bundle sessions, not long-term crash-free operation. The independent font change remained installed during this check.

### Follow-up: stylesheet inputs and unsigned asset boundary

Read-only inspection traced the existing link to `Settings/Components/PortalWithFoundationStylelink`, which uses Foundation's current sheet. Foundation's registry constructs sheets and rules from Lua tables, including caller-provided token overrides; no external local palette input was established. `SettingsUIDelegate.getMenuContainerPositionOverride` is a VR-specific positioning routine, not a desktop layout configuration file. A filename inventory found no separate theme/style/GUI JSON, XML, or INI configuration in the installed player.

The apparent native `ThemeOverrideConfigureCookie` and `ThemeOverrideAddUpdateCookie` strings occur with embedded-webview cookie handling and system light/dark values. They are not evidence of arbitrary in-game RGB controls. `InExperienceContainerRespectUserTheme` instead leads to `NotificationService.SelectedTheme`, through `useSelectedTheme`; its reach over the Escape menu and an external setter have not been established. No blind flag trial was installed.

The menu references 25 local-asset URI strings, including existing button, slider, dropdown, and keyboard-icon PNGs. This is a separate candidate customization boundary from signed Lua/model metadata. The serialized player preferences additionally expose `PreferredTextSize`, `PreferredTransparency`, and `ReducedMotion`; they do not establish arbitrary positions or RGB palettes. No preference values were exported or changed.

A narrowly scoped next probe changes only RGB values of the existing `content/textures/ui/Settings/Help/EscapeIcon.png` to orange. A private Rust helper decoded and re-encoded the inspected 48x48 RGBA8 asset, then verified an exact pixel roundtrip and preservation of every alpha value and image dimensions. Original asset SHA-256: `4bf027a696d8f0b1de1bab224a9be0e83808a447bf8415ee9ebaabacd5370464`; candidate SHA-256: `277697407ad0fe30b6977862ba10207d386fad25c404e20320488102c999fc26`.

Isolated recovery tests passed for install, restore, and refusal to overwrite an external change. Live installation initially refused because the previous player process remained running. After the user confirmed exiting, the same verified Ferns process had no main window; that leftover process was ended. The user-approved icon candidate was then installed with verified candidate and recovery hashes. The signed bundle was reverified as original. Expected test observation: an orange ESC-key icon beside Resume. This is only an asset-color test, not proof of panel-color or layout control, and remains subject to a user-performed launch and stability check.

The user reported no visible change and stable play. The latest session contained no signature-failure, fatal-runtime-error, or duplicate-StyleLink markers; the PNG still matched the installed candidate. This establishes neither asset acceptance nor a working color override. Follow-up found that the newer React `MenuButtons` component uses the Foundation image key `icons/controls/keys/key_esc`, with a text/shapes fallback, rather than the legacy PNG literal in `SettingsHub.createGui`. A source literal alone was insufficient evidence that the modified asset was active.

### Prepared ordinary-settings layout trial

The installed flag module named `FFlagCreateInExperienceMenuReact` actually defines `CreateInExperienceMenuReact6`. Reading only the relevant known settings confirmed that `FFlagCreateInExperienceMenuReact6` and `FFlagMenuButtonsMountWithIEM` are both set to the string `True`. These selectors occur in menu creation, visibility, and mounting routines. The legacy button construction path still references the inspected Escape icon. The newer React component instead supplies its Foundation image key to the button hint.

An offline candidate changes exactly those two existing settings to `False`, preserving all other parsed settings. It does not alter models, scripts, signature checks, or flag enforcement. Whether this frozen player accepts the local overrides remains unproven. If accepted, this should select the older menu/button construction path, providing a measurable layout-preset experiment rather than arbitrary layout editing. The existing icon probe remains installed for observation if that path becomes active.

The exact settings backup and candidate are private local artifacts outside the repository. Isolated install/restore and external-change conflict tests passed. Candidate settings SHA-256: `82207f964641589ff2eb33a93c22cef5cb931dac512327b6b1e58a8319bbe7d6`. After user approval and confirmation of exit, the verified Ferns process still had no window; that leftover process was ended. Installation then passed, with the exact two-key difference, both values set to `False`, recovery backup, and original signed bundle independently verified. The user-performed restart and visual test are pending; no successful layout or color capability is claimed.

The user supplied a screenshot and reported that the menu looked unchanged. Inspection after launch confirmed the exact candidate settings hash and both `False` values still present: the launcher had not overwritten this trial. The session had no inspected signature-failure or fatal-runtime-error markers. This does not distinguish ignored local overrides from other menu paths taking precedence; it does not establish a working layout selector. After the user confirmed exit, the verified windowless Ferns process was ended and both this settings trial and the unused PNG trial were restored. Exact original hashes were verified for settings (`102b0839759010d4321524cb80978990e8c691f391475ec27a5949dc3ab63f12`) and PNG (`4bf027a696d8f0b1de1bab224a9be0e83808a447bf8415ee9ebaabacd5370464`).

Offline inspection traced the generated Foundation ESC icon to atlas keys `img_set_1x_4`, `img_set_2x_14`, and `img_set_3x_12`. Their rectangles are respectively offset/size `(340, 0)/(36, 36)`, `(322, 148)/(72, 72)`, and `(712, 400)/(108, 108)`. Generated fallback maps reference asset IDs rather than the loose legacy PNG. Deprecated and generated maps differ, and fallback IDs alone do not establish the active resource. These findings narrow the next read-only asset-resolution investigation; no atlas or cache file has been changed.

The generated fallback IDs for those keys are respectively `92780936720611`, `111245267460179`, and `126455711000639`; the deprecated fallback map has different IDs. None appears in the bundle's CoreScripts asset map or the inspected session log. A bounded read of 1,500 cache-file prefixes also found no matching ID; this is not an exhaustive cache search or proof of absence.

The local flag-cache backup contains `True` for both menu selector settings, and the session logs show both cache loading/verification and a local-settings load message. The latter is truncated before the relevant keys, so it cannot confirm their effective runtime values. Conflicting cache values do not establish precedence. Roblox's [official local FastFlag allowlist announcement](https://devforum.roblox.com/t/allowlist-for-local-client-configuration-via-fast-flags/3966569) also documents that arbitrary local keys may be ignored; its enforcement in this modded Ferns build remains unverified. No cache, allowlist, or enforcement changes were attempted. Repeating the same menu flag trial is not justified by these observations.

### Resolved packaged atlas path and focused color trial

Further inspection of `FoundationImages/Images` recovered its local image-path format: `rbxasset://%s/SpriteSheets/%s.png`. Its package-path routine prefixes the hierarchy with `LuaPackages/`. The installed primary candidates are therefore under `ExtraContent/LuaPackages/Packages/_Index/FoundationImages/FoundationImages/SpriteSheets/`, not the legacy `content/textures/ui/Settings` directory. All three previously traced atlas filenames exist there. The remote IDs identified earlier are fallbacks, not proof that this installation uses remote images.

Visual inspection of the 1x atlas confirmed the ESC-key glyph inside the recovered rectangle. A private Rust generator prepared three candidates, recoloring only nontransparent RGB pixels inside the corresponding ESC rectangle. Decode/re-encode checks verified unchanged dimensions, every alpha value, and every decoded pixel outside the rectangle. The 2x indexed-color input was expanded to RGBA without changing decoded pixels outside the target. A preview of the generated 1x atlas showed only ESC orange; neighbouring L/R and other atlas graphics remained unchanged.

The three-file installer verifies every target/candidate and synchronizes all original backups before modifying any target. Isolated tests passed install/restore, refusal to overwrite external changes before partial restoration, and rollback after an injected failure on the second file. The live installer then confirmed the player was closed and installed the candidates, verifying the original signed-bundle hash afterward. No flags, scripts, models, caches, or signature enforcement were changed by this trial. Its restore command is local `python atlas_accent_trial.py restore` from the private analysis directory.

The user joined and supplied a screenshot showing the orange ESC glyph beside Resume. This confirms local atlas recoloring affects this CoreGui icon in the tested session. The atlas trial remains installed with recovery copies retained. Panel colors, arbitrary RGB settings, and layout changes remain unproven; the result establishes an asset-color customization boundary only.

### Actual UI properties through saved accessibility preferences

Installed-script inspection found direct consumers for native saved preferences: `SettingsHub.onPreferredTransparencyChanged` updates `MenuContainer.BackgroundTransparency`, UIBlox's style provider reads preferred text size and updates text-size offsets, and PlayerList dispatches transparency and reduced-motion settings. This provides specific data-flow evidence for native accessibility controls rather than an arbitrary color/layout hook. The inspected `CoreGuiConfiguration` reflection classes expose enabled/open states, not palette or layout properties.

SOLstrap's native Interface page now contains text-size, transparency, and reduced-motion controls backed by `GlobalBasicSettings_13.xml`. The older `GlobalBasicSettings_revival_13.xml` lacks those properties and has not been recently updated; the normal profile contains them and was updated during the recent sessions. The file is shared with other Windows Roblox players, which is disclosed in the UI. No live preference value has been changed automatically.

The implementation changes only selected XML preferences and preserves other bytes. It rejects duplicate/incorrectly typed properties, nonfinite/out-of-range values, and running Roblox players. Missing entries are represented explicitly as unset; selecting one creates only that property, and recovery restores its original absence. Recovery stores only the original/applied/pending preference values, not the full user profile. Staged recovery supports interruption before replacement or before final commit; restoration preserves unrelated later changes and refuses conflicting changes to the same preferences. Forty-one core/launcher tests pass, including targeted XML preservation, absent-field insertion/removal, process-guard refusal, conflicts, and interrupted updates. Visual behavior of these new controls remains unverified until the user performs a test.

Formatting, strict workspace Clippy, and the release build passed. The rebuilt native app was reopened and its Interface page visually inspected: text-size selection, transparency slider, reduced-motion checkbox, apply, and restore actions rendered correctly. The live shared profile was read successfully, with Medium text and transparency 1 shown; no preference was applied during this UI inspection. The next proposed manual test is transparency 0 to check the actual menu panel's response.

After the user approved the transparency test and closed Ferns, the profile had been rewritten without any of the three preference entries. The initial editor refused the write. It was updated to preserve explicit absence without guessing native defaults; the additional absent-field recovery test passed, along with strict Clippy and a new release build. The refreshed app showed the missing entries as Player default.

The authorized test was then applied through the native Interface page: only `PreferredTransparency` was added with float value `0`; text size and reduced motion remained absent. Verification found 42 properties versus the prior 41, a recovery record with all three original entries absent and only transparency applied, no pending write, and the original signed-bundle hash unchanged. Inspection of `onPreferredTransparencyChanged` shows multiplication of the theme's menu transparency by this preference before assignment to `MenuContainer.BackgroundTransparency`.

The user joined and supplied a screenshot showing a solid black outer menu panel, reporting that it looked fully solid. This confirms saved-preference control of an actual menu-panel property in the tested session. The first log inspection found no signature-failure, fatal-runtime-error, or duplicate-StyleLink markers, with the player still running and responding. This does not establish arbitrary panel RGB colors, positions, text-size behavior, or reduced-motion behavior. Transparency zero remains applied with its original absence preserved in recovery.

Further layout investigation found the serialized `UserGameSettings.PeoplePageLayout` enum (`Card = 0`, `List = 1`) and its consumers in PeopleReactView. The Settings PeopleWrapper connects that package to the in-experience menu but also gates the newer implementation behind feature selectors. The profile already contains Card zero while the tested screenshot shows a list; this is not a demonstrated layout control for the active path. No new flag or layout trial was installed.

On exit after the transparency test, Ferns serialized Medium text size and full motion into previously absent entries. The first editor treated the entire three-value group as a conflict, despite SOLstrap having changed only transparency. Recovery was corrected to evaluate ownership and conflicts per preference: untouched fields retain external changes, and a newly selected field captures its then-current value as the recovery baseline. Existing owned fields retain their original recovery values. A new test covers player-populated defaults, subsequent text-size application, and restoration that preserves untouched motion and unrelated settings. All 42 tests and strict Clippy pass. No user preference was changed while correcting this behavior.

The new release build passed and the rebuilt Interface page showed no conflict. The user-authorized Largest text-size test was applied through the native controls, retaining transparency zero and full motion. Verification found `PreferredTextSize = 4`, `PreferredTransparency = 0`, and `ReducedMotion = false`; recovery retains Medium (`1`) as the pre-text-test value and absence as the original transparency state, with no pending write. The signed bundle remains original. The player had been closed and its verified windowless remnant ended before application.

The user then joined and confirmed that the text-size change worked. The inspected session log contained no signature-failure, fatal-runtime-error, or duplicate-StyleLink markers. This confirms the tested native text-size setting; it does not establish every component's scaling or row-spacing behavior. Reduced motion has not been tested. Largest text and transparency zero remain saved, with original preference recovery retained.

## Native GUI palette rendering (2026-10-19)

REA's focused `BackgroundColor3` trace (private Evidence ID `ev_d57803ad05f258303b481c9d9c8409112760f9ea39444962be64c4cfbb788311`) found reflected property-name data references, not a recovered native setter or external theme input. The protected on-disk code limits that conclusion. Read-only examination of the historical 2016 GUI setters and renderer helped identify a separate shader-asset boundary; it does not establish compatibility between historical and current formats.

The installed modern D3D11 pack was independently parsed as RBXS version 11: 428 shader names, 17 feature flags, seven capability names, and 7,935 fixed-size variant records. Every record passed name, offset, size, wrapper, and DXBC-size bounds checks. Eight variants of `UIFS`, `UIFSMdrSDBox`, and `UIFSAdvSDBox` were selected. Original shader payloads, world shaders, compiled scripts, and executable bytes remain unchanged. Replacements append locally generated payloads and update only the selected directory records.

The native Rust implementation compiles SOLstrap's own RGB-mapping function with the system Direct3D compiler, redirects the existing final output to a temporary register, and appends the mapping. Existing clipping early returns and alpha outputs are preserved. Normal DXBC container checksums are regenerated and each resulting shader is accepted by Direct3D's software device before installation. No shader binary or historical source is embedded in SOLstrap.

This is a color-class rendering remap, not an object-level CoreGui integration. Dark neutral output colors map between a selected dark background and lighter surface; blue-dominant colors map to a selected accent. White text and other color classes remain outside those mappings. Matching game-authored UI and image pixels can also change, as directly observed in the live trials. Individual component colors and free layout control remain unresolved.

The initial live candidate changed only some backgrounds. Widening the neutral match changed the actual People-menu rows. A separate texture-sampling candidate was abandoned after its session ended with a hang-monitor timeout; causality was not established. The simpler arithmetic remap remained responsive for over six minutes and produced warm brown menu rows. The native app then installed its own generated palette, and a fresh game session visibly showed warm brown rows and an orange Resume button with readable white text. The original InExperience bundle hash and recovery-pack hash were reverified.

Recovery uses a synchronized, atomically saved original pack and a small versioned record, including a pending-write identity. An isolated test using copies of the pinned installed files passed initial apply, another palette, exact original restoration, interrupted-write restoration, rejection of outside file changes, and rejection of a corrupt recovery backup. This test uses explicitly supplied local bytes and is excluded from normal CI; no player files or recovery bytes enter Git. Live repeat-color and restoration results are recorded after completion below.

Live repeat-color verification: the native picker changed the accent from orange `[224,92,23]` to teal `[23,224,184]`. After a full exit and website join, the actual Resume button rendered teal while the selected surfaces remained brown. Screenshots of both native-generated palettes are retained privately. The tested sessions had no inspected shader-error, signature-failure, or fatal-runtime-error markers; this is bounded session evidence, not a general crash-fix claim. The native `--gui-colors-restore` operation then restored the exact original pack SHA-256 `63a233028114abdd3e554c12283b0d94c28d7528fe91f77870ac9fdc7597e71f`, with no pending recovery write.

The final native app reapplied the warm preset after restoration. Its pack hash exactly matches the earlier visually verified native orange candidate (`41f39f4f721e54fbc48334c2b9ac458a71f5bc1b4c9ef0b1b6b779179c458d09`), with accent `[224,92,23]` and no pending write. Ferns is closed; the app is left on Interface, ready for the next ordinary website join.

### Accessibility-editor reset correction

The editor previously replaced its draft on refresh, including focus changes and failed Apply operations. It now merges untouched fields while retaining edited fields, reports same-field external conflicts, and provides explicit Discard. A stale Apply is rejected under the writer lock before changing either the profile or recovery record. Five editor-state tests cover draft retention, unrelated outside changes, conflicts, discard, and successful apply. Native visual QA confirmed that Refresh preserves an unsaved Largest text choice; the temporary choice was discarded without changing the user's saved preferences.

## Current log source

This installation's recent sessions write under `%LOCALAPPDATA%/Roblox/logs`; the earlier `Ferns/Roblox/logs` file is stale. SOLstrap now chooses the folder with the newest player log between these two locations and reports the selected path. The shared folder can contain sessions from other Roblox installations, so the app labels that ownership limitation. Log contents and authentication tickets are not exported.
