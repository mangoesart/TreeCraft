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
    /// A cascading menu (open_menu) instead of a flat list: its items, the open submenu path and
    /// the submenus' boxes this frame (so clicks inside them don't close it).
    pub tree: Vec<MenuItem>,
    pub path: Vec<usize>,
    pub sub_rects: Vec<Rect>,
}

/// One entry of a cascading menu: a command, a submenu (children) or a separator line.
#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: String,
    pub children: Vec<MenuItem>,
    pub sep: bool,
    /// Shown with a tick (an option that's on, or the current choice).
    pub on: bool,
    /// Text on the right (a hot key).
    pub key: String,
}

impl MenuItem {
    pub fn leaf(s: &str) -> Self {
        MenuItem { label: s.to_string(), children: vec![], sep: false, on: false, key: String::new() }
    }
    pub fn sub(s: &str, children: Vec<MenuItem>) -> Self {
        MenuItem { label: s.to_string(), children, sep: false, on: false, key: String::new() }
    }
    pub fn sep() -> Self {
        MenuItem { label: String::new(), children: vec![], sep: true, on: false, key: String::new() }
    }
    /// An entry with a tick when `on`.
    pub fn check(s: &str, on: bool) -> Self {
        MenuItem { on, ..MenuItem::leaf(s) }
    }
    /// The same entry with a hot key shown on its right.
    pub fn hot(mut self, key: &str) -> Self {
        self.key = key.to_string();
        self
    }
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
    menu_result: Option<(u64, Vec<usize>)>,
    pub blocked: bool,
    pub clip: Option<Rect>,
    pub rects: HashMap<String, Rect>,
    pub texts: Vec<String>,
    pub tooltip: Option<String>,
    pub sw: f32,
    pub sh: f32,
    /// The number box being typed into, its text, and how a press on a number field began.
    pub edit_id: u64,
    pub edit_buf: String,
    /// The whole number is selected (just clicked or double-clicked): typing replaces it.
    pub edit_all: bool,
    press_on_box: bool,
    drag_moved: bool,
    /// The number box being typed into was drawn this frame (if not, typing stops).
    edit_seen: bool,
    /// The next number fields take any value (no lower or upper limit; picture 38).
    pub unclamped: bool,
    /// Copy / Paste on a group's header (picture 38): the rows drawn under it now are copied
    /// into the clipboard or take its values. The clipboard is the group's title and each row's
    /// key (without the panel's prefix) with its value.
    pub grp_copy: bool,
    pub grp_paste: bool,
    pub grp_clip: Option<(String, HashMap<String, f32>)>,
    /// Preferences > Property Bar > Use sliders where possible off (picture 45): number fields
    /// without the dial, just the number and its arrows.
    pub plain_numbers: bool,
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
            menu_result: None,
            blocked: false,
            clip: None,
            rects: HashMap::new(),
            texts: Vec::new(),
            tooltip: None,
            sw: 1600.0,
            sh: 900.0,
            edit_id: 0,
            edit_buf: String::new(),
            edit_all: false,
            press_on_box: false,
            drag_moved: false,
            edit_seen: false,
            unclamped: false,
            grp_copy: false,
            grp_paste: false,
            grp_clip: None,
            plain_numbers: false,
        }
    }

    /// A row under a group being copied or pasted: remember its value, or take the clipboard's.
    /// Returns true when the value was replaced.
    pub fn grp_value(&mut self, key: &str, v: &mut f32) -> bool {
        if !self.grp_copy && !self.grp_paste {
            return false;
        }
        let k = key.split_once('.').map(|(_, b)| b).unwrap_or(key).to_string();
        if self.grp_copy {
            if let Some((_, m)) = self.grp_clip.as_mut() {
                m.insert(k, *v);
            }
            return false;
        }
        if let Some(x) = self.grp_clip.as_ref().and_then(|(_, m)| m.get(&k)).copied() {
            if (x - *v).abs() > 1e-9 {
                *v = x;
                return true;
            }
        }
        false
    }

    /// Call at the start of each frame with fresh input.
    pub fn begin(&mut self, inp: Inp) {
        self.inp = inp;
        // a number box no longer shown stops being typed into
        if !self.edit_seen {
            self.edit_id = 0;
        }
        self.edit_seen = false;
        self.blocked = false;
        self.rects.clear();
        self.texts.clear();
        self.tooltip = None;
        if let Some(p) = &self.popup {
            let mp = vec2(self.inp.mx, self.inp.my);
            let inside = p.rect.contains(mp) || p.sub_rects.iter().any(|r| r.contains(mp));
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

    /// Text turned on its side, reading bottom to top, centred on (cx, cy) (picture 70: the
    /// mode tabs down a side of the Properties panel).
    pub fn text_vertical(&mut self, s: &str, cx: f32, cy: f32, size: f32, color: Color) {
        let fs = (size * self.scale).round() as u16;
        let tw = self.text_w(s, size);
        draw_text_ex(
            s,
            (cx + size * 0.32).round(),
            (cy + tw * 0.5).round(),
            TextParams { font: self.font.as_ref(), font_size: fs.max(1), font_scale: 1.0 / self.scale, color, rotation: -std::f32::consts::FRAC_PI_2, ..Default::default() },
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

    /// Whether a number box is being typed into (keyboard shortcuts wait meanwhile).
    pub fn editing(&self) -> bool {
        self.edit_id != 0
    }

    /// A number field like the original's (picture 32): a horizontal spinner dial on the left —
    /// drag it sideways to turn the value up or down (Shift: finer) — and a box on the right to
    /// type a value (click it, type, Enter; Esc cancels). Dragging the box works too. Values stay
    /// within min..max, except that ranges above 1 can be typed or turned past their top.
    pub fn slider(&mut self, key: &str, r: Rect, v: &mut f32, min: f32, max: f32, dec: usize) -> bool {
        let id = hid(key);
        self.record(&format!("sl:{}", key), r);
        let mut changed = self.grp_value(key, v);
        // unclamped fields (Sweep, First, Last ...; picture 38) take any value
        let free = self.unclamped;
        let top = if free { f32::MAX } else if max > 1.0 { max * 100.0 } else { max };
        let min_v = min;
        let min = if free { f32::MIN } else { min };
        let span = (max - min_v).max(1e-6);
        // arrows on either side of the number step the value down / up (pictures 36, 38): whole
        // numbers by 1, others by a round step about a fiftieth of the usual range
        let aw = if r.w >= 70.0 { 12.0 } else { 0.0 };
        let bw0 = if self.plain_numbers { (r.w - 2.0 * aw - 3.0).max(20.0) } else { ((r.w - 2.0 * aw) * 0.42).clamp(38.0, 84.0).min(r.w) };
        let ra = Rect::new(r.x + r.w - aw, r.y, aw, r.h);
        let la = Rect::new(ra.x - bw0 - aw - if aw > 0.0 { 2.0 } else { 0.0 }, r.y, aw, r.h);
        let step = if dec == 0 {
            1.0
        } else {
            let raw = span / 50.0;
            let mag = 10f32.powf(raw.log10().floor());
            let nice = if raw / mag < 1.5 { mag } else if raw / mag < 3.5 { 2.0 * mag } else { 5.0 * mag };
            nice.max(10f32.powi(-(dec as i32)))
        };
        if aw > 0.0 {
            for (rr, dir, k) in [(la, -1.0f32, 0), (ra, 1.0, 1)] {
                let hot_a = self.hover(rr);
                draw_rectangle(rr.x, rr.y, rr.w, rr.h, if hot_a { C_BTN_HOT } else { C_BTN });
                let (cx, cy) = (rr.x + rr.w * 0.5, rr.y + rr.h * 0.5);
                let c = if hot_a { WHITE } else { C_TEXT_DIM };
                if k == 0 {
                    draw_triangle(vec2(cx + 2.5, cy - 4.0), vec2(cx + 2.5, cy + 4.0), vec2(cx - 2.5, cy), c);
                } else {
                    draw_triangle(vec2(cx - 2.5, cy - 4.0), vec2(cx - 2.5, cy + 4.0), vec2(cx + 2.5, cy), c);
                }
                if hot_a && self.inp.l_pressed && !self.blocked && self.edit_id != id {
                    let nv = ((*v + dir * step) / step).round() * step;
                    let nv = nv.clamp(min, top);
                    if (nv - *v).abs() > 1e-9 {
                        *v = nv;
                        changed = true;
                    }
                    self.inp.l_pressed = false;
                }
            }
        }
        // the parts: dial, then the left arrow, the typing box and the right arrow
        let bw = bw0;
        let boxr = Rect::new(ra.x - bw - if aw > 0.0 { 1.0 } else { 0.0 }, r.y, bw, r.h);
        let dial_end = if aw > 0.0 { la.x - 1.0 } else { boxr.x - 2.0 };
        let dial = Rect::new(r.x, r.y, (dial_end - r.x).max(0.0), r.h);
        let hot = (self.hover(dial) || self.hover(boxr)) && !self.hover(la);
        let editing = self.edit_id == id;
        if editing {
            self.edit_seen = true;
            // typing
            for ch in self.inp.chars.clone() {
                if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' {
                    // the first key typed over a selected number replaces it (picture 35)
                    if self.edit_all {
                        self.edit_buf.clear();
                        self.edit_all = false;
                    }
                    self.edit_buf.push(ch);
                }
            }
            if self.inp.key(KeyCode::Backspace) {
                if self.edit_all {
                    self.edit_buf.clear();
                    self.edit_all = false;
                } else {
                    self.edit_buf.pop();
                }
            }
            if self.inp.key(KeyCode::Left) || self.inp.key(KeyCode::Right) || self.inp.key(KeyCode::End) {
                self.edit_all = false;
            }
            if self.inp.ctrl && self.inp.key(KeyCode::A) || (self.inp.l_double && boxr.contains(self.mouse())) {
                self.edit_all = true;
            }
            let commit = self.inp.key(KeyCode::Enter) || self.inp.key(KeyCode::KpEnter) || self.inp.key(KeyCode::Tab) || (self.inp.l_pressed && !boxr.contains(self.mouse()));
            if self.inp.key(KeyCode::Escape) {
                self.edit_id = 0;
            } else if commit {
                if let Ok(x) = self.edit_buf.trim().parse::<f32>() {
                    let nv = x.clamp(min, top);
                    if (nv - *v).abs() > 1e-9 {
                        *v = nv;
                        changed = true;
                    }
                }
                self.edit_id = 0;
            }
            self.inp.chars.clear();
            self.inp.keys.clear();
        } else if hot && self.inp.l_pressed && !self.blocked {
            self.active = id;
            self.drag_start_val = *v;
            self.drag_start_x = self.inp.mx;
            // on the box: a click without dragging starts typing
            self.press_on_box = boxr.contains(self.mouse());
            self.drag_moved = false;
        }
        if self.active == id && !editing {
            if self.inp.l_down {
                let dx = self.inp.mx - self.drag_start_x;
                if dx.abs() > 3.0 {
                    self.drag_moved = true;
                }
                if self.drag_moved || !self.press_on_box {
                    // the dial: about 300 units of mouse travel cover the usual range
                    let mut d = dx / 300.0 * span;
                    if self.inp.shift {
                        d *= 0.1;
                    }
                    let nv = (self.drag_start_val + d).clamp(min, top);
                    if (nv - *v).abs() > 1e-7 {
                        *v = nv;
                        changed = true;
                    }
                }
            } else {
                if self.press_on_box && !self.drag_moved {
                    self.edit_id = id;
                    self.edit_seen = true;
                    self.edit_buf = format!("{:.*}", dec, *v);
                    self.edit_all = true;
                }
                self.active = 0;
            }
        }
        // the dial: ribs that roll as the value changes, darker toward its edges like a cylinder
        if dial.w > 6.0 {
            draw_rectangle(dial.x, dial.y, dial.w, dial.h, C_FIELD);
            let phase = ((*v - min_v) / span * 300.0).rem_euclid(7.0);
            let mut x = dial.x - phase + 7.0;
            let on = self.active == id;
            while x < dial.x + dial.w {
                if x > dial.x + 1.0 {
                    let t = ((x - dial.x) / dial.w - 0.5).abs() * 2.0;
                    let a = (1.0 - t * t) * if on { 0.5 } else { 0.32 };
                    draw_line(x, dial.y + 3.0, x, dial.y + dial.h - 3.0, 1.0, Color::new(1.0, 1.0, 1.0, a));
                }
                x += 7.0;
            }
            for (k, a) in [(0.0, 0.35), (1.0, 0.2), (2.0, 0.1)] {
                draw_rectangle(dial.x + k * 2.0, dial.y, 2.0, dial.h, Color::new(0.0, 0.0, 0.0, a));
                draw_rectangle(dial.x + dial.w - 2.0 - k * 2.0, dial.y, 2.0, dial.h, Color::new(0.0, 0.0, 0.0, a));
            }
            // where the value sits in its usual range
            let frac = ((*v - min_v) / span).clamp(0.0, 1.0);
            let fill = if on { C_ACCENT2 } else { C_ACCENT };
            draw_rectangle(dial.x, dial.y + dial.h - 2.0, dial.w * frac, 2.0, fill);
            if self.hover(dial) || on {
                draw_rectangle_lines(dial.x, dial.y, dial.w, dial.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.25));
                self.tooltip.get_or_insert_with(|| "Drag sideways to change (Shift: finer); click the number to type".to_string());
            }
        }
        // the box
        draw_rectangle(boxr.x, boxr.y, boxr.w, boxr.h, if editing { Color::new(0.09, 0.09, 0.1, 1.0) } else { C_FIELD });
        if editing {
            draw_rectangle_lines(boxr.x, boxr.y, boxr.w, boxr.h, 1.0, C_ACCENT2);
            let s = self.edit_buf.clone();
            let w = self.text_w(&s, 12.5);
            let tx = boxr.x + boxr.w - w - 8.0;
            if self.edit_all && !s.is_empty() {
                // the selected number: highlighted
                draw_rectangle(tx - 2.0, boxr.y + 3.0, w + 4.0, boxr.h - 6.0, C_ACCENT);
            }
            self.text(&s, tx, boxr.y + (boxr.h - 12.5) * 0.5 - 1.0, 12.5, WHITE);
            if !self.edit_all && (get_time() * 2.0) as i64 % 2 == 0 {
                draw_line(boxr.x + boxr.w - 6.0, boxr.y + 3.0, boxr.x + boxr.w - 6.0, boxr.y + boxr.h - 3.0, 1.0, WHITE);
            }
        } else {
            if self.hover(boxr) {
                draw_rectangle_lines(boxr.x, boxr.y, boxr.w, boxr.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.25));
            }
            let s = format!("{:.*}", dec, *v);
            let w = self.text_w(&s, 12.5);
            self.text(&s, boxr.x + boxr.w - w - 6.0, boxr.y + (boxr.h - 12.5) * 0.5 - 1.0, 12.5, C_TEXT);
        }
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
        let mut fv = if *v { 1.0 } else { 0.0 };
        if self.grp_value(key, &mut fv) {
            *v = fv > 0.5;
            return true;
        }
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
        let mut fv = *sel as f32;
        if self.grp_value(key, &mut fv) && (fv as usize) < items.len() {
            *sel = fv as usize;
            changed = true;
        }
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
        self.popup = Some(Popup { id, rect: Rect::new(anchor.x, y, w, h), items, current, tree: vec![], path: vec![], sub_rects: vec![] });
        self.inp.l_pressed = false;
    }

    /// Open a cascading menu below `anchor`; hovering an entry with children opens its submenu.
    /// The chosen entry comes back from take_menu_result as its path of indexes.
    pub fn open_menu(&mut self, id: u64, anchor: Rect, tree: Vec<MenuItem>) {
        let (w, h) = self.menu_size(&tree, anchor.w);
        let mut y = anchor.y + anchor.h;
        if y + h > self.sh {
            y = (anchor.y - h).max(0.0);
        }
        self.popup = Some(Popup { id, rect: Rect::new(anchor.x, y, w, h), items: vec![], current: None, tree, path: vec![], sub_rects: vec![] });
        self.inp.l_pressed = false;
    }

    fn menu_size(&self, items: &[MenuItem], min_w: f32) -> (f32, f32) {
        let w = items.iter().map(|s| self.text_w(&s.label, 12.5) + 44.0 + if s.key.is_empty() { 0.0 } else { self.text_w(&s.key, 12.0) + 24.0 }).fold(min_w, f32::max);
        let h = items.iter().map(|s| if s.sep { 7.0 } else { 22.0 }).sum::<f32>() + 4.0;
        (w, h)
    }

    /// The path of the entry chosen from a cascading menu opened with `open_menu`.
    pub fn take_menu_result(&mut self, key: &str) -> Option<Vec<usize>> {
        match &self.menu_result {
            Some((rid, _)) if *rid == hid(key) => self.menu_result.take().map(|r| r.1),
            _ => None,
        }
    }

    /// Draw a cascading menu; returns (new open path, boxes of the submenus, chosen path).
    fn draw_menu(&mut self, root: Rect, tree: &[MenuItem], path: &[usize]) -> (Vec<usize>, Vec<Rect>, Option<Vec<usize>>) {
        let m = vec2(self.inp.mx, self.inp.my);
        let mut new_path = path.to_vec();
        let mut subs = vec![];
        let mut chosen = None;
        let mut items: Vec<MenuItem> = tree.to_vec();
        let mut r = root;
        for level in 0..8 {
            draw_rectangle(r.x + 3.0, r.y + 3.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.24, 0.24, 0.24, 1.0));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.16));
            let mut y = r.y + 2.0;
            let mut open_row: Option<(usize, Rect)> = None;
            for (i, it) in items.iter().enumerate() {
                if it.sep {
                    draw_line(r.x + 6.0, y + 3.5, r.x + r.w - 6.0, y + 3.5, 1.0, Color::new(1.0, 1.0, 1.0, 0.16));
                    y += 7.0;
                    continue;
                }
                let ir = Rect::new(r.x + 2.0, y, r.w - 4.0, 22.0);
                y += 22.0;
                self.rects.insert(format!("pop:{}", it.label), ir);
                let hot = ir.contains(m);
                let open = path.get(level) == Some(&i) && !it.children.is_empty();
                if hot {
                    draw_rectangle(ir.x, ir.y, ir.w, ir.h, C_ACCENT);
                } else if open {
                    draw_rectangle(ir.x, ir.y, ir.w, ir.h, Color::new(1.0, 1.0, 1.0, 0.1));
                }
                if it.on {
                    // a tick
                    let (tx, ty) = (ir.x + 5.0, ir.y + 11.0);
                    let c = if hot { WHITE } else { C_TEXT };
                    draw_line(tx, ty, tx + 3.0, ty + 3.5, 1.6, c);
                    draw_line(tx + 3.0, ty + 3.5, tx + 9.0, ty - 4.0, 1.6, c);
                }
                self.text(&it.label, ir.x + 18.0, ir.y + 4.0, 12.5, if hot { WHITE } else { C_TEXT });
                if !it.key.is_empty() {
                    let kw = self.text_w(&it.key, 12.0);
                    self.text(&it.key, ir.x + ir.w - kw - 20.0, ir.y + 4.5, 12.0, if hot { WHITE } else { C_TEXT_DIM });
                }
                if !it.children.is_empty() {
                    let (ax, ay) = (ir.x + ir.w - 12.0, ir.y + 11.0);
                    draw_triangle(vec2(ax, ay - 4.0), vec2(ax, ay + 4.0), vec2(ax + 5.0, ay), if hot { WHITE } else { C_TEXT_DIM });
                }
                if hot {
                    new_path.truncate(level);
                    if !it.children.is_empty() {
                        new_path.push(i);
                    } else if self.inp.l_pressed {
                        let mut p = path[..level.min(path.len())].to_vec();
                        p.push(i);
                        chosen = Some(p);
                    }
                }
                if open {
                    open_row = Some((i, ir));
                }
            }
            let Some((k, row)) = open_row else { break };
            let children = items[k].children.clone();
            let (w, h) = self.menu_size(&children, 120.0);
            let mut x = r.x + r.w - 2.0;
            if x + w > self.sw {
                x = (r.x - w + 2.0).max(0.0);
            }
            let ny = (row.y - 2.0).min(self.sh - h).max(0.0);
            r = Rect::new(x, ny, w, h);
            subs.push(r);
            items = children;
        }
        (new_path, subs, chosen)
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
        let tree = self.popup.as_ref().filter(|p| !p.tree.is_empty()).map(|p| (p.id, p.rect, p.tree.clone(), p.path.clone()));
        if let Some((pid, prect, tree, path)) = tree {
            let (np, subs, chosen) = self.draw_menu(prect, &tree, &path);
            if let Some(p) = self.popup.as_mut() {
                p.path = np;
                p.sub_rects = subs;
            }
            if let Some(c) = chosen {
                self.menu_result = Some((pid, c));
                close = true;
            }
            if self.inp.key(KeyCode::Escape) {
                close = true;
            }
        }
        let popup = self.popup.as_ref().filter(|p| p.tree.is_empty()).map(|p| (p.id, p.rect, p.items.clone(), p.current));
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

