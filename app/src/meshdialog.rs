//! The "Meshes" add/remove dialog opened by the +/- button, matching the original's
//! CAssetAddRemoveDialog (texts and tooltips set in fn 0x14004e3a0, 8.4.2): Add new, Duplicate,
//! Rename, Move up, Move down, Remove, a list of names, and OK / Cancel. Changes apply on OK.
use crate::app::{App, Sel};
use crate::model::*;
use crate::ui::*;
use macroquad::prelude::*;
use crate::model::Material;

pub struct AssetDialog<T: AssetItem> {
    /// (original index, item) in the order shown; None = added in this dialog.
    pub items: Vec<(Option<usize>, T)>,
    /// Window title ("Meshes" / "Materials") and the name Add new starts from.
    pub title: String,
    pub default_name: String,
    /// What Add new copies.
    template: T,
    pub selected: Vec<bool>,
    pub current: Option<usize>,
    pub editing: Option<(usize, String)>,
    pub pos: Vec2,
    drag: Option<Vec2>,
    caret_t: f64,
}

/// The original's naming rule (fn 0x1400509b0): while the name is taken, if its last word is a
/// number add one to it, otherwise append " 2".
pub fn unique_name_in(names: &[String], base: &str) -> String {
    let taken = |s: &str| names.iter().any(|n| n == s);
    let mut words: Vec<String> = base.split(' ').map(|w| w.to_string()).collect();
    let mut name = base.to_string();
    let mut guard = 0;
    while taken(&name) && guard < 10000 {
        guard += 1;
        match words.last().and_then(|w| w.parse::<i64>().ok()) {
            Some(n) => {
                let last = words.len() - 1;
                words[last] = (n + 1).to_string();
            }
            None => words.push("2".to_string()),
        }
        name = words.join(" ");
    }
    name
}

pub type MeshDialog = AssetDialog<LeafMesh>;
pub type MaterialDialog = AssetDialog<Material>;

const W: f32 = 490.0;
const H: f32 = 380.0;

impl<T: AssetItem> AssetDialog<T> {
    pub fn open(list: &[T], current: usize, sw: f32, sh: f32, title: &str, default_name: &str, template: T) -> Self {
        let n = list.len();
        let mut selected = vec![false; n];
        let cur = if n > 0 { Some(current.min(n - 1)) } else { None };
        if let Some(c) = cur {
            selected[c] = true;
        }
        AssetDialog {
            items: list.iter().cloned().enumerate().map(|(i, m)| (Some(i), m)).collect(),
            title: title.to_string(),
            default_name: default_name.to_string(),
            template,
            selected,
            current: cur,
            editing: None,
            pos: vec2((sw - W) * 0.5, (sh - H) * 0.4),
            drag: None,
            caret_t: get_time(),
        }
    }

    fn unique_name(&self, base: &str) -> String {
        let names: Vec<String> = self.items.iter().map(|(_, m)| m.asset_name().to_string()).collect();
        unique_name_in(&names, base)
    }

    fn select_only(&mut self, i: usize) {
        for s in &mut self.selected {
            *s = false;
        }
        if i < self.selected.len() {
            self.selected[i] = true;
            self.current = Some(i);
        }
    }

    /// Add new: a copy of the template (an empty mesh, a blank material) with a unique name.
    fn add_new(&mut self) {
        let base = self.default_name.clone();
        let name = self.unique_name(&base);
        let mut m = self.template.clone();
        m.set_asset_name(name);
        self.items.push((None, m));
        self.selected.push(false);
        let i = self.items.len() - 1;
        self.select_only(i);
    }

    /// Duplicate (fn 0x1405ff5d0 case 4): the copy is named "<name> Copy", added at the end of the
    /// list, and its name is opened for editing straight away.
    fn duplicate(&mut self) {
        if let Some(c) = self.current {
            let mut m = self.items[c].1.clone();
            let n = self.unique_name(&format!("{} Copy", m.asset_name()));
            m.set_asset_name(n);
            self.items.push((None, m));
            self.selected.push(false);
            let i = self.items.len() - 1;
            self.select_only(i);
            self.start_rename();
        }
    }

