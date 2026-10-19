use eframe::egui;
use solstrap_core::interface_colors::{Animation, Effect, Gradient, Palette};

pub struct Editor {
    pub palette: Palette,
    hex: [[String; 2]; 3],
    preview_motion: bool,
}
fn hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}
fn parse(s: &str) -> Option<[u8; 3]> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 || !s.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&s[..2], 16).ok()?,
        u8::from_str_radix(&s[2..4], 16).ok()?,
        u8::from_str_radix(&s[4..], 16).ok()?,
    ])
}
fn color(ui: &mut egui::Ui, label: &str, c: &mut [u8; 3], text: &mut String) -> bool {
    ui.horizontal(|ui| {
        if ui
            .color_edit_button_srgb(c)
            .on_hover_text("Open color picker")
            .changed()
        {
            *text = hex(*c);
        }
        ui.label(label);
        if ui
            .add(
                egui::TextEdit::singleline(text)
                    .desired_width(88.0)
                    .char_limit(7),
            )
            .changed()
        {
            if let Some(value) = parse(text) {
                *c = value;
            }
        }
        let valid = parse(text).is_some();
        if !valid {
            ui.colored_label(egui::Color32::LIGHT_RED, "Use six hex digits");
        }
        valid
    })
    .inner
}
impl Editor {
    pub fn new(palette: Palette) -> Self {
        Self {
            hex: [
                [
                    hex(palette.background),
                    hex(palette.background_effect.secondary),
                ],
                [hex(palette.surface), hex(palette.surface_effect.secondary)],
                [hex(palette.accent), hex(palette.accent_effect.secondary)],
            ],
            palette,
            preview_motion: true,
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, seconds: f32, allow_motion: bool) -> bool {
        ui.label("Choose any color with the swatch or enter #RRGGBB.");
        ui.small("Gradients span the game window. Animation uses the client clock; restart Ferns after Apply. Gradients and motion need in-game verification.");
        if self.palette.animated() {
            ui.add_enabled(
                allow_motion,
                egui::Checkbox::new(&mut self.preview_motion, "Animate preview"),
            );
            if !allow_motion {
                ui.small("Preview paused by Reduce interface motion. Game animation is controlled separately below.");
            }
        }
        let moving = allow_motion && self.preview_motion;
        let time = if moving { seconds } else { 0.0 };
        let p = &mut self.palette;
        let mut valid = true;
        for (i, (label, primary, effect)) in [
            (
                "Dark background",
                &mut p.background,
                &mut p.background_effect,
            ),
            ("Lighter surfaces", &mut p.surface, &mut p.surface_effect),
            ("Accent", &mut p.accent, &mut p.accent_effect),
        ]
        .into_iter()
        .enumerate()
        {
            ui.push_id(i, |ui| {
                ui.add_space(8.0);
                ui.strong(label);
                valid &= color(ui, "Primary", primary, &mut self.hex[i][0]);
                ui.horizontal(|ui| {
                    ui.label("Fill");
                    egui::ComboBox::from_id_salt("fill")
                        .selected_text(format!("{:?}", effect.gradient))
                        .show_ui(ui, |ui| {
                            for g in [Gradient::Solid, Gradient::Linear, Gradient::Radial] {
                                ui.selectable_value(&mut effect.gradient, g, format!("{g:?}"));
                            }
                        });
                    if effect.gradient == Gradient::Solid && effect.animation == Animation::Flow {
                        effect.animation = Animation::None;
                    }
                    ui.label("Animation");
                    egui::ComboBox::from_id_salt("motion")
                        .selected_text(animation_label(effect.animation))
                        .show_ui(ui, |ui| {
                            for a in [
                                Animation::None,
                                Animation::ColorFade,
                                Animation::Flow,
                                Animation::Rainbow,
                            ] {
                                let response = ui.add_enabled(
                                    a != Animation::Flow || effect.gradient != Gradient::Solid,
                                    egui::Button::selectable(
                                        effect.animation == a,
                                        animation_label(a),
                                    ),
                                );
                                if response.clicked() {
                                    effect.animation = a;
                                }
                            }
                        });
                });
                if effect.active() {
                    valid &= color(ui, "Secondary", &mut effect.secondary, &mut self.hex[i][1]);
                }
                if effect.gradient == Gradient::Linear {
                    ui.add(egui::Slider::new(&mut effect.angle_degrees, 0..=359).text("Angle °"));
                }
                if effect.gradient != Gradient::Solid {
                    ui.add(
                        egui::Slider::new(&mut effect.spread_percent, 25..=400).text("Spread %"),
                    );
                }
                if effect.animated() {
                    let mut duration = effect.period_ms as f32 / 1000.0;
                    if ui
                        .add(egui::Slider::new(&mut duration, 0.5..=60.0).text("Cycle seconds"))
                        .changed()
                    {
                        effect.period_ms = (duration * 1000.0).round() as u32;
                    }
                }
                preview(ui, *primary, *effect, time);
            });
        }
        if moving && self.palette.animated() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        valid && self.palette.validate().is_ok()
    }
}
fn animation_label(a: Animation) -> &'static str {
    match a {
        Animation::None => "Off",
        Animation::ColorFade => "Color fade",
        Animation::Flow => "Gradient flow",
        Animation::Rainbow => "Rainbow",
    }
}
fn preview(ui: &mut egui::Ui, primary: [u8; 3], e: Effect, time: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(520.0), 54.0),
        egui::Sense::hover(),
    );
    let mut mesh = egui::Mesh::default();
    const X: usize = 48;
    const Y: usize = 8;
    for y in 0..=Y {
        for x in 0..=X {
            let uv = [x as f32 / X as f32, y as f32 / Y as f32];
            let c = e.sample(primary, uv, time);
            mesh.colored_vertex(
                egui::pos2(
                    rect.left() + uv[0] * rect.width(),
                    rect.top() + uv[1] * rect.height(),
                ),
                egui::Color32::from_rgb(c[0], c[1], c[2]),
            );
        }
    }
    for y in 0..Y {
        for x in 0..X {
            let i = (y * (X + 1) + x) as u32;
            let w = (X + 1) as u32;
            mesh.add_triangle(i, i + 1, i + w);
            mesh.add_triangle(i + 1, i + w + 1, i + w);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_input_accepts_full_rgb_and_rejects_incomplete() {
        assert_eq!(parse("#12aBef"), Some([18, 171, 239]));
        for s in ["#123", "#GGFFFF", "éFFFFF", "#1234567"] {
            assert!(parse(s).is_none());
        }
    }
}