/// A filled rectangle with rounded corners.
pub fn draw_round_rect(r: Rect, rad: f32, c: Color) {
    let rad = rad.min(r.w * 0.5).min(r.h * 0.5).max(0.0);
    draw_rectangle(r.x + rad, r.y, r.w - 2.0 * rad, r.h, c);
    draw_rectangle(r.x, r.y + rad, rad, r.h - 2.0 * rad, c);
    draw_rectangle(r.x + r.w - rad, r.y + rad, rad, r.h - 2.0 * rad, c);
    for (cx, cy) in [(r.x + rad, r.y + rad), (r.x + r.w - rad, r.y + rad), (r.x + rad, r.y + r.h - rad), (r.x + r.w - rad, r.y + r.h - rad)] {
        draw_circle(cx, cy, rad, c);
    }
}

/// The outline of a rectangle with rounded corners.
pub fn draw_round_rect_lines(r: Rect, rad: f32, t: f32, c: Color) {
    let rad = rad.min(r.w * 0.5).min(r.h * 0.5).max(0.0);
    draw_line(r.x + rad, r.y, r.x + r.w - rad, r.y, t, c);
    draw_line(r.x + rad, r.y + r.h, r.x + r.w - rad, r.y + r.h, t, c);
    draw_line(r.x, r.y + rad, r.x, r.y + r.h - rad, t, c);
    draw_line(r.x + r.w, r.y + rad, r.x + r.w, r.y + r.h - rad, t, c);
    let seg = 6;
    for (cx, cy, a0) in [(r.x + rad, r.y + rad, 180.0f32), (r.x + r.w - rad, r.y + rad, 270.0), (r.x + r.w - rad, r.y + r.h - rad, 0.0), (r.x + rad, r.y + r.h - rad, 90.0)] {
        for i in 0..seg {
            let (t0, t1) = ((a0 + 90.0 * i as f32 / seg as f32).to_radians(), (a0 + 90.0 * (i + 1) as f32 / seg as f32).to_radians());
            draw_line(cx + rad * t0.cos(), cy + rad * t0.sin(), cx + rad * t1.cos(), cy + rad * t1.sin(), t, c);
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
