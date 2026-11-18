//! The welcome window (picture 37, replacing picture 36's full-screen splash): a small window in
//! the middle of the app, like Blender's splash. A rectangular picture on top (a tree growing on a
//! dusk sky) with the name "TreeCraft" in the middle and the version at the top right; below it
//! New File, Recent Files, Open..., Recover Last Session, and Documentation / What's New (left
//! blank for now). It stays after TreeCraft has loaded until a click outside it (or Esc).
//! Also: TreeCraft's own Start-menu shortcut, carrying the program's icon (picture 36).

use crate::app::App;
use crate::ui::*;
use macroquad::prelude::*;

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

struct Br {
    a: Vec2,
    b: Vec2,
    w: f32,
    depth: u32,
}

fn grow(out: &mut Vec<Br>, leaves: &mut Vec<(Vec2, u32)>, a: Vec2, ang: f32, len: f32, w: f32, depth: u32, max: u32, seed: &mut u32) {
    let b = a + vec2(ang.sin(), -ang.cos()) * len;
    out.push(Br { a, b, w, depth });
    if depth >= max {
        leaves.push((b, *seed));
        return;
    }
    let mut rnd = || {
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (*seed >> 8) as f32 / (1u32 << 24) as f32
    };
    let spread = 0.38 + rnd() * 0.18;
    let (l1, l2) = (0.72 + rnd() * 0.1, 0.68 + rnd() * 0.1);
    let tilt = (rnd() - 0.5) * 0.2;
    grow(out, leaves, b, ang - spread + tilt, len * l1, w * 0.7, depth + 1, max, seed);
    grow(out, leaves, b, ang + spread + tilt, len * l2, w * 0.7, depth + 1, max, seed);
}

/// One tree growing at `base` (`size` = trunk length), `t` seconds in.
fn tree(base: Vec2, size: f32, max: u32, seed: u32, t: f32, dark: f32) {
    let mut brs = vec![];
    let mut leaves = vec![];
    let mut s = seed;
    grow(&mut brs, &mut leaves, base, 0.0, size, size * 0.19, 0, max, &mut s);
    let g = smooth(t / 1.6) * (max + 1) as f32;
    for br in &brs {
        let f = (g - br.depth as f32).clamp(0.0, 1.0);
        if f <= 0.0 {
            continue;
        }
        let e = br.a.lerp(br.b, f);
        let shade = (0.85 - br.depth as f32 * 0.05) * dark;
        let c = Color::new(0.42 * shade, 0.30 * shade, 0.22 * shade, 1.0);
        draw_line(br.a.x, br.a.y, e.x, e.y, br.w.max(1.0), c);
        draw_circle(e.x, e.y, br.w.max(1.0) * 0.5, c);
    }
    let lf = smooth((t - 1.1) / 0.9);
    if lf > 0.0 {
        for (p, s) in &leaves {
            let k = (*s % 5) as f32;
            let col = if s % 7 == 0 {
                Color::new(1.0 * dark, 0.78 * dark, 0.30 * dark, 0.9 * lf)
            } else {
                Color::new((0.35 + k * 0.05) * dark, (0.70 + k * 0.04) * dark, 0.36 * dark, 0.85 * lf)
            };
            let sway = (t * 1.5 + p.x * 0.01).sin() * 1.2;
            draw_circle(p.x + sway, p.y, size * 0.075 * (0.7 + 0.3 * lf), col);
        }
    }
}

