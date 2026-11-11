//! A small immediate-mode UI toolkit styled after the original's dark theme.
use macroquad::prelude::*;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

// Theme colours (from the original's style sheet: #292929, #3d3d3d, #474747, #4f4f4f, #5875b9, #da9747)
pub const C_BG: Color = Color::new(0.161, 0.161, 0.161, 1.0); // #292929
pub const C_VIEW: Color = Color::new(0.165, 0.165, 0.165, 1.0);
pub const C_PANEL: Color = Color::new(0.231, 0.231, 0.231, 1.0); // ~#3b3b3b
pub const C_PANEL2: Color = Color::new(0.20, 0.20, 0.20, 1.0);
pub const C_BTN: Color = Color::new(0.278, 0.278, 0.278, 1.0); // #474747
pub const C_BTN_HOT: Color = Color::new(0.310, 0.310, 0.310, 1.0); // #4f4f4f
pub const C_FIELD: Color = Color::new(0.180, 0.180, 0.180, 1.0); // #2e2e2e
pub const C_ACCENT: Color = Color::new(0.345, 0.459, 0.725, 1.0); // #5875b9
pub const C_ACCENT2: Color = Color::new(0.424, 0.525, 0.757, 1.0); // #6c86c1
pub const C_ORANGE: Color = Color::new(0.855, 0.592, 0.278, 1.0); // #da9747
pub const C_TEXT: Color = Color::new(1.0, 1.0, 1.0, 0.86);
pub const C_TEXT_DIM: Color = Color::new(1.0, 1.0, 1.0, 0.5);
pub const C_TEXT_FAINT: Color = Color::new(1.0, 1.0, 1.0, 0.28);
pub const C_LINE: Color = Color::new(1.0, 1.0, 1.0, 0.12);

#[derive(Default, Clone, Debug)]
pub struct Inp {
    pub mx: f32,
    pub my: f32,
    pub l_down: bool,
    pub l_pressed: bool,
    pub l_released: bool,
    pub l_double: bool,
    pub r_down: bool,
    pub r_pressed: bool,
    pub r_released: bool,
    pub m_down: bool,
    pub m_pressed: bool,
    pub wheel: f32,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub keys: Vec<KeyCode>,
    /// Typed characters this frame (for text fields).
    pub chars: Vec<char>,
}

impl Inp {
    pub fn key(&self, k: KeyCode) -> bool {
        self.keys.contains(&k)
    }
}

pub struct RealInput {
    last_click_t: f64,
    last_click: Vec2,
}

impl RealInput {
    pub fn new() -> Self {
        RealInput { last_click_t: -10.0, last_click: Vec2::ZERO }
    }
    pub fn read(&mut self, scale: f32) -> Inp {
        let (mx, my) = mouse_position();
        let mut i = Inp {
            mx: mx / scale,
            my: my / scale,
            l_down: is_mouse_button_down(MouseButton::Left),
            l_pressed: is_mouse_button_pressed(MouseButton::Left),
            l_released: is_mouse_button_released(MouseButton::Left),
            r_down: is_mouse_button_down(MouseButton::Right),
            r_pressed: is_mouse_button_pressed(MouseButton::Right),
            r_released: is_mouse_button_released(MouseButton::Right),
            m_down: is_mouse_button_down(MouseButton::Middle),
            m_pressed: is_mouse_button_pressed(MouseButton::Middle),
            wheel: mouse_wheel().1,
            shift: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
            ctrl: is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl),
            alt: is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt),
            ..Default::default()
        };
        if i.wheel.abs() > 0.0 {
            i.wheel = i.wheel.signum();
        }
        if i.l_pressed {
            let t = get_time();
            let p = vec2(i.mx, i.my);
            if t - self.last_click_t < 0.35 && p.distance(self.last_click) < 6.0 {
                i.l_double = true;
                self.last_click_t = -10.0;
            } else {
                self.last_click_t = t;
                self.last_click = p;
            }
        }
        for k in get_keys_pressed() {
            i.keys.push(k);
        }
        while let Some(c) = get_char_pressed() {
            if !c.is_control() {
                i.chars.push(c);
            }
        }
        i
    }
}

pub struct Popup {
    pub id: u64,
    pub rect: Rect,
    pub items: Vec<String>,
    pub current: Option<usize>,
}

