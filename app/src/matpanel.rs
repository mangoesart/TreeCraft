//! The Materials tab, laid out like the original's Materials panel (fn 0x140063f80 in 8.4.2):
//! drag handle, material picker, +/- (Materials window), + (browse for textures, one new
//! material each), copy/paste menu; preview; Settings / Meshes sub-tabs; then the ten texture
//! slots (Color, Opacity, Normal, Gloss, Specular, Metallic, Subsurface, Subsurface%, AO, Custom),
//! each with its value, a tick box and a picture.

use crate::app::{group_box, icon_clipboard, icon_hand, App, Sel};
use crate::model::Material;
use crate::model::*;
use crate::textures;
use crate::ui::*;
use macroquad::prelude::*;

const SUBTABS: [&str; 2] = ["Settings", "Meshes"];
/// Material preview shapes ("Card" is now "Plane").
const PREVIEW_SHAPES: [&str; 4] = ["Sphere", "Plane", "Cylinder", "Custom..."];

impl App {
    pub fn draw_materials(&mut self, ui: &mut Ui, tb: Rect, body: Rect) {
        let r = Rect::new(tb.x, tb.y, tb.w, body.y + body.h - tb.y);
        let x0 = r.x + 8.0;
        let w = r.w - 16.0;
        let y = r.y + 4.0;
        let cur = self.top_row(ui, x0, y, w);
        let top = y + 38.0;
        draw_line(r.x, top - 3.0, r.x + r.w, top - 3.0, 1.0, C_LINE);

        // everything under the top row scrolls
        let area = Rect::new(r.x, top, r.w, r.y + r.h - top);
        let m = ui.mouse();
        if area.contains(m) && !ui.blocked && ui.popup.is_none() && ui.inp.wheel != 0.0 {
            self.mat_scroll = (self.mat_scroll - ui.inp.wheel * 40.0).max(0.0);
        }
        ui.set_clip(Some(area));
        let mut y = top + 4.0 - self.mat_scroll;

        // preview
        let ph = (w * 0.42).clamp(110.0, 230.0);
        let pr = Rect::new(x0, y, w, ph);
        ui.record("matpreview", pr);
        checker(pr, 12.0);
        if cur.is_some() {
            if let Some(rt) = &self.preview_rt {
                draw_texture_ex(&rt.texture, pr.x, pr.y, WHITE, DrawTextureParams { dest_size: Some(vec2(pr.w, pr.h)), flip_y: true, ..Default::default() });
            }
            // preview shape: Sphere, Plane, Cylinder or a mesh file of the person's choosing
            let custom = self.preview_shape == 3;
            let sr = Rect::new(pr.x + pr.w - 112.0 - if custom { 28.0 } else { 0.0 }, pr.y + 4.0, 108.0, 20.0);
            let shapes: Vec<String> = PREVIEW_SHAPES.iter().map(|s| s.to_string()).collect();
            let prev = self.preview_shape;
            if ui.combo("mat.shape", sr, &shapes, &mut self.preview_shape) && self.preview_shape == 3 && !self.load_custom_preview() && self.custom_preview_name.is_empty() {
                self.preview_shape = prev;
            }
            let mut controls = sr;
            if self.preview_shape == 3 {
                let lr = Rect::new(sr.x + sr.w + 4.0, sr.y, 24.0, 20.0);
                if ui.button("mat.shapeload", lr, "...") {
                    self.load_custom_preview();
                }
                if ui.hover(lr) {
                    ui.tooltip = Some(format!("Load another mesh for the preview ({})", self.custom_preview_name));
                }
                controls.w += 28.0;
                ui.text_fit(&self.custom_preview_name.clone(), pr.x + 6.0, pr.y + 6.0, (sr.x - pr.x - 12.0).max(0.0), 11.0, C_TEXT_DIM);
            }
            ui.text("drag to turn • double-click to reset", pr.x + 6.0, pr.y + pr.h - 16.0, 10.5, C_TEXT_FAINT);
            // it turns only when the person drags it; a click also shows the material in Properties
            let m = ui.mouse();
            let over = pr.contains(m) && !controls.contains(m) && !ui.blocked && ui.popup.is_none();
            if over && ui.active == 0 && ui.inp.l_pressed {
                if ui.inp.l_double {
                    // fn 0x1402ecdd0: the view and zoom go back to their start
                    self.mat_yaw = 0.6;
                    self.mat_pitch = 0.25;
                    self.mat_zoom = 0.0;
                } else {
                    self.mat_orbit = Some((m, false));
                    ui.active = hid("matpreview.orbit");
                }
                if let Some(c) = cur {
                    self.select(Sel::Material(c));
                }
            }
            if over && ui.inp.wheel != 0.0 {
                // fn 0x1402ece70: zoom += wheel delta x -0.001 (Qt gives 120 per notch -> 0.12)
                self.mat_zoom = (self.mat_zoom - ui.inp.wheel * 0.12).clamp(-2.2, 8.4);
            }
            if let Some((last, _)) = self.mat_orbit {
                if ui.inp.l_down {
                    let d = m - last;
                    self.mat_yaw -= d.x * 0.01;
                    self.mat_pitch = (self.mat_pitch + d.y * 0.01).clamp(-1.5, 1.5);
                    self.mat_orbit = Some((m, true));
                } else {
                    self.mat_orbit = None;
                }
            }
        }
        if let Some(m) = cur.and_then(|c| self.materials.get(c)).filter(|m| m.is_set) {
            let list: Vec<String> = m.set_members.iter().map(|(k, wgt)| format!("{} ({:.2})", self.materials.get(*k).map(|x| x.name.as_str()).unwrap_or("?"), wgt)).collect();
            let t = format!("Material set: {}", list.join(", "));
            ui.text_fit(&t, pr.x + 6.0, pr.y + pr.h - 18.0, pr.w - 12.0, 12.0, WHITE);
        }
        draw_rectangle_lines(pr.x, pr.y, pr.w, pr.h, 1.0, C_LINE);
        self.preview_aspect = pr.w / pr.h.max(1.0);
        y += ph + 6.0;

        let c = match cur {
            Some(c) => c,
            None => {
                ui.text("No materials. Use +/- to add one, or + to make one from a texture.", x0, y + 6.0, 12.5, C_TEXT_FAINT);
                ui.set_clip(None);
                return;
            }
        };

        // Settings / Meshes sub-tabs
        let mut tx = x0;
        for (i, t) in SUBTABS.iter().enumerate() {
            let tw = ui.text_w(t, 12.5) + 18.0;
            let tr = Rect::new(tx, y, tw, 22.0);
            ui.record(&format!("mattab:{}", t), tr);
            // the app's sub-tab look (pictures 61, 62): plain words, an accent line under the pick
            let hot = ui.hover(tr);
            if hot && i != self.mat_subtab {
                draw_rectangle(tr.x, tr.y, tr.w, tr.h, Color::new(1.0, 1.0, 1.0, 0.05));
            }
            ui.text_center(t, tr, 12.5, if i == self.mat_subtab || hot { WHITE } else { C_TEXT_DIM });
            if i == self.mat_subtab {
                draw_rectangle(tr.x + 4.0, tr.y + tr.h - 2.0, tr.w - 8.0, 2.0, C_ACCENT);
            }
            if ui.clicked(tr) {
                self.mat_subtab = i;
                self.color_edit = None;
            }
            tx += tw + 2.0;
        }
        draw_line(x0, y + 22.0, x0 + w, y + 22.0, 1.0, C_LINE);
        y += 28.0;
        let before = self.materials[c].clone_settings_key();
        let sub_h = if self.mat_subtab == 0 { self.settings_tab(ui, c, x0, y, w) } else { self.meshes_tab(ui, c, x0, y, w) };
        y += sub_h + 8.0;

        // the ten texture slots, sized so all of them fit in the panel below the settings
        // (picture 24): the column count that gives the largest pictures wins
        let gap = 6.0;
        let avail = (area.y + area.h) - (y + self.mat_scroll) - 4.0;
        let fit = |cols: usize| -> f32 {
            let bw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
            let rows = (SLOT_NAMES.len() + cols - 1) / cols;
            let by_h = avail / rows as f32 - gap - (18.0 + 26.0 + 8.0);
            (bw - 12.0).min(by_h).min(200.0)
        };
        let cols = (2..=5usize).filter(|&c| c == 2 || w / c as f32 >= 90.0).max_by(|a, b| fit(*a).total_cmp(&fit(*b))).unwrap_or(2);
        let bw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
        let thumb = fit(cols).max(36.0);
        let bh = 18.0 + 26.0 + thumb + 8.0;
        for s in 0..SLOT_NAMES.len() {
            let bx = x0 + (s % cols) as f32 * (bw + gap);
            let by = y + (s / cols) as f32 * (bh + gap);
            self.slot_box(ui, c, s, Rect::new(bx, by, bw, bh), thumb);
        }
        let rows = (SLOT_NAMES.len() + cols - 1) / cols;
        y += rows as f32 * (bh + gap);
        let content_h = y + self.mat_scroll - top;
        self.mat_scroll = self.mat_scroll.min((content_h - area.h + 10.0).max(0.0));
        ui.set_clip(None);

        // keep the picture and the model up to date with what changed here
        let m = &mut self.materials[c];
        if m.clone_settings_key() != before {
            self.dirty = true;
        }
        if m.dirty_base || m.dirty_tex {
            textures::refresh_material(m);
            self.dirty = true;
        }
    }

