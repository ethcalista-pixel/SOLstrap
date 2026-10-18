# SOLstrap design direction

The user's supplied orange volcano-interface reference establishes the visual direction: orange, espresso, cream type, sculpted corners, and tactile depth. The first interpretation used too much flat orange and oversized condensed text; the user requested a revision. The revised design makes warm charcoal the work surface, concentrates orange into a shaped page header and actions, and gives navigation and controls consistent alignment. Keep the game-specific volcano imagery out of the interface.

## Tokens

| Role | Color |
| --- | --- |
| Warm-charcoal canvas | `#1F1B18` |
| Espresso navigation | `#161311` |
| Brown-charcoal panels | `#2A231E` |
| Inset controls | `#1B1714` |
| Burnt-orange header | `#C7692B` |
| Orange actions | `#EF9A50` |
| Cream text | `#EFE5D8` |

Use Segoe UI Semibold for the wordmark and page title and regular Segoe UI for settings and help. Load installed Windows fonts at runtime and use bundled egui fallbacks elsewhere. Do not redistribute Windows fonts. Keep title sizes modest; the app is a settings utility.

## Structure

```text
espresso sidebar | sculpted orange header: title, help, search, refresh
                 |
icon + Client    | warm-charcoal work surface
icon + Interface | panel: section heading and aligned controls
icon + Graphics  |
icon + Capture   | panel: evidence, dependencies, and recovery
icon + Presence  |
-----------------|
icon + Diagnostics
icon + Feature status
icon + Settings  |
```

Keep content left aligned, labels consistent, and controls close to their explanations. Navigation uses small vector icons, a warm selected surface, and an orange edge marker. Reserve the reference's asymmetric corners for the page header; ordinary panels use quieter seams and shallow static shadows. Avoid orange title stickers on every panel, centered sidebar labels, oversized marketing text, decorative statistics, and background animation. Disabled features explain their dependency instead of presenting fake controls.

The reference supersedes the initial green Tauri design. The settings app and launcher are native Rust; no webview is part of the new build. Use event-driven repainting, no constant animation loop, and no unnecessary UI service during game launch. Measure resource use after building.

Design guidance consulted: [Anthropic Frontend Design skill](https://github.com/anthropics/skills/blob/main/skills/frontend-design/SKILL.md). Applied guidance: choose an explicit identity, give structural decoration a purpose, keep copy useful, and critique the rendered interface. The user's palette and reference take priority over generic styling suggestions.
