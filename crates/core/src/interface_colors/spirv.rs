//! Append RGB mapping to the verified mobile GUI fragment shaders.
//! Opcode values follow Khronos SPIR-V 1.0 and GLSL.std.450 grammars.
//! Original control flow, texture sampling, interfaces and alpha are retained.
use super::{Animation, Effect, Gradient, Palette};
use anyhow::{ensure, Context, Result};
use std::collections::HashMap;

fn instruction(op: u32, args: &[u32]) -> Vec<u32> {
    let mut words = vec![((args.len() as u32 + 1) << 16) | op];
    words.extend(args);
    words
}
fn parse(words: &[u32]) -> Result<Vec<Vec<u32>>> {
    ensure!(
        words.len() >= 5 && words[0] == 0x07230203 && words[1] == 0x10000,
        "Unsupported SPIR-V module"
    );
    let mut result = vec![];
    let mut at = 5;
    while at < words.len() {
        let len = (words[at] >> 16) as usize;
        ensure!(
            len > 0 && at + len <= words.len(),
            "Invalid SPIR-V instruction bounds"
        );
        result.push(words[at..at + len].to_vec());
        at += len;
    }
    Ok(result)
}
struct Emit {
    next: u32,
    float: u32,
    boolean: u32,
    ext: u32,
    constants: HashMap<u32, u32>,
    declarations: Vec<u32>,
    body: Vec<u32>,
}
impl Emit {
    fn id(&mut self) -> u32 {
        let id = self.next;
        self.next += 1;
        id
    }
    fn constant(&mut self, value: f32) -> u32 {
        let bits = value.to_bits();
        if let Some(id) = self.constants.get(&bits) {
            return *id;
        }
        let id = self.id();
        self.declarations
            .extend(instruction(43, &[self.float, id, bits]));
        self.constants.insert(bits, id);
        id
    }
    fn result(&mut self, op: u32, ty: u32, args: &[u32]) -> u32 {
        let id = self.id();
        let mut a = vec![ty, id];
        a.extend(args);
        self.body.extend(instruction(op, &a));
        id
    }
    fn binary(&mut self, op: u32, a: u32, b: u32) -> u32 {
        self.result(op, self.float, &[a, b])
    }
    fn ext(&mut self, op: u32, args: &[u32]) -> u32 {
        let mut a = vec![self.ext, op];
        a.extend(args);
        self.result(12, self.float, &a)
    }
    fn compare(&mut self, op: u32, a: u32, b: u32) -> u32 {
        self.result(op, self.boolean, &[a, b])
    }
    fn select(&mut self, condition: u32, a: u32, b: u32) -> u32 {
        self.result(169, self.float, &[condition, a, b])
    }
    fn mix(&mut self, a: u32, b: u32, t: u32) -> u32 {
        self.ext(46, &[a, b, t])
    }
    fn layer(&mut self, rgb: [u8; 3], e: Effect, uv: [u32; 2], time: u32) -> [u32; 3] {
        let zero = self.constant(0.0);
        let one = self.constant(1.0);
        let half = self.constant(0.5);
        let scale = self.constant(100.0 / e.spread_percent as f32);
        let x = self.binary(131, uv[0], half);
        let y = self.binary(131, uv[1], half);
        let coord = match e.gradient {
            Gradient::Solid => zero,
            Gradient::Linear => {
                let angle = (e.angle_degrees as f32).to_radians();
                let co = self.constant(angle.cos());
                let si = self.constant(angle.sin());
                let a = self.binary(133, x, co);
                let b = self.binary(133, y, si);
                let sum = self.binary(129, a, b);
                let scaled = self.binary(133, sum, scale);
                self.binary(129, half, scaled)
            }
            Gradient::Radial => {
                let a = self.binary(133, x, x);
                let b = self.binary(133, y, y);
                let sum = self.binary(129, a, b);
                let len = self.ext(31, &[sum]);
                let twice = self.constant(2.0);
                let factor = self.binary(133, twice, scale);
                self.binary(133, len, factor)
            }
        };
        let mut t = self.ext(43, &[coord, zero, one]);
        let speed = self.constant(1000.0 / e.period_ms as f32);
        let phase = self.binary(133, time, speed);
        if matches!(e.animation, Animation::ColorFade | Animation::Flow) {
            let angle = if e.animation == Animation::Flow {
                let co = self.binary(133, coord, half);
                self.binary(131, co, phase)
            } else {
                phase
            };
            let tau = self.constant(std::f32::consts::TAU);
            let angle = self.binary(133, angle, tau);
            let cosine = self.ext(14, &[angle]);
            let cosine = self.binary(133, half, cosine);
            let cycle = self.binary(131, half, cosine);
            t = if e.animation == Animation::Flow || e.gradient == Gradient::Solid {
                cycle
            } else {
                let inverse = self.binary(131, one, t);
                self.mix(t, inverse, cycle)
            };
        }
        let mut color = std::array::from_fn(|i| {
            let a = self.constant(rgb[i] as f32 / 255.0);
            let b = self.constant(e.secondary[i] as f32 / 255.0);
            self.mix(a, b, t)
        });
        if e.animation == Animation::Rainbow {
            let max = self.ext(40, &[color[0], color[1]]);
            let value = self.ext(40, &[max, color[2]]);
            let quarter = self.constant(0.25);
            let coord = self.binary(133, coord, quarter);
            let phase = self.binary(129, phase, coord);
            color = std::array::from_fn(|i| {
                let off = self.constant([0.0, 2.0 / 3.0, 1.0 / 3.0][i]);
                let hue = self.binary(129, phase, off);
                let hue = self.ext(10, &[hue]);
                let six = self.constant(6.0);
                let three = self.constant(3.0);
                let hue = self.binary(133, hue, six);
                let hue = self.binary(131, hue, three);
                let hue = self.ext(4, &[hue]);
                let hue = self.binary(131, hue, one);
                let hue = self.ext(43, &[hue, zero, one]);
                self.binary(133, hue, value)
            });
        }
        color
    }
}

