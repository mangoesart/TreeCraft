//! A colour wheel that opens from a colour swatch (picture 31, Blender-like): a hue/saturation
//! disc with a brightness bar, RGB or HSV values, a hex code, and presets (a fixed palette plus the
//! colours used last, which are remembered).

use crate::prefs::{hsv_to_rgb, rgb_to_hsv};
use crate::ui::*;
use macroquad::models::Vertex;
use macroquad::prelude::*;

/// A fixed palette of presets.
pub const PRESETS: [[f32; 3]; 12] = [
    [1.0, 1.0, 1.0],
    [0.5, 0.5, 0.5],
    [0.0, 0.0, 0.0],
    [1.0, 0.62, 0.2],
    [1.0, 0.85, 0.2],
    [0.9, 0.25, 0.2],
    [0.95, 0.4, 0.7],
    [0.6, 0.35, 0.95],
    [0.2, 0.5, 1.0],
    [0.2, 0.8, 0.9],
    [0.3, 0.85, 0.35],
    [0.55, 0.38, 0.22],
];

/// An open colour wheel: which colour it edits, where it opened, and its own state.
pub struct Wheel {
    pub key: String,
    pub at: Vec2,
    pub hsv_mode: bool,
    /// 1 dragging on the disc, 2 on the brightness bar.
    pub drag: u8,
    pub hex: String,
    pub hex_focus: bool,
    /// Hue / saturation / value kept here so hue survives grey colours.
    pub hsv: (f32, f32, f32),
    pub last: [f32; 3],
    /// Where it was drawn (clicks there don't close what opened it).
    pub rect: Rect,
}

impl Wheel {
    pub fn new(key: &str, at: Vec2, c: [f32; 3]) -> Self {
        Wheel { key: key.to_string(), at, hsv_mode: true, drag: 0, hex: to_hex(c), hex_focus: false, hsv: rgb_to_hsv(c[0], c[1], c[2]), last: c, rect: Rect::new(at.x, at.y, 0.0, 0.0) }
    }
}

