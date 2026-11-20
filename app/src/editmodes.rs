//! The Properties panel's edit modes and property curves (picture 24):
//! - Generator: edits apply to every node a generator makes (the default).
//! - Node (Tab): click one branch or leaf in the viewport and give just that one offsets
//!   (length / size, radius, angle, roll, hidden) on top of what its generator computes — the
//!   original's "Property edits in this mode are offsets to the value computed by the generator"
//!   (fn 0x1405325b0 tooltips).
//! - Freehand: drag in the viewport to draw a branch into the selected branch generator.
//! - The curve editor for a property's Parent (green), Distribution (blue) and Profile (yellow)
//!   curves.

use crate::app::{App, Sel};
use crate::model::*;
use crate::ui::*;
use macroquad::prelude::*;

/// Closest approach of a ray (origin `o`, unit direction `d`) and a segment a-b: the distance, how
/// far along the ray, and where on the segment (0..1).
fn ray_segment(o: Vec3, d: Vec3, a: Vec3, b: Vec3) -> (f32, f32, f32) {
    let u = b - a;
    let w = o - a;
    let (bb, c, dd, e) = (d.dot(u), u.dot(u), d.dot(w), u.dot(w));
    let den = c - bb * bb;
    let mut t = if c < 1e-9 { 0.0 } else if den.abs() < 1e-9 { (e / c).clamp(0.0, 1.0) } else { ((e - bb * dd) / den).clamp(0.0, 1.0) };
    if c < 1e-9 {
        t = 0.0;
    }
    let q = a + u * t;
    let s = (q - o).dot(d);
    let p = o + d * s;
    (p.distance(q), s, t)
}

/// Freehand's bend controller on a generator (picture 46).
pub(crate) struct FhCtrl {
    id: u32,
    lower: f32,
    upper: f32,
    len: f32,
    pivot: Vec3,
    top: Vec3,
    tan: Vec3,
    band: Vec<Vec3>,
}

impl App {
    /// The curve editor keeps the mouse while it is over it (or dragging); elsewhere the app
    /// works as usual, the editor staying open beside it (picture 34).
    pub fn curve_takes_input(&self, mx: f32, my: f32) -> bool {
        if self.curve_edit.is_none() {
            return false;
        }
        let r = self.curve_win;
        let inside = r.w > 10.0 && mx >= r.x - 8.0 && mx <= r.x + r.w + 8.0 && my >= r.y - 8.0 && my <= r.y + r.h + 8.0;
        inside || self.curve_win_drag != 0 || self.curve_drag.is_some() || self.curve_popup_open
    }

    /// The Variance Settings window keeps the mouse while over it.
    pub fn var_takes_input(&self, mx: f32, my: f32) -> bool {
        self.var_edit.is_some() && self.var_rect.w > 10.0 && self.var_rect.contains(vec2(mx, my))
    }