pub(super) fn patch(bytes: &[u8], palette: Palette) -> Result<Vec<u8>> {
    ensure!(bytes.len().is_multiple_of(4), "Misaligned SPIR-V");
    let words: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let ins = parse(&words)?;
    let find = |op: u32, predicate: &dyn Fn(&[u32]) -> bool| -> Result<u32> {
        ins.iter()
            .find(|i| i[0] & 65535 == op && predicate(i))
            .map(|i| i[1])
            .context("Required shader declaration is missing")
    };
    let float = find(22, &|i| i.len() == 3 && i[2] == 32)?;
    let boolean = find(20, &|_| true)?;
    let vec2 = find(23, &|i| i[2] == float && i[3] == 2)?;
    let vec4 = find(23, &|i| i[2] == float && i[3] == 4)?;
    let uint = find(21, &|i| i[2] == 32 && i[3] == 0)?;
    let ext = find(11, &|i| i.get(2) == Some(&0x4c534c47))?; // GLSL.std.450
    let output = ins
        .iter()
        .find(|i| i[0] & 65535 == 71 && i.len() == 4 && i[2] == 30 && i[3] == 0)
        .and_then(|_| {
            ins.iter()
                .find(|i| i[0] & 65535 == 59 && i.len() >= 4 && i[3] == 3)
        })
        .map(|i| i[2])
        .context("Missing fragment output")?;
    ensure!(
        ins.iter()
            .filter(|i| i[0] & 65535 == 62 && i.get(1) == Some(&output))
            .count()
            == 1,
        "Unexpected fragment output stores"
    );
    let frag = ins
        .iter()
        .find(|i| i[0] & 65535 == 71 && i.len() == 4 && i[2] == 11 && i[3] == 15)
        .map(|i| i[1])
        .context("Missing fragment coordinates")?;
    let existing_global = ins
        .iter()
        .find(|i| {
            i[0] & 65535 == 71
                && i.len() == 4
                && i[2] == 33
                && i[3] == 0
                && ins.iter().any(|j| {
                    j[0] & 65535 == 71 && j.len() == 4 && j[1] == i[1] && j[2] == 34 && j[3] == 0
                })
        })
        .map(|i| i[1]);
    let mut e = Emit {
        next: words[3],
        float,
        boolean,
        ext,
        constants: HashMap::new(),
        declarations: vec![],
        body: vec![],
    };
    for i in &ins {
        if i[0] & 65535 == 43 && i.len() == 4 && i[1] == float {
            e.constants.insert(i[3], i[2]);
        }
    }
    let mut decorations = vec![];
    let variable = if let Some(variable) = existing_global {
        variable
    } else {
        let global = e.id();
        let pointer = e.id();
        let variable = e.id();
        e.declarations
            .extend(instruction(30, &[global, vec4, vec2]));
        e.declarations
            .extend(instruction(32, &[pointer, 2, global]));
        e.declarations
            .extend(instruction(59, &[pointer, variable, 2]));
        for args in [[global, 0, 35, 304], [global, 1, 35, 800]] {
            decorations.extend(instruction(72, &args));
        }
        decorations.extend(instruction(71, &[global, 2]));
        decorations.extend(instruction(71, &[variable, 34, 0]));
        decorations.extend(instruction(71, &[variable, 33, 0]));
        variable
    };
    let pointer_for = |ty: u32, e: &mut Emit| -> u32 {
        if let Some(i) = ins
            .iter()
            .find(|i| i[0] & 65535 == 32 && i[2] == 2 && i[3] == ty)
        {
            i[1]
        } else {
            let id = e.id();
            e.declarations.extend(instruction(32, &[id, 2, ty]));
            id
        }
    };
    let ptr4 = pointer_for(vec4, &mut e);
    let ptr2 = pointer_for(vec2, &mut e);
    let index0 = e.id();
    let index1 = e.id();
    e.declarations.extend(instruction(43, &[uint, index0, 0]));
    e.declarations.extend(instruction(43, &[uint, index1, 1]));
    let (time_chain, view_chain) = if existing_global.is_some() {
        let time_index = e.id();
        let view_index = e.id();
        e.declarations
            .extend(instruction(43, &[uint, time_index, 11]));
        e.declarations
            .extend(instruction(43, &[uint, view_index, 31]));
        (
            vec![variable, index0, time_index],
            vec![variable, index0, view_index],
        )
    } else {
        (vec![variable, index0], vec![variable, index1])
    };
    let timeptr = e.result(65, ptr4, &time_chain);
    let time4 = e.result(61, vec4, &[timeptr]);
    let time = e.result(81, float, &[time4, 3]);
    let viewptr = e.result(65, ptr2, &view_chain);
    let view = e.result(61, vec2, &[viewptr]);
    let position = e.result(61, vec4, &[frag]);
    let uv = std::array::from_fn(|i| {
        let p = e.result(81, float, &[position, i as u32]);
        let v = e.result(81, float, &[view, i as u32]);
        e.binary(133, p, v)
    });
    let source = ins
        .iter()
        .find(|i| i[0] & 65535 == 62 && i.get(1) == Some(&output))
        .unwrap()[2];
    let rgba: [u32; 4] = std::array::from_fn(|i| e.result(81, float, &[source, i as u32]));
    let hi = e.ext(40, &[rgba[0], rgba[1]]);
    let hi = e.ext(40, &[hi, rgba[2]]);
    let lo = e.ext(37, &[rgba[0], rgba[1]]);
    let lo = e.ext(37, &[lo, rgba[2]]);
    let delta = e.binary(131, hi, lo);
    let threshold = e.constant(0.04);
    let dark = e.constant(0.40);
    let neutral = e.compare(184, delta, threshold);
    let low = e.compare(184, hi, dark);
    let neutral = e.result(167, boolean, &[neutral, low]);
    let t = e.binary(136, hi, dark);
    let background = e.layer(palette.background, palette.background_effect, uv, time);
    let surface = e.layer(palette.surface, palette.surface_effect, uv, time);
    let accent = e.layer(palette.accent, palette.accent_effect, uv, time);
    let ratio = e.constant(1.55);
    let red = e.binary(133, rgba[0], ratio);
    let r = e.compare(186, rgba[2], red);
    let ratio = e.constant(1.30);
    let green = e.binary(133, rgba[1], ratio);
    let g = e.compare(186, rgba[2], green);
    let minimum = e.constant(0.025);
    let b = e.compare(186, rgba[2], minimum);
    let blue = e.result(167, boolean, &[r, g]);
    let blue = e.result(167, boolean, &[blue, b]);
    let mapped: [u32; 3] = std::array::from_fn(|i| {
        let c = e.mix(background[i], surface[i], t);
        let c = e.select(neutral, c, rgba[i]);
        e.select(blue, accent[i], c)
    });
    let result = e.result(80, vec4, &[mapped[0], mapped[1], mapped[2], rgba[3]]);
    let mut out = words[..5].to_vec();
    out[3] = e.next;
    let mut decorated = false;
    let mut declared = false;
    for i in ins {
        let op = i[0] & 65535;
        if !decorated && (19..=39).contains(&op) {
            out.extend(&decorations);
            decorated = true;
        }
        if !declared && op == 54 {
            out.extend(&e.declarations);
            declared = true;
        }
        if op == 62 && i[1] == output {
            out.extend(&e.body);
            out.extend(instruction(62, &[output, result]));
        } else {
            out.extend(i);
        }
    }
    ensure!(decorated && declared, "Missing SPIR-V insertion points");
    Ok(out.into_iter().flat_map(u32::to_le_bytes).collect())
}
