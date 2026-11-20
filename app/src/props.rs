//! The Properties panel: rows of labelled controls, grouped like the original's property editor.
use crate::app::{App, Sel};
use crate::model::*;
use crate::model::Material;
use crate::textures;
use crate::ui::*;
use macroquad::prelude::*;

pub struct Rows<'a> {
    pub ui: &'a mut Ui,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub changed: bool,
    pub prefix: String,
    pub closed: &'a mut std::collections::HashSet<String>,
    open: bool,
    /// The curves the shown generator has ("<property>|<kind>"), and the curve button clicked.
    pub curve_keys: std::collections::HashSet<String>,
    pub curve_req: Option<(String, u8)>,
    /// Each property's variance amount (for the +/- buttons) and the one clicked (picture 35).
    pub var_amounts: std::collections::HashMap<String, f32>,
    pub var_req: Option<(String, Rect)>,
    /// The UV preview's settings and pictures for the shown branch generator (picture 25).
    pub uv: Option<UvView>,
    /// Preferences > Property Bar (picture 45): Use group stacking (opening a group closes the
    /// others), Group click (0 Slams to top, 1 Reveals), the group a click opened last frame to
    /// bring into view, the panel's visible top and bottom, and what this frame asks for.
    pub stack: bool,
    pub click_mode: usize,
    pub focus: Option<String>,
    pub view: (f32, f32),
    pub opened: Option<String>,
    pub scroll_req: Option<f32>,
    seen: Vec<String>,
    focus_start: Option<f32>,
    /// The sub-tab picked inside each main tab that has them (pictures 61, 62), by
    /// "branch:<tab>" / "leaf:<tab>", in and out.
    pub sub_tabs: std::collections::HashMap<String, usize>,
}

/// What the UV tab's preview needs: show the picture or not, which one (0 Diffuse, 1 Normal,
/// 2 Opacity), the pictures of the generator's material, and a typical branch's length.
pub struct UvView {
    pub show: bool,
    /// Repeat the picture over the whole preview (off: one copy over UV 0..1; picture 28).
    pub tile: bool,
    pub slot: usize,
    pub tex: [Option<Texture2D>; 3],
    pub length: f32,
    /// Wheel zoom, middle-drag pan, height (0 = automatic), the edge or pan being dragged
    /// (1 top, 2 bottom, 3 pan), the mouse last frame, and where it was drawn (picture 26).
    pub zoom: f32,
    pub pan: Vec2,
    pub height: f32,
    pub drag: u8,
    pub last_mouse: Vec2,
    pub rect: Option<Rect>,
    /// How far the panel should scroll so a dragged top edge follows the mouse.
    pub scroll_by: f32,
}

const RH: f32 = 24.0;

impl Drop for Rows<'_> {
    /// The panel is finished: nothing after it is copied or pasted.
    fn drop(&mut self) {
        self.ui.grp_copy = false;
        self.ui.grp_paste = false;
    }
}

impl<'a> Rows<'a> {
    fn lw(&self) -> f32 {
        (self.w * 0.46).floor()
    }
    fn key(&self, k: &str) -> String {
        format!("{}.{}", self.prefix, k)
    }
    /// A group that a click opened last frame has been drawn to here: scroll to it (Group click).
    fn end_focus(&mut self) {
        let Some(start) = self.focus_start.take() else { return };
        let (top, bottom) = self.view;
        let d = if self.click_mode == 0 {
            // Slams to top: its header at the top of the panel
            start - top - 4.0
        } else if self.y > bottom {
            // Reveals: just enough to show all of it, the header kept in view
            (self.y - bottom + 8.0).min(start - top - 4.0)
        } else if start < top {
            start - top - 4.0
        } else {
            0.0
        };
        if d.abs() > 0.5 {
            self.scroll_req = Some(d);
        }
    }

    /// The panel is drawn: scroll requests settled, and with group stacking every other group
    /// closed after one was opened.
    pub fn finish(&mut self) {
        self.end_focus();
        if let (true, Some(k)) = (self.stack, self.opened.clone()) {
            for g in self.seen.clone() {
                if g != k {
                    self.closed.insert(g);
                }
            }
        }
    }

