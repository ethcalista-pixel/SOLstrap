//! Decorative, bounded cursor wake. No widget interaction or continuous idle loop.
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use std::collections::VecDeque;
const LIFETIME: f64 = 1.35;
const CAP: usize = 40;
struct Wake {
    pos: Pos2,
    direction: Vec2,
    born: f64,
}
#[derive(Default)]
pub struct Field {
    wakes: VecDeque<Wake>,
    last: Option<(Pos2, f64)>,
    now: f64,
}
impl Field {
    pub fn update(&mut self, ctx: &egui::Context, reduced: bool) {
        let (now, pointer, focused) = ctx.input(|i| {
            (
                i.time,
                i.pointer.hover_pos(),
                i.viewport().focused.unwrap_or(true),
            )
        });
        self.now = now;
        if reduced || !focused {
            self.wakes.clear();
            self.last = None;
            return;
        }
        self.wakes.retain(|w| now - w.born < LIFETIME);
        match pointer {
            Some(pos) => {
                if let Some((previous, time)) = self.last {
                    let delta = pos - previous;
                    if delta.length_sq() > 9.0 && now - time > 1.0 / 120.0 {
                        let count = (delta.length() / 14.0).ceil().clamp(1.0, 6.0) as usize;
                        for n in 1..=count {
                            self.wakes.push_back(Wake {
                                pos: previous + delta * (n as f32 / count as f32),
                                direction: delta.normalized(),
                                born: now,
                            });
                        }
                        while self.wakes.len() > CAP {
                            self.wakes.pop_front();
                        }
                        self.last = Some((pos, now));
                    }
                } else {
                    self.last = Some((pos, now));
                }
            }
            None => self.last = None,
        }
        if !self.wakes.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
    pub fn grid(&self, ctx: &egui::Context) {
        let rect = ctx.content_rect();
        // Painting only: the overlay never registers an interactive area, so
        // inputs, navigation and color-picker popups retain their hit targets.
        let p = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("solar-cursor-wake"),
            ))
            .with_clip_rect(rect);
        let light = !ctx.style().visuals.dark_mode;
        let step = 16.0_f32.max((rect.area() / 10000.0).sqrt());
        let start = egui::pos2(
            (rect.left() / step).ceil() * step,
            (rect.top() / step).ceil() * step,
        );
        let mut y = start.y;
        while y < rect.bottom() {
            let mut x = start.x;
            while x < rect.right() {
                let base = egui::pos2(x, y);
                let mut shift = Vec2::ZERO;
                let mut heat = 0.0_f32;
                for w in &self.wakes {
                    let delta = base - w.pos;
                    let d2 = delta.length_sq();
                    if d2 > 14400.0 {
                        continue;
                    }
                    let age = (self.now - w.born) as f32;
                    let distance = d2.sqrt();
                    let influence = (1.0 - distance / 120.0).powi(2)
                        * (1.0 - age / LIFETIME as f32).max(0.0).powi(2);
                    let radial = if distance > 0.5 {
                        delta / distance
                    } else {
                        Vec2::ZERO
                    };
                    shift += (w.direction * 10.0
                        + radial * ((distance * 0.055 - age * 8.0).sin() * 9.0))
                        * influence;
                    heat += influence;
                }
                let strength = heat.min(1.0);
                let c = if light {
                    Color32::from_rgba_unmultiplied(110, 82, 41, 37 + (strength * 45.0) as u8)
                } else {
                    Color32::from_rgba_unmultiplied(230, 170, 83, 31 + (strength * 65.0) as u8)
                };
                let shift = shift / (1.0 + shift.length() / 24.0);
                p.circle_filled(base + shift, 0.72 + strength * 0.55, c);
                x += step;
            }
            y += step;
        }
    }
    pub fn trail(&self, ctx: &egui::Context) {
        if self.wakes.is_empty() {
            return;
        }
        let p = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("solar-cursor-wake"),
        ));
        for (a, b) in self.wakes.iter().zip(self.wakes.iter().skip(1)) {
            let fade = (1.0 - (self.now - a.born) / LIFETIME).clamp(0.0, 1.0) as f32;
            let sway = Vec2::new(-a.direction.y, a.direction.x)
                * ((self.now - a.born) as f32 * 7.0).sin()
                * (1.0 - fade)
                * 7.0;
            let line = [a.pos + sway, b.pos + sway];
            p.line_segment(
                line,
                Stroke::new(
                    7.0,
                    Color32::from_rgba_unmultiplied(255, 137, 36, (fade * 9.0) as u8),
                ),
            );
            p.line_segment(
                line,
                Stroke::new(
                    1.2,
                    Color32::from_rgba_unmultiplied(255, 184, 82, (fade * 65.0) as u8),
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        field: &mut Field,
        ctx: &egui::Context,
        time: f64,
        pos: Pos2,
        reduced: bool,
        focused: bool,
    ) {
        let mut input = egui::RawInput {
            time: Some(time),
            events: vec![egui::Event::PointerMoved(pos)],
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .focused = Some(focused);
        let _ = ctx.run(input, |ctx| field.update(ctx, reduced));
    }
    #[test]
    fn wake_settles_is_bounded_and_respects_motion_and_focus() {
        let ctx = egui::Context::default();
        let mut field = Field::default();
        frame(&mut field, &ctx, 0.0, egui::pos2(0.0, 0.0), false, true);
        for n in 1..100 {
            frame(
                &mut field,
                &ctx,
                n as f64 * 0.02,
                egui::pos2(n as f32 * 9.0, 80.0),
                false,
                true,
            );
            assert!(field.wakes.len() <= CAP);
        }
        assert!(!field.wakes.is_empty());
        frame(&mut field, &ctx, 4.0, egui::pos2(891.0, 80.0), false, true);
        assert!(field.wakes.is_empty());
        frame(&mut field, &ctx, 4.1, egui::pos2(920.0, 80.0), false, true);
        assert!(!field.wakes.is_empty());
        frame(&mut field, &ctx, 4.2, egui::pos2(930.0, 80.0), true, true);
        assert!(field.wakes.is_empty());
        assert!(field.last.is_none());
        frame(&mut field, &ctx, 4.3, egui::pos2(940.0, 80.0), false, true);
        frame(&mut field, &ctx, 4.4, egui::pos2(980.0, 80.0), false, true);
        assert!(!field.wakes.is_empty());
        frame(&mut field, &ctx, 4.5, egui::pos2(990.0, 80.0), false, false);
        assert!(field.wakes.is_empty());
        assert!(field.last.is_none());
    }
}
