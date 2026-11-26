//! The viewport's top-right corner (picture 27, Blender-like; the original has neither):
//! - a shading button (Studio / MatCap / Flat) whose drop-down shows the choices with a ball
//!   preview; Studio is the default and the ball only shows while the drop-down is open;
//! - a navigation gizmo: the axes as the camera sees them; clicking one looks along it (a second
//!   click looks from the other side). Its design and size are in Preferences > Viewport.

use crate::app::App;
use crate::prefs::GIZMO_STYLES;
use crate::render::{matcap_image, set_matcap, MATCAPS};
use crate::ui::*;
use macroquad::prelude::*;

pub const SHADINGS: [&str; 3] = ["Studio", "MatCap", "Flat"];

fn tex_from(px: &[u8], size: usize) -> Texture2D {
    let t = Texture2D::from_rgba8(size as u16, size as u16, px);
    t.set_filter(FilterMode::Linear);
    t
}

/// The view (Cam::set_view number) that looks along an axis: +X Right, -X Left, +Y Top,
/// -Y Bottom, +Z Front, -Z Back.
fn view_for(axis: usize, positive: bool) -> usize {
    match (axis, positive) {
        (0, true) => 4,
        (0, false) => 3,
        (1, true) => 5,
        (1, false) => 6,
        (2, true) => 1,
        _ => 2,
    }
}

/// A ball drawn the way a wire mode draws the tree (picture 40): 0 Scribed (the lit ball with thin
/// lines), 1 WireShaded (the lit ball with bold dark wires), 2 Wireframe (light wires only).
fn wire_ball(lit: &Texture2D, kind: usize, c: Vec2, rad: f32) {
    if kind < 2 {
        draw_texture_ex(lit, c.x - rad, c.y - rad, WHITE, DrawTextureParams { dest_size: Some(vec2(rad * 2.0, rad * 2.0)), ..Default::default() });
    } else {
        draw_circle(c.x, c.y, rad, Color::new(0.0, 0.0, 0.0, 0.25));
    }
    let (col, w) = match kind {
        0 => (Color::new(0.0, 0.0, 0.0, 0.35), (rad / 16.0).max(0.6)),
        1 => (Color::new(0.02, 0.02, 0.03, 0.85), (rad / 9.0).max(1.0)),
        _ => (Color::new(0.85, 0.9, 1.0, 0.9), (rad / 12.0).max(0.8)),
    };
    let seg = 24;
    // the outline, three meridians and two parallels, like a sphere's wireframe
    let ellipse = |rx: f32, ry: f32, oy: f32| {
        for i in 0..seg {
            let (a0, a1) = (i as f32 / seg as f32 * std::f32::consts::TAU, (i + 1) as f32 / seg as f32 * std::f32::consts::TAU);
            draw_line(c.x + a0.cos() * rx, c.y + oy + a0.sin() * ry, c.x + a1.cos() * rx, c.y + oy + a1.sin() * ry, w, col);
        }
    };
    ellipse(rad, rad, 0.0);
    ellipse(rad * 0.38, rad, 0.0);
    ellipse(rad * 0.78, rad, 0.0);
    draw_line(c.x, c.y - rad, c.x, c.y + rad, w, col);
    for lat in [-0.5f32, 0.0, 0.5] {
        let ry = rad * lat;
        let rx = (rad * rad - ry * ry).max(0.0).sqrt();
        ellipse(rx, rx * 0.22, ry);
    }
}

/// A light bulb icon (picture 42) centred at `c`, rays around it when `on`.
fn bulb_icon(c: Vec2, s: f32, col: Color, on: bool) {
    draw_circle(c.x, c.y - s * 0.15, s * 0.42, col);
    draw_rectangle(c.x - s * 0.2, c.y + s * 0.22, s * 0.4, s * 0.26, col);
    draw_line(c.x - s * 0.2, c.y + s * 0.34, c.x + s * 0.2, c.y + s * 0.34, 1.0, Color::new(0.0, 0.0, 0.0, 0.5));
    if on {
        for k in 0..8 {
            let a = k as f32 / 8.0 * std::f32::consts::TAU;
            let (dx, dy) = (a.cos(), a.sin());
            draw_line(c.x + dx * s * 0.58, c.y - s * 0.15 + dy * s * 0.58, c.x + dx * s * 0.78, c.y - s * 0.15 + dy * s * 0.78, 1.4, col);
        }
    }
}

/// A fan icon: three blades round a hub.
fn fan_icon(c: Vec2, s: f32, col: Color, spin: f32) {
    for k in 0..3 {
        let a = spin + k as f32 / 3.0 * std::f32::consts::TAU;
        let (dx, dy) = (a.cos(), a.sin());
        let tip = c + vec2(dx, dy) * s * 0.5;
        let side = vec2(-dy, dx) * s * 0.2;
        draw_triangle(c, tip + side, tip - side * 0.3, col);
        draw_circle(tip.x + side.x * 0.35, tip.y + side.y * 0.35, s * 0.15, col);
    }
    draw_circle(c.x, c.y, s * 0.13, Color::new(0.1, 0.1, 0.12, 1.0));
}

thread_local! {
    /// The person for scale (picture 45: the reference figure the person gave), made once.
    static HUMAN: std::cell::RefCell<Option<Texture2D>> = const { std::cell::RefCell::new(None) };
}

/// A person for scale beside the height indicator (picture 45: the person's own reference
/// figure, its background cut away), `tall` pixels high with the feet at `foot`.
fn person(foot: Vec2, tall: f32, col: Color) {
    let _ = col;
    let t = HUMAN.with(|h| {
        let mut h = h.borrow_mut();
        if h.is_none() {
            let t = Texture2D::from_file_with_format(include_bytes!("../assets/human.png"), Some(ImageFormat::Png));
            t.set_filter(FilterMode::Linear);
            *h = Some(t);
        }
        h.clone().unwrap()
    });
    let w = tall * t.width() / t.height().max(1.0);
    draw_texture_ex(&t, foot.x - w * 0.5, foot.y - tall, WHITE, DrawTextureParams { dest_size: Some(vec2(w, tall)), ..Default::default() });
}

