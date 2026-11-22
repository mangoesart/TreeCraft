//! The Edit Map window (picture 74): a click on a material's loaded texture opens it. It shows
//! the picture's file (browse for another, reload it, clear it), the adjustments that suit the
//! slot (colour, normal or value map; textures::adjust_image applies them) and a preview of the
//! result. Its changes apply at once and are saved with the material.
use crate::app::App;
use crate::model::{MapAdjust, SLOT_NAMES};
use crate::ui::*;
use macroquad::prelude::*;

struct State {
    mat: usize,
    slot: usize,
    rect: Rect,
    drag: Option<Vec2>,
    /// Resizing from a corner (picture 75): 0 top left, 1 top right, 2 bottom left, 3 bottom
    /// right, with the mouse and the window where the drag started.
    resize: Option<(u8, Vec2, Rect)>,
}

thread_local! {
    static OPEN: std::cell::RefCell<Option<State>> = const { std::cell::RefCell::new(None) };
}

const CHANNELS: [&str; 5] = ["Luminance", "Red", "Green", "Blue", "Alpha"];

impl App {
    /// Open the Edit Map window on material `mat`'s slot `slot`.
    pub fn open_map_edit(&mut self, mat: usize, slot: usize) {
        let (w, h) = (760.0, 470.0);
        let keep = OPEN.with(|o| o.borrow().as_ref().map(|s| s.rect));
        // (placed beside the panels; kept inside the window when drawn)
        let rect = keep.unwrap_or(Rect::new(360.0, 110.0, w, h));
        OPEN.with(|o| *o.borrow_mut() = Some(State { mat, slot, rect, drag: None, resize: None }));
    }

    pub fn map_edit_takes_input(&self, x: f32, y: f32) -> bool {
        OPEN.with(|o| o.borrow().as_ref().map_or(false, |s| s.rect.contains(vec2(x, y)) || s.drag.is_some() || s.resize.is_some() || corner_at(s.rect, vec2(x, y)).is_some()))
    }

    /// Apply a slot's adjustments again (its picture, or the material's colour picture).
    fn refresh_map(&mut self, mat: usize, slot: usize) {
        let Some(m) = self.materials.get_mut(mat) else { return };
        if slot == 0 || slot == 1 {
            m.dirty_tex = true;
            crate::textures::refresh_material(m);
        }
        if slot != 0 {
            if let Some(sl) = m.slots.get_mut(slot) {
                crate::textures::refresh_slot(sl, slot);
            }
        }
    }