    pub fn group(&mut self, title: &str) -> bool {
        // the group before this one has ended: its rows are no longer copied or pasted
        self.ui.grp_copy = false;
        self.ui.grp_paste = false;
        self.end_focus();
        let key = self.key(&format!("grp.{}", title));
        self.seen.push(key.clone());
        let r = Rect::new(self.x - 6.0, self.y + 4.0, self.w + 12.0, 24.0);
        self.ui.record(&format!("grp:{}", title), r);
        let open = !self.closed.contains(&key);
        // Copy and Paste on the right of the header (picture 38): copy every value in the group,
        // paste them into the same group of another generator, material or mesh
        let pb = Rect::new(r.x + r.w - 28.0, r.y + 2.0, 24.0, 20.0);
        let cb = Rect::new(pb.x - 27.0, r.y + 2.0, 24.0, 20.0);
        let can_paste = self.ui.grp_clip.as_ref().map_or(false, |(g, _)| g == title);
        let (copy_click, paste_click) = if open { self.copy_paste_buttons(title, cb, pb, can_paste) } else { (false, false) };
        if copy_click {
            self.ui.grp_clip = Some((title.to_string(), Default::default()));
            self.ui.grp_copy = true;
        } else if paste_click {
            self.ui.grp_paste = true;
        }
        let hot = self.ui.hover(r) && !cb.contains(self.ui.mouse()) && !pb.contains(self.ui.mouse());
        draw_rectangle(r.x, r.y, r.w, r.h, if hot { Color::new(1.0, 1.0, 1.0, 0.08) } else { Color::new(1.0, 1.0, 1.0, 0.04) });
        let ax = r.x + 12.0;
        let ay = r.y + 12.0;
        if open {
            draw_triangle(vec2(ax - 4.0, ay - 2.0), vec2(ax + 4.0, ay - 2.0), vec2(ax, ay + 3.0), C_TEXT_DIM);
        } else {
            draw_triangle(vec2(ax - 2.0, ay - 4.0), vec2(ax - 2.0, ay + 4.0), vec2(ax + 3.0, ay), C_TEXT_DIM);
        }
        self.ui.text(title, r.x + 22.0, r.y + 5.0, 13.5, WHITE);
        if hot && self.ui.inp.l_pressed {
            if open {
                self.closed.insert(key.clone());
            } else {
                self.closed.remove(&key);
                self.opened = Some(key.clone());
            }
        }
        if open && self.focus.as_deref() == Some(key.as_str()) {
            self.focus_start = Some(r.y);
        }
        self.y += 32.0;
        self.open = open;
        open
    }
    /// The header's two small buttons: Copy (two sheets) and Paste (a clipboard). Paste is dim
    /// until this group has been copied. Returns which was clicked.
    fn copy_paste_buttons(&mut self, title: &str, cb: Rect, pb: Rect, can_paste: bool) -> (bool, bool) {
        let m = self.ui.mouse();
        let mut out = (false, false);
        for (k, b) in [(0, cb), (1, pb)] {
            let enabled = k == 0 || can_paste;
            let hot = enabled && self.ui.hover(b);
            self.ui.record(&format!("grp{}:{}", if k == 0 { "copy" } else { "paste" }, title), b);
            draw_round_rect(b, 3.0, if hot { C_BTN_HOT } else { Color::new(1.0, 1.0, 1.0, 0.05) });
            let c = if !enabled { C_TEXT_FAINT } else if hot { WHITE } else { C_TEXT_DIM };
            let (x, y) = (b.x + 7.0, b.y + 3.0);
            if k == 0 {
                // two sheets, one over the other
                draw_rectangle_lines(x + 3.0, y, 8.0, 10.0, 1.2, c);
                draw_rectangle(x, y + 3.5, 8.0, 10.0, if hot { C_BTN_HOT } else { C_PANEL2 });
                draw_rectangle_lines(x, y + 3.5, 8.0, 10.0, 1.2, c);
            } else {
                // a clipboard with its clip
                draw_rectangle_lines(x, y + 1.5, 10.0, 12.0, 1.2, c);
                draw_rectangle(x + 2.5, y, 5.0, 3.0, c);
                draw_line(x + 2.5, y + 7.0, x + 7.5, y + 7.0, 1.0, c);
                draw_line(x + 2.5, y + 10.0, x + 7.5, y + 10.0, 1.0, c);
            }
            if b.contains(m) {
                self.ui.tooltip = Some(if k == 0 {
                    format!("Copy every value in {}", title)
                } else if can_paste {
                    format!("Paste the copied {} values here", title)
                } else {
                    format!("Paste: copy a {} group first", title)
                });
            }
            if hot && self.ui.inp.l_pressed {
                self.ui.inp.l_pressed = false;
                if k == 0 {
                    out.0 = true;
                } else {
                    out.1 = true;
                }
            }
        }
        out
    }
    /// A row's name, right-aligned against its control as the original lays them out
    /// (picture 57); group and section headings keep their own alignment.
    fn label(&mut self, label: &str) -> Rect {
        let lw = self.lw();
        let tw = self.ui.text_w(label, 12.5).min(lw - 14.0);
        self.ui.text_fit(label, self.x + lw - 8.0 - tw, self.y + 5.0, lw - 14.0, 12.5, C_TEXT_DIM);
        Rect::new(self.x + lw, self.y + 2.0, self.w - lw, RH - 4.0)
    }
    pub fn f(&mut self, k: &str, label: &str, v: &mut f32, min: f32, max: f32, dec: usize) {
        let r = self.label(label);
        let key = self.key(k);
        if self.ui.slider(&key, r, v, min, max, dec) {
            self.changed = true;
        }
        self.y += RH;
    }
    pub fn i(&mut self, k: &str, label: &str, v: &mut i32, min: i32, max: i32) {
        let r = self.label(label);
        let key = self.key(k);
        if self.ui.islider(&key, r, v, min, max) {
            self.changed = true;
        }
        self.y += RH;
    }
    pub fn b(&mut self, k: &str, label: &str, v: &mut bool) {
        let r = self.label(label);
        let key = self.key(k);
        if self.ui.checkbox(&key, r, v) {
            self.changed = true;
        }
        self.y += RH;
    }
    pub fn combo(&mut self, k: &str, label: &str, items: &[String], v: &mut usize) {
        let r = self.label(label);
        let key = self.key(k);
        if self.ui.combo(&key, r, items, v) {
            self.changed = true;
        }
        self.y += RH;
    }
    pub fn color(&mut self, k: &str, label: &str, c: &mut [f32; 3]) {
        let r = self.label(label);
        draw_rectangle(r.x, r.y, 26.0, r.h, Color::new(c[0], c[1], c[2], 1.0));
        draw_rectangle_lines(r.x, r.y, 26.0, r.h, 1.0, C_LINE);
        let sw = (r.w - 32.0) / 3.0;
        let names = ["r", "g", "b"];
        for i in 0..3 {
            let rr = Rect::new(r.x + 30.0 + i as f32 * (sw + 1.0), r.y, sw - 1.0, r.h);
            let key = self.key(&format!("{}.{}", k, names[i]));
            if self.ui.slider(&key, rr, &mut c[i], 0.0, 1.0, 2) {
                self.changed = true;
            }
        }
        self.y += RH;
    }
    pub fn button(&mut self, k: &str, label: &str) -> bool {
        let r = Rect::new(self.x + self.lw(), self.y + 2.0, self.w - self.lw(), RH - 4.0);
        let key = self.key(k);
        let c = self.ui.button(&key, r, label);
        self.y += RH;
        c
    }
    pub fn info(&mut self, label: &str, value: &str) {
        let r = self.label(label);
        self.ui.text_fit(value, r.x + 4.0, r.y + 3.0, r.w - 6.0, 12.5, C_TEXT);
        self.y += RH;
    }
    /// A section inside a group (U, V, Absolute ...; picture 32): a band with an accent mark, so
    /// the rows under it read as one set.
    pub fn section(&mut self, text: &str) {
        self.y += 6.0;
        let r = Rect::new(self.x - 2.0, self.y, self.w + 4.0, 22.0);
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.22));
        draw_rectangle(r.x, r.y, 3.0, r.h, C_ACCENT);
        self.ui.text_fit(text, r.x + 10.0, r.y + 4.0, r.w - 14.0, 13.0, WHITE);
        self.y += 26.0;
    }
    /// A heading for a part inside a section (Fractional, Twist ...), indented under it.
    pub fn subheading_in(&mut self, text: &str) {
        self.ui.text_fit(text, self.x + 14.0, self.y + 5.0, self.w - 16.0, 12.0, C_TEXT);
        draw_line(self.x + 14.0, self.y + 20.0, self.x + self.w, self.y + 20.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.06));
        self.y += 23.0;
    }
    /// A small heading inside a group (the original's sub-groups such as "Absolute" or "Shared").
    pub fn subheading(&mut self, text: &str) {
        self.ui.text_fit(text, self.x + 2.0, self.y + 6.0, self.w - 4.0, 12.0, C_TEXT);
        draw_line(self.x + 2.0, self.y + 21.0, self.x + self.w, self.y + 21.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.06));
        self.y += 24.0;
    }
    fn faint_label(&mut self, label: &str) -> Rect {
        let lw = self.lw();
        let tw = self.ui.text_w(label, 12.5).min(lw - 14.0);
        self.ui.text_fit(label, self.x + lw - 8.0 - tw, self.y + 5.0, lw - 14.0, 12.5, C_TEXT_FAINT);
        Rect::new(self.x + lw, self.y + 2.0, self.w - lw, RH - 4.0)
    }
    /// Rows for settings kept but not used by the tree yet: faint names; return true when changed.
    pub fn f_faint(&mut self, k: &str, label: &str, v: &mut f32, min: f32, max: f32, dec: usize) -> bool {
        let r = self.faint_label(label);
        let key = self.key(k);
        let c = self.ui.slider(&key, r, v, min, max, dec);
        self.changed |= c;
        self.y += RH;
        c
    }
    pub fn i_faint(&mut self, k: &str, label: &str, v: &mut i32, min: i32, max: i32) -> bool {
        let r = self.faint_label(label);
        let key = self.key(k);
        let c = self.ui.islider(&key, r, v, min, max);
        self.changed |= c;
        self.y += RH;
        c
    }
    pub fn b_faint(&mut self, k: &str, label: &str, v: &mut bool) -> bool {
        let r = self.faint_label(label);
        let key = self.key(k);
        let c = self.ui.checkbox(&key, r, v);
        self.changed |= c;
        self.y += RH;
        c
    }
    pub fn note(&mut self, text: &str) {
        self.ui.text_fit(text, self.x + 6.0, self.y + 4.0, self.w - 8.0, 12.0, C_TEXT_FAINT);
        self.y += 20.0;
    }

    /// A number row with the original's curve buttons on its right (picture 24): green Parent,
    /// blue Distribution, yellow Profile; lit when the property has that curve.
    pub fn with_curves(&mut self, prop: &str, kinds: [bool; 3], var: bool, draw: impl FnOnce(&mut Rows)) {
        const CW: f32 = 15.0;
        const VW: f32 = 26.0;
        let full = self.w;
        self.w = full - (CW + 3.0) * 3.0 - 2.0 - VW - 3.0;
        draw(self);
        self.w = full;
        let y = self.y - RH + 4.0;
        // the +/- button opening Variance Settings (picture 35): pink with the amount when set
        if var {
            let vr = Rect::new(self.x + full - (CW + 3.0) * 3.0 - VW - 2.0, y - 1.0, VW, CW + 2.0);
            let amount = self.var_amounts.get(prop).copied().unwrap_or(0.0);
            let hot = self.ui.hover(vr);
            let set = amount.abs() > 1e-6;
            draw_rectangle(vr.x, vr.y, vr.w, vr.h, if set { Color::new(0.78, 0.36, 0.55, 1.0) } else if hot { C_BTN_HOT } else { C_BTN });
            let label = if set { let s = format!("{:.2}", amount); s.trim_start_matches('0').to_string() } else { "+/-".to_string() };
            self.ui.text_center(&label, vr, 10.5, if set { WHITE } else { C_TEXT });
            self.ui.record(&format!("var:{}", prop), vr);
            if hot {
                self.ui.tooltip = Some("Variance Settings: how much each node's value may vary".into());
                if self.ui.inp.l_pressed && !self.ui.blocked && self.ui.popup.is_none() {
                    self.var_req = Some((prop.to_string(), vr));
                }
            }
        }
        for k in 0..3u8 {
            // only the curves the original gives this property (stcurves.rs, picture 34)
            if !kinds[k as usize] {
                continue;
            }
            let r = Rect::new(self.x + full - (CW + 3.0) * (3 - k) as f32 + 3.0, y, CW, CW);
            // green Parent; the second curve blue or yellow as the original colours it
            let col = match k {
                0 => Color::new(0.35, 0.80, 0.35, 1.0),
                1 => Color::new(0.30, 0.55, 1.0, 1.0),
                _ => if crate::stcurves::second_is_yellow(prop) { Color::new(0.98, 0.82, 0.25, 1.0) } else { Color::new(0.30, 0.55, 1.0, 1.0) },
            };
            let on = self.curve_keys.contains(&format!("{}|{}", prop, k));
            let hot = self.ui.hover(r);
            draw_rectangle(r.x, r.y, r.w, r.h, if on { col } else { Color::new(0.13, 0.13, 0.14, 1.0) });
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, if hot { WHITE } else { Color::new(col.r, col.g, col.b, 0.8) });
            // a little curve inside
            let lc = if on { Color::new(0.05, 0.05, 0.05, 1.0) } else { col };
            let pts = [(0.15, 0.75), (0.4, 0.62), (0.6, 0.35), (0.85, 0.25)];
            for w in pts.windows(2) {
                draw_line(r.x + r.w * w[0].0, r.y + r.h * w[0].1, r.x + r.w * w[1].0, r.y + r.h * w[1].1, 1.2, lc);
            }
            self.ui.record(&format!("curve:{}:{}", prop, k), r);
            if hot {
                self.ui.tooltip = Some(format!("{} curve{}", crate::model::CURVE_KINDS[k as usize], if on { " (set) — click to edit" } else { " — click to add" }));
                if self.ui.inp.l_pressed && !self.ui.blocked && self.ui.popup.is_none() {
                    self.curve_req = Some((prop.to_string(), k));
                }
            }
        }
    }
}