    /// Top row: drag handle, material picker, +/-, +, copy/paste. Returns the current material.
    fn top_row(&mut self, ui: &mut Ui, x0: f32, y: f32, w: f32) -> Option<usize> {
        let hand = Rect::new(x0, y, 30.0, 30.0);
        ui.record("btn:mat.drag", hand);
        let hot = ui.hover(hand);
        draw_rectangle(hand.x, hand.y, hand.w, hand.h, if hot || self.mat_drag { C_BTN_HOT } else { C_BTN });
        icon_hand(hand);
        if hot {
            ui.tooltip = Some("Drag this material and drop it on the model or a generator to use it".into());
            if ui.inp.l_pressed && !self.materials.is_empty() {
                self.mat_drag = true;
                ui.active = hid("mat.drag");
            }
        }
        let bw = 34.0;
        let combo_w = (w - 36.0 - (bw + 4.0) * 3.0).max(60.0);
        // fn 0x140068ed0: the picker lists only materials that aren't sets (+0x6cc == 0);
        // with none it shows "No materials" and the panel is disabled.
        let listed: Vec<usize> = (0..self.materials.len()).filter(|i| !self.materials[*i].is_set).collect();
        let names: Vec<String> = if listed.is_empty() { vec!["No materials".into()] } else { listed.iter().map(|i| self.materials[*i].name.clone()).collect() };
        let mut pos = listed.iter().position(|i| *i == self.mat_cur).unwrap_or(0);
        let cr = Rect::new(x0 + 36.0, y + 4.0, combo_w, 22.0);
        if ui.combo("mat.pick", cr, &names, &mut pos) && !listed.is_empty() {
            self.color_edit = None;
            self.select(Sel::Material(listed[pos]));
        }
        let cur = listed.get(pos).copied().unwrap_or(0);
        let bx = cr.x + cr.w + 4.0;
        let r1 = Rect::new(bx, y + 4.0, bw, 22.0);
        if ui.button("mat.addremove", r1, "+/-") {
            self.open_material_dialog(ui.sw, ui.sh);
        }
        if ui.hover(r1) {
            ui.tooltip = Some("Manage the materials.".into());
        }
        let r2 = Rect::new(bx + bw + 4.0, y + 4.0, bw, 22.0);
        if ui.button("mat.browseadd", r2, "+") {
            self.add_materials_from_textures();
        }
        if ui.hover(r2) {
            ui.tooltip = Some("Browse for a texture and add a material for it to the list automatically.".into());
        }
        let r3 = Rect::new(bx + (bw + 4.0) * 2.0, y + 4.0, bw, 22.0);
        if ui.button("mat.copypaste", r3, "") {
            ui.open_popup(hid("mat.copypaste"), r3, vec!["Copy".into(), "Paste".into(), "Duplicate".into()], None);
        }
        icon_clipboard(r3);
        if ui.hover(r3) {
            ui.tooltip = Some("Copy/Paste menu.".into());
        }
        if let Some(i) = ui.take_popup_result("mat.copypaste") {
            if !self.materials.is_empty() {
                match i {
                    0 => {
                        self.mat_clip = Some(self.materials[cur].clone());
                        self.set_status("Material copied");
                    }
                    1 => match self.mat_clip.clone() {
                        Some(mut m) => {
                            m.name = self.materials[cur].name.clone();
                            self.materials[cur] = m;
                            self.dirty = true;
                            self.set_status("Material pasted");
                        }
                        None => self.set_status("Nothing copied yet"),
                    },
                    _ => {
                        let mut m = self.materials[cur].clone();
                        let names: Vec<String> = self.materials.iter().map(|x| x.name.clone()).collect();
                        m.name = crate::meshdialog::unique_name_in(&names, &format!("{} Copy", m.name));
                        self.materials.push(m);
                        self.select(Sel::Material(self.materials.len() - 1));
                    }
                }
            }
        }
        // drag-and-drop the material onto the model
        if self.mat_drag {
            if ui.inp.l_down {
                if let Some(m) = self.materials.get(cur) {
                    ui.tooltip = Some(format!("Drop on the viewport to use \"{}\"", m.name));
                }
            } else {
                self.mat_drag = false;
                if self.viewport.contains(ui.mouse()) && !self.materials.is_empty() {
                    self.drop_material_on_model(cur);
                }
            }
        }
        if listed.is_empty() {
            None
        } else {
            self.mat_cur = cur;
            Some(cur)
        }
    }

