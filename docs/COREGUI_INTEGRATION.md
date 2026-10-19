# Integration needed for all-games CoreGui themes

Status: proposed integration contract, not an implemented or available Ferns API.

SOLstrap can modify local font-family assets, and a user has visually confirmed that change on Ferns 0.724. Menu colors and layout are supplied by compiled interface modules in the InExperience bundle. We have not found a supported way for the external SOLstrap process to override those module values.

## Information needed from the Ferns client maintainer

Identify the supported mechanism, if any, for loading a modified InExperience interface bundle or an approved startup CoreScript on this build. Provide that loader's source/integration code or a development build with a documented configuration hook. Permission to develop SOLstrap is already established; the missing item is the technical integration mechanism.

The exact initial targets are `CoreGui/RobloxGui/Modules/Settings/Theme` for the Escape menu, UIBlox's `Core/Style/StyleProvider`, and Foundation's `Providers/Style/StyleProvider`. The available bundle, instance paths, and findings are documented in [COREGUI_INVESTIGATION.md](COREGUI_INVESTIGATION.md). Other components such as chat and the player list need their own layout wiring; changing the Escape menu provider alone is not an all-component solution.

## Responsibilities after a loading mechanism is demonstrated

1. SOLstrap writes a versioned, validated local data profile with color tokens and bounded per-component layout preferences. Profiles contain data only, not executable Lua.
2. The client integration reads that data and makes it available to its trusted interface code before the relevant providers mount. A normal CoreScript cannot be assumed to read arbitrary local files; this host-side handoff must be implemented explicitly.
3. Interface code maps the profile into the active menu/style providers and component layout calculations, preserving normal buttons, input handling, and accessibility behavior.
4. Defaults remain available when SOLstrap is absent, the profile is invalid, or the bridge version is unsupported. SOLstrap reports actual bridge compatibility rather than treating a saved profile as an applied theme.

## First acceptance test

In a fresh session, apply a distinctive Escape-menu background/accent color and a modest width change. Confirm the visible changes and a versioned bridge-start marker. Repeat in another game with no place modifications. Restore defaults and confirm the original layout/colors. Then expand component coverage and expose supported controls in the SOLstrap app.

No color picker or layout slider should claim to apply changes until this client integration has passed that test.