fn mode_rows(r: &mut Rows, p: &mut Params, on_ground: bool) {
    let modes: Vec<String> = GEN_MODES.iter().map(|s| s.to_string()).collect();
    r.combo("gen_mode", "Mode", &modes, &mut p.gen_mode);
    // Controls per mode follow the original's property names (Generation:<Mode>:...).
    match p.gen_mode {
        MODE_ABSOLUTE => {
            r.i("number", "Number", &mut p.number, 0, 60);
            r.f("jumble", "Jumble", &mut p.jumble, 0.0, 1.0, 2);
        }
        MODE_INTERVAL | MODE_PHYLLOTAXY if on_ground => {
            r.i("number", "Number", &mut p.number, 0, 60);
        }
        MODE_INTERVAL => {
            r.f("interval", "Frequency (per 10 units)", &mut p.interval, 0.05, 20.0, 2);
            r.i("per_node", "Count", &mut p.per_node, 1, 12);
            r.f("jumble", "Spread", &mut p.jumble, 0.0, 1.0, 2);
            r.f("spiral", "Spiral", &mut p.spiral, -10.0, 10.0, 2);
            r.i("limit", "Limit", &mut p.limit, 1, 1000);
        }
        MODE_PHYLLOTAXY => {
            r.f("interval", "Internode length", &mut p.interval, 0.1, 20.0, 2);
            r.i("per_node", "Children per node", &mut p.per_node, 1, 12);
            r.f("jumble", "Spread", &mut p.jumble, 0.0, 1.0, 2);
            r.f("spiral", "Spiral", &mut p.spiral, -10.0, 10.0, 2);
            r.i("limit", "Limit", &mut p.limit, 1, 1000);
        }
        _ => {
            let styles: Vec<String> = STYLES.iter().map(|s| s.to_string()).collect();
            let mut st = if p.relative { 1 } else { 0 };
            r.combo("style", "Style", &styles, &mut st);
            p.relative = st == 1;
            let label = if p.relative { "Frequency (per 40 units)" } else { "Frequency" };
            r.f("frequency", label, &mut p.frequency, 0.0, 120.0, 1);
        }
    }
    if !on_ground {
        r.f("first", "First", &mut p.first, 0.0, 1.0, 3);
        r.f("last", "Last", &mut p.last, 0.0, 1.0, 3);
    }
    if p.gen_mode == MODE_CLASSIC || on_ground {
        r.b("balanced", "Orientation: Balanced", &mut p.balanced);
        r.f("offset", "Orientation: Offset", &mut p.offset, 0.0, 1.0, 3);
    } else if p.gen_mode != MODE_ABSOLUTE {
        r.b("balanced", "Balance (stagger nodes)", &mut p.balanced);
    }
    r.f("sweep", "Orientation: Sweep", &mut p.sweep, 0.0, 1.0, 3);
    r.f("roll", "Orientation: Roll", &mut p.roll, -1.0, 1.0, 3);
    r.f("size_scalar", "Size scalar", &mut p.size_scalar, 0.1, 3.0, 2);
}

/// A material list with "None" first (a generator without a material draws plain grey).
/// The UV tab's preview (picture 25; the original has none): one typical branch's bark unrolled —
/// across is once around the branch (U repeats), down is along it (V) — with the chosen picture
/// tiled underneath. It follows every UV change at once.
fn uv_preview(r: &mut Rows, p: &mut Params) {
    let Some(mut uv) = r.uv.take() else { return };
    r.subheading("UV preview");
    let names: Vec<String> = ["Diffuse", "Normal", "Opacity"].iter().map(|s| s.to_string()).collect();
    r.b("uv.show", "Show texture", &mut uv.show);
    r.b("uv.tile", "Tile texture", &mut uv.tile);
    r.combo("uv.slot", "Texture", &names, &mut uv.slot);
    let (x, w) = (r.x + 2.0, r.w - 4.0);
    // its height: dragged by the top or bottom edge (picture 26); the width follows the panel
    if uv.height <= 0.0 {
        uv.height = (w * 0.8).min(320.0);
    }
    uv.height = uv.height.clamp(80.0, 1200.0);
    let area = Rect::new(x, r.y + 6.0, w, uv.height);
    // the unrolled branch: U from 0 to its U repeats, V from 0 to length / circumference x V scale
    let r0 = (p.radius_abs * p.size_scalar).max(0.01);
    // the same UV map the bark uses (gen.rs UvMap)
    let uvm = crate::gen::UvMap::new(p, r0, uv.length.max(0.01), false);
    let (u_max, v_max) = uvm.extent();
    let (u_max, v_max) = (u_max.max(0.05), v_max.max(0.05));
    // tiled: the whole island fits; not tiled: the single picture (UV 0..1) fills the width so it
    // is big enough to see under the island (picture 29); the rest of the island pans into view
    let extent = if uv.tile { u_max.max(v_max).max(1.0) } else { u_max.max(1.0) };
    // UV units -> panel units: fitted, then the wheel zoom and the middle-drag pan (picture 26)
    let s = area.w.min(area.h) / extent * uv.zoom;
    let origin = vec2(area.x + (area.w - area.w.min(area.h)) * 0.5, area.y) + uv.pan;
    let m = r.ui.mouse();
    let inp = r.ui.inp.clone();
    let free = !r.ui.blocked && r.ui.popup.is_none();
    let top_edge = Rect::new(area.x, area.y - 5.0, area.w, 9.0);
    let bottom_edge = Rect::new(area.x, area.y + area.h - 4.0, area.w, 9.0);
    if free && uv.drag == 0 {
        if (top_edge.contains(m) || bottom_edge.contains(m)) && r.ui.active == 0 {
            r.ui.tooltip = Some("Drag to make the preview taller or shorter".into());
            if inp.l_pressed {
                uv.drag = if top_edge.contains(m) { 1 } else { 2 };
                r.ui.active = hid("uv.preview.drag");
            }
        } else if area.contains(m) {
            if inp.wheel != 0.0 {
                // wheel up zooms in, about the point under the mouse
                let zn = (uv.zoom * 1.15f32.powf(inp.wheel)).clamp(0.2, 30.0);
                let base = vec2(area.x + (area.w - area.w.min(area.h)) * 0.5, area.y);
                uv.pan = m - base - (m - base - uv.pan) * (zn / uv.zoom);
                uv.zoom = zn;
            }
            if inp.m_pressed {
                uv.drag = 3;
            }
            if inp.l_double {
                uv.zoom = 1.0;
                uv.pan = Vec2::ZERO;
            }
        }
    }
    let d = m - uv.last_mouse;
    match uv.drag {
        1 if inp.l_down => {
            // the top edge follows the mouse: taller by what it moved up, and the panel scrolls
            // the same amount so the bottom edge stays where it is
            let before = uv.height;
            uv.height = (uv.height - d.y).clamp(80.0, 1200.0);
            uv.scroll_by += uv.height - before;
        }
        2 if inp.l_down => uv.height = (uv.height + d.y).clamp(80.0, 1200.0),
        3 if inp.m_down => uv.pan += d,
        0 => {}
        _ => {
            if uv.drag != 3 {
                r.ui.active = 0;
            }
            uv.drag = 0;
        }
    }
    uv.last_mouse = m;
    // everything below is drawn inside the area only
    let prev_clip = r.ui.clip;
    let clip = match prev_clip {
        Some(c) => {
            let x0 = c.x.max(area.x);
            let y0 = c.y.max(area.y);
            let x1 = (c.x + c.w).min(area.x + area.w);
            let y1 = (c.y + c.h).min(area.y + area.h);
            Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
        }
        None => area,
    };
    r.ui.set_clip(Some(clip));
    checker(area, 8.0);
    if uv.show {
        if let Some(t) = uv.tex.get(uv.slot).cloned().flatten() {
            // one picture per UV unit over the whole visible area
            // tiled: every UV unit in view; not tiled (the default): the one copy over 0..1
            let (i0, i1, j0, j1) = if uv.tile {
                (
                    ((area.x - origin.x) / s).floor() as i32,
                    ((area.x + area.w - origin.x) / s).ceil() as i32,
                    ((area.y - origin.y) / s).floor() as i32,
                    ((area.y + area.h - origin.y) / s).ceil() as i32,
                )
            } else {
                (0, 1, 0, 1)
            };
            if (i1 - i0) * (j1 - j0) < 4000 {
                for i in i0..i1 {
                    for j in j0..j1 {
                        draw_texture_ex(&t, origin.x + i as f32 * s, origin.y + j as f32 * s, WHITE, DrawTextureParams { dest_size: Some(vec2(s, s)), ..Default::default() });
                    }
                }
            }
        } else {
            r.ui.text_fit("No picture: assign a material with this texture", area.x + 8.0, area.y + area.h - 20.0, area.w - 16.0, 11.5, C_TEXT_DIM);
        }
    }
    // the branch's UV island: a column per radial segment, a row per length segment
    let col = Color::new(1.0, 0.85, 0.25, 0.9);
    let radial = p.seg_radial.clamp(3, 48);
    let rows_n = p.seg_len.clamp(2, 64);
    // the island as the bark really maps: every grid point through the same UV map (offsets,
    // twist, V style and anchor all show)
    let to_px = |q: Vec2| origin + q * s;
    let len = uv.length.max(0.01);
    for j in 0..=radial {
        let a = j as f32 / radial as f32;
        for k in 0..rows_n {
            let (d0, d1) = (len * k as f32 / rows_n as f32, len * (k + 1) as f32 / rows_n as f32);
            let (p0, p1) = (to_px(uvm.uv(a, d0)), to_px(uvm.uv(a, d1)));
            draw_line(p0.x, p0.y, p1.x, p1.y, 1.0, col);
        }
    }
    for k in 0..=rows_n {
        let d = len * k as f32 / rows_n as f32;
        let (p0, p1) = (to_px(uvm.uv(0.0, d)), to_px(uvm.uv(1.0, d)));
        draw_line(p0.x, p0.y, p1.x, p1.y, 1.0, col);
    }
    r.ui.set_clip(prev_clip);
    // the edges that resize it
    let hot_top = top_edge.contains(m) || uv.drag == 1;
    let hot_bottom = bottom_edge.contains(m) || uv.drag == 2;
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 1.0, C_LINE);
    for (hot, yy) in [(hot_top, area.y), (hot_bottom, area.y + area.h)] {
        let c = if hot { C_ACCENT } else { Color::new(1.0, 1.0, 1.0, 0.25) };
        draw_line(area.x + area.w * 0.42, yy, area.x + area.w * 0.58, yy, if hot { 3.0 } else { 2.0 }, c);
    }
    r.ui.record("uv.preview", area);
    uv.rect = Some(area);
    r.y = area.y + area.h + 6.0;
    r.ui.text_fit(&format!("U {:.2} around  •  V {:.2} along  •  wheel zoom, middle-drag pan, double-click reset", u_max, v_max), r.x + 4.0, r.y, r.w - 8.0, 11.0, C_TEXT_FAINT);
    r.y += 18.0;
    r.uv = Some(uv);
}