pub struct Ui {
    pub inp: Inp,
    pub font: Option<Font>,
    pub scale: f32,
    pub active: u64,
    drag_start_val: f32,
    pub drag_start_x: f32,
    pub popup: Option<Popup>,
    popup_result: Option<(u64, usize)>,
    pub blocked: bool,
    pub clip: Option<Rect>,
    pub rects: HashMap<String, Rect>,
    pub texts: Vec<String>,
    pub tooltip: Option<String>,
    pub sw: f32,
    pub sh: f32,
}

pub fn hid(s: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish() | 1
}

impl Ui {
    pub fn new(font: Option<Font>) -> Self {
        Ui {
            inp: Inp::default(),
            font,
            scale: 1.0,
            active: 0,
            drag_start_val: 0.0,
            drag_start_x: 0.0,
            popup: None,
            popup_result: None,
            blocked: false,
            clip: None,
            rects: HashMap::new(),
            texts: Vec::new(),
            tooltip: None,
            sw: 1600.0,
            sh: 900.0,
        }
    }

    /// Call at the start of each frame with fresh input.
    pub fn begin(&mut self, inp: Inp) {
        self.inp = inp;
        self.blocked = false;
        self.rects.clear();
        self.texts.clear();
        self.tooltip = None;
        if let Some(p) = &self.popup {
            let inside = p.rect.contains(vec2(self.inp.mx, self.inp.my));
            if inside {
                self.blocked = true;
            } else if self.inp.l_pressed || self.inp.r_pressed {
                // click outside closes the popup and is swallowed
                self.popup = None;
                self.blocked = true;
                self.inp.l_pressed = false;
                self.inp.r_pressed = false;
            }
        }
        if !self.inp.l_down && !self.inp.l_released {
            self.active = 0;
        }
    }

    pub fn mouse(&self) -> Vec2 {
        vec2(self.inp.mx, self.inp.my)
    }

    pub fn record(&mut self, key: &str, r: Rect) {
        self.rects.insert(key.to_string(), r);
    }

    pub fn in_clip(&self) -> bool {
        match self.clip {
            Some(c) => c.contains(self.mouse()),
            None => true,
        }
    }

    pub fn hover(&self, r: Rect) -> bool {
        !self.blocked && self.active == 0 && r.contains(self.mouse()) && self.in_clip()
    }
    pub fn hover_any(&self, r: Rect) -> bool {
        !self.blocked && r.contains(self.mouse()) && self.in_clip()
    }
    pub fn clicked(&self, r: Rect) -> bool {
        self.hover(r) && self.inp.l_pressed
    }

    pub fn set_clip(&mut self, r: Option<Rect>) {
        self.clip = r;
        let gl = unsafe { get_internal_gl() };
        match r {
            Some(r) => {
                let s = self.scale;
                gl.quad_gl.scissor(Some((
                    (r.x * s) as i32,
                    (r.y * s) as i32,
                    (r.w * s).ceil() as i32,
                    (r.h * s).ceil() as i32,
                )));
            }
            None => gl.quad_gl.scissor(None),
        }
    }

    // ---------- text ----------
    pub fn text_w(&self, s: &str, size: f32) -> f32 {
        let fs = (size * self.scale).round() as u16;
        measure_text(s, self.font.as_ref(), fs.max(1), 1.0 / self.scale).width
    }

    /// Draw text with its top-left corner at (x, y).
    pub fn text(&mut self, s: &str, x: f32, y: f32, size: f32, color: Color) {
        let fs = (size * self.scale).round() as u16;
        draw_text_ex(
            s,
            x.round(),
            (y + size * 0.78).round(),
            TextParams {
                font: self.font.as_ref(),
                font_size: fs.max(1),
                font_scale: 1.0 / self.scale,
                color,
                ..Default::default()
            },
        );
        if self.texts.len() < 4000 {
            self.texts.push(s.to_string());
        }
    }

    pub fn text_center(&mut self, s: &str, r: Rect, size: f32, color: Color) {
        let w = self.text_w(s, size);
        self.text(s, r.x + (r.w - w) * 0.5, r.y + (r.h - size) * 0.5 - 1.0, size, color);
    }

    /// Text shortened with "..." to fit a width.
    pub fn text_fit(&mut self, s: &str, x: f32, y: f32, maxw: f32, size: f32, color: Color) {
        if self.text_w(s, size) <= maxw {
            self.text(s, x, y, size, color);
            return;
        }
        let mut t: String = s.to_string();
        while !t.is_empty() && self.text_w(&format!("{}...", t), size) > maxw {
            t.pop();
        }
        self.text(&format!("{}...", t), x, y, size, color);
    }

    // ---------- widgets ----------
    pub fn button(&mut self, key: &str, r: Rect, label: &str) -> bool {
        self.button_ex(key, r, label, false, true)
    }

