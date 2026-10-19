use eframe::egui::{
    self, Color32, FontData, FontFamily, FontId, Margin, RichText, Stroke, TextStyle, Ui, Visuals,
};
use solstrap_core::settings::{Appearance, Settings};
use std::sync::Arc;
pub const CANVAS: Color32 = Color32::from_rgb(12, 14, 15);
pub const SIDEBAR: Color32 = Color32::from_rgb(10, 12, 13);
pub const PANEL: Color32 = Color32::from_rgb(19, 22, 23);
pub const FIELD: Color32 = Color32::from_rgb(13, 16, 17);
pub const ORANGE: Color32 = Color32::from_rgb(255, 166, 71);
pub const CREAM: Color32 = Color32::from_rgb(238, 230, 212);
pub const MUTED: Color32 = Color32::from_rgb(149, 153, 145);
pub const LINE: Color32 = Color32::from_rgb(53, 56, 49);
pub const HEADER: Color32 = Color32::from_rgb(185, 101, 28);
pub fn configure(ctx: &egui::Context, settings: &Settings) {
    let light = match settings.appearance {
        Appearance::Light => true,
        Appearance::Dark => false,
        Appearance::System => ctx.system_theme() == Some(egui::Theme::Light),
    };
    let mut v = if light {
        Visuals::light()
    } else {
        Visuals::dark()
    };
    let text = if light { FIELD } else { CREAM };
    let border = if light {
        Color32::from_rgb(186, 168, 135)
    } else {
        LINE
    };
    v.override_text_color = Some(text);
    v.panel_fill = if light {
        Color32::from_rgba_unmultiplied(247, 236, 211, 243)
    } else {
        Color32::from_rgba_unmultiplied(19, 22, 23, 238)
    };
    v.window_fill = if light {
        Color32::from_rgb(247, 236, 211)
    } else {
        PANEL
    };
    v.faint_bg_color = if light {
        Color32::from_rgb(232, 218, 191)
    } else {
        FIELD
    };
    v.extreme_bg_color = v.faint_bg_color;
    v.text_edit_bg_color = Some(v.faint_bg_color);
    v.selection.bg_fill = if light {
        Color32::from_rgb(220, 187, 128)
    } else {
        Color32::from_rgb(88, 56, 27)
    };
    v.selection.stroke = Stroke::new(1.0, text);
    v.hyperlink_color = if light { HEADER } else { ORANGE };
    v.widgets.noninteractive.bg_fill = v.panel_fill;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(
        1.0,
        if light {
            Color32::from_rgb(90, 83, 65)
        } else {
            MUTED
        },
    );
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        w.corner_radius = 2.into();
        w.fg_stroke = Stroke::new(1.0, text);
        w.bg_stroke = Stroke::new(1.0, border);
    }
    v.widgets.inactive.bg_fill = if light {
        Color32::from_rgb(238, 225, 200)
    } else {
        Color32::from_rgb(28, 31, 29)
    };
    v.widgets.hovered.bg_fill = if light {
        Color32::from_rgb(224, 204, 169)
    } else {
        Color32::from_rgb(48, 39, 27)
    };
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, if light { HEADER } else { ORANGE });
    v.widgets.active.bg_fill = if light {
        Color32::from_rgb(217, 189, 142)
    } else {
        Color32::from_rgb(74, 48, 24)
    };
    v.widgets.active.bg_stroke = Stroke::new(1.0, if light { HEADER } else { ORANGE });
    v.widgets.inactive.weak_bg_fill = v.widgets.inactive.bg_fill;
    v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
    v.widgets.active.weak_bg_fill = v.widgets.active.bg_fill;
    v.window_corner_radius = 3.into();
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(10.0, 10.0);
        s.spacing.button_padding = egui::vec2(12.0, 8.0);
        s.spacing.interact_size.y = 32.0;
        s.animation_time = if settings.reduced_motion { 0.0 } else { 0.12 };
        for (style, size) in [
            (TextStyle::Body, 13.0),
            (TextStyle::Button, 13.0),
            (TextStyle::Small, 11.0),
            (TextStyle::Heading, 25.0),
        ] {
            s.text_styles.insert(style, FontId::monospace(size));
        }
    });
}
pub fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for path in [
        "C:/Windows/Fonts/CascadiaMono.ttf",
        "C:/Windows/Fonts/consola.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "solar-terminal".into(),
                Arc::new(FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, "solar-terminal".into());
            break;
        }
    }
    ctx.set_fonts(fonts);
}
pub fn display_font(size: f32) -> FontId {
    FontId::monospace(size)
}
pub fn panel(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(ui.visuals().panel_fill)
        .stroke(Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(3)
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new("//").color(if ui.visuals().dark_mode {
                    ORANGE
                } else {
                    HEADER
                }));
                ui.label(RichText::new(title).strong().size(14.0));
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(8.0);
            content(ui);
        });
    ui.add_space(14.0);
}
pub fn hint(ui: &mut Ui, text: impl Into<String>) {
    ui.label(
        RichText::new(text)
            .size(12.0)
            .color(if ui.visuals().dark_mode {
                MUTED
            } else {
                Color32::from_rgb(97, 83, 61)
            }),
    );
}
pub fn primary(ui: &mut Ui, text: &str, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).color(FIELD).strong())
            .fill(ORANGE)
            .corner_radius(2),
    )
}
pub fn field_row(ui: &mut Ui, label: &str, value: &str) {
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(142.0, 21.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.set_min_width(142.0);
                ui.label(RichText::new(label).color(if ui.visuals().dark_mode {
                    MUTED
                } else {
                    Color32::from_rgb(97, 83, 61)
                }));
            },
        );
        ui.add(egui::Label::new(value).wrap());
    });
}
pub fn canvas(ctx: &egui::Context) -> Color32 {
    if ctx.style().visuals.dark_mode {
        CANVAS
    } else {
        Color32::from_rgb(239, 229, 208)
    }
}
pub fn navigation(ui: &mut Ui, text: &str, selected: bool, index: usize) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, text)
    });
    let hovered = response.hovered();
    if selected || hovered || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            2,
            if selected {
                Color32::from_rgb(45, 34, 21)
            } else {
                Color32::from_rgb(25, 29, 28)
            },
        );
        ui.painter().rect_stroke(
            rect,
            2,
            Stroke::new(
                1.0,
                if selected || response.has_focus() {
                    Color32::from_rgb(121, 79, 35)
                } else {
                    LINE
                },
            ),
            egui::StrokeKind::Inside,
        );
    }
    let color = if selected {
        ORANGE
    } else if hovered {
        CREAM
    } else {
        MUTED
    };
    ui.painter().text(
        rect.left_center() + egui::vec2(11.0, 0.0),
        egui::Align2::LEFT_CENTER,
        format!("{:02}", index + 1),
        FontId::monospace(11.0),
        color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(39.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text,
        FontId::monospace(13.0),
        if selected { CREAM } else { color },
    );
    if selected {
        ui.painter().text(
            rect.right_center() - egui::vec2(10.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            ">",
            FontId::monospace(13.0),
            ORANGE,
        );
    }
    let keyboard = response.has_focus()
        && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space));
    if response.clicked() {
        response.request_focus();
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
        || keyboard
}
pub fn sun(p: &egui::Painter, center: egui::Pos2, radius: f32) {
    p.circle_stroke(center, radius, Stroke::new(1.0, ORANGE));
    p.circle_filled(center, radius * 0.48, ORANGE);
    for i in 0..12 {
        let angle = i as f32 * std::f32::consts::TAU / 12.0;
        let dir = egui::vec2(angle.cos(), angle.sin());
        p.line_segment(
            [center + dir * (radius + 4.0), center + dir * (radius + 7.0)],
            Stroke::new(1.0, ORANGE),
        );
    }
}