/// A Materials list like the original's (picture 25): a title with - / + on the right; each entry
/// has a remove box, a Material choice ("Inherited" = the parent's, first) and a Weight.
fn material_list(r: &mut Rows, key: &str, title: &str, mat_names: &[String], list: &mut Vec<(usize, f32)>) {
    let (x, w) = (r.x, r.w);
    r.ui.text(title, x + 2.0, r.y + 5.0, 12.5, C_TEXT);
    let minus = Rect::new(x + w - 46.0, r.y + 2.0, 20.0, 19.0);
    let plus = Rect::new(x + w - 22.0, r.y + 2.0, 20.0, 19.0);
    if r.ui.button(&format!("{}.{}.minus", r.prefix, key), minus, "-") && !list.is_empty() {
        list.pop();
        r.changed = true;
    }
    if r.ui.button(&format!("{}.{}.plus", r.prefix, key), plus, "+") {
        list.push((INHERITED, 1.0));
        r.changed = true;
    }
    draw_line(x + 2.0, r.y + 23.0, x + w, r.y + 23.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.06));
    r.y += 26.0;
    if list.is_empty() {
        r.note(if title == "Branch" { "No material: plain grey. Press + to add one." } else { "None (TreeCraft has no branch extensions yet)." });
        return;
    }
    let mut items = vec!["Inherited".to_string()];
    items.extend(mat_names.iter().cloned());
    let mut remove = None;
    for (i, (m, wt)) in list.iter_mut().enumerate() {
        let xb = Rect::new(x + w - 18.0, r.y + 3.0, 17.0, 18.0);
        if r.ui.button(&format!("{}.{}.x{}", r.prefix, key, i), xb, "x") {
            remove = Some(i);
        }
        let mut sel = if *m == INHERITED || *m >= mat_names.len() { 0 } else { *m + 1 };
        let before = sel;
        r.w = w - 22.0;
        r.combo(&format!("{}.m{}", key, i), "Material", &items, &mut sel);
        r.w = w;
        if sel != before {
            *m = if sel == 0 { INHERITED } else { sel - 1 };
        }
        r.f(&format!("{}.w{}", key, i), "Weight", wt, 0.0, 1.0, 2);
        r.y += 4.0;
    }
    if let Some(i) = remove {
        list.remove(i);
        r.changed = true;
    }
}

#[allow(dead_code)]
fn material_choice(r: &mut Rows, key: &str, label: &str, mat_names: &[String], v: &mut usize) {
    let mut items = vec!["None".to_string()];
    items.extend(mat_names.iter().cloned());
    let mut i = if *v == NO_MATERIAL || *v >= mat_names.len() { 0 } else { *v + 1 };
    r.combo(key, label, &items, &mut i);
    *v = if i == 0 { NO_MATERIAL } else { i - 1 };
}

fn seed_rows(r: &mut Rows, p: &mut Params) {
    r.i("seed", "Generation seed", &mut p.seed, 0, 9999);
    if r.button("rand", "Randomize") {
        p.seed = (macroquad::rand::gen_range(0, 9999)) as i32;
        r.changed = true;
    }
}

#[allow(clippy::too_many_arguments)]
/// Node mode: the picked node's offsets (the original: "Property edits in this mode are offsets to
/// the value computed by the generator and should be used for fine tuning only").
fn node_mode_rows(r: &mut Rows, n: &mut Node, pick: Option<usize>) {
    if r.group("Node") {
        let Some(i) = pick else {
            r.note("Click one of this generator's branches or leaves in the viewport");
            r.note("to fine-tune just that one. Tab goes back to Generator mode.");
            r.info("Edited nodes", &n.p.node_edits.len().to_string());
            if !n.p.node_edits.is_empty() && r.button("node.resetall", "Reset every node") {
                n.p.node_edits.clear();
                r.changed = true;
            }
            return;
        };
        let label = if i >= usize::MAX / 2 { format!("Hand-drawn {}", usize::MAX - i + 1) } else { format!("Node {}", i + 1) };
        r.info("Editing", &label);
        let mut e = n.p.node_edits.get(&i).cloned().unwrap_or_default();
        let before = e.clone();
        let size_label = if n.kind == Kind::Leaf { "Size x" } else { "Length x" };
        r.f("ne.len", size_label, &mut e.length, 0.0, 3.0, 2);
        if n.kind != Kind::Leaf {
            r.f("ne.rad", "Radius x", &mut e.radius, 0.0, 3.0, 2);
            r.f("ne.ang", "Angle offset", &mut e.angle, -0.5, 0.5, 3);
        }
        r.f("ne.roll", "Roll offset (turns)", &mut e.roll, -0.5, 0.5, 3);
        r.b("ne.hide", "Hidden (pruned)", &mut e.hidden);
        if e != before {
            if e == NodeEdit::default() {
                n.p.node_edits.remove(&i);
            } else {
                n.p.node_edits.insert(i, e);
            }
            r.changed = true;
        }
        if r.button("ne.reset", "Reset this node") {
            n.p.node_edits.remove(&i);
            r.changed = true;
        }
    }
}

/// Freehand mode (picture 46): the bend controller's settings for the selected generator - its
/// pivot (the lower handle), the top of its band (the upper handle), how far and which way it
/// bends and how the bend spreads over the band - plus Reset, and branches drawn by hand.
fn freehand_rows(r: &mut Rows, n: &mut Node) {
    if r.group("Freehand") {
        if n.kind == Kind::Leaf {
            r.note("Freehand bends trunks and branches: pick a Trunk or Branch generator.");
            return;
        }
        r.note("In the viewport: drag the green top handle to bend, the blue band");
        r.note("to slide the controller, the pink handle to move the pivot.");
        r.section("Bend controller");
        let get = |p: &Params, k: &str, d: f32| p.extra.get(k).map(|v| *v as f32).unwrap_or(d);
        let mut lower = get(&n.p, "Freehand:Lower", 0.3);
        let mut upper = get(&n.p, "Freehand:Upper", 1.0);
        let mut angle = get(&n.p, "Freehand:Angle", 0.0);
        let axis = vec3(get(&n.p, "Freehand:AxisX", 0.0), get(&n.p, "Freehand:AxisY", 0.0), get(&n.p, "Freehand:AxisZ", 1.0)).normalize_or(Vec3::Z);
        // the way the top leans, as degrees round the vertical
        let lean = axis.cross(Vec3::Y);
        let mut dir = if lean.length_squared() > 1e-6 { lean.x.atan2(lean.z).to_degrees() } else { 0.0 };
        let mut shape = get(&n.p, "Freehand:Shape", 1.0);
        let before = (lower, upper, angle, dir, shape);
        r.f("fh.lower", "Pivot (lower handle)", &mut lower, 0.0, 1.0, 2);
        r.f("fh.upper", "Top (upper handle)", &mut upper, 0.0, 1.0, 2);
        r.f("fh.angle", "Bend (degrees)", &mut angle, -180.0, 180.0, 1);
        r.f("fh.dir", "Bend direction (degrees)", &mut dir, -180.0, 180.0, 0);
        r.f("fh.shape", "Bend shape (soft .. at the top)", &mut shape, 0.2, 4.0, 2);
        if (lower, upper, angle, dir, shape) != before {
            let lower = lower.clamp(0.0, 0.95);
            let upper = upper.clamp(lower + 0.05, 1.0);
            let d = dir.to_radians();
            let ax = Vec3::Y.cross(vec3(d.sin(), 0.0, d.cos())).normalize_or(Vec3::Z);
            for (k, v) in [("Lower", lower), ("Upper", upper), ("Angle", angle), ("AxisX", ax.x), ("AxisY", ax.y), ("AxisZ", ax.z), ("Shape", shape.clamp(0.2, 4.0))] {
                n.p.extra.insert(format!("Freehand:{}", k), v as f64);
            }
            r.changed = true;
        }
        // bends made before the controller was moved stay (picture 47)
        let kept = n.p.extra.get("Freehand:KeptCount").copied().unwrap_or(0.0) as usize;
        r.info("Kept bends", &kept.to_string());
        if kept > 0 && r.button("fh.undokept", "Remove the last kept bend") {
            n.p.extra.retain(|k, _| !k.starts_with(&format!("Freehand:Kept{}:", kept - 1)));
            n.p.extra.insert("Freehand:KeptCount".into(), (kept - 1) as f64);
            r.changed = true;
        }
        if r.button("fh.resetbend", "Reset all bends") {
            n.p.extra.retain(|k, _| !k.starts_with("Freehand:"));
            r.changed = true;
        }
        // Reset: the generator back to a new one's settings (its name and links stay)
        if r.button("fh.resetgen", "Reset generator to default") {
            let mut q = Params::new(n.kind);
            q.neutralize();
            n.p = q;
            r.changed = true;
        }
        r.section("Drawn branches");
        r.note("Ctrl + drag in the viewport draws a new branch.");
        r.info("Drawn branches", &n.p.drawn.len().to_string());
        if !n.p.drawn.is_empty() {
            if r.button("free.undo", "Remove the last drawn branch") {
                n.p.drawn.pop();
                r.changed = true;
            }
            if r.button("free.clear", "Clear drawn branches") {
                n.p.drawn.clear();
                r.changed = true;
            }
        }
    }
}