    /// The material goes onto the generator last picked in the graph (its leaves for a Leaf
    /// generator, its bark otherwise); with none picked, onto the first leaf or trunk generator.
    fn drop_material_on_model(&mut self, cur: usize) {
        let leafy = self.materials[cur].two_sided;
        let target = self
            .last_node
            .filter(|id| self.node(*id).map(|n| n.kind != Kind::Tree).unwrap_or(false))
            .or_else(|| self.nodes.iter().find(|n| if leafy { n.kind == Kind::Leaf } else { n.kind == Kind::Trunk }).map(|n| n.id));
        let name = self.materials[cur].name.clone();
        match target.and_then(|id| self.nodes.iter_mut().find(|n| n.id == id)) {
            Some(n) => {
                if n.kind == Kind::Leaf {
                    n.p.leaf_material = cur;
                } else {
                    // dropped by hand: the generator's Materials list becomes this one material
                    n.p.material = cur;
                    n.p.mat_list = vec![(cur, 1.0)];
                }
                let nn = n.name.clone();
                self.set_status(&format!("\"{}\" now uses material \"{}\"", nn, name));
                self.dirty = true;
            }
            None => self.set_status("Add a generator first"),
        }
    }

    /// "+": pick texture files; each becomes a new material with that picture in its Color slot.
    pub(crate) fn add_materials_from_textures(&mut self) {
        let files = crate::prefs::file_dialog().add_filter("Images", &["png", "jpg", "jpeg", "tga", "bmp"]).pick_files().unwrap_or_default();
        for f in files {
            let f = f.to_string_lossy().to_string();
            let stem = std::path::Path::new(&f).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or("Material".into());
            let names: Vec<String> = self.materials.iter().map(|x| x.name.clone()).collect();
            let mut m = Material::blank(&crate::meshdialog::unique_name_in(&names, &stem));
            m.source = 4;
            m.file = f.clone();
            m.slots[0].file = f;
            m.dirty_base = true;
            textures::refresh_material(&mut m);
            self.materials.push(m);
            self.select(Sel::Material(self.materials.len() - 1));
        }
    }

