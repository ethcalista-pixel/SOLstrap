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
        };
        app.refresh(&cc.egui_ctx);
        app
    }

    fn refresh(&mut self, ctx: &egui::Context) {
        match model::snapshot() {
            Ok(snapshot) => {
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
            .exact_width(177.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(skin::SIDEBAR)
                    .inner_margin(Margin::symmetric(16, 22)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("SOLstrap")
                        .font(skin::display_font(36.0))
                        .color(skin::CREAM),
                );
                ui.label(RichText::new("For Ferns").size(12.0).color(skin::MUTED));
                ui.add_space(27.0);
                for page in Page::ALL {
                    let active = self.page == page;
                    let text = RichText::new(page.nav()).size(14.0).color(if active {
                        skin::FIELD
                    } else {
                        skin::CREAM
                    });
                    let button = egui::Button::new(text)
                        .fill(if active {
                            skin::ORANGE
                        } else {
                            Color32::TRANSPARENT
                        })
                        .stroke(Stroke::NONE)
                        .corner_radius(egui::CornerRadius {
                            nw: 12,
                            ne: 5,
                            sw: 12,
                            se: 12,
                        });
                    if ui.add_sized([ui.available_width(), 38.0], button).clicked() {
                        self.page = page;
                        self.search.clear();
                    }
                    ui.add_space(2.0);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("0.1.0 development alpha")
                            .color(skin::MUTED)
                            .size(11.0),
                    );
                    ui.label(
                        RichText::new("Rust · native desktop")
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
                let width = (ui.available_width() - 155.0).max(150.0);
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
                        "Repair or reinstall Ferns before enabling website integration.",
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

    fn interface(&mut self, ui: &mut egui::Ui) {
        skin::panel(ui, "CoreGui customization", |ui| {
            ui.label("Client integration is under investigation. Game interface changes are not applied in this alpha.");
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
                        egui::Label::new(RichText::new(name).strong()),
                    );
                    skin::hint(ui, scope);
                });
                ui.separator();
            }
            ui.add_space(4.0);
            skin::hint(ui, "Changes will target Roblox CoreGui, not game-authored interfaces. Full replacement themes and accessibility presets are in scope.");
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
                skin::field_row(ui, "Game file changes", "None applied by SOLstrap");
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
                    .fill(skin::CANVAS)
                    .inner_margin(Margin::symmetric(28, 22)),
            )
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(skin::SIDEBAR)
                    .corner_radius(16)
                    .inner_margin(Margin::symmetric(15, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let response = ui.add_sized(
                                [(ui.available_width() - 105.0).max(180.0), 28.0],
                                egui::TextEdit::singleline(&mut self.search)
                                    .hint_text("Search features…")
                                    .frame(false)
                                    .text_color(skin::CREAM),
                            );
                            if response.changed() {
                                self.page = Page::Compatibility;
                            }
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Refresh").color(skin::CREAM))
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
                ui.add_space(19.0);
                ui.label(
                    RichText::new(self.page.title())
                        .font(skin::display_font(31.0))
                        .color(skin::CREAM),
                );
                ui.label(
                    RichText::new(self.page.description())
                        .color(skin::CREAM)
                        .size(13.0),
                );
                ui.add_space(14.0);
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
                            Page::Interface => self.interface(ui),
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
