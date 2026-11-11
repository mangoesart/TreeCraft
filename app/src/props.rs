//! The Properties panel: rows of labelled controls, grouped like the original's property editor.
use crate::app::{App, Sel};
use crate::model::*;
use crate::model::Material;
use crate::render::VIEWS;
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
}

const RH: f32 = 24.0;

impl<'a> Rows<'a> {
    fn lw(&self) -> f32 {
        (self.w * 0.46).floor()
    }
    fn key(&self, k: &str) -> String {
        format!("{}.{}", self.prefix, k)
    }
    pub fn group(&mut self, title: &str) -> bool {
        let key = self.key(&format!("grp.{}", title));
        let r = Rect::new(self.x - 6.0, self.y + 4.0, self.w + 12.0, 24.0);
        self.ui.record(&format!("grp:{}", title), r);
        let open = !self.closed.contains(&key);
        let hot = self.ui.hover(r);
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
                self.closed.insert(key);
            } else {
                self.closed.remove(&key);
            }
        }
        self.y += 32.0;
        self.open = open;
        open
    }
    fn label(&mut self, label: &str) -> Rect {
        let lw = self.lw();
        self.ui.text_fit(label, self.x + 6.0, self.y + 5.0, lw - 10.0, 12.5, C_TEXT_DIM);
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
    pub fn note(&mut self, text: &str) {
        self.ui.text_fit(text, self.x + 6.0, self.y + 4.0, self.w - 8.0, 12.0, C_TEXT_FAINT);
        self.y += 20.0;
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

fn seed_rows(r: &mut Rows, p: &mut Params) {
    r.i("seed", "Generation seed", &mut p.seed, 0, 9999);
    if r.button("rand", "Randomize") {
        p.seed = (macroquad::rand::gen_range(0, 9999)) as i32;
        r.changed = true;
    }
}

pub fn node_props(r: &mut Rows, n: &mut Node, on_ground: bool, mat_names: &[String], mesh_names: &[String], stats: (usize, usize, usize, usize), count: usize, max_anchor: usize) {
    let kind = n.kind;
    let p = &mut n.p;
    match kind {
        Kind::Tree => {
            if r.group("Shape") {
                r.f("tree_radius", "Radius (trunk spread)", &mut p.tree_radius, 0.0, 60.0, 1);
            }
            if r.group("Level of Detail") {
                r.i("lod_preview", "Preview leaf LOD", &mut p.lod_preview, 0, 3);
            }
            if r.group("Statistics") {
                r.info("Triangles", &stats.0.to_string());
                r.info("Vertices", &stats.1.to_string());
                r.info("Branches", &stats.2.to_string());
                r.info("Leaves", &stats.3.to_string());
            }
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
    if r.group("Gloss / Specular") {
        r.f("gloss", "Gloss", &mut m.gloss, 0.0, 1.0, 2);
        r.color("specular", "Specular", &mut m.specular_color);
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
    rfd::FileDialog::new()
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
            Some(n) => (n.name.clone(), n.kind.type_label().to_string()),
            None => ("Nothing selected".into(), String::new()),
        },
        Sel::Material(i) => (app.materials.get(i).map(|m| m.name.clone()).unwrap_or_default(), "Material".into()),
        Sel::Mesh(i) => (app.meshes.get(i).map(|m| m.name.clone()).unwrap_or_default(), "Mesh".into()),
        Sel::Window => ("Window properties".into(), "Viewport and lighting".into()),
    };
    let bw = 62.0;
    let bx = x + w - bw * 3.0;
    ui.text_fit(&title, x, y + 2.0, bx - x - 8.0, 15.0, WHITE);
    ui.record(&format!("hdr:{}", title), Rect::new(x, y, bx - x - 8.0, 20.0));
    if !sub.is_empty() {
        ui.text_fit(&sub, x, y + 21.0, bx - x - 8.0, 11.5, C_TEXT_DIM);
    }
    let is_node = matches!(app.sel, Sel::Node(_));
    draw_rectangle(bx - 2.0, y - 2.0, bw * 3.0 + 4.0, 26.0, C_BG);
    ui.button_ex("mode.gen", Rect::new(bx, y, bw, 22.0), "Generator", is_node, true);
    ui.button_ex("mode.node", Rect::new(bx + bw, y, bw, 22.0), "Node", false, false);
    ui.button_ex("mode.free", Rect::new(bx + bw * 2.0, y, bw, 22.0), "Freehand", false, false);
    y += 44.0;
    draw_line(panel.x + 8.0, y - 6.0, panel.x + panel.w - 8.0, y - 6.0, 1.0, C_LINE);

    let body = Rect::new(panel.x, y, panel.w, panel.y + panel.h - y);
    if ui.hover_any(body) && ui.inp.wheel != 0.0 && ui.popup.is_none() {
        app.prop_scroll = (app.prop_scroll - ui.inp.wheel * 60.0).max(0.0);
    }
    ui.set_clip(Some(body));
    let start_y = y - app.prop_scroll;

    let mat_names: Vec<String> = app.materials.iter().map(|m| m.name.clone()).collect();
    let mnames = mesh_names(&app.meshes);
    let mut closed = std::mem::take(&mut app.closed_groups);
    let mut rows = Rows { ui, x, y: start_y, w, changed: false, prefix: String::new(), closed: &mut closed, open: true };
    match app.sel {
        Sel::None => {
            rows.note("");
            rows.y -= 20.0;
            let mut act = None;
            for (sec, items) in [
                ("Edit", vec![("Global properties", 1), ("Window properties", 2)]),
                ("Actions", vec![("Randomize", 3), ("Frame tree", 4)]),
                ("File", vec![("New tree", 5)]),
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
            rows.ui.text("Help", x, rows.y + 4.0, 14.0, C_TEXT);
            rows.y += 28.0;
            for line in [
                "Generation tab: Add a Trunk, Branch or Leaf to the",
                "selected node; click a node to edit it here.",
                "Shift+drag from one node onto another to re-link it.",
                "Viewport: left-drag orbit, middle/shift-drag pan,",
                "right-drag or wheel zoom, F frame the tree.",
                "Keys 0 perspective, 1 front, 3 right, 7 top, 5 ortho.",
            ] {
                rows.note(line);
            }
            match act {
                Some(1) => {
                    let root = app.root_id();
                    app.select(Sel::Node(root));
                }
                Some(2) => app.select(Sel::Window),
                Some(3) => app.randomize(None),
                Some(4) => app.frame_tree(),
                Some(5) => app.new_tree(),
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
                node_props(&mut rows, &mut n, on_ground, &mat_names, &mnames, stats, count, max_anchor);
                if rows.changed {
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
        Sel::Window => {
            rows.prefix = "win".into();
            if rows.group("Camera") {
                let views: Vec<String> = VIEWS.iter().map(|s| s.to_string()).collect();
                let mut v = app.cam.view;
                rows.combo("view", "View", &views, &mut v);
                if v != app.cam.view {
                    app.cam.set_view(v);
                }
                rows.b("ortho", "Orthographic", &mut app.cam.ortho);
                rows.f("fov", "Field of view", &mut app.cam.fov, 10.0, 100.0, 0);
            }
            if rows.group("Display") {
                rows.b("grid", "Show grid", &mut app.show_grid);
                let before = app.highlight;
                rows.b("hl", "Highlight selected generator", &mut app.highlight);
                if before != app.highlight {
                    app.dirty = true;
                }
                rows.b("unlit", "Unlit (texture colours only)", &mut app.light.unlit);
            }
            if rows.group("Lighting") {
                rows.f("az", "Sun direction", &mut app.light.azimuth, -180.0, 180.0, 0);
                rows.f("el", "Sun height", &mut app.light.elevation, 5.0, 90.0, 0);
            }
        }
    }
    let content_h = rows.y - start_y;
    drop(rows);
    app.closed_groups = closed;
    let max_scroll = (content_h - body.h + 20.0).max(0.0);
    app.prop_scroll = app.prop_scroll.min(max_scroll);
    ui.set_clip(None);
    if max_scroll > 0.0 {
        let frac = body.h / (content_h + 20.0);
        let th = (body.h * frac).max(24.0);
        let ty = body.y + (body.h - th) * (app.prop_scroll / max_scroll);
        draw_rectangle(panel.x + panel.w - 5.0, ty, 3.0, th, Color::new(1.0, 1.0, 1.0, 0.2));
    }
}