    /// Settings sub-tab: Main, Variations, Atlas, Other. Returns its height.
    fn settings_tab(&mut self, ui: &mut Ui, c: usize, x0: f32, y: f32, w: f32) -> f32 {
        let two = w >= 420.0;
        let gw = if two { (w - 8.0) * 0.5 } else { w };
        let col2 = if two { x0 + gw + 8.0 } else { x0 };
        let (y_var, y_atlas, y_other) = if two { (y, y + 92.0, y + 92.0) } else { (y + 92.0, y + 184.0, y + 262.0) };

        // Main
        group_box(ui, Rect::new(x0, y, gw, 86.0), "Main");
        let m = &mut self.materials[c];
        ui.text("Two-sided", x0 + 8.0, y + 20.0, 12.0, C_TEXT_DIM);
        let cb = Rect::new(x0 + 72.0, y + 18.0, 16.0, 16.0);
        if ui.checkbox("mat.twosided", cb, &mut m.two_sided) {
            self.dirty = true;
        }
        if ui.hover(cb) {
            ui.tooltip = Some("Face culling.".into());
        }
        let sw = Rect::new(x0 + gw - 60.0, y + 16.0, 52.0, 20.0);
        ui.text("Season", sw.x - 48.0, y + 20.0, 12.0, C_TEXT_DIM);
        let m = &mut self.materials[c];
        color_field(ui, &mut self.color_edit, (1, 0), sw, &mut m.season_color);
        let nb = Rect::new(x0 + 8.0, y + 50.0, gw - 16.0, 24.0);
        if ui.button("mat.newset", nb, "Make New Set...") {
            // fn 0x14006d8c0 ("add material set"): a new default material named "<name> Set"
            // (made unique by fn 0x1400af740), marked as a set, whose one member is the current
            // material with weight 1.0; it's added at the end and shown.
            let names: Vec<String> = self.materials.iter().map(|x| x.name.clone()).collect();
            let base = c;
            let mut s = Material::blank(&crate::meshdialog::unique_name_in(&names, &format!("{} Set", self.materials[c].name)));
            s.is_set = true;
            s.set_members = vec![(base, 1.0)];
            textures::refresh_material(&mut s);
            let sn = s.name.clone();
            self.materials.push(s);
            // sets aren't listed in this panel's picker, so the current material stays shown
            self.set_status(&format!("Made material set \"{}\"", sn));
            self.dirty = true;
            return if two { 184.0 } else { 340.0 };
        }

        // Variations
        group_box(ui, Rect::new(col2, y_var, gw, 86.0), "Variations");
        let vw = (gw - 16.0 - 8.0) / 3.0;
        let m = &mut self.materials[c];
        for i in 0..3 {
            let vr = Rect::new(col2 + 8.0 + i as f32 * (vw + 4.0), y_var + 16.0, vw, 20.0);
            color_field(ui, &mut self.color_edit, (2, i), vr, &mut m.var_colors[i]);
        }
        ui.text("Amount", col2 + 8.0, y_var + 54.0, 12.0, C_TEXT_DIM);
        // fn 0x140068ed0: slider position = Amount squared x 10000 (constant 0x141031c10)
        let mut pos = m.variation * m.variation;
        if ui.slider("mat.varamount", Rect::new(col2 + 60.0, y_var + 50.0, gw - 68.0, 20.0), &mut pos, 0.0, 1.0, 2) {
            m.variation = pos.max(0.0).sqrt();
        }

        // Atlas
        group_box(ui, Rect::new(x0, y_atlas, gw, 72.0), "Atlas");
        ui.text("Scale", x0 + 8.0, y_atlas + 20.0, 12.0, C_TEXT_DIM);
        let sr = Rect::new(x0 + 50.0, y_atlas + 16.0, gw - 58.0, 20.0);
        ui.slider("mat.atlasscale", sr, &mut m.atlas_scale, 0.0, 4.0, 2);
        if ui.hover(sr) {
            ui.tooltip = Some("Scale the size of this material in unwrapped maps.".into());
        }
        ui.text("Atlas", x0 + 8.0, y_atlas + 46.0, 12.0, C_TEXT_DIM);
        let modes: Vec<String> = ATLAS_MODES.iter().map(|s| s.to_string()).collect();
        ui.combo("mat.atlasmode", Rect::new(x0 + 50.0, y_atlas + 42.0, gw - 58.0, 20.0), &modes, &mut m.atlas_mode);

        // Other
        group_box(ui, Rect::new(col2, y_other, gw, 72.0), "Other");
        ui.text("Max resolution", col2 + 8.0, y_other + 20.0, 12.0, C_TEXT_DIM);
        let res: Vec<String> = MAX_RESOLUTIONS.iter().map(|s| s.to_string()).collect();
        let lx = col2 + 100.0;
        ui.combo("mat.maxres", Rect::new(lx, y_other + 16.0, gw - 108.0, 20.0), &res, &mut m.max_res);
        ui.text("User data", col2 + 8.0, y_other + 46.0, 12.0, C_TEXT_DIM);
        let ur = Rect::new(lx, y_other + 42.0, gw - 108.0, 20.0);
        text_field(ui, "mat.userdata", ur, &mut m.user_data, &mut self.mat_text_focus);
        if ui.hover(ur) {
            ui.tooltip = Some("String associated with this material that can be retrieved via the SDK.".into());
        }
        if two {
            184.0
        } else {
            340.0
        }
    }