    pub fn draw_map_edit(&mut self, ui: &mut Ui) {
        let Some(mut st) = OPEN.with(|o| o.borrow_mut().take()) else { return };
        if st.mat >= self.materials.len() || st.slot >= SLOT_NAMES.len() {
            return;
        }
        let was_blocked = ui.blocked;
        ui.blocked = false;
        st.rect.x = st.rect.x.min(ui.sw - st.rect.w).max(0.0);
        st.rect.y = st.rect.y.min(ui.sh - st.rect.h).max(0.0);
        let m = ui.mouse();
        // resizing from any of its four corners (picture 75), at least 600 x 380
        if let Some((c, m0, r0)) = st.resize {
            if ui.inp.l_down {
                let d = m - m0;
                let (min_w, min_h) = (600.0, 380.0);
                let (mut x0, mut y0, mut x1, mut y1) = (r0.x, r0.y, r0.x + r0.w, r0.y + r0.h);
                if c == 0 || c == 2 {
                    x0 = (r0.x + d.x).min(x1 - min_w).max(0.0);
                } else {
                    x1 = (x1 + d.x).max(x0 + min_w).min(ui.sw);
                }
                if c == 0 || c == 1 {
                    y0 = (r0.y + d.y).min(y1 - min_h).max(0.0);
                } else {
                    y1 = (y1 + d.y).max(y0 + min_h).min(ui.sh);
                }
                st.rect = Rect::new(x0, y0, x1 - x0, y1 - y0);
            } else {
                st.resize = None;
            }
        } else if ui.inp.l_pressed {
            if let Some(c) = corner_at(st.rect, m) {
                st.resize = Some((c, m, st.rect));
                ui.inp.l_pressed = false;
            }
        }
        if let Some(c) = corner_at(st.rect, m).or(st.resize.map(|r| r.0)) {
            let _ = c;
            ui.tooltip = Some("Drag to resize".into());
        }
        // moving by the title bar
        let title = Rect::new(st.rect.x, st.rect.y, st.rect.w - 30.0, 26.0);
        if ui.inp.l_pressed && title.contains(m) && corner_at(st.rect, m).is_none() {
            st.drag = Some(m - vec2(st.rect.x, st.rect.y));
        }
        if let Some(off) = st.drag {
            if ui.inp.l_down {
                st.rect.x = (m.x - off.x).clamp(0.0, (ui.sw - 80.0).max(0.0));
                st.rect.y = (m.y - off.y).clamp(0.0, (ui.sh - 40.0).max(0.0));
            } else {
                st.drag = None;
            }
        }
        let r = st.rect;
        draw_rectangle(r.x + 4.0, r.y + 5.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.4));
        draw_round_rect(r, 6.0, C_PANEL);
        draw_round_rect_lines(r, 6.0, 1.0, C_LINE);
        draw_rectangle(r.x, r.y, r.w, 26.0, C_PANEL2);
        // the corner grips
        for (cx, cy, sx, sy) in [(r.x, r.y, 1.0, 1.0), (r.x + r.w, r.y, -1.0, 1.0), (r.x, r.y + r.h, 1.0, -1.0), (r.x + r.w, r.y + r.h, -1.0, -1.0)] {
            let gc = Color::new(1.0, 1.0, 1.0, 0.35);
            for k in 1..=3 {
                let o = 4.0 * k as f32;
                draw_line(cx + sx * o, cy + sy * 2.0, cx + sx * 2.0, cy + sy * o, 1.0, gc);
            }
        }
        let slot_name = SLOT_NAMES[st.slot];
        let kind = crate::textures::map_kind(st.slot);
        let mat_name = self.materials[st.mat].name.clone();
        ui.text(&format!("Edit Map - {}   ({})", slot_name, mat_name), r.x + 10.0, r.y + 6.0, 13.0, C_TEXT);
        ui.record("mapedit", r);
        let mut close = false;
        let xr = Rect::new(r.x + r.w - 28.0, r.y + 3.0, 22.0, 20.0);
        if ui.button("mapedit.close", xr, "x") || ui.inp.key(KeyCode::Escape) {
            close = true;
        }
        // the file: browse, reload, clear
        let file = if st.slot == 0 { self.materials[st.mat].file.clone() } else { self.materials[st.mat].slots[st.slot].file.clone() };
        let fy = r.y + 36.0;
        ui.text("File", r.x + 10.0, fy + 3.0, 12.5, C_TEXT_DIM);
        let fr = Rect::new(r.x + 46.0, fy, r.w - 46.0 - 10.0 - 3.0 * 76.0, 22.0);
        draw_rectangle(fr.x, fr.y, fr.w, fr.h, C_FIELD);
        ui.text_fit(if file.is_empty() { "(no picture)" } else { &file }, fr.x + 5.0, fr.y + 4.0, fr.w - 10.0, 12.0, C_TEXT);
        let mut action = 0;
        for (k, label) in ["Browse...", "Reload", "Clear"].iter().enumerate() {
            let br = Rect::new(fr.x + fr.w + 6.0 + k as f32 * 76.0, fy, 72.0, 22.0);
            if ui.button(&format!("mapedit.file{}", k), br, label) {
                action = k + 1;
            }
        }
        match action {
            1 => {
                if let Some(f) = crate::props::pick_image() {
                    self.load_slot(st.mat, st.slot, Some(f));
                }
            }
            2 if !file.is_empty() => self.load_slot(st.mat, st.slot, Some(file.clone())),
            3 => {
                self.load_slot(st.mat, st.slot, None);
                close = true;
            }
            _ => {}
        }
        // adjustments on the left
        let ax = r.x + 10.0;
        let aw = 300.0;
        let mut y = r.y + 72.0;
        ui.text(match kind {
            0 => "Adjustments (colour map)",
            1 => "Adjustments (normal map)",
            _ => "Adjustments (value map)",
        }, ax, y, 13.0, C_TEXT);
        y += 24.0;
        let mut a = self.materials[st.mat].slots[st.slot].adjust.clone();
        let before = a.clone();
        let d = MapAdjust::default();
        let lw = 110.0;
        let mut check = |ui: &mut Ui, y: &mut f32, key: &str, label: &str, v: &mut bool, tip: &str| {
            ui.text(label, ax, *y + 3.0, 12.5, C_TEXT_DIM);
            let cb = Rect::new(ax + lw, *y + 2.0, 16.0, 16.0);
            ui.checkbox(key, cb, v);
            if Rect::new(ax, *y, lw + 20.0, 20.0).contains(ui.mouse()) {
                ui.tooltip = Some(tip.into());
            }
            *y += 22.0;
        };
        if kind == 0 {
            check(ui, &mut y, "me.srgb", "sRGB to linear", &mut a.srgb, "On for colour pictures (most are sRGB). Off takes the values as linear and brightens them to show as colour");
        }
        if kind == 1 {
            check(ui, &mut y, "me.norm", "Normalize", &mut a.normalize, "Rescale each texel of the normal map to unit length");
        }
        if kind == 2 {
            ui.text("Channel", ax, y + 3.0, 12.5, C_TEXT_DIM);
            let names: Vec<String> = CHANNELS.iter().map(|s| s.to_string()).collect();
            ui.combo("me.chan", Rect::new(ax + lw, y, aw - lw, 20.0), &names, &mut a.channel);
            y += 24.0;
        }
        check(ui, &mut y, "me.inv", "Invert", &mut a.invert, "Turn the picture's values over (1 - value)");
        if kind != 2 {
            ui.text("Invert R G B", ax, y + 3.0, 12.5, C_TEXT_DIM);
            for k in 0..3 {
                let cb = Rect::new(ax + lw + k as f32 * 26.0, y + 2.0, 16.0, 16.0);
                ui.checkbox(&format!("me.inv{}", k), cb, &mut a.invert_rgb[k]);
            }
            if Rect::new(ax, y, lw + 80.0, 20.0).contains(ui.mouse()) {
                ui.tooltip = Some(if kind == 1 { "Flip one axis of the normal map (green flips up and down, for maps made the other way)".into() } else { "Invert the red, green or blue channel alone".into() });
            }
            y += 24.0;
        }
        // sliders, each with a button putting it back
        let mut slider = |ui: &mut Ui, y: &mut f32, key: &str, label: &str, v: &mut f32, lo: f32, hi: f32, def: f32| {
            ui.text(label, ax, *y + 3.0, 12.5, C_TEXT_DIM);
            let sr = Rect::new(ax + lw, *y, aw - lw - 26.0, 20.0);
            ui.slider(key, sr, v, lo, hi, 2);
            let rr = Rect::new(ax + aw - 22.0, *y, 22.0, 20.0);
            if ui.button(&format!("{}.reset", key), rr, "o") {
                *v = def;
            }
            if rr.contains(ui.mouse()) {
                ui.tooltip = Some("Put it back".into());
            }
            *y += 24.0;
        };
        if kind != 1 {
            slider(ui, &mut y, "me.br", "Brightness", &mut a.brightness, -1.0, 1.0, d.brightness);
            slider(ui, &mut y, "me.ct", "Contrast", &mut a.contrast, -1.0, 1.0, d.contrast);
        }
        if kind == 0 {
            slider(ui, &mut y, "me.sat", "Saturation", &mut a.saturation, -1.0, 1.0, d.saturation);
            slider(ui, &mut y, "me.r", "Red", &mut a.red, -1.0, 1.0, d.red);
            slider(ui, &mut y, "me.g", "Green", &mut a.green, -1.0, 1.0, d.green);
            slider(ui, &mut y, "me.b", "Blue", &mut a.blue, -1.0, 1.0, d.blue);
        }
        if kind != 1 {
            slider(ui, &mut y, "me.min", "Minimum", &mut a.min, 0.0, 1.0, d.min);
            slider(ui, &mut y, "me.max", "Maximum", &mut a.max, 0.0, 1.0, d.max);
        }
        y += 6.0;
        if ui.button("me.resetall", Rect::new(ax, y, 140.0, 22.0), "Reset all") {
            a = MapAdjust::default();
        }
        if a != before {
            self.materials[st.mat].slots[st.slot].adjust = a;
            self.refresh_map(st.mat, st.slot);
        }
        // the map, as the material uses it now
        let px = ax + aw + 16.0;
        let pr = Rect::new(px, r.y + 92.0, r.x + r.w - 10.0 - px, r.h - 102.0);
        let mt = &self.materials[st.mat];
        let pic = if st.slot == 0 { mt.tex.clone() } else { mt.slots[st.slot].tex.clone() };
        let dims = if st.slot == 0 { mt.base.as_ref().map(|i| (i.w, i.h)) } else { mt.slots[st.slot].image.as_ref().map(|i| (i.w, i.h)) };
        ui.text(&match dims { Some((w, h)) => format!("Map  ({} x {})", w, h), None => "Map".to_string() }, px, r.y + 72.0, 13.0, C_TEXT);
        checker(pr, 12.0);
        if let Some(t) = pic {
            let s = (pr.w / t.width().max(1.0)).min(pr.h / t.height().max(1.0));
            let (w, h) = (t.width() * s, t.height() * s);
            draw_texture_ex(&t, pr.x + (pr.w - w) * 0.5, pr.y + (pr.h - h) * 0.5, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
        }
        draw_rectangle_lines(pr.x, pr.y, pr.w, pr.h, 1.0, C_LINE);
        // clicks inside are the window's; a click outside closes it
        let inside = r.contains(m);
        if ui.inp.l_pressed && !inside && st.drag.is_none() && ui.popup.is_none() {
            close = true;
        }
        if inside {
            ui.inp.l_pressed = false;
            ui.inp.r_pressed = false;
        }
        ui.blocked = was_blocked;
        if !close {
            OPEN.with(|o| *o.borrow_mut() = Some(st));
        }
    }
}

/// Which corner grip of the window `r` is under `m` (14 px squares): 0 top left, 1 top right,
/// 2 bottom left, 3 bottom right.
fn corner_at(r: Rect, m: Vec2) -> Option<u8> {
    let g = 14.0;
    let near = |x: f32, y: f32| (m.x - x).abs() <= g && (m.y - y).abs() <= g;
    if near(r.x, r.y) {
        Some(0)
    } else if near(r.x + r.w, r.y) {
        Some(1)
    } else if near(r.x, r.y + r.h) {
        Some(2)
    } else if near(r.x + r.w, r.y + r.h) {
        Some(3)
    } else {
        None
    }
}