pub fn to_hex(c: [f32; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", (c[0].clamp(0.0, 1.0) * 255.0).round() as u8, (c[1].clamp(0.0, 1.0) * 255.0).round() as u8, (c[2].clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn from_hex(s: &str) -> Option<[f32; 3]> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some([((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0])
}

fn vtx(p: Vec2, c: Color) -> Vertex {
    Vertex { position: vec3(p.x, p.y, 0.0), uv: Vec2::ZERO, color: [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8, (c.a * 255.0) as u8], normal: Vec4::ZERO }
}

/// Draw the wheel and edit `c`; `recent` are the colours used last. Returns true when `c`
/// changed (the caller remembers it in `recent` when the wheel closes).
pub fn draw(ui: &mut Ui, w: &mut Wheel, c: &mut [f32; 3], recent: &[[f32; 3]]) -> bool {
    // a colour changed from outside (another swatch, a preset elsewhere): follow it
    if *c != w.last {
        w.hsv = rgb_to_hsv(c[0], c[1], c[2]);
        w.last = *c;
        if !w.hex_focus {
            w.hex = to_hex(*c);
        }
    }
    let pw = 236.0;
    let ph = 352.0;
    let x = w.at.x.min(ui.sw - pw - 4.0).max(4.0);
    let y = w.at.y.min(ui.sh - ph - 4.0).max(4.0);
    let p = Rect::new(x, y, pw, ph);
    w.rect = p;
    draw_rectangle(p.x + 3.0, p.y + 4.0, p.w, p.h, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_round_rect(p, 6.0, C_PANEL);
    draw_round_rect_lines(p, 6.0, 1.0, C_LINE);
    ui.record("colorwheel", p);
    let (mut h, mut s, mut v) = w.hsv;
    let mut changed = false;
    let m = ui.mouse();
    // the disc: hue around, saturation outward, at the current brightness
    let rad = 76.0;
    let ctr = vec2(p.x + 12.0 + rad, p.y + 12.0 + rad);
    let (segs, rings) = (72usize, 8usize);
    let mut verts = vec![];
    let mut idx: Vec<u16> = vec![];
    for ri in 0..=rings {
        let sr = ri as f32 / rings as f32;
        for si in 0..=segs {
            let a = si as f32 / segs as f32;
            let (r, g, b) = hsv_to_rgb(a, sr, v);
            let ang = a * std::f32::consts::TAU;
            verts.push(vtx(ctr + vec2(ang.cos(), -ang.sin()) * rad * sr, Color::new(r, g, b, 1.0)));
        }
    }
    for ri in 0..rings {
        for si in 0..segs {
            let a = (ri * (segs + 1) + si) as u16;
            let b = a + 1;
            let cc = a + (segs + 1) as u16;
            let d = cc + 1;
            idx.extend_from_slice(&[a, cc, b, b, cc, d]);
        }
    }
    draw_mesh(&Mesh { vertices: verts, indices: idx, texture: None });
    draw_circle_lines(ctr.x, ctr.y, rad, 1.0, Color::new(0.0, 0.0, 0.0, 0.5));
    let ang = h * std::f32::consts::TAU;
    let dot = ctr + vec2(ang.cos(), -ang.sin()) * rad * s;
    draw_circle_lines(dot.x, dot.y, 5.0, 2.0, if v > 0.5 { BLACK } else { WHITE });
    // the brightness bar
    let bar = Rect::new(p.x + p.w - 30.0, p.y + 12.0, 16.0, rad * 2.0);
    let (tr, tg, tb) = hsv_to_rgb(h, s, 1.0);
    draw_mesh(&Mesh {
        vertices: vec![vtx(vec2(bar.x, bar.y), Color::new(tr, tg, tb, 1.0)), vtx(vec2(bar.x + bar.w, bar.y), Color::new(tr, tg, tb, 1.0)), vtx(vec2(bar.x, bar.y + bar.h), BLACK), vtx(vec2(bar.x + bar.w, bar.y + bar.h), BLACK)],
        indices: vec![0, 2, 1, 1, 2, 3],
        texture: None,
    });
    draw_rectangle_lines(bar.x, bar.y, bar.w, bar.h, 1.0, C_LINE);
    let vy = bar.y + (1.0 - v) * bar.h;
    draw_rectangle_lines(bar.x - 2.0, vy - 2.0, bar.w + 4.0, 4.0, 1.5, WHITE);
    // dragging on them
    let free = !ui.blocked;
    if free && ui.inp.l_pressed {
        if m.distance(ctr) <= rad + 3.0 {
            w.drag = 1;
        } else if bar.contains(m) {
            w.drag = 2;
        }
    }
    if !ui.inp.l_down {
        w.drag = 0;
    }
    match w.drag {
        1 => {
            let d = m - ctr;
            h = (-d.y).atan2(d.x).rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
            s = (d.length() / rad).clamp(0.0, 1.0);
            changed = true;
        }
        2 => {
            v = (1.0 - (m.y - bar.y) / bar.h).clamp(0.0, 1.0);
            changed = true;
        }
        _ => {}
    }
    // RGB / HSV values
    let mut yy = p.y + rad * 2.0 + 22.0;
    let half = (p.w - 24.0) * 0.5;
    if ui.button_ex("cw.rgb", Rect::new(p.x + 12.0, yy, half, 20.0), "RGB", !w.hsv_mode, true) {
        w.hsv_mode = false;
    }
    if ui.button_ex("cw.hsv", Rect::new(p.x + 12.0 + half, yy, half, 20.0), "HSV", w.hsv_mode, true) {
        w.hsv_mode = true;
    }
    yy += 26.0;
    let names = if w.hsv_mode { ["Hue", "Saturation", "Value"] } else { ["Red", "Green", "Blue"] };
    let mut vals = if w.hsv_mode { [h, s, v] } else { *c };
    for (i, n) in names.iter().enumerate() {
        let rr = Rect::new(p.x + 12.0, yy, p.w - 24.0, 19.0);
        let before = vals[i];
        ui.slider(&format!("cw.{}.{}", w.key, n), rr, &mut vals[i], 0.0, 1.0, 3);
        ui.text(n, rr.x + 6.0, rr.y + 2.0, 11.5, C_TEXT);
        if vals[i] != before {
            changed = true;
            if w.hsv_mode {
                h = vals[0];
                s = vals[1];
                v = vals[2];
            } else {
                *c = vals;
                let (h2, s2, v2) = rgb_to_hsv(c[0], c[1], c[2]);
                v = v2;
                if s2 > 1e-4 {
                    h = h2;
                }
                s = s2;
            }
        }
        yy += 22.0;
    }
    // hex code
    ui.text("Hex", p.x + 12.0, yy + 4.0, 12.0, C_TEXT_DIM);
    let mut hex = std::mem::take(&mut w.hex);
    let mut focus = w.hex_focus;
    crate::matpanel::text_field(ui, &format!("cw.hex.{}", w.key), Rect::new(p.x + 44.0, yy, p.w - 56.0, 21.0), &mut hex, &mut focus);
    if let Some(rgb) = from_hex(&hex) {
        if focus && rgb != *c {
            *c = rgb;
            let (h2, s2, v2) = rgb_to_hsv(c[0], c[1], c[2]);
            h = h2;
            s = s2;
            v = v2;
            changed = true;
        }
    }
    w.hex = hex;
    w.hex_focus = focus;
    yy += 28.0;
    // presets: the palette, then the colours used last
    ui.text("Presets", p.x + 12.0, yy, 11.5, C_TEXT_DIM);
    yy += 16.0;
    let cell = (p.w - 24.0) / 12.0;
    let mut pick = None;
    for (row, list) in [PRESETS.to_vec(), recent.iter().take(12).cloned().collect::<Vec<_>>()].iter().enumerate() {
        for (k, col) in list.iter().enumerate() {
            let r = Rect::new(p.x + 12.0 + k as f32 * cell, yy + row as f32 * (cell + 3.0), cell - 2.0, cell - 2.0);
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(col[0], col[1], col[2], 1.0));
            let hot = r.contains(m);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, if hot { WHITE } else { C_LINE });
            if hot {
                ui.tooltip = Some(if row == 0 { to_hex(*col) } else { format!("{} (used last)", to_hex(*col)) });
                if ui.inp.l_pressed {
                    pick = Some(*col);
                }
            }
        }
    }
    if let Some(col) = pick {
        *c = col;
        let (h2, s2, v2) = rgb_to_hsv(c[0], c[1], c[2]);
        h = h2;
        s = s2;
        v = v2;
        changed = true;
    }
    if changed && (w.drag != 0 || w.hsv_mode && pick.is_none() && !w.hex_focus) {
        let (r, g, b) = hsv_to_rgb(h, s, v);
        *c = [r, g, b];
    }
    w.hsv = (h, s, v);
    w.last = *c;
    if changed && !w.hex_focus {
        w.hex = to_hex(*c);
    }
    changed
}