    fn set_mesh_row(&mut self, c: usize, i: usize, v: usize) {
        if i == 0 {
            self.materials[c].cutout_mesh = v;
        } else {
            self.materials[c].mesh_rows[i - 1] = v;
        }
        if v > 0 {
            // the mesh's reference picture follows the material it's assigned to
            self.meshes[v - 1].ref_material = c;
        }
        self.dirty = true;
    }

    /// Meshes sub-tab: Unwrap, Add, the cutout-mesh row and one row per supplemental mesh.
    fn meshes_tab(&mut self, ui: &mut Ui, c: usize, x0: f32, y: f32, w: f32) -> f32 {
        ui.text("Unwrap", x0, y + 4.0, 12.0, C_TEXT_DIM);
        let modes: Vec<String> = UNWRAP_MODES.iter().map(|s| s.to_string()).collect();
        ui.combo("mat.unwrap", Rect::new(x0 + 52.0, y, 90.0, 22.0), &modes, &mut self.materials[c].unwrap);
        let ar = Rect::new(x0 + w - 80.0, y, 80.0, 22.0);
        if ui.button("mat.meshadd", ar, "Add") {
            self.materials[c].mesh_rows.push(0);
        }
        let mut names: Vec<String> = vec!["None".into()];
        names.extend(self.meshes.iter().map(|m| m.name.clone()));
        let mut ry = y + 28.0;
        let mut remove = None;
        // fn 0x140068ed0: one row more than the supplemental meshes; row 0 is the material's
        // cutout mesh (+0x128) and its delete button is disabled and blank, the others say "X".
        let n = self.materials[c].mesh_rows.len() + 1;
        for i in 0..n {
            let stored = if i == 0 { self.materials[c].cutout_mesh } else { self.materials[c].mesh_rows[i - 1] };
            let mut v = stored.min(names.len() - 1);
            let cw = w - 60.0 - 28.0 - 26.0 - 12.0;
            if ui.combo(&format!("mat.meshrow{}", i), Rect::new(x0, ry, cw, 22.0), &names, &mut v) {
                self.set_mesh_row(c, i, v);
            }
            let er = Rect::new(x0 + cw + 4.0, ry, 60.0, 22.0);
            if ui.button(&format!("mat.meshedit{}", i), er, "Edit...") {
                // "Generate mesh": with no mesh in the row a new cutout mesh for this material
                // is made first (estimated); then the cutout editor opens on it.
                if v == 0 {
                    let names: Vec<String> = self.meshes.iter().map(|m| m.name.clone()).collect();
                    let mut m = LeafMesh::card(&crate::meshdialog::unique_name_in(&names, &self.materials[c].name));
                    m.ref_material = c;
                    self.meshes.push(m);
                    v = self.meshes.len();
                    self.set_mesh_row(c, i, v);
                }
                self.tab = 2;
                self.mesh_lod = 0;
                self.select(Sel::Mesh(v - 1));
                self.open_cutout_editor(v - 1);
            }
            if ui.hover(er) {
                ui.tooltip = Some("Generate mesh: edit the cutout mesh for this material.".into());
            }
            let gr = Rect::new(er.x + er.w + 4.0, ry, 28.0, 22.0);
            if ui.button_ex(&format!("mat.meshgo{}", i), gr, "", false, v > 0) {
                self.tab = 2;
                self.select(Sel::Mesh(v - 1));
            }
            let (cx, cy) = (gr.x + gr.w * 0.5, gr.y + gr.h * 0.5);
            let ac = Color::new(1.0, 1.0, 1.0, if v > 0 { 0.85 } else { 0.35 });
            draw_rectangle(cx - 7.0, cy - 2.0, 8.0, 4.0, ac);
            draw_triangle(vec2(cx + 7.0, cy), vec2(cx + 1.0, cy - 5.5), vec2(cx + 1.0, cy + 5.5), ac);
            if ui.hover(gr) {
                ui.tooltip = Some("Go to this mesh in the Meshes tab.".into());
            }
            let xr = Rect::new(gr.x + gr.w + 4.0, ry, 22.0, 22.0);
            if ui.button_ex(&format!("mat.meshdel{}", i), xr, if i == 0 { "" } else { "X" }, false, i > 0) {
                remove = Some(i - 1);
            }
            if i > 0 && ui.hover(xr) {
                ui.tooltip = Some("Remove this mesh from the material.".into());
            }
            ry += 26.0;
        }
        if let Some(i) = remove {
            self.materials[c].mesh_rows.remove(i);
            self.dirty = true;
        }        (ry - y).max(120.0)
    }