    /// Move up / Move down (fn 0x1405ff5d0 cases 0 and 1): every selected item moves one place,
    /// and stays selected.
    fn move_by(&mut self, d: i32) {
        let n = self.items.len();
        if n < 2 {
            return;
        }
        let order: Vec<usize> = if d < 0 { (1..n).collect() } else { (0..n - 1).rev().collect() };
        for i in order {
            let t = (i as i32 + d) as usize;
            if self.selected[i] && !self.selected[t] {
                self.items.swap(i, t);
                self.selected.swap(i, t);
                if self.current == Some(i) {
                    self.current = Some(t);
                } else if self.current == Some(t) {
                    self.current = Some(i);
                }
            }
        }
    }

    fn remove_selected(&mut self) {
        let mut i = 0;
        while i < self.items.len() {
            if self.selected[i] {
                self.items.remove(i);
                self.selected.remove(i);
            } else {
                i += 1;
            }
        }
        self.current = None;
        if !self.items.is_empty() {
            let c = 0;
            self.select_only(c);
        }
    }

    /// Rename edits the first selected item (fn 0x1405ff5d0 case 3).
    fn start_rename(&mut self) {
        if let Some(c) = self.selected.iter().position(|s| *s).or(self.current) {
            self.editing = Some((c, self.items[c].1.asset_name().to_string()));
            self.caret_t = get_time();
        }
    }

    fn commit_rename(&mut self) {
        if let Some((i, text)) = self.editing.take() {
            let t = text.trim();
            if !t.is_empty() && i < self.items.len() {
                self.items[i].1.set_asset_name(t.to_string());
            }
        }
    }
}

impl App {
    pub fn open_mesh_dialog(&mut self, sw: f32, sh: f32) {
        // a new mesh starts empty: no cutout or file in any level, so the preview is blank
        let mut t = LeafMesh::empty("Mesh");
        t.ref_material = self.mat_cur.min(self.materials.len().saturating_sub(1)).max(if self.materials.len() > 1 { 1 } else { 0 });
        self.mesh_dialog = Some(MeshDialog::open(&self.meshes, self.mesh_cur, sw, sh, "Meshes", "Mesh", t));
    }

    /// The Materials window (+/- in the Materials tab), same rules as the Meshes window; Add new
    /// makes a blank "Material".
    pub fn open_material_dialog(&mut self, sw: f32, sh: f32) {
        let t = Material::blank("Material");
        self.mat_dialog = Some(MaterialDialog::open(&self.materials, self.mat_cur, sw, sh, "Materials", "Material", t));
    }

    /// Apply the dialog's list: new order, names, additions and removals. References from
    /// leaf generators, "More" links and material cutouts follow their meshes.
    fn apply_mesh_dialog(&mut self, d: MeshDialog) {
        let old_len = self.meshes.len();
        // old index -> new index (None when removed)
        let mut map: Vec<Option<usize>> = vec![None; old_len];
        for (ni, (oi, _)) in d.items.iter().enumerate() {
            if let Some(o) = oi {
                if *o < old_len {
                    map[*o] = Some(ni);
                }
            }
        }
        let remap = |k: usize| -> usize {
            if k == 0 || k > old_len {
                0
            } else {
                map[k - 1].map(|n| n + 1).unwrap_or(0)
            }
        };
        let mut meshes: Vec<LeafMesh> = d.items.into_iter().map(|(_, m)| m).collect();
        for m in &mut meshes {
            m.more = remap(m.more);
        }
        for n in &mut self.nodes {
            n.p.leaf_mesh = remap(n.p.leaf_mesh);
        }
        for mat in &mut self.materials {
            mat.cutout_mesh = remap(mat.cutout_mesh);
            for row in &mut mat.mesh_rows {
                *row = remap(*row);
            }
        }
        self.meshes = meshes;
        // like the original (fn 0x1400860d0), the panel stays on the mesh that was showing before,
        // when it still exists; otherwise on the first mesh
        let prev = self.mesh_cur;
        self.mesh_cur = map.get(prev).copied().flatten().unwrap_or(0).min(self.meshes.len().saturating_sub(1));
        self.mesh_lod = 0;
        if self.meshes.is_empty() {
            if matches!(self.sel, Sel::Mesh(_)) {
                self.select(Sel::None);
            }
        } else {
            self.select(Sel::Mesh(self.mesh_cur));
        }
        self.dirty = true;
    }

