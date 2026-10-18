use eframe::egui::{
    self, Color32, FontData, FontFamily, FontId, Margin, RichText, Shadow, Stroke, TextStyle, Ui,
    Visuals,
};
use solstrap_core::settings::{Appearance, Settings};
use std::sync::Arc;

pub const CANVAS: Color32 = Color32::from_rgb(31, 27, 24);
pub const SIDEBAR: Color32 = Color32::from_rgb(22, 19, 17);
pub const PANEL: Color32 = Color32::from_rgb(42, 35, 30);
pub const FIELD: Color32 = Color32::from_rgb(27, 23, 20);
pub const ORANGE: Color32 = Color32::from_rgb(239, 154, 80);
pub const CREAM: Color32 = Color32::from_rgb(239, 229, 216);
pub const MUTED: Color32 = Color32::from_rgb(176, 158, 141);
pub const LINE: Color32 = Color32::from_rgb(70, 56, 45);
pub const HEADER: Color32 = Color32::from_rgb(199, 105, 43);

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
        Color32::from_rgb(239, 215, 181)
    } else {
        Color32::from_rgb(53, 43, 35)
    };
    visuals.widgets.inactive.weak_bg_fill = visuals.widgets.inactive.bg_fill;
    visuals.widgets.inactive.bg_stroke = Stroke::new(
        1.0,
        if light {
            Color32::from_rgb(190, 153, 109)
        } else {
            LINE
        },
    );
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, if light { FIELD } else { CREAM });
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(103, 66, 38);
    visuals.widgets.hovered.weak_bg_fill = visuals.widgets.hovered.bg_fill;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ORANGE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, CREAM);
    visuals.widgets.active.bg_fill = ORANGE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, FIELD);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, CREAM);
    visuals.text_edit_bg_color = Some(if light {
        Color32::from_rgb(252, 234, 211)
    } else {
        FIELD
    });
    visuals.widgets.inactive.corner_radius = 7.into();
    visuals.widgets.hovered.corner_radius = 7.into();
    visuals.widgets.active.corner_radius = 7.into();
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 9.0);
        style.spacing.button_padding = egui::vec2(13.0, 8.0);
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
            .insert(TextStyle::Heading, display_font(27.0));
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
    if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/seguisb.ttf") {
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
        .stroke(Stroke::new(
            1.0,
            if ui.visuals().dark_mode {
                LINE
            } else {
                Color32::from_rgb(205, 168, 121)
            },
        ))
        .corner_radius(13)
        .inner_margin(Margin::same(19))
        .shadow(Shadow {
            offset: [0, 3],
            blur: 8,
            spread: 0,
            color: Color32::from_black_alpha(30),
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let (marker, _) =
                    ui.allocate_exact_size(egui::vec2(3.0, 15.0), egui::Sense::hover());
                ui.painter().rect_filled(
                    marker,
                    1,
                    if ui.visuals().dark_mode {
                        ORANGE
                    } else {
                        HEADER
                    },
                );
                ui.label(
                    RichText::new(title)
                        .strong()
                        .size(15.0)
                        .color(if ui.visuals().dark_mode { CREAM } else { FIELD }),
                );
            });
            ui.add_space(8.0);
            content(ui);
        });
    ui.add_space(11.0);
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
        ui.allocate_ui_with_layout(
            egui::vec2(132.0, 21.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.set_min_width(132.0);
                ui.label(RichText::new(label).color(if ui.visuals().dark_mode {
                    MUTED
                } else {
                    PANEL
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
        Color32::from_rgb(241, 225, 205)
    }
}

pub fn navigation(ui: &mut Ui, text: &str, selected: bool, icon: usize) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 39.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, text)
    });
    let hovered = response.hovered();
    if selected || hovered || response.has_focus() {
        ui.painter().rect_filled(
            rect,
            9,
            if selected {
                Color32::from_rgb(60, 42, 28)
            } else {
                Color32::from_rgb(37, 30, 25)
            },
        );
    }
    if selected {
        let line = egui::Rect::from_min_size(
            rect.left_top() + egui::vec2(0.0, 10.0),
            egui::vec2(3.0, 19.0),
        );
        ui.painter().rect_filled(line, 2, ORANGE);
    }
    let color = if selected {
        ORANGE
    } else if hovered {
        CREAM
    } else {
        MUTED
    };
    draw_icon(
        ui.painter(),
        rect.left_center() + egui::vec2(23.0, 0.0),
        icon,
        color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(44.0, 0.0),
        egui::Align2::LEFT_CENTER,
        text,
        FontId::proportional(14.0),
        if selected { CREAM } else { color },
    );
    let keyboard = response.has_focus()
        && ui.input(|input| {
            input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Space)
        });
    if response.clicked() {
        response.request_focus();
    }
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
        || keyboard
}

fn draw_icon(painter: &egui::Painter, center: egui::Pos2, icon: usize, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let rect = egui::Rect::from_center_size(center, egui::vec2(16.0, 14.0));
    match icon {
        0 => {
            painter.rect_stroke(rect, 2, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    center + egui::vec2(0.0, 7.0),
                    center + egui::vec2(0.0, 10.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-5.0, 10.0),
                    center + egui::vec2(5.0, 10.0),
                ],
                stroke,
            );
        }
        1 => {
            painter.rect_stroke(rect, 2, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    center + egui::vec2(-7.0, -2.0),
                    center + egui::vec2(7.0, -2.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-2.0, -1.0),
                    center + egui::vec2(-2.0, 6.0),
                ],
                stroke,
            );
        }
        2 => {
            painter.line_segment(
                [
                    center + egui::vec2(-8.0, 6.0),
                    center + egui::vec2(-3.0, -6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-3.0, -6.0),
                    center + egui::vec2(7.0, 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-8.0, 6.0),
                    center + egui::vec2(7.0, 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-3.0, -6.0),
                    center + egui::vec2(1.0, 6.0),
                ],
                stroke,
            );
        }
        3 => {
            painter.rect_stroke(rect, 3, stroke, egui::StrokeKind::Inside);
            painter.circle_stroke(center, 4.0, stroke);
        }
        4 => {
            painter.circle_stroke(center + egui::vec2(0.0, -4.0), 4.0, stroke);
            painter.add(egui::Shape::line(
                vec![
                    center + egui::vec2(-7.0, 8.0),
                    center + egui::vec2(-5.0, 3.0),
                    center + egui::vec2(5.0, 3.0),
                    center + egui::vec2(7.0, 8.0),
                ],
                stroke,
            ));
        }
        5 => {
            painter.add(egui::Shape::line(
                vec![
                    center + egui::vec2(-8.0, 1.0),
                    center + egui::vec2(-4.0, 1.0),
                    center + egui::vec2(-1.0, -7.0),
                    center + egui::vec2(2.0, 7.0),
                    center + egui::vec2(5.0, -1.0),
                    center + egui::vec2(8.0, -1.0),
                ],
                stroke,
            ));
        }
        6 => {
            painter.rect_stroke(rect, 2, stroke, egui::StrokeKind::Inside);
            for y in [-4.0, 0.0, 4.0] {
                painter.line_segment(
                    [center + egui::vec2(-4.0, y), center + egui::vec2(5.0, y)],
                    stroke,
                );
            }
        }
        _ => {
            for (y, x) in [(-5.0, -3.0), (0.0, 4.0), (5.0, -1.0)] {
                painter.line_segment(
                    [center + egui::vec2(-8.0, y), center + egui::vec2(8.0, y)],
                    stroke,
                );
                painter.circle_filled(center + egui::vec2(x, y), 2.5, color);
            }
        }
    }
}