    /// One texture slot: name, value (colour or number), tick box and picture. Click the picture
    /// to load a texture; right-click to clear it.
    fn slot_box(&mut self, ui: &mut Ui, c: usize, s: usize, r: Rect, thumb: f32) {
        group_box(ui, Rect::new(r.x, r.y, r.w, r.h), SLOT_NAMES[s]);
        let vr = Rect::new(r.x + 6.0, r.y + 18.0, r.w - 12.0, 20.0);
        let m = &mut self.materials[c];
        let changed = if s == 4 {
            // Specular (picture 61): one value, not a colour; the three channels stay equal
            let mut v = (m.specular_color[0] + m.specular_color[1] + m.specular_color[2]) / 3.0;
            let c = ui.slider(&format!("mat.slotval{}", s), vr, &mut v, 0.0, 1.0, 2);
            if c {
                let v = v.clamp(0.0, 1.0);
                m.specular_color = [v, v, v];
            }
            c
        } else if let Some(col) = m.slot_color(s) {
            color_field(ui, &mut self.color_edit, (0, s), vr, col)
        } else if let Some(v) = m.slot_value(s) {
            let max = if s == 2 { 2.0 } else { 1.0 };
            if s == 3 {
                // Roughness: shown as 1 - gloss
                let mut rough = 1.0 - *v;
                let c = ui.slider(&format!("mat.slotval{}", s), vr, &mut rough, 0.0, 1.0, 2);
                *v = 1.0 - rough;
                c
            } else {
                ui.slider(&format!("mat.slotval{}", s), vr, v, 0.0, max, 2)
            }
        } else {
            false
        };
        if changed && (s == 1) {
            m.dirty_tex = true;
        }
        let tr = Rect::new(r.x + 6.0, r.y + 44.0, r.w - 12.0, thumb);
        ui.record(&format!("matslot:{}", SLOT_NAMES[s]), tr);
        checker(tr, 10.0);
        let m = &self.materials[c];
        let pic = m.slots[s].tex.clone().or_else(|| if s == 0 && m.source != 0 { m.tex.clone() } else { None });
        if let Some(t) = pic {
            let side = tr.w.min(tr.h);
            draw_texture_ex(&t, tr.x + (tr.w - side) * 0.5, tr.y + (tr.h - side) * 0.5, WHITE, DrawTextureParams { dest_size: Some(vec2(side, side)), ..Default::default() });
        }
        draw_rectangle_lines(tr.x, tr.y, tr.w, tr.h, 1.0, C_LINE);
        let cb = Rect::new(tr.x + 3.0, tr.y + 3.0, 16.0, 16.0);
        let mut on = m.slots[s].enabled;
        if ui.checkbox(&format!("mat.sloton{}", s), cb, &mut on) {
            let m = &mut self.materials[c];
            m.slots[s].enabled = on;
            if s == 0 {
                m.dirty_base = true;
            } else if s == 1 {
                m.dirty_tex = true;
            }
            self.dirty = true;
        }
        if ui.hover(tr) && !ui.hover(cb) {
            let file = &self.materials[c].slots[s].file;
            ui.tooltip = Some(if file.is_empty() {
                format!("Click to load a {} texture.", SLOT_NAMES[s])
            } else {
                format!("{}\nClick to change, right-click to clear.", file)
            });
            if ui.inp.l_pressed && ui.popup.is_none() && !ui.blocked {
                if let Some(f) = crate::props::pick_image() {
                    self.load_slot(c, s, Some(f));
                }
            } else if ui.inp.r_pressed {
                self.load_slot(c, s, None);
            }
        }
    }