    /// Apply the Materials window on OK: new order, names, additions and removals. Branch and leaf
    /// generators and mesh reference materials keep pointing at the same materials; removed ones
    /// fall back to the first material.
    fn apply_material_dialog(&mut self, d: MaterialDialog) {
        let old_len = self.materials.len();
        let mut map: Vec<Option<usize>> = vec![None; old_len];
        for (ni, (oi, _)) in d.items.iter().enumerate() {
            if let Some(o) = oi {
                if *o < old_len {
                    map[*o] = Some(ni);
                }
            }
        }
        let remap = |k: usize| -> usize { map.get(k).copied().flatten().unwrap_or(0) };
        let mut mats: Vec<Material> = d.items.into_iter().map(|(_, m)| m).collect();
        for m in &mut mats {
            crate::textures::refresh_material(m);
            m.set_members = m.set_members.iter().filter(|(k, _)| map.get(*k).copied().flatten().is_some()).map(|(k, w)| (remap(*k), *w)).collect();
        }
        for n in &mut self.nodes {
            n.p.material = remap(n.p.material);
            n.p.leaf_material = remap(n.p.leaf_material);
            // Materials lists: entries whose material went are dropped; Inherited stays
            for list in [&mut n.p.mat_list, &mut n.p.ext_list] {
                list.retain(|(k, _)| *k == INHERITED || map.get(*k).copied().flatten().is_some());
                for (k, _) in list.iter_mut() {
                    if *k != INHERITED {
                        *k = remap(*k);
                    }
                }
            }
        }
        for m in &mut self.meshes {
            m.ref_material = remap(m.ref_material);
        }
        self.materials = mats;
        let prev = self.mat_cur;
        self.mat_cur = remap(prev).min(self.materials.len().saturating_sub(1));
        if self.materials.is_empty() {
            if matches!(self.sel, Sel::Material(_)) {
                self.select(Sel::None);
            }
        } else {
            self.select(Sel::Material(self.mat_cur));
        }
        self.dirty = true;
    }

    /// Draw whichever add/remove dialog is open on top of everything. Returns true while open.
    pub fn draw_mesh_dialog(&mut self, ui: &mut Ui) -> bool {
        if let Some(mut d) = self.mesh_dialog.take() {
            match draw_dialog(&mut d, ui) {
                Outcome::Ok => self.apply_mesh_dialog(d),
                Outcome::Cancel => {}
                Outcome::Open => self.mesh_dialog = Some(d),
            }
            return true;
        }
        if let Some(mut d) = self.mat_dialog.take() {
            match draw_dialog(&mut d, ui) {
                Outcome::Ok => self.apply_material_dialog(d),
                Outcome::Cancel => {}
                Outcome::Open => self.mat_dialog = Some(d),
            }
            return true;
        }
        false
    }
}

pub enum Outcome {
    Open,
    Ok,
    Cancel,
}