impl App {
    /// The light controller and the fan under the navigation gizmo (picture 42, after the
    /// original's): click the bulb, then drag in the viewport to move the sun (click again to
    /// stop); the wheel over it changes the sun's intensity, shown under it; right-click opens
    /// Preferences > Lighting. The fan is for the wind system to come.
    pub fn draw_light_controls(&mut self, ui: &mut Ui, x: f32, y: f32) -> Rect {
        let w = 34.0;
        // Preferences > Tree Window > Show fan (picture 45)
        let show_fan = self.prefs.show_fan;
        let p = Rect::new(x - w * 0.5, y, w, if show_fan { 84.0 } else { 48.0 });
        draw_round_rect(p, 6.0, Color::new(0.15, 0.16, 0.18, 0.85));
        let m = ui.mouse();
        let bulb = Rect::new(p.x + 2.0, p.y + 2.0, w - 4.0, 44.0);
        let fan = Rect::new(p.x + 2.0, p.y + 48.0, w - 4.0, 34.0);
        let on = self.light_drag_mode;
        if on {
            draw_round_rect(bulb, 5.0, Color::new(0.30, 0.47, 0.78, 1.0));
        } else if bulb.contains(m) {
            draw_round_rect(bulb, 5.0, Color::new(1.0, 1.0, 1.0, 0.1));
        }
        let sc = self.light.sun_color;
        bulb_icon(vec2(bulb.x + bulb.w * 0.5, bulb.y + 15.0), 18.0, Color::new(0.55 + sc[0] * 0.45, 0.55 + sc[1] * 0.45, 0.4 + sc[2] * 0.4, 1.0), on);
        let t = format!("{:.2}", self.light.sun_intensity);
        ui.text_center(&t, Rect::new(bulb.x, bulb.y + 30.0, bulb.w, 12.0), 10.0, C_TEXT_DIM);
        ui.record("btn:vp.light", bulb);
        if bulb.contains(m) && !ui.blocked && ui.popup.is_none() {
            ui.tooltip = Some(if on { "Light: drag in the viewport to move the sun (click to stop); wheel: intensity; right-click: Lighting settings".into() } else { "Light: click, then drag in the viewport to move the sun; wheel: intensity; right-click: Lighting settings".into() });
            if ui.inp.l_pressed {
                self.light_drag_mode = !on;
                self.set_status(if !on { "Light controller: drag in the viewport to move the sun" } else { "Light controller off" });
                ui.inp.l_pressed = false;
            }
            if ui.inp.wheel != 0.0 {
                self.light.sun_intensity = (self.light.sun_intensity + ui.inp.wheel.signum() * 0.1).clamp(0.0, 10.0);
                ui.inp.wheel = 0.0;
            }
            if ui.inp.r_pressed {
                self.select(crate::app::Sel::Light);
                ui.inp.r_pressed = false;
            }
        }
        if !show_fan {
            return p;
        }
        if fan.contains(m) {
            draw_round_rect(fan, 5.0, Color::new(1.0, 1.0, 1.0, 0.1));
        }
        fan_icon(vec2(fan.x + fan.w * 0.5, fan.y + fan.h * 0.5), 22.0, C_TEXT_DIM, 0.4);
        ui.record("btn:vp.fan", fan);
        if fan.contains(m) && !ui.blocked && ui.popup.is_none() {
            ui.tooltip = Some("Wind controller: arrives with the wind system".into());
            if ui.inp.l_pressed {
                self.set_status("The wind controller arrives with the wind system");
                ui.inp.l_pressed = false;
            }
        }
        p
    }

    /// The height indicator's button under the light controls (picture 87): a small person, lit
    /// while the indicator shows; clicking it shows or hides the indicator, its scale and person.
    pub fn draw_height_button(&mut self, ui: &mut Ui, x: f32, y: f32) -> Rect {
        let w = 34.0;
        let p = Rect::new(x - w * 0.5, y, w, 48.0);
        draw_round_rect(p, 6.0, Color::new(0.15, 0.16, 0.18, 0.85));
        let b = Rect::new(p.x + 2.0, p.y + 2.0, w - 4.0, 44.0);
        let m = ui.mouse();
        let on = self.show_height;
        if on {
            draw_round_rect(b, 5.0, Color::new(0.30, 0.47, 0.78, 1.0));
        } else if b.contains(m) {
            draw_round_rect(b, 5.0, Color::new(1.0, 1.0, 1.0, 0.1));
        }
        person(vec2(b.x + b.w * 0.5, b.y + b.h - 3.0), b.h - 6.0, WHITE);
        if !on {
            // dimmed while hidden
            draw_round_rect(b, 5.0, Color::new(0.15, 0.16, 0.18, 0.55));
        }
        ui.record("btn:vp.height", b);
        if b.contains(m) && !ui.blocked && ui.popup.is_none() {
            ui.tooltip = Some(if on { "Height indicator: shown (click to hide)".into() } else { "Height indicator: hidden (click to show)".into() });
            if ui.inp.l_pressed {
                self.show_height = !on;
                self.set_status(if !on { "Height indicator shown" } else { "Height indicator hidden" });
                ui.inp.l_pressed = false;
            }
        }
        p
    }

    /// Show > Height indicator (pictures 42, 43; TreeCraft's own design): a soft, thin line
    /// standing beside the tree with a tick at every round step and the heights beside them, the
    /// full height on a small tag at the top, and a person 1.8 units tall at its foot for a sense of
    /// scale. Only once the tree has a generator.
    pub fn draw_height_indicator(&self, ui: &mut Ui, r: Rect, area: Rect) {
        if !self.show_height || self.opacity_view() || !self.nodes.iter().any(|n| n.kind != crate::model::Kind::Tree) || self.geo_stats.0 == 0 {
            return;
        }
        let (bmin, bmax) = self.bounds;
        let h = bmax.y.max(0.0);
        if h < 0.01 {
            return;
        }
        let m = self.cam.camera3d(None, r.w / r.h.max(1.0), None).matrix();
        let proj = |p: Vec3| -> Option<Vec2> {
            let c = m * p.extend(1.0);
            if c.w <= 1e-5 {
                return None;
            }
            let n = c.truncate() / c.w;
            Some(vec2(r.x + (n.x + 1.0) * 0.5 * r.w, r.y + (1.0 - n.y) * 0.5 * r.h))
        };
        let width = (bmax.x - bmin.x).max(bmax.z - bmin.z).max(h * 0.1);
        let right = Vec3::Y.cross(self.cam.dir()).normalize_or(Vec3::X);
        let centre = vec3((bmin.x + bmax.x) * 0.5, 0.0, (bmin.z + bmax.z) * 0.5);
        let foot = centre - right * (width * 0.5 + h * 0.08);
        let (Some(a), Some(b)) = (proj(foot), proj(foot + Vec3::Y * h)) else { return };
        if (b - a).length() < 30.0 {
            return;
        }
        ui.set_clip(Some(area));
        // Preferences > Tree Window > Size of height indicator (picture 45)
        let hs = self.prefs.height_size.clamp(0.5, 3.0);
        let line = Color::new(1.0, 1.0, 1.0, 0.55);
        let soft = Color::new(0.0, 0.0, 0.0, 0.18);
        draw_line(a.x + 1.0, a.y + 1.0, b.x + 1.0, b.y + 1.0, 2.0 * hs, soft);
        draw_line(a.x, a.y, b.x, b.y, 1.5 * hs, line);
        // a round step giving 4 to 10 ticks
        let raw = h / 6.0;
        let mag = 10f32.powf(raw.log10().floor());
        let step = if raw / mag < 1.5 { mag } else if raw / mag < 3.5 { 2.0 * mag } else { 5.0 * mag };
        let n = (h / step).floor() as usize;
        let dir = (b - a).normalize_or(vec2(0.0, -1.0));
        let side = vec2(-dir.y, dir.x);
        let side = if side.x < 0.0 { -side } else { side };
        for i in 0..=n {
            let hy = i as f32 * step;
            if let Some(q) = proj(foot + Vec3::Y * hy) {
                draw_line(q.x, q.y, q.x + side.x * 6.0 * hs, q.y + side.y * 6.0 * hs, 1.2 * hs, line);
                let label = if step >= 1.0 { format!("{}", hy.round() as i64) } else { format!("{:.1}", hy) };
                let p = q + side * 9.0 * hs - vec2(0.0, 5.5 * hs);
                ui.text(&label, p.x, p.y, 10.5 * hs, Color::new(1.0, 1.0, 1.0, 0.75));
            }
        }
        // the full height on a small tag at the top
        draw_circle(b.x, b.y, 2.5, line);
        let tag = format!("{:.2}", h);
        let tw = ui.text_w(&tag, 10.5 * hs) + 10.0 * hs;
        let tr = Rect::new(b.x - tw * 0.5, (b.y - 20.0 * hs).max(area.y + 2.0), tw, 15.0 * hs);
        draw_round_rect(tr, 4.0, Color::new(0.0, 0.0, 0.0, 0.35));
        ui.text_center(&tag, tr, 10.5 * hs, Color::new(1.0, 1.0, 1.0, 0.9));
        // a person for scale (1.8 units tall), standing just beside the line's foot
        let pf = foot - right * 0.7;
        if let (Some(f0), Some(f1)) = (proj(pf), proj(pf + Vec3::Y * 1.8)) {
            let tall = (f0 - f1).length();
            if tall > 6.0 {
                person(f0, tall, Color::new(0.16, 0.18, 0.2, 0.55));
            }
        }
        ui.set_clip(None);
    }

