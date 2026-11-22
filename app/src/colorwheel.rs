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

// ------------------------------------------------------------------ swatches (pictures 74, 75)

/// A two-colour linear gradient for a material's Color (pictures 74, 75): from `c1` at a branch's
/// base to `c2` at its tip, reversed by `flip`. It has its own base colour, so turning it off
/// gives back the plain colour set before.
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub on: bool,
    pub c1: [f32; 3],
    pub c2: [f32; 3],
    pub flip: bool,
    /// Where along the branch the two colours meet half way (picture 77): 0.5 the middle,
    /// lower toward the base, higher toward the tip.
    pub mid: f32,
}

/// Where a gradient is at 	 (0 base .. 1 tip) with its blend point at mid: 0..0.5 up to the
/// blend point, 0.5..1 after it.
pub fn grad_t(t: f32, mid: f32) -> f32 {
    let m = mid.clamp(0.02, 0.98);
    let t = t.clamp(0.0, 1.0);
    if t < m { 0.5 * t / m } else { 0.5 + 0.5 * (t - m) / (1.0 - m) }
}

/// What Copy in the wheel took: a colour, or a whole gradient (picture 75).
#[derive(Clone)]
enum Clip {
    Color([f32; 3]),
    Grad(Gradient),
}

/// The wheel opened from a panel swatch: what it edits and what it changed this frame.
struct Open {
    wheel: Wheel,
    value: [f32; 3],
    from_wheel: bool,
    swatch: Rect,
    seen: bool,
    grad: Option<Gradient>,
    /// Which colour the wheel edits when the gradient is on: 0 its base, 1 its tip.
    stop: u8,
    extra: Rect,
    title: Rect,
    /// Dragging the wheel by its title strip (picture 75): the grab point's offset.
    drag: Option<Vec2>,
    /// The swatch's plain colour (picture 76: what comes back when the gradient is turned off).
    plain: [f32; 3],
}

thread_local! {
    static OPEN: std::cell::RefCell<Option<Open>> = const { std::cell::RefCell::new(None) };
    static CLOSED: std::cell::RefCell<Vec<[f32; 3]>> = const { std::cell::RefCell::new(Vec::new()) };
    static CLIP: std::cell::RefCell<Option<Clip>> = const { std::cell::RefCell::new(None) };
}

/// A colour field as a swatch only (picture 74, Blender-like): no numbers; clicking it opens the
/// colour wheel beside it. Returns true when the colour changed.
pub fn swatch(ui: &mut Ui, key: &str, r: Rect, c: &mut [f32; 3]) -> bool {
    swatch_ex(ui, key, r, c, None)
}

/// The colour the wheel edits for a swatch: the gradient's base or tip while it's on, else the
/// plain colour.
fn edited(c: &[f32; 3], g: Option<&Gradient>, stop: u8) -> [f32; 3] {
    match g.filter(|g| g.on) {
        Some(g) if stop == 1 => g.c2,
        Some(g) => g.c1,
        None => *c,
    }
}

