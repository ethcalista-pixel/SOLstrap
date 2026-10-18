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

No real protocol changes, Ferns client file changes, or client joins were performed. Native theme persistence and file-dialog flows have not been manually verified. Desktop automation reported a window-bounds error during appearance interaction and later detected user activity; further automated input was stopped to leave the app available for review.

## Manual acceptance checks before a community alpha

- In a desktop session, confirm navigation, app appearance persistence, search, settings import/export, and readable error states.
- In a controlled installation, enable integration, join through the Ferns website, and verify both original launch routes still work.
- Confirm repeat joins, joining with an existing client session, and launching from a browser while the settings app is closed.
- Restore integration and verify the original registry commands are identical to their backups.
- Test interruption during installation and conflicts caused by Ferns re-registering its launch handlers.
- Confirm settings/report exports and imports through native file dialogs.
- Test portable cleanup after restoration and paths containing spaces/non-ASCII characters.

These checks remain pending until explicitly recorded. CoreGui and experimental features require separate real-client proofs of concept.