    /// Put a picture in a slot (None clears it). The Color slot drives the material's picture,
    /// Opacity its alpha and Normal the shader's normal map; the others are kept with the material.
    fn load_slot(&mut self, c: usize, s: usize, file: Option<String>) {
        let m = &mut self.materials[c];
        match file {
            Some(f) => match textures::load_image_file(&f) {
                Ok(img) => {
                    let t = Texture2D::from_rgba8(img.w as u16, img.h as u16, &img.px);
                    t.set_filter(FilterMode::Linear);
                    crate::textures::set_repeat(&t);
                    m.slots[s].tex = Some(t);
                    m.slots[s].image = Some(img);
                    m.slots[s].file = f.clone();
                    if s == 0 {
                        m.source = 4;
                        m.file = f;
                        m.slots[0].tex = None; // the adjusted picture is shown instead
                    }
                }
                Err(e) => {
                    self.set_status(&format!("Couldn't load the picture: {}", e));
                    return;
                }
            },
            None => {
                m.slots[s] = MatSlot { enabled: m.slots[s].enabled, ..Default::default() };
                if s == 0 {
                    m.source = 0;
                    m.file.clear();
                }
            }
        }
        let m = &mut self.materials[c];
        if s == 0 {
            m.dirty_base = true;
        }
        if s == 1 {
            m.dirty_tex = true;
        }
        textures::refresh_material(m);
        self.dirty = true;
    }
}