/// A swatch whose wheel also offers a gradient (a material's Color).
pub fn swatch_ex(ui: &mut Ui, key: &str, r: Rect, c: &mut [f32; 3], mut grad: Option<&mut Gradient>) -> bool {
    let mut changed = false;
    let open_here = OPEN.with(|o| o.borrow().as_ref().map_or(false, |o| o.wheel.key == key));
    if open_here {
        OPEN.with(|o| {
            let mut o = o.borrow_mut();
            let st = o.as_mut().unwrap();
            st.seen = true;
            st.swatch = r;
            if st.from_wheel {
                // the wheel's changes come back: the gradient as it now is, or the plain colour
                match (&mut grad, &st.grad) {
                    (Some(g), Some(sg)) => {
                        **g = sg.clone();
                        if !sg.on {
                            *c = st.value;
                        }
                    }
                    _ => *c = st.value,
                }
                st.from_wheel = false;
                changed = true;
            } else {
                st.grad = grad.as_ref().map(|g| (**g).clone());
                st.value = edited(c, st.grad.as_ref(), st.stop);
            }
            st.plain = *c;
        });
    }
    // the swatch: the colour, or the gradient it runs through
    match grad.as_ref().filter(|g| g.on) {
        Some(g) => {
            let (a, b) = if g.flip { (g.c2, g.c1) } else { (g.c1, g.c2) };
            let n = 24;
            for i in 0..n {
                let t = grad_t((i as f32 + 0.5) / n as f32, if g.flip { 1.0 - g.mid } else { g.mid });
                let col = Color::new(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, 1.0);
                draw_rectangle(r.x + r.w * i as f32 / n as f32, r.y, r.w / n as f32 + 0.5, r.h, col);
            }
        }
        None => draw_rectangle(r.x, r.y, r.w, r.h, Color::new(c[0], c[1], c[2], 1.0)),
    }
    let hot = r.contains(ui.mouse()) && !ui.blocked && ui.popup.is_none();
    draw_rectangle_lines(r.x, r.y, r.w, r.h, if open_here { 2.0 } else { 1.0 }, if open_here { C_ACCENT } else if hot { WHITE } else { C_LINE });
    ui.record(&format!("swatch:{}", key), r);
    if hot {
        ui.tooltip = Some(if open_here { "Click to close the colour wheel".into() } else { "Click to choose the colour".into() });
        if ui.inp.l_pressed {
            ui.inp.l_pressed = false;
            close();
            if !open_here {
                let g = grad.as_ref().map(|g| (**g).clone());
                let v = edited(c, g.as_ref(), 0);
                let at = vec2(r.x + r.w + 10.0, (r.y - 30.0).max(30.0));
                let z = Rect::new(0.0, 0.0, 0.0, 0.0);
                OPEN.with(|o| {
                    *o.borrow_mut() = Some(Open { wheel: Wheel::new(key, at, v), value: v, from_wheel: false, swatch: r, seen: true, grad: g, stop: 0, extra: z, title: z, drag: None, plain: *c });
                });
            }
        }
    }
    changed
}

/// Close the open swatch wheel (its colour goes to the recent colours).
pub fn close() {
    let v = OPEN.with(|o| o.borrow_mut().take().map(|o| o.value));
    if let Some(v) = v {
        CLOSED.with(|c| c.borrow_mut().push(v));
    }
}

/// Whether the open swatch wheel covers this spot (the panels behind it then get no clicks).
pub fn takes_input(x: f32, y: f32) -> bool {
    let p = vec2(x, y);
    OPEN.with(|o| o.borrow().as_ref().map_or(false, |o| o.wheel.rect.contains(p) || o.extra.contains(p) || o.title.contains(p) || o.drag.is_some()))
}