pub fn node_props(r: &mut Rows, n: &mut Node, on_ground: bool, mat_names: &[String], mesh_names: &[String], stats: (usize, usize, usize, usize), count: usize, max_anchor: usize, tab: Option<usize>) {
    let kind = n.kind;
    let p = &mut n.p;
    match kind {
        // Global properties: the original's "Properties - Tree" panel (picture 49)
        Kind::Tree => crate::winprops::tree_rows(r, p, stats),
        // generators: the original's tabs and properties (stpanel.rs), with TreeCraft's own
        // settings at the end of their groups
        Kind::Trunk | Kind::Branch | Kind::Leaf if tab.is_some() => {
            let t = tab.unwrap_or(0);
            let mut extra = |r: &mut Rows, p: &mut Params, g: &str| match (kind, g) {
                (_, "Generation") => {
                    if kind != Kind::Leaf {
                        r.b("collect", "Contribute to collections", &mut p.collect);
                    }
                    r.info("Instances", &count.to_string());
                }
                (Kind::Leaf, "Leaves") => {
                    r.f("leaf_size_pct", "Size: + % of parent", &mut p.leaf_size_pct, 0.0, 0.5, 3);
                    let mut mi = p.leaf_mesh.min(mesh_names.len().saturating_sub(1));
                    r.combo("leaf_mesh", "Mesh", mesh_names, &mut mi);
                    p.leaf_mesh = mi;
                    material_choice(r, "leaf_material", "Material", mat_names, &mut p.leaf_material);
                }
                (Kind::Leaf, "Local Orientation") => {
                    r.f("start_angle", "Angle from parent", &mut p.start_angle, 0.0, 1.0, 3);
                    r.f("angle_var", "Angle variance", &mut p.angle_var, 0.0, 0.5, 3);
                    r.f("droop", "Droop", &mut p.droop, -1.0, 1.0, 2);
                }
                (_, "Spine") => {
                    r.f("len_profile", "Shorter toward parent end", &mut p.len_profile, 0.0, 1.0, 2);
                    r.f("angle_profile", "Angle change along parent", &mut p.angle_profile, -1.0, 1.0, 2);
                }
                (_, "Skin") => {
                    r.f("taper", "Tip radius (taper)", &mut p.taper, 0.0, 1.0, 2);
                    r.f("flare", "Base flare", &mut p.flare, 0.0, 3.0, 2);
                    // ends stay open unless asked (picture 34)
                    r.b("cap_ends", "Cap ends", &mut p.cap_ends);
                }
                // the Segments tab's Length "+ Relative" is the length segment count now (picture 39)
                (Kind::Trunk | Kind::Branch, "UV Mapping") => uv_preview(r, p),
                (_, "Materials") => {
                    // the original's Materials group (picture 25): nothing until assigned by hand;
                    // a material set the older way shows as the list's one entry
                    if p.mat_list.is_empty() && p.material < mat_names.len() {
                        p.mat_list.push((p.material, 1.0));
                        p.material = NO_MATERIAL;
                    }
                    material_list(r, "mats", "Branch", mat_names, &mut p.mat_list);
                    material_list(r, "exts", "Extension", mat_names, &mut p.ext_list);
                }
                _ => {}
            };
            let _ = on_ground;
            crate::stpanel::draw(r, p, kind, t, &mut extra);
        }
        Kind::Trunk | Kind::Branch => {
            if r.group("Generation") {
                mode_rows(r, p, on_ground);
                seed_rows(r, p);
                r.b("collect", "Contribute to collections", &mut p.collect);
                r.info("Instances", &count.to_string());
            }
            if r.group("Spine") {
                r.f("len_abs", "Length: absolute", &mut p.len_abs, 0.0, 100.0, 2);
                r.f("len_pct", "Length: + % of parent", &mut p.len_pct, 0.0, 1.5, 3);
                r.f("len_var", "Length variance", &mut p.len_var, 0.0, 1.0, 2);
                r.f("len_profile", "Shorter toward parent end", &mut p.len_profile, 0.0, 1.0, 2);
                r.f("start_angle", "Start angle", &mut p.start_angle, 0.0, 1.0, 3);
                r.f("angle_var", "Start angle variance", &mut p.angle_var, 0.0, 0.5, 3);
                r.f("angle_profile", "Angle change along parent", &mut p.angle_profile, -1.0, 1.0, 2);
                r.f("gravity", "Gravity", &mut p.gravity, -1.0, 1.0, 2);
                r.f("noise_amt", "Noise amount", &mut p.noise_amt, 0.0, 3.0, 2);
                r.f("noise_turb", "Noise turbulence", &mut p.noise_turb, 0.0, 1.0, 2);
                r.f("jink", "Jink", &mut p.jink, 0.0, 1.0, 2);
                r.f("jink_freq", "Jink frequency", &mut p.jink_freq, 0.0, 1.0, 2);
            }
            if r.group("Skin") {
                r.f("radius_abs", "Radius: absolute", &mut p.radius_abs, 0.0, 10.0, 3);
                r.f("radius_pct", "Radius: + % of parent", &mut p.radius_pct, 0.0, 1.5, 3);
                r.f("radius_var", "Radius variance", &mut p.radius_var, 0.0, 1.0, 2);
                r.f("taper", "Tip radius (taper)", &mut p.taper, 0.0, 1.0, 2);
                r.f("flare", "Base flare", &mut p.flare, 0.0, 3.0, 2);
                r.b("clamp", "Clamp to parent", &mut p.clamp_parent);
            }
            if r.group("Segments") {
                r.i("seg_len", "Length segments", &mut p.seg_len, 2, 64);
                r.i("seg_radial", "Radial segments", &mut p.seg_radial, 3, 32);
            }
            if r.group("Materials") {
                let mut m = p.material.min(mat_names.len().saturating_sub(1));
                r.combo("material", "Branch material", mat_names, &mut m);
                p.material = m;
                r.f("u_tile", "U tiling", &mut p.u_tile, 0.1, 8.0, 2);
                r.f("v_tile", "V tiling", &mut p.v_tile, 0.1, 8.0, 2);
            }
        }
        Kind::Leaf => {
            if r.group("Generation") {
                mode_rows(r, p, on_ground);
                seed_rows(r, p);
                // All anchors, then 1..5 (the original's) or more when meshes use higher numbers
                let mut choices: Vec<String> = ANCHOR_CHOICES.iter().map(|s| s.to_string()).collect();
                for k in 6..=max_anchor.max(p.anchor_pick) {
                    choices.push(k.to_string());
                }
                r.combo("anchor_pick", "Anchors (under a leaf)", &choices, &mut p.anchor_pick);
                r.info("Instances", &count.to_string());
            }
            if r.group("Leaves") {
                r.f("leaf_size", "Size", &mut p.leaf_size, 0.0, 10.0, 2);
                r.f("leaf_size_pct", "Size: + % of parent", &mut p.leaf_size_pct, 0.0, 0.5, 3);
                r.f("leaf_size_var", "Size variance", &mut p.leaf_size_var, 0.0, 1.0, 2);
                let mut mi = p.leaf_mesh.min(mesh_names.len().saturating_sub(1));
                r.combo("leaf_mesh", "Mesh", mesh_names, &mut mi);
                p.leaf_mesh = mi;
                let mut ma = p.leaf_material.min(mat_names.len().saturating_sub(1));
                r.combo("leaf_material", "Material", mat_names, &mut ma);
                p.leaf_material = ma;
            }
            if r.group("Local Orientation") {
                r.f("start_angle", "Angle from parent", &mut p.start_angle, 0.0, 1.0, 3);
                r.f("angle_var", "Angle variance", &mut p.angle_var, 0.0, 0.5, 3);
                r.f("face_sky", "Face sky", &mut p.face_sky, 0.0, 1.0, 2);
                r.f("droop", "Droop", &mut p.droop, -1.0, 1.0, 2);
                r.f("leaf_twist", "Twist", &mut p.leaf_twist, -1.0, 1.0, 2);
            }
            if r.group("Deformation") {
                r.f("fold", "Fold", &mut p.fold, -1.0, 1.0, 2);
                r.f("curl", "Curl", &mut p.curl, -1.0, 1.0, 2);
                r.f("scale_x", "Scale X (width)", &mut p.scale_x, 0.1, 3.0, 2);
                r.f("scale_y", "Scale Y (length)", &mut p.scale_y, 0.1, 3.0, 2);
            }
        }
    }
}