/// The picture at the top of the welcome window, drawn into `r` (UI units).
pub fn draw_art(ui: &mut Ui, r: Rect, t: f32) {
    ui.set_clip(Some(r));
    let (x0, y0, w, h) = (r.x, r.y, r.w, r.h);
    // the sky: deep indigo above, teal toward the horizon
    let steps = 30;
    for i in 0..steps {
        let f = i as f32 / steps as f32;
        let c = Color::new(0.06 + 0.05 * f, 0.08 + 0.18 * f, 0.18 + 0.09 * f, 1.0);
        draw_rectangle(x0, y0 + h * f, w, h / steps as f32 + 1.0, c);
    }
    let ground = y0 + h * 0.84;
    // a warm glow behind the name
    for k in 0..10 {
        let rr = h * (0.75 - k as f32 * 0.06);
        draw_circle(x0 + w * 0.5, ground - h * 0.2, rr, Color::new(1.0, 0.72, 0.38, 0.016 + k as f32 * 0.004));
    }
    // stars
    for k in 0..50u32 {
        let x = x0 + ((k * 7919) % 1000) as f32 / 1000.0 * w;
        let y = y0 + ((k * 104729) % 1000) as f32 / 1000.0 * h * 0.55;
        let tw = 0.4 + 0.6 * ((t * 2.0 + k as f32).sin() * 0.5 + 0.5);
        draw_circle(x, y, 0.8 + (k % 3) as f32 * 0.35, Color::new(1.0, 1.0, 1.0, 0.25 * tw));
    }
    // distant hills, the ground, and the trees
    for i in 0..24 {
        let f = i as f32 / 23.0;
        let hh = h * (0.06 + 0.04 * (f * 9.0).sin().abs());
        draw_rectangle(x0 + w * f - w / 46.0, ground - hh, w / 23.0 + 1.0, hh, Color::new(0.06, 0.13, 0.14, 1.0));
    }
    draw_rectangle(x0, ground, w, h - (ground - y0), Color::new(0.04, 0.09, 0.09, 1.0));
    tree(vec2(x0 + w * 0.18, ground), h * 0.145, 7, 7, t, 1.0);
    tree(vec2(x0 + w * 0.88, ground), h * 0.105, 6, 23, t - 0.25, 0.8);
    // the name in the middle, and a line under it
    let fade = smooth((t - 0.2) / 0.8);
    let size = (h * 0.2).min(60.0);
    let name = "TreeCraft";
    let nw = ui.text_w(name, size);
    let nx = x0 + (w - nw) * 0.5;
    let ny = y0 + h * 0.36;
    ui.text(name, nx + 2.0, ny + 3.0, size, Color::new(0.0, 0.0, 0.0, 0.45 * fade));
    ui.text(name, nx, ny, size, Color::new(0.97, 0.94, 0.85, fade));
    let tag = "Grow, shape and export trees";
    let ts = 13.0;
    let tw = ui.text_w(tag, ts);
    ui.text(tag, x0 + (w - tw) * 0.5, ny + size * 1.08, ts, Color::new(0.64, 0.87, 0.68, fade));
    // the version at the top right
    let ver = format!("{} Beta", env!("CARGO_PKG_VERSION"));
    let vw = ui.text_w(&ver, 12.0);
    ui.text(&ver, x0 + w - vw - 12.0, y0 + 9.0, 12.0, Color::new(0.92, 0.92, 0.92, 0.9));
    ui.text("Artwork: TreeCraft", x0 + w - ui.text_w("Artwork: TreeCraft", 11.0) - 12.0, y0 + h - 20.0, 11.0, Color::new(1.0, 1.0, 1.0, 0.35));
    ui.set_clip(None);
}

// ---- small line icons for the welcome window's items

fn page(x: f32, y: f32, c: Color) {
    draw_rectangle_lines(x + 2.0, y + 1.0, 10.0, 13.0, 1.2, c);
    draw_line(x + 4.5, y + 5.0, x + 9.5, y + 5.0, 1.0, c);
    draw_line(x + 4.5, y + 8.0, x + 9.5, y + 8.0, 1.0, c);
}

