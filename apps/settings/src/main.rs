#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod model;
mod skin;

use anyhow::{ensure, Result};
use eframe::egui::{self, Color32, Margin, RichText, Stroke};
use model::{Snapshot, FEATURES};
use solstrap_core::{
    discovery,
    settings::{Appearance, Settings},
    write_json,
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Client,
    Interface,
    Graphics,
    Capture,
    Presence,
    Diagnostics,
    Compatibility,
    Settings,
}

impl Page {
    const ALL: [Self; 8] = [
        Self::Client,
        Self::Interface,
        Self::Graphics,
        Self::Capture,
        Self::Presence,
        Self::Diagnostics,
        Self::Compatibility,
        Self::Settings,
    ];
    fn title(self) -> &'static str {
        match self {
            Self::Client => "Ferns client",
            Self::Interface => "Game interface",
            Self::Graphics => "Graphics",
            Self::Capture => "Screenshots",
            Self::Presence => "Discord presence",
            Self::Diagnostics => "Diagnostics",
            Self::Compatibility => "Feature status",
            Self::Settings => "App settings",
        }
    }
    fn nav(self) -> &'static str {
        match self {
            Self::Client => "Client",
            Self::Interface => "Interface",
            Self::Capture => "Capture",
            Self::Presence => "Presence",
            Self::Settings => "Settings",
            _ => self.title(),
        }
    }
    fn description(self) -> &'static str {
        match self {
            Self::Client => "Installation and website launching.",
            Self::Interface => "Customize the Roblox interface in Ferns.",
            Self::Graphics => "Performance settings and rendering integrations.",
            Self::Capture => "Capture, shortcuts, and your gallery.",
            Self::Presence => "Choose what you share about your session.",
            Self::Diagnostics => "Inspect your installation without changing game files.",
            Self::Compatibility => "Available features, requirements, and work still to do.",
            Self::Settings => "Preferences for the SOLstrap app.",
        }
    }
}

