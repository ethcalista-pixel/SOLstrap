use eframe::egui::{
    self, Color32, FontData, FontFamily, FontId, Margin, RichText, Shadow, Stroke, TextStyle, Ui,
    Visuals,
};
use solstrap_core::settings::{Appearance, Settings};
use std::sync::Arc;

pub const CANVAS: Color32 = Color32::from_rgb(174, 73, 17);
pub const SIDEBAR: Color32 = Color32::from_rgb(54, 22, 9);
pub const PANEL: Color32 = Color32::from_rgb(70, 27, 8);
pub const FIELD: Color32 = Color32::from_rgb(48, 20, 10);
pub const ORANGE: Color32 = Color32::from_rgb(225, 124, 43);
pub const CREAM: Color32 = Color32::from_rgb(255, 222, 165);
pub const MUTED: Color32 = Color32::from_rgb(204, 166, 127);
pub const LINE: Color32 = Color32::from_rgb(111, 53, 20);

pub fn configure(ctx: &egui::Context, settings: &Settings) {
    let light = match settings.appearance {
        Appearance::Light => true,
        Appearance::Dark => false,
        Appearance::System => ctx.system_theme() == Some(egui::Theme::Light),
    };
    let mut visuals = if light {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    visuals.override_text_color = Some(if light { FIELD } else { CREAM });
    visuals.panel_fill = if light {
        Color32::from_rgb(247, 207, 153)
    } else {
        PANEL
    };
    visuals.window_fill = visuals.panel_fill;
    visuals.faint_bg_color = if light {
        Color32::from_rgb(239, 186, 116)
    } else {
        FIELD
    };
    visuals.extreme_bg_color = visuals.faint_bg_color;
    visuals.selection.bg_fill = ORANGE;
    visuals.selection.stroke = Stroke::new(1.0, FIELD);
    visuals.hyperlink_color = if light { PANEL } else { CREAM };
    visuals.widgets.noninteractive.bg_fill = visuals.panel_fill;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, if light { PANEL } else { MUTED });
    visuals.widgets.inactive.bg_fill = if light {
        Color32::from_rgb(234, 169, 95)
    } else {
        Color32::from_rgb(96, 40, 12)
    };
    visuals.widgets.inactive.weak_bg_fill = visuals.widgets.inactive.bg_fill;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(133, 64, 22));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, if light { FIELD } else { CREAM });
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(153, 73, 24);
    visuals.widgets.hovered.weak_bg_fill = visuals.widgets.hovered.bg_fill;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ORANGE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, CREAM);
    visuals.widgets.active.bg_fill = ORANGE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, FIELD);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, CREAM);
    visuals.widgets.inactive.corner_radius = 7.into();
    visuals.widgets.hovered.corner_radius = 7.into();
    visuals.widgets.active.corner_radius = 7.into();
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.button_padding = egui::vec2(15.0, 9.0);
        style.spacing.interact_size.y = 32.0;
        style.animation_time = if settings.reduced_motion { 0.0 } else { 0.10 };
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(12.0));
        style
            .text_styles
            .insert(TextStyle::Heading, display_font(29.0));
    });
}

pub fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
        fonts
            .font_data
            .insert("system-ui".into(), Arc::new(FontData::from_owned(bytes)));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "system-ui".into());
    }
    let mut display = fonts.families[&FontFamily::Proportional].clone();
    if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/impact.ttf") {
        fonts
            .font_data
            .insert("display".into(), Arc::new(FontData::from_owned(bytes)));
        display.insert(0, "display".into());
    }
    fonts
        .families
        .insert(FontFamily::Name("display".into()), display);
    ctx.set_fonts(fonts);
}

pub fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("display".into()))
}

pub fn panel(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    let fill = ui.visuals().panel_fill;
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, Color32::from_rgb(124, 56, 17)))
        .corner_radius(19)
        .inner_margin(Margin::same(21))
        .shadow(Shadow {
            offset: [0, 6],
            blur: 15,
            spread: 0,
            color: Color32::from_black_alpha(65),
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::Frame::new()
                .fill(ORANGE)
                .corner_radius(egui::CornerRadius {
                    nw: 10,
                    ne: 10,
                    sw: 3,
                    se: 10,
                })
                .inner_margin(Margin::symmetric(14, 6))
                .show(ui, |ui| {
                    ui.label(RichText::new(title).color(FIELD).strong().size(14.0));
                });
            ui.add_space(9.0);
            content(ui);
        });
    ui.add_space(13.0);
}

pub fn hint(ui: &mut Ui, text: impl Into<String>) {
    let color = if ui.visuals().dark_mode { MUTED } else { PANEL };
    ui.label(RichText::new(text).size(12.0).color(color));
}

pub fn primary(ui: &mut Ui, text: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).color(FIELD).strong())
            .fill(ORANGE)
            .corner_radius(9),
    )
}

pub fn field_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.add_sized(
            [130.0, 21.0],
            egui::Label::new(RichText::new(label).color(if ui.visuals().dark_mode {
                MUTED
            } else {
                PANEL
            })),
        );
        ui.add(egui::Label::new(value).wrap());
    });
}
