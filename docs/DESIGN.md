# SOLstrap design direction

The user's supplied orange volcano-interface reference establishes the visual direction: burnt orange canvas, espresso panels, cream type, sculpted corners, overlapping tabs, and tactile shadows. Translate its structure into a usable desktop settings utility. Keep the game-specific volcano imagery out of the interface.

## Tokens

| Role | Color |
| --- | --- |
| Burnt-orange canvas | `#AE4911` |
| Espresso navigation | `#361609` |
| Deep-brown panels | `#461B08` |
| Inset controls | `#30140A` |
| Orange actions | `#E17C2B` |
| Cream text | `#FFDEA5` |

Use a condensed display face for the SOLstrap wordmark and page title, and a readable system sans serif for settings and help. On Windows, read installed Impact and Segoe UI at runtime; use bundled egui fallback fonts elsewhere. Do not redistribute Windows fonts.

## Structure

```text
espresso sidebar | inset search bar                      refresh
                 | page title
Client           | brief explanation
Interface        |
Graphics         | [orange section tab]
Capture          | (deep-brown rounded panel: real controls)
Presence         |
Diagnostics      | [orange section tab]
Feature status   | (deep-brown rounded panel: dependencies)
Settings         |
```

Keep content left aligned, labels consistent, and controls close to their explanations. Use the reference's asymmetric tab corners and soft, static shadows for the identity. Do not add oversized marketing headlines, decorative statistics, repetitive badges, or background animations. Disabled features explain their integration dependency rather than presenting fake controls.

The reference supersedes the initial green Tauri design. The settings app and launcher are native Rust; no webview is part of the new build. Use event-driven repainting, no constant animation loop, and no unnecessary UI service during game launch. Measure resource use after building.

Design guidance consulted: [Anthropic Frontend Design skill](https://github.com/anthropics/skills/blob/main/skills/frontend-design/SKILL.md). Applied guidance: choose an explicit identity, give structural decoration a purpose, keep copy useful, and critique the rendered interface. The user's palette and reference take priority over generic styling suggestions.
