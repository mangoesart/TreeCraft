//! Application state and the dock panels (Generation graph, Materials, Meshes) and viewport.
use crate::gen::{self, GenCtx};
use crate::meshio;
use crate::model::*;
use crate::model::Material;
use crate::render::{self, Cam, GpuTree, Lighting, VIEWS};
use crate::textures;
use crate::ui::*;
use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sel {
    None,
    Node(u32),
    Material(usize),
    Mesh(usize),
    Window,
}

pub const TABS: [&str; 3] = ["Generation", "Materials", "Meshes"];

pub struct Icons {
    pub tree: Texture2D,
    pub branch: Texture2D,
    pub leaf: Texture2D,
    pub ring: Texture2D,
}

impl Icons {
    pub fn load() -> Self {
        let l = |b: &[u8]| {
            let t = Texture2D::from_file_with_format(b, Some(ImageFormat::Png));
            t.set_filter(FilterMode::Linear);
            t
        };
        Icons {
            tree: l(include_bytes!("../assets/icons/generator_tree.png")),
            branch: l(include_bytes!("../assets/icons/generator_branch.png")),
            leaf: l(include_bytes!("../assets/icons/generator_leaf.png")),
            ring: l(include_bytes!("../assets/icons/generator_border.png")),
        }
    }
    fn for_kind(&self, k: Kind) -> &Texture2D {
        match k {
            Kind::Tree => &self.tree,
            Kind::Trunk | Kind::Branch => &self.branch,
            Kind::Leaf => &self.leaf,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum GraphDrag {
    None,
    Node { id: u32, start: Vec2, moved: bool },
    Link { from: u32 },
    Pan,
}

#[derive(Clone, Copy, PartialEq)]
enum VpDrag {
    None,
    Orbit,
    Pan,
    Zoom,
}

pub struct App {
    pub nodes: Vec<Node>,
    pub next_id: u32,
    pub materials: Vec<Material>,
    pub meshes: Vec<LeafMesh>,
    pub sel: Sel,
    pub tab: usize,
    pub cam: Cam,
    pub light: Lighting,
    pub show_grid: bool,
    pub highlight: bool,
    pub dirty: bool,
    pub gpu: Option<GpuTree>,
    pub geo_stats: (usize, usize, usize, usize),
    pub per_node: HashMap<u32, usize>,
    pub bounds: (Vec3, Vec3),
    pub prop_scroll: f32,
    pub closed_groups: HashSet<String>,
    pub graph_off: Vec2,
    pub graph_zoom: f32,
    graph_drag: GraphDrag,
    vp_drag: VpDrag,
    last_mouse: Vec2,
    pub mesh_cur: usize,
    pub mesh_lod: usize,
    mesh_drag: Option<i32>,
    pub mat_cur: usize,
    /// Material preview shape: index into PREVIEW_SHAPES.
    pub preview_shape: usize,
    pub mat_yaw: f32,
    pub mat_pitch: f32,
    pub mat_orbit: Option<(Vec2, bool)>,
    /// Meshes tab preview: false = wireframe (default), true = with its material.
    pub mesh_show_material: bool,
    /// Material preview zoom offset (fn 0x1402ece70: wheel delta x -0.001, reset to 0).
    pub mat_zoom: f32,
    /// Mesh preview right-click menu (fn 0x1402ecb70): Lit and Scribed.
    pub mesh_lit: bool,
    pub mesh_scribed: bool,
    pub preview_rt: Option<RenderTarget>,
    pub status: String,
    pub status_t: f64,
    pub gm: Option<macroquad::material::Material>,
    pub icons: Icons,
    pub viewport: Rect,
    pub regen_count: usize,
    graph_mouse: Vec2,
    pub preview_aspect: f32,
    pub hand_drag: bool,
    pub mesh_dialog: Option<crate::meshdialog::MeshDialog>,
    pub mat_dialog: Option<crate::meshdialog::MaterialDialog>,
    pub mat_subtab: usize,
    pub mat_scroll: f32,
    pub mat_clip: Option<Material>,
    pub color_edit: Option<(usize, usize)>,
    pub last_node: Option<u32>,
    pub mat_drag: bool,
    pub mat_text_focus: bool,
    pub mesh_clip: Option<LeafMesh>,
    pub mesh_editing: bool,
    /// The Edit Cutout window while it's open.
    pub cutout: Option<crate::cutout::CutoutEditor>,
    /// Where the Edit Cutout window was last placed.
    pub cutout_rect: Option<Rect>,
    /// widths the person dragged the Properties panel and the Generation/Materials/Meshes panel to
    /// (None = automatic), and which splitter is being dragged (1 = left, 2 = right)
    pub left_w: Option<f32>,
    pub dock_w: Option<f32>,
    pub split_drag: u8,
    pub mesh_yaw: f32,
    pub mesh_pitch: f32,
    pub mesh_zoom: f32,
    mesh_orbit: bool,
    pub mesh_rt: Option<RenderTarget>,
    mesh_preview_aspect: f32,
    pub show_alignment: bool,
    graph_mouse_prev_mesh: Vec2,
    sphere: Vec<Mesh>,
    card: Vec<Mesh>,
    cylinder: Vec<Mesh>,
    custom_preview: Vec<Mesh>,
    pub custom_preview_name: String,
}

impl App {
    pub fn new() -> Self {
        let gm = match render::make_material() {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("shader error: {}", e);
                None
            }
        };
        let mk = |v: Vec<(Vec<macroquad::models::Vertex>, Vec<u16>)>| v.into_iter().map(|(a, b)| Mesh { vertices: a, indices: b, texture: None }).collect::<Vec<_>>();
        let mut app = App {
            nodes: vec![],
            next_id: 1,
            materials: vec![],
            meshes: vec![],
            sel: Sel::None,
            tab: 0,
            cam: Cam::new(),
            light: Lighting { azimuth: 35.0, elevation: 50.0, unlit: false },
            show_grid: true,
            highlight: true,
            dirty: true,
            gpu: None,
            geo_stats: (0, 0, 0, 0),
            per_node: HashMap::new(),
            bounds: (Vec3::ZERO, Vec3::ONE),
            prop_scroll: 0.0,
            closed_groups: HashSet::new(),
            graph_off: Vec2::ZERO,
            graph_zoom: 1.0,
            graph_drag: GraphDrag::None,
            vp_drag: VpDrag::None,
            last_mouse: Vec2::ZERO,
            mesh_cur: 0,
            mesh_lod: 0,
            mesh_drag: None,
            mat_cur: 0,
            preview_shape: 0,
            mat_yaw: 0.6,
            mat_pitch: 0.25,
            mat_orbit: None,
            mesh_show_material: false,
            mat_zoom: 0.0,
            mesh_lit: true,
            mesh_scribed: false,
            preview_rt: None,
            status: String::new(),
            status_t: 0.0,
            gm,
            icons: Icons::load(),
            viewport: Rect::new(0.0, 0.0, 1.0, 1.0),
            regen_count: 0,
            graph_mouse: Vec2::ZERO,
            preview_aspect: 1.0,
            hand_drag: false,
            mesh_dialog: None,
            mat_dialog: None,
            mat_subtab: 0,
            mat_scroll: 0.0,
            mat_clip: None,
            color_edit: None,
            last_node: None,
            mat_drag: false,
            mat_text_focus: false,
            mesh_clip: None,
            mesh_editing: false,
            cutout: None,
            cutout_rect: None,
            left_w: None,
            dock_w: None,
            split_drag: 0,
            mesh_yaw: 0.0,
            mesh_pitch: 0.0,
            mesh_zoom: 1.0,
            mesh_orbit: false,
            mesh_rt: None,
            mesh_preview_aspect: 1.0,
            show_alignment: false,
            graph_mouse_prev_mesh: Vec2::ZERO,
            sphere: mk(render::sphere_mesh(1.0)),
            card: mk(render::card_mesh(1.0)),
            cylinder: mk(render::cylinder_mesh(0.75, 1.9)),
            custom_preview: vec![],
            custom_preview_name: String::new(),
        };
        app.new_tree();
        // the starting tree comes with a bark and a leaf material and a leaf mesh; File > New tree
        // starts from a clean slate
        app.reset_library();
        // starting graph: one of each building block, all on their default settings
        let root = app.root_id();
        let trunk = app.add_node(Kind::Trunk, root);
        let br = app.add_node(Kind::Branch, trunk);
        app.add_node(Kind::Leaf, br);
        app.select(Sel::None);
        app.regenerate();
        app.frame_tree();
        app
    }

    fn reset_library(&mut self) {
        self.materials = vec![Material::bark(), Material::leaf()];
        for m in &mut self.materials {
            textures::refresh_material(m);
        }
        let mut cut = LeafMesh::card("Leaf cutout");
        if let Some(img) = &self.materials[1].adjusted {
            cut.lods[0].outline = textures::auto_cutout(img, cut.auto_points as usize, cut.threshold, cut.margin);
            cut.lods[1].outline = textures::reduce_outline(cut.lods[0].outline.clone(), 6);
            cut.lods[2].outline = textures::reduce_outline(cut.lods[0].outline.clone(), 4);
        }
        self.meshes = vec![cut];
    }

    pub fn set_status(&mut self, s: &str) {
        self.status = s.to_string();
        self.status_t = get_time();
    }

    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn root_id(&self) -> u32 {
        self.nodes.iter().find(|n| n.kind == Kind::Tree).map(|n| n.id).unwrap_or(0)
    }
    pub fn instance_count(&self, id: u32) -> usize {
        *self.per_node.get(&id).unwrap_or(&0)
    }

    pub fn new_tree(&mut self) {
        self.nodes.clear();
        self.next_id = 1;
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(Node { id, kind: Kind::Tree, name: "Tree".into(), parent: None, pos: vec2(0.0, 0.0), p: Params::new(Kind::Tree), hidden: false });
        self.sel = Sel::None;
        self.dirty = true;
        self.graph_off = Vec2::ZERO;
        // a new tree starts from a clean slate: no materials, textures or meshes
        self.materials.clear();
        self.meshes.clear();
        self.mat_cur = 0;
        self.mesh_cur = 0;
        self.mesh_lod = 0;
        self.mesh_editing = false;
        self.cutout = None;
        self.mesh_dialog = None;
        self.mat_dialog = None;
        self.color_edit = None;
        self.mat_scroll = 0.0;
        self.last_node = None;
    }

    pub fn select(&mut self, s: Sel) {
        if self.sel != s {
            self.sel = s;
            self.prop_scroll = 0.0;
            self.dirty = true;
        }
        if let Sel::Material(i) = s {
            self.mat_cur = i;
        }
        if let Sel::Mesh(i) = s {
            self.mesh_cur = i;
        }
        if let Sel::Node(id) = s {
            self.last_node = Some(id);
        }
    }

    pub fn add_node(&mut self, kind: Kind, parent: u32) -> u32 {
        let parent = match self.node(parent) {
            // under a Leaf generator only Leaf generators fit (they spawn on its leaves' anchors)
            Some(p) if p.kind.can_parent() && !(p.kind == Kind::Leaf && kind != Kind::Leaf) => parent,
            _ => self.root_id(),
        };
        let pp = self.node(parent).map(|n| n.pos).unwrap_or(Vec2::ZERO);
        let siblings = self.nodes.iter().filter(|n| n.parent == Some(parent)).count();
        let id = self.next_id;
        self.next_id += 1;
        let same = self.nodes.iter().filter(|n| n.kind == kind).count();
        let name = if same == 0 { kind.label().to_string() } else { format!("{} {}", kind.label(), same + 1) };
        let mut p = Params::new(kind);
        p.seed += id as i32 * 13;
        if kind == Kind::Leaf && self.node(parent).map(|n| n.kind == Kind::Leaf).unwrap_or(false) {
            p.start_angle = 0.0; // along each anchor's line
            p.first = 0.0; // First/Last pick anchors by height (fn 0x14026f500): keep them all
            p.last = 1.0;
        }
        let x = pp.x + if siblings == 0 { 0.0 } else { (siblings as f32 + 1.0) / 2.0 * 90.0 * if siblings % 2 == 1 { 1.0 } else { -1.0 } };
        self.nodes.push(Node { id, kind, name, parent: Some(parent), pos: vec2(x, pp.y - 100.0), p, hidden: false });
        self.select(Sel::Node(id));
        self.dirty = true;
        id
    }

    fn descendants(&self, id: u32) -> Vec<u32> {
        let mut out = vec![id];
        let mut i = 0;
        while i < out.len() {
            let cur = out[i];
            for n in &self.nodes {
                if n.parent == Some(cur) {
                    out.push(n.id);
                }
            }
            i += 1;
        }
        out
    }

    pub fn delete_node(&mut self, id: u32) {
        if self.node(id).map(|n| n.kind == Kind::Tree).unwrap_or(true) {
            self.set_status("The Tree node can't be deleted");
            return;
        }
        let gone = self.descendants(id);
        self.nodes.retain(|n| !gone.contains(&n.id));
        self.select(Sel::None);
        self.dirty = true;
    }

    pub fn duplicate_node(&mut self, id: u32) {
        let src = match self.node(id) {
            Some(n) if n.kind != Kind::Tree => n.clone(),
            _ => return,
        };
        let nid = self.next_id;
        self.next_id += 1;
        let mut n = src.clone();
        n.id = nid;
        n.name = format!("{} copy", src.name);
        n.pos += vec2(80.0, 0.0);
        n.p.seed += 97;
        self.nodes.push(n);
        self.select(Sel::Node(nid));
        self.dirty = true;
    }

    pub fn randomize(&mut self, id: Option<u32>) {
        for n in &mut self.nodes {
            if id.map(|i| i == n.id).unwrap_or(true) {
                n.p.seed = macroquad::rand::gen_range(0, 9999);
            }
        }
        self.dirty = true;
    }

    pub fn arrange(&mut self) {
        let root = self.root_id();
        let mut x_cursor = 0.0f32;
        fn place(app: &mut App, id: u32, depth: i32, x: &mut f32) -> f32 {
            let kids: Vec<u32> = app.nodes.iter().filter(|n| n.parent == Some(id)).map(|n| n.id).collect();
            let px = if kids.is_empty() {
                let v = *x;
                *x += 90.0;
                v
            } else {
                let xs: Vec<f32> = kids.iter().map(|k| place(app, *k, depth + 1, x)).collect();
                (xs[0] + xs[xs.len() - 1]) * 0.5
            };
            if let Some(n) = app.nodes.iter_mut().find(|n| n.id == id) {
                n.pos = vec2(px, -(depth as f32) * 100.0);
            }
            px
        }
        let rx = place(self, root, 0, &mut x_cursor);
        for n in &mut self.nodes {
            n.pos.x -= rx;
        }
        self.graph_off = Vec2::ZERO;
    }

    fn reparent(&mut self, child: u32, parent: u32) {
        if child == parent {
            return;
        }
        let ck = self.node(child).map(|c| c.kind);
        let ok_parent = self.node(parent).map(|p| p.kind.can_parent() && !(p.kind == Kind::Leaf && ck != Some(Kind::Leaf))).unwrap_or(false);
        if !ok_parent || ck == Some(Kind::Tree) || self.descendants(child).contains(&parent) {
            self.set_status("Can't link those two nodes");
            return;
        }
        if let Some(c) = self.nodes.iter_mut().find(|n| n.id == child) {
            c.parent = Some(parent);
        }
        self.dirty = true;
        self.set_status("Linked");
    }

    pub fn regenerate(&mut self) {
        for m in &mut self.materials {
            textures::refresh_material(m);
        }
        let selected = if let Sel::Node(id) = self.sel { Some(id) } else { None };
        let ctx = GenCtx { nodes: &self.nodes, materials: &self.materials, meshes: &self.meshes, selected, highlight: self.highlight };
        let geo = gen::generate(&ctx);
        self.geo_stats = (geo.tris, geo.verts, geo.spines, geo.leaves);
        if let Some(w) = geo.warnings.first() {
            let w = w.clone();
            self.set_status(&w);
        }
        self.per_node = geo.per_node.clone();
        self.bounds = (geo.bmin, geo.bmax);
        self.gpu = Some(render::upload(&geo));
        self.dirty = false;
        self.regen_count += 1;
        // collection meshes are rebuilt from the generators that contribute to them; when one
        // changes, compute once more so leaves using it see the new geometry
        let mut changed = false;
        for (mi, g) in geo.collections {
            if let Some(m) = self.meshes.get_mut(mi) {
                let same = m
                    .collected
                    .as_ref()
                    .map(|c| c.pos.len() == g.pos.len() && c.pos.first() == g.pos.first() && c.pos.last() == g.pos.last())
                    .unwrap_or(false);
                if !same {
                    m.collected = Some(g);
                    changed = true;
                }
            }
        }
        if changed && self.regen_count % 50 != 0 {
            self.dirty = true;
        }
    }

    pub fn frame_tree(&mut self) {
        let (a, b) = self.bounds;
        let b2 = vec3(b.x, b.y.max(a.y + 1.0), b.z);
        self.cam.frame(vec3(a.x, a.y.min(0.0), a.z), b2);
    }

    // ------------------------------------------------------------------ viewport
    pub fn viewport_input(&mut self, ui: &mut Ui, r: Rect) {
        let m = ui.mouse();
        let over = r.contains(m) && !ui.blocked && ui.popup.is_none();
        let inp = ui.inp.clone();
        if over && inp.wheel != 0.0 {
            self.cam.zoom(inp.wheel);
        }
        if over && ui.active == 0 && self.vp_drag == VpDrag::None {
            if inp.l_pressed {
                self.vp_drag = if inp.shift || inp.alt && inp.ctrl { VpDrag::Pan } else { VpDrag::Orbit };
                self.last_mouse = m;
                ui.active = hid("viewport");
            } else if inp.m_pressed {
                self.vp_drag = VpDrag::Pan;
                self.last_mouse = m;
            } else if inp.r_pressed {
                self.vp_drag = VpDrag::Zoom;
                self.last_mouse = m;
            }
        }
        let d = m - self.last_mouse;
        match self.vp_drag {
            VpDrag::Orbit => {
                if inp.l_down {
                    self.cam.orbit(d.x, d.y);
                } else {
                    self.vp_drag = VpDrag::None;
                }
            }
            VpDrag::Pan => {
                if inp.l_down || inp.m_down {
                    self.cam.pan(-d.x, d.y, r.h);
                } else {
                    self.vp_drag = VpDrag::None;
                }
            }
            VpDrag::Zoom => {
                if inp.r_down {
                    self.cam.zoom(-d.y * 0.03);
                } else {
                    self.vp_drag = VpDrag::None;
                }
            }
            VpDrag::None => {}
        }
        self.last_mouse = m;
    }

    pub fn draw_viewport_bg(&self, r: Rect) {
        let top = Color::new(0.26, 0.27, 0.29, 1.0);
        let bot = Color::new(0.14, 0.14, 0.15, 1.0);
        let steps = 24;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let c = Color::new(top.r + (bot.r - top.r) * t, top.g + (bot.g - top.g) * t, top.b + (bot.b - top.b) * t, 1.0);
            draw_rectangle(r.x, r.y + r.h * t, r.w, r.h / steps as f32 + 1.0, c);
        }
    }

    /// `r` in UI units, `scale` physical pixels per unit, `sh` framebuffer height in pixels.
    pub fn draw_viewport_3d(&mut self, r: Rect, scale: f32, sh: f32) {
        let vp = ((r.x * scale) as i32, (sh - (r.y + r.h) * scale) as i32, (r.w * scale) as i32, (r.h * scale) as i32);
        let cam3 = self.cam.camera3d(Some(vp), r.w / r.h.max(1.0), None);
        set_camera(&cam3);
        if self.show_grid {
            render::draw_grid(50.0, 2.5);
        }
        if let (Some(gpu), Some(gm)) = (&mut self.gpu, &self.gm) {
            for (mi, meshes) in &mut gpu.parts {
                if let Some(m) = self.materials.get(*mi) {
                    for mesh in meshes.iter_mut() {
                        mesh.texture = m.tex.clone();
                    }
                }
            }
            draw_gpu(gpu, &self.materials, gm, &self.light, self.cam.position());
        }
        set_default_camera();
    }

    pub fn draw_viewport_overlay(&mut self, ui: &mut Ui, r: Rect) {
        let views: Vec<String> = VIEWS.iter().map(|s| s.to_string()).collect();
        let mut v = self.cam.view;
        if ui.combo("vp.view", Rect::new(r.x + 8.0, r.y + 8.0, 120.0, 22.0), &views, &mut v) {
            self.cam.set_view(v);
        }
        let ob = Rect::new(r.x + 132.0, r.y + 8.0, 92.0, 22.0);
        if ui.button_ex("vp.ortho", ob, "Orthographic", self.cam.ortho, true) {
            self.cam.ortho = !self.cam.ortho;
        }
        if ui.button("vp.frame", Rect::new(r.x + 228.0, r.y + 8.0, 54.0, 22.0), "Frame") {
            self.frame_tree();
        }
        if ui.button_ex("vp.grid", Rect::new(r.x + 286.0, r.y + 8.0, 44.0, 22.0), "Grid", self.show_grid, true) {
            self.show_grid = !self.show_grid;
        }
        if ui.button("vp.light", Rect::new(r.x + 334.0, r.y + 8.0, 70.0, 22.0), "Lighting...") {
            self.select(Sel::Window);
        }
        let name = self.cam.view_name();
        ui.text(&name, r.x + 10.0, r.y + 36.0, 12.0, C_TEXT_DIM);
        let s = format!(
            "Triangles: {}   Vertices: {}   Branches: {}   Leaves: {}",
            self.geo_stats.0, self.geo_stats.1, self.geo_stats.2, self.geo_stats.3
        );
        ui.record("stat", Rect::new(r.x + 10.0, r.y + r.h - 22.0, 300.0, 16.0));
        ui.text(&s, r.x + 10.0, r.y + r.h - 22.0, 12.0, C_TEXT_DIM);
        if !self.status.is_empty() && get_time() - self.status_t < 4.0 {
            let w = ui.text_w(&self.status, 12.5) + 20.0;
            let br = Rect::new(r.x + (r.w - w) * 0.5, r.y + r.h - 46.0, w, 24.0);
            draw_rectangle(br.x, br.y, br.w, br.h, Color::new(0.0, 0.0, 0.0, 0.55));
            let st = self.status.clone();
            ui.text_center(&st, br, 12.5, WHITE);
        }
        // axis gizmo
        let c = vec2(r.x + r.w - 40.0, r.y + r.h - 40.0);
        let d = self.cam.dir();
        let right = Vec3::Y.cross(d).normalize_or_zero();
        let up = d.cross(right).normalize_or_zero();
        for (ax, col, l) in [(Vec3::X, Color::new(0.9, 0.35, 0.35, 1.0), "X"), (Vec3::Y, Color::new(0.4, 0.85, 0.4, 1.0), "Y"), (Vec3::Z, Color::new(0.4, 0.55, 1.0, 1.0), "Z")] {
            let p = vec2(-ax.dot(right), -ax.dot(up)) * 22.0;
            let p = vec2(-p.x, p.y);
            draw_line(c.x, c.y, c.x + p.x, c.y + p.y, 2.0, col);
            ui.text(l, c.x + p.x * 1.25 - 3.0, c.y + p.y * 1.25 - 6.0, 11.0, col);
        }
    }

    // ------------------------------------------------------------------ dock
    pub fn draw_dock(&mut self, ui: &mut Ui, r: Rect) {
        draw_rectangle(r.x, r.y, r.w, r.h, C_PANEL);
        draw_rectangle(r.x, r.y, r.w, 26.0, C_PANEL2);
        let mut x = r.x;
        for (i, t) in TABS.iter().enumerate() {
            let w = ui.text_w(t, 13.0) + 20.0;
            let tr = Rect::new(x, r.y, w, 26.0);
            ui.record(&format!("tab:{}", t), tr);
            if i == self.tab {
                draw_rectangle(tr.x, tr.y, tr.w, tr.h, C_PANEL);
            }
            let hot = ui.hover(tr);
            ui.text_center(t, tr, 13.0, if i == self.tab || hot { WHITE } else { C_TEXT_DIM });
            if hot && ui.inp.l_pressed {
                self.tab = i;
            }
            x += w;
        }
        let tb = Rect::new(r.x, r.y + 26.0, r.w, 30.0);
        let body = Rect::new(r.x, r.y + 56.0, r.w, r.h - 56.0);
        match self.tab {
            0 => self.draw_generation(ui, tb, body),
            1 => self.draw_materials(ui, tb, body),
            _ => self.draw_meshes(ui, tb, body),
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, tb: Rect, key: &str, add_items: &[&str], act_items: &[&str], randomize: bool) -> (Option<usize>, Option<usize>, bool) {
        let addk = format!("{}.add", key);
        let actk = format!("{}.actions", key);
        let ar = Rect::new(tb.x + 6.0, tb.y + 4.0, 52.0, 22.0);
        if ui.tool_button(&addk, ar, "+ Add", ui.popup_open_for(&addk)) {
            ui.open_popup(hid(&addk), ar, add_items.iter().map(|s| s.to_string()).collect(), None);
        }
        let cr = Rect::new(tb.x + 62.0, tb.y + 4.0, 68.0, 22.0);
        if ui.tool_button(&actk, cr, "Actions", ui.popup_open_for(&actk)) {
            ui.open_popup(hid(&actk), cr, act_items.iter().map(|s| s.to_string()).collect(), None);
        }
        let mut rnd = false;
        if randomize {
            rnd = ui.tool_button(&format!("{}.rand", key), Rect::new(tb.x + 134.0, tb.y + 4.0, 80.0, 22.0), "Randomize", false);
        }
        draw_line(tb.x, tb.y + tb.h, tb.x + tb.w, tb.y + tb.h, 1.0, C_LINE);
        (ui.take_popup_result(&addk), ui.take_popup_result(&actk), rnd)
    }

    fn graph_to_screen(&self, body: Rect, p: Vec2) -> Vec2 {
        let origin = vec2(body.x + body.w * 0.5, body.y + body.h - 70.0);
        origin + (p + self.graph_off) * self.graph_zoom
    }

    fn draw_generation(&mut self, ui: &mut Ui, tb: Rect, body: Rect) {
        let sel_node = if let Sel::Node(id) = self.sel { Some(id) } else { None };
        let (add, act, rnd) = self.toolbar(ui, tb, "gen", &["Trunk", "Branch", "Leaf"], &["Duplicate", "Delete", "Hide / show", "Arrange nodes", "Frame graph"], true);
        if let Some(i) = add {
            let kind = [Kind::Trunk, Kind::Branch, Kind::Leaf][i];
            let parent = sel_node.unwrap_or_else(|| {
                // default parent: Tree for trunks, newest branch-type node otherwise
                if kind == Kind::Trunk {
                    self.root_id()
                } else {
                    self.nodes.iter().rev().find(|n| n.kind.makes_spines()).map(|n| n.id).unwrap_or(self.root_id())
                }
            });
            self.add_node(kind, parent);
        }
        if let Some(i) = act {
            match (i, sel_node) {
                (0, Some(id)) => self.duplicate_node(id),
                (1, Some(id)) => self.delete_node(id),
                (2, Some(id)) => {
                    if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                        n.hidden = !n.hidden;
                    }
                    self.dirty = true;
                }
                (3, _) => self.arrange(),
                (4, _) => {
                    self.graph_off = Vec2::ZERO;
                    self.graph_zoom = 1.0;
                }
                _ => self.set_status("Select a node first"),
            }
        }
        if rnd {
            self.randomize(sel_node);
        }
        // context menu result
        if let Some(i) = ui.take_popup_result("gen.ctx") {
            if let Some(id) = sel_node {
                match i {
                    0 => {
                        self.add_node(Kind::Branch, id);
                    }
                    1 => {
                        self.add_node(Kind::Leaf, id);
                    }
                    2 => self.duplicate_node(id),
                    3 => {
                        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                            n.hidden = !n.hidden;
                        }
                        self.dirty = true;
                    }
                    _ => self.delete_node(id),
                }
            }
        }

        ui.set_clip(Some(body));
        // background grid dots
        let step = 40.0 * self.graph_zoom;
        let o = self.graph_to_screen(body, Vec2::ZERO);
        let mut gx = body.x + (o.x - body.x).rem_euclid(step);
        while gx < body.x + body.w {
            let mut gy = body.y + (o.y - body.y).rem_euclid(step);
            while gy < body.y + body.h {
                draw_rectangle(gx, gy, 1.5, 1.5, Color::new(1.0, 1.0, 1.0, 0.07));
                gy += step;
            }
            gx += step;
        }
        let rad = 26.0 * self.graph_zoom;
        // links
        for n in &self.nodes {
            if let Some(pid) = n.parent {
                if let Some(p) = self.node(pid) {
                    let a = self.graph_to_screen(body, p.pos) - vec2(0.0, rad);
                    let b = self.graph_to_screen(body, n.pos) + vec2(0.0, rad);
                    let c = if sel_node == Some(n.id) { C_ORANGE } else { Color::new(1.0, 1.0, 1.0, 0.35) };
                    draw_line(a.x, a.y, b.x, b.y, 2.0, c);
                    draw_circle(a.x, a.y, 3.0, c);
                }
            }
        }
        let m = ui.mouse();
        let mut hover_node = None;
        for n in self.nodes.iter() {
            let c = self.graph_to_screen(body, n.pos);
            if c.distance(m) <= rad && body.contains(m) && !ui.blocked {
                hover_node = Some(n.id);
            }
        }
        // nodes
        let nodes = self.nodes.clone();
        for n in &nodes {
            let c = self.graph_to_screen(body, n.pos);
            let alpha = if n.hidden { 0.35 } else { 1.0 };
            let tex = self.icons.for_kind(n.kind);
            let tint = if n.kind == Kind::Trunk { Color::new(0.85, 0.75, 0.7, alpha) } else { Color::new(1.0, 1.0, 1.0, alpha) };
            draw_circle(c.x + 2.0, c.y + 3.0, rad, Color::new(0.0, 0.0, 0.0, 0.3));
            draw_texture_ex(tex, c.x - rad, c.y - rad, tint, DrawTextureParams { dest_size: Some(vec2(rad * 2.0, rad * 2.0)), ..Default::default() });
            let selected = sel_node == Some(n.id);
            if selected || hover_node == Some(n.id) {
                let rr = rad * 1.18;
                let col = if selected { C_ORANGE } else { Color::new(1.0, 1.0, 1.0, 0.45) };
                draw_texture_ex(&self.icons.ring, c.x - rr, c.y - rr, col, DrawTextureParams { dest_size: Some(vec2(rr * 2.0, rr * 2.0)), ..Default::default() });
            }
            let fs = (12.0 * self.graph_zoom.sqrt()).clamp(9.0, 15.0);
            ui.text(&n.name, c.x + rad + 8.0, c.y - fs * 0.5 - 1.0, fs, if selected { WHITE } else { C_TEXT });
            if n.kind != Kind::Tree {
                let cnt = self.instance_count(n.id).to_string();
                let cw = ui.text_w(&cnt, 10.5) + 8.0;
                let br = Rect::new(c.x + rad * 0.55, c.y - rad * 1.05, cw, 15.0);
                draw_rectangle(br.x, br.y, br.w, br.h, Color::new(0.1, 0.1, 0.1, 0.85));
                ui.text_center(&cnt, br, 10.5, C_TEXT_DIM);
            }
            ui.record(&format!("node:{}", n.name), Rect::new(c.x - rad, c.y - rad, rad * 2.0, rad * 2.0));
        }

        // interaction
        let inp = ui.inp.clone();
        let in_body = body.contains(m) && !ui.blocked && ui.popup.is_none();
        if in_body && inp.wheel != 0.0 {
            let before = (m - self.graph_to_screen(body, Vec2::ZERO)) / self.graph_zoom;
            self.graph_zoom = (self.graph_zoom * 1.1f32.powf(inp.wheel)).clamp(0.4, 2.5);
            let after = (m - self.graph_to_screen(body, Vec2::ZERO)) / self.graph_zoom;
            self.graph_off += after - before;
        }
        if in_body && ui.active == 0 && self.graph_drag == GraphDrag::None {
            if inp.l_pressed {
                if let Some(id) = hover_node {
                    self.select(Sel::Node(id));
                    self.graph_drag = if inp.shift { GraphDrag::Link { from: id } } else { GraphDrag::Node { id, start: m, moved: false } };
                    ui.active = hid("graph");
                } else {
                    self.select(Sel::None);
                    self.graph_drag = GraphDrag::Pan;
                    ui.active = hid("graph");
                }
            } else if inp.r_pressed {
                if let Some(id) = hover_node {
                    self.select(Sel::Node(id));
                    ui.open_popup(hid("gen.ctx"), Rect::new(m.x, m.y, 10.0, 0.0), vec!["Add branch".into(), "Add leaf".into(), "Duplicate".into(), "Hide / show".into(), "Delete".into()], None);
                } else {
                    self.graph_drag = GraphDrag::Pan;
                }
            } else if inp.m_pressed {
                self.graph_drag = GraphDrag::Pan;
            }
        }
        let d = m - self.last_graph_mouse();
        match self.graph_drag {
            GraphDrag::Node { id, start, moved } => {
                if inp.l_down {
                    let mv = moved || start.distance(m) > 4.0;
                    if mv {
                        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                            n.pos += d / self.graph_zoom;
                        }
                    }
                    self.graph_drag = GraphDrag::Node { id, start, moved: mv };
                } else {
                    self.graph_drag = GraphDrag::None;
                }
            }
            GraphDrag::Link { from } => {
                if let Some(f) = self.node(from) {
                    let a = self.graph_to_screen(body, f.pos);
                    draw_line(a.x, a.y, m.x, m.y, 2.0, C_ACCENT2);
                }
                if !inp.l_down {
                    if let Some(to) = hover_node {
                        // dragging from child onto the new parent
                        self.reparent(from, to);
                    }
                    self.graph_drag = GraphDrag::None;
                }
            }
            GraphDrag::Pan => {
                if inp.l_down || inp.r_down || inp.m_down {
                    self.graph_off += d / self.graph_zoom;
                } else {
                    self.graph_drag = GraphDrag::None;
                }
            }
            GraphDrag::None => {}
        }
        self.graph_mouse = m;
        if self.nodes.len() == 1 {
            let t = "Use + Add to grow a trunk from the Tree node";
            let w = ui.text_w(t, 12.5);
            ui.text(t, body.x + (body.w - w) * 0.5, body.y + 30.0, 12.5, C_TEXT_FAINT);
        }
        ui.set_clip(None);
        let hint = "Shift+drag a node onto another to link it  •  right-click a node for more";
        ui.text_fit(hint, body.x + 8.0, body.y + body.h - 18.0, body.w - 16.0, 11.0, C_TEXT_FAINT);
    }

    fn last_graph_mouse(&self) -> Vec2 {
        self.graph_mouse
    }

    // ------------------------------------------------------------------ materials (see matpanel.rs)
    /// Render the material preview (sphere or card) into its picture. Call before the UI pass.
    pub fn render_preview(&mut self) {
        if self.tab != 1 {
            return;
        }
        let gm = match &self.gm {
            Some(g) => g,
            None => return,
        };
        let rt = self.preview_rt.get_or_insert_with(|| {
            let r = render_target_ex(512, 512, RenderTargetParams { depth: true, ..Default::default() });
            r.texture.set_filter(FilterMode::Linear);
            r
        });
        let cur = self.mat_cur.min(self.materials.len().saturating_sub(1));
        let m = match self.materials.get(cur) {
            Some(m) => m,
            None => return,
        };
        let m = if m.is_set { m.set_members.first().and_then(|(k, _)| self.materials.get(*k)).unwrap_or(m) } else { m };
        // turned only by the person (drag on the preview), never on its own
        let cam = Cam { yaw: self.mat_yaw, pitch: self.mat_pitch, dist: (3.6 + self.mat_zoom).clamp(1.4, 12.0), target: Vec3::ZERO, ortho: false, view: 0, fov: 40.0 };
        let mut c3 = cam.camera3d(None, self.preview_aspect.max(0.2), Some(rt.clone()));
        c3.viewport = None;
        set_camera(&c3);
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        gl_use_material(gm);
        render::apply_material(gm, m, &self.light, cam.position());
        let src = match self.preview_shape {
            1 => &self.card,
            2 => &self.cylinder,
            3 if !self.custom_preview.is_empty() => &self.custom_preview,
            _ => &self.sphere,
        };
        for mesh in src {
            let tmp = Mesh { vertices: mesh.vertices.clone(), indices: mesh.indices.clone(), texture: m.tex.clone() };
            draw_mesh(&tmp);
        }
        unsafe { get_internal_gl() }.flush();
        gl_use_default_material();
        set_default_camera();
    }

    /// "Custom" material preview: pick an OBJ and show the material on it. Returns false when
    /// nothing was loaded.
    pub fn load_custom_preview(&mut self) -> bool {
        let f = match rfd::FileDialog::new().add_filter("Wavefront OBJ", &["obj"]).pick_file() {
            Some(f) => f.to_string_lossy().to_string(),
            None => return false,
        };
        match meshio::load_obj(&f) {
            Ok(g) if !g.idx.is_empty() => {
                self.custom_preview = render::geom_preview_mesh(&g).into_iter().map(|(a, b)| Mesh { vertices: a, indices: b, texture: None }).collect();
                self.custom_preview_name = std::path::Path::new(&f).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                true
            }
            Ok(_) => {
                self.set_status("That mesh file has no faces");
                false
            }
            Err(e) => {
                self.set_status(&format!("Couldn't load the mesh: {}", e));
                false
            }
        }
    }

    // ------------------------------------------------------------------ meshes
    /// The Meshes tab, laid out like the original's Meshes panel (fn 0x140076fd0 in 8.4.2):
    /// drag handle, mesh picker, +/-, +, copy/paste; Geometry (Edit Mesh..., High/Med/Low
    /// files, More); preview; Show / Alignment / V:F; Transform; Collection; Wind direction;
    /// Proxy collision.
    fn draw_meshes(&mut self, ui: &mut Ui, tb: Rect, body: Rect) {
        let r = Rect::new(tb.x, tb.y, tb.w, body.y + body.h - tb.y);
        let x0 = r.x + 8.0;
        let w = r.w - 16.0;
        let mut y = r.y + 4.0;

        // ---- top row
        let hand = Rect::new(x0, y, 30.0, 30.0);
        ui.record("btn:mesh.drag", hand);
        let hot = ui.hover(hand);
        draw_rectangle(hand.x, hand.y, hand.w, hand.h, if hot || self.hand_drag { C_BTN_HOT } else { C_BTN });
        icon_hand(hand);
        if hot {
            ui.tooltip = Some("Drag this mesh and drop it on the model to use it.".into());
            if ui.inp.l_pressed && !self.meshes.is_empty() {
                self.hand_drag = true;
                ui.active = hid("mesh.drag");
            }
        }
        let bw = 34.0;
        let combo_w = (w - 36.0 - (bw + 4.0) * 3.0).max(60.0);
        let names: Vec<String> = if self.meshes.is_empty() { vec!["No meshes".into()] } else { self.meshes.iter().map(|m| m.name.clone()).collect() };
        let mut cur = self.mesh_cur.min(self.meshes.len().saturating_sub(1));
        let cr = Rect::new(x0 + 36.0, y + 4.0, combo_w, 22.0);
        if ui.combo("mesh.pick", cr, &names, &mut cur) && !self.meshes.is_empty() {
            self.mesh_lod = 0;
            self.select(Sel::Mesh(cur));
        }
        if ui.hover(cr) {
            ui.tooltip = Some("Select a mesh to edit. Meshes are used as leaf geometry.".into());
        }
        let bx = cr.x + cr.w + 4.0;
        let r1 = Rect::new(bx, y + 4.0, bw, 22.0);
        if ui.button("mesh.addremove", r1, "+/-") {
            self.open_mesh_dialog(ui.sw, ui.sh);
        }
        if ui.hover(r1) {
            ui.tooltip = Some("Manage the meshes: add, duplicate, rename, reorder and remove.".into());
        }
        let r2 = Rect::new(bx + bw + 4.0, y + 4.0, bw, 22.0);
        if ui.button("mesh.browseadd", r2, "+") {
            self.add_mesh_from_file();
        }
        if ui.hover(r2) {
            ui.tooltip = Some("Browse for a mesh and add it to the list automatically...".into());
        }
        let r3 = Rect::new(bx + (bw + 4.0) * 2.0, y + 4.0, bw, 22.0);
        if ui.button("mesh.copypaste", r3, "") {
            ui.open_popup(hid("mesh.copypaste"), r3, vec!["Copy".into(), "Paste".into(), "Duplicate".into()], None);
        }
        icon_clipboard(r3);
        if ui.hover(r3) {
            ui.tooltip = Some("Copy/Paste menu...".into());
        }
        if let Some(i) = ui.take_popup_result("mesh.addremove") {
            match i {
                0 | 1 => {
                    let mut m = LeafMesh::card(&format!("Mesh {}", self.meshes.len() + 1));
                    m.ref_material = self.mat_cur.min(self.materials.len().saturating_sub(1));
                    if i == 1 {
                        if let Some(img) = self.materials.get(m.ref_material).and_then(|x| x.adjusted.clone()) {
                            m.lods[0].outline = textures::auto_cutout(&img, m.auto_points as usize, m.threshold, m.margin);
                        }
                    }
                    self.meshes.push(m);
                    self.mesh_lod = 0;
                    self.select(Sel::Mesh(self.meshes.len() - 1));
                }
                _ => self.remove_mesh(cur),
            }
        }
        if let Some(i) = ui.take_popup_result("mesh.copypaste") {
            if !self.meshes.is_empty() {
                match i {
                    0 => {
                        self.mesh_clip = Some(self.meshes[cur].clone());
                        self.set_status("Mesh copied");
                    }
                    1 => match self.mesh_clip.clone() {
                        Some(mut m) => {
                            m.name = self.meshes[cur].name.clone();
                            self.meshes[cur] = m;
                            self.dirty = true;
                            self.set_status("Mesh pasted");
                        }
                        None => self.set_status("Nothing copied yet"),
                    },
                    _ => {
                        // the original (fn 0x1406003e0 case 9): "<name> Copy", made unique, added at
                        // the end with the same settings, and shown in the panel
                        let mut m = self.meshes[cur].clone();
                        let names: Vec<String> = self.meshes.iter().map(|x| x.name.clone()).collect();
                        m.name = crate::meshdialog::unique_name_in(&names, &format!("{} Copy", m.name));
                        self.meshes.push(m);
                        self.mesh_lod = 0;
                        self.select(Sel::Mesh(self.meshes.len() - 1));
                    }
                }
            }
        }
        // drag-and-drop the mesh onto the model
        if self.hand_drag {
            if ui.inp.l_down {
                if let Some(m) = self.meshes.get(cur) {
                    ui.tooltip = Some(format!("Drop on the viewport to use \"{}\" for the leaves", m.name));
                }
            } else {
                self.hand_drag = false;
                if self.viewport.contains(ui.mouse()) && !self.meshes.is_empty() {
                    self.drop_mesh_on_model(cur);
                }
            }
        }
        y += 38.0;
        if self.meshes.is_empty() {
            ui.text("No meshes. Use +/- to add one, or + to load a mesh file.", x0, y + 6.0, 12.5, C_TEXT_FAINT);
            return;
        }
        self.mesh_cur = cur;

        // ---- Geometry
        ui.text("Geometry", x0, y, 12.0, C_TEXT_DIM);
        y += 17.0;
        let lw = 40.0;
        let er = Rect::new(x0 + lw, y, w - lw - 90.0, 22.0);
        if ui.button("mesh.edit", er, "Edit Mesh...") {
            self.open_cutout_editor(cur);
        }
        y += 26.0;
        for l in 0..3 {
            let lab = LOD_NAMES[l];
            let tw = ui.text_w(lab, 12.0);
            ui.text(lab, x0 + lw - tw - 6.0, y + 4.0, 12.0, C_TEXT_DIM);
            let fr = Rect::new(x0 + lw, y, w - lw - 90.0, 22.0);
            let selected = self.mesh_lod == l;
            draw_rectangle(fr.x, fr.y, fr.w, fr.h, C_FIELD);
            if selected {
                draw_rectangle_lines(fr.x, fr.y, fr.w, fr.h, 1.0, C_ACCENT2);
            }
            let lod = &self.meshes[cur].lods[l];
            let desc = if self.meshes[cur].collection {
                "(collection)".to_string()
            } else if !lod.file.is_empty() {
                std::path::Path::new(&lod.file).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
            } else if self.meshes[cur].embedded {
                // the original labels every level of an embedded mesh this way (fn 0x14007e690)
                "Embedded".to_string()
            } else {
                String::new()
            };
            ui.text_fit(&desc, fr.x + 6.0, fr.y + 4.0, fr.w - 10.0, 12.0, C_TEXT);
            ui.record(&format!("mesh.lodfield.{}", l), fr);
            if ui.clicked(fr) {
                self.mesh_lod = l;
            }
            let b1 = Rect::new(fr.x + fr.w + 4.0, y, 26.0, 22.0);
            if ui.button(&format!("mesh.browse{}", l), b1, "...") {
                self.browse_lod(cur, l);
            }
            if ui.hover(b1) {
                ui.tooltip = Some(format!("Browse for the {} mesh file...", lab.to_lowercase()));
            }
            let b2 = Rect::new(b1.x + 30.0, y, 26.0, 22.0);
            if ui.button(&format!("mesh.reload{}", l), b2, "") {
                self.reload_lod(cur, l);
            }
            icon_reload(b2);
            if ui.hover(b2) {
                ui.tooltip = Some("Reload this mesh.".into());
            }
            let b3 = Rect::new(b2.x + 30.0, y, 26.0, 22.0);
            if ui.button(&format!("mesh.export{}", l), b3, "") {
                self.export_lod(cur, l);
            }
            icon_export(b3);
            if ui.hover(b3) {
                ui.tooltip = Some("Export this mesh...".into());
            }
            y += 25.0;
        }
        {
            let tw = ui.text_w("More", 12.0);
            ui.text("More", x0 + lw - tw - 6.0, y + 4.0, 12.0, C_TEXT_DIM);
            let mut more_names = vec!["None".to_string()];
            for (i, m) in self.meshes.iter().enumerate() {
                more_names.push(if i == cur { format!("{} (this mesh)", m.name) } else { m.name.clone() });
            }
            let mut more = self.meshes[cur].more.min(more_names.len() - 1);
            let mr = Rect::new(x0 + lw, y, w - lw - 90.0, 22.0);
            if ui.combo("mesh.more", mr, &more_names, &mut more) {
                if more == cur + 1 {
                    self.set_status("A mesh can't supply its own extra levels");
                } else {
                    self.meshes[cur].more = more;
                    self.dirty = true;
                }
            }
            if ui.hover(mr) {
                ui.tooltip = Some("Another mesh whose levels continue this one's (LOD 3 and lower).".into());
            }
            y += 28.0;
        }

        // ---- preview / cutout editor
        let bottom_h = 232.0;
        let pv = Rect::new(x0, y, w, (r.y + r.h - bottom_h - y).max(90.0));
        ui.record("meshcanvas", pv);
        if self.mesh_editing {
            // editing now happens in the Edit Cutout window (cutout.rs)
            self.mesh_editing = false;
        } else {
            checker(pv, 12.0);
            if let Some(rt) = &self.mesh_rt {
                draw_texture_ex(&rt.texture, pv.x, pv.y, WHITE, DrawTextureParams { dest_size: Some(vec2(pv.w, pv.h)), flip_y: true, ..Default::default() });
            }
            draw_rectangle_lines(pv.x, pv.y, pv.w, pv.h, 1.0, C_LINE);
            ui.text("drag to turn • wheel to zoom • double-click to reset", pv.x + 6.0, pv.y + pv.h - 16.0, 10.5, C_TEXT_FAINT);
            // wireframe by default; the material view is optional
            let bw = Rect::new(pv.x + pv.w - 150.0, pv.y + 4.0, 76.0, 20.0);
            let bm = Rect::new(pv.x + pv.w - 72.0, pv.y + 4.0, 68.0, 20.0);
            if ui.button_ex("mesh.wire", bw, "Wireframe", !self.mesh_show_material, true) {
                self.mesh_show_material = false;
            }
            if ui.button_ex("mesh.matview", bm, "Material", self.mesh_show_material, true) {
                self.mesh_show_material = true;
            }
            if ui.hover(bm) {
                ui.tooltip = Some("Show the mesh with its reference material.".into());
            }
            let m = ui.mouse();
            let on_buttons = Rect::new(bw.x, bw.y, bm.x + bm.w - bw.x, bw.h).contains(m);
            let over = pv.contains(m) && !on_buttons && !ui.blocked && ui.popup.is_none();
            if over && ui.active == 0 && ui.inp.l_pressed {
                if ui.inp.l_double {
                    self.mesh_yaw = 0.0;
                    self.mesh_pitch = 0.0;
                    self.mesh_zoom = 1.0;
                } else {
                    self.mesh_orbit = true;
                    ui.active = hid("meshpreview");
                }
            }
            if self.mesh_orbit {
                if ui.inp.l_down {
                    let d = m - self.graph_mouse_prev_mesh;
                    self.mesh_yaw -= d.x * 0.01;
                    self.mesh_pitch = (self.mesh_pitch + d.y * 0.01).clamp(-1.5, 1.5);
                } else {
                    self.mesh_orbit = false;
                }
            }
            if over && ui.inp.wheel != 0.0 {
                self.mesh_zoom = (self.mesh_zoom * 0.88f32.powf(ui.inp.wheel)).clamp(0.2, 5.0);
            }
            // right-click view menu, like the original's mesh preview (fn 0x1402ecb70)
            if over && ui.active == 0 && ui.inp.r_pressed {
                let tick = |on: bool, s: &str| format!("{} {}", if on { "[x]" } else { "[  ]" }, s);
                ui.open_popup(hid("mesh.viewmenu"), Rect::new(m.x, m.y, 10.0, 0.0), vec![tick(self.mesh_lit, "Lit"), tick(self.mesh_scribed, "Scribed")], None);
            }
            match ui.take_popup_result("mesh.viewmenu") {
                Some(0) => self.mesh_lit = !self.mesh_lit,
                Some(1) => self.mesh_scribed = !self.mesh_scribed,
                _ => {}
            }
            if ui.hover(pv) && !on_buttons && ui.tooltip.is_none() && !self.mesh_orbit {
                ui.tooltip = Some("Right-click for Lit / Scribed".into());
            }
            self.graph_mouse_prev_mesh = m;
        }
        self.mesh_preview_aspect = pv.w / pv.h.max(1.0);
        y = pv.y + pv.h + 6.0;

        // ---- Show / Alignment / counts
        ui.text("Show", x0, y + 4.0, 12.0, C_TEXT_DIM);
        let shows: Vec<String> = LOD_NAMES.iter().map(|s| s.to_string()).collect();
        let mut show = self.mesh_lod.min(2);
        if ui.combo("mesh.show", Rect::new(x0 + 36.0, y, 70.0, 22.0), &shows, &mut show) {
            self.mesh_lod = show;
        }
        if ui.hover(Rect::new(x0 + 36.0, y, 70.0, 22.0)) {
            ui.tooltip = Some("Choose which LOD mesh is displayed.".into());
        }
        let ar = Rect::new(x0 + 116.0, y, 100.0, 22.0);
        ui.checkbox("mesh.align", ar, &mut self.show_alignment);
        ui.text("Alignment", ar.x + 20.0, y + 4.0, 12.0, C_TEXT);
        if ui.hover(ar) {
            ui.tooltip = Some("Show the intended orientation: use Orient and Flip to match it.".into());
        }
        let (nv, nf) = meshio::processed_geom(&self.meshes, cur, self.mesh_lod).map(|g| (g.pos.len(), g.tris())).unwrap_or((0, 0));
        // the original's label: "V:%1  F:%2", plus "  A:%1" when the mesh has anchors (fn 0x1406003e0)
        let anchors = if meshio::processed_geom(&self.meshes, cur, self.mesh_lod).is_some()
            && self.meshes[cur].lods[self.meshes[cur].lod_index(self.mesh_lod)].geom.is_none()
            && !self.meshes[cur].collection
        {
            1
        } else {
            0
        };
        let vf = if anchors > 0 { format!("V:{}  F:{}  A:{}", nv, nf, anchors) } else { format!("V:{}  F:{}", nv, nf) };
        let vw = ui.text_w(&vf, 12.0);
        ui.text(&vf, x0 + w - vw, y + 4.0, 12.0, C_TEXT_DIM);
        ui.record("mesh.vf", Rect::new(x0 + w - vw, y, vw, 20.0));
        y += 28.0;

        // ---- two columns of settings
        let cw = (w - 8.0) * 0.5;
        let lx = x0;
        let rx = x0 + cw + 8.0;
        let mut changed = false;
        let m = &mut self.meshes[cur];
        // Transform
        group_box(ui, Rect::new(lx, y, cw, 104.0), "Transform");
        let lab_w = 62.0;
        let row = |ui: &mut Ui, x: f32, yy: f32, label: &str| {
            let tw = ui.text_w(label, 12.0);
            ui.text(label, x + lab_w - tw - 6.0, yy + 4.0, 12.0, C_TEXT_DIM);
        };
        let mut yy = y + 16.0;
        row(ui, lx, yy, "Orient");
        let orients: Vec<String> = ORIENTS.iter().map(|o| o.0.to_string()).collect();
        let fr = Rect::new(lx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.combo("mesh.orient", fr, &orients, &mut m.orient);
        if ui.hover(fr) {
            ui.tooltip = Some("Reorient the mesh, if necessary (convert coordinate systems).".into());
        }
        yy += 25.0;
        row(ui, lx, yy, "Flip");
        let flips: Vec<String> = FLIPS.iter().map(|s| s.to_string()).collect();
        let fr = Rect::new(lx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.combo("mesh.flip", fr, &flips, &mut m.flip);
        if ui.hover(fr) {
            ui.tooltip = Some("Flip the model around the specified axis.".into());
        }
        yy += 25.0;
        let cb = Rect::new(lx + 6.0, yy, cw * 0.5 - 6.0, 18.0);
        changed |= ui.checkbox("mesh.flipnormals", cb, &mut m.flip_normals);
        ui.text("Flip normals", cb.x + 20.0, yy + 2.0, 12.0, C_TEXT);
        if ui.hover(cb) {
            ui.tooltip = Some("Make the model's normals point in the opposite direction.".into());
        }
        let cb = Rect::new(lx + cw * 0.5, yy, cw * 0.5 - 6.0, 18.0);
        changed |= ui.checkbox("mesh.fixwinding", cb, &mut m.fix_winding);
        ui.text("Fix winding", cb.x + 20.0, yy + 2.0, 12.0, C_TEXT);
        if ui.hover(cb) {
            ui.tooltip = Some("Make each face's winding match its normals.".into());
        }
        // Collection
        let cy = y + 110.0;
        group_box(ui, Rect::new(lx, cy, cw, 84.0), "");
        let cb = Rect::new(lx + 6.0, cy + 2.0, cw - 12.0, 18.0);
        changed |= ui.checkbox("mesh.collection", cb, &mut m.collection);
        ui.text("Collection", cb.x + 20.0, cy + 4.0, 12.0, C_TEXT);
        if ui.hover(cb) {
            ui.tooltip = Some("Build this mesh from generators marked \"Contribute to collections\".".into());
        }
        let on = m.collection;
        let mut yy = cy + 26.0;
        row(ui, lx, yy, "Spines");
        let sp: Vec<String> = SPINE_STYLES.iter().map(|s| s.to_string()).collect();
        let fr = Rect::new(lx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.combo("mesh.spines", fr, &sp, &mut m.spine_style);
        if !on {
            draw_rectangle(fr.x, fr.y, fr.w, fr.h, Color::new(0.23, 0.23, 0.23, 0.6));
        }
        yy += 25.0;
        row(ui, lx, yy, "Pivot");
        let pvs: Vec<String> = PIVOT_STYLES.iter().map(|s| s.to_string()).collect();
        let fr = Rect::new(lx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.combo("mesh.pivot", fr, &pvs, &mut m.pivot_style);
        if !on {
            draw_rectangle(fr.x, fr.y, fr.w, fr.h, Color::new(0.23, 0.23, 0.23, 0.6));
        }
        if ui.hover(fr) {
            ui.tooltip = Some("Origin: lie on the original geometry. First: relative to the first collected object.".into());
        }
        // Wind direction
        group_box(ui, Rect::new(rx, y, cw, 70.0), "Wind direction");
        let mut yy = y + 16.0;
        row(ui, rx, yy, "Growth");
        let gs: Vec<String> = GROWTHS.iter().map(|s| s.to_string()).collect();
        let fr = Rect::new(rx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.combo("mesh.growth", fr, &gs, &mut m.growth);
        if ui.hover(fr) {
            ui.tooltip = Some("Specifies the axis that will align with the wind direction (and grows along the leaf).".into());
        }
        yy += 25.0;
        row(ui, rx, yy, "Ripple");
        let rs: Vec<String> = RIPPLES.iter().map(|s| s.to_string()).collect();
        let fr = Rect::new(rx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.combo("mesh.ripple", fr, &rs, &mut m.ripple);
        if ui.hover(fr) {
            ui.tooltip = Some("Specifies the vector along which rippling will occur.".into());
        }
        // Proxy collision
        let py = y + 78.0;
        group_box(ui, Rect::new(rx, py, cw, 70.0), "Proxy collision");
        let mut yy = py + 16.0;
        row(ui, rx, yy, "Scale");
        let fr = Rect::new(rx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.slider("mesh.colscale", fr, &mut m.collision_scale, 0.0, 10.0, 2);
        if ui.hover(fr) {
            ui.tooltip = Some("Multiplies the collision distance for each instance of this mesh.".into());
        }
        yy += 25.0;
        row(ui, rx, yy, "Weight");
        let fr = Rect::new(rx + lab_w, yy, cw - lab_w - 6.0, 22.0);
        changed |= ui.slider("mesh.weight", fr, &mut m.weight, 0.0, 10.0, 2);
        if ui.hover(fr) {
            ui.tooltip = Some("Multiplies the likelihood that this mesh is chosen when it collides with another.".into());
        }
        if changed {
            self.dirty = true;
        }
    }

    fn remove_mesh(&mut self, cur: usize) {
        if self.meshes.is_empty() {
            return;
        }
        self.meshes.remove(cur);
        for n in &mut self.nodes {
            if n.p.leaf_mesh == cur + 1 {
                n.p.leaf_mesh = 0;
            } else if n.p.leaf_mesh > cur + 1 {
                n.p.leaf_mesh -= 1;
            }
        }
        for m in &mut self.meshes {
            if m.more == cur + 1 {
                m.more = 0;
            } else if m.more > cur + 1 {
                m.more -= 1;
            }
        }
        self.mesh_cur = 0;
        self.mesh_lod = 0;
        self.select(Sel::None);
        self.dirty = true;
    }

    fn pick_mesh_file() -> Option<String> {
        rfd::FileDialog::new().add_filter("Mesh (OBJ)", &["obj"]).pick_file().map(|p| p.to_string_lossy().to_string())
    }

    fn add_mesh_from_file(&mut self) {
        if let Some(f) = Self::pick_mesh_file() {
            match meshio::load_obj(&f) {
                Ok(g) => {
                    let name = std::path::Path::new(&f).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or("Mesh".into());
                    let mut m = LeafMesh::card(&name);
                    m.lods[0] = MeshLod { file: f, outline: vec![], geom: Some(g), ..Default::default() };
                    m.embedded = false;
                    m.ref_material = self.mat_cur.min(self.materials.len().saturating_sub(1));
                    self.meshes.push(m);
                    self.mesh_lod = 0;
                    self.select(Sel::Mesh(self.meshes.len() - 1));
                    self.set_status(&format!("Added mesh \"{}\"", name));
                }
                Err(e) => self.set_status(&format!("Couldn't read that mesh: {}", e)),
            }
        }
    }

    /// Browse… for one level, following the original (fn 0x1406003e0 cases 3 and 8): loading a file
    /// over a level's own cutout asks first; loading High offers to find the related LOD files.
    fn browse_lod(&mut self, cur: usize, l: usize) {
        let has_embedded = self.meshes[cur].embedded && self.meshes[cur].lods.iter().any(|x| !x.is_empty());
        if has_embedded {
            let go = rfd::MessageDialog::new()
                .set_title("TreeCraft")
                .set_description("Loading a file will delete this asset's embedded data.\n\nDo you wish to continue?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if go != rfd::MessageDialogResult::Yes {
                return;
            }
        }
        let f = match Self::pick_mesh_file() {
            Some(f) => f,
            None => return,
        };
        match meshio::load_obj(&f) {
            Ok(g) => {
                if self.meshes[cur].embedded {
                    // loading a file deletes the asset's embedded data (fn 0x1400805d0)
                    self.meshes[cur].lods = [MeshLod::default(), MeshLod::default(), MeshLod::default()];
                    self.meshes[cur].embedded = false;
                }
                self.meshes[cur].lods[l] = MeshLod { file: f.clone(), outline: vec![], geom: Some(g), ..Default::default() };
                self.mesh_lod = l;
                self.dirty = true;
            }
            Err(e) => {
                self.set_status(&format!("Couldn't read that mesh: {}", e));
                return;
            }
        }
        if l == 0 {
            let ask = rfd::MessageDialog::new()
                .set_title("TreeCraft")
                .set_description("Attempt to find associated LOD meshes?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if ask == rfd::MessageDialogResult::Yes {
                let found = meshio::find_lod_files(&f);
                let mut n = 0;
                for (k, path) in found.iter().take(2).enumerate() {
                    if let Ok(g) = meshio::load_obj(path) {
                        self.meshes[cur].lods[k + 1] = MeshLod { file: path.clone(), outline: vec![], geom: Some(g), ..Default::default() };
                        n += 1;
                    }
                }
                self.set_status(&format!("Found {} more LOD mesh{}", n, if n == 1 { "" } else { "es" }));
            }
        }
    }

    fn reload_lod(&mut self, cur: usize, l: usize) {
        let file = self.meshes[cur].lods[l].file.clone();
        if file.is_empty() {
            self.set_status("This level is a cutout; there is no file to reload");
            return;
        }
        match meshio::load_obj(&file) {
            Ok(g) => {
                self.meshes[cur].lods[l].geom = Some(g);
                self.dirty = true;
                self.set_status("Reloaded");
            }
            Err(e) => self.set_status(&format!("Couldn't reload: {}", e)),
        }
    }

    fn export_lod(&mut self, cur: usize, l: usize) {
        let g = match meshio::processed_geom(&self.meshes, cur, l) {
            Some(g) => g,
            None => {
                self.set_status("This level is empty");
                return;
            }
        };
        let name = format!("{}_{}.obj", self.meshes[cur].name.replace(' ', "_"), LOD_NAMES[l]);
        if let Some(p) = rfd::FileDialog::new().add_filter("Mesh (OBJ)", &["obj"]).set_file_name(&name).save_file() {
            match meshio::save_obj(&p.to_string_lossy(), &g) {
                Ok(()) => self.set_status("Mesh exported"),
                Err(e) => self.set_status(&format!("Couldn't export: {}", e)),
            }
        }
    }

    /// Dropping a mesh on the model: the selected leaf generator uses it (or the first one).
    pub fn drop_mesh_on_model(&mut self, cur: usize) {
        let target = match self.sel {
            Sel::Node(id) if self.node(id).map(|n| n.kind == Kind::Leaf).unwrap_or(false) => Some(id),
            _ => self.nodes.iter().find(|n| n.kind == Kind::Leaf).map(|n| n.id),
        };
        match target {
            Some(id) => {
                let name = self.meshes[cur].name.clone();
                if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                    n.p.leaf_mesh = cur + 1;
                    let nn = n.name.clone();
                    self.set_status(&format!("\"{}\" now uses mesh \"{}\"", nn, name));
                }
                self.dirty = true;
            }
            None => self.set_status("Add a Leaf generator first"),
        }
    }

    /// Render the selected mesh into the Meshes tab's preview picture (before the UI pass).
    pub fn render_mesh_preview(&mut self) {
        if self.tab != 2 || self.mesh_editing || self.meshes.is_empty() {
            return;
        }
        let gm = match &self.gm {
            Some(g) => g,
            None => return,
        };
        let rt = self.mesh_rt.get_or_insert_with(|| {
            let r = render_target_ex(640, 512, RenderTargetParams { depth: true, ..Default::default() });
            r.texture.set_filter(FilterMode::Linear);
            r
        });
        let cur = self.mesh_cur.min(self.meshes.len() - 1);
        let g = meshio::processed_geom(&self.meshes, cur, self.mesh_lod);
        let (bmin, bmax) = g.as_ref().map(meshio::bounds).unwrap_or((Vec3::splat(-0.5), Vec3::splat(0.5)));
        let center = (bmin + bmax) * 0.5;
        let radius = ((bmax - bmin).length() * 0.5).max(0.05);
        let cam = Cam {
            yaw: self.mesh_yaw,
            pitch: self.mesh_pitch,
            dist: radius / (20.0f32.to_radians()).sin() * 1.15 * self.mesh_zoom,
            target: center,
            ortho: false,
            view: 0,
            fov: 40.0,
        };
        let c3 = cam.camera3d(None, self.mesh_preview_aspect.max(0.2), Some(rt.clone()));
        set_camera(&c3);
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        if let Some(g) = &g {
            let refm = self.meshes[cur].ref_material;
            let fallback = Material::new("?", 0);
            let mat = self.materials.get(refm).unwrap_or(&fallback);
            let verts: Vec<macroquad::models::Vertex> = g
                .pos
                .iter()
                .enumerate()
                .map(|(i, p)| macroquad::models::Vertex {
                    position: *p,
                    uv: g.uv[i],
                    color: [255; 4],
                    normal: vec4(g.nrm[i].x, g.nrm[i].y, g.nrm[i].z, 1.0),
                })
                .collect();
            if self.mesh_show_material {
                let idx: Vec<u16> = g.idx.iter().map(|&i| i as u16).collect();
                gl_use_material(gm);
                let light = Lighting { unlit: !self.mesh_lit, ..self.light };
                render::apply_material(gm, mat, &light, cam.position());
                draw_mesh(&Mesh { vertices: verts, indices: idx, texture: mat.tex.clone() });
                unsafe { get_internal_gl() }.flush();
                gl_use_default_material();
            }
            if !self.mesh_show_material || self.mesh_scribed {
                // wireframe (the default), or edges drawn over the material when Scribed is on
                let wc = Color::new(0.95, 0.97, 1.0, 1.0);
                let mut seen: HashSet<(u32, u32)> = HashSet::new();
                for tri in g.idx.chunks(3) {
                    if tri.len() < 3 {
                        break;
                    }
                    for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                        if seen.insert((a.min(b), a.max(b))) {
                            draw_line_3d(g.pos[a as usize], g.pos[b as usize], wc);
                        }
                    }
                }
            }
        }
        // axes at the mesh origin (the attachment point), growth axis in yellow; an empty mesh shows
        // an empty preview, like the original
        if g.is_some() {
            let len = radius * 1.2;
            draw_line_3d(Vec3::ZERO, Vec3::X * len, Color::new(0.9, 0.35, 0.35, 1.0));
            draw_line_3d(Vec3::ZERO, Vec3::Y * len, Color::new(0.4, 0.85, 0.4, 1.0));
            draw_line_3d(Vec3::ZERO, Vec3::Z * len, Color::new(0.4, 0.55, 1.0, 1.0));
            let ga = meshio::growth_axis(self.meshes[cur].growth) * len * 1.1;
            draw_line_3d(Vec3::ZERO, ga, Color::new(1.0, 0.85, 0.2, 1.0));
        }
        if self.show_alignment {
            // the intended orientation: a leaf lying in X/Y, growing up +Y from the origin, facing +Z
            let s = radius * 1.6;
            let mut prev: Option<Vec3> = None;
            let shape: Vec<Vec3> = (0..=24)
                .map(|i| {
                    let t = i as f32 / 24.0;
                    let a = t * std::f32::consts::TAU;
                    let yv = (1.0 - a.cos()) * 0.5;
                    vec3(a.sin() * 0.35 * (1.0 - yv * 0.4), yv, 0.0) * s
                })
                .collect();
            for p in shape {
                if let Some(q) = prev {
                    draw_line_3d(q, p, Color::new(1.0, 1.0, 1.0, 0.7));
                }
                prev = Some(p);
            }
            draw_line_3d(Vec3::ZERO, Vec3::Y * s, Color::new(1.0, 1.0, 1.0, 0.5));
        }
        set_default_camera();
    }
}

pub(crate) fn group_box(ui: &mut Ui, r: Rect, title: &str) {
    draw_rectangle_lines(r.x, r.y + 6.0, r.w, r.h - 6.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.14));
    if !title.is_empty() {
        let tw = ui.text_w(title, 12.0);
        draw_rectangle(r.x + 6.0, r.y, tw + 8.0, 12.0, C_PANEL);
        ui.text(title, r.x + 10.0, r.y - 1.0, 12.0, C_TEXT_DIM);
    }
}

pub(crate) fn icon_hand(r: Rect) {
    let c = Color::new(1.0, 1.0, 1.0, 0.85);
    let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
    draw_rectangle(cx - 6.0, cy - 1.0, 12.0, 9.0, c);
    for (i, h) in [6.0f32, 9.0, 9.0, 7.0].iter().enumerate() {
        draw_rectangle(cx - 6.0 + i as f32 * 3.2, cy - 1.0 - h, 2.4, *h, c);
    }
    draw_rectangle(cx + 5.0, cy - 2.0, 4.0, 2.4, c);
}

fn icon_reload(r: Rect) {
    let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
    let c = Color::new(1.0, 1.0, 1.0, 0.8);
    let mut prev: Option<Vec2> = None;
    for i in 0..=14 {
        let a = 0.6 + i as f32 / 14.0 * 5.0;
        let p = vec2(cx + a.cos() * 6.0, cy + a.sin() * 6.0);
        if let Some(q) = prev {
            draw_line(q.x, q.y, p.x, p.y, 1.5, c);
        }
        prev = Some(p);
    }
    let tip = vec2(cx + 0.6f32.cos() * 6.0, cy + 0.6f32.sin() * 6.0);
    draw_triangle(tip + vec2(-3.0, -3.0), tip + vec2(3.0, -1.0), tip + vec2(0.0, 3.0), c);
}

fn icon_export(r: Rect) {
    let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
    let c = Color::new(1.0, 1.0, 1.0, 0.8);
    draw_rectangle_lines(cx - 2.0, cy - 6.0, 8.0, 12.0, 1.2, c);
    draw_line(cx - 8.0, cy, cx + 1.0, cy, 1.5, c);
    draw_triangle(vec2(cx + 3.0, cy), vec2(cx - 1.0, cy - 3.5), vec2(cx - 1.0, cy + 3.5), c);
}

pub(crate) fn icon_clipboard(r: Rect) {
    let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
    let c = Color::new(1.0, 1.0, 1.0, 0.85);
    draw_rectangle_lines(cx - 5.0, cy - 6.0, 10.0, 13.0, 1.3, c);
    draw_rectangle(cx - 2.5, cy - 7.5, 5.0, 3.0, c);
    draw_line(cx - 3.0, cy - 1.0, cx + 3.0, cy - 1.0, 1.0, c);
    draw_line(cx - 3.0, cy + 2.0, cx + 3.0, cy + 2.0, 1.0, c);
}

fn draw_gpu(gpu: &GpuTree, mats: &[Material], gm: &macroquad::material::Material, light: &Lighting, cam_pos: Vec3) {
    gl_use_material(gm);
    for (mi, meshes) in &gpu.parts {
        let fallback = Material::new("?", 0);
        let mut m = mats.get(*mi).unwrap_or(&fallback);
        // a material set draws as its first member (weights pick between members in the original)
        if m.is_set {
            if let Some(x) = m.set_members.first().and_then(|(k, _)| mats.get(*k)) {
                m = x;
            }
        }
        render::apply_material(gm, m, light, cam_pos);
        for mesh in meshes {
            draw_mesh(mesh);
        }
        unsafe { get_internal_gl() }.flush();
    }
    gl_use_default_material();
}