    /// The MatCap pictures (made once) and the chosen one handed to the shader.
    fn matcap_textures(&mut self) -> Vec<Texture2D> {
        if self.matcap_tex.is_empty() {
            self.matcap_tex = (0..MATCAPS.len()).map(|k| tex_from(&matcap_image(k, 128, Vec3::Y), 128)).collect();
            set_matcap(self.matcap_tex[self.matcap_idx.min(MATCAPS.len() - 1)].clone());
        }
        self.matcap_tex.clone()
    }

    /// Ball pictures for the shading buttons: Studio (lit by the current sun), Flat, then one per
    /// MatCap, each with clear corners.
    fn shading_balls(&mut self) -> Vec<Texture2D> {
        let key = ((self.light.azimuth * 10.0) as i32, (self.light.elevation * 10.0) as i32);
        if self.shade_balls.is_empty() || self.shade_balls_key != key {
            let mut v = vec![tex_from(&matcap_image(10, 64, self.light.dir()), 64), tex_from(&matcap_image(11, 64, Vec3::Y), 64)];
            v.extend((0..MATCAPS.len()).map(|k| tex_from(&matcap_image(20 + k, 64, Vec3::Y), 64)));
            self.shade_balls = v;
            self.shade_balls_key = key;
        }
        self.shade_balls.clone()
    }