/// Material properties. Returns (tree needs rebuild, texture needs rebuild, base needs reload).
pub fn material_props(r: &mut Rows, m: &mut Material, mesh_names: &[String]) -> (bool, bool, bool) {
    let mut tex = false;
    let mut base = false;
    if r.group("General") {
        r.info("Name", &m.name.clone());
        r.b("two_sided", "Two sided", &mut m.two_sided);
    }
    if r.group("Color") {
        r.color("color", "Tint", &mut m.color);
        let src: Vec<String> = MAP_SOURCES.iter().map(|s| s.to_string()).collect();
        let before = r.changed;
        r.changed = false;
        r.combo("source", "Texture", &src, &mut m.source);
        if r.changed {
            base = true;
            if m.source == 4 && m.file.is_empty() {
                if let Some(f) = pick_image() {
                    m.file = f;
                }
            }
        }
        r.changed |= before;
        if m.source == 4 {
            let fname = std::path::Path::new(&m.file).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "(none)".into());
            r.info("File", &fname);
            if r.button("load", "Load image...") {
                if let Some(f) = pick_image() {
                    m.file = f;
                    base = true;
                    r.changed = true;
                }
            }
        }
        let before = r.changed;
        r.changed = false;
        r.f("brightness", "Brightness", &mut m.brightness, -1.0, 1.0, 2);
        r.f("contrast", "Contrast", &mut m.contrast, -1.0, 1.0, 2);
        r.f("saturation", "Saturation", &mut m.saturation, -1.0, 1.0, 2);
        r.f("hue", "Hue shift", &mut m.hue, -0.5, 0.5, 2);
        if r.changed {
            tex = true;
        }
        r.changed |= before;
    }
    if r.group("Opacity") {
        let before = r.changed;
        r.changed = false;
        r.b("alpha", "Use texture alpha", &mut m.opacity_from_alpha);
        if r.changed {
            tex = true;
        }
        r.changed |= before;
        r.f("cutoff", "Alpha cutoff", &mut m.cutoff, 0.0, 1.0, 2);
    }
    if r.group("Normal") {
        r.f("bump", "Bump strength", &mut m.normal_strength, 0.0, 3.0, 2);
    }
    if r.group("Roughness / Specular") {
        // Roughness = 1 - the original's Gloss (white = rough, black = glossy; picture 25)
        let mut rough = 1.0 - m.gloss;
        r.f("gloss", "Roughness", &mut rough, 0.0, 1.0, 2);
        m.gloss = 1.0 - rough;
        // Specular (picture 61): one value; the three channels stay equal
        let mut spec = (m.specular_color[0] + m.specular_color[1] + m.specular_color[2]) / 3.0;
        let before = spec;
        r.f("specular", "Specular", &mut spec, 0.0, 1.0, 2);
        if spec != before {
            let v = spec.clamp(0.0, 1.0);
            m.specular_color = [v, v, v];
        }
        r.f("metallic", "Metallic", &mut m.metallic, 0.0, 1.0, 2);
    }
    if r.group("Subsurface") {
        r.color("sss_color", "Color", &mut m.sss_color);
        r.f("sss_amount", "Amount", &mut m.sss_amount, 0.0, 1.0, 2);
    }
    if r.group("AO") {
        r.f("ao", "Amount", &mut m.ao, 0.0, 1.0, 2);
    }
    if r.group("Variation") {
        r.f("variation", "Amount", &mut m.variation, 0.0, 1.0, 2);
        r.color("v1", "Color 1", &mut m.var_colors[0]);
        r.color("v2", "Color 2", &mut m.var_colors[1]);
        r.color("v3", "Color 3", &mut m.var_colors[2]);
    }
    if r.group("Cutout") {
        r.combo("cutout", "Cutout mesh", mesh_names, &mut m.cutout_mesh);
    }
    (r.changed, tex, base)
}

pub fn pick_image() -> Option<String> {
    crate::prefs::file_dialog()
        .add_filter("Images", &["png", "jpg", "jpeg", "tga", "bmp"])
        .pick_file()
        .map(|p| p.to_string_lossy().to_string())
}