fn icon(kind: &str, x: f32, y: f32, c: Color) {
    match kind {
        "new" => {
            page(x, y, c);
            draw_circle(x + 12.0, y + 3.0, 3.6, C_PANEL);
            draw_line(x + 9.5, y + 3.0, x + 14.5, y + 3.0, 1.4, c);
            draw_line(x + 12.0, y + 0.5, x + 12.0, y + 5.5, 1.4, c);
        }
        "tree" => {
            draw_line(x + 7.0, y + 14.0, x + 7.0, y + 8.0, 1.6, c);
            draw_circle_lines(x + 7.0, y + 5.5, 4.5, 1.2, c);
        }
        "file" => {
            page(x, y, c);
            draw_circle(x + 11.0, y + 11.0, 3.0, C_PANEL);
            draw_circle_lines(x + 11.0, y + 11.0, 2.6, 1.0, c);
        }
        "more" => {
            draw_circle_lines(x + 6.0, y + 6.0, 4.5, 1.3, c);
            draw_line(x + 9.3, y + 9.3, x + 13.5, y + 13.5, 1.6, c);
        }
        "open" => {
            draw_rectangle(x + 1.0, y + 4.0, 13.0, 9.0, c);
            draw_rectangle(x + 1.0, y + 2.0, 5.0, 3.0, c);
        }
        "recover" => {
            draw_arc(x + 7.5, y + 7.5, 24, 5.5, 60.0, 1.3, 280.0, c);
            draw_triangle(vec2(x + 1.0, y + 4.0), vec2(x + 5.5, y + 4.5), vec2(x + 2.5, y + 8.5), c);
            draw_line(x + 7.5, y + 7.5, x + 7.5, y + 4.5, 1.2, c);
            draw_line(x + 7.5, y + 7.5, x + 10.0, y + 8.5, 1.2, c);
        }
        "doc" => {
            draw_rectangle_lines(x + 1.0, y + 2.0, 6.5, 11.0, 1.2, c);
            draw_rectangle_lines(x + 7.5, y + 2.0, 6.5, 11.0, 1.2, c);
        }
        _ => {
            // a star
            let cx = x + 7.5;
            let cy = y + 7.5;
            for i in 0..5 {
                let a = std::f32::consts::TAU * i as f32 / 5.0 - std::f32::consts::FRAC_PI_2;
                draw_line(cx, cy, cx + a.cos() * 6.5, cy + a.sin() * 6.5, 1.4, c);
            }
        }
    }
}

impl App {
    /// The welcome window keeps the mouse while it is open.
    pub fn welcome_takes_input(&self) -> bool {
        self.welcome.is_some()
    }