    /// The shading choice as three balls (Studio, MatCap, Flat; hover one for a bigger preview with
    /// its description, picture 30) and an arrow for the drop-down, right-aligned at `y`. Returns
    /// the rectangles it covers and where it starts (for the Selection menu to its left).
    pub fn draw_shading_menu(&mut self, ui: &mut Ui, r: Rect, y: f32) -> (Vec<Rect>, f32) {
        let mats = self.matcap_textures();
        let balls = self.shading_balls();
        let cell = 24.0;
        let bw = cell * 3.0 + 22.0;
        let b = Rect::new(r.x + r.w - bw - 8.0, y, bw, 22.0);
        draw_round_rect(b, 4.0, C_PANEL2);
        let m = ui.mouse();
        let mut hover: Option<usize> = None;
        for k in 0..3usize {
            let cr = Rect::new(b.x + 2.0 + k as f32 * cell, b.y + 1.0, cell - 2.0, 20.0);
            let on = self.shading as usize == k;
            if on {
                draw_round_rect(cr, 4.0, Color::new(0.30, 0.47, 0.78, 1.0));
            } else if cr.contains(m) {
                draw_round_rect(cr, 4.0, Color::new(1.0, 1.0, 1.0, 0.1));
            }
            let t = match k {
                0 => &balls[0],
                1 => &balls[2 + self.matcap_idx.min(MATCAPS.len() - 1)],
                _ => &balls[1],
            };
            let s = 16.0;
            draw_texture_ex(t, cr.x + (cr.w - s) * 0.5, cr.y + 2.0, WHITE, DrawTextureParams { dest_size: Some(vec2(s, s)), ..Default::default() });
            ui.record(&format!("btn:vp.shading.{}", SHADINGS[k]), cr);
            if cr.contains(m) && !ui.blocked && ui.popup.is_none() {
                hover = Some(k);
                if ui.inp.l_pressed {
                    self.shading = k as u8;
                    ui.inp.l_pressed = false;
                }
            }
        }
        // the arrow: the drop-down with the choices and the MatCaps
        let ar = Rect::new(b.x + cell * 3.0 + 2.0, b.y + 1.0, 18.0, 20.0);
        if ar.contains(m) || self.shading_open {
            draw_round_rect(ar, 4.0, Color::new(1.0, 1.0, 1.0, if self.shading_open { 0.16 } else { 0.1 }));
        }
        draw_triangle(vec2(ar.x + 5.0, ar.y + 8.0), vec2(ar.x + 13.0, ar.y + 8.0), vec2(ar.x + 9.0, ar.y + 13.0), C_TEXT_DIM);
        ui.record("btn:vp.shading", ar);
        if ar.contains(m) && ui.inp.l_pressed && !ui.blocked && ui.popup.is_none() {
            self.shading_open = !self.shading_open;
            self.sel_open = false;
            ui.inp.l_pressed = false;
        }
        let mut covered = vec![b];
        // hovering a ball: a bigger ball with what it does
        if let (Some(k), false) = (hover, self.shading_open) {
            let pw = 250.0;
            let p = Rect::new(r.x + r.w - pw - 8.0, b.y + b.h + 4.0, pw, 84.0);
            draw_rectangle(p.x + 3.0, p.y + 4.0, p.w, p.h, Color::new(0.0, 0.0, 0.0, 0.35));
            draw_round_rect(p, 6.0, C_PANEL);
            draw_round_rect_lines(p, 6.0, 1.0, C_LINE);
            let t = match k {
                0 => &balls[0],
                1 => &balls[2 + self.matcap_idx.min(MATCAPS.len() - 1)],
                _ => &balls[1],
            };
            draw_texture_ex(t, p.x + 10.0, p.y + 10.0, WHITE, DrawTextureParams { dest_size: Some(vec2(64.0, 64.0)), ..Default::default() });
            let (title, text): (String, &str) = match k {
                0 => ("Studio".into(), "Lit by the sun and sky (Preferences > Lighting)."),
                1 => (format!("MatCap: {}", MATCAPS[self.matcap_idx.min(MATCAPS.len() - 1)]), "A lit-sphere look that ignores the lights; the arrow picks another."),
                _ => ("Flat".into(), "No lighting: every surface shows its texture's own colours."),
            };
            ui.text(&title, p.x + 84.0, p.y + 12.0, 13.0, WHITE);
            // the description over up to three lines
            let mut line = String::new();
            let mut yy = p.y + 32.0;
            for word in text.split(' ') {
                let test = if line.is_empty() { word.to_string() } else { format!("{} {}", line, word) };
                if ui.text_w(&test, 11.5) > p.w - 94.0 && !line.is_empty() {
                    ui.text(&line, p.x + 84.0, yy, 11.5, C_TEXT_DIM);
                    yy += 15.0;
                    line = word.to_string();
                } else {
                    line = test;
                }
            }
            ui.text(&line, p.x + 84.0, yy, 11.5, C_TEXT_DIM);
            covered.push(p);
        }
        if !self.shading_open {
            return (covered, b.x);
        }
        let pw = 268.0;
        // (picture 75: the Texel density choices sit under the lighting ones)
        let p = Rect::new(r.x + r.w - pw - 8.0, b.y + b.h + 4.0, pw, 136.0 + crate::texel::MENU_H);
        draw_rectangle(p.x + 3.0, p.y + 4.0, p.w, p.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(p, 6.0, C_PANEL);
        draw_round_rect_lines(p, 6.0, 1.0, C_LINE);
        ui.text("Lighting", p.x + 10.0, p.y + 7.0, 12.0, C_TEXT_DIM);
        let tw = (p.w - 20.0) / 3.0;
        for (k, name) in SHADINGS.iter().enumerate() {
            let tr = Rect::new(p.x + 10.0 + k as f32 * tw, p.y + 26.0, tw - 2.0, 22.0);
            if ui.button_ex(&format!("vp.shading.{}", name), tr, name, self.shading as usize == k, true) {
                self.shading = k as u8;
            }
        }
        let body = Rect::new(p.x + 10.0, p.y + 56.0, p.w - 20.0, 136.0 - 64.0);
        match self.shading {
            1 => {
                // pick a MatCap
                let n = mats.len();
                let size = ((body.w - (n as f32 - 1.0) * 6.0) / n as f32).min(body.h - 18.0);
                for (k, t) in mats.iter().enumerate() {
                    let br = Rect::new(body.x + k as f32 * (size + 6.0), body.y, size, size);
                    draw_texture_ex(&balls[2 + k], br.x, br.y, WHITE, DrawTextureParams { dest_size: Some(vec2(size, size)), ..Default::default() });
                    let on = self.matcap_idx == k;
                    if on || br.contains(ui.mouse()) {
                        draw_circle_lines(br.x + size * 0.5, br.y + size * 0.5, size * 0.5 + 1.0, if on { 2.0 } else { 1.0 }, if on { C_ACCENT } else { WHITE });
                    }
                    ui.record(&format!("matcap:{}", MATCAPS[k]), br);
                    if ui.clicked(br) {
                        self.matcap_idx = k;
                        set_matcap(t.clone());
                    }
                }
                ui.text_fit(MATCAPS[self.matcap_idx.min(n - 1)], body.x, body.y + size + 4.0, body.w * 0.4, 11.5, C_TEXT_DIM);
                // show the textures' colours under the MatCap (off by default, picture 29)
                let cb = Rect::new(body.x + body.w * 0.45, body.y + size + 3.0, 16.0, 16.0);
                ui.checkbox("vp.matcap.tex", cb, &mut self.matcap_texture);
                ui.text("Texture", cb.x + 22.0, cb.y + 1.0, 11.5, C_TEXT_DIM);
                ui.record("matcap.texture", cb);
            }
            k => {
                // a ball showing the look: lit by the sun (Studio) or uncoloured flat
                let kind = if k == 2 { 11 } else { 10 };
                let size = body.h.min(80.0);
                let key = (kind, (self.light.azimuth * 10.0) as i32, (self.light.elevation * 10.0) as i32);
                if self.ball_key != key || self.ball_tex.is_none() {
                    self.ball_tex = Some(tex_from(&matcap_image(kind as usize, 96, self.light.dir()), 96));
                    self.ball_key = key;
                }
                if let Some(t) = &self.ball_tex {
                    draw_texture_ex(t, body.x, body.y, WHITE, DrawTextureParams { dest_size: Some(vec2(size, size)), ..Default::default() });
                }
                let text = if k == 2 { "No lighting: every surface shows its texture's own colours." } else { "Lit by the sun and sky (Preferences > Lighting)." };
                let mut yy = body.y + 4.0;
                for line in text.split(": ") {
                    ui.text_fit(line, body.x + size + 10.0, yy, body.w - size - 10.0, 11.5, C_TEXT_DIM);
                    yy += 16.0;
                }
            }
        }
        crate::texel::menu(ui, Rect::new(p.x + 10.0, p.y + 136.0 - 4.0, p.w - 20.0, crate::texel::MENU_H));
        covered.push(p);
        // a click anywhere else closes it
        if ui.inp.l_pressed && !p.contains(ui.mouse()) && !b.contains(ui.mouse()) {
            self.shading_open = false;
        }
        (covered, b.x)
    }

    /// The tips (picture 43, moved out of the Properties panel): a round "?" at the viewport's
    /// bottom left opens a card with the controls grouped by where they're used (Help > Controls
    /// opens it too). A click outside, the "?" again or Esc closes it. Returns what it covers.
    pub fn draw_help_card(&mut self, ui: &mut Ui, r: Rect) -> Vec<Rect> {
        let m = ui.mouse();
        let c = vec2(r.x + 20.0, r.y + r.h - 44.0);
        let br = Rect::new(c.x - 11.0, c.y - 11.0, 22.0, 22.0);
        let hot = br.contains(m);
        draw_circle(c.x, c.y, 11.0, if self.help_open { Color::new(0.30, 0.47, 0.78, 1.0) } else if hot { Color::new(0.25, 0.27, 0.3, 0.95) } else { Color::new(0.15, 0.16, 0.18, 0.85) });
        draw_circle_lines(c.x, c.y, 11.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.35));
        ui.text_center("?", br, 13.0, WHITE);
        ui.record("btn:vp.help", br);
        let mut covered = vec![br];
        if hot && !ui.blocked && ui.popup.is_none() {
            ui.tooltip = Some("Tips: controls and shortcuts".into());
            if ui.inp.l_pressed {
                self.help_open = !self.help_open;
                ui.inp.l_pressed = false;
            }
        }
        if !self.help_open {
            return covered;
        }
        let sections: [(&str, &[(&str, &str)]); 5] = [
            ("Viewport", &[("Left-drag", "orbit"), ("Middle / Shift + drag", "pan"), ("Wheel, right-drag", "zoom"), ("F", "frame the tree"), ("Click / double-click", "pick / centre on it"), ("Numpad 1 3 7, 5", "front, right, top; ortho")]),
            ("Generation editor", &[("+ Add", "add under the selected node"), ("Click a node", "edit it in Properties"), ("Shift + drag onto a node", "re-link it"), ("Delete", "remove it"), ("Tab", "Generator / Node mode")]),
            ("Show and hide", &[("1, 3", "leaves, branches"), ("H, I", "hide, isolate"), ("Z", "zoom to the selection")]),
            ("Light", &[("Hold V + drag", "turn the sun around"), ("Bulb, then drag", "move the sun"), ("Wheel on the bulb", "brightness"), ("Right-click the bulb", "light settings")]),
            ("Files", &[("Ctrl+N, O, S", "new, open, save"), ("Ctrl+Shift+S", "save as"), ("Ctrl+E", "export mesh"), ("Ctrl+Z, Ctrl+Y", "undo, redo")]),
        ];
        let col_w: f32 = 330.0;
        let lines_left: usize = sections[..2].iter().map(|s| s.1.len() + 2).sum();
        let lines_right: usize = sections[2..].iter().map(|s| s.1.len() + 2).sum();
        let h = lines_left.max(lines_right) as f32 * 17.0 + 40.0;
        let card = Rect::new(r.x + 8.0, (c.y - 18.0 - h).max(r.y + 40.0), (col_w * 2.0 + 30.0).min(r.w - 16.0), h);
        draw_rectangle(card.x + 3.0, card.y + 4.0, card.w, card.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(card, 8.0, Color::new(0.13, 0.14, 0.16, 0.96));
        draw_round_rect_lines(card, 8.0, 1.0, C_LINE);
        ui.text("Tips", card.x + 14.0, card.y + 10.0, 14.0, WHITE);
        let xb = Rect::new(card.x + card.w - 26.0, card.y + 8.0, 18.0, 18.0);
        ui.text_center("x", xb, 13.0, if xb.contains(m) { WHITE } else { C_TEXT_DIM });
        for (col, range) in [(0usize, 0..2), (1, 2..5)] {
            let x = card.x + 14.0 + col as f32 * (col_w + 10.0);
            let mut y = card.y + 34.0;
            for (title, items) in &sections[range] {
                ui.text(title, x, y, 12.5, Color::new(0.55, 0.75, 1.0, 1.0));
                y += 19.0;
                for (k, v) in items.iter() {
                    ui.text_fit(k, x + 6.0, y, 158.0, 11.5, C_TEXT);
                    ui.text_fit(v, x + 168.0, y, col_w - 174.0, 11.5, C_TEXT_DIM);
                    y += 17.0;
                }
                y += 15.0;
            }
        }
        covered.push(card);
        let close = (ui.inp.l_pressed && !card.contains(m) && !br.contains(m)) || (ui.inp.l_pressed && xb.contains(m)) || ui.inp.key(KeyCode::Escape);
        if close {
            self.help_open = false;
        }
        if card.contains(m) {
            ui.inp.l_pressed = false;
        }
        covered
    }

    /// Render > Scribed, WireShaded and Wireframe as three balls (picture 40), left of `right`:
    /// each ball drawn the way that mode draws the tree; a click turns the mode on, a second click
    /// goes back to Standard; hovering shows a bigger ball with what it does. They stay in the
    /// Render menu too. Returns what it covers and where it starts.
    pub fn draw_wire_balls(&mut self, ui: &mut Ui, r: Rect, y: f32, right: f32) -> (Vec<Rect>, f32) {
        let _ = r;
        let balls = self.shading_balls();
        let modes = [(1.0f32, "Scribed", "The shaded tree with every triangle's edge drawn thinly over it."), (crate::app::MODE_WIRESHADED, "WireShaded", "The shaded tree with its wires on top (thickness and colour: Preferences > Viewport)."), (crate::app::MODE_WIREFRAME, "Wireframe", "Only the wires, see-through.")];
        let cell = 24.0;
        let bw = cell * 3.0 + 4.0;
        let b = Rect::new(right - bw - 6.0, y, bw, 22.0);
        draw_round_rect(b, 4.0, C_PANEL2);
        let m = ui.mouse();
        let mut hover = None;
        for (k, (mode, name, _)) in modes.iter().enumerate() {
            let cr = Rect::new(b.x + 2.0 + k as f32 * cell, b.y + 1.0, cell - 2.0, 20.0);
            let on = self.light.mode == *mode;
            if on {
                draw_round_rect(cr, 4.0, Color::new(0.30, 0.47, 0.78, 1.0));
            } else if cr.contains(m) {
                draw_round_rect(cr, 4.0, Color::new(1.0, 1.0, 1.0, 0.1));
            }
            wire_ball(&balls[0], k, vec2(cr.x + cr.w * 0.5, cr.y + 10.0), 8.0);
            ui.record(&format!("btn:vp.render.{}", name), cr);
            if cr.contains(m) && !ui.blocked && ui.popup.is_none() {
                hover = Some(k);
                if ui.inp.l_pressed {
                    self.light.mode = if on { 0.0 } else { *mode };
                    ui.inp.l_pressed = false;
                }
            }
        }
        let mut covered = vec![b];
        if let (Some(k), false) = (hover, self.shading_open || self.sel_open) {
            let pw = 250.0;
            let p = Rect::new((b.x + b.w - pw).max(4.0), b.y + b.h + 4.0, pw, 84.0);
            draw_rectangle(p.x + 3.0, p.y + 4.0, p.w, p.h, Color::new(0.0, 0.0, 0.0, 0.35));
            draw_round_rect(p, 6.0, C_PANEL);
            draw_round_rect_lines(p, 6.0, 1.0, C_LINE);
            wire_ball(&balls[0], k, vec2(p.x + 42.0, p.y + 42.0), 32.0);
            ui.text(modes[k].1, p.x + 84.0, p.y + 12.0, 13.0, WHITE);
            let mut line = String::new();
            let mut yy = p.y + 32.0;
            for word in modes[k].2.split(' ') {
                let test = if line.is_empty() { word.to_string() } else { format!("{} {}", line, word) };
                if ui.text_w(&test, 11.5) > p.w - 94.0 && !line.is_empty() {
                    ui.text(&line, p.x + 84.0, yy, 11.5, C_TEXT_DIM);
                    yy += 15.0;
                    line = word.to_string();
                } else {
                    line = test;
                }
            }
            ui.text(&line, p.x + 84.0, yy, 11.5, C_TEXT_DIM);
            covered.push(p);
        }
        (covered, b.x)
    }

    /// The Selection menu (picture 30): how the selected generator is shown — Solid fill (colour
    /// and transparency), Outline (colour and thickness) or Both — as a button left of `right`.
    pub fn draw_selection_menu(&mut self, ui: &mut Ui, r: Rect, y: f32, right: f32) -> Vec<Rect> {
        use crate::prefs::SEL_MODES;
        // picture 40: the three ways at one click (Solid fill, Outline, Both), then a colour chip
        // opening the colours and amounts
        let cell = 24.0;
        let bw = cell * 3.0 + 30.0;
        let b = Rect::new(right - bw - 6.0, y, bw, 22.0);
        draw_round_rect(b, 4.0, C_PANEL2);
        let m = ui.mouse();
        let (fill, outl) = (self.prefs.sel_fill, self.prefs.sel_outline);
        let (fc, oc) = (Color::new(fill[0], fill[1], fill[2], 1.0), Color::new(outl[0], outl[1], outl[2], 1.0));
        for k in 0..3usize {
            let cr = Rect::new(b.x + 2.0 + k as f32 * cell, b.y + 1.0, cell - 2.0, 20.0);
            let on = self.prefs.sel_mode == k;
            if on {
                draw_round_rect(cr, 4.0, Color::new(0.30, 0.47, 0.78, 1.0));
            } else if cr.contains(m) {
                draw_round_rect(cr, 4.0, Color::new(1.0, 1.0, 1.0, 0.1));
            }
            let (cx, cy) = (cr.x + cr.w * 0.5, cr.y + cr.h * 0.5);
            match k {
                0 => draw_circle(cx, cy, 6.0, fc),
                1 => draw_circle_lines(cx, cy, 6.0, 2.0, oc),
                _ => {
                    draw_circle(cx, cy, 6.0, fc);
                    draw_circle_lines(cx, cy, 6.5, 2.0, oc);
                }
            }
            ui.record(&format!("btn:vp.sel.{}", SEL_MODES[k]), cr);
            if cr.contains(m) && !ui.blocked && ui.popup.is_none() {
                ui.tooltip = Some(format!("Show the selected generator with: {}", SEL_MODES[k]));
                if ui.inp.l_pressed {
                    self.prefs.sel_mode = k;
                    self.prefs.save();
                    ui.inp.l_pressed = false;
                }
            }
        }
        // the colour chip: fill colour over outline colour, with an arrow
        let chip = Rect::new(b.x + cell * 3.0 + 2.0, b.y + 1.0, 26.0, 20.0);
        if chip.contains(m) || self.sel_open {
            draw_round_rect(chip, 4.0, Color::new(1.0, 1.0, 1.0, if self.sel_open { 0.16 } else { 0.1 }));
        }
        draw_rectangle(chip.x + 3.0, chip.y + 4.0, 8.0, 6.0, fc);
        draw_rectangle(chip.x + 3.0, chip.y + 10.0, 8.0, 6.0, oc);
        draw_rectangle_lines(chip.x + 3.0, chip.y + 4.0, 8.0, 12.0, 1.0, C_LINE);
        draw_triangle(vec2(chip.x + 14.0, chip.y + 8.0), vec2(chip.x + 22.0, chip.y + 8.0), vec2(chip.x + 18.0, chip.y + 13.0), C_TEXT_DIM);
        ui.record("btn:vp.selection", chip);
        if chip.contains(m) && !ui.blocked && ui.popup.is_none() {
            ui.tooltip = Some("Selection colours, transparency and outline thickness".into());
            if ui.inp.l_pressed {
                self.sel_open = !self.sel_open;
                self.shading_open = false;
                ui.inp.l_pressed = false;
            }
        }
        let mut covered = vec![b];
        if !self.sel_open {
            if self.wheel.is_some() {
                self.close_wheel();
            }
            return covered;
        }
        let pw = 268.0;
        let p = Rect::new((b.x + b.w - pw).max(r.x + 4.0), b.y + b.h + 4.0, pw, 164.0);
        draw_rectangle(p.x + 3.0, p.y + 4.0, p.w, p.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(p, 6.0, C_PANEL);
        draw_round_rect_lines(p, 6.0, 1.0, C_LINE);
        ui.text("Selection", p.x + 10.0, p.y + 7.0, 12.0, C_TEXT_DIM);
        let before = (self.prefs.sel_mode, self.prefs.sel_fill, self.prefs.sel_outline, self.prefs.sel_thickness);
        let tw = (p.w - 20.0) / 3.0;
        for (k, name) in SEL_MODES.iter().enumerate() {
            let tr = Rect::new(p.x + 10.0 + k as f32 * tw, p.y + 26.0, tw - 2.0, 22.0);
            if ui.button_ex(&format!("vp.sel.{}", name), tr, name, self.prefs.sel_mode == k, true) {
                self.prefs.sel_mode = k;
            }
        }
        // a colour: a swatch that opens the colour wheel (picture 31), and one more slider
        let wheel_key = self.wheel.as_ref().map(|w| w.key.clone());
        let color_row = |ui: &mut Ui, key: &str, title: &str, y0: f32, c: [f32; 4], last: (&str, f32, f32)| -> (f32, Option<Rect>) {
            ui.text(title, p.x + 10.0, y0 + 2.0, 12.0, C_TEXT);
            let sw = Rect::new(p.x + 10.0, y0 + 20.0, 64.0, 22.0);
            draw_rectangle(sw.x, sw.y, sw.w, sw.h, Color::new(c[0], c[1], c[2], 1.0));
            let open = wheel_key.as_deref() == Some(key);
            draw_rectangle_lines(sw.x, sw.y, sw.w, sw.h, if open { 2.0 } else { 1.0 }, if open { C_ACCENT } else if sw.contains(ui.mouse()) { WHITE } else { C_LINE });
            ui.record(&format!("swatch:{}", key), sw);
            let clicked = ui.clicked(sw).then_some(sw);
            if sw.contains(ui.mouse()) {
                ui.tooltip = Some("Click to choose the colour".into());
            }
            let sx = sw.x + sw.w + 10.0;
            ui.text(last.0, sx, y0 + 24.0, 11.5, C_TEXT_DIM);
            let mut v = if last.0 == "Transparency" { 1.0 - c[3] } else { last.1 };
            let rr = Rect::new(sx + 82.0, y0 + 21.0, p.x + p.w - 10.0 - (sx + 82.0), 18.0);
            ui.slider(&format!("{}.last", key), rr, &mut v, 0.0, last.2, 2);
            (v, clicked)
        };
        let fill_on = self.prefs.sel_mode != 1;
        let outline_on = self.prefs.sel_mode != 0;
        let fill = self.prefs.sel_fill;
        let (tr, fill_click) = color_row(ui, "sel.fill", if fill_on { "Solid fill" } else { "Solid fill (off)" }, p.y + 56.0, fill, ("Transparency", 0.0, 1.0));
        self.prefs.sel_fill[3] = 1.0 - tr;
        let oc = self.prefs.sel_outline;
        let (th, outline_click) = color_row(ui, "sel.outline", if outline_on { "Outline" } else { "Outline (off)" }, p.y + 110.0, oc, ("Thickness (px)", self.prefs.sel_thickness, 12.0));
        self.prefs.sel_thickness = th.max(1.0);
        // a swatch click opens (or closes) the wheel for that colour
        for (key, click, c) in [("sel.fill", fill_click, fill), ("sel.outline", outline_click, oc)] {
            if let Some(sw) = click {
                if self.wheel.as_ref().map_or(false, |w| w.key == key) {
                    self.close_wheel();
                } else {
                    self.close_wheel();
                    self.wheel = Some(crate::colorwheel::Wheel::new(key, vec2(p.x - 242.0, sw.y - 20.0), [c[0], c[1], c[2]]));
                }
            }
        }
        // the wheel, over everything else in the corner
        let mut wheel_rect = None;
        if let Some(mut w) = self.wheel.take() {
            let mut c3 = match w.key.as_str() {
                "sel.fill" => [self.prefs.sel_fill[0], self.prefs.sel_fill[1], self.prefs.sel_fill[2]],
                _ => [self.prefs.sel_outline[0], self.prefs.sel_outline[1], self.prefs.sel_outline[2]],
            };
            let recent = self.prefs.recent_colors.clone();
            if crate::colorwheel::draw(ui, &mut w, &mut c3, &recent) {
                let target = if w.key == "sel.fill" { &mut self.prefs.sel_fill } else { &mut self.prefs.sel_outline };
                target[0] = c3[0];
                target[1] = c3[1];
                target[2] = c3[2];
            }
            wheel_rect = Some(w.rect);
            covered.push(w.rect);
            // a click outside the wheel and its swatches closes it
            let on_swatch = fill_click.is_some() || outline_click.is_some();
            let closing = ui.inp.l_pressed && !w.rect.contains(ui.mouse()) && !on_swatch;
            self.wheel = Some(w);
            if closing {
                self.close_wheel();
            }
        }
        if before != (self.prefs.sel_mode, self.prefs.sel_fill, self.prefs.sel_outline, self.prefs.sel_thickness) && !ui.inp.l_down {
            self.prefs.save();
        } else if before != (self.prefs.sel_mode, self.prefs.sel_fill, self.prefs.sel_outline, self.prefs.sel_thickness) {
            self.sel_dirty = true;
        }
        if self.sel_dirty && !ui.inp.l_down {
            self.prefs.save();
            self.sel_dirty = false;
        }
        covered.push(p);
        if ui.inp.l_pressed && !p.contains(ui.mouse()) && !b.contains(ui.mouse()) && !wheel_rect.map_or(false, |r| r.contains(ui.mouse())) {
            self.sel_open = false;
            self.close_wheel();
        }
        covered
    }

    /// Close the colour wheel, remembering its colour among the colours used last.
    pub fn close_wheel(&mut self) {
        if let Some(w) = self.wheel.take() {
            let c = match w.key.as_str() {
                "sel.fill" => [self.prefs.sel_fill[0], self.prefs.sel_fill[1], self.prefs.sel_fill[2]],
                _ => [self.prefs.sel_outline[0], self.prefs.sel_outline[1], self.prefs.sel_outline[2]],
            };
            self.prefs.recent_colors.retain(|x| (x[0] - c[0]).abs() + (x[1] - c[1]).abs() + (x[2] - c[2]).abs() > 0.004);
            self.prefs.recent_colors.insert(0, c);
            self.prefs.recent_colors.truncate(12);
            self.prefs.save();
        }
    }

    /// Selection > Outline, second pass: rings the selected generator's shape over the viewport
    /// (`r` in UI units, `f` macroquad units per UI unit).
    pub fn draw_selection_outline(&self, r: Rect, f: f32) {
        if !self.sel_drawn || self.prefs.sel_mode == 0 {
            return;
        }
        let (Some(om), Some(rt)) = (&self.outline_mat, &self.sel_rt) else { return };
        let (w, h) = (rt.texture.width(), rt.texture.height());
        // Node mode (picture 67): the generator is outlined in orange instead
        let c = if self.edit_mode == 1 { [1.0, 0.55, 0.12, 1.0] } else { self.prefs.sel_outline };
        gl_use_material(om);
        om.set_uniform("OutColor", vec4(c[0], c[1], c[2], c[3]));
        om.set_uniform("Texel", vec2(1.0 / w.max(1.0), 1.0 / h.max(1.0)));
        om.set_uniform("Thick", self.prefs.sel_thickness * w / r.w.max(1.0));
        draw_texture_ex(&rt.texture, r.x * f, r.y * f, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w * f, r.h * f)), flip_y: true, ..Default::default() });
        gl_use_default_material();
    }

    /// The navigation gizmo with its centre at `c`. Returns the rectangle it covers.
    pub fn draw_nav_gizmo(&mut self, ui: &mut Ui, c: Vec2) -> Option<Rect> {
        let style = self.prefs.gizmo_style.min(GIZMO_STYLES.len() - 1);
        if style == 4 {
            return None;
        }
        let k = self.prefs.gizmo_size.clamp(0.6, 2.0);
        let rad = 40.0 * k;
        let area = Rect::new(c.x - rad - 12.0 * k, c.y - rad - 12.0 * k, (rad + 12.0 * k) * 2.0, (rad + 12.0 * k) * 2.0);
        let d = self.cam.dir();
        let up_ref = if d.y.abs() > 0.99 { vec3(-self.cam.yaw.sin(), 0.0, -self.cam.yaw.cos()) * d.y.signum() } else { Vec3::Y };
        let right = up_ref.cross(d).normalize_or_zero();
        let up = d.cross(right).normalize_or_zero();
        let proj = |v: Vec3| vec2(v.dot(right), -v.dot(up));
        let cols = [self.prefs.color("axis_x"), self.prefs.color("axis_y"), self.prefs.color("axis_z")];
        let names = ["X", "Y", "Z"];
        let m = ui.mouse();
        let over = area.contains(m) && !ui.blocked && ui.popup.is_none();
        if over {
            draw_circle(c.x, c.y, rad + 10.0 * k, Color::new(1.0, 1.0, 1.0, 0.06));
        }
        // (screen point, depth, axis, positive) for each end, far ones first
        let mut ends: Vec<(Vec2, f32, usize, bool)> = vec![];
        for a in 0..3 {
            for pos in [true, false] {
                let mut v = Vec3::ZERO;
                v[a] = if pos { 1.0 } else { -1.0 };
                ends.push((c + proj(v) * rad, v.dot(d), a, pos));
            }
        }
        ends.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut hit: Option<(usize, bool)> = None;
        match style {
            2 => {
                // a view cube: the faces turned towards the camera, labelled
                let labels = [["Right", "Left"], ["Top", "Bottom"], ["Front", "Back"]];
                let s = rad * 0.62;
                let mut faces: Vec<(f32, usize, bool)> = vec![];
                for a in 0..3 {
                    for pos in [true, false] {
                        let mut nrm = Vec3::ZERO;
                        nrm[a] = if pos { 1.0 } else { -1.0 };
                        let depth = nrm.dot(d);
                        if depth > 0.02 {
                            faces.push((depth, a, pos));
                        }
                    }
                }
                faces.sort_by(|a, b| a.0.total_cmp(&b.0));
                for (depth, a, pos) in faces {
                    let mut nrm = Vec3::ZERO;
                    nrm[a] = if pos { 1.0 } else { -1.0 };
                    let (u, v) = ([Vec3::Y, Vec3::Z, Vec3::X][a], [Vec3::Z, Vec3::X, Vec3::Y][a]);
                    let q: Vec<Vec2> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].iter().map(|(x, y)| c + proj(nrm + u * *x + v * *y) * s).collect();
                    let inside = (0..4).all(|i| {
                        let (p0, p1) = (q[i], q[(i + 1) % 4]);
                        let cr = (p1 - p0).perp_dot(m - p0);
                        cr >= 0.0
                    }) || (0..4).all(|i| {
                        let (p0, p1) = (q[i], q[(i + 1) % 4]);
                        (p1 - p0).perp_dot(m - p0) <= 0.0
                    });
                    let hot = over && inside;
                    if hot {
                        hit = Some((a, pos));
                    }
                    let base = cols[a];
                    let shade = 0.35 + 0.45 * depth;
                    let fill = if hot { Color::new(0.95, 0.95, 0.95, 0.95) } else { Color::new(base.r * shade + 0.15, base.g * shade + 0.15, base.b * shade + 0.15, 0.92) };
                    draw_triangle(q[0], q[1], q[2], fill);
                    draw_triangle(q[0], q[2], q[3], fill);
                    for i in 0..4 {
                        draw_line(q[i].x, q[i].y, q[(i + 1) % 4].x, q[(i + 1) % 4].y, 1.0, Color::new(0.0, 0.0, 0.0, 0.6));
                    }
                    if depth > 0.3 {
                        let ctr = c + proj(nrm) * s;
                        let l = labels[a][if pos { 0 } else { 1 }];
                        let fs = 10.5 * k;
                        let tw = ui.text_w(l, fs);
                        ui.text(l, ctr.x - tw * 0.5, ctr.y - fs * 0.6, fs, if hot { BLACK } else { WHITE });
                    }
                }
            }
            _ => {
                for (p, depth, a, pos) in &ends {
                    let col = cols[*a];
                    let r_ball = match style {
                        3 => 5.0 * k,
                        _ => 9.0 * k,
                    };
                    let hot = over && p.distance(m) <= r_ball + 2.0;
                    if hot {
                        hit = Some((*a, *pos));
                    }
                    let fade = 0.55 + 0.45 * ((depth + 1.0) * 0.5);
                    if *pos {
                        // the line from the centre (Blender-like and Arrows)
                        if style != 3 {
                            draw_line(c.x, c.y, p.x, p.y, 2.0 * k, Color::new(col.r, col.g, col.b, fade));
                        }
                        match style {
                            1 => {
                                // an arrow head and the letter beyond it
                                let dir = (*p - c).normalize_or_zero();
                                let side = vec2(-dir.y, dir.x);
                                draw_triangle(*p + dir * 7.0 * k, *p - dir * 3.0 * k + side * 5.0 * k, *p - dir * 3.0 * k - side * 5.0 * k, col);
                                let lp = *p + dir * 14.0 * k;
                                ui.text(names[*a], lp.x - 4.0 * k, lp.y - 7.0 * k, 12.0 * k, if hot { WHITE } else { col });
                            }
                            _ => {
                                draw_circle(p.x, p.y, r_ball, Color::new(col.r, col.g, col.b, fade));
                                if style == 0 {
                                    let fs = 11.0 * k;
                                    let tw = ui.text_w(names[*a], fs);
                                    ui.text(names[*a], p.x - tw * 0.5, p.y - fs * 0.62, fs, Color::new(0.08, 0.08, 0.08, 1.0));
                                }
                            }
                        }
                    } else if style == 1 {
                        // the negative ends: short faint ticks
                        let dir = (*p - c).normalize_or_zero();
                        draw_line(c.x, c.y, c.x + dir.x * rad * 0.35, c.y + dir.y * rad * 0.35, 1.5 * k, Color::new(col.r, col.g, col.b, 0.4));
                        draw_circle(p.x, p.y, 3.0 * k, Color::new(col.r, col.g, col.b, 0.35));
                    } else {
                        // the negative ends: rings
                        draw_circle(p.x, p.y, r_ball, Color::new(col.r * 0.45, col.g * 0.45, col.b * 0.45, 0.75 * fade));
                        draw_circle_lines(p.x, p.y, r_ball, 1.5, Color::new(col.r, col.g, col.b, fade));
                    }
                    if hot {
                        draw_circle_lines(p.x, p.y, r_ball + 2.0, 1.5, WHITE);
                    }
                }
            }
        }
        // a grey "P" in the gizmo's corner, clear of every axis: back to the perspective view
        // (picture 29); it stays put however the view turns
        let pc = c + vec2(rad + 2.0 * k, rad + 2.0 * k);
        let pr = 8.0 * k;
        let p_hot = over && pc.distance(m) <= pr + 2.0;
        draw_circle(pc.x, pc.y, pr, if p_hot { Color::new(0.62, 0.62, 0.64, 1.0) } else { Color::new(0.42, 0.42, 0.44, 0.9) });
        let fs = 11.0 * k;
        let tw = ui.text_w("P", fs);
        ui.text("P", pc.x - tw * 0.5, pc.y - fs * 0.62, fs, Color::new(0.1, 0.1, 0.1, 1.0));
        ui.record("vp.gizmo.persp", Rect::new(pc.x - pr, pc.y - pr, pr * 2.0, pr * 2.0));
        if p_hot {
            hit = None;
            ui.tooltip = Some("Perspective view".into());
            if ui.inp.l_pressed {
                self.cam.set_view(0);
                ui.inp.l_pressed = false;
            }
        }
        if let Some((a, pos)) = hit {
            let v = view_for(a, pos);
            ui.tooltip = Some(format!("Look along {}{} ({})", if pos { "+" } else { "-" }, names[a], crate::render::VIEWS[v]));
            if ui.inp.l_pressed {
                // a second click on the same axis looks from the other side
                let v = if self.cam.view == v { view_for(a, !pos) } else { v };
                self.cam.set_view(v);
                ui.inp.l_pressed = false;
            }
        } else if over && ui.inp.l_pressed {
            // dragging the gizmo's background turns the view
            ui.inp.l_pressed = false;
            self.gizmo_orbit = true;
        }
        if self.gizmo_orbit {
            if ui.inp.l_down {
                let dm = m - self.last_mouse_nav;
                self.cam.orbit(dm.x, dm.y);
            } else {
                self.gizmo_orbit = false;
            }
        }
        self.last_mouse_nav = m;
        ui.record("vp.gizmo", area);
        Some(area)
    }
}