    pub fn button_ex(&mut self, key: &str, r: Rect, label: &str, on: bool, enabled: bool) -> bool {
        self.record(&format!("btn:{}", key), r);
        let hot = enabled && self.hover(r);
        let bg = if on {
            C_ACCENT
        } else if hot {
            C_BTN_HOT
        } else {
            C_BTN
        };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        let col = if !enabled {
            C_TEXT_FAINT
        } else if on {
            WHITE
        } else {
            C_TEXT
        };
        self.text_center(label, r, 13.0, col);
        enabled && hot && self.inp.l_pressed
    }

    /// Flat toolbar button (no background until hovered).
    pub fn tool_button(&mut self, key: &str, r: Rect, label: &str, on: bool) -> bool {
        self.record(&format!("btn:{}", key), r);
        let hot = self.hover(r);
        if on {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(1.0, 1.0, 1.0, 0.16));
        } else if hot {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(1.0, 1.0, 1.0, 0.10));
        }
        self.text_center(label, r, 12.5, if on { WHITE } else { C_TEXT });
        hot && self.inp.l_pressed
    }

    pub fn slider(&mut self, key: &str, r: Rect, v: &mut f32, min: f32, max: f32, dec: usize) -> bool {
        let id = hid(key);
        self.record(&format!("sl:{}", key), r);
        let hot = self.hover(r);
        let mut changed = false;
        if hot && self.inp.l_pressed {
            self.active = id;
            self.drag_start_val = *v;
            self.drag_start_x = self.inp.mx;
            // click jumps to position when far from current
            let frac = ((self.inp.mx - r.x) / r.w).clamp(0.0, 1.0);
            let nv = min + frac * (max - min);
            if (nv - *v).abs() > (max - min) * 0.0 {
                *v = nv;
                self.drag_start_val = nv;
                changed = true;
            }
        }
        if self.active == id && self.inp.l_down {
            let mut d = (self.inp.mx - self.drag_start_x) / r.w * (max - min);
            if self.inp.shift {
                d *= 0.1;
            }
            let nv = (self.drag_start_val + d).clamp(min, max);
            if (nv - *v).abs() > 1e-7 {
                *v = nv;
                changed = true;
            }
        }
        let frac = ((*v - min) / (max - min)).clamp(0.0, 1.0);
        draw_rectangle(r.x, r.y, r.w, r.h, C_FIELD);
        let fill = if self.active == id { C_ACCENT2 } else { C_ACCENT };
        draw_rectangle(r.x, r.y, r.w * frac, r.h, Color::new(fill.r, fill.g, fill.b, 0.55));
        if hot || self.active == id {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.25));
        }
        let s = format!("{:.*}", dec, *v);
        let w = self.text_w(&s, 12.5);
        self.text(&s, r.x + r.w - w - 6.0, r.y + (r.h - 12.5) * 0.5 - 1.0, 12.5, C_TEXT);
        changed
    }

    pub fn islider(&mut self, key: &str, r: Rect, v: &mut i32, min: i32, max: i32) -> bool {
        let mut f = *v as f32;
        let ch = self.slider(key, r, &mut f, min as f32, max as f32, 0);
        let nv = f.round() as i32;
        if ch && nv != *v {
            *v = nv;
            return true;
        }
        false
    }

    pub fn checkbox(&mut self, key: &str, r: Rect, v: &mut bool) -> bool {
        self.record(&format!("cb:{}", key), r);
        let b = Rect::new(r.x, r.y + (r.h - 14.0) * 0.5, 14.0, 14.0);
        let hot = self.hover(r);
        draw_rectangle(b.x, b.y, b.w, b.h, if hot { C_BTN_HOT } else { C_FIELD });
        draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.3));
        if *v {
            draw_rectangle(b.x + 3.0, b.y + 3.0, 8.0, 8.0, C_ACCENT2);
        }
        if hot && self.inp.l_pressed {
            *v = !*v;
            return true;
        }
        false
    }

    pub fn combo(&mut self, key: &str, r: Rect, items: &[String], sel: &mut usize) -> bool {
        let id = hid(key);
        self.record(&format!("cmb:{}", key), r);
        let mut changed = false;
        if let Some((rid, idx)) = self.popup_result {
            if rid == id {
                self.popup_result = None;
                if idx != *sel {
                    *sel = idx;
                    changed = true;
                }
            }
        }
        let hot = self.hover(r);
        draw_rectangle(r.x, r.y, r.w, r.h, if hot { C_BTN_HOT } else { C_BTN });
        let label = items.get(*sel).cloned().unwrap_or_else(|| "-".to_string());
        self.text_fit(&label, r.x + 6.0, r.y + (r.h - 12.5) * 0.5 - 1.0, r.w - 22.0, 12.5, C_TEXT);
        // arrow
        let ax = r.x + r.w - 11.0;
        let ay = r.y + r.h * 0.5;
        draw_triangle(vec2(ax - 4.0, ay - 2.0), vec2(ax + 4.0, ay - 2.0), vec2(ax, ay + 3.0), C_TEXT_DIM);
        if hot && self.inp.l_pressed {
            self.open_popup(id, r, items.to_vec(), Some(*sel));
        }
        changed
    }

    pub fn open_popup(&mut self, id: u64, anchor: Rect, items: Vec<String>, current: Option<usize>) {
        let ih = 22.0;
        let w = items
            .iter()
            .map(|s| self.text_w(s, 12.5) + 28.0)
            .fold(anchor.w, f32::max);
        let h = ih * items.len() as f32 + 4.0;
        let sh = self.sh;
        let mut y = anchor.y + anchor.h;
        if y + h > sh {
            y = (anchor.y - h).max(0.0);
        }
        self.popup = Some(Popup { id, rect: Rect::new(anchor.x, y, w, h), items, current });
        self.inp.l_pressed = false;
    }

    /// Returns the item chosen from a menu-style popup opened with `open_popup`.
    pub fn take_popup_result(&mut self, key: &str) -> Option<usize> {
        let id = hid(key);
        if let Some((rid, idx)) = self.popup_result {
            if rid == id {
                self.popup_result = None;
                return Some(idx);
            }
        }
        None
    }

    pub fn popup_open_for(&self, key: &str) -> bool {
        self.popup.as_ref().map(|p| p.id == hid(key)).unwrap_or(false)
    }

    /// Draw the open popup last, on top of everything.
    pub fn end(&mut self) {
        let mut close = false;
        let mut result = None;
        let popup = self.popup.as_ref().map(|p| (p.id, p.rect, p.items.clone(), p.current));
        if let Some((pid, prect, pitems, pcur)) = popup {
            let r = prect;
            draw_rectangle(r.x + 3.0, r.y + 3.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.24, 0.24, 0.24, 1.0));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.16));
            let m = vec2(self.inp.mx, self.inp.my);
            let items = pitems;
            let cur = pcur;
            for (i, it) in items.iter().enumerate() {
                let ir = Rect::new(r.x + 2.0, r.y + 2.0 + i as f32 * 22.0, r.w - 4.0, 22.0);
                self.rects.insert(format!("pop:{}", it), ir);
                let hot = ir.contains(m);
                if hot {
                    draw_rectangle(ir.x, ir.y, ir.w, ir.h, C_ACCENT);
                }
                if Some(i) == cur {
                    draw_circle(ir.x + 8.0, ir.y + 11.0, 2.5, C_TEXT);
                }
                let it2 = it.clone();
                self.text(&it2, ir.x + 18.0, ir.y + 4.0, 12.5, if hot { WHITE } else { C_TEXT });
                if hot && self.inp.l_pressed {
                    result = Some(i);
                    close = true;
                }
            }
            if self.inp.key(KeyCode::Escape) {
                close = true;
            }
            if let Some(i) = result {
                self.popup_result = Some((pid, i));
            }
        }
        if close {
            self.popup = None;
        }
        if let Some(t) = self.tooltip.clone() {
            let w = self.text_w(&t, 12.0) + 12.0;
            let x = (self.inp.mx + 14.0).min(self.sw - w - 4.0);
            let y = self.inp.my + 18.0;
            draw_rectangle(x, y, w, 20.0, Color::new(0.1, 0.1, 0.1, 0.95));
            self.text(&t, x + 6.0, y + 3.0, 12.0, C_TEXT);
        }
    }
}

pub fn checker(r: Rect, cell: f32) {
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.33, 0.33, 0.33, 1.0));
    let nx = (r.w / cell).ceil() as i32;
    let ny = (r.h / cell).ceil() as i32;
    for y in 0..ny {
        for x in 0..nx {
            if (x + y) % 2 == 0 {
                let cx = r.x + x as f32 * cell;
                let cy = r.y + y as f32 * cell;
                draw_rectangle(cx, cy, cell.min(r.x + r.w - cx), cell.min(r.y + r.h - cy), Color::new(0.42, 0.42, 0.42, 1.0));
            }
        }
    }
}