    /// Draw the welcome window and carry out a click in it; a click outside (or Esc) closes it.
    pub fn draw_welcome(&mut self, ui: &mut Ui) {
        let Some(t0) = self.welcome else { return };
        let t = (get_time() - t0) as f32;
        ui.blocked = false;
        let w = 560.0f32.min(ui.sw - 20.0);
        let art_h = (w * 0.5).round();
        let rows = self.prefs.recent.len().min(5).max(1) + 1;
        let h = art_h + 16.0 + 22.0 + rows as f32 * 22.0 + 24.0 + 44.0 + 14.0;
        let r = Rect::new(((ui.sw - w) * 0.5).round(), ((ui.sh - h) * 0.5).round().max(30.0), w, h);
        ui.record("welcome.window", r);
        let fade = smooth(t / 0.25);
        draw_rectangle(r.x + 4.0, r.y + 6.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.4 * fade));
        draw_round_rect(r, 6.0, C_PANEL);
        draw_art(ui, Rect::new(r.x, r.y, r.w, art_h), t);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, C_LINE);

        let m = ui.mouse();
        let pressed = ui.inp.l_pressed;
        let mut action: Option<String> = None;
        let col_w = (r.w - 80.0) * 0.5;
        let lx = r.x + 40.0;
        let rx = lx + col_w + 20.0;
        let row_h = 22.0;
        let item = |ui: &mut Ui, key: &str, kind: &str, label: &str, x: f32, y: f32, enabled: bool, action: &mut Option<String>| {
            let ir = Rect::new(x - 6.0, y - 2.0, col_w - 4.0, row_h - 2.0);
            ui.record(&format!("welcome.{}", key), ir);
            let hot = enabled && ir.contains(m);
            if hot {
                draw_round_rect(ir, 3.0, C_BTN_HOT);
            }
            let c = if enabled { C_TEXT } else { C_TEXT_DIM };
            icon(kind, x, y + 1.0, if hot { WHITE } else { c });
            ui.text_fit(label, x + 24.0, y + 1.0, col_w - 34.0, 12.5, if hot { WHITE } else { c });
            if hot && pressed {
                *action = Some(key.to_string());
            }
        };
        let mut y = r.y + art_h + 16.0;
        ui.text("New File", lx, y, 12.0, C_TEXT_DIM);
        ui.text("Recent Files", rx, y, 12.0, C_TEXT_DIM);
        y += 22.0;
        item(ui, "new", "new", "Empty Tree", lx, y, true, &mut action);
        item(ui, "demo", "tree", "Demo Tree", lx, y + row_h, true, &mut action);
        let recent: Vec<String> = self.prefs.recent.iter().take(5).cloned().collect();
        if recent.is_empty() {
            ui.text("(no recent files)", rx + 24.0, y + 1.0, 12.5, C_TEXT_DIM);
        }
        for (i, p) in recent.iter().enumerate() {
            let name = std::path::Path::new(p).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| p.clone());
            item(ui, &format!("recent{}", i), "file", &name, rx, y + row_h * i as f32, true, &mut action);
            if Rect::new(rx - 6.0, y + row_h * i as f32 - 2.0, col_w - 4.0, row_h - 2.0).contains(m) {
                self.set_status(p);
            }
        }
        let ry = y + row_h * recent.len().max(1) as f32;
        item(ui, "more", "more", "More...", rx, ry, true, &mut action);
        // a rule, then Open / Recover on the left and the future help links on the right
        let by = r.y + r.h - 58.0;
        draw_line(lx - 6.0, by - 12.0, r.x + r.w - 40.0, by - 12.0, 1.0, C_LINE);
        let has_session = App::session_path().map_or(false, |p| p.exists());
        item(ui, "open", "open", "Open...", lx, by, true, &mut action);
        item(ui, "recover", "recover", "Recover Last Session", lx, by + row_h, has_session, &mut action);
        item(ui, "docs", "doc", "Documentation", rx, by, true, &mut action);
        item(ui, "whatsnew", "star", "What's New", rx, by + row_h, true, &mut action);

        let outside = pressed && !r.contains(m);
        let esc = is_key_pressed(KeyCode::Escape);
        if let Some(a) = action {
            self.welcome = None;
            match a.as_str() {
                "new" => self.file_new(),
                "demo" => {
                    if self.confirm_discard() {
                        self.load_demo();
                        self.tree_path = None;
                        self.regenerate();
                        self.frame_tree();
                        self.undo_reset();
                    }
                }
                "more" | "open" => self.open_tree(),
                "recover" => self.recover_session(),
                // left blank for now (picture 37): documentation comes later
                "docs" | "whatsnew" => {}
                s => {
                    if let Some(i) = s.strip_prefix("recent").and_then(|n| n.parse::<usize>().ok()) {
                        if let Some(p) = recent.get(i) {
                            let p = std::path::PathBuf::from(p);
                            if !p.exists() {
                                self.set_status(&format!("{} isn't there any more", p.display()));
                                self.prefs.remove_recent(&p.display().to_string());
                            } else if self.confirm_discard() {
                                self.open_path(p);
                            }
                        }
                    }
                }
            }
        } else if outside || esc {
            self.welcome = None;
        }
        // the click belongs to the window (or closed it): nothing behind sees it
        ui.inp.l_pressed = false;
        ui.blocked = true;
    }
}

/// TreeCraft's own Start-menu shortcut (Windows), with the program's icon. An older pin keeps the
/// icon Windows cached for it, so this gives the person a fresh entry to pin. Made once, when
/// missing.
pub fn ensure_start_menu_shortcut() {
    #[cfg(windows)]
    {
        let Some(appdata) = std::env::var_os("APPDATA") else { return };
        let lnk = std::path::PathBuf::from(appdata).join("Microsoft").join("Windows").join("Start Menu").join("Programs").join("TreeCraft.lnk");
        if lnk.exists() {
            return;
        }
        let Ok(exe) = std::env::current_exe() else { return };
        let esc = |p: &std::path::Path| p.display().to_string().replace('\'', "''");
        let script = format!(
            "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{}');$s.TargetPath='{}';$s.WorkingDirectory='{}';$s.IconLocation='{},0';$s.Description='TreeCraft';$s.Save()",
            esc(&lnk),
            esc(&exe),
            esc(exe.parent().unwrap_or(std::path::Path::new("."))),
            esc(&exe)
        );
        let _ = std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script]).spawn();
    }
}