/// The add/remove window itself (same for meshes and materials).
fn draw_dialog<T: AssetItem>(d: &mut AssetDialog<T>, ui: &mut Ui) -> Outcome {
        ui.blocked = false;
        let m = ui.mouse();
        let r = Rect::new(d.pos.x, d.pos.y, W, H);
        // shadow, frame, title bar
        draw_rectangle(r.x + 6.0, r.y + 6.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.4));
        draw_rectangle(r.x, r.y, r.w, r.h, C_PANEL);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(1.0, 1.0, 1.0, 0.2));
        let title = Rect::new(r.x, r.y, r.w, 32.0);
        draw_rectangle(title.x, title.y, title.w, title.h, Color::new(0.93, 0.93, 0.93, 1.0));
        // small leaf mark in place of the original's window icon
        draw_rectangle(r.x + 10.0, r.y + 9.0, 14.0, 14.0, Color::new(0.45, 0.72, 0.3, 1.0));
        draw_rectangle(r.x + 13.0, r.y + 12.0, 8.0, 8.0, Color::new(0.93, 0.93, 0.93, 1.0));
        let ttl = d.title.clone();
        ui.text(&ttl, r.x + 32.0, r.y + 8.0, 14.0, Color::new(0.35, 0.35, 0.35, 1.0));
        let close = Rect::new(r.x + r.w - 34.0, r.y + 4.0, 26.0, 24.0);
        let close_hot = close.contains(m);
        if close_hot {
            draw_rectangle(close.x, close.y, close.w, close.h, Color::new(0.9, 0.25, 0.25, 1.0));
        }
        let xc = if close_hot { WHITE } else { Color::new(0.4, 0.4, 0.4, 1.0) };
        let (cx, cy) = (close.x + close.w * 0.5, close.y + close.h * 0.5);
        draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.3, xc);
        draw_line(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.3, xc);
        ui.record(&format!("dlg:{}", d.title), r);

        let mut close_dialog = false;
        let mut apply = false;
        if ui.inp.l_pressed && close_hot {
            close_dialog = true;
        }
        // drag the window by its title bar
        if ui.inp.l_pressed && title.contains(m) && !close_hot {
            d.drag = Some(m - d.pos);
        }
        if let Some(off) = d.drag {
            if ui.inp.l_down {
                d.pos = m - off;
                d.pos.x = d.pos.x.clamp(0.0, (ui.sw - 60.0).max(0.0));
                d.pos.y = d.pos.y.clamp(0.0, (ui.sh - 40.0).max(0.0));
            } else {
                d.drag = None;
            }
        }

        // left column of buttons
        let bx = r.x + 6.0;
        let bw = 96.0;
        let mut by = r.y + 40.0;
        let has_cur = d.current.is_some();
        let tip = |ui: &mut Ui, rr: Rect, t: &str| {
            if rr.contains(ui.mouse()) {
                ui.tooltip = Some(t.to_string());
            }
        };
        let br = Rect::new(bx, by, bw, 28.0);
        if ui.button("dlg.addnew", br, "Add new") {
            d.commit_rename();
            d.add_new();
        }
        tip(ui, br, "Add new asset");
        by += 34.0;
        let br = Rect::new(bx, by, bw, 28.0);
        if ui.button_ex("dlg.duplicate", br, "Duplicate", false, has_cur) {
            d.commit_rename();
            d.duplicate();
        }
        tip(ui, br, "Duplicate selected");
        by += 34.0;
        let br = Rect::new(bx, by, bw, 28.0);
        if ui.button_ex("dlg.rename", br, "Rename", false, has_cur) {
            d.commit_rename();
            d.start_rename();
        }
        tip(ui, br, "Rename selected");
        by += 34.0;
        let arrow = |b: Rect, up: bool, on: bool| {
            let c = if on { Color::new(0.85, 0.85, 0.85, 1.0) } else { Color::new(0.45, 0.45, 0.45, 1.0) };
            let (cx, cy) = (b.x + b.w * 0.5, b.y + b.h * 0.5);
            let s = 6.0;
            if up {
                draw_triangle(vec2(cx - s, cy + s * 0.5), vec2(cx + s, cy + s * 0.5), vec2(cx, cy - s * 0.6), c);
            } else {
                draw_triangle(vec2(cx - s, cy - s * 0.5), vec2(cx + s, cy - s * 0.5), vec2(cx, cy + s * 0.6), c);
            }
        };
        let up = Rect::new(bx + 8.0, by, 32.0, 28.0);
        let can_up = d.selected.iter().skip(1).any(|s| *s);
        if ui.button_ex("dlg.up", up, "", false, can_up) {
            d.commit_rename();
            d.move_by(-1);
        }
        arrow(up, true, can_up);
        tip(ui, up, "Move up");
        let dn = Rect::new(bx + 54.0, by, 32.0, 28.0);
        let can_dn = d.selected.len() > 1 && d.selected[..d.selected.len() - 1].iter().any(|s| *s);
        if ui.button_ex("dlg.down", dn, "", false, can_dn) {
            d.commit_rename();
            d.move_by(1);
        }
        arrow(dn, false, can_dn);
        tip(ui, dn, "Move down");
        let any_sel = d.selected.iter().any(|s| *s);
        let br = Rect::new(bx, r.y + r.h - 82.0, bw, 28.0);
        if ui.button_ex("dlg.remove", br, "Remove", false, any_sel) {
            d.editing = None;
            d.remove_selected();
        }
        tip(ui, br, "Remove selected assets");

        // the list
        let list = Rect::new(r.x + bw + 12.0, r.y + 38.0, r.w - bw - 18.0, r.h - 86.0);
        draw_rectangle(list.x, list.y, list.w, list.h, Color::new(0.27, 0.27, 0.27, 1.0));
        let rh = 22.0;
        let mut clicked_row: Option<usize> = None;
        let mut double_row: Option<usize> = None;
        for (i, (_, mesh)) in d.items.iter().enumerate() {
            let rr = Rect::new(list.x + 2.0, list.y + 2.0 + i as f32 * rh, list.w - 4.0, rh);
            if rr.y + rr.h > list.y + list.h {
                break;
            }
            ui.record(&format!("dlg.row:{}", mesh.asset_name()), rr);
            if d.selected[i] {
                draw_rectangle(rr.x, rr.y, rr.w, rr.h, C_ACCENT);
            } else if rr.contains(m) {
                draw_rectangle(rr.x, rr.y, rr.w, rr.h, Color::new(1.0, 1.0, 1.0, 0.07));
            }
            let editing_this = matches!(&d.editing, Some((e, _)) if *e == i);
            if !editing_this {
                let name = mesh.asset_name().to_string();
                ui.text_fit(&name, rr.x + 6.0, rr.y + 4.0, rr.w - 10.0, 13.0, if d.selected[i] { WHITE } else { C_TEXT });
            }
            if ui.inp.l_pressed && rr.contains(m) {
                if ui.inp.l_double {
                    double_row = Some(i);
                } else {
                    clicked_row = Some(i);
                }
            }
        }
        if d.items.is_empty() {
            ui.text(&format!("No {}. Use Add new.", d.title.to_lowercase()), list.x + 8.0, list.y + 8.0, 12.5, C_TEXT_FAINT);
        }
        if let Some(i) = clicked_row {
            if !matches!(&d.editing, Some((e, _)) if *e == i) {
                d.commit_rename();
            }
            if ui.inp.ctrl {
                d.selected[i] = !d.selected[i];
                d.current = Some(i);
            } else if ui.inp.shift && d.current.is_some() {
                let c = d.current.unwrap();
                let (a, b) = (c.min(i), c.max(i));
                for k in 0..d.selected.len() {
                    d.selected[k] = k >= a && k <= b;
                }
            } else {
                d.select_only(i);
            }
        }
        if let Some(i) = double_row {
            d.commit_rename();
            d.select_only(i);
            d.start_rename();
        }
        // inline name editor
        if let Some((i, mut text)) = d.editing.take() {
            let rr = Rect::new(list.x + 2.0, list.y + 2.0 + i as f32 * rh, list.w - 4.0, rh);
            for c in &ui.inp.chars {
                if text.chars().count() < 64 {
                    text.push(*c);
                }
            }
            let mut done = None;
            if ui.inp.key(KeyCode::Backspace) {
                text.pop();
                d.caret_t = get_time();
            }
            if ui.inp.key(KeyCode::Enter) || ui.inp.key(KeyCode::KpEnter) {
                done = Some(true);
            }
            if ui.inp.key(KeyCode::Escape) {
                done = Some(false);
            }
            draw_rectangle(rr.x, rr.y, rr.w, rr.h, Color::new(0.12, 0.12, 0.12, 1.0));
            draw_rectangle_lines(rr.x, rr.y, rr.w, rr.h, 1.0, C_ACCENT2);
            ui.text_fit(&text, rr.x + 6.0, rr.y + 4.0, rr.w - 14.0, 13.0, WHITE);
            if ((get_time() - d.caret_t) * 2.0) as i64 % 2 == 0 {
                let tw = ui.text_w(&text, 13.0).min(rr.w - 14.0);
                draw_line(rr.x + 7.0 + tw, rr.y + 4.0, rr.x + 7.0 + tw, rr.y + rr.h - 4.0, 1.0, WHITE);
            }
            ui.record("dlg.edit", rr);
            match done {
                Some(true) => {
                    d.editing = Some((i, text));
                    d.commit_rename();
                }
                Some(false) => {}
                None => d.editing = Some((i, text)),
            }
        } else if ui.inp.key(KeyCode::Escape) {
            close_dialog = true;
        } else if ui.inp.key(KeyCode::F2) {
            d.start_rename();
        } else if ui.inp.key(KeyCode::Delete) && d.selected.iter().any(|s| *s) {
            d.remove_selected();
        }

        // OK / Cancel
        let okr = Rect::new(r.x + r.w - 206.0, r.y + r.h - 40.0, 96.0, 30.0);
        if ui.button_ex("dlg.ok", okr, "OK", true, true) {
            d.commit_rename();
            apply = true;
        }
        let car = Rect::new(r.x + r.w - 104.0, r.y + r.h - 40.0, 96.0, 30.0);
        if ui.button("dlg.cancel", car, "Cancel") {
            close_dialog = true;
        }

        // modal: nothing behind the dialog reacts this frame
        ui.blocked = true;
        ui.inp.l_pressed = false;
        ui.inp.r_pressed = false;
        ui.inp.keys.clear();
        // modal: nothing behind the dialog reacts this frame
        if apply {
            d.commit_rename();
            return Outcome::Ok;
        }
        if close_dialog {
            return Outcome::Cancel;
        }
        Outcome::Open
}