pub fn draw_panel(app: &mut App, ui: &mut Ui, panel: Rect) {
    // tab header like the original's "Properties" dock title
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, C_PANEL);
    draw_rectangle(panel.x, panel.y, panel.w, 26.0, C_PANEL2);
    draw_rectangle(panel.x, panel.y, 84.0, 26.0, C_PANEL);
    ui.text("Properties", panel.x + 8.0, panel.y + 6.0, 13.0, C_TEXT);
    let x = panel.x + 12.0;
    let w = panel.w - 24.0;
    let mut y = panel.y + 36.0;

    // header row: what's selected + edit modes
    let (title, sub): (String, String) = match app.sel {
        Sel::None => ("Nothing selected".to_string(), String::new()),
        Sel::Node(id) => match app.node(id) {
            // (picture 62: unhighlighted after one click on empty space, still editable)
            Some(n) => (n.name.clone(), if app.sel_soft { format!("{}  -  not highlighted (click empty space again to deselect)", n.kind.type_label()) } else { n.kind.type_label().to_string() }),
            None => ("Nothing selected".into(), String::new()),
        },
        Sel::Material(i) => (app.materials.get(i).map(|m| m.name.clone()).unwrap_or_default(), "Material".into()),
        Sel::Mesh(i) => (app.meshes.get(i).map(|m| m.name.clone()).unwrap_or_default(), "Mesh".into()),
        Sel::Window => ("Window properties".into(), "The scene's viewport, rendering and capture".into()),
        Sel::Light => ("Light".into(), "Lighting (the viewport stays in view)".into()),
        Sel::Camera(i) => (
            app.cameras.get(i).map(|c| c.name.clone()).unwrap_or_default(),
            match app.cameras.get(i) {
                Some(c) if c.ortho => format!("Orthographic camera ({})", crate::render::VIEWS[c.view.min(6)]),
                _ => "Perspective camera".to_string(),
            },
        ),
    };
    let bw = 62.0;
    let bx = x + w - bw * 3.0;
    // back arrow (picture 48): to the panel shown before, or the default view
    let mut tx = x;
    if !matches!(app.sel, Sel::None) {
        let br = Rect::new(x - 4.0, y, 24.0, 22.0);
        let hot = ui.hover(br) && ui.popup.is_none();
        if ui.button_ex("prop.back", br, "", false, true) {
            if app.sel_back.is_empty() {
                app.select(Sel::None);
            } else {
                app.undo_selection();
            }
        }
        let c = if hot { WHITE } else { C_TEXT };
        let (cx, cy) = (br.x + br.w * 0.5, br.y + br.h * 0.5);
        draw_line(cx - 6.0, cy, cx + 6.0, cy, 2.0, c);
        draw_line(cx - 6.0, cy, cx - 1.0, cy - 5.0, 2.0, c);
        draw_line(cx - 6.0, cy, cx - 1.0, cy + 5.0, 2.0, c);
        if hot {
            ui.tooltip = Some(if app.sel_back.is_empty() { "Back to the default view".into() } else { "Back to the previous panel\nHot key: Alt+Z".into() });
        }
        tx = x + 26.0;
    }
    ui.text_fit(&title, tx, y + 2.0, bx - tx - 8.0, 15.0, WHITE);
    ui.record(&format!("hdr:{}", title), Rect::new(tx, y, bx - tx - 8.0, 20.0));
    if !sub.is_empty() {
        ui.text_fit(&sub, tx, y + 21.0, bx - tx - 8.0, 11.5, C_TEXT_DIM);
    }
    let is_node = matches!(app.sel, Sel::Node(_));
    draw_rectangle(bx - 2.0, y - 2.0, bw * 3.0 + 4.0, 26.0, C_BG);
    // edit modes (picture 24): Generator edits every node, Node one node (Tab switches), Freehand
    // draws branches in the viewport
    let modes = [("mode.gen", "Generator"), ("mode.node", "Node"), ("mode.free", "Freehand")];
    for (k, (key, label)) in modes.iter().enumerate() {
        let r = Rect::new(bx + bw * k as f32, y, bw, 22.0);
        if ui.button_ex(key, r, label, app.edit_mode as usize == k, true) {
            app.set_edit_mode(k as u8);
        }
        if ui.hover(r) {
            ui.tooltip = Some(
                match k {
                    0 => "All property edits apply to the selected generator(s).\nHot key: Tab",
                    1 => "Property edits apply to the picked node: offsets to what the generator computes.\nHot key: Tab",
                    _ => "Draw branches by hand in the viewport into the selected branch generator.",
                }
                .into(),
            );
        }
    }
    let _ = is_node;
    y += 44.0;
    draw_line(panel.x + 8.0, y - 6.0, panel.x + panel.w - 8.0, y - 6.0, 1.0, C_LINE);

    let body = Rect::new(panel.x, y, panel.w, panel.y + panel.h - y);
    // (the wheel over the UV preview zooms it instead, picture 26)
    let over_uv = app.uv_rect.map_or(false, |r| r.contains(ui.mouse()));
    app.uv_rect = None;
    if ui.hover_any(body) && ui.inp.wheel != 0.0 && ui.popup.is_none() && !over_uv {
        app.prop_scroll_target = (app.prop_scroll_target - ui.inp.wheel * 60.0).max(0.0);
    }
    // Preferences > Property Bar > Animate scrolling (picture 45): glide toward where it scrolls
    if app.prefs.animate_scroll && (app.prop_scroll_target - app.prop_scroll).abs() > 0.5 {
        app.prop_scroll += (app.prop_scroll_target - app.prop_scroll) * 0.25;
    } else {
        app.prop_scroll = app.prop_scroll_target;
    }
    ui.set_clip(Some(body));
    let start_y = y - app.prop_scroll;

    let mat_names: Vec<String> = app.materials.iter().map(|m| m.name.clone()).collect();
    let mnames = mesh_names(&app.meshes);
    let mut closed = std::mem::take(&mut app.closed_groups);
    let mut rows = Rows {
        ui,
        x,
        y: start_y,
        w,
        changed: false,
        prefix: String::new(),
        closed: &mut closed,
        open: true,
        curve_keys: Default::default(),
        curve_req: None,
        uv: None,
        var_amounts: Default::default(),
        var_req: None,
        stack: app.prefs.group_stacking,
        click_mode: app.prefs.group_click,
        focus: app.prop_focus.take(),
        view: (body.y, body.y + body.h),
        opened: None,
        scroll_req: None,
        seen: vec![],
        focus_start: None,
        sub_tabs: std::mem::take(&mut app.prop_sub_tabs),
    };
    match app.sel {
        Sel::None => {
            rows.note("");
            rows.y -= 20.0;
            let mut act = None;
            // the original's default panel: Edit (Wind, Lighting, Global and Window properties,
            // which open their Preferences pages), Actions, File
            for (sec, items) in [
                ("Edit", vec![("Wind", 10), ("Lighting", 11), ("Global properties", 1), ("Window properties", 2)]),
                // (Actions left out for now, picture 49)
                ("File", vec![("New tree", 5), ("Open...", 6), ("Save", 7), ("Save As...", 8)]),
            ] {
                rows.ui.text(sec, x, rows.y + 4.0, 14.0, C_TEXT);
                rows.y += 28.0;
                let bw2 = (w - 8.0) / 2.0;
                for (k, (label, id)) in items.iter().enumerate() {
                    let r = Rect::new(x + 4.0 + (k % 2) as f32 * (bw2 + 6.0) - 4.0, rows.y + (k / 2) as f32 * 32.0, bw2 - 2.0, 27.0);
                    if rows.ui.button(&format!("home.{}", label), r, label) {
                        act = Some(*id);
                    }
                }
                rows.y += ((items.len() + 1) / 2) as f32 * 32.0 + 12.0;
            }
            // the tips moved to the viewport's "?" card (picture 43)
            rows.note("Tips: the ? at the viewport's bottom left, or Help > Controls.");
            let page = |app: &mut App, p: usize| {
                app.prefs_open = true;
                app.prefs_page = p;
            };
            match act {
                // Preferences pages: 1 Viewport, 2 Wind, 3 Global; Lighting is its own panel here
                // (picture 43)
                // Global properties: the Tree generator's panel; Window properties: the scene's
                // (both here in the Properties panel now, picture 49)
                Some(1) => {
                    if let Some(t) = app.nodes.iter().find(|n| n.kind == Kind::Tree).map(|n| n.id) {
                        app.select(Sel::Node(t));
                    }
                }
                Some(2) => app.select(Sel::Window),
                Some(10) => page(app, 2),
                Some(11) => app.select(Sel::Light),
                Some(3) => app.randomize(None),
                Some(4) => app.frame_tree(),
                Some(5) => app.file_new(),
                Some(6) => app.open_tree(),
                Some(7) => {
                    app.save_tree(false);
                }
                Some(8) => {
                    app.save_tree(true);
                }
                Some(12) => {
                    app.ao_on = !app.ao_on;
                    app.set_status(if app.ao_on { "Ambient occlusion on" } else { "Ambient occlusion off" });
                }
                Some(13) => app.export_obj(),
                _ => {}
            }
        }
        Sel::Node(id) => {
            let on_ground = app.node(id).and_then(|n| n.parent).map(|p| app.node(p).map(|q| q.kind == Kind::Tree).unwrap_or(false)).unwrap_or(false);
            let stats = (app.geo_stats.0, app.geo_stats.1, app.geo_stats.2, app.geo_stats.3);
            let count = app.instance_count(id);
            if let Some(idx) = app.nodes.iter().position(|n| n.id == id) {
                rows.prefix = format!("n{}", id);
                let mut n = app.nodes[idx].clone();
                // the highest anchor number any mesh uses, so the Anchors choice can reach it
                let max_anchor = app.meshes.iter().flat_map(|m| m.lods.iter()).flat_map(|l| l.anchors.iter()).map(|a| a.id).max().unwrap_or(0);
                // generators show the original's tabs (All, Gen, Spine ...); the Tree keeps its rows
                let tab = if n.kind == Kind::Tree {
                    None
                } else {
                    let t = crate::stpanel::tab_bar(&mut rows, n.kind, if n.kind == Kind::Leaf { app.prop_tab_leaf } else { app.prop_tab });
                    if n.kind == Kind::Leaf {
                        app.prop_tab_leaf = t;
                    } else {
                        app.prop_tab = t;
                    }
                    Some(t)
                };
                if app.edit_mode == 1 && n.kind != Kind::Tree {
                    // Node mode (Tab): one node's offsets from what the generator computes
                    node_mode_rows(&mut rows, &mut n, app.node_pick.filter(|p| p.0 == id).map(|p| p.1));
                } else if app.edit_mode == 2 && n.kind != Kind::Tree {
                    freehand_rows(&mut rows, &mut n);
                } else {
                    rows.curve_keys = n.p.curves.keys().cloned().collect();
                    // variance amounts for the +/- buttons: TreeCraft's own four, then the rest
                    let mut va: std::collections::HashMap<String, f32> = n.p.variances.iter().map(|(k, v)| (k.clone(), v.amount)).collect();
                    for nm in ["Spine:Length:+ % of parent", "Spine:Orientation:Start angle", "Skin:Radius:+ % of parent", "Leaves:Size"] {
                        if let Some(v) = crate::stpanel::wire_var(&mut n.p, nm) {
                            va.insert(nm.to_string(), *v);
                        }
                    }
                    rows.var_amounts = va;
                    // the UV preview: the generator's first assigned material and its typical length
                    // (an Inherited first entry takes the parent's, as the bark does; picture 39)
                    let mi = app.shown_material(id);
                    let mat = mi.and_then(|mi| app.materials.get(mi));
                    let lens: Vec<f32> = app.geo.as_ref().map(|g| g.nodes.iter().filter(|x| x.0 == id).map(|x| x.2.distance(x.3)).collect()).unwrap_or_default();
                    let length = if lens.is_empty() { n.p.len_abs.max(1.0) } else { lens.iter().sum::<f32>() / lens.len() as f32 };
                    rows.uv = Some(UvView {
                        show: app.uv_show_tex,
                        tile: app.uv_tile,
                        slot: app.uv_slot,
                        tex: [mat.and_then(|m| m.tex.clone()), mat.and_then(|m| m.slots.get(2).and_then(|s| s.tex.clone())), mat.and_then(|m| m.slots.get(1).and_then(|s| s.tex.clone()))],
                        length,
                        zoom: app.uv_zoom,
                        pan: app.uv_pan,
                        height: app.uv_height,
                        drag: app.uv_drag,
                        last_mouse: app.uv_last_mouse,
                        rect: None,
                        scroll_by: 0.0,
                    });
                    node_props(&mut rows, &mut n, on_ground, &mat_names, &mnames, stats, count, max_anchor, tab);
                    if let Some(u) = rows.uv.take() {
                        app.uv_show_tex = u.show;
                        app.uv_tile = u.tile;
                        app.uv_slot = u.slot;
                        app.uv_zoom = u.zoom;
                        app.uv_pan = u.pan;
                        app.uv_height = u.height;
                        app.uv_drag = u.drag;
                        app.uv_last_mouse = u.last_mouse;
                        app.uv_rect = u.rect;
                        app.prop_scroll = (app.prop_scroll + u.scroll_by).max(0.0);
                        app.prop_scroll_target = app.prop_scroll;
                    }
                }
                if let Some((prop, vr)) = rows.var_req.take() {
                    // a +/- button: the Variance Settings window for it
                    app.var_edit = Some((id, prop, vr));
                    app.var_just_opened = true;
                }
                if let Some((prop, k)) = rows.curve_req.take() {
                    // a curve button: open the curve editor (a new curve starts as the original's default shape for it, picture 41)
                    n.p.curves.entry(format!("{}|{}", prop, k)).or_insert_with(|| crate::stcurves::default_curve(&prop, k).map(crate::editmodes::preset_curve).unwrap_or_else(Curve::flat));
                    app.curve_edit = Some((id, prop, k));
                    app.curve_just_opened = true;
                    app.curve_sel.clear();
                    rows.changed = true;
                }
                if rows.changed {
                    // Preferences > General > Warn about node edit losses (picture 45): a change to
                    // how many nodes the generator makes, or where, can leave single-node edits on
                    // other nodes than before
                    let o = &app.nodes[idx].p;
                    let moved = o.frequency != n.p.frequency || o.number != n.p.number || o.seed != n.p.seed || o.gen_mode != n.p.gen_mode || o.first != n.p.first || o.last != n.p.last || o.interval != n.p.interval || o.per_node != n.p.per_node || o.limit != n.p.limit || o.relative != n.p.relative;
                    if moved && app.prefs.warn_node_edits && !n.p.node_edits.is_empty() {
                        app.set_status(&format!("Warning: {} has {} single-node edit(s); this change moves its nodes, so they may now apply to different nodes", n.name, n.p.node_edits.len()));
                    }
                    app.nodes[idx] = n;
                    app.dirty = true;
                }
            }
        }
        Sel::Material(i) => {
            if i < app.materials.len() {
                rows.prefix = format!("m{}", i);
                let mut m = app.materials[i].clone();
                let (ch, tex, base) = material_props(&mut rows, &mut m, &mnames);
                if ch || tex || base {
                    if base {
                        m.dirty_base = true;
                    }
                    if tex {
                        m.dirty_tex = true;
                    }
                    app.materials[i] = m;
                    textures::refresh_material(&mut app.materials[i]);
                    app.dirty = true;
                }
            }
        }
        Sel::Mesh(i) => {
            if i < app.meshes.len() {
                rows.prefix = format!("me{}", i);
                let mut m = app.meshes[i].clone();
                let l = app.mesh_lod.min(2);
                let mut act = 0;
                if rows.group("Mesh") {
                    rows.info("Name", &m.name.clone());
                    let mut rm = m.ref_material.min(mat_names.len().saturating_sub(1));
                    rows.combo("ref", "Reference material", &mat_names, &mut rm);
                    m.ref_material = rm;
                    rows.f("scale", "Scale", &mut m.scale, 0.01, 10.0, 2);
                    rows.note("Orient, Flip, Collection, Wind and Proxy settings are in the Meshes tab.");
                }
                if rows.group("Cutout") {
                    rows.info("Level", LOD_NAMES[l]);
                    rows.i("points", "Outline points", &mut m.auto_points, 3, 32);
                    rows.f("thr", "Alpha threshold", &mut m.threshold, 0.01, 0.99, 2);
                    rows.f("margin", "Margin", &mut m.margin, 0.0, 0.1, 3);
                    if rows.button("gen", "Generate from material") {
                        act = 1;
                    }
                    if rows.button("card", "Reset to card") {
                        act = 2;
                    }
                }
                if rows.group("Level of Detail") {
                    for k in 0..3 {
                        let lod = &m.lods[k];
                        let d = if !lod.file.is_empty() {
                            format!("file ({} faces)", lod.geom.as_ref().map(|g| g.tris()).unwrap_or(0))
                        } else if lod.outline.len() >= 3 {
                            format!("cutout, {} points", lod.outline.len())
                        } else {
                            if k == 0 { "empty".to_string() } else { "empty (uses the level above)".to_string() }
                        };
                        rows.info(LOD_NAMES[k], &d);
                    }
                    if rows.button("addlod", "Fill Med and Low from High") {
                        act = 3;
                    }
                    if rows.button("remlod", "Clear the shown level") {
                        act = 4;
                    }
                }
                match act {
                    1 => {
                        if let Some(img) = app.materials.get(m.ref_material).and_then(|mm| mm.adjusted.clone()) {
                            m.lods[l] = MeshLod { file: String::new(), outline: textures::auto_cutout(&img, m.auto_points as usize, m.threshold, m.margin), geom: None, ..Default::default() };
                        }
                        rows.changed = true;
                    }
                    2 => {
                        m.lods[l] = MeshLod { file: String::new(), outline: card_outline(), geom: None, ..Default::default() };
                        rows.changed = true;
                    }
                    3 => {
                        let src = m.lods[0].outline.clone();
                        if src.len() >= 3 {
                            let n = src.len();
                            m.lods[1] = MeshLod { file: String::new(), outline: textures::reduce_outline(src.clone(), (n * 2 / 3).max(3)), geom: None, ..Default::default() };
                            m.lods[2] = MeshLod { file: String::new(), outline: textures::reduce_outline(src, (n / 3).max(3)), geom: None, ..Default::default() };
                            rows.changed = true;
                        }
                    }
                    4 => {
                        if l == 0 {
                            m.lods[0] = MeshLod { file: String::new(), outline: card_outline(), geom: None, ..Default::default() };
                        } else {
                            m.lods[l] = MeshLod::default();
                        }
                        rows.changed = true;
                    }
                    _ => {}
                }
                if rows.changed {
                    app.meshes[i] = m;
                    app.dirty = true;
                }
            }
        }
        Sel::Camera(i) => {
            rows.prefix = format!("cam{}", i);
            crate::cameras::camera_rows(&mut rows, app, i);
        }
        Sel::Light => {
            rows.prefix = "light".into();
            light_rows(&mut rows, app);
        }
        Sel::Window => {
            rows.prefix = "win".into();
            // the original's "Properties - <scene>" panel (picture 49)
            crate::winprops::window_rows(&mut rows, app);
        }
    }
    let content_h = rows.y - start_y;
    rows.finish();
    let (opened, scroll_req, changed) = (rows.opened.clone(), rows.scroll_req, rows.changed);
    app.prop_sub_tabs = std::mem::take(&mut rows.sub_tabs);
    drop(rows);
    app.closed_groups = closed;
    // a group a click opened is brought into view next frame (Group click)
    if opened.is_some() {
        app.prop_focus = opened;
    }
    if let Some(d) = scroll_req {
        app.prop_scroll_target = (app.prop_scroll + d).max(0.0);
    }
    // Preferences > Tree Window > Hide selection indicator on edit (picture 45): while a value
    // here is being changed
    let now = get_time();
    if changed || (ui.active != 0 && panel.contains(ui.mouse())) {
        app.prop_edit_t = now;
    }
    let editing = now - app.prop_edit_t < 0.3;
    let hide = match app.prefs.hide_sel_on_edit {
        0 => editing,
        1 => editing && app.wind_on,
        _ => false,
    };
    if hide != app.sel_hidden {
        app.sel_hidden = hide;
        app.dirty = true;
    }
    let max_scroll = (content_h - body.h + 20.0).max(0.0);
    app.prop_scroll = app.prop_scroll.min(max_scroll);
    app.prop_scroll_target = app.prop_scroll_target.min(max_scroll);
    ui.set_clip(None);
    if max_scroll > 0.0 {
        let frac = body.h / (content_h + 20.0);
        let th = (body.h * frac).max(24.0);
        let ty = body.y + (body.h - th) * (app.prop_scroll / max_scroll);
        draw_rectangle(panel.x + panel.w - 5.0, ty, 3.0, th, Color::new(1.0, 1.0, 1.0, 0.2));
    }
}