/// Draw the open swatch wheel over everything (once a frame, after the panels): a title strip to
/// move it by, the wheel, and under it Copy / Paste and, for a material's Color, the gradient.
/// `recent` gets the colours closed wheels ended on.
pub fn draw_open(ui: &mut Ui, recent: &mut Vec<[f32; 3]>) {
    for c in CLOSED.with(|c| std::mem::take(&mut *c.borrow_mut())) {
        recent.retain(|x| (x[0] - c[0]).abs() + (x[1] - c[1]).abs() + (x[2] - c[2]).abs() > 0.004);
        recent.insert(0, c);
        recent.truncate(12);
    }
    let Some(mut st) = OPEN.with(|o| o.borrow_mut().take()) else { return };
    // the swatch that opened it went away (another panel)
    if !st.seen {
        CLOSED.with(|c| c.borrow_mut().push(st.value));
        return;
    }
    st.seen = false;
    let blocked = ui.blocked;
    ui.blocked = false;
    let m = ui.mouse();
    // moving it by the title strip (picture 75)
    if let Some(off) = st.drag {
        if ui.inp.l_down {
            st.wheel.at = vec2((m.x - off.x).clamp(0.0, ui.sw - 60.0), (m.y - off.y).clamp(24.0, ui.sh - 60.0));
        } else {
            st.drag = None;
        }
    }
    let mut v = st.value;
    let wheel_changed = draw(ui, &mut st.wheel, &mut v, recent);
    let wr = st.wheel.rect;
    st.title = Rect::new(wr.x, wr.y - 22.0, wr.w, 22.0);
    let tr = st.title;
    draw_rectangle(tr.x, tr.y, tr.w, tr.h, C_PANEL2);
    draw_rectangle_lines(tr.x, tr.y, tr.w, tr.h, 1.0, C_LINE);
    ui.text("Colour", tr.x + 8.0, tr.y + 4.0, 12.0, C_TEXT);
    ui.text("drag to move", tr.x + tr.w - 108.0, tr.y + 5.0, 10.5, C_TEXT_FAINT);
    // X closes it (picture 77)
    let xr = Rect::new(tr.x + tr.w - 22.0, tr.y + 2.0, 18.0, 18.0);
    let x_hot = xr.contains(m);
    draw_round_rect(xr, 3.0, if x_hot { Color::new(0.85, 0.3, 0.3, 1.0) } else { Color::new(1.0, 1.0, 1.0, 0.06) });
    let xc = xr.center();
    draw_line(xc.x - 4.0, xc.y - 4.0, xc.x + 4.0, xc.y + 4.0, 1.5, WHITE);
    draw_line(xc.x - 4.0, xc.y + 4.0, xc.x + 4.0, xc.y - 4.0, 1.5, WHITE);
    let mut x_closed = false;
    if x_hot {
        ui.tooltip = Some("Close".into());
        if ui.inp.l_pressed {
            ui.inp.l_pressed = false;
            x_closed = true;
        }
    }
    if ui.inp.l_pressed && tr.contains(m) {
        st.drag = Some(m - st.wheel.at);
        ui.inp.l_pressed = false;
    }
    if wheel_changed {
        st.value = v;
        if let Some(g) = st.grad.as_mut().filter(|g| g.on) {
            if st.stop == 1 {
                g.c2 = v;
            } else {
                g.c1 = v;
            }
        }
        st.from_wheel = true;
    }
    // under the wheel: Copy / Paste, then the gradient for a material's Color
    let has_grad = st.grad.is_some();
    let grad_on = st.grad.as_ref().map_or(false, |g| g.on);
    let eh = if !has_grad { 36.0 } else if grad_on { 148.0 } else { 64.0 };
    let e = Rect::new(wr.x, (wr.y + wr.h + 4.0).min(ui.sh - eh - 4.0), wr.w, eh);
    st.extra = e;
    draw_rectangle(e.x + 3.0, e.y + 4.0, e.w, e.h, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_round_rect(e, 6.0, C_PANEL);
    draw_round_rect_lines(e, 6.0, 1.0, C_LINE);
    let bw = (e.w - 24.0) / 2.0;
    let what = if grad_on { "gradient" } else { "colour" };
    let cr = Rect::new(e.x + 8.0, e.y + 7.0, bw, 22.0);
    if ui.button("cw.copy", cr, &format!("Copy {}", what)) {
        let clip = match st.grad.as_ref().filter(|g| g.on) {
            Some(g) => Clip::Grad(g.clone()),
            None => Clip::Color(st.value),
        };
        CLIP.with(|c| *c.borrow_mut() = Some(clip));
    }
    let clip = CLIP.with(|c| c.borrow().clone());
    let pr = Rect::new(e.x + 16.0 + bw, e.y + 7.0, bw, 22.0);
    let paste_label = match &clip {
        Some(Clip::Grad(_)) if has_grad => "Paste gradient",
        Some(_) => "Paste colour",
        None => "Paste",
    };
    if ui.button("cw.paste", pr, paste_label) {
        match clip {
            Some(Clip::Grad(g)) if has_grad => {
                st.grad = Some(Gradient { on: true, ..g });
                st.stop = 0;
                st.value = st.grad.as_ref().unwrap().c1;
                st.from_wheel = true;
            }
            Some(Clip::Color(c)) | Some(Clip::Grad(Gradient { c1: c, .. })) => {
                st.value = c;
                if let Some(g) = st.grad.as_mut().filter(|g| g.on) {
                    if st.stop == 1 {
                        g.c2 = c;
                    } else {
                        g.c1 = c;
                    }
                }
                st.from_wheel = true;
            }
            None => {}
        }
    }
    if let Some(mut g) = st.grad.clone() {
        let before = g.clone();
        let y0 = e.y + 36.0;
        let cb = Rect::new(e.x + 10.0, y0 + 4.0, 16.0, 16.0);
        ui.checkbox("cw.grad", cb, &mut g.on);
        ui.text("Gradient along the branch", e.x + 32.0, y0 + 4.0, 12.5, C_TEXT);
        if g.on && !before.on {
            // turned on: it starts from the plain colour; the tip keeps what it had
            g.c1 = st.value;
            st.stop = 0;
        }
        if g.on {
            let bw2 = (e.w - 20.0 - 70.0) / 2.0;
            for (k, label) in ["Base", "Tip"].iter().enumerate() {
                let br = Rect::new(e.x + 10.0 + k as f32 * (bw2 + 4.0), y0 + 28.0, bw2, 22.0);
                if ui.button_ex(&format!("cw.stop{}", k), br, label, st.stop == k as u8, true) {
                    st.stop = k as u8;
                }
            }
            let fr = Rect::new(e.x + e.w - 66.0, y0 + 28.0, 56.0, 22.0);
            if ui.button_ex("cw.flip", fr, "Flip", g.flip, true) {
                g.flip = !g.flip;
            }
            // the gradient as it will run, base to tip
            let pr = Rect::new(e.x + 10.0, y0 + 58.0, e.w - 20.0, 18.0);
            let n = 32;
            for i in 0..n {
                let t = (i as f32 + 0.5) / n as f32;
                let t2 = grad_t(if g.flip { 1.0 - t } else { t }, g.mid);
                let col = Color::new(g.c1[0] + (g.c2[0] - g.c1[0]) * t2, g.c1[1] + (g.c2[1] - g.c1[1]) * t2, g.c1[2] + (g.c2[2] - g.c1[2]) * t2, 1.0);
                draw_rectangle(pr.x + pr.w * i as f32 / n as f32, pr.y, pr.w / n as f32 + 0.5, pr.h, col);
            }
            draw_rectangle_lines(pr.x, pr.y, pr.w, pr.h, 1.0, C_LINE);
            // the blend point (picture 77): where along the branch base and tip meet half way
            let mx = pr.x + pr.w * if g.flip { 1.0 - g.mid } else { g.mid };
            draw_triangle(vec2(mx, pr.y + pr.h + 1.0), vec2(mx - 5.0, pr.y + pr.h + 7.0), vec2(mx + 5.0, pr.y + pr.h + 7.0), WHITE);
            ui.text("Blend point", e.x + 10.0, y0 + 88.0, 11.5, C_TEXT_DIM);
            let sr = Rect::new(e.x + 86.0, y0 + 85.0, e.w - 96.0, 18.0);
            ui.slider("cw.mid", sr, &mut g.mid, 0.05, 0.95, 2);
            if Rect::new(e.x, y0 + 84.0, e.w, 20.0).contains(m) {
                ui.tooltip = Some("Where along the branch the base and tip colours meet half way: lower toward the base, higher toward the tip".into());
            }
        }
        if g != before {
            // the wheel now shows the end being edited (or the plain colour when turned off)
            // (picture 76: turned off, the plain colour from before comes back, not the base)
            st.value = if g.on { if st.stop == 1 { g.c2 } else { g.c1 } } else { st.plain };
            st.grad = Some(g);
            st.from_wheel = true;
        } else if let Some(g) = &st.grad {
            if g.on {
                let want = if st.stop == 1 { g.c2 } else { g.c1 };
                if !wheel_changed && want != st.value {
                    st.value = want;
                }
            }
        }
    }
    // a click outside the wheel, its strip, its panel and its swatch closes it
    // a wheel with a gradient (a material's Color) stays open until X (picture 77); a plain
    // colour's wheel also closes on a click outside it
    let outside = ui.inp.l_pressed && st.drag.is_none() && !st.wheel.rect.contains(m) && !st.extra.contains(m) && !st.title.contains(m) && !st.swatch.contains(m);
    let closing = x_closed || (outside && !has_grad);
    if st.wheel.rect.contains(m) || st.extra.contains(m) || st.title.contains(m) {
        ui.inp.l_pressed = false;
    }
    ui.blocked = blocked;
    if closing {
        CLOSED.with(|c| c.borrow_mut().push(st.value));
    } else {
        OPEN.with(|o| *o.borrow_mut() = Some(st));
    }
}