struct App {
    page: Page,
    snapshot: Option<Snapshot>,
    error: Option<String>,
    notice: Option<String>,
    root_input: String,
    search: String,
    fingerprint: Option<String>,
    hash_job: Option<Receiver<Result<String>>>,
    window_focused: bool,
    interface_draft: Option<model::InterfaceDraft>,
    color_draft: Option<solstrap_core::interface_colors::Palette>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        skin::load_fonts(&cc.egui_ctx);
        let mut app = Self {
            page: Page::Client,
            snapshot: None,
            error: None,
            notice: None,
            root_input: String::new(),
            search: String::new(),
            fingerprint: None,
            hash_job: None,
            window_focused: true,
            interface_draft: None,
            color_draft: None,
        };
        app.refresh(&cc.egui_ctx);
        app
    }

    fn refresh(&mut self, ctx: &egui::Context) {
        match model::snapshot() {
            Ok(snapshot) => {
                if self.color_draft.is_none() {
                    self.color_draft = snapshot.interface_colors.as_ref().map(|s| s.palette);
                }
                if let Some(status) = &snapshot.interface_preferences {
                    match &mut self.interface_draft {
                        Some(draft) => draft.refresh(status.current),
                        None => {
                            self.interface_draft = Some(model::InterfaceDraft::new(status.current))
                        }
                    }
                }
                self.root_input = snapshot
                    .settings
                    .client_root
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                skin::configure(ctx, &snapshot.settings);
                self.snapshot = Some(snapshot);
            }
            Err(error) => {
                skin::configure(ctx, &Settings::default());
                self.error = Some(format!("{error:#}"));
            }
        }
    }

    fn operation(
        &mut self,
        ctx: &egui::Context,
        operation: impl FnOnce() -> Result<()>,
        success: &str,
    ) {
        self.error = None;
        self.notice = None;
        match operation() {
            Ok(()) => {
                self.refresh(ctx);
                if self.error.is_none() {
                    self.notice = Some(success.into());
                }
            }
            Err(error) => {
                self.refresh(ctx);
                self.error = Some(format!("{error:#}"));
            }
        }
    }

    fn save(&mut self, ctx: &egui::Context, settings: Settings) {
        self.operation(ctx, || model::save(&settings), "Preferences saved.");
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(204.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(skin::SIDEBAR)
                    .inner_margin(Margin::symmetric(18, 24)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("SOLstrap")
                        .font(skin::display_font(26.0))
                        .color(skin::CREAM),
                );
                ui.label(
                    RichText::new("Ferns companion")
                        .size(12.0)
                        .color(skin::MUTED),
                );
                ui.add_space(30.0);
                for (index, page) in Page::ALL.into_iter().enumerate() {
                    if index == 5 {
                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(8.0);
                    }
                    if skin::navigation(ui, page.nav(), self.page == page, index) {
                        self.page = page;
                        self.search.clear();
                    }
                    ui.add_space(2.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("Version 0.1.0 · Alpha")
                            .color(skin::MUTED)
                            .size(11.0),
                    );
                    ui.add_space(9.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("GitHub repository")
                                    .color(skin::CREAM)
                                    .size(12.0),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(
                            "https://github.com/ethcalista-pixel/SOLstrap",
                        ));
                    }
                });
            });
    }

    fn client(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(snapshot) = &self.snapshot {
            if !snapshot.health.startup.is_empty() {
                skin::panel(ui, "Startup needs attention", |ui| {
                    for finding in &snapshot.health.startup {
                        ui.label(RichText::new(&finding.title).color(skin::ORANGE).strong());
                        ui.label(&finding.evidence);
                        skin::hint(ui, &finding.next_step);
                        ui.add_space(6.0);
                    }
                });
            }
        }
        skin::panel(ui, "Installation", |ui| {
            if let Some(snapshot) = &self.snapshot {
                skin::field_row(
                    ui,
                    "Client",
                    if snapshot.installation.detected {
                        "Ferns player detected"
                    } else {
                        "Player not found"
                    },
                );
                skin::field_row(
                    ui,
                    "Current location",
                    &snapshot.installation.root.display().to_string(),
                );
            }
            ui.add_space(7.0);
            ui.label("Installation folder");
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 180.0).max(120.0);
                ui.add_sized(
                    [width, 33.0],
                    egui::TextEdit::singleline(&mut self.root_input)
                        .hint_text("Automatic detection"),
                );
                if ui.button("Browse…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Select Ferns root containing Player")
                        .pick_folder()
                    {
                        self.root_input = path.display().to_string();
                    }
                }
                if skin::primary(ui, "Save", self.snapshot.is_some()).clicked() {
                    let mut settings = self.snapshot.as_ref().unwrap().settings.clone();
                    settings.client_root = if self.root_input.trim().is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(self.root_input.trim()))
                    };
                    self.fingerprint = None;
                    self.save(ctx, settings);
                }
            });
            skin::hint(ui, "Leave empty to use the standard Ferns installation.");
        });

        skin::panel(ui, "Website launching", |ui| {
            ui.label("Join from the Ferns website. SOLstrap forwards the link to your original launcher.");
            ui.add_space(5.0);
            let mut enable = false;
            let mut restore = false;
            if let Some(snapshot) = &self.snapshot {
                for protocol in &snapshot.integration.protocols {
                    skin::field_row(
                        ui,
                        &protocol.protocol,
                        if protocol.managed {
                            "SOLstrap enabled"
                        } else if protocol.registered {
                            "Original / external handler"
                        } else {
                            "Not registered"
                        },
                    );
                }
                ui.add_space(7.0);
                for launcher in snapshot
                    .installation
                    .launchers
                    .iter()
                    .filter(|launcher| !launcher.exists)
                {
                    ui.label(
                        RichText::new(format!("Missing Ferns launcher: {}", launcher.protocol))
                            .color(skin::ORANGE)
                            .strong(),
                    );
                    skin::hint(ui, launcher.target.display().to_string());
                    skin::hint(
                        ui,
                        "See the startup findings above before enabling website integration.",
                    );
                }
                if !snapshot.launcher_available {
                    skin::hint(ui, "solstrap-launcher must be next to this executable.");
                }
                let ready = snapshot.installation.detected
                    && snapshot.launcher_available
                    && snapshot
                        .installation
                        .launchers
                        .iter()
                        .all(|launcher| launcher.exists)
                    && !snapshot.integration.recovery_available;
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    enable = skin::primary(ui, "Enable integration", ready).clicked();
                    restore = ui
                        .add_enabled(
                            snapshot.integration.recovery_available,
                            egui::Button::new("Restore original handlers"),
                        )
                        .clicked();
                });
            }
            skin::hint(ui, "Original handlers are backed up first. Restore them before moving or removing SOLstrap.");
            if enable {
                self.operation(
                    ctx,
                    || model::set_integration(true),
                    "Website integration enabled.",
                );
            }
            if restore {
                self.operation(
                    ctx,
                    || model::set_integration(false),
                    "Original website handlers restored.",
                );
            }
        });
    }

    fn interface(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let mut apply_colors = None;
        let mut restore_colors = false;
        skin::panel(ui, "GUI palette · Experimental", |ui| {
            skin::hint(ui, "Recolors dark neutral GUI surfaces and blue accents on the tested Ferns D3D11 build. Matching game UI and image colors may also change. This does not change layout or individual CoreGui properties. Fully close the player before applying; join again to load the palette.");
            if let Some(snapshot) = &self.snapshot {
                if let Some(error) = &snapshot.interface_colors_error {
                    skin::hint(ui, error);
                }
                if let Some(status) = &snapshot.interface_colors {
                    if !status.supported {
                        ui.colored_label(skin::ORANGE,"This shader pack is not supported. Restore other shader experiments or repair the client first.");
                    }
                    if status.conflict {
                        ui.colored_label(skin::ORANGE,"Shader files changed outside SOLstrap. The current file and recovery backup are preserved.");
                    }
                    if let Some(palette) = &mut self.color_draft {
                        ui.horizontal(|ui| {
                            ui.color_edit_button_srgb(&mut palette.background);
                            ui.label("Dark background");
                        });
                        ui.horizontal(|ui| {
                            ui.color_edit_button_srgb(&mut palette.surface);
                            ui.label("Lighter surface");
                        });
                        ui.horizontal(|ui| {
                            ui.color_edit_button_srgb(&mut palette.accent);
                            ui.label("Blue-accent replacement");
                        });
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add_enabled(
                                    status.supported
                                        && !status.conflict
                                        && (!status.installed || *palette != status.palette),
                                    egui::Button::new("Apply GUI palette"),
                                )
                                .clicked()
                            {
                                apply_colors = Some(*palette);
                            }
                            if ui
                                .add_enabled(
                                    status.recovery_available
                                        && !status.conflict
                                        && status.installed,
                                    egui::Button::new("Restore original GUI colors"),
                                )
                                .clicked()
                            {
                                restore_colors = true;
                            }
                            if ui.button("Warm orange preset").clicked() {
                                *palette = Default::default();
                            }
                        });
                    }
                    skin::hint(
                        ui,
                        if status.installed {
                            "Palette installed. Restart Ferns after changing colors."
                        } else {
                            "Original shader colors. Applying saves a verified recovery copy first."
                        },
                    );
                }
            }
        });
        if let Some(palette) = apply_colors {
            self.operation(
                ctx,
                || model::apply_interface_colors(palette),
                "GUI palette installed. Join again to load the new colors.",
            );
        }
        if restore_colors {
            self.operation(
                ctx,
                model::restore_interface_colors,
                "Original GUI shaders restored. Join again to reload them.",
            );
        }
        let mut apply_preferences = None;
        let mut restore_preferences = false;
        skin::panel(ui, "Player accessibility preferences", |ui| {
            skin::hint(ui, "Adjust text size, panel transparency, and interface motion where supported. These preferences are shared with other Windows Roblox players. Fully close the player before applying, then join again to reload them.");
            if let Some(snapshot) = &self.snapshot {
                if let Some(error) = &snapshot.interface_preferences_error {
                    skin::hint(ui, error);
                }
                if let Some(status) = &snapshot.interface_preferences {
                    if status.conflict {
                        ui.colored_label(skin::ORANGE, "Preferences changed outside SOLstrap. Existing changes and recovery are preserved.");
                    }
                    if let Some(editor) = &mut self.interface_draft {
                        let draft = &mut editor.values;
                        let labels = ["Medium", "Large", "Larger", "Largest"];
                        ui.horizontal(|ui| {
                            ui.label("Text size");
                            egui::ComboBox::from_id_salt("player-text-size")
                                .selected_text(
                                    draft.text_size.map_or("Player default", |size| {
                                        labels[usize::from(size - 1)]
                                    }),
                                )
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut draft.text_size,
                                        None,
                                        "Player default",
                                    );
                                    for (index, label) in labels.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut draft.text_size,
                                            Some(index as u8 + 1),
                                            *label,
                                        );
                                    }
                                });
                        });
                        let mut set_transparency = draft.transparency.is_some();
                        if ui
                            .checkbox(&mut set_transparency, "Set panel transparency")
                            .changed()
                        {
                            draft.transparency = set_transparency.then_some(1.0);
                        }
                        if let Some(value) = &mut draft.transparency {
                            ui.add(egui::Slider::new(value, 0.0..=1.0).text("Panel transparency"));
                        }
                        ui.horizontal(|ui| {
                            ui.label("Interface motion");
                            egui::ComboBox::from_id_salt("player-motion")
                                .selected_text(match draft.reduced_motion {
                                    None => "Player default",
                                    Some(false) => "Full motion",
                                    Some(true) => "Reduced motion",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut draft.reduced_motion,
                                        None,
                                        "Player default",
                                    );
                                    ui.selectable_value(
                                        &mut draft.reduced_motion,
                                        Some(false),
                                        "Full motion",
                                    );
                                    ui.selectable_value(
                                        &mut draft.reduced_motion,
                                        Some(true),
                                        "Reduced motion",
                                    );
                                });
                        });
                        editor.refresh(status.current);
                        let conflict = status.conflict || editor.conflict(status.current);
                        if editor.conflict(status.current) {
                            ui.colored_label(skin::ORANGE, "A preference you edited also changed in the player settings. Your unsaved choices are kept. Discard them to load the current player values before editing again.");
                        }
                        if editor.dirty() {
                            skin::hint(ui, "Unsaved changes — kept while this window is open. Apply after fully closing Ferns and other Roblox players. A failed apply keeps your choices here.");
                        } else {
                            skin::hint(ui, "Showing saved player preferences. Join again after applying to load changes in the player.");
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !conflict && editor.dirty(),
                                    egui::Button::new("Apply player preferences"),
                                )
                                .clicked()
                            {
                                apply_preferences = Some((editor.values, status.current));
                            }
                            if ui
                                .add_enabled(
                                    editor.dirty(),
                                    egui::Button::new("Discard unsaved changes"),
                                )
                                .clicked()
                            {
                                *editor = model::InterfaceDraft::new(status.current);
                            }
                            if ui
                                .add_enabled(
                                    status.recovery_available && !conflict && !editor.dirty(),
                                    egui::Button::new("Restore previous preferences"),
                                )
                                .clicked()
                            {
                                restore_preferences = true;
                            }
                        });
                    }
                    skin::hint(ui, "Text size and panel transparency were visually confirmed on the tested Ferns build. Reduced motion still needs verification. These controls do not set arbitrary panel colors or positions.");
                }
            }
        });
        if let Some((values, expected)) = apply_preferences {
            self.operation(
                ctx,
                || model::apply_interface_preferences(values, expected),
                "Player preferences saved. Join again to reload them.",
            );
        }
        if restore_preferences {
            self.operation(
                ctx,
                model::restore_interface_preferences,
                "Previous player preferences restored. Join again to reload them.",
            );
        }
        skin::panel(ui, "Interface font experiment", |ui| {
            ui.label("Test a visible font change without executing a client script.");
            skin::hint(ui, "Temporarily maps Builder Sans to the bundled Builder Mono faces. After a full client restart, interface text using Builder Sans should become monospaced. Game text using that same font may also change. This is not a CoreGui layout or color theme.");
            let mut install = false;
            let mut restore = false;
            if let Some(snapshot) = &self.snapshot {
                if let Some(error) = &snapshot.font_test_error {
                    skin::hint(ui, error);
                }
                if let Some(status) = &snapshot.font_test {
                    skin::field_row(
                        ui,
                        "Asset test",
                        if status.installed {
                            "Installed · restart client to test"
                        } else if status.conflict {
                            "External change detected · backup retained"
                        } else {
                            "Original font / test not installed"
                        },
                    );
                    ui.horizontal(|ui| {
                        install = skin::primary(
                            ui,
                            "Install font test",
                            snapshot.installation.detected && !status.recovery_available,
                        )
                        .clicked();
                        restore = ui
                            .add_enabled(
                                status.recovery_available,
                                egui::Button::new("Restore original font"),
                            )
                            .clicked();
                    });
                }
            }
            skin::hint(ui, "The original font definition is backed up first. Restoration preserves later changes. Installing the test does not confirm that the current client uses this asset.");
            if install {
                self.operation(ctx, || model::set_font_test(true), "Font test installed. Fully exit Ferns and join again, then check whether interface text is monospaced.");
            }
            if restore {
                self.operation(
                    ctx,
                    || model::set_font_test(false),
                    "Original font restored. Restart Ferns to reload it.",
                );
            }
        });
        skin::panel(ui, "Test CoreGui access", |ui| {
            if self.snapshot.as_ref().is_some_and(|s| {
                s.installation
                    .root
                    .join("Player/ExtraContent/models/InExperience/InExperience.rbxm")
                    .is_file()
            }) {
                skin::hint(ui, "An InExperience interface bundle is present. The earlier script-only test did not execute on this build. This trial also tests the normal local loader setting; a visible label is still required to confirm execution.");
            }
            ui.label(
                "Test the local CoreScript loader setting together with a visible label probe.",
            );
            skin::hint(ui, "Backs up client settings and the loose script, changes one normal local loader setting, and adds a small ‘SOLstrap - CoreGui test’ label. Restart Ferns to test it. The client may ignore the setting; if joining fails, restore the trial here. No executable or signature checks are changed.");
            let mut install = false;
            let mut restore = false;
            if let Some(snapshot) = &self.snapshot {
                skin::field_row(
                    ui,
                    "Recovery folder",
                    &snapshot.data_directory.display().to_string(),
                );
                if let Some(error) = &snapshot.probe_error {
                    ui.label(RichText::new(error).color(skin::ORANGE));
                }
                if let Some(error) = &snapshot.loader_trial_error {
                    ui.label(RichText::new(error).color(skin::ORANGE));
                }
                let loader_recovery = snapshot
                    .loader_trial
                    .as_ref()
                    .is_some_and(|s| s.recovery_available);
                if let Some(trial) = &snapshot.loader_trial {
                    skin::field_row(
                        ui,
                        "Loader trial",
                        if trial.installed {
                            "Installed · restart Ferns to test"
                        } else if trial.recovery_available {
                            "Recovery available · trial changed or restored externally"
                        } else {
                            "Not installed"
                        },
                    );
                }
                if let Some(probe) = &snapshot.probe {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        install = skin::primary(
                            ui,
                            "Install loader trial",
                            snapshot.coregui.loose_script_present
                                && !probe.recovery_available
                                && !loader_recovery,
                        )
                        .clicked();
                    });
                    if probe.installed {
                        skin::hint(ui, "Test installed for the next join. This does not alter the current game session.");
                    }
                    if probe.conflict {
                        skin::hint(ui, "The script changed after installation. Restoration will preserve that change and retain the backup.");
                    }
                }
                restore = ui
                    .add_enabled(
                        loader_recovery
                            || snapshot
                                .probe
                                .as_ref()
                                .is_some_and(|p| p.recovery_available),
                        egui::Button::new("Restore script / loader trial"),
                    )
                    .clicked();
                let successful = snapshot
                    .coregui
                    .logs
                    .iter()
                    .any(|log| log.probe_write_succeeded);
                let failed = snapshot
                    .coregui
                    .logs
                    .iter()
                    .any(|log| log.probe_write_failed);
                skin::field_row(
                    ui,
                    "Test log evidence",
                    if successful {
                        "A successful write marker was observed"
                    } else if failed {
                        "The script reported a failed CoreGui write"
                    } else {
                        "No CoreGui test result observed yet"
                    },
                );
                skin::hint(ui, "Log markers may come from an earlier test. Confirm the visible label in your new session.");
            }
            if install {
                self.operation(ctx, || model::set_probe(true), "Loader trial and label probe installed. Fully exit Ferns and rejoin to test the label.");
            }
            if restore {
                self.operation(
                    ctx,
                    || model::set_probe(false),
                    "Original script and any loader-trial settings restored. Restart Ferns to clear the test.",
                );
            }
        });
        skin::panel(ui, "Integration evidence", |ui| {
            if let Some(snapshot) = &self.snapshot {
                let report = &snapshot.coregui;
                skin::field_row(
                    ui,
                    "Loose CoreScript",
                    if report.loose_script_present {
                        "Present in the client"
                    } else {
                        "Not found"
                    },
                );
                skin::field_row(
                    ui,
                    "Player logs",
                    &format!("{} recent files inspected", report.logs.len()),
                );
                skin::field_row(
                    ui,
                    "Execution marker",
                    if report.logs.iter().any(|log| log.script_started_marker) {
                        "Observed in a recent log"
                    } else {
                        "Not observed in inspected log sections"
                    },
                );
                skin::field_row(
                    ui,
                    "Chrome module",
                    if report.logs.iter().any(|log| log.chrome_require_succeeded) {
                        "Successful require marker observed"
                    } else {
                        "No successful require marker observed"
                    },
                );
                skin::hint(ui, "Existing diagnostic markers are clues. They do not prove SOLstrap can modify CoreGui. The compatibility test above is optional and changes only the backed-up test script.");
            }
        });
        skin::panel(ui, "CoreGui customization", |ui| {
            ui.label("The temporary test above checks access. Full customization controls are not available yet.");
            ui.add_space(9.0);
            for (name, scope) in [
                ("Topbar", "Buttons, layout, and appearance"),
                ("Chat", "Fonts, colors, size, and position"),
                ("Player list", "Layout, colors, and text"),
                ("Backpack", "Hotbar layout and sizing"),
                ("Escape menu", "Appearance and supported replacements"),
                ("Loading screen", "Layout and branding"),
                ("Notifications", "Appearance and placement"),
                (
                    "Theme system",
                    "Scale, transparency, preview, import/export, reset",
                ),
            ] {
                ui.horizontal_top(|ui| {
                    ui.add_sized(
                        [110.0, 22.0],
                        egui::Label::new(
                            RichText::new(name)
                                .strong()
                                .color(ui.visuals().text_color()),
                        ),
                    );
                    skin::hint(ui, scope);
                });
                ui.separator();
            }
            ui.add_space(4.0);
            skin::hint(ui, "Object-level CoreGui themes and layout controls remain under investigation. The experimental palette above changes shared GUI rendering and can also affect game-authored interfaces.");
        });
    }

    fn planned(&self, ui: &mut egui::Ui, category: &str) {
        for feature in FEATURES
            .iter()
            .filter(|feature| feature.category == category)
        {
            skin::panel(ui, feature.name, |ui| {
                ui.label(feature.description);
                ui.add_space(5.0);
                skin::hint(ui, format!("Status: {}", feature.status));
            });
        }
        skin::hint(ui, "Unavailable features have no effect on your client.");
    }

    fn diagnostics(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        skin::panel(ui, "Startup & protection history", |ui| {
            if let Some(snapshot) = &self.snapshot {
                for finding in &snapshot.health.startup {
                    ui.label(
                        RichText::new(&finding.title)
                            .strong()
                            .color(ui.visuals().text_color()),
                    );
                    ui.label(&finding.evidence);
                    skin::hint(ui, &finding.next_step);
                    ui.add_space(6.0);
                }
                if snapshot.health.startup.is_empty() {
                    ui.label("Required player and launcher files are present.");
                }
                let report = &snapshot.health.defender;
                if let Some(error) = &report.error {
                    skin::hint(ui, error);
                } else if report.events.is_empty() {
                    ui.label("No matching launcher detections found in the inspected history.");
                }
                if !report.events.is_empty() {
                    egui::CollapsingHeader::new(format!(
                        "{} matching historical events",
                        report.events.len()
                    ))
                    .show(ui, |ui| {
                        for event in &report.events {
                            ui.add_space(4.0);
                            ui.label(RichText::new(&event.threat).color(skin::ORANGE));
                            let time = event
                                .time_utc
                                .split('.')
                                .next()
                                .unwrap_or_default()
                                .trim_end_matches('Z')
                                .replace('T', " ");
                            skin::hint(
                                ui,
                                format!("{time} UTC · event {} · {}", event.event_id, event.action),
                            );
                        }
                    });
                }
                if report.limited {
                    skin::hint(ui, "History reached the 128-event inspection limit.");
                }
                skin::hint(ui, "Read-only Defender history from the last seven days, matched to this installation's player launcher. Past detections do not establish the status or safety of a reinstalled file.");
            }
        });
        skin::panel(ui, "Session failure clues", |ui| {
            if let Some(snapshot) = &self.snapshot {
                if snapshot.health.session.is_empty() {
                    ui.label("No recognized failure markers in the inspected log sections.");
                }
                for finding in &snapshot.health.session {
                    ui.label(
                        RichText::new(&finding.title)
                            .strong()
                            .color(ui.visuals().text_color()),
                    );
                    ui.label(&finding.evidence);
                    skin::hint(ui, &finding.next_step);
                    ui.add_space(8.0);
                }
                skin::hint(ui, "Clues cover explicit access violations, allocation failures, DNS, certificate validation, and connection failures. They are evidence for investigation, not confirmed root causes.");
            }
        });
        skin::panel(ui, "Current player logs", |ui| {
            if let Some(snapshot) = &self.snapshot {
                let report = &snapshot.coregui;
                skin::field_row(ui, "Log location", &report.log_folder.display().to_string());
                if report.shared_log_folder {
                    skin::hint(ui, "Shared Roblox log folder: these files may include sessions from other client installations. The newest available folder was selected; session ownership is not verified.");
                }
                if report.logs.is_empty() {
                    ui.label(
                        "No matching player logs are available yet. Join a game, then refresh.",
                    );
                }
                for log in &report.logs {
                    ui.add_space(3.0);
                    ui.label(RichText::new(&log.file_name).size(12.0));
                    skin::hint(
                        ui,
                        format!(
                            "{:.1} KiB inspected · {} CoreScript markers · {} exception markers{}",
                            log.inspected_bytes as f64 / 1024.0,
                            log.core_debug_markers,
                            log.exception_markers,
                            if log.partial {
                                " · head and tail only"
                            } else {
                                ""
                            }
                        ),
                    );
                }
                if report.scan_errors > 0 {
                    skin::hint(
                        ui,
                        format!(
                            "{} log files or folders could not be read.",
                            report.scan_errors
                        ),
                    );
                }
                ui.add_space(5.0);
                skin::hint(ui, "Scans up to four Ferns player logs, bounded to 512 KiB each. Only counts and diagnostic booleans are exported; log text and authentication links are excluded. Exception markers alone do not establish the cause of a crash.");
            }
        });
        skin::panel(ui, "Client details", |ui| {
            if let Some(snapshot) = &self.snapshot {
                skin::field_row(
                    ui,
                    "Player executable",
                    &snapshot.installation.player.display().to_string(),
                );
                skin::field_row(
                    ui,
                    "Player size",
                    &snapshot
                        .installation
                        .player_bytes
                        .map(|bytes| format!("{:.1} MiB", bytes as f64 / 1048576.0))
                        .unwrap_or_else(|| "Unavailable".into()),
                );
                skin::field_row(
                    ui,
                    "Loose Lua scripts",
                    &snapshot.installation.core_script_files.to_string(),
                );
                skin::field_row(
                    ui,
                    "Game file changes",
                    if snapshot
                        .interface_colors
                        .as_ref()
                        .is_some_and(|colors| colors.installed)
                    {
                        "GUI rendering palette installed"
                    } else if snapshot
                        .font_test
                        .as_ref()
                        .is_some_and(|font| font.installed)
                    {
                        "Interface font experiment installed"
                    } else if snapshot.probe.as_ref().is_some_and(|probe| probe.installed) {
                        "CoreGui compatibility test installed"
                    } else {
                        "No active SOLstrap test detected"
                    },
                );
                skin::field_row(ui, "UI runtime", "Native Rust / egui · OpenGL renderer");
                skin::hint(
                    ui,
                    "Loose script files do not establish access to the complete CoreGui.",
                );
            }
        });
        skin::panel(ui, "Player fingerprint", |ui| {
            ui.label("Calculate a SHA-256 identifier for the installed player.");
            ui.add_space(4.0);
            let ready = self.hash_job.is_none()
                && self
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.installation.detected);
            if ui
                .add_enabled(
                    ready,
                    egui::Button::new(if self.hash_job.is_some() {
                        "Calculating…"
                    } else {
                        "Calculate fingerprint"
                    }),
                )
                .clicked()
            {
                let path = self.snapshot.as_ref().unwrap().installation.player.clone();
                let (send, receive) = mpsc::channel();
                let repaint = ctx.clone();
                self.hash_job = Some(receive);
                std::thread::spawn(move || {
                    let _ = send.send(discovery::fingerprint(&path));
                    repaint.request_repaint();
                });
            }
            if let Some(hash) = &self.fingerprint {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(hash).monospace().size(12.0));
                if ui.small_button("Copy fingerprint").clicked() {
                    ctx.copy_text(hash.clone());
                }
            }
            skin::hint(
                ui,
                "A fingerprint identifies a file. Authenticity checks require a Ferns manifest.",
            );
        });
        skin::panel(ui, "Local report", |ui| {
            ui.label("Export installation details, preferences, launch status, feature status, and the fingerprint if calculated.");
            skin::hint(ui, "Includes local paths. No upload, client logs, crash dumps, or login tickets. Review before sharing.");
            ui.add_space(5.0);
            if ui
                .add_enabled(
                    self.snapshot.is_some(),
                    egui::Button::new("Save compatibility report…"),
                )
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Save local compatibility report")
                    .set_file_name("solstrap-compatibility.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    let result = write_json(
                        &path,
                        &serde_json::json!({ "snapshot": self.snapshot, "fingerprint": self.fingerprint, "features": FEATURES, "ui": "native-rust-egui" }),
                    );
                    self.record(result, "Compatibility report saved locally.");
                }
            }
        });
    }

    fn record(&mut self, result: Result<()>, success: &str) {
        self.error = None;
        self.notice = None;
        match result {
            Ok(()) => self.notice = Some(success.into()),
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        skin::panel(ui, "Appearance", |ui| {
            ui.label("Theme for the SOLstrap app. This does not change the game interface.");
            ui.add_space(6.0);
            if let Some(snapshot) = &self.snapshot {
                let mut settings = snapshot.settings.clone();
                let previous = settings.clone();
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.appearance, Appearance::Dark, "Ember");
                    ui.selectable_value(&mut settings.appearance, Appearance::Light, "Daylight");
                    ui.selectable_value(&mut settings.appearance, Appearance::System, "System");
                });
                ui.checkbox(&mut settings.reduced_motion, "Reduce interface motion");
                if previous != settings {
                    self.save(ctx, settings);
                }
            }
        });
        skin::panel(ui, "Global preferences", |ui| {
            ui.label("Import or export SOLstrap settings. There are no per-game profiles.");
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.snapshot.is_some(), egui::Button::new("Export…"))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Export SOLstrap settings")
                        .set_file_name("solstrap-settings.json")
                        .add_filter("JSON", &["json"])
                        .save_file()
                    {
                        self.record(
                            write_json(&path, &self.snapshot.as_ref().unwrap().settings),
                            "Preferences exported.",
                        );
                    }
                }
                if ui.button("Import…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Import SOLstrap settings")
                        .add_filter("JSON", &["json"])
                        .pick_file()
                    {
                        let result = (|| -> Result<()> {
                            ensure!(
                                std::fs::metadata(&path)?.len() <= 65536,
                                "Settings files must be smaller than 64 KiB"
                            );
                            let document = std::fs::read_to_string(path)?;
                            model::save(&Settings::import(&document)?)
                        })();
                        self.operation(ctx, || result, "Preferences imported.");
                    }
                }
            });
            skin::hint(ui, "Exports include your configured client path. They contain no website login tickets.");
        });
        skin::panel(ui, "Runtime", |ui| {
            skin::field_row(ui, "Application", "Rust with egui/eframe");
            skin::field_row(ui, "Web runtime", "None");
            skin::field_row(ui, "Launcher", "Separate native Rust executable");
            skin::hint(ui, "The settings app repaints on interaction. Website joins do not open this window. Close it when you finish configuring SOLstrap.");
        });
    }

    fn compatibility(&self, ui: &mut egui::Ui) {
        let query = self.search.to_lowercase();
        let mut count = 0;
        for feature in FEATURES.iter().filter(|feature| {
            format!(
                "{} {} {} {}",
                feature.name, feature.category, feature.status, feature.description
            )
            .to_lowercase()
            .contains(&query)
        }) {
            count += 1;
            skin::panel(ui, feature.name, |ui| {
                skin::hint(ui, format!("{} / {}", feature.category, feature.status));
                ui.label(feature.description);
            });
        }
        if count == 0 {
            ui.label("No matching features. Try a different search.");
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let focused = ctx.input(|input| input.viewport().focused.unwrap_or(false));
        if focused && !self.window_focused {
            self.refresh(ctx);
        }
        self.window_focused = focused;
        if let Some(job) = &self.hash_job {
            match job.try_recv() {
                Ok(Ok(hash)) => {
                    self.fingerprint = Some(hash);
                    self.hash_job = None;
                }
                Ok(Err(error)) => {
                    self.error = Some(format!("{error:#}"));
                    self.hash_job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error = Some("Fingerprint worker stopped unexpectedly".into());
                    self.hash_job = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        self.sidebar(ctx);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(skin::canvas(ctx))
                    .inner_margin(Margin::symmetric(26, 24)),
            )
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(skin::HEADER)
                    .corner_radius(egui::CornerRadius {
                        nw: 18,
                        ne: 18,
                        sw: 4,
                        se: 18,
                    })
                    .inner_margin(Margin::symmetric(20, 16))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(self.page.title())
                                .font(skin::display_font(27.0))
                                .color(skin::FIELD),
                        );
                        ui.label(
                            RichText::new(self.page.description())
                                .color(skin::FIELD)
                                .size(13.0),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let response = ui.add_sized(
                                [(ui.available_width() - 105.0).max(180.0), 28.0],
                                egui::TextEdit::singleline(&mut self.search)
                                    .hint_text("Search features…")
                                    .frame(true)
                                    .text_color(skin::CREAM),
                            );
                            if response.changed() {
                                self.page = Page::Compatibility;
                            }
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Refresh").color(skin::FIELD))
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::NONE),
                                )
                                .clicked()
                            {
                                self.error = None;
                                self.refresh(ctx);
                            }
                        });
                    });
                ui.add_space(20.0);
                if let Some(error) = &self.error {
                    egui::Frame::new()
                        .fill(skin::FIELD)
                        .corner_radius(9)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.label(RichText::new(error).color(skin::CREAM));
                        });
                    if ui.small_button("Dismiss message").clicked() {
                        self.error = None;
                    }
                    ui.add_space(10.0);
                }
                if let Some(notice) = self.notice.clone() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(notice).color(skin::CREAM));
                        if ui.small_button("Dismiss").clicked() {
                            self.notice = None;
                        }
                    });
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.page {
                            Page::Client => self.client(ui, ctx),
                            Page::Interface => self.interface(ui, ctx),
                            Page::Graphics => self.planned(ui, "Graphics"),
                            Page::Capture => self.planned(ui, "Capture"),
                            Page::Presence => self.planned(ui, "Presence"),
                            Page::Diagnostics => self.diagnostics(ui, ctx),
                            Page::Compatibility => self.compatibility(ui),
                            Page::Settings => self.settings(ui, ctx),
                        }
                        ui.add_space(10.0);
                    });
            });
    }
}

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon.png"))
        .unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 760.0])
            .with_min_inner_size([880.0, 640.0])
            .with_app_id("dev.solstrap.native")
            .with_icon(icon),
        renderer: eframe::Renderer::Glow,
        multisampling: 0,
        ..Default::default()
    };
    eframe::run_native(
        "SOLstrap",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