    /// The original's Variance Settings window (CVarianceSettingsDialog, fn 0x1404f57d0): Variance,
    /// Distribution (Uniform (random) / Gaussian (bell curve)), and Cohesion: Cohesive .. Random,
    /// Scale, Offset. It opens beside the property's +/- button; a click elsewhere closes it.
    pub fn draw_variance_window(&mut self, ui: &mut Ui) {
        let Some((id, prop, at)) = self.var_edit.clone() else { return };
        let Some(idx) = self.nodes.iter().position(|n| n.id == id) else {
            self.var_edit = None;
            return;
        };
        ui.blocked = false;
        let (w, h) = (270.0, 168.0);
        let x = (at.x + at.w - w).clamp(4.0, ui.sw - w - 4.0);
        let y = if at.y + at.h + h + 4.0 < ui.sh { at.y + at.h + 4.0 } else { at.y - h - 4.0 };
        let r = Rect::new(x, y, w, h);
        self.var_rect = r;
        draw_rectangle(r.x + 3.0, r.y + 4.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(r, 6.0, C_PANEL);
        draw_round_rect_lines(r, 6.0, 1.0, C_LINE);
        ui.text_center("Variance Settings", Rect::new(r.x, r.y + 2.0, r.w, 18.0), 12.0, WHITE);
        ui.record("variance.window", r);
        let p = &mut self.nodes[idx].p;
        let mut v = p.variances.get(&prop).cloned().unwrap_or_default();
        // TreeCraft's own four keep their amount in their fields
        let own = crate::stpanel::wire_var(p, &prop).map(|f| *f);
        if let Some(a) = own {
            v.amount = a;
        }
        let before = v.clone();
        let lx = r.x + 10.0;
        let fx = r.x + 92.0;
        let fw = r.w - 102.0;
        let mut yy = r.y + 24.0;
        ui.text("Variance", lx, yy + 3.0, 12.0, C_TEXT_DIM);
        let hi = (v.amount.abs() * 2.0).max(1.0);
        ui.slider("var.amount", Rect::new(fx, yy, fw, 20.0), &mut v.amount, 0.0, hi, 3);
        yy += 25.0;
        ui.text("Distribution", lx, yy + 3.0, 12.0, C_TEXT_DIM);
        let dists: Vec<String> = VAR_DISTRIBUTIONS.iter().map(|s| s.to_string()).collect();
        let mut di = v.gaussian as usize;
        ui.combo("var.dist", Rect::new(fx, yy, fw, 20.0), &dists, &mut di);
        v.gaussian = di == 1;
        yy += 28.0;
        // the Cohesion group
        let gbox = Rect::new(r.x + 6.0, yy, r.w - 12.0, r.y + r.h - yy - 6.0);
        draw_rectangle_lines(gbox.x, gbox.y + 6.0, gbox.w, gbox.h - 6.0, 1.0, C_LINE);
        draw_rectangle(gbox.x + 8.0, gbox.y, ui.text_w("Cohesion", 11.5) + 8.0, 12.0, C_PANEL);
        ui.text("Cohesion", gbox.x + 12.0, gbox.y - 1.0, 11.5, C_TEXT);
        yy += 16.0;
        ui.text("Cohesive", lx + 4.0, yy + 3.0, 11.5, C_TEXT_DIM);
        let cw = ui.text_w("Random", 11.5);
        ui.text("Random", r.x + r.w - 14.0 - cw, yy + 3.0, 11.5, C_TEXT_DIM);
        let cr = Rect::new(lx + 60.0, yy + 4.0, r.w - 84.0 - 60.0 - cw + 30.0, 12.0);
        // a plain slider: Cohesive on the left, Random on the right
        draw_rectangle(cr.x, cr.y + 4.0, cr.w, 4.0, C_FIELD);
        let hx = cr.x + v.cohesion.clamp(0.0, 1.0) * cr.w;
        draw_rectangle(hx - 4.0, cr.y, 8.0, 12.0, if self.var_cohesion_drag { C_ACCENT2 } else { C_BTN_HOT });
        let m = ui.mouse();
        if ui.inp.l_pressed && Rect::new(cr.x - 4.0, cr.y - 3.0, cr.w + 8.0, 18.0).contains(m) {
            self.var_cohesion_drag = true;
        }
        if self.var_cohesion_drag {
            if ui.inp.l_down {
                v.cohesion = ((m.x - cr.x) / cr.w).clamp(0.0, 1.0);
            } else {
                self.var_cohesion_drag = false;
            }
        }
        yy += 22.0;
        ui.text("Scale", lx + 4.0, yy + 3.0, 11.5, C_TEXT_DIM);
        ui.slider("var.scale", Rect::new(fx, yy, fw - 6.0, 19.0), &mut v.scale, 0.0, 10.0, 2);
        yy += 23.0;
        ui.text("Offset", lx + 4.0, yy + 3.0, 11.5, C_TEXT_DIM);
        ui.slider("var.offset", Rect::new(fx, yy, fw - 6.0, 19.0), &mut v.offset, 0.0, 1.0, 3);
        if v != before {
            let p = &mut self.nodes[idx].p;
            if let Some(f) = crate::stpanel::wire_var(p, &prop) {
                *f = v.amount;
            }
            if v == Variance::default() {
                p.variances.remove(&prop);
            } else {
                p.variances.insert(prop.clone(), v);
            }
            self.dirty = true;
        }
        // a click elsewhere closes it
        if ui.inp.l_pressed && !r.contains(m) && ui.popup.is_none() && !self.var_just_opened && !self.var_cohesion_drag {
            self.var_edit = None;
        }
        self.var_just_opened = false;
        if (r.contains(m) || self.var_cohesion_drag) && ui.popup.is_none() {
            ui.blocked = true;
            ui.inp.l_pressed = false;
        } else {
            ui.blocked = false;
        }
    }

    pub fn set_edit_mode(&mut self, k: u8) {
        if self.edit_mode != k {
            self.edit_mode = k;
            self.node_pick = None;
            self.free_stroke.clear();
            self.set_status(match k {
                1 => "Node mode: click a branch or leaf to fine-tune just that one (Tab: back to Generator)",
                2 => "Freehand: drag in the viewport to draw a branch",
                _ => "Generator mode: edits apply to every node of the generator",
            });
        }
    }

    fn view_matrix(&self) -> Mat4 {
        let r = self.viewport;
        self.cam.camera3d(None, r.w / r.h.max(1.0), None).matrix()
    }

    /// A point in tree space on the viewport (UI units), or None behind the camera.
    fn to_screen(&self, m: &Mat4, p: Vec3) -> Option<Vec2> {
        let c = *m * p.extend(1.0);
        if c.w <= 1e-5 {
            return None;
        }
        let n = c.truncate() / c.w;
        let r = self.viewport;
        Some(vec2(r.x + (n.x + 1.0) * 0.5 * r.w, r.y + (1.0 - n.y) * 0.5 * r.h))
    }

    /// Where a screen point meets the plane through `on` facing the camera.
    fn screen_to_plane(&self, m: &Mat4, s: Vec2, on: Vec3) -> Option<Vec3> {
        let r = self.viewport;
        let nx = (s.x - r.x) / r.w * 2.0 - 1.0;
        let ny = 1.0 - (s.y - r.y) / r.h * 2.0;
        let inv = m.inverse();
        let a = inv * vec4(nx, ny, -1.0, 1.0);
        let b = inv * vec4(nx, ny, 1.0, 1.0);
        let (a, b) = (a.truncate() / a.w, b.truncate() / b.w);
        let dir = (b - a).normalize_or_zero();
        let nrm = (self.cam.position() - self.cam.target).normalize_or_zero();
        let den = dir.dot(nrm);
        if den.abs() < 1e-6 {
            return None;
        }
        let t = (on - a).dot(nrm) / den;
        Some(a + dir * t)
    }

    /// The ray from the camera through a screen point (origin, unit direction).
    fn screen_ray(&self, mat: &Mat4, s: Vec2) -> (Vec3, Vec3) {
        let r = self.viewport;
        let nx = (s.x - r.x) / r.w * 2.0 - 1.0;
        let ny = 1.0 - (s.y - r.y) / r.h * 2.0;
        let inv = mat.inverse();
        let a = inv * vec4(nx, ny, -1.0, 1.0);
        let b = inv * vec4(nx, ny, 1.0, 1.0);
        let (a, b) = (a.truncate() / a.w, b.truncate() / b.w);
        (a, (b - a).normalize_or_zero())
    }

    /// The node under a screen point: (generator, node number, the 3D point clicked). The mouse
    /// ray is tested against every branch's tube (its spine with its radius) and every leaf
    /// (picture 26: clicks anywhere on a thick trunk count); the nearest hit wins. Thin things
    /// also count within a few pixels of their line.
    fn node_under(&self, mat: &Mat4, m: Vec2) -> Option<(u32, usize, Vec3)> {
        let g = self.geo.as_ref()?;
        let (o, d) = self.screen_ray(mat, m);
        // a few pixels' worth of slack at the clicked depth
        let px_world = |depth: f32| -> f32 {
            let fov = self.cam.fov.to_radians();
            let h = if self.cam.ortho { self.cam.view_height() } else { 2.0 * depth.max(0.1) * (fov * 0.5).tan() };
            h / self.viewport.h.max(1.0)
        };
        let mut best: Option<(f32, u32, usize, Vec3)> = None;
        for (id, i, pts, rad) in &g.hits {
            for k in 0..pts.len() {
                let (a, b) = (pts[k], pts[(k + 1).min(pts.len() - 1)]);
                if k + 1 == pts.len() && pts.len() > 1 {
                    break;
                }
                let (dist, s, t) = ray_segment(o, d, a, b);
                if s <= 0.0 {
                    continue;
                }
                let rr = rad.get(k).copied().unwrap_or(0.1) + (rad.get(k + 1).copied().unwrap_or(0.1) - rad.get(k).copied().unwrap_or(0.1)) * t;
                let tol = rr.max(0.0) + px_world(s) * 4.0;
                if dist <= tol {
                    // where the ray enters the tube
                    let into = (rr * rr - dist * dist).max(0.0).sqrt();
                    let depth = (s - into).max(0.0);
                    if best.map_or(true, |x| depth < x.0) {
                        best = Some((depth, *id, *i, o + d * depth));
                    }
                }
            }
        }
        if let Some((_, id, i, p)) = best {
            return Some((id, i, p));
        }
        self.node_under_line(mat, m)
    }

    /// Fallback: the node whose base-to-tip line passes within 12 units of the mouse.
    fn node_under_line(&self, mat: &Mat4, m: Vec2) -> Option<(u32, usize, Vec3)> {
        let g = self.geo.as_ref()?;
        let mut best: Option<(f32, u32, usize, Vec3)> = None;
        for (id, i, a, b) in &g.nodes {
            let (Some(sa), Some(sb)) = (self.to_screen(mat, *a), self.to_screen(mat, *b)) else { continue };
            let ab = sb - sa;
            let t = ((m - sa).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            let d = (sa + ab * t).distance(m);
            if d < 12.0 && best.map_or(true, |x| d < x.0) {
                best = Some((d, *id, *i, a.lerp(*b, t)));
            }
        }
        // on a bent branch the bone points are closer to what's drawn than the straight line
        if let Some((_, id, i, mut p)) = best {
            let mut bd = f32::MAX;
            for bone in &g.bones {
                for q in bone {
                    if let Some(s) = self.to_screen(mat, *q) {
                        let d = s.distance(m);
                        if d < bd && d < 16.0 {
                            bd = d;
                            p = *q;
                        }
                    }
                }
            }
            return Some((id, i, p));
        }
        None
    }

    /// Generator mode (picture 25): a click (without dragging) on a branch or leaf selects its
    /// generator; a double-click also centres the camera on that spot and zooms in, so the view
    /// turns around it from then on.
    pub fn viewport_pick(&mut self, ui: &Ui, view: Rect) {
        if self.edit_mode != 0 {
            return;
        }
        let m = ui.mouse();
        let over = view.contains(m) && !ui.blocked && ui.popup.is_none() && !self.vp_ui.iter().any(|b| b.contains(m));
        if over && ui.inp.l_pressed && !ui.inp.alt {
            self.click_start = Some(m);
            if ui.inp.l_double {
                let mat = self.view_matrix();
                if let Some((id, _, p)) = self.node_under(&mat, m) {
                    self.select(Sel::Node(id));
                    self.cam.target = p;
                    self.cam.dist = (self.cam.dist * 0.5).max(1.0);
                    self.set_status("Centred the view on that spot");
                }
                self.click_start = None;
            }
        }
        if !ui.inp.l_down {
            if let Some(s) = self.click_start.take() {
                if over && s.distance(m) < 4.0 {
                    let mat = self.view_matrix();
                    match self.node_under(&mat, m) {
                        Some((id, _, _)) => self.select(Sel::Node(id)),
                        // a click on empty space lets go of the selected generator (picture 26)
                        None => {
                            // ... and of a material, mesh or any other panel: back to the default view (picture 48)
                            if !matches!(self.sel, Sel::None) {
                                self.select(Sel::None);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Node and Freehand modes take left clicks in the viewport before the camera does.
    /// Returns true when the click was theirs.
    pub fn edit_mode_input(&mut self, ui: &mut Ui, view: Rect) -> bool {
        if self.edit_mode == 0 {
            return false;
        }
        let m = ui.mouse();
        let over = view.contains(m) && !ui.blocked && ui.popup.is_none() && !self.vp_ui.iter().any(|b| b.contains(m));
        let mat = self.view_matrix();
        match self.edit_mode {
            1 => {
                if !(over && ui.inp.l_pressed && !ui.inp.alt) {
                    return false;
                }
                // the node under the mouse (its tube or leaf, picture 26)
                match self.node_under(&mat, m) {
                    Some((id, i, _)) => {
                        self.select(Sel::Node(id));
                        self.node_pick = Some((id, i));
                    }
                    None => self.node_pick = None,
                }
                true
            }
            _ => {
                if self.free_stroke.is_empty() {
                    // the bend controller's handles first (picture 46)
                    if self.freehand_input(ui, over, &mat) {
                        return true;
                    }
                    // Ctrl + drag draws a new branch
                    if !(over && ui.inp.l_pressed && !ui.inp.alt && ui.inp.ctrl) {
                        return false;
                    }
                    // start on an existing branch to grow from it
                    let mut anchor = None;
                    if let Some(g) = &self.geo {
                        let mut bd = 14.0;
                        for b in &g.bones {
                            for p in b {
                                if let Some(s) = self.to_screen(&mat, *p) {
                                    let d = s.distance(m);
                                    if d < bd {
                                        bd = d;
                                        anchor = Some(*p);
                                    }
                                }
                            }
                        }
                    }
                    self.free_anchor = anchor;
                    self.free_stroke.push(m);
                    return true;
                }
                if ui.inp.l_down {
                    if self.free_stroke.last().map_or(true, |l| l.distance(m) > 3.0) {
                        self.free_stroke.push(m);
                    }
                } else {
                    let stroke = std::mem::take(&mut self.free_stroke);
                    self.finish_stroke(&mat, stroke);
                }
                true
            }
        }
    }

    /// Freehand's bend controller on the selected generator (picture 46): its first node's spine,
    /// the pivot and top as shares of its length, the points there, and the spine's direction at
    /// the pivot.
    fn fh_controller(&self) -> Option<FhCtrl> {
        let Sel::Node(id) = self.sel else { return None };
        let n = self.node(id)?;
        if !n.kind.makes_spines() {
            return None;
        }
        let g = self.geo.as_ref()?;
        let pts = g.hits.iter().find(|h| h.0 == id && h.2.len() > 1)?.2.clone();
        let get = |k: &str, d: f32| n.p.extra.get(k).map(|v| *v as f32).unwrap_or(d);
        let lower = get("Freehand:Lower", 0.3).clamp(0.0, 0.95);
        let upper = get("Freehand:Upper", 1.0).clamp(lower + 0.05, 1.0);
        let mut cum = vec![0.0f32; pts.len()];
        for k in 1..pts.len() {
            cum[k] = cum[k - 1] + pts[k].distance(pts[k - 1]);
        }
        let len = cum[pts.len() - 1].max(1e-4);
        let at = |s: f32| -> (Vec3, Vec3) {
            let d = s.clamp(0.0, 1.0) * len;
            let mut k = 0;
            while k + 2 < pts.len() && cum[k + 1] < d {
                k += 1;
            }
            let f = ((d - cum[k]) / (cum[k + 1] - cum[k]).max(1e-6)).clamp(0.0, 1.0);
            (pts[k].lerp(pts[k + 1], f), (pts[k + 1] - pts[k]).normalize_or(Vec3::Y))
        };
        let (pivot, tan) = at(lower);
        let (top, _) = at(upper);
        let band: Vec<Vec3> = (0..=16).map(|i| at(lower + (upper - lower) * i as f32 / 16.0).0).collect();
        Some(FhCtrl { id, lower, upper, len, pivot, top, tan, band })
    }

    /// Freehand mode: the bend controller's handles take left drags (Ctrl + drag draws a branch).
    fn freehand_input(&mut self, ui: &mut Ui, over: bool, mat: &Mat4) -> bool {
        let m = ui.mouse();
        let ctrl = self.fh_controller();
        // what's under the mouse, to light it (picture 47)
        self.fh_hover = match (&ctrl, over) {
            (Some(c), true) => self.fh_hit(mat, m, c),
            _ => 0,
        };
        if self.fh_drag == 0 {
            if !(over && ui.inp.l_pressed && !ui.inp.alt) || ui.inp.ctrl {
                return false;
            }
            let Some(c) = ctrl else { return false };
            let (Some(sp), Some(st)) = (self.to_screen(mat, c.pivot), self.to_screen(mat, c.top)) else { return false };
            let part = self.fh_hit(mat, m, &c);
            if part == 0 {
                return false;
            }
            // screen length of one whole spine-length along the band, and its screen direction
            let span = (st - sp).length() / (c.upper - c.lower).max(0.05);
            let sdir = (st - sp).normalize_or(vec2(0.0, -1.0));
            // moving the band or the pivot keeps the bend made so far where it is (picture 47): it
            // joins the kept bends and the controller starts a new, straight one
            if part != 1 {
                if let Some(n) = self.nodes.iter_mut().find(|x| x.id == c.id) {
                    let ang = n.p.extra.get("Freehand:Angle").copied().unwrap_or(0.0);
                    if ang.abs() > 0.01 {
                        let i = n.p.extra.get("Freehand:KeptCount").copied().unwrap_or(0.0) as usize;
                        for k in ["Lower", "Upper", "Angle", "AxisX", "AxisY", "AxisZ", "Shape"] {
                            if let Some(v) = n.p.extra.get(&format!("Freehand:{}", k)).copied() {
                                n.p.extra.insert(format!("Freehand:Kept{}:{}", i, k), v);
                            }
                        }
                        n.p.extra.insert("Freehand:KeptCount".into(), (i + 1) as f64);
                        n.p.extra.insert("Freehand:Angle".into(), 0.0);
                        self.dirty = true;
                    }
                }
            }
            self.fh_drag = part;
            self.fh_grab = (m, c.lower, c.upper, span, sdir);
            return true;
        }
        if !ui.inp.l_down {
            self.fh_drag = 0;
            return true;
        }
        let Some(c) = ctrl else {
            self.fh_drag = 0;
            return true;
        };
        let (m0, lo0, up0, span, sdir) = self.fh_grab;
        let ds = (m - m0).dot(sdir) / span.max(1.0);
        let mut vals: Vec<(&str, f32)> = vec![];
        match self.fh_drag {
            1 => {
                // the top handle: aim the band from the pivot at the mouse (on the plane facing the
                // camera through the top), turning round the axis between the two
                if let Some(target) = self.screen_to_plane(mat, m, c.top) {
                    let v0 = c.tan * (c.len * (c.upper - c.lower)).max(0.01);
                    let v1 = target - c.pivot;
                    let axis = v0.cross(v1);
                    if axis.length_squared() > 1e-10 && v1.length_squared() > 1e-8 {
                        let axis = axis.normalize();
                        let ang = v0.normalize().dot(v1.normalize()).clamp(-1.0, 1.0).acos().to_degrees();
                        vals = vec![("Angle", ang), ("AxisX", axis.x), ("AxisY", axis.y), ("AxisZ", axis.z)];
                    }
                }
            }
            2 => {
                // the band: slide the whole controller along the generator
                let w = up0 - lo0;
                let lo = (lo0 + ds).clamp(0.0, 1.0 - w);
                vals = vec![("Lower", lo), ("Upper", lo + w)];
            }
            _ => {
                // the pivot: stretch or shorten the band from below
                vals = vec![("Lower", (lo0 + ds).clamp(0.0, up0 - 0.05))];
            }
        }
        if let Some(n) = self.nodes.iter_mut().find(|x| x.id == c.id) {
            for (k, v) in vals {
                n.p.extra.insert(format!("Freehand:{}", k), v as f64);
            }
            self.dirty = true;
        }
        true
    }

    /// Which part of the controller is under the mouse: 1 top handle, 2 band, 3 pivot, 0 none.
    fn fh_hit(&self, mat: &Mat4, m: Vec2, c: &FhCtrl) -> u8 {
        let (Some(sp), Some(st)) = (self.to_screen(mat, c.pivot), self.to_screen(mat, c.top)) else { return 0 };
        if st.distance(m) < 18.0 {
            return 1;
        }
        if sp.distance(m) < 17.0 {
            return 3;
        }
        let on_band = c.band.windows(2).any(|w| {
            if let (Some(a), Some(b)) = (self.to_screen(mat, w[0]), self.to_screen(mat, w[1])) {
                let ab = b - a;
                let t = ((m - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                (a + ab * t).distance(m) < 11.0
            } else {
                false
            }
        });
        if on_band {
            2
        } else {
            0
        }
    }

    /// Over the viewport in Freehand mode (pictures 46, 47): the controller, drawn as a glowing
    /// band that shades from cyan at the pivot to green at the top with small beads along it, a
    /// pill-shaped hinge at the pivot and a ringed knob with bend arrows at the top; the part under
    /// the mouse or being dragged grows and brightens. The bend, and how many earlier bends are
    /// kept, show on a small tag by the knob.
    fn draw_freehand_controller(&self) {
        let Some(c) = self.fh_controller() else { return };
        let mat = self.view_matrix();
        let pts: Vec<Vec2> = c.band.iter().filter_map(|p| self.to_screen(&mat, *p)).collect();
        if pts.len() < 2 {
            return;
        }
        let lit = |part: u8| self.fh_drag == part || (self.fh_drag == 0 && self.fh_hover == part);
        let cyan = Color::new(0.2, 0.75, 1.0, 1.0);
        let green = Color::new(0.3, 0.95, 0.5, 1.0);
        let mix = |t: f32| Color::new(cyan.r + (green.r - cyan.r) * t, cyan.g + (green.g - cyan.g) * t, cyan.b + (green.b - cyan.b) * t, 1.0);
        let n = pts.len() - 1;
        let bw = if lit(2) { 6.0 } else { 4.0 };
        // a soft glow, a dark edge, then the coloured band
        for w in pts.windows(2) {
            draw_line(w[0].x, w[0].y, w[1].x, w[1].y, bw + 12.0, Color::new(0.2, 0.75, 1.0, if lit(2) { 0.22 } else { 0.12 }));
        }
        for w in pts.windows(2) {
            draw_line(w[0].x, w[0].y, w[1].x, w[1].y, bw + 3.0, Color::new(0.03, 0.06, 0.1, 0.85));
            draw_circle(w[1].x, w[1].y, (bw + 3.0) * 0.5, Color::new(0.03, 0.06, 0.1, 0.85));
        }
        for (k, w) in pts.windows(2).enumerate() {
            let c0 = mix(k as f32 / n as f32);
            draw_line(w[0].x, w[0].y, w[1].x, w[1].y, bw, c0);
            draw_circle(w[1].x, w[1].y, bw * 0.5, c0);
        }
        // beads along the band
        for (k, p) in pts.iter().enumerate().skip(2).step_by(3) {
            if k < n - 1 {
                draw_circle(p.x, p.y, bw * 0.32, Color::new(1.0, 1.0, 1.0, 0.75));
            }
        }
        // the pivot: a pill across the band with a hinge dot
        let (sp, st) = (pts[0], pts[n]);
        let d = (pts[1] - pts[0]).normalize_or(vec2(0.0, -1.0));
        let nrm = vec2(-d.y, d.x);
        let pw = if lit(3) { 15.0 } else { 12.0 };
        let pink = if lit(3) { Color::new(1.0, 0.5, 0.8, 1.0) } else { Color::new(0.92, 0.3, 0.68, 1.0) };
        draw_circle(sp.x, sp.y, pw + 8.0, Color::new(0.95, 0.3, 0.7, if lit(3) { 0.22 } else { 0.1 }));
        draw_line(sp.x - nrm.x * pw, sp.y - nrm.y * pw, sp.x + nrm.x * pw, sp.y + nrm.y * pw, 11.0, Color::new(0.05, 0.03, 0.06, 0.9));
        for e in [-1.0f32, 1.0] {
            draw_circle(sp.x + nrm.x * pw * e, sp.y + nrm.y * pw * e, 5.5, Color::new(0.05, 0.03, 0.06, 0.9));
            draw_circle(sp.x + nrm.x * pw * e, sp.y + nrm.y * pw * e, 4.0, pink);
        }
        draw_line(sp.x - nrm.x * pw, sp.y - nrm.y * pw, sp.x + nrm.x * pw, sp.y + nrm.y * pw, 8.0, pink);
        draw_circle(sp.x, sp.y, 4.5, WHITE);
        draw_circle(sp.x, sp.y, 2.2, pink);
        // the top: a ringed knob with a small arrow either side (the ways it bends)
        let r = if lit(1) { 12.0 } else { 9.5 };
        draw_circle(st.x, st.y, r + 9.0, Color::new(0.3, 0.95, 0.5, if lit(1) { 0.25 } else { 0.12 }));
        draw_circle(st.x, st.y, r + 2.5, Color::new(0.03, 0.06, 0.1, 0.9));
        draw_circle(st.x, st.y, r, green);
        draw_circle_lines(st.x, st.y, r - 3.0, 1.5, Color::new(1.0, 1.0, 1.0, 0.9));
        draw_circle(st.x, st.y, 2.5, WHITE);
        let td = (pts[n] - pts[n - 1]).normalize_or(vec2(0.0, -1.0));
        let tn = vec2(-td.y, td.x);
        for e in [-1.0f32, 1.0] {
            let tip = st + tn * e * (r + 12.0);
            let base = st + tn * e * (r + 5.0);
            draw_triangle(tip, base + td * 4.5, base - td * 4.5, Color::new(1.0, 1.0, 1.0, if lit(1) { 0.95 } else { 0.6 }));
        }
        // the tag: this bend and how many earlier ones are kept
        let ang = self.node(c.id).and_then(|n| n.p.extra.get("Freehand:Angle")).copied().unwrap_or(0.0);
        let kept = self.node(c.id).and_then(|n| n.p.extra.get("Freehand:KeptCount")).copied().unwrap_or(0.0) as usize;
        let mut t = if ang.abs() > 0.05 { format!("{:.0} deg", ang) } else { "drag to bend".to_string() };
        if kept > 0 {
            t = format!("{}  +{} kept", t, kept);
        }
        let tw = 10.0 + t.len() as f32 * 7.3;
        let tr = Rect::new(st.x + r + 16.0, st.y - 10.0, tw, 20.0);
        draw_rectangle(tr.x + 2.0, tr.y + 2.0, tr.w, tr.h, Color::new(0.0, 0.0, 0.0, 0.25));
        draw_rectangle(tr.x, tr.y, tr.w, tr.h, Color::new(0.06, 0.08, 0.11, 0.85));
        draw_rectangle(tr.x, tr.y, 3.0, tr.h, green);
        draw_text(&t, tr.x + 8.0, tr.y + 14.5, 16.0, WHITE);
    }

    /// Turn a drawn stroke into a branch of the selected branch generator (a new "Hand Drawn"
    /// trunk generator when none is selected).
    fn finish_stroke(&mut self, mat: &Mat4, stroke: Vec<Vec2>) {
        if stroke.len() < 3 {
            return;
        }
        let on = self.free_anchor.unwrap_or(self.cam.target);
        let mut pts: Vec<Vec3> = stroke.iter().filter_map(|s| self.screen_to_plane(mat, *s, on)).collect();
        if let (Some(a), Some(first)) = (self.free_anchor, pts.first_mut()) {
            *first = a;
        }
        // even spacing, about 16 points
        let total: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
        if pts.len() < 2 || total < 0.05 {
            return;
        }
        let n = 16;
        let mut out = vec![pts[0]];
        let mut acc = 0.0;
        let step = total / n as f32;
        let mut want = step;
        for w in pts.windows(2) {
            let l = w[0].distance(w[1]);
            while acc + l >= want && out.len() < n {
                let f = (want - acc) / l.max(1e-6);
                out.push(w[0].lerp(w[1], f));
                want += step;
            }
            acc += l;
        }
        out.push(*pts.last().unwrap());
        let target = match self.sel {
            Sel::Node(id) if self.node(id).map_or(false, |n| n.kind.makes_spines()) => id,
            _ => {
                let root = self.root_id();
                let id = self.add_node(Kind::Trunk, root);
                if let Some(nd) = self.nodes.iter_mut().find(|x| x.id == id) {
                    nd.name = "Hand Drawn".into();
                    nd.p.neutralize();
                    // only the drawn branches: no procedural ones
                    nd.p.frequency = 0.0;
                    nd.p.number = 0;
                    nd.p.radius_abs = (total * 0.03).max(0.05);
                }
                id
            }
        };
        if let Some(nd) = self.nodes.iter_mut().find(|x| x.id == target) {
            nd.p.drawn.push(out);
        }
        self.select(Sel::Node(target));
        self.dirty = true;
        self.set_status("Drew a branch");
    }

    /// In the 3D pass: the picked node (Node mode).
    pub fn draw_edit_3d(&self) {
        if self.edit_mode != 1 {
            return;
        }
        let (Some((id, i)), Some(g)) = (self.node_pick, &self.geo) else { return };
        let c = self.prefs.color("picked");
        for (nid, ni, a, b) in &g.nodes {
            if *nid == id && *ni == i {
                let r = (a.distance(*b) * 0.03).clamp(0.03, 0.4);
                draw_sphere(*a, r, None, c);
                draw_sphere(*b, r * 0.7, None, c);
                draw_line_3d(*a, *b, c);
            }
        }
    }

    /// Over the viewport: the stroke being drawn (Freehand).
    pub fn draw_edit_overlay(&self) {
        if self.edit_mode == 2 {
            self.draw_freehand_controller();
            let c = self.prefs.color("picked");
            for w in self.free_stroke.windows(2) {
                draw_line(w[0].x, w[0].y, w[1].x, w[1].y, 2.5, c);
            }
            let v = self.viewport;
            let t = "Freehand: knob bends, band slides, pink hinge moves the pivot; Ctrl+drag draws";
            draw_rectangle(v.x + 38.0, v.y + v.h - 46.0, 600.0, 20.0, Color::new(0.0, 0.0, 0.0, 0.45));
            draw_text(t, v.x + 44.0, v.y + v.h - 31.0, 16.0, c);
        } else if self.edit_mode == 1 {
            let v = self.viewport;
            let t = "Node mode: click a branch or leaf to fine-tune it (Tab: Generator mode)";
            draw_rectangle(v.x + 8.0, v.y + v.h - 46.0, 470.0, 20.0, Color::new(0.0, 0.0, 0.0, 0.45));
            draw_text(t, v.x + 14.0, v.y + v.h - 31.0, 16.0, self.prefs.color("picked"));
        }
    }

    /// The curve editor, laid out like the original's (SplineEditorDialog, fn 0x1403a3870;
    /// right-click menu fn 0x1400dba00): a title "Curve - generator:property:kind", a Pin button
    /// ("Pin this curve editor so it doesn't go away"), Parent level, Draw, Presets, X and Y of the
    /// picked point, and the graph. It moves by its title and resizes from any edge or corner;
    /// unpinned, a click elsewhere closes it.
    pub fn draw_curve_window(&mut self, ui: &mut Ui) {
        let Some((id, prop, kind)) = self.curve_edit.clone() else { return };
        let key = format!("{}|{}", prop, kind);
        let Some(idx) = self.nodes.iter().position(|n| n.id == id) else {
            self.curve_edit = None;
            return;
        };
        ui.blocked = false;
        let (sw, sh) = (ui.sw, ui.sh);
        if self.curve_win.w < 10.0 {
            self.curve_win = Rect::new((sw - 520.0) * 0.5, (sh - 380.0) * 0.5, 520.0, 380.0);
        }
        let m = ui.mouse();
        let d = m - self.curve_last_mouse;
        self.curve_last_mouse = m;
        // moving and resizing
        let mut r = self.curve_win;
        let grip = 7.0;
        let near = |a: f32, b: f32| (a - b).abs() <= grip;
        let zone = {
            let inside_x = m.x >= r.x - grip && m.x <= r.x + r.w + grip;
            let inside_y = m.y >= r.y - grip && m.y <= r.y + r.h + grip;
            let (l, rt, t, b) = (near(m.x, r.x) && inside_y, near(m.x, r.x + r.w) && inside_y, near(m.y, r.y) && inside_x, near(m.y, r.y + r.h) && inside_x);
            (l as u8) | (rt as u8) << 1 | (t as u8) << 2 | (b as u8) << 3
        };
        let title = Rect::new(r.x, r.y, r.w, 24.0);
        if ui.inp.l_pressed && ui.popup.is_none() {
            if zone != 0 {
                self.curve_win_drag = zone;
                ui.inp.l_pressed = false;
            } else if title.contains(m) && m.x < r.x + r.w - 56.0 {
                self.curve_win_drag = 16;
                ui.inp.l_pressed = false;
            }
        }
        if self.curve_win_drag != 0 {
            if ui.inp.l_down {
                let z = self.curve_win_drag;
                if z == 16 {
                    r.x += d.x;
                    r.y += d.y;
                } else {
                    if z & 1 != 0 {
                        let nx = (r.x + d.x).min(r.x + r.w - 380.0);
                        r.w += r.x - nx;
                        r.x = nx;
                    }
                    if z & 2 != 0 {
                        r.w = (r.w + d.x).max(380.0);
                    }
                    if z & 4 != 0 {
                        let ny = (r.y + d.y).min(r.y + r.h - 260.0);
                        r.h += r.y - ny;
                        r.y = ny;
                    }
                    if z & 8 != 0 {
                        r.h = (r.h + d.y).max(260.0);
                    }
                }
                r.x = r.x.clamp(0.0, sw - 60.0);
                r.y = r.y.clamp(0.0, sh - 30.0);
                self.curve_win = r;
            } else {
                self.curve_win_drag = 0;
            }
        }
        // the frame
        draw_rectangle(r.x + 4.0, r.y + 5.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(r, 6.0, C_PANEL);
        draw_round_rect_lines(r, 6.0, 1.0, C_LINE);
        let colr = [Color::new(0.35, 0.80, 0.35, 1.0), Color::new(0.30, 0.55, 1.0, 1.0), Color::new(0.98, 0.82, 0.25, 1.0)][kind as usize % 3];
        draw_round_rect(Rect::new(r.x, r.y, r.w, 24.0), 6.0, C_PANEL2);
        draw_rectangle(r.x, r.y + 22.0, r.w, 2.0, colr);
        let gname = self.nodes[idx].name.replace(' ', "_");
        let title_text = format!("Curve - {}:{}:{}", gname, prop, CURVE_KINDS[kind as usize % 3]);
        ui.text_fit(&title_text, r.x + 10.0, r.y + 5.0, r.w - 80.0, 12.5, WHITE);
        ui.record("curve.window", r);
        // pin and close
        let pin = Rect::new(r.x + r.w - 52.0, r.y + 3.0, 22.0, 18.0);
        if ui.button_ex("curve.pin", pin, "P", self.curve_pin, true) {
            self.curve_pin = !self.curve_pin;
        }
        if ui.hover(pin) {
            ui.tooltip = Some("Pin this curve editor so it doesn't go away.".into());
        }
        let xb = Rect::new(r.x + r.w - 26.0, r.y + 3.0, 22.0, 18.0);
        let mut close = ui.button("curve.close", xb, "x");
        // the toolbar: Parent level, Draw, Presets, X and Y
        let mut curve = self.nodes[idx].p.curves.get(&key).cloned().unwrap_or_else(Curve::flat);
        curve.fix_kinds();
        let mut changed = false;
        let ty = r.y + 30.0;
        let mut tx = r.x + 10.0;
        ui.text("Parent:", tx, ty + 3.0, 12.0, if kind == 0 { C_TEXT } else { C_TEXT_FAINT });
        tx += ui.text_w("Parent:", 12.0) + 6.0;
        let mut levels = vec![];
        let mut cur = self.nodes[idx].parent;
        while let Some(pid) = cur {
            if levels.len() >= 4 {
                break;
            }
            match self.node(pid) {
                Some(pn) => {
                    levels.push(format!("{} - {}", levels.len() + 1, pn.name));
                    cur = pn.parent;
                }
                None => break,
            }
        }
        if levels.is_empty() {
            levels.push("1 - Tree".into());
        }
        let pr = Rect::new(tx, ty, 110.0, 20.0);
        let mut lv = (self.nodes[idx].p.curve_levels.get(&key).copied().unwrap_or(1) as usize).saturating_sub(1).min(levels.len() - 1);
        let lv_before = lv;
        if kind == 0 {
            ui.combo("curve.level", pr, &levels, &mut lv);
            if ui.hover(pr) {
                ui.tooltip = Some("Parent level".into());
            }
        } else {
            draw_rectangle(pr.x, pr.y, pr.w, pr.h, C_FIELD);
            ui.text("-", pr.x + 6.0, pr.y + 3.0, 12.0, C_TEXT_FAINT);
        }
        tx += pr.w + 6.0;
        let dr = Rect::new(tx, ty, 46.0, 20.0);
        if ui.button_ex("curve.draw", dr, "Draw", self.curve_draw, true) {
            self.curve_draw = !self.curve_draw;
        }
        if ui.hover(dr) {
            ui.tooltip = Some("Draw the curve freehand: drag across the graph".into());
        }
        tx += dr.w + 6.0;
        let presets: Vec<String> = std::iter::once("Presets".to_string()).chain(CURVE_PRESETS.iter().map(|s| s.to_string())).collect();
        let ps = Rect::new(tx, ty, 120.0, 20.0);
        let mut pi = 0usize;
        if ui.combo("curve.presets", ps, &presets, &mut pi) && pi > 0 {
            curve = preset_curve(pi - 1);
            self.curve_sel.clear();
            changed = true;
        }
        if ui.hover(ps) {
            ui.tooltip = Some("Set curve to preset".into());
        }
        tx += ps.w + 8.0;
        // X and Y of the first picked point
        let fw = ((r.x + r.w - 10.0 - tx) * 0.5 - 18.0).max(40.0);
        if let Some(&si) = self.curve_sel.first().filter(|&&i| i < curve.pts.len()) {
            let n = curve.pts.len();
            let (mut px, mut py) = (curve.pts[si].x, curve.pts[si].y);
            ui.text("X", tx, ty + 3.0, 12.0, C_TEXT);
            let xr = Rect::new(tx + 12.0, ty, fw, 20.0);
            if si > 0 && si + 1 < n {
                let (lo, hi) = (curve.pts[si - 1].x + 0.001, curve.pts[si + 1].x - 0.001);
                if ui.slider("curve.x", xr, &mut px, lo, hi, 3) {
                    curve.pts[si].x = px.clamp(lo, hi);
                    changed = true;
                }
            } else {
                draw_rectangle(xr.x, xr.y, xr.w, xr.h, C_FIELD);
                ui.text(&format!("{:.3}", px), xr.x + 6.0, xr.y + 3.0, 12.0, C_TEXT_FAINT);
            }
            ui.text("Y", xr.x + xr.w + 6.0, ty + 3.0, 12.0, C_TEXT);
            let yr = Rect::new(xr.x + xr.w + 18.0, ty, fw, 20.0);
            if ui.slider("curve.y", yr, &mut py, 0.0, 1.0, 3) {
                curve.pts[si].y = py.clamp(0.0, 1.0);
                changed = true;
            }
        }
        // the graph
        let g = Rect::new(r.x + 30.0, r.y + 60.0, r.w - 44.0, r.h - 92.0);
        draw_rectangle(g.x, g.y, g.w, g.h, Color::new(0.33, 0.33, 0.34, 1.0));
        for k in 0..=10 {
            let f = k as f32 / 10.0;
            let a = if k % 5 == 0 { 0.16 } else { 0.07 };
            draw_line(g.x + g.w * f, g.y, g.x + g.w * f, g.y + g.h, 1.0, Color::new(1.0, 1.0, 1.0, a));
            draw_line(g.x, g.y + g.h * f, g.x + g.w, g.y + g.h * f, 1.0, Color::new(1.0, 1.0, 1.0, a));
        }
        ui.text("1", g.x - 14.0, g.y - 6.0, 11.0, C_TEXT_DIM);
        ui.text("0", g.x - 14.0, g.y + g.h - 8.0, 11.0, C_TEXT_DIM);
        let axis = match kind {
            0 => "where the child sits on its parent: base  ->  tip",
            1 => "the children on one parent: first  ->  last",
            _ => "along the branch itself: base  ->  tip",
        };
        ui.text_fit(axis, g.x, g.y + g.h + 4.0, g.w * 0.6, 11.0, C_TEXT_DIM);
        ui.text_fit("Right-click for more  -  Del, Ctrl-A, Ctrl-C, Ctrl-V", g.x + g.w * 0.55, g.y + g.h + 4.0, g.w * 0.45, 11.0, C_TEXT_FAINT);
        let to_s = |p: Vec2| vec2(g.x + p.x * g.w, g.y + (1.0 - p.y) * g.h);
        let from_s = |s: Vec2| vec2(((s.x - g.x) / g.w).clamp(0.0, 1.0), (1.0 - (s.y - g.y) / g.h).clamp(0.0, 1.0));
        let mut hover_pt: Option<(usize, f32)> = None;
        for (i, p) in curve.pts.iter().enumerate() {
            // a generous grab radius, nearest point first (picture 36)
            // (picture 56: wider still, 18 px, so handles are easy to catch)
            let dd = to_s(*p).distance(m);
            if dd < 18.0 && hover_pt.map_or(true, |(_, best): (usize, f32)| dd < best) {
                hover_pt = Some((i, dd));
            }
        }
        let hover_pt = hover_pt.map(|h| h.0);
        // a handle on the graph's edge (the end points) can be caught from just outside it
        let over_graph = (g.contains(m) || (hover_pt.is_some() && r.contains(m))) && ui.popup.is_none() && self.curve_win_drag == 0;
        // keys while the mouse is over the window
        if r.contains(m) && !ui.editing() {
            if ui.inp.key(KeyCode::Delete) {
                changed |= delete_points(&mut curve, &mut self.curve_sel);
            }
            if ui.inp.ctrl && ui.inp.key(KeyCode::A) {
                self.curve_sel = (0..curve.pts.len()).collect();
            }
            if ui.inp.ctrl && ui.inp.key(KeyCode::C) {
                self.curve_clip = Some(curve.clone());
            }
            if ui.inp.ctrl && ui.inp.key(KeyCode::V) {
                if let Some(c) = self.curve_clip.clone() {
                    curve = c;
                    self.curve_sel.clear();
                    changed = true;
                }
            }
        }
        if self.curve_draw {
            // freehand: the stroke replaces the points under it
            if over_graph && ui.inp.l_pressed {
                self.curve_drag = Some(usize::MAX);
                self.curve_draw_last = None;
            }
            if self.curve_drag == Some(usize::MAX) {
                if ui.inp.l_down {
                    let q = from_s(m);
                    let last = self.curve_draw_last.unwrap_or(q);
                    let (x0, x1) = (last.x.min(q.x), last.x.max(q.x));
                    let n = curve.pts.len();
                    let mut keep = vec![];
                    for (i, p) in curve.pts.iter().enumerate() {
                        let interior = i > 0 && i + 1 < n;
                        if !(interior && p.x >= x0 - 1e-4 && p.x <= x1 + 1e-4) {
                            keep.push((*p, curve.kind(i)));
                        }
                    }
                    if q.x <= 0.0005 {
                        keep[0].0.y = q.y;
                    } else if q.x >= 0.9995 {
                        let l = keep.len() - 1;
                        keep[l].0.y = q.y;
                    } else {
                        let at = keep.iter().position(|(p, _)| p.x > q.x).unwrap_or(keep.len());
                        keep.insert(at, (q, POINT_SMOOTH));
                    }
                    curve.pts = keep.iter().map(|k| k.0).collect();
                    curve.kinds = keep.iter().map(|k| k.1).collect();
                    self.curve_draw_last = Some(q);
                    self.curve_sel.clear();
                    changed = true;
                } else {
                    self.curve_drag = None;
                }
            }
        } else {
            if over_graph && ui.inp.l_pressed {
                match hover_pt {
                    Some(i) => {
                        if ui.inp.shift {
                            if !self.curve_sel.contains(&i) {
                                self.curve_sel.push(i);
                            }
                        } else if !self.curve_sel.contains(&i) {
                            self.curve_sel = vec![i];
                        }
                        self.curve_drag = Some(i);
                    }
                    None => {
                        if ui.inp.l_double {
                            changed |= insert_point(&mut curve, from_s(m), POINT_SMOOTH, &mut self.curve_sel);
                        } else {
                            self.curve_sel.clear();
                        }
                    }
                }
            }
            if let Some(i) = self.curve_drag.filter(|&i| i != usize::MAX) {
                if ui.inp.l_down {
                    // the picked points move together; the ends stay at the ends. The grabbed
                    // point follows the mouse itself (picture 56), so it never lags behind
                    let n = curve.pts.len();
                    let _ = d;
                    let want = vec2(((m.x - g.x) / g.w).clamp(0.0, 1.0), (1.0 - (m.y - g.y) / g.h).clamp(-1.0, 2.0));
                    let cur = curve.pts.get(i).copied().unwrap_or(want);
                    let dx = want.x - cur.x;
                    let dy = want.y - cur.y;
                    let mut sel = self.curve_sel.clone();
                    if !sel.contains(&i) {
                        sel = vec![i];
                    }
                    for &s in &sel {
                        if s >= n {
                            continue;
                        }
                        let mut q = curve.pts[s] + vec2(dx, dy);
                        if s == 0 {
                            q.x = 0.0;
                        } else if s == n - 1 {
                            q.x = 1.0;
                        } else {
                            q.x = q.x.clamp(curve.pts[s - 1].x + 0.001, curve.pts[s + 1].x - 0.001);
                        }
                        q.y = q.y.clamp(0.0, 1.0);
                        if curve.pts[s] != q {
                            curve.pts[s] = q;
                            changed = true;
                        }
                    }
                } else {
                    self.curve_drag = None;
                }
            }
        }
        // right-click: the original's menu
        if over_graph && ui.inp.r_pressed {
            self.curve_ctx_at = from_s(m);
            if let Some(i) = hover_pt {
                if !self.curve_sel.contains(&i) {
                    self.curve_sel = vec![i];
                }
            }
            let items = vec![
                MenuItem::leaf("Insert point here"),
                MenuItem::leaf("Insert step point here"),
                MenuItem::leaf("Insert collapsed point here"),
                MenuItem::sep(),
                MenuItem::leaf("Delete selected control points").hot("DEL"),
                MenuItem::sep(),
                MenuItem::sub("Scale curve", vec![MenuItem::leaf("From curve max"), MenuItem::leaf("From curve average"), MenuItem::leaf("From curve middle"), MenuItem::leaf("From curve min")]),
                MenuItem::leaf("Flip curve vertically"),
                MenuItem::leaf("Flip curve horizontally"),
                MenuItem::sep(),
                MenuItem::leaf("Select all control points").hot("Ctrl-A"),
                MenuItem::leaf("Copy curve").hot("Ctrl-C"),
                MenuItem::leaf("Paste curve").hot("Ctrl-V"),
                MenuItem::sep(),
                MenuItem::leaf("Remove curve"),
            ];
            ui.open_menu(hid("curve.ctx"), Rect::new(m.x, m.y, 10.0, 0.0), items);
        }
        if let Some(path) = ui.take_menu_result("curve.ctx") {
            let at = self.curve_ctx_at;
            match (path.first().copied(), path.get(1).copied()) {
                (Some(0), _) => changed |= insert_point(&mut curve, at, POINT_SMOOTH, &mut self.curve_sel),
                (Some(1), _) => changed |= insert_point(&mut curve, at, POINT_STEP, &mut self.curve_sel),
                (Some(2), _) => changed |= insert_point(&mut curve, at, POINT_COLLAPSED, &mut self.curve_sel),
                (Some(4), _) => changed |= delete_points(&mut curve, &mut self.curve_sel),
                (Some(6), Some(k)) => {
                    // scale the curve 1.25x about its max, average, middle (0.5) or min
                    let ys: Vec<f32> = curve.pts.iter().map(|p| p.y).collect();
                    let pivot = match k {
                        0 => ys.iter().cloned().fold(0.0, f32::max),
                        1 => ys.iter().sum::<f32>() / ys.len().max(1) as f32,
                        2 => 0.5,
                        _ => ys.iter().cloned().fold(1.0, f32::min),
                    };
                    for p in &mut curve.pts {
                        p.y = (pivot + (p.y - pivot) * 1.25).clamp(0.0, 1.0);
                    }
                    changed = true;
                }
                (Some(7), _) => {
                    for p in &mut curve.pts {
                        p.y = 1.0 - p.y;
                    }
                    changed = true;
                }
                (Some(8), _) => {
                    let mut pk: Vec<(Vec2, u8)> = curve.pts.iter().enumerate().map(|(i, p)| (vec2(1.0 - p.x, p.y), curve.kind(i))).collect();
                    pk.reverse();
                    curve.pts = pk.iter().map(|x| x.0).collect();
                    curve.kinds = pk.iter().map(|x| x.1).collect();
                    self.curve_sel.clear();
                    changed = true;
                }
                (Some(10), _) => self.curve_sel = (0..curve.pts.len()).collect(),
                (Some(11), _) => self.curve_clip = Some(curve.clone()),
                (Some(12), _) => {
                    if let Some(c) = self.curve_clip.clone() {
                        curve = c;
                        self.curve_sel.clear();
                        changed = true;
                    }
                }
                (Some(14), _) => {
                    self.nodes[idx].p.curves.remove(&key);
                    self.dirty = true;
                    close = true;
                }
                _ => {}
            }
        }
        // the curve and its points
        for k in 0..160 {
            let (x0, x1) = (k as f32 / 160.0, (k + 1) as f32 / 160.0);
            let (a, b) = (to_s(vec2(x0, curve.eval(x0))), to_s(vec2(x1, curve.eval(x1))));
            draw_line(a.x, a.y, b.x, b.y, 2.0, colr);
        }
        for (i, p) in curve.pts.iter().enumerate() {
            let s = to_s(*p);
            let picked = self.curve_sel.contains(&i);
            let hot = hover_pt == Some(i);
            let rad = if hot || picked { 5.5 } else { 4.0 };
            match curve.kind(i) {
                POINT_STEP => draw_rectangle(s.x - rad, s.y - rad, rad * 2.0, rad * 2.0, if picked { WHITE } else { colr }),
                POINT_COLLAPSED => draw_poly(s.x, s.y, 4, rad * 1.3, 45.0, if picked { WHITE } else { colr }),
                _ => draw_circle(s.x, s.y, rad, if picked { WHITE } else { colr }),
            }
            if hot {
                draw_circle_lines(s.x, s.y, rad + 2.0, 1.0, WHITE);
            }
        }
        // the level choice
        if lv != lv_before {
            if lv == 0 {
                self.nodes[idx].p.curve_levels.remove(&key);
            } else {
                self.nodes[idx].p.curve_levels.insert(key.clone(), lv as u8 + 1);
            }
            self.dirty = true;
        }
        if changed && !close {
            curve.fix_kinds();
            self.nodes[idx].p.curves.insert(key, curve);
            self.dirty = true;
        }
        // unpinned: a click elsewhere puts it away
        // (a click on one of its own lists - Presets, Parent, the right-click menu - hangs outside
        // the window but belongs to it, picture 35)
        let own_list_was_open = self.curve_popup_open;
        self.curve_popup_open = ui.popup_open_for("curve.presets") || ui.popup_open_for("curve.level") || ui.popup_open_for("curve.ctx");
        let outside = ui.inp.l_pressed && !r.contains(m) && zone == 0 && ui.popup.is_none() && !own_list_was_open;
        if outside && !self.curve_pin && !self.curve_just_opened {
            close = true;
        }
        self.curve_just_opened = false;
        if close {
            self.curve_edit = None;
            self.curve_drag = None;
            self.curve_sel.clear();
        }
        // input over the window stays with it (except while one of the lists is open: the list
        // reads its click at the end of the frame, picture 36)
        if (r.contains(m) || self.curve_win_drag != 0 || self.curve_drag.is_some()) && ui.popup.is_none() {
            ui.blocked = true;
            ui.inp.l_pressed = false;
            ui.inp.r_pressed = false;
            ui.inp.keys.clear();
        } else {
            ui.blocked = false;
        }
    }
}

/// The original's curve presets (fn 0x1403a3870), in its order.
pub const CURVE_PRESETS: [&str; 23] = [
    "Max", "Min", "Half", "Linear growth", "Linear decay", "Exp growth", "Exp decay", "Rise", "Droop", "Round", "Bell", "S-Curve", "Conical", "Fade in/out",
    "Season - blossom", "Season - spring", "Season - summer", "Season - transition to autumn", "Season - autumn", "Season - dead",
    "Resolution - standard", "Resolution - aggressive", "Resolution - draft knockout",
];

/// A preset's shape (the names are the original's; the shapes are TreeCraft's).
pub fn preset_curve(i: usize) -> Curve {
    let sampled = |f: &dyn Fn(f32) -> f32, n: usize| Curve { pts: (0..=n).map(|k| { let x = k as f32 / n as f32; vec2(x, f(x).clamp(0.0, 1.0)) }).collect(), kinds: vec![] };
    match i {
        0 => Curve::from_points(&[(0.0, 1.0), (1.0, 1.0)]),
        1 => Curve::from_points(&[(0.0, 0.0), (1.0, 0.0)]),
        2 => Curve::from_points(&[(0.0, 0.5), (1.0, 0.5)]),
        3 => Curve { pts: vec![vec2(0.0, 0.0), vec2(1.0, 1.0)], kinds: vec![POINT_COLLAPSED, POINT_COLLAPSED] },
        4 => Curve { pts: vec![vec2(0.0, 1.0), vec2(1.0, 0.0)], kinds: vec![POINT_COLLAPSED, POINT_COLLAPSED] },
        5 => sampled(&|x| x * x, 6),
        6 => sampled(&|x| (1.0 - x) * (1.0 - x), 6),
        7 => Curve::from_points(&[(0.0, 0.0), (0.3, 0.8), (1.0, 1.0)]),
        8 => Curve::from_points(&[(0.0, 1.0), (0.7, 0.8), (1.0, 0.0)]),
        9 => sampled(&|x| (1.0 - (2.0 * x - 1.0).powi(2)).max(0.0).sqrt(), 8),
        10 => sampled(&|x| (-((x - 0.5) / 0.2).powi(2) * 0.5).exp(), 8),
        11 => sampled(&|x| x * x * (3.0 - 2.0 * x), 6),
        12 => Curve { pts: vec![vec2(0.0, 1.0), vec2(1.0, 0.0)], kinds: vec![POINT_SMOOTH, POINT_SMOOTH] }.with_mid(0.5, 0.62),
        13 => Curve::from_points(&[(0.0, 0.0), (0.15, 1.0), (0.85, 1.0), (1.0, 0.0)]),
        14 => Curve::from_points(&[(0.0, 0.0), (0.1, 1.0), (0.25, 0.0), (1.0, 0.0)]),
        15 => Curve::from_points(&[(0.0, 0.2), (0.25, 1.0), (0.5, 0.6), (1.0, 0.4)]),
        16 => Curve::from_points(&[(0.0, 0.4), (0.5, 1.0), (1.0, 0.4)]),
        17 => Curve::from_points(&[(0.0, 0.0), (0.5, 0.0), (0.75, 1.0), (1.0, 1.0)]),
        18 => Curve::from_points(&[(0.0, 0.0), (0.6, 0.0), (0.85, 1.0), (1.0, 0.8)]),
        19 => Curve::from_points(&[(0.0, 0.0), (0.85, 0.0), (1.0, 1.0)]),
        20 => Curve::from_points(&[(0.0, 1.0), (0.5, 0.5), (1.0, 0.25)]),
        21 => Curve::from_points(&[(0.0, 1.0), (0.3, 0.3), (1.0, 0.1)]),
        _ => Curve::from_points(&[(0.0, 1.0), (0.15, 0.2), (1.0, 0.0)]),
    }
}

trait WithMid {
    fn with_mid(self, x: f32, y: f32) -> Self;
}
impl WithMid for Curve {
    fn with_mid(mut self, x: f32, y: f32) -> Self {
        self.pts.insert(1, vec2(x, y));
        self.kinds.insert(1, POINT_SMOOTH);
        self
    }
}

/// Insert a point of `kind` at `q` (the original refuses ones too close to another point).
fn insert_point(c: &mut Curve, q: Vec2, kind: u8, sel: &mut Vec<usize>) -> bool {
    c.fix_kinds();
    if c.pts.iter().any(|p| (p.x - q.x).abs() < 0.01) {
        return false;
    }
    let at = c.pts.iter().position(|p| p.x > q.x).unwrap_or(c.pts.len());
    if at == 0 || at >= c.pts.len() {
        return false;
    }
    c.pts.insert(at, q);
    c.kinds.insert(at, kind);
    *sel = vec![at];
    true
}

/// Delete the picked points (never the two ends: "The last two control points cannot be deleted").
fn delete_points(c: &mut Curve, sel: &mut Vec<usize>) -> bool {
    c.fix_kinds();
    let n = c.pts.len();
    let mut del: Vec<usize> = sel.iter().cloned().filter(|&i| i > 0 && i + 1 < n).collect();
    del.sort_unstable();
    del.dedup();
    for &i in del.iter().rev() {
        c.pts.remove(i);
        c.kinds.remove(i);
    }
    sel.clear();
    !del.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_curve_preset_is_a_whole_curve() {
        for i in 0..CURVE_PRESETS.len() {
            let c = preset_curve(i);
            assert!(c.pts.len() >= 2, "{}", CURVE_PRESETS[i]);
            assert!(c.pts[0].x.abs() < 1e-6 && (c.pts[c.pts.len() - 1].x - 1.0).abs() < 1e-6, "{} spans 0..1", CURVE_PRESETS[i]);
            assert!(c.pts.windows(2).all(|w| w[1].x > w[0].x), "{} rises in x", CURVE_PRESETS[i]);
            assert!(c.kinds.is_empty() || c.kinds.len() == c.pts.len(), "{} kinds", CURVE_PRESETS[i]);
            for k in 0..=20 {
                let y = c.eval(k as f32 / 20.0);
                assert!((0.0..=1.0).contains(&y), "{} stays within 0..1", CURVE_PRESETS[i]);
            }
        }
    }

    #[test]
    fn a_ray_finds_how_close_it_passes_a_branch() {
        let (d, s, t) = ray_segment(vec3(0.0, 0.0, 10.0), vec3(0.0, 0.0, -1.0), vec3(0.0, -1.0, 0.0), vec3(0.0, 1.0, 0.0));
        assert!(d < 1e-5 && (s - 10.0).abs() < 1e-4 && (t - 0.5).abs() < 1e-4);
        let (d, _, _) = ray_segment(vec3(0.5, 0.3, 10.0), vec3(0.0, 0.0, -1.0), vec3(0.0, -1.0, 0.0), vec3(0.0, 1.0, 0.0));
        assert!((d - 0.5).abs() < 1e-4);
        // a leaf: a single point
        let (d, s, _) = ray_segment(vec3(0.0, 0.2, 5.0), vec3(0.0, 0.0, -1.0), vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0));
        assert!((d - 0.2).abs() < 1e-4 && (s - 5.0).abs() < 1e-4);
    }
}