/// The original's light settings in the Properties panel (fn 0x140292fb0; pictures 42, 43):
/// Environment (Preset from its light_presets.xml, Directional colour, intensity, direction and
/// height, Ambient Sky / Horizon / Ground and intensity), Ambient Occlusion (Brightness,
/// Contrast, Min, Max, Render / Clear) and Shadows (Quality, Ground weight).
fn light_rows(r: &mut Rows, app: &mut App) {
    use crate::render::{hex_rgb, LIGHT_PRESETS, SHADOW_PRESETS, SHADOW_QUALITIES};
    if r.group("Environment") {
        // the preset whose values are in place, if any
        let near = |a: [f32; 3], c: u32| {
            let b = hex_rgb(c);
            (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs() < 0.02
        };
        let l = app.light;
        let mut pick = LIGHT_PRESETS
            .iter()
            .position(|p| (l.sun_intensity - p.1).abs() < 1e-3 && (l.amb_intensity - p.2).abs() < 1e-3 && near(l.sun_color, p.4) && near(l.amb_sky, p.5) && near(l.amb_horizon, p.6) && near(l.amb_ground, p.7))
            .map_or(0, |k| k + 1);
        let before = pick;
        let names: Vec<String> = std::iter::once("(custom)".to_string()).chain(LIGHT_PRESETS.iter().map(|p| p.0.to_string())).collect();
        r.combo("preset", "Preset", &names, &mut pick);
        if pick != before && pick > 0 {
            app.light.apply_preset(pick - 1);
        }
        r.section("Directional");
        r.color("sun", "Color", &mut app.light.sun_color);
        r.f("suni", "Intensity", &mut app.light.sun_intensity, 0.0, 5.0, 2);
        r.f("az", "Direction (degrees)", &mut app.light.azimuth, -180.0, 180.0, 0);
        r.f("el", "Height (degrees)", &mut app.light.elevation, 5.0, 90.0, 0);
        r.section("Ambient");
        r.color("sky", "Sky", &mut app.light.amb_sky);
        r.color("hor", "Horizon", &mut app.light.amb_horizon);
        r.color("gnd", "Ground", &mut app.light.amb_ground);
        r.f("ambi", "Intensity", &mut app.light.amb_intensity, 0.0, 3.0, 2);
        let mut lit = !app.light.unlit;
        r.b("lit", "Lit", &mut lit);
        app.light.unlit = !lit;
    }
    if r.group("Ambient Occlusion") {
        r.f("aob", "Brightness", &mut app.light.ao_brightness, -1.0, 1.0, 2);
        r.f("aoc", "Contrast", &mut app.light.ao_contrast, -1.0, 1.0, 2);
        r.f("aomin", "Min", &mut app.light.ao_min, 0.0, 1.0, 2);
        r.f("aomax", "Max", &mut app.light.ao_max, 0.0, 1.0, 2);
        r.section("Computation");
        if r.button("aorender", "Render") {
            app.ao_on = true;
            app.set_status("Ambient occlusion on");
        }
        if r.button("aoclear", "Clear") {
            app.ao_on = false;
            app.set_status("Ambient occlusion cleared");
        }
    }
    if r.group("Shadows") {
        let qs: Vec<String> = SHADOW_QUALITIES.iter().map(|s| s.to_string()).collect();
        r.combo("shq", "Quality", &qs, &mut app.light.shadow_quality);
        r.f("gw", "Ground weight", &mut app.light.ground_weight, 0.0, 1.0, 2);
        r.f("wrap", "Shadow side light", &mut app.light.wrap, 0.0, 1.0, 2);
        // shadow looks after game styles (picture 44): one click sets how soft and light the
        // shadow side is, the ground shadow's weight and the ambient occlusion
        r.section("Shadow presets");
        let l = app.light;
        let mut pick = SHADOW_PRESETS
            .iter()
            .position(|p| (l.wrap - p.1).abs() < 1e-3 && (l.amb_intensity - p.2).abs() < 1e-3 && (l.ground_weight - p.3).abs() < 1e-3 && (l.ao_min - p.4).abs() < 1e-3 && (l.ao_contrast - p.5).abs() < 1e-3)
            .unwrap_or(SHADOW_PRESETS.len());
        let before = pick;
        // Default first (picture 45), the styles, then "(custom)" when the values match none
        let names: Vec<String> = SHADOW_PRESETS.iter().map(|p| p.0.to_string()).chain(std::iter::once("(custom)".to_string())).collect();
        r.combo("shpreset", "Style", &names, &mut pick);
        if pick != before && pick < SHADOW_PRESETS.len() {
            let (_, w, ai, gw, aomin, aoc) = SHADOW_PRESETS[pick];
            app.light.wrap = w;
            app.light.amb_intensity = ai;
            app.light.ground_weight = gw;
            app.light.ao_min = aomin;
            app.light.ao_contrast = aoc;
        }
        if !app.prefs.show_ground {
            r.note("Shadows fall on the ground: Show > Ground.");
        }
    }
    let l = &mut app.light;
    l.sun_intensity = l.sun_intensity.max(0.0);
    l.amb_intensity = l.amb_intensity.max(0.0);
    l.elevation = l.elevation.clamp(5.0, 90.0);
    l.ao_brightness = l.ao_brightness.clamp(-1.0, 1.0);
    l.ao_contrast = l.ao_contrast.clamp(-1.0, 1.0);
    l.ao_min = l.ao_min.clamp(0.0, 1.0);
    l.ao_max = l.ao_max.clamp(0.0, 1.0);
    l.ground_weight = l.ground_weight.clamp(0.0, 1.0);
    l.wrap = l.wrap.clamp(0.0, 1.0);
}