impl Material {
    /// A cheap fingerprint of the values edited in this panel, to know when the model needs
    /// rebuilding.
    fn clone_settings_key(&self) -> (bool, [i32; 4], usize, usize, usize) {
        let q = |v: f32| (v * 1000.0) as i32;
        (self.two_sided, [q(self.variation), q(self.atlas_scale), q(self.opacity), q(self.normal_strength)], self.atlas_mode, self.max_res, self.unwrap)
    }
}

/// A colour swatch; clicking it opens three sliders (red, green, blue) in its place.
fn color_field(ui: &mut Ui, open: &mut Option<(usize, usize)>, key: (usize, usize), r: Rect, c: &mut [f32; 3]) -> bool {
    let is_open = *open == Some(key);
    let sw = if is_open { Rect::new(r.x, r.y, 18.0, r.h) } else { r };
    draw_rectangle(sw.x, sw.y, sw.w, sw.h, Color::new(c[0], c[1], c[2], 1.0));
    draw_rectangle_lines(sw.x, sw.y, sw.w, sw.h, 1.0, C_LINE);
    let k = format!("color.{}.{}", key.0, key.1);
    ui.record(&k, sw);
    if ui.hover(sw) {
        ui.tooltip = Some(if is_open { "Click to close".into() } else { "Click to edit the colour".into() });
    }
    if ui.clicked(sw) {
        *open = if is_open { None } else { Some(key) };
    }
    let mut changed = false;
    if is_open {
        let w = (r.w - 22.0) / 3.0;
        for i in 0..3 {
            let rr = Rect::new(r.x + 22.0 + i as f32 * w, r.y, w - 2.0, r.h);
            changed |= ui.slider(&format!("{}.{}", k, i), rr, &mut c[i], 0.0, 1.0, 2);
        }
    }
    changed
}

/// A one-line text box: click to type, Enter or a click elsewhere to finish.
pub(crate) fn text_field(ui: &mut Ui, key: &str, r: Rect, s: &mut String, focus: &mut bool) {
    ui.record(key, r);
    draw_rectangle(r.x, r.y, r.w, r.h, C_FIELD);
    if ui.inp.l_pressed {
        *focus = ui.hover(r);
    }
    if *focus {
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, C_ACCENT2);
        for ch in &ui.inp.chars {
            if !ch.is_control() {
                s.push(*ch);
            }
        }
        if ui.inp.key(KeyCode::Backspace) {
            s.pop();
        }
        if ui.inp.key(KeyCode::Enter) || ui.inp.key(KeyCode::Escape) {
            *focus = false;
        }
    }
    let shown = if *focus && (get_time() * 2.0) as i64 % 2 == 0 { format!("{}|", s) } else { s.clone() };
    ui.text_fit(&shown, r.x + 4.0, r.y + 3.0, r.w - 8.0, 12.0, C_TEXT);
}
