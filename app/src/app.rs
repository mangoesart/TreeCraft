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
    /// The light's settings in the Properties panel (picture 43).
    Light,
}

pub const TABS: [&str; 3] = ["Generation", "Materials", "Meshes"];

/// Render > WireShaded and Render > Wireframe (picture 24): drawn here, not in the shader.
pub const MODE_WIRESHADED: f32 = 6.0;
pub const MODE_WIREFRAME: f32 = 7.0;

/// Render modes in the original's render_modes folder subfolders (ordered by its naming rule).
const RENDER_LIGHTING: [&str; 6] = ["Untextured", "Diffuse", "Specular", "Ambient", "Subsurface", "Shadow"];
const RENDER_MATERIAL: [&str; 9] = ["Color", "Opacity", "Roughness", "Normal", "Specular", "Metallic", "Subsurface", "Subsurface Amount", "Custom"];
const RENDER_VCOL: [&str; 4] = ["Color", "Red", "Green", "Blue"];
const RENDER_OTHER: [&str; 10] = ["Phong", "Blinn-Phong", "Oren-Nayar", "Lambert", "Flat", "Billboard Subsurface Amount", "Depth", "Overdraw", "Texcoord", "Texcoord Seams"];
/// Wind strengths offered by the Wind menu, and the seasons by the Season menu (0 spring .. 1 winter).
const WIND_LEVELS: [(&str, f32); 4] = [("Calm", 0.25), ("Breeze", 0.5), ("Strong", 1.0), ("Gale", 2.0)];
/// "None" (no tint, the texture as it is: picture 25) first; a season tints leaves.
const SEASONS: [(&str, f32); 7] = [("None", -1.0), ("Spring", 0.0), ("Early summer", 0.2), ("Summer", 0.4), ("Early autumn", 0.6), ("Autumn", 0.8), ("Winter", 1.0)];

/// The generator templates at the top of the original's Add menu (its templates folder, in order).
const GEN_TEMPLATES: [&str; 11] =
    ["Trunk", "Trunk Tube", "Bifurcating Branches", "Big Branches", "Branch Tubes", "Little Branches", "Single Branch", "Twigs", "Cap", "Fronds", "Leaf Mesh"];

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
    /// From a node's top (output) socket: the node dropped on becomes its child.
    LinkDown { parent: u32 },
    /// A force / collision object's node.
    Scene { idx: usize },
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
    /// The same shader drawn additively (Render > Other > Overdraw).
    pub gm_over: Option<macroquad::material::Material>,
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
    /// Viewport toolbar (picture-18): Show menu flags, the render mode chosen, a copied camera,
    /// the collapsed toolbar, and the nodes hidden by Visibility > Toggle isolation.
    pub show_leaves: bool,
    pub show_branches: bool,
    pub render_mode: String,
    pub cam_copy: Option<render::Cam>,
    pub toolbar_collapsed: bool,
    pub isolation: Option<Vec<u32>>,
    /// Scene objects (forces and collision objects), the one picked, and their Show toggles.
    pub scene: Vec<SceneObj>,
    pub scene_sel: Option<usize>,
    pub show_forces: bool,
    pub show_coll: bool,
    /// Show menu extras: bones (spines), normals, no back faces, and toggles for kinds of
    /// geometry TreeCraft doesn't make yet (fronds, zones, targets, proxies).
    pub show_bones: bool,
    pub show_normals: bool,
    pub one_sided: bool,
    pub show_other: [bool; 4],
    /// Wind sway (on, strength), season (0 spring .. 1 winter), ambient occlusion on, leaf
    /// collision quality (0 None .. 3 High), gizmo mode (0 Translate, 1 Rotate, 2 Scale).
    pub wind_on: bool,
    pub wind_strength: f32,
    pub season: f32,
    pub ao_on: bool,
    pub leaf_collision: u8,
    pub gizmo_mode: u8,
    gizmo_drag: Option<(u8, Vec2, SceneObj)>,
    /// The last generated tree (for bones, normals and export).
    pub geo: Option<gen::TreeGeo>,
    /// Undo / Redo history (no limit) and the state last kept.
    pub(crate) undo_stack: Vec<crate::undo::Snapshot>,
    pub(crate) redo_stack: Vec<crate::undo::Snapshot>,
    pub(crate) undo_cur: Option<crate::undo::Snapshot>,
    /// The tree's state when last saved, opened or started (closing asks to save when it differs).
    pub(crate) saved_sig: String,
    /// Edit > Undo selection / Redo selection history.
    pub(crate) sel_back: Vec<Sel>,
    pub(crate) sel_fwd: Vec<Sel>,
    /// Window > Layout: 0 normal, 1 full screen, 2 full screen with no bars.
    pub layout_mode: u8,
    /// Edit mode (picture 24): 0 Generator, 1 Node (Tab), 2 Freehand.
    pub edit_mode: u8,
    /// Node mode: the picked node (generator id, its number in the generator).
    pub node_pick: Option<(u32, usize)>,
    /// The curve being edited (generator id, property name, kind 0 Parent 1 Distribution 2 Profile).
    pub curve_edit: Option<(u32, String, u8)>,
    pub(crate) curve_drag: Option<usize>,
    /// The curve editor window (picture 34): where it is, what is being dragged (edges as bits,
    /// 16 = moving), the mouse last frame, picked points, Pin, Draw, the copied curve, where the
    /// right-click menu opened, the last freehand point, and whether it opened this frame.
    pub curve_win: Rect,
    pub(crate) curve_win_drag: u8,
    pub(crate) curve_last_mouse: Vec2,
    pub(crate) curve_sel: Vec<usize>,
    pub curve_pin: bool,
    pub curve_draw: bool,
    pub(crate) curve_clip: Option<Curve>,
    pub(crate) curve_ctx_at: Vec2,
    pub(crate) curve_draw_last: Option<Vec2>,
    pub(crate) curve_just_opened: bool,
    /// One of the curve editor's own lists was open last frame.
    pub(crate) curve_popup_open: bool,
    /// Variance Settings (picture 35): the property (generator id, name, its +/- button), the
    /// window's place, whether it opened this frame, and the Cohesion slider being dragged.
    pub var_edit: Option<(u32, String, Rect)>,
    pub(crate) var_rect: Rect,
    pub(crate) var_just_opened: bool,
    pub(crate) var_cohesion_drag: bool,
    /// The welcome window (picture 37): when it opened (seconds), None once closed.
    pub welcome: Option<f64>,
    /// The Preferences window's place and size once moved (picture 40), what's being dragged
    /// (1 the title bar, 2..5 a corner) and where the drag began.
    pub prefs_win: Option<Rect>,
    pub(crate) prefs_win_drag: u8,
    pub(crate) prefs_win_grab: (Vec2, Rect),
    pub(crate) prefs_corners: Vec<Rect>,
    /// Freehand: the stroke being drawn (screen points) and where it attaches.
    pub(crate) free_stroke: Vec<Vec2>,
    pub(crate) free_anchor: Option<Vec3>,
    /// Freehand's bend controller (picture 46): the part dragged (1 top, 2 band, 3 pivot) and the
    /// mouse, pivot, top, screen span and screen direction when the drag began.
    pub(crate) fh_drag: u8,
    /// The controller part under the mouse (picture 47).
    pub(crate) fh_hover: u8,
    pub(crate) fh_grab: (Vec2, f32, f32, f32, Vec2),
    /// Where a left click started in the viewport (a click without dragging picks a mesh).
    pub(crate) click_start: Option<Vec2>,
    /// UV tab preview: show the picture, and which (0 Diffuse, 1 Normal, 2 Opacity).
    pub uv_show_tex: bool,
    pub uv_slot: usize,
    /// UV preview: repeat the picture (default off, picture 28).
    pub uv_tile: bool,
    /// UV preview zoom, pan, height, drag state and last mouse, and where it was drawn last frame.
    pub uv_zoom: f32,
    pub uv_pan: Vec2,
    pub uv_height: f32,
    pub uv_drag: u8,
    pub uv_last_mouse: Vec2,
    pub uv_rect: Option<Rect>,
    /// Viewport shading menu (picture 27): 0 Studio, 1 MatCap, 2 Flat; its drop-down; the MatCap
    /// chosen and the pictures; the drop-down's ball preview and what it was drawn for.
    pub shading: u8,
    pub shading_open: bool,
    pub matcap_idx: usize,
    /// MatCap shows the textures' colours too (picture 29).
    pub matcap_texture: bool,
    pub(crate) matcap_tex: Vec<Texture2D>,
    pub(crate) ball_tex: Option<Texture2D>,
    pub(crate) ball_key: (i32, i32, i32),
    /// Dragging the navigation gizmo's background turns the view.
    pub(crate) gizmo_orbit: bool,
    /// Selection menu (picture 30): its drop-down; the outline's mask picture and shaders, and
    /// whether the mask was drawn this frame.
    pub sel_open: bool,
    /// The colour wheel opened from a swatch (picture 31).
    pub wheel: Option<crate::colorwheel::Wheel>,
    pub(crate) mask_mat: Option<macroquad::material::Material>,
    /// The tree's shadow on the ground (picture 42).
    pub(crate) shadow_mat: Option<macroquad::material::Material>,
    /// The shadow's shape, drawn before it's laid on the ground (picture 45).
    pub(crate) shadow_rt: Option<RenderTarget>,
    /// The viewport's light controller (picture 42): dragging in the viewport moves the sun.
    pub light_drag_mode: bool,
    /// Show > Height indicator (picture 42).
    pub show_height: bool,
    /// The viewport's "?" tips card is open (picture 43).
    pub help_open: bool,
    /// When the last autosave (or its clock's start) was (picture 45).
    pub last_autosave: f64,
    /// When a value in the Properties panel last changed, and whether the selection indicator
    /// was hidden for it (Preferences > Tree Window > Hide selection indicator on edit).
    pub prop_edit_t: f64,
    pub sel_hidden: bool,
    /// The mouse button a viewport drag started with (0 left, 1 middle, 2 right).
    pub(crate) vp_btn: u8,
    /// The Properties panel scrolls toward this (Animate scrolling) and a group to bring into
    /// view after a click opened it (Group click).
    pub prop_scroll_target: f32,
    pub prop_focus: Option<String>,
    pub(crate) outline_mat: Option<macroquad::material::Material>,
    pub(crate) sel_rt: Option<RenderTarget>,
    pub(crate) sel_drawn: bool,
    pub(crate) sel_dirty: bool,
    /// Ball pictures for the shading buttons (Studio, Flat) and what the Studio one was drawn for.
    pub(crate) shade_balls: Vec<Texture2D>,
    pub(crate) shade_balls_key: (i32, i32),
    pub(crate) last_mouse_nav: Vec2,
    /// Preferences > Themes: 0 the 3D viewport's colours, 1 the Node Editor's.
    pub prefs_theme_tab: usize,
    /// Where each force / collision object sits in the node graph (None = laid out automatically).
    pub scene_gpos: Vec<Option<Vec2>>,
    /// Viewport pictures to save after the next frames (File > Export image / sequence / Render
    /// photo): path, camera turn for that picture, and whether the viewport's own controls hide.
    pub shot_queue: Vec<(std::path::PathBuf, Option<f32>, bool)>,
    /// Edit > Preferences: the window is open, the page shown, and the settings.
    pub prefs_open: bool,
    /// The file the tree was opened from or last saved to (File > Save writes there).
    pub tree_path: Option<std::path::PathBuf>,
    /// The Properties panel's tab for branch and for leaf generators (0 = All).
    pub prop_tab: usize,
    pub prop_tab_leaf: usize,
    pub prefs_page: usize,
    pub prefs: crate::prefs::Prefs,
    pub(crate) prefs_pick: Option<usize>,
    pub(crate) prefs_drag: u8,
    pub(crate) prefs_name: String,
    pub(crate) prefs_name_focus: bool,
    /// Wind direction (degrees around the vertical).
    pub wind_dir: f32,
    /// The viewport's own controls this frame (the toolbar, the view menu button): clicks there
    /// don't turn the camera.
    pub vp_ui: Vec<Rect>,
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
            // a 10 am / 2 pm sun (45 degrees up, from the front right) and the original's Daylight
            // preset and Ambient Occlusion / Shadows defaults (fn 0x140292fb0; picture 42)
            light: Lighting {
                azimuth: 35.0,
                elevation: 45.0,
                unlit: false,
                mode: 0.0,
                wind: 0.0,
                time: 0.0,
                season: -1.0,
                one_sided: false,
                ao_on: false,
                overdraw: false,
                no_alpha_test: false,
                wind_dir: 0.0,
                shading: 0.0,
                sel_fill: [1.0, 0.62, 0.22, 0.38],
                sun_color: [1.0, 1.0, 1.0],
                sun_intensity: 2.0,
                amb_sky: crate::render::hex_rgb(0x5391d3),
                amb_horizon: crate::render::hex_rgb(0xb2e3ff),
                amb_ground: crate::render::hex_rgb(0x8a8570),
                amb_intensity: 0.4,
                ao_brightness: 0.0,
                ao_contrast: 0.0,
                ao_min: 0.25,
                ao_max: 1.0,
                shadow_quality: 2,
                ground_weight: 0.3,
                // a little light round onto the shadow side, so it isn't too dark (picture 44)
                wrap: 0.3,
                flip_back: true,
            },
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
            gm_over: render::make_overdraw_material().map_err(|e| eprintln!("overdraw shader error: {}", e)).ok(),
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
            show_leaves: true,
            show_branches: true,
            render_mode: "Standard".into(),
            cam_copy: None,
            toolbar_collapsed: false,
            isolation: None,
            scene: vec![],
            scene_sel: None,
            show_forces: true,
            show_coll: true,
            show_bones: false,
            show_normals: false,
            one_sided: false,
            show_other: [true; 4],
            wind_on: false,
            wind_strength: 0.5,
            season: -1.0,
            ao_on: false,
            leaf_collision: 0,
            gizmo_mode: 0,
            gizmo_drag: None,
            geo: None,
            undo_stack: vec![],
            redo_stack: vec![],
            undo_cur: None,
            saved_sig: String::new(),
            sel_back: vec![],
            sel_fwd: vec![],
            layout_mode: 0,
            prefs_theme_tab: 0,
            edit_mode: 0,
            node_pick: None,
            curve_edit: None,
            curve_drag: None,
            curve_win: Rect::new(0.0, 0.0, 0.0, 0.0),
            curve_win_drag: 0,
            curve_last_mouse: Vec2::ZERO,
            curve_sel: vec![],
            curve_pin: false,
            curve_draw: false,
            curve_clip: None,
            curve_ctx_at: Vec2::ZERO,
            curve_draw_last: None,
            curve_just_opened: false,
            curve_popup_open: false,
            var_edit: None,
            var_rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            var_just_opened: false,
            welcome: None,
            prefs_win: None,
            prefs_win_drag: 0,
            prefs_win_grab: (Vec2::ZERO, Rect::new(0.0, 0.0, 0.0, 0.0)),
            prefs_corners: vec![],
            var_cohesion_drag: false,
            free_stroke: vec![],
            free_anchor: None,
            fh_drag: 0,
            fh_hover: 0,
            fh_grab: (Vec2::ZERO, 0.0, 0.0, 1.0, Vec2::ZERO),
            click_start: None,
            uv_show_tex: true,
            uv_slot: 0,
            uv_tile: false,
            uv_zoom: 1.0,
            uv_pan: Vec2::ZERO,
            uv_height: 0.0,
            uv_drag: 0,
            uv_last_mouse: Vec2::ZERO,
            uv_rect: None,
            shading: 0,
            shading_open: false,
            matcap_idx: 0,
            matcap_texture: false,
            matcap_tex: vec![],
            ball_tex: None,
            ball_key: (0, 0, 0),
            gizmo_orbit: false,
            sel_open: false,
            wheel: None,
            mask_mat: render::make_mask_material().map_err(|e| eprintln!("selection mask shader error: {}", e)).ok(),
            shadow_mat: render::make_shadow_material().map_err(|e| eprintln!("shadow shader error: {}", e)).ok(),
            shadow_rt: None,
            light_drag_mode: false,
            show_height: true,
            help_open: false,
            last_autosave: 0.0,
            prop_edit_t: -10.0,
            sel_hidden: false,
            vp_btn: 0,
            prop_scroll_target: 0.0,
            prop_focus: None,
            outline_mat: render::make_outline_material().map_err(|e| eprintln!("outline shader error: {}", e)).ok(),
            sel_rt: None,
            sel_drawn: false,
            sel_dirty: false,
            shade_balls: vec![],
            shade_balls_key: (i32::MIN, 0),
            last_mouse_nav: Vec2::ZERO,
            scene_gpos: vec![],
            shot_queue: vec![],
            prefs_open: false,
            tree_path: None,
            prop_tab: 0,
            prop_tab_leaf: 0,
            prefs_page: 0,
            prefs: crate::prefs::Prefs::load(),
            prefs_pick: None,
            prefs_drag: 0,
            prefs_name: String::new(),
            prefs_name_focus: false,
            wind_dir: 0.0,
            vp_ui: vec![],
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
        // the program starts empty (picture-20): only the Tree generator, nothing in the viewport,
        // and no materials or meshes either (picture-24 removed the default Bark and Leaf)
        app.select(Sel::None);
        app.regenerate();
        app.frame_tree();
        app.undo_reset();
        app
    }

    /// A small tree (trunk, branches, leaves) on default settings, for the self-test.
    pub fn load_demo(&mut self) {
        // the demo has a bark and a leaf material and a leaf cutout
        self.reset_library();
        let root = self.root_id();
        let trunk = self.add_node(Kind::Trunk, root);
        let br = self.add_node(Kind::Branch, trunk);
        self.add_node(Kind::Leaf, br);
        self.select(Sel::None);
        self.regenerate();
        self.frame_tree();
        self.undo_reset();
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

    /// The material a generator's bark shows (for the UV preview): its Materials list's heaviest
    /// entry, or its own material; an Inherited entry (or none at all) takes the parent
    /// generator's, as the bark does (picture 39).
    pub fn shown_material(&self, id: u32) -> Option<usize> {
        let mut cur = Some(id);
        for _ in 0..32 {
            let n = self.node(cur?)?;
            let pick = if n.p.mat_list.is_empty() {
                n.p.material
            } else {
                n.p.mat_list.iter().cloned().fold((INHERITED, f32::MIN), |a, e| if e.1 > a.1 { e } else { a }).0
            };
            if pick != INHERITED {
                return (pick < self.materials.len()).then_some(pick);
            }
            cur = n.parent;
        }
        None
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
            self.sel_back.push(self.sel);
            if self.sel_back.len() > 200 {
                self.sel_back.remove(0);
            }
            self.sel_fwd.clear();
            self.sel = s;
            self.prop_scroll = 0.0;
            self.prop_scroll_target = 0.0;
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
        let x = pp.x + if siblings == 0 { 0.0 } else { (siblings as f32 + 1.0) / 2.0 * 150.0 * if siblings % 2 == 1 { 1.0 } else { -1.0 } };
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
                *x += 150.0;
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
        // the toolbar's Show menu: leaves off skips leaf generators; branches off keeps their
        // spines (leaves stay where they grow) but draws no bark
        let mut geo = self.generate_with(None);
        self.geo_stats = (geo.tris, geo.verts, geo.spines, geo.leaves.saturating_sub(geo.leaves_removed));
        if geo.leaves_removed > 0 && geo.warnings.is_empty() {
            geo.warnings.push(format!("Leaf collision removed {} leaves", geo.leaves_removed));
        }
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
        let collections = std::mem::take(&mut geo.collections);
        geo.leaf_grid.clear();
        self.geo = Some(geo);
        for (mi, g) in collections {
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
        // Preferences > Viewport > Zoom distance scale (the original's "Scales how far away cameras
        // are placed from target objects during zoom operations")
        self.cam.dist *= self.prefs.zoom_scale.max(0.05);
    }

    // ------------------------------------------------------------------ viewport
    pub fn viewport_input(&mut self, ui: &mut Ui, r: Rect) {
        let m = ui.mouse();
        let over = r.contains(m) && !ui.blocked && ui.popup.is_none() && !self.vp_ui.iter().any(|b| b.contains(m));
        let inp = ui.inp.clone();
        // Preferences > Tree Window > Navigation (picture 45): reversed directions
        let (zs, rs, ps) = (if self.prefs.rev_zoom { -1.0 } else { 1.0 }, if self.prefs.rev_rotate { -1.0 } else { 1.0 }, if self.prefs.rev_pan { -1.0 } else { 1.0 });
        if over && inp.wheel != 0.0 {
            self.cam.zoom(inp.wheel * zs);
        }
        // the Gizmo: pressing a force or collision object picks it and drags it (Translate moves
        // it in the view plane, Rotate turns its direction, Scale changes its strength or size)
        let vp = self.cam.camera3d(None, r.w / r.h.max(1.0), None).matrix();
        let to_screen = |p: Vec3| -> Option<Vec2> {
            let c = vp * p.extend(1.0);
            if c.w <= 0.0 {
                return None;
            }
            Some(vec2(r.x + (c.x / c.w * 0.5 + 0.5) * r.w, r.y + (0.5 - c.y / c.w * 0.5) * r.h))
        };
        if over && ui.active == 0 && self.vp_drag == VpDrag::None && inp.l_pressed && self.gizmo_drag.is_none() {
            let hit = self.scene.iter().enumerate().filter(|(_, o)| if o.kind.is_force() { self.show_forces } else { self.show_coll }).find_map(|(k, o)| {
                let s = to_screen(o.pos)?;
                (s.distance(m) < 16.0 * self.prefs.manip_size).then_some(k)
            });
            if let Some(k) = hit {
                self.scene_sel = Some(k);
                self.gizmo_drag = Some((self.gizmo_mode, m, self.scene[k].clone()));
                ui.active = hid("viewport.gizmo");
                ui.inp.l_pressed = false;
                return;
            }
        }
        if let Some((mode, start, o0)) = self.gizmo_drag.clone() {
            if inp.l_down {
                if let Some(k) = self.scene_sel.filter(|&k| k < self.scene.len()) {
                    let d = m - start;
                    let fwd = self.cam.dir();
                    let right = Vec3::Y.cross(fwd).normalize_or_zero();
                    let up = fwd.cross(right).normalize_or_zero();
                    let p = &self.prefs;
                    let snap = (p.snap_move, p.snap_move_unit, p.snap_rotate, p.snap_rotate_deg, p.snap_scale, p.snap_scale_amount);
                    let o = &mut self.scene[k];
                    match mode {
                        0 => {
                            // world units per pixel at the object's distance
                            let dist = (o0.pos - self.cam.position()).length().max(1.0);
                            let k_px = 2.0 * dist * (self.cam.fov.to_radians() * 0.5).tan() / r.h.max(1.0);
                            // cam.dir() points back toward the camera, so `right` is screen-right
                            o.pos = o0.pos + right * d.x * k_px - up * d.y * k_px;
                            // Preferences > Viewport: translation snapping to the snap unit
                            if snap.0 {
                                let u = snap.1.max(0.001);
                                o.pos = (o.pos / u).round() * u;
                            }
                        }
                        1 => {
                            // rotation snapping: whole steps of the snap angle
                            let (mut ay, mut ax) = (d.x * 0.01, d.y * 0.01);
                            if snap.2 {
                                let s = snap.3.to_radians().max(0.001);
                                ay = (ay / s).round() * s;
                                ax = (ax / s).round() * s;
                            }
                            let q = Quat::from_axis_angle(Vec3::Y, ay) * Quat::from_axis_angle(right, ax);
                            o.dir = (q * o0.dir).normalize_or_zero();
                        }
                        _ => {
                            let mut f = (-d.y * 0.01).exp();
                            // scale snapping: whole steps of the snap amount
                            if snap.4 {
                                let s = snap.5.max(0.001);
                                f = ((f / s).round() * s).max(s);
                            }
                            o.size = o0.size * f;
                            o.length = o0.length * f;
                        }
                    }
                    if o.kind.is_force() {
                        self.dirty = true;
                    }
                }
            } else {
                self.gizmo_drag = None;
                ui.active = 0;
            }
            return;
        }
        if over && ui.active == 0 && self.vp_drag == VpDrag::None {
            // Front, Back, Left, Right, Top and Bottom only zoom and pan unless Preferences >
            // Viewport > Rotate fixed views is on (picture 27)
            let fixed = self.cam.view != 0 && !self.prefs.rotate_fixed_views;
            let orbit = if fixed { VpDrag::Pan } else { VpDrag::Orbit };
            // Preferences > Tree Window > Control scheme (picture 45)
            let start = match self.prefs.nav_scheme {
                // Autodesk Maya: Alt + left orbits, Alt + middle pans, Alt + right zooms
                1 => {
                    if !inp.alt {
                        None
                    } else if inp.l_pressed {
                        Some((orbit, 0))
                    } else if inp.m_pressed {
                        Some((VpDrag::Pan, 1))
                    } else if inp.r_pressed {
                        Some((VpDrag::Zoom, 2))
                    } else {
                        None
                    }
                }
                // Autodesk 3ds Max: middle pans, Alt + middle orbits, Ctrl + Alt + middle zooms
                2 => {
                    if inp.m_pressed {
                        Some((if inp.alt && inp.ctrl { VpDrag::Zoom } else if inp.alt { orbit } else { VpDrag::Pan }, 1))
                    } else {
                        None
                    }
                }
                _ => {
                    if inp.l_pressed {
                        Some((if inp.shift || inp.alt && inp.ctrl { VpDrag::Pan } else { orbit }, 0))
                    } else if inp.m_pressed {
                        Some((VpDrag::Pan, 1))
                    } else if inp.r_pressed {
                        Some((VpDrag::Zoom, 2))
                    } else {
                        None
                    }
                }
            };
            if let Some((drag, btn)) = start {
                self.vp_drag = drag;
                self.vp_btn = btn;
                self.last_mouse = m;
                if btn == 0 {
                    ui.active = hid("viewport");
                }
            }
        }
        // the button the drag started with is still held
        let held = match self.vp_btn {
            0 => inp.l_down,
            1 => inp.m_down,
            _ => inp.r_down,
        };
        let d = m - self.last_mouse;
        match self.vp_drag {
            VpDrag::Orbit => {
                if inp.l_down && self.light_drag_mode {
                    // the light controller is on (picture 42): a drag moves the sun instead
                    self.light.azimuth = (self.light.azimuth - d.x * 0.4 + 540.0).rem_euclid(360.0) - 180.0;
                    self.light.elevation = (self.light.elevation + d.y * 0.3).clamp(5.0, 90.0);
                } else if held {
                    self.cam.orbit(d.x * rs, d.y * rs);
                } else {
                    self.vp_drag = VpDrag::None;
                }
            }
            VpDrag::Pan => {
                if held {
                    self.cam.pan(-d.x * ps, d.y * ps, r.h);
                } else {
                    self.vp_drag = VpDrag::None;
                }
            }
            VpDrag::Zoom => {
                if held {
                    self.cam.zoom(-d.y * 0.03 * zs);
                } else {
                    self.vp_drag = VpDrag::None;
                }
            }
            VpDrag::None => {}
        }
        self.last_mouse = m;
    }

    pub fn draw_viewport_bg(&self, r: Rect) {
        // the original's viewport (picture 42): sky above the horizon, ground colour below it
        if self.prefs.bg_style == 1 {
            let (sky, hor, gnd) = (self.prefs.color("sky_top"), self.prefs.color("sky_horizon"), self.prefs.color("sky_ground"));
            let p = self.cam.pitch;
            let hy = if self.cam.ortho {
                if p.abs() < 0.02 { r.y + r.h * 0.5 } else if p > 0.0 { r.y - 1.0 } else { r.y + r.h + 1.0 }
            } else {
                r.y + r.h * 0.5 - r.h * 0.5 * p.tan() / (self.cam.fov.to_radians() * 0.5).tan()
            };
            let steps = 48;
            for i in 0..steps {
                let y0 = r.y + r.h * i as f32 / steps as f32;
                let ym = y0 + r.h / steps as f32 * 0.5;
                let c = if ym < hy {
                    let t = ((hy - ym) / (r.h * 0.7)).clamp(0.0, 1.0).sqrt();
                    Color::new(hor.r + (sky.r - hor.r) * t, hor.g + (sky.g - hor.g) * t, hor.b + (sky.b - hor.b) * t, 1.0)
                } else {
                    let t = ((ym - hy) / (r.h * 0.8)).clamp(0.0, 1.0);
                    Color::new(gnd.r * (1.0 - 0.12 * t), gnd.g * (1.0 - 0.12 * t), gnd.b * (1.0 - 0.12 * t), 1.0)
                };
                draw_rectangle(r.x, y0, r.w, r.h / steps as f32 + 1.0, c);
            }
            // a crisp horizon line
            if hy > r.y && hy < r.y + r.h {
                draw_rectangle(r.x, hy - 1.0, r.w, 2.0, Color::new(hor.r, hor.g, hor.b, 1.0));
            }
            return;
        }
        let top = self.prefs.color("bg_top");
        let bot = self.prefs.color("bg_bottom");
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
        // the ground under the tree (Preferences > Viewport > Ground; picture 42)
        if self.prefs.show_ground {
            let ext = (self.bounds.1 - self.bounds.0).length();
            // below the grid by a share of the camera distance, so they never fight (picture 43)
            render::draw_ground((ext * 0.9).clamp(12.0, 400.0), -(self.cam.dist * 0.004).max(0.02), self.prefs.color("ground"));
        }
        // the world origin (picture 46): a filled disc in its own colour, lit up while the Tree
        // node is selected
        {
            let tree_sel = matches!(self.sel, Sel::Node(id) if self.node(id).map_or(false, |n| n.kind == Kind::Tree));
            let c = self.prefs.color("origin");
            let (fill, ring, rw) = if tree_sel {
                (Color::new((c.r * 1.3).min(1.0), (c.g * 1.3).min(1.0), (c.b * 1.3).min(1.0), (c.a + 0.1).min(1.0)), self.prefs.color("picked"), 0.14)
            } else {
                (Color::new(c.r, c.g, c.b, c.a * 0.75), Color::new(c.r * 0.6, c.g * 0.6, c.b * 0.6, c.a), 0.035)
            };
            let ext = (self.bounds.1 - self.bounds.0).length();
            let rad = (ext * 0.1).clamp(2.5, 40.0);
            render::draw_disc(rad, (self.cam.dist * 0.0008).max(0.004), fill, ring, rw * rad);
        }
        if self.show_grid {
            let p = &self.prefs;
            render::draw_grid(50.0, 2.5, p.color("grid"), p.color("grid_major"), p.color("grid_x"), p.color("grid_z"));
        }
        // the toolbar's settings reach the shader through the lighting block
        self.light.wind = if self.wind_on { self.wind_strength } else { 0.0 };
        self.light.time = get_time() as f32;
        self.light.season = self.season;
        self.light.one_sided = self.one_sided;
        self.light.ao_on = self.ao_on;
        self.light.overdraw = self.light.mode == 47.0;
        self.light.no_alpha_test = !self.prefs.alpha_test;
        self.light.wind_dir = self.wind_dir;
        self.light.shading = if self.shading == 1 && self.matcap_texture { 3.0 } else { self.shading as f32 };
        // Selection > Solid fill (or Both): its colour; Outline only: no fill
        let mut fill = self.prefs.sel_fill;
        if self.prefs.sel_mode == 1 {
            fill[3] = 0.0;
        }
        self.light.sel_fill = fill;
        self.light.flip_back = self.prefs.flip_back;
        let gm = if self.light.overdraw { self.gm_over.as_ref().or(self.gm.as_ref()) } else { self.gm.as_ref() };
        if let (Some(gpu), Some(gm)) = (&mut self.gpu, gm) {
            for (mi, meshes) in &mut gpu.parts {
                let tex = self.materials.get(*mi).and_then(|m| if self.prefs.textures { m.tex.clone() } else { None });
                for mesh in meshes.iter_mut() {
                    // Preferences > Viewport > Texture mapping off, or no material: plain colour
                    mesh.texture = tex.clone();
                }
            }
            // Preferences > Viewport > Wireframe and Render > Wireframe: see-through lines only
            // (the original's "Enables wireframe (lines only) rendering"); WireShaded: the
            // standard shading with every edge on top (picture 24)
            let mode = self.light.mode;
            let wire_only = self.prefs.wireframe || mode == MODE_WIREFRAME;
            // the tree's shadow on the ground (pictures 42-45): its shape drawn white into a picture
            // (larger than the viewport by Preferences > Render > Shadow anti-aliasing samples, so
            // its edges come out smooth), then laid over the ground once as a dark see-through
            // layer before the tree is drawn - one hard shadow that can't flicker against the
            // grid or itself; the tree covers it where it stands in front
            if self.prefs.show_ground && self.light.shadow_quality > 0 && self.light.elevation > 3.0 && !wire_only {
                if let Some(sm) = &self.shadow_mat {
                    let ss = [1.0f32, 1.5, 2.0, 2.5, 3.0][self.prefs.shadow_aa.min(4)];
                    let (w, h) = (((vp.2.max(1) as f32 * ss) as u32).min(4096), ((vp.3.max(1) as f32 * ss) as u32).min(4096));
                    if self.shadow_rt.as_ref().map_or(true, |t| t.texture.width() as u32 != w || t.texture.height() as u32 != h) {
                        let t = render_target(w, h);
                        t.texture.set_filter(FilterMode::Linear);
                        self.shadow_rt = Some(t);
                    }
                    if let Some(rt) = self.shadow_rt.clone() {
                        set_camera(&self.cam.camera3d(None, r.w / r.h.max(1.0), Some(rt.clone())));
                        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
                        gl_use_material(sm);
                        sm.set_uniform("LightDir", self.light.dir());
                        sm.set_uniform("Shift", Vec2::ZERO);
                        sm.set_uniform("Lift", 0.0f32);
                        sm.set_uniform("ShadowColor", vec4(1.0, 1.0, 1.0, 1.0));
                        sm.set_uniform("GroundCol", vec4(1.0, 1.0, 1.0, 0.0));
                        sm.set_uniform("Radius", 1.0e6f32);
                        for (mi, meshes) in &gpu.parts {
                            let m = self.materials.get(*mi);
                            sm.set_uniform("Cutoff", m.filter(|m| m.opacity_from_alpha).map_or(0.0, |m| m.cutoff.max(0.01)));
                            for mesh in meshes.iter() {
                                draw_mesh(mesh);
                            }
                        }
                        unsafe { get_internal_gl() }.flush();
                        gl_use_default_material();
                        // laid over the viewport (screen units), then back to the 3D camera
                        set_default_camera();
                        let f = scale / screen_dpi_scale().max(0.5);
                        let dark = (self.light.ground_weight * 1.6).clamp(0.0, 0.9);
                        draw_texture_ex(&rt.texture, r.x * f, r.y * f, Color::new(0.02, 0.02, 0.03, dark), DrawTextureParams { dest_size: Some(vec2(r.w * f, r.h * f)), flip_y: true, ..Default::default() });
                        unsafe { get_internal_gl() }.flush();
                        set_camera(&cam3);
                    }
                }
            }
            if !wire_only {
                let mut light = self.light.clone();
                if mode == MODE_WIRESHADED {
                    light.mode = 0.0;
                }
                draw_gpu(gpu, &self.materials, gm, &light, self.cam.position());
            }
            // Selection > Outline (picture 30): the selected generator's shape into a picture the
            // size of the viewport; draw_selection_outline rings it afterwards
            self.sel_drawn = false;
            if self.prefs.sel_mode != 0 && self.highlight && !self.sel_hidden && matches!(self.sel, Sel::Node(_)) {
                if let Some(mm) = &self.mask_mat {
                    let (w, h) = (vp.2.max(1) as u32, vp.3.max(1) as u32);
                    if self.sel_rt.as_ref().map_or(true, |t| t.texture.width() as u32 != w || t.texture.height() as u32 != h) {
                        let t = render_target(w, h);
                        t.texture.set_filter(FilterMode::Nearest);
                        self.sel_rt = Some(t);
                    }
                    if let Some(rt) = self.sel_rt.clone() {
                        set_camera(&self.cam.camera3d(None, r.w / r.h.max(1.0), Some(rt)));
                        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
                        gl_use_material(mm);
                        mm.set_uniform("Time", self.light.time);
                        mm.set_uniform("Wind", self.light.wind);
                        let wa = self.light.wind_dir.to_radians();
                        mm.set_uniform("WindDir", vec2(wa.cos(), wa.sin()));
                        for (mi, meshes) in &gpu.parts {
                            let m = self.materials.get(*mi);
                            let leaf = m.map_or(false, |m| m.opacity_from_alpha);
                            mm.set_uniform("IsLeaf", if leaf { 1.0f32 } else { 0.0 });
                            mm.set_uniform("Cutoff", m.filter(|m| m.opacity_from_alpha).map_or(0.0, |m| m.cutoff.max(0.01)));
                            for mesh in meshes.iter() {
                                draw_mesh(mesh);
                            }
                            unsafe { get_internal_gl() }.flush();
                        }
                        gl_use_default_material();
                        set_camera(&cam3);
                        self.sel_drawn = true;
                    }
                }
            }
            // Scribed: the triangles' edges over the shaded tree
            if mode == 1.0 || mode == MODE_WIRESHADED || wire_only {
                let c = if wire_only {
                    self.prefs.color("wire_only")
                } else if mode == MODE_WIRESHADED {
                    self.prefs.color("wire_shaded")
                } else {
                    self.prefs.color("scribed")
                };
                if mode == 1.0 && !wire_only {
                    // Scribed: thin lines as before
                    let mut n = 0;
                    'all: for (_, meshes) in &gpu.parts {
                        for mesh in meshes {
                            for t in mesh.indices.chunks(3) {
                                if t.len() < 3 {
                                    continue;
                                }
                                let p = |i: u16| mesh.vertices[i as usize].position;
                                draw_line_3d(p(t[0]), p(t[1]), c);
                                draw_line_3d(p(t[1]), p(t[2]), c);
                                draw_line_3d(p(t[2]), p(t[0]), c);
                                n += 1;
                                if n > 60000 {
                                    break 'all;
                                }
                            }
                        }
                    }
                } else {
                    // WireShaded / Wireframe (picture 38): lines of Preferences > Viewport > Wire
                    // thickness, each a thin strip facing the camera; the selected generator's
                    // (its vertices marked by alpha, gen.rs) brighter by Selected wire brightness
                    let k = self.prefs.wire_sel_boost;
                    let csel = Color::new(c.r + (1.0 - c.r) * k, c.g + (1.0 - c.g) * k, c.b + (1.0 - c.b) * k, (c.a + (1.0 - c.a) * k).min(1.0));
                    let px = if self.cam.ortho { self.cam.view_height() / vp.3.max(1) as f32 } else { 2.0 * (self.cam.fov.to_radians() * 0.5).tan() / vp.3.max(1) as f32 };
                    draw_thick_wires(&gpu.parts, self.cam.position(), self.cam.ortho, px * self.prefs.wire_px.max(0.5), c, csel);
                }
            }
            // Show > Normals: a short line along each vertex normal
            if self.show_normals {
                let c = self.prefs.color("normals");
                let len = ((self.bounds.1 - self.bounds.0).length() * 0.01).max(0.05);
                let mut n = 0;
                'nrm: for (_, meshes) in &gpu.parts {
                    for mesh in meshes {
                        for v in &mesh.vertices {
                            let nn = vec3(v.normal.x, v.normal.y, v.normal.z);
                            draw_line_3d(v.position, v.position + nn * len, c);
                            n += 1;
                            if n > 40000 {
                                break 'nrm;
                            }
                        }
                    }
                }
            }
        }
        // Skin:Type "Spine only" (picture 39): each spine a yellow line with a mark at every
        // segment, brighter when its generator is selected
        if let Some(geo) = self.geo.as_ref().filter(|g| !g.spine_lines.is_empty()) {
            let px = if self.cam.ortho { self.cam.view_height() / vp.3.max(1) as f32 } else { 2.0 * (self.cam.fov.to_radians() * 0.5).tan() / vp.3.max(1) as f32 };
            let sel = if let Sel::Node(id) = self.sel { Some(id) } else { None };
            draw_spine_lines(&geo.spine_lines, self.cam.position(), self.cam.ortho, px, sel);
        }
        // Show > Bones: every branch's spine
        if self.show_bones {
            if let Some(geo) = &self.geo {
                // drawn over the bark: what's drawn so far goes out, then the depth is cleared
                unsafe {
                    let mut gl = get_internal_gl();
                    gl.flush();
                    gl.quad_context.clear(None, Some(1.0), None);
                }
                let c = self.prefs.color("bones");
                for b in &geo.bones {
                    for w in b.windows(2) {
                        draw_line_3d(w[0], w[1], c);
                    }
                }
            }
        }
        self.draw_scene_objects();
        self.draw_edit_3d();
        set_default_camera();
    }

    /// Forces and collision objects in the viewport (when shown), the picked one highlighted.
    fn draw_scene_objects(&self) {
        for (k, o) in self.scene.iter().enumerate() {
            if (o.kind.is_force() && !self.show_forces) || (!o.kind.is_force() && !self.show_coll) {
                continue;
            }
            let picked = self.scene_sel == Some(k);
            let c = if picked { self.prefs.color("picked") } else if o.kind.is_force() { self.prefs.color("forces") } else { self.prefs.color("collision") };
            match o.kind {
                SceneKind::Direction => {
                    // an arrow along the force, longer with strength
                    let d = o.dir.normalize_or_zero();
                    let end = o.pos + d * (4.0 + o.size * 4.0);
                    draw_line_3d(o.pos, end, c);
                    let side = any_perp3(d);
                    let up = d.cross(side);
                    for s in [side, -side, up, -up] {
                        draw_line_3d(end, end - d * 1.2 + s * 0.6, c);
                    }
                    draw_sphere_wires(o.pos, 0.4, None, c);
                }
                SceneKind::Magnet => {
                    draw_sphere_wires(o.pos, 0.6 + o.size.abs() * 0.4, None, c);
                    for a in [Vec3::X, Vec3::Y, Vec3::Z] {
                        draw_line_3d(o.pos - a * 1.5, o.pos + a * 1.5, c);
                    }
                }
                SceneKind::Sphere => draw_sphere_wires(o.pos, o.size, None, c),
                SceneKind::Capsule => {
                    let d = o.dir.normalize_or_zero();
                    let (a, b) = (o.pos - d * o.length * 0.5, o.pos + d * o.length * 0.5);
                    draw_sphere_wires(a, o.size, None, c);
                    draw_sphere_wires(b, o.size, None, c);
                    let side = any_perp3(d);
                    let up = d.cross(side);
                    for s in [side, -side, up, -up] {
                        draw_line_3d(a + s * o.size, b + s * o.size, c);
                    }
                }
            }
        }
    }

    pub fn draw_viewport_overlay(&mut self, ui: &mut Ui, r: Rect) {
        // the toolbar across the top (picture-18), then the view menu below it
        let th = self.viewport_toolbar(ui, r);
        let vb = Rect::new(r.x + 8.0, r.y + th + 6.0, 160.0, 22.0);
        self.vp_ui = vec![Rect::new(r.x, r.y, r.w, th), vb];
        let label = if self.cam.ortho { format!("{} (ortho)", VIEWS[self.cam.view.min(VIEWS.len() - 1)]) } else { VIEWS[self.cam.view.min(VIEWS.len() - 1)].to_string() };
        let clicked = ui.button_ex("vp.view", Rect::new(vb.x, vb.y, vb.w, vb.h), &label, ui.popup_open_for("vp.view"), true);
        draw_triangle(vec2(vb.x + vb.w - 14.0, vb.y + 9.0), vec2(vb.x + vb.w - 6.0, vb.y + 9.0), vec2(vb.x + vb.w - 10.0, vb.y + 14.0), C_TEXT_DIM);
        if clicked {
            let mut items: Vec<MenuItem> = VIEWS.iter().enumerate().map(|(i, v)| MenuItem::check(v, i == self.cam.view)).collect();
            items.push(MenuItem::sep());
            items.push(MenuItem::check("Orthographic", self.cam.ortho));
            ui.open_menu(hid("vp.view"), vb, items);
        }
        if let Some(p) = ui.take_menu_result("vp.view") {
            match p.first().copied() {
                Some(i) if i < VIEWS.len() => self.cam.set_view(i),
                Some(_) => self.cam.ortho = !self.cam.ortho,
                None => {}
            }
        }
        let s = format!(
            "Triangles: {}   Vertices: {}   Branches: {}   Leaves: {}",
            self.geo_stats.0, self.geo_stats.1, self.geo_stats.2, self.geo_stats.3
        );
        ui.record("stat", Rect::new(r.x + 10.0, r.y + r.h - 22.0, 300.0, 16.0));
        if self.prefs.labels {
            // dark on the light sky background (picture 42)
            let lc = if self.prefs.bg_style == 1 { Color::new(0.12, 0.13, 0.15, 0.85) } else { self.prefs.color("labels") };
            ui.text(&s, r.x + 10.0, r.y + r.h - 22.0, 12.0, lc);
        }
        if !self.status.is_empty() && get_time() - self.status_t < 4.0 {
            let w = ui.text_w(&self.status, 12.5) + 20.0;
            let br = Rect::new(r.x + (r.w - w) * 0.5, r.y + r.h - 46.0, w, 24.0);
            draw_rectangle(br.x, br.y, br.w, br.h, Color::new(0.0, 0.0, 0.0, 0.55));
            let st = self.status.clone();
            ui.text_center(&st, br, 12.5, WHITE);
        }
        // top right (picture 27): the navigation gizmo, then the shading button and its drop-down
        // over it (the gizmo hides while the drop-down is open)
        let gk = self.prefs.gizmo_size.clamp(0.6, 2.0);
        // Show > Height indicator (picture 42), under the corner's controls
        self.draw_height_indicator(ui, r, Rect::new(r.x, vb.y + vb.h + 4.0, r.w, (r.y + r.h - vb.y - vb.h - 4.0).max(10.0)));
        if !self.shading_open && !self.sel_open {
            let gc = vec2(r.x + r.w - 62.0 * gk, vb.y + 34.0 + 54.0 * gk);
            if let Some(a) = self.draw_nav_gizmo(ui, gc) {
                self.vp_ui.push(a);
            }
        }
        // the light controller and the fan at the upper left, under the view menu (picture 43)
        let lc = self.draw_light_controls(ui, vb.x + 17.0, vb.y + vb.h + 8.0);
        self.vp_ui.push(lc);
        // the "?" tips card at the bottom left (picture 43)
        let hc = self.draw_help_card(ui, r);
        self.vp_ui.extend(hc);
        let (covered, sx) = self.draw_shading_menu(ui, r, vb.y);
        self.vp_ui.extend(covered);
        // the wire modes left of it, then the Selection bar (pictures 30, 40)
        let (wc, wx) = self.draw_wire_balls(ui, r, vb.y, sx);
        self.vp_ui.extend(wc);
        let sc = self.draw_selection_menu(ui, r, vb.y, wx);
        self.vp_ui.extend(sc);
    }

    /// The original's viewport toolbar (fn 0x1405325b0; its menus fn 0x140537430): groups View
    /// (Render, Show, Zoom), Scene (Forces, Collision Objects, Wind, Light), Edit (Generators /
    /// Nodes, Season, Add, Visibility, Gizmo) and Post (AO, Collision, Export, collapse), each
    /// button a menu or a command with the original's help as its tooltip. Groups wrap onto
    /// another row when the viewport is narrow. Returns the height used.
    fn viewport_toolbar(&mut self, ui: &mut Ui, r: Rect) -> f32 {
        // (button label, key, tooltip)
        type B = (&'static str, &'static str, &'static str);
        let groups: [(&str, &[B]); 4] = [
            ("View", &[
                ("Render", "vt.render", "Specify the style in which the scene is rendered."),
                ("Show", "vt.show", "Control which geometry types are shown in the viewport."),
                ("Zoom", "vt.zoom", "Zoom the camera view to show the selection. If nothing is selected, zoom out far enough to show the whole model. (Z)"),
            ]),
            ("Scene", &[
                ("Forces", "vt.forces", "Access force functions: modeling tools that manipulate growth in world space. (6 toggles forces)"),
                ("Collision Objects", "vt.coll", "Access collision object functions: simple primitives for collision at run time. (7 toggles them)"),
                ("Wind", "vt.wind", "Access high level wind functions. (8 toggles wind)"),
                ("Light", "vt.light", "Access light properties."),
            ]),
            ("Edit", &[
                ("Generators", "vt.gens", "All property edits apply to the selected generator(s). (Tab)"),
                ("Nodes", "vt.nodes", "All property edits apply to the selected node(s), as offsets for fine tuning. (Tab)"),
                ("Season", "vt.season", "Set the season of the current model. (Alt+S)"),
                ("Add", "vt.add", "Add geometry to the model based on a set of predefined templates."),
                ("Visibility", "vt.vis", "Control how much of the model is visible, to reduce computing and rendering temporarily."),
                ("Gizmo", "vt.gizmo", "Set options for the transform gizmo."),
            ]),
            ("Post", &[
                ("AO", "vt.ao", "Compute ambient occlusion on the model. (O)"),
                ("Collision", "vt.lcoll", "Eliminate colliding leaf meshes (High, Medium, Low or None)."),
                ("Export", "vt.export", "Export the model as a mesh."),
                ("<", "vt.collapse", "Collapse or expand the toolbar."),
            ]),
        ];
        let row_h = 46.0;
        let m = ui.mouse();
        let mut x = r.x + 6.0;
        let mut y = r.y + 4.0;
        let mut rows = 1.0;
        // collapsed: only the expand arrow
        if self.toolbar_collapsed {
            let b = Rect::new(r.x + 6.0, r.y + 4.0, 26.0, 22.0);
            draw_rectangle(r.x, r.y, r.w, 30.0, Color::new(0.16, 0.16, 0.17, 0.92));
            if ui.tool_button("vt.expand", b, ">", false) {
                self.toolbar_collapsed = false;
            }
            if b.contains(m) {
                ui.tooltip = Some("Expand the toolbar".into());
            }
            return 30.0;
        }
        // lay the groups out first (wrapping), then draw the strip and the buttons
        let mut placed: Vec<(&str, Rect, Vec<(B, Rect)>)> = vec![];
        for (title, buttons) in groups.iter() {
            let ws: Vec<f32> = buttons.iter().map(|b| ui.text_w(b.0, 12.5) + if b.0 == "<" { 22.0 } else { 40.0 }).collect();
            let gw = ws.iter().sum::<f32>() + 2.0 * (ws.len() as f32 - 1.0) + 8.0;
            if x + gw > r.x + r.w - 4.0 && x > r.x + 6.0 {
                x = r.x + 6.0;
                y += row_h;
                rows += 1.0;
            }
            let g = Rect::new(x, y, gw, row_h - 6.0);
            let mut bx = x + 4.0;
            let mut bs = vec![];
            for (b, w) in buttons.iter().zip(ws) {
                bs.push((*b, Rect::new(bx, y + 15.0, w, 24.0)));
                bx += w + 3.0;
            }
            placed.push((title, g, bs));
            x += gw + 8.0;
        }
        let h = rows * row_h + 2.0;
        draw_rectangle(r.x, r.y, r.w, h, Color::new(0.16, 0.16, 0.17, 0.92));
        draw_line(r.x, r.y + h, r.x + r.w, r.y + h, 2.0, Color::new(0.35, 0.55, 0.85, 0.8));
        let mut clicked: Option<(&str, Rect)> = None;
        for (title, g, bs) in &placed {
            draw_round_rect(*g, 6.0, Color::new(0.195, 0.2, 0.21, 1.0));
            ui.text(title, g.x + 6.0, g.y + 1.0, 11.0, C_TEXT_DIM);
            for ((label, key, tip), br) in bs {
                let on = match *key {
                    "vt.gens" => true,
                    "vt.ao" => self.ao_on,
                    k => ui.popup_open_for(k),
                };
                // rounded button: a soft fill, brighter when hovered, accent when on
                ui.record(&format!("btn:{}", key), *br);
                let hot = br.contains(m) && !ui.blocked;
                let fill = if on {
                    Color::new(0.30, 0.47, 0.78, 1.0)
                } else if hot {
                    Color::new(0.33, 0.34, 0.37, 1.0)
                } else {
                    Color::new(0.24, 0.25, 0.27, 1.0)
                };
                draw_round_rect(*br, 6.0, fill);
                draw_round_rect_lines(*br, 6.0, 1.0, Color::new(1.0, 1.0, 1.0, if hot { 0.22 } else { 0.08 }));
                if *label == "<" {
                    ui.text_center("<", *br, 13.0, C_TEXT);
                } else {
                    toolbar_icon(key, vec2(br.x + 13.0, br.y + br.h * 0.5));
                    ui.text(label, br.x + 26.0, br.y + 5.0, 12.5, if on { WHITE } else { C_TEXT });
                }
                if hot && ui.inp.l_pressed && ui.popup.is_none() {
                    clicked = Some((key, *br));
                }
                if br.contains(m) && ui.popup.is_none() {
                    ui.tooltip = Some(tip.to_string());
                }
            }
        }
        if let Some((key, br)) = clicked {
            let tree = self.toolbar_menu(key);
            if tree.is_empty() {
                match key {
                    "vt.gens" => {}
                    "vt.nodes" => self.set_status("Node editing (offsets on single branches and leaves) isn't built yet; edits apply to generators"),
                    "vt.ao" => {
                        // AO (O): ambient occlusion from how deep each point sits in the crown
                        self.ao_on = !self.ao_on;
                        self.set_status(if self.ao_on { "Ambient occlusion on" } else { "Ambient occlusion off" });
                    }
                    "vt.export" => self.export_obj(),
                    "vt.collapse" => self.toolbar_collapsed = true,
                    _ => {}
                }
            } else {
                ui.open_menu(hid(key), br, tree);
            }
        }
        // what was chosen in an open menu
        for key in ["vt.render", "vt.show", "vt.zoom", "vt.forces", "vt.coll", "vt.wind", "vt.light", "vt.add", "vt.vis", "vt.gizmo", "vt.lcoll", "vt.season"] {
            let Some(path) = ui.take_menu_result(key) else { continue };
            self.toolbar_command(key, &path);
        }
        h
    }

    /// The Render menu: the original's render_modes folder, ordered and named like its templates
    /// (leading underscores first, then removed).
    fn render_menu(cur: &str) -> Vec<MenuItem> {
        // each mode's number in the shader (render.rs) follows its place: top 0-5, Lighting
        // 10-15, Material 20-28, Vertex Colors 30-33, Other 40-49
        let list = |group: &str, v: &[&str]| {
            v.iter()
                .map(|s| {
                    let full = if group.is_empty() { s.to_string() } else { format!("{} > {}", group, s) };
                    MenuItem::check(s, full == cur)
                })
                .collect::<Vec<_>>()
        };
        // WireShaded and Wireframe are the person's additions (picture 24), Blender-like
        let mut top = list("", &["Standard", "Scribed", "WireShaded", "Wireframe", "AO", "Albedo Check", "Subsurface Check"]);
        top.push(MenuItem::sub("Lighting", list("Lighting", &RENDER_LIGHTING)));
        top.extend(list("", &["Lightmap"]));
        top.push(MenuItem::sub("Material", list("Material", &RENDER_MATERIAL)));
        top.push(MenuItem::sub("Vertex Colors", list("Vertex Colors", &RENDER_VCOL)));
        top.push(MenuItem::sub("Other", list("Other", &RENDER_OTHER)));
        top
    }

    /// The shader's number for a render mode chosen from render_menu (by its path).
    fn render_mode_number(path: &[usize]) -> Option<f32> {
        let a = *path.first()?;
        let b = path.get(1).copied().unwrap_or(0) as f32;
        Some(match a {
            0 | 1 => a as f32,
            2 => MODE_WIRESHADED,
            3 => MODE_WIREFRAME,
            4..=6 => a as f32 - 2.0,
            7 => 10.0 + b,
            8 => 5.0,
            9 => 20.0 + b,
            10 => 30.0 + b,
            11 => 40.0 + b,
            _ => return None,
        })
    }

    /// The menu a toolbar button opens (empty for buttons that act straight away).
    fn toolbar_menu(&self, key: &str) -> Vec<MenuItem> {
        let leaf = MenuItem::leaf;
        let chk = MenuItem::check;
        let objects = |forces: bool| -> Vec<MenuItem> {
            self.scene
                .iter()
                .enumerate()
                .filter(|(_, o)| o.kind.is_force() == forces)
                .map(|(k, o)| chk(&o.name, self.scene_sel == Some(k)))
                .collect()
        };
        match key {
            "vt.render" => Self::render_menu(&self.render_mode),
            "vt.show" => vec![
                chk("Leaves", self.show_leaves).hot("1"),
                chk("Fronds", self.show_other[0]).hot("2"),
                chk("Branches", self.show_branches).hot("3"),
                chk("Zones", self.show_other[1]).hot("4"),
                chk("Targets", self.show_other[2]).hot("5"),
                chk("Proxies", self.show_other[3]).hot("9"),
                MenuItem::sep(),
                chk("Bones", self.show_bones),
                chk("Normals", self.show_normals),
                chk("Disable two sided rendering", self.one_sided),
                MenuItem::sep(),
                chk("Grid", self.show_grid),
                // picture 42
                chk("Ground", self.prefs.show_ground),
                chk("Height indicator", self.show_height),
            ],
            "vt.zoom" => vec![
                leaf("Selected").hot("Z"),
                leaf("Selected (all views)"),
                leaf("All"),
                leaf("All (all views)"),
                MenuItem::sep(),
                leaf("Copy camera"),
                leaf("Paste camera"),
                leaf("Reset camera"),
            ],
            "vt.forces" => {
                let mut v = vec![MenuItem::sub("Add force", vec![leaf("Direction"), leaf("Magnet"), leaf("Mesh")]), leaf("Delete selected"), chk("Show forces", self.show_forces).hot("6")];
                let list = objects(true);
                if !list.is_empty() {
                    v.push(MenuItem::sep());
                    v.extend(list);
                }
                v
            }
            "vt.coll" => {
                let mut v = vec![MenuItem::sub("Add collision object", vec![leaf("Sphere"), leaf("Capsule")]), leaf("Delete selected"), chk("Show collision objects", self.show_coll).hot("7")];
                let list = objects(false);
                if !list.is_empty() {
                    v.push(MenuItem::sep());
                    v.extend(list);
                }
                v
            }
            "vt.wind" => vec![
                chk("Wind", self.wind_on).hot("8"),
                MenuItem::sub("Strength", WIND_LEVELS.iter().map(|(n, s)| chk(n, (self.wind_strength - s).abs() < 1e-3)).collect()),
            ],
            "vt.light" => vec![chk("Lit", !self.light.unlit), MenuItem::sep(), leaf("Properties...")],
            "vt.add" => Self::gen_add_menu(),
            "vt.vis" => vec![leaf("Toggle selected").hot("H"), chk("Toggle isolation", self.isolation.is_some()).hot("I"), leaf("Show all")],
            "vt.gizmo" => vec![chk("Translate", self.gizmo_mode == 0), chk("Rotate", self.gizmo_mode == 1), chk("Scale", self.gizmo_mode == 2)],
            "vt.lcoll" => ["None", "Low", "Medium", "High"].iter().enumerate().map(|(i, n)| chk(n, self.leaf_collision as usize == i)).rev().collect(),
            "vt.season" => SEASONS.iter().map(|(n, s)| chk(n, (self.season - s).abs() < 1e-3)).collect::<Vec<_>>(),
            _ => vec![],
        }
    }

    /// Write the current tree as a Wavefront OBJ (positions, texture coordinates, normals; one
    /// group per material) to a file the person picks.
    pub(crate) fn export_obj(&mut self) {
        let Some(gpu) = &self.gpu else { return };
        let Some(path) = crate::prefs::file_dialog().set_title("Export tree").add_filter("Wavefront OBJ", &["obj"]).set_file_name("tree.obj").save_file() else { return };
        let s = self.obj_text(gpu);
        match std::fs::write(&path, s) {
            Ok(()) => self.set_status(&format!("Exported {}", path.display())),
            Err(e) => self.set_status(&format!("Export failed: {}", e)),
        }
    }

    /// Geometry as Wavefront OBJ text (one group per material).
    pub(crate) fn obj_text(&self, gpu: &GpuTree) -> String {
        use std::fmt::Write as _;
        let mut s = String::from("# TreeCraft export\n");
        let mut base = 1usize;
        for (mi, meshes) in &gpu.parts {
            let name = self.materials.get(*mi).map(|m| m.name.replace(' ', "_")).unwrap_or_else(|| format!("material_{}", mi));
            let _ = writeln!(s, "g {}\nusemtl {}", name, name);
            for mesh in meshes {
                for v in &mesh.vertices {
                    let _ = writeln!(s, "v {} {} {}", v.position.x, v.position.y, v.position.z);
                }
                for v in &mesh.vertices {
                    let _ = writeln!(s, "vt {} {}", v.uv.x, 1.0 - v.uv.y);
                }
                for v in &mesh.vertices {
                    let _ = writeln!(s, "vn {} {} {}", v.normal.x, v.normal.y, v.normal.z);
                }
                for t in mesh.indices.chunks(3) {
                    if t.len() == 3 {
                        let (a, b, c) = (base + t[0] as usize, base + t[1] as usize, base + t[2] as usize);
                        let _ = writeln!(s, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}");
                    }
                }
                base += mesh.vertices.len();
            }
        }
        s
    }

    /// The generation inputs for this tree (Show settings, forces, leaf collision); `only` limits
    /// the drawn geometry to those generators.
    pub(crate) fn generate_with(&self, only: Option<std::collections::HashSet<u32>>) -> gen::TreeGeo {
        let selected = if let Sel::Node(id) = self.sel { Some(id) } else { None };
        let mut shown = self.nodes.clone();
        for n in &mut shown {
            if !self.show_leaves && n.kind == Kind::Leaf {
                n.hidden = true;
            }
        }
        let forces: Vec<SceneObj> = self.scene.iter().filter(|o| o.kind.is_force()).cloned().collect();
        let ctx = GenCtx {
            nodes: &shown,
            materials: &self.materials,
            meshes: &self.meshes,
            selected,
            highlight: self.highlight && only.is_none() && !self.sel_hidden,
            show_branches: self.show_branches,
            forces: &forces,
            leaf_collision: self.leaf_collision,
            only,
        };
        gen::generate(&ctx)
    }

    /// Add a force or collision object in front of the tree and pick it.
    fn add_scene_object(&mut self, kind: SceneKind) {
        let (a, b) = self.bounds;
        let h = (b.y - a.y).max(4.0);
        let n = self.scene.iter().filter(|o| o.kind == kind).count() + 1;
        let o = match kind {
            SceneKind::Direction => SceneObj { kind, name: format!("Direction {}", n), pos: vec3(b.x + 4.0, h * 0.6, 0.0), dir: vec3(-1.0, 0.0, 0.0), size: 0.5, length: 0.0 },
            SceneKind::Magnet => SceneObj { kind, name: format!("Magnet {}", n), pos: vec3(b.x + 6.0, h * 0.8, 0.0), dir: Vec3::Y, size: 0.5, length: 0.0 },
            SceneKind::Sphere => SceneObj { kind, name: format!("Sphere {}", n), pos: vec3(0.0, h * 0.5, 0.0), dir: Vec3::Y, size: h * 0.12, length: 0.0 },
            SceneKind::Capsule => SceneObj { kind, name: format!("Capsule {}", n), pos: vec3(0.0, h * 0.35, 0.0), dir: Vec3::Y, size: h * 0.05, length: h * 0.5 },
        };
        self.scene.push(o);
        self.scene_sel = Some(self.scene.len() - 1);
        if kind.is_force() {
            self.show_forces = true;
            self.dirty = true;
        } else {
            self.show_coll = true;
        }
        self.set_status("Added; drag it in the viewport with the Gizmo (Translate, Rotate or Scale)");
    }

    /// Carry out a toolbar menu choice (`path` into the menu opened for `key`).
    pub fn toolbar_command(&mut self, key: &str, path: &[usize]) {
        let pick = |tree: &[MenuItem]| -> Vec<String> {
            let mut items = tree;
            let mut trail = vec![];
            for &k in path {
                if let Some(it) = items.get(k) {
                    trail.push(it.label.clone());
                    items = &it.children;
                }
            }
            trail
        };
        let not_built = |s: &mut Self, what: &str| s.set_status(&format!("{} isn't built yet", what));
        match key {
            "vt.render" => {
                let t = pick(&Self::render_menu(""));
                if let Some(n) = Self::render_mode_number(path) {
                    self.render_mode = t.join(" > ");
                    self.light.mode = n;
                }
            }
            "vt.show" => match path.first().copied() {
                Some(0) => {
                    self.show_leaves = !self.show_leaves;
                    self.dirty = true;
                }
                Some(2) => {
                    self.show_branches = !self.show_branches;
                    self.dirty = true;
                }
                Some(i @ (1 | 3 | 4 | 5)) => {
                    // fronds, zones, targets and proxies: TreeCraft doesn't grow these kinds yet,
                    // so there's nothing of them to show or hide
                    let k = [0, 0, 0, 1, 2, 3][i];
                    self.show_other[k] = !self.show_other[k];
                    let name = ["Fronds", "Zones", "Targets", "Proxies"][k];
                    self.set_status(&format!("{} {} (this tree has none: TreeCraft doesn't grow them yet)", name, if self.show_other[k] { "shown" } else { "hidden" }));
                }
                Some(7) => self.show_bones = !self.show_bones,
                Some(8) => self.show_normals = !self.show_normals,
                Some(9) => self.one_sided = !self.one_sided,
                Some(11) => self.show_grid = !self.show_grid,
                Some(12) => {
                    self.prefs.show_ground = !self.prefs.show_ground;
                    self.prefs.save();
                }
                Some(13) => self.show_height = !self.show_height,
                _ => {}
            },
            "vt.zoom" => match path.first().copied() {
                Some(0) | Some(1) => {
                    if matches!(self.sel, Sel::Node(_)) {
                        self.set_status("Zooming to a selected generator isn't built yet: showing the whole tree");
                    }
                    self.frame_tree();
                }
                Some(2) | Some(3) => self.frame_tree(),
                Some(5) => {
                    self.cam_copy = Some(self.cam);
                    self.set_status("Camera copied");
                }
                Some(6) => match self.cam_copy {
                    Some(c) => self.cam = c,
                    None => self.set_status("Copy a camera first"),
                },
                Some(7) => {
                    self.cam = render::Cam::new();
                    self.frame_tree();
                }
                _ => {}
            },
            "vt.light" => match path.first().copied() {
                Some(0) => self.light.unlit = !self.light.unlit,
                Some(2) => {
                    // Light > Properties...: the light's settings in the Properties panel (picture 43)
                    self.select(Sel::Light);
                }
                _ => {}
            },
            "vt.add" => {
                let menu = Self::gen_add_menu();
                self.gen_add(&menu, path);
            }
            "vt.vis" => {
                let sel = if let Sel::Node(id) = self.sel { Some(id) } else { None };
                match path.first().copied() {
                    Some(0) => match sel {
                        Some(id) => {
                            if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                                n.hidden = !n.hidden;
                            }
                            self.dirty = true;
                        }
                        None => self.set_status("Select a generator first"),
                    },
                    Some(1) => {
                        if let Some(hidden) = self.isolation.take() {
                            // back to how it was
                            for n in &mut self.nodes {
                                n.hidden = hidden.contains(&n.id);
                            }
                            self.dirty = true;
                        } else if let Some(id) = sel {
                            // only the selected generator, the ones it grows on and those under it
                            let mut keep = self.descendants(id);
                            keep.push(id);
                            let mut p = self.node(id).and_then(|n| n.parent);
                            while let Some(pid) = p {
                                keep.push(pid);
                                p = self.node(pid).and_then(|n| n.parent);
                            }
                            self.isolation = Some(self.nodes.iter().filter(|n| n.hidden).map(|n| n.id).collect());
                            for n in &mut self.nodes {
                                if !keep.contains(&n.id) {
                                    n.hidden = true;
                                }
                            }
                            self.dirty = true;
                        } else {
                            self.set_status("Select a generator first");
                        }
                    }
                    Some(2) => {
                        for n in &mut self.nodes {
                            n.hidden = false;
                        }
                        self.isolation = None;
                        self.dirty = true;
                    }
                    _ => {}
                }
            }
            "vt.forces" | "vt.coll" => {
                let forces = key == "vt.forces";
                let t = pick(&self.toolbar_menu(key));
                match (path.first().copied(), t.last().map(|s| s.as_str())) {
                    (Some(0), Some("Direction")) => self.add_scene_object(SceneKind::Direction),
                    (Some(0), Some("Magnet")) => self.add_scene_object(SceneKind::Magnet),
                    (Some(0), Some("Mesh")) => not_built(self, "A mesh force (growth around a 3D mesh)"),
                    (Some(0), Some("Sphere")) => self.add_scene_object(SceneKind::Sphere),
                    (Some(0), Some("Capsule")) => self.add_scene_object(SceneKind::Capsule),
                    (Some(1), _) => match self.scene_sel.filter(|&k| self.scene.get(k).map(|o| o.kind.is_force() == forces).unwrap_or(false)) {
                        Some(k) => {
                            self.scene.remove(k);
                            self.scene_sel = None;
                            self.dirty |= forces;
                        }
                        None => self.set_status(if forces { "Pick a force first (from this menu)" } else { "Pick a collision object first (from this menu)" }),
                    },
                    (Some(2), _) => {
                        if forces {
                            self.show_forces = !self.show_forces;
                        } else {
                            self.show_coll = !self.show_coll;
                        }
                    }
                    (Some(_), Some(name)) => {
                        // pick an object by its name
                        let name = name.to_string();
                        self.scene_sel = self.scene.iter().position(|o| o.name == name);
                    }
                    _ => {}
                }
            }
            "vt.wind" => match path.first().copied() {
                Some(0) => self.wind_on = !self.wind_on,
                Some(1) => {
                    if let Some((_, s)) = path.get(1).and_then(|&i| WIND_LEVELS.get(i)) {
                        self.wind_strength = *s;
                        self.wind_on = true;
                    }
                }
                _ => {}
            },
            "vt.gizmo" => {
                if let Some(&i) = path.first() {
                    self.gizmo_mode = i as u8;
                    self.set_status("Drag a force or collision object in the viewport to change it");
                }
            }
            "vt.lcoll" => {
                if let Some(&i) = path.first() {
                    self.leaf_collision = (3 - i.min(3)) as u8;
                    self.dirty = true;
                }
            }
            "vt.season" => {
                if let Some((_, s)) = path.first().and_then(|&i| SEASONS.get(i)) {
                    self.season = *s;
                }
            }
            _ => {}
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

    fn toolbar(&mut self, ui: &mut Ui, tb: Rect, key: &str, add_tree: Vec<MenuItem>, act_items: &[&str], randomize: bool) -> (Option<Vec<usize>>, Option<usize>, bool) {
        let addk = format!("{}.add", key);
        let actk = format!("{}.actions", key);
        let ar = Rect::new(tb.x + 6.0, tb.y + 4.0, 52.0, 22.0);
        if ui.tool_button(&addk, ar, "+ Add", ui.popup_open_for(&addk)) {
            ui.open_menu(hid(&addk), ar, add_tree);
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
        (ui.take_menu_result(&addk), ui.take_popup_result(&actk), rnd)
    }

    /// The Generation tab's Add menu, laid out like the original's (fn 0x140228270): the
    /// templates folder's files and folders sorted by name (leading underscores first), names with
    /// leading underscores removed and the rest turned into spaces, folders as submenus, then
    /// "From file...", a line and "Default generator".
    fn gen_add_menu() -> Vec<MenuItem> {
        let l = MenuItem::leaf;
        let s = MenuItem::sub;
        let mut top: Vec<MenuItem> = GEN_TEMPLATES.iter().map(|t| l(t)).collect();
        top.push(s("Decorations", ["Cavity", "Gash", "Knot", "Lump", "Peeling Bark", "Roots", "Shell"].iter().map(|t| l(t)).collect()));
        top.push(s(
            "Hand Drawn",
            vec![
                l("Trunk"),
                l("Branches"),
                s("Legacy", ["SubD Setup", "SubD Trunk", "SubD Branch", "Branch", "Palm Frond", "Trunk"].iter().map(|t| l(t)).collect()),
            ],
        ));
        top.push(s(
            "Photogrammetry",
            [
                "Texture Blended Extension",
                "Vertex Blended Extension",
                "Click Placed Branches",
                "Click Placed Custom Mesh",
                "Mesh",
                "Cutting Stitch",
                "Place a spine in a mesh",
                "Place 3D Anchors",
                "Procedural Branch Off Mesh",
                "Single Target",
            ]
            .iter()
            .map(|t| l(t))
            .collect(),
        ));
        top.push(s("Zones", vec![l("Terrain"), l("Zone")]));
        top.push(l("From file..."));
        top.push(MenuItem::sep());
        top.push(s("Default generator", vec![l("Trunk"), l("Branch"), l("Leaf Mesh")]));
        top
    }

    /// Act on an entry chosen from the Add menu (Generation tab or viewport toolbar).
    fn gen_add(&mut self, menu: &[MenuItem], path: &[usize]) {
        let sel_node = if let Sel::Node(id) = self.sel { Some(id) } else { None };
        // the chosen entry and the submenus it's in
        let mut items = menu;
        let mut trail: Vec<String> = vec![];
        for &k in path {
            if let Some(it) = items.get(k) {
                trail.push(it.label.clone());
                items = &it.children;
            }
        }
        let label = trail.last().cloned().unwrap_or_default();
        // a template: the generators (and scene objects) the original's template file holds
        if label == "From file..." {
            let Some(path) = crate::prefs::file_dialog().set_title("Add from template").add_filter("Templates", &["stt"]).add_filter("All files", &["*"]).pick_file() else { return };
            match crate::templates::load_stt(&path) {
                Ok(list) => self.build_template(list),
                Err(e) => self.set_status(&format!("Couldn't read that template: {}", e)),
            }
            return;
        }
        if trail.first().map(|s| s != "Default generator").unwrap_or(false) {
            if let Some(list) = crate::templates::make(&trail) {
                self.build_template(list);
                return;
            }
        }
        match Self::template_kind(&label) {
            Some(kind) => {
                let parent = sel_node.unwrap_or_else(|| {
                    // default parent: Tree for trunks, newest branch-type node otherwise
                    if kind == Kind::Trunk {
                        self.root_id()
                    } else {
                        self.nodes.iter().rev().find(|n| n.kind.makes_spines()).map(|n| n.id).unwrap_or(self.root_id())
                    }
                });
                let id = self.add_node(kind, parent);
                if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                    // named after the menu entry (default generators keep their usual names)
                    if trail.first().map(|s| s != "Default generator").unwrap_or(false) {
                        n.name = label.clone();
                    }
                    // new generators start neutral and without a material (picture-22)
                    n.p.neutralize();
                }
            }
            None => self.set_status(&format!("The {} generator isn't built in TreeCraft yet", trail.join(" > "))),
        }
    }

    /// Make what a template holds: its generators (the first on the picked generator, or the Tree
    /// for trunks / the newest branch generator otherwise; "child" ones on the template's previous
    /// generator) with the template's values, and its scene objects.
    fn build_template(&mut self, list: Vec<crate::templates::Make>) {
        use crate::templates::Make;
        let sel_node = if let Sel::Node(id) = self.sel { Some(id) } else { None };
        let mut prev: Option<u32> = None;
        let mut made = 0;
        let mut last_name = String::new();
        for m in list {
            match m {
                Make::Gen(t) => {
                    let parent = match (t.child, prev) {
                        (true, Some(p)) => p,
                        _ => sel_node.filter(|&id| self.node(id).map(|n| n.kind != Kind::Leaf || t.kind == Kind::Leaf).unwrap_or(false)).unwrap_or_else(|| {
                            if t.kind == Kind::Trunk {
                                self.root_id()
                            } else {
                                self.nodes.iter().rev().find(|n| n.kind.makes_spines()).map(|n| n.id).unwrap_or(self.root_id())
                            }
                        }),
                    };
                    let id = self.add_node(t.kind, parent);
                    if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                        n.name = t.name.clone();
                        crate::templates::apply(&mut n.p, &t);
                        // the template's layout (mode, length, angle, radius) with neutral
                        // shape and no material (picture-22)
                        n.p.neutralize();
                    }
                    if t.kind != Kind::Leaf {
                        prev = Some(id);
                    }
                    last_name = t.name.clone();
                    made += 1;
                    self.dirty = true;
                }
                Make::Scene(kind, name) => {
                    self.add_scene_object(kind);
                    if let Some(o) = self.scene.last_mut() {
                        o.name = name.clone();
                    }
                    last_name = name;
                    made += 1;
                }
                Make::Grid => {
                    self.show_grid = true;
                    self.set_status("Terrain: the ground grid is shown");
                }
            }
        }
        if made > 0 {
            self.set_status(&format!("Added {}", last_name));
        }
    }

    #[cfg(test)]
    pub(crate) fn gen_add_menu_for_tests() -> Vec<MenuItem> {
        Self::gen_add_menu()
    }

    /// The generator this app makes for an Add menu entry (by its name), or None when that
    /// kind of generator isn't built yet.
    fn template_kind(label: &str) -> Option<Kind> {
        match label {
            "Trunk" | "Trunk Tube" | "SubD Trunk" => Some(Kind::Trunk),
            "Bifurcating Branches" | "Big Branches" | "Branch Tubes" | "Little Branches" | "Single Branch" | "Twigs" | "Roots" | "Branches"
            | "Branch" | "SubD Branch" | "Click Placed Branches" | "Procedural Branch Off Mesh" => Some(Kind::Branch),
            "Leaf Mesh" | "Fronds" | "Palm Frond" | "Click Placed Custom Mesh" | "Mesh" => Some(Kind::Leaf),
            _ => None,
        }
    }

    fn graph_to_screen(&self, body: Rect, p: Vec2) -> Vec2 {
        let origin = vec2(body.x + body.w * 0.5, body.y + body.h - 70.0);
        origin + (p + self.graph_off) * self.graph_zoom
    }

    fn draw_generation(&mut self, ui: &mut Ui, tb: Rect, body: Rect) {
        let sel_node = if let Sel::Node(id) = self.sel { Some(id) } else { None };
        let menu = Self::gen_add_menu();
        let (add, act, rnd) = self.toolbar(ui, tb, "gen", menu.clone(), &["Duplicate", "Delete", "Hide / show", "Arrange nodes", "Frame graph"], true);
        if let Some(path) = add {
            self.gen_add(&menu, &path);
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
        // context menus: on a generator (Add > the whole Add menu, Duplicate, Hide / show, Delete),
        // on empty space (the whole Add menu), on a force or collision object (Delete)
        if let Some(path) = ui.take_menu_result("gen.ctx") {
            match (path.first().copied(), sel_node) {
                (Some(0), _) if path.len() > 1 => self.gen_add(&menu, &path[1..]),
                (Some(2), Some(id)) => self.duplicate_node(id),
                (Some(3), Some(id)) => {
                    if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
                        n.hidden = !n.hidden;
                    }
                    self.dirty = true;
                }
                (Some(4), Some(id)) => self.delete_node(id),
                _ => {}
            }
        }
        if let Some(path) = ui.take_menu_result("gen.ctx.add") {
            self.gen_add(&menu, &path);
        }
        if let Some(path) = ui.take_menu_result("gen.ctx.scene") {
            if let (Some(0), Some(i)) = (path.first().copied(), self.scene_sel) {
                if i < self.scene.len() {
                    self.scene.remove(i);
                    if i < self.scene_gpos.len() {
                        self.scene_gpos.remove(i);
                    }
                    self.scene_sel = None;
                    self.dirty = true;
                }
            }
        }

        // the Node Editor's colours (Preferences > Themes > Node Editor)
        let pc = |k: &str| self.prefs.color(k);
        let (c_bg, c_grid, c_body, c_text, c_wire, c_sel, c_scene) = (pc("node_bg"), pc("node_grid"), pc("node_body"), pc("node_text"), pc("node_wire"), pc("node_selected"), pc("node_scene"));
        let head = |k: Kind| match k {
            Kind::Tree => pc("node_tree"),
            Kind::Trunk => pc("node_trunk"),
            Kind::Branch => pc("node_branch"),
            Kind::Leaf => pc("node_leaf"),
        };
        let fade = |c: Color, a: f32| Color::new(c.r, c.g, c.b, c.a * a);

        ui.set_clip(Some(body));
        draw_rectangle(body.x, body.y, body.w, body.h, c_bg);
        // the background grid: dots, lines, crosses or none, at Preferences > Node Editor's
        // spacing (picture 41)
        let z = self.graph_zoom * self.prefs.node_scale;
        let step = (self.prefs.node_grid_step * self.graph_zoom).max(4.0);
        let o = self.graph_to_screen(body, Vec2::ZERO);
        let grid = self.prefs.node_grid;
        let (gx0, gy0) = (body.x + (o.x - body.x).rem_euclid(step), body.y + (o.y - body.y).rem_euclid(step));
        if grid == 1 {
            let mut gx = gx0;
            while gx < body.x + body.w {
                draw_line(gx, body.y, gx, body.y + body.h, 1.0, c_grid);
                gx += step;
            }
            let mut gy = gy0;
            while gy < body.y + body.h {
                draw_line(body.x, gy, body.x + body.w, gy, 1.0, c_grid);
                gy += step;
            }
        } else if grid != 3 {
            let mut gx = gx0;
            while gx < body.x + body.w {
                let mut gy = gy0;
                while gy < body.y + body.h {
                    if grid == 2 {
                        draw_line(gx - 3.0, gy, gx + 3.0, gy, 1.0, c_grid);
                        draw_line(gx, gy - 3.0, gx, gy + 3.0, 1.0, c_grid);
                    } else {
                        draw_rectangle(gx, gy, 1.5, 1.5, c_grid);
                    }
                    gy += step;
                }
                gx += step;
            }
        }
        // node boxes: a coloured header with the name, a body with the kind and count, a socket
        // below (from the parent) and above (to the children)
        let style = self.prefs.node_style.min(NODE_STYLES.len() - 1);
        let nsz = node_size(style);
        let (nw, nh, hh) = (nsz.x * z, nsz.y * z, 20.0 * z);
        let pr = 4.5 * z.sqrt();
        let boxr = |c: Vec2| Rect::new(c.x - nw * 0.5, c.y - nh * 0.5, nw, nh);
        let top = |c: Vec2| c - vec2(0.0, nh * 0.5);
        let bottom = |c: Vec2| c + vec2(0.0, nh * 0.5);
        let ww = (self.prefs.node_wire_px * z.sqrt()).max(0.8);
        let wire_style = self.prefs.node_wire_style;
        // wires (blue), brighter on the picked generator's
        for n in &self.nodes {
            if let Some(p) = n.parent.and_then(|pid| self.node(pid)) {
                let a = top(self.graph_to_screen(body, p.pos));
                let b = bottom(self.graph_to_screen(body, n.pos));
                let on = sel_node == Some(n.id) || sel_node == Some(p.id);
                let col = if on { Color::new((c_wire.r + 0.35).min(1.0), (c_wire.g + 0.3).min(1.0), (c_wire.b + 0.1).min(1.0), 1.0) } else { fade(c_wire, if n.hidden { 0.4 } else { 0.9 }) };
                draw_wire(a, b, if on { ww * 1.3 } else { ww }, col, wire_style);
            }
        }
        // forces and collision objects: their own nodes, beside the tree's
        self.scene_gpos.resize(self.scene.len(), None);
        let right = self.nodes.iter().map(|n| n.pos.x).fold(0.0f32, f32::max) + 200.0;
        let spos: Vec<Vec2> = (0..self.scene.len()).map(|k| self.scene_gpos[k].unwrap_or(vec2(right, -(k as f32) * 80.0))).collect();

        let m = ui.mouse();
        let can_hover = body.contains(m) && !ui.blocked;
        let mut hover_node = None;
        let mut hover_port: Option<(u32, bool)> = None;
        let mut hover_scene = None;
        if can_hover {
            for n in self.nodes.iter() {
                let c = self.graph_to_screen(body, n.pos);
                if top(c).distance(m) <= pr * 2.2 {
                    hover_port = Some((n.id, true));
                } else if n.kind != Kind::Tree && bottom(c).distance(m) <= pr * 2.2 {
                    hover_port = Some((n.id, false));
                }
                if boxr(c).contains(m) {
                    hover_node = Some(n.id);
                }
            }
            for (k, p) in spos.iter().enumerate() {
                if boxr(self.graph_to_screen(body, *p)).contains(m) {
                    hover_scene = Some(k);
                }
            }
        }
        let fs = (12.0 * z.sqrt()).clamp(9.0, 15.0);
        let fs2 = (10.5 * z.sqrt()).clamp(8.0, 13.0);
        let counts = self.prefs.node_counts;
        let draw_box = |ui: &mut Ui, r: Rect, header: Color, alpha: f32, outline: Option<(Color, f32)>, title: &str, sub: &str| {
            let _ = (fs, fs2, hh);
            draw_node_shape(ui, style, r, header, c_body, c_text, alpha, outline, title, if counts { sub } else { "" }, z);
        };
        let socket = |c: Vec2, linked: bool, hot: bool, alpha: f32| {
            draw_circle(c.x, c.y, pr + 1.5, fade(Color::new(0.05, 0.05, 0.06, 1.0), alpha));
            draw_circle(c.x, c.y, pr, if linked || hot { fade(c_wire, alpha) } else { fade(c_body, alpha) });
            if hot {
                draw_circle_lines(c.x, c.y, pr + 2.5, 1.5, WHITE);
            }
        };
        // generators
        let nodes = self.nodes.clone();
        for n in &nodes {
            let c = self.graph_to_screen(body, n.pos);
            let r = boxr(c);
            let alpha = if n.hidden { 0.45 } else { 1.0 };
            let selected = sel_node == Some(n.id);
            let outline = if selected { Some((c_sel, 2.0)) } else if hover_node == Some(n.id) { Some((Color::new(1.0, 1.0, 1.0, 0.45), 1.0)) } else { None };
            let sub = if n.kind == Kind::Tree {
                format!("Tree  •  {} generators", nodes.len() - 1)
            } else {
{
                let c = self.instance_count(n.id);
                let what = match (n.kind, c == 1) {
                    (Kind::Trunk, true) => "trunk",
                    (Kind::Trunk, false) => "trunks",
                    (Kind::Branch, true) => "branch",
                    (Kind::Branch, false) => "branches",
                    (_, true) => "leaf",
                    _ => "leaves",
                };
                format!("{} {}{}", c, what, if n.hidden { "  (hidden)" } else { "" })
            }
            };
            // a Spine only generator (picture 39): a black header, "spines", and a black badge
            // with a yellow chain of segments, like the original's spine-only node
            let spine_only = n.kind.makes_spines() && crate::gen::pex(&n.p, "Skin:Type", 0.0) as i32 == 3;
            let sub = if spine_only { format!("{} spine only", self.instance_count(n.id)) } else { sub };
            let header = if spine_only { Color::new(0.07, 0.07, 0.07, 1.0) } else { head(n.kind) };
            draw_box(ui, r, header, alpha, outline, &n.name, &sub);
            if spine_only {
                let rb = (nh - hh) * 0.36;
                let bc = vec2(r.x + r.w - rb - 7.0 * z, r.y + hh + (nh - hh) * 0.5);
                draw_circle(bc.x, bc.y, rb + 1.5 * z, fade(Color::new(0.95, 0.78, 0.15, 1.0), alpha));
                draw_circle(bc.x, bc.y, rb, fade(Color::new(0.02, 0.02, 0.02, 1.0), alpha));
                // the chain: links along a gentle diagonal arc
                let links = 5;
                let mut prev: Option<Vec2> = None;
                for i in 0..links {
                    let t = i as f32 / (links - 1) as f32;
                    let p = bc + vec2((t - 0.5) * rb * 1.2, (t - 0.5) * rb * 1.2) + vec2(-(t * (1.0 - t)) * rb * 0.7, (t * (1.0 - t)) * rb * 0.7);
                    if let Some(q) = prev {
                        draw_line(q.x, q.y, p.x, p.y, (1.6 * z).max(1.0), fade(Color::new(0.95, 0.78, 0.15, 1.0), alpha));
                    }
                    draw_circle(p.x, p.y, rb * 0.17, fade(Color::new(1.0, 0.85, 0.2, 1.0), alpha));
                    prev = Some(p);
                }
            }
            let has_kids = nodes.iter().any(|k| k.parent == Some(n.id));
            if n.kind.can_parent() {
                socket(top(c), has_kids, hover_port == Some((n.id, true)), alpha);
            }
            if n.kind != Kind::Tree {
                socket(bottom(c), n.parent.is_some(), hover_port == Some((n.id, false)), alpha);
            }
            ui.record(&format!("node:{}", n.name), r);
        }
        // forces and collision objects
        for (k, o) in self.scene.clone().iter().enumerate() {
            let r = boxr(self.graph_to_screen(body, spos[k]));
            let picked = self.scene_sel == Some(k);
            let outline = if picked { Some((c_sel, 2.0)) } else if hover_scene == Some(k) { Some((Color::new(1.0, 1.0, 1.0, 0.45), 1.0)) } else { None };
            let sub = format!("{}  •  {}", o.kind.name(), if o.kind.is_force() { "force" } else { "collision" });
            draw_box(ui, r, c_scene, 1.0, outline, &o.name, &sub);
            ui.record(&format!("scene:{}", o.name), r);
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
        let at = Rect::new(m.x, m.y, 10.0, 0.0);
        if in_body && ui.active == 0 && self.graph_drag == GraphDrag::None {
            if inp.l_pressed {
                if let Some((id, up)) = hover_port {
                    // drag from a socket to link: from the top one to a new child, from the
                    // bottom one to a new parent
                    self.select(Sel::Node(id));
                    self.graph_drag = if up { GraphDrag::LinkDown { parent: id } } else { GraphDrag::Link { from: id } };
                    ui.active = hid("graph");
                } else if let Some(id) = hover_node {
                    self.select(Sel::Node(id));
                    self.graph_drag = if inp.shift { GraphDrag::Link { from: id } } else { GraphDrag::Node { id, start: m, moved: false } };
                    ui.active = hid("graph");
                } else if let Some(k) = hover_scene {
                    self.select(Sel::None);
                    self.scene_sel = Some(k);
                    self.graph_drag = GraphDrag::Scene { idx: k };
                    ui.active = hid("graph");
                } else {
                    self.select(Sel::None);
                    self.graph_drag = GraphDrag::Pan;
                    ui.active = hid("graph");
                }
            } else if inp.r_pressed {
                if let Some(id) = hover_node {
                    self.select(Sel::Node(id));
                    let tree = vec![MenuItem::sub("Add", menu.clone()), MenuItem::sep(), MenuItem::leaf("Duplicate"), MenuItem::leaf("Hide / show"), MenuItem::leaf("Delete")];
                    ui.open_menu(hid("gen.ctx"), at, tree);
                } else if let Some(k) = hover_scene {
                    self.scene_sel = Some(k);
                    ui.open_menu(hid("gen.ctx.scene"), at, vec![MenuItem::leaf("Delete")]);
                } else {
                    // empty space: every generator the Add menu offers
                    self.select(Sel::None);
                    ui.open_menu(hid("gen.ctx.add"), at, menu.clone());
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
                    let a = bottom(self.graph_to_screen(body, f.pos));
                    draw_wire(m, a, ww, c_wire, wire_style);
                }
                if !inp.l_down {
                    if let Some(to) = hover_node {
                        // the dragged node gets the node it's dropped on as its parent
                        self.reparent(from, to);
                    }
                    self.graph_drag = GraphDrag::None;
                }
            }
            GraphDrag::LinkDown { parent } => {
                if let Some(f) = self.node(parent) {
                    let a = top(self.graph_to_screen(body, f.pos));
                    draw_wire(a, m, ww, c_wire, wire_style);
                }
                if !inp.l_down {
                    if let Some(to) = hover_node {
                        self.reparent(to, parent);
                    }
                    self.graph_drag = GraphDrag::None;
                }
            }
            GraphDrag::Scene { idx } => {
                if inp.l_down {
                    if let Some(p) = spos.get(idx) {
                        if idx < self.scene_gpos.len() {
                            self.scene_gpos[idx] = Some(*p + d / self.graph_zoom);
                        }
                    }
                } else {
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
            let t = "Use + Add (or right-click here) to grow a trunk from the Tree node";
            let w = ui.text_w(t, 12.5);
            ui.text(t, body.x + (body.w - w) * 0.5, body.y + 30.0, 12.5, C_TEXT_FAINT);
        }
        ui.set_clip(None);
        let hint = "Drag from a socket onto a node to link  •  right-click for the Add menu";
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
        let f = match crate::prefs::file_dialog().add_filter("Wavefront OBJ", &["obj"]).pick_file() {
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
        crate::prefs::file_dialog().add_filter("Mesh (OBJ)", &["obj"]).pick_file().map(|p| p.to_string_lossy().to_string())
    }

    pub(crate) fn add_mesh_from_file(&mut self) {
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
        if let Some(p) = crate::prefs::file_dialog().add_filter("Mesh (OBJ)", &["obj"]).set_file_name(&name).save_file() {
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
                // the mesh preview always draws plainly (no viewport render mode, wind or season)
                let light = Lighting { unlit: !self.mesh_lit, mode: 0.0, wind: 0.0, season: -1.0, one_sided: false, ao_on: false, overdraw: false, ..self.light };
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

/// A small drawn icon (about 16 px) for each viewport toolbar button, centred on `c`.
fn toolbar_icon(key: &str, c: Vec2) {
    let (x, y) = (c.x, c.y);
    let w = Color::new(0.92, 0.93, 0.95, 1.0);
    let ring = |r: f32, t: f32, col: Color| draw_circle_lines(x, y, r, t, col);
    match key {
        "vt.render" => {
            // a shaded ball
            draw_circle(x, y, 6.5, Color::new(0.35, 0.55, 0.9, 1.0));
            draw_circle(x - 1.5, y - 1.5, 4.5, Color::new(0.55, 0.75, 1.0, 1.0));
            draw_circle(x - 2.5, y - 2.5, 1.8, WHITE);
        }
        "vt.show" => {
            // an eye
            for s in [-1.0f32, 1.0] {
                let mut prev = vec2(x - 7.0, y);
                for i in 1..=8 {
                    let t = i as f32 / 8.0;
                    let p = vec2(x - 7.0 + 14.0 * t, y + s * 4.5 * (t * std::f32::consts::PI).sin());
                    draw_line(prev.x, prev.y, p.x, p.y, 1.4, w);
                    prev = p;
                }
            }
            draw_circle(x, y, 2.6, Color::new(0.4, 0.75, 1.0, 1.0));
        }
        "vt.zoom" => {
            ring(4.5, 1.6, w);
            draw_line(x + 3.2, y + 3.2, x + 7.0, y + 7.0, 2.2, w);
        }
        "vt.forces" => {
            // a curved push arrow
            let col = Color::new(0.45, 0.85, 1.0, 1.0);
            let mut prev = vec2(x - 7.0, y + 4.0);
            for i in 1..=8 {
                let t = i as f32 / 8.0;
                let p = vec2(x - 7.0 + 12.0 * t, y + 4.0 - 8.0 * t * t);
                draw_line(prev.x, prev.y, p.x, p.y, 1.8, col);
                prev = p;
            }
            draw_triangle(vec2(x + 7.0, y - 5.5), vec2(x + 2.0, y - 6.0), vec2(x + 5.0, y - 1.0), col);
        }
        "vt.coll" => {
            let col = Color::new(1.0, 0.55, 0.5, 1.0);
            draw_circle_lines(x - 3.0, y + 1.0, 4.0, 1.4, col);
            draw_rectangle_lines(x + 1.5, y - 6.0, 6.0, 12.0, 1.4, col);
        }
        "vt.wind" => {
            for (dy, l) in [(-4.0f32, 12.0f32), (0.0, 9.0), (4.0, 13.0)] {
                let mut prev = vec2(x - 7.0, y + dy);
                for i in 1..=6 {
                    let t = i as f32 / 6.0;
                    let p = vec2(x - 7.0 + l * t, y + dy + (t * 6.28).sin() * 1.2);
                    draw_line(prev.x, prev.y, p.x, p.y, 1.4, Color::new(0.75, 0.9, 1.0, 1.0));
                    prev = p;
                }
            }
        }
        "vt.light" => {
            let col = Color::new(1.0, 0.82, 0.25, 1.0);
            draw_circle(x, y, 3.5, col);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::PI / 4.0;
                draw_line(x + a.cos() * 5.0, y + a.sin() * 5.0, x + a.cos() * 7.5, y + a.sin() * 7.5, 1.4, col);
            }
        }
        "vt.gens" => {
            // a little tree
            draw_rectangle(x - 1.2, y, 2.4, 7.0, Color::new(0.65, 0.45, 0.3, 1.0));
            draw_circle(x, y - 2.0, 5.5, Color::new(0.4, 0.8, 0.4, 1.0));
        }
        "vt.nodes" => {
            let col = Color::new(0.85, 0.85, 0.9, 1.0);
            draw_line(x - 5.0, y + 4.0, x, y - 4.0, 1.4, col);
            draw_line(x, y - 4.0, x + 5.0, y + 4.0, 1.4, col);
            for (px, py) in [(x - 5.0, y + 4.0), (x, y - 4.0), (x + 5.0, y + 4.0)] {
                draw_circle(px, py, 2.4, Color::new(0.95, 0.7, 0.3, 1.0));
            }
        }
        "vt.season" => {
            // a leaf, half green half autumn
            draw_circle(x - 1.5, y, 5.0, Color::new(0.45, 0.8, 0.35, 1.0));
            draw_circle(x + 1.5, y, 5.0, Color::new(0.95, 0.55, 0.15, 1.0));
            draw_line(x - 6.0, y + 6.0, x + 4.0, y - 4.0, 1.2, Color::new(0.3, 0.2, 0.1, 1.0));
        }
        "vt.add" => {
            draw_circle(x, y, 7.0, Color::new(0.35, 0.75, 0.45, 1.0));
            draw_line(x - 4.0, y, x + 4.0, y, 2.0, WHITE);
            draw_line(x, y - 4.0, x, y + 4.0, 2.0, WHITE);
        }
        "vt.vis" => {
            ring(6.5, 1.4, w);
            draw_circle(x, y, 2.5, w);
            draw_line(x - 6.0, y + 6.0, x + 6.0, y - 6.0, 1.6, Color::new(1.0, 0.5, 0.4, 1.0));
        }
        "vt.gizmo" => {
            draw_line(x - 5.0, y + 5.0, x + 6.0, y + 5.0, 2.0, Color::new(0.95, 0.35, 0.35, 1.0));
            draw_line(x - 5.0, y + 5.0, x - 5.0, y - 6.0, 2.0, Color::new(0.4, 0.9, 0.4, 1.0));
            draw_line(x - 5.0, y + 5.0, x + 3.0, y - 3.0, 2.0, Color::new(0.4, 0.6, 1.0, 1.0));
        }
        "vt.ao" => {
            draw_circle(x, y, 6.5, Color::new(0.25, 0.25, 0.28, 1.0));
            draw_circle(x - 1.5, y - 1.5, 5.0, Color::new(0.6, 0.6, 0.62, 1.0));
            draw_circle(x - 2.5, y - 2.5, 2.8, Color::new(0.9, 0.9, 0.9, 1.0));
        }
        "vt.lcoll" => {
            draw_circle(x - 2.5, y, 4.5, Color::new(0.45, 0.8, 0.35, 0.9));
            draw_circle(x + 2.5, y, 4.5, Color::new(0.35, 0.65, 0.3, 0.9));
            draw_line(x - 3.0, y - 3.0, x + 3.0, y + 3.0, 1.6, WHITE);
            draw_line(x - 3.0, y + 3.0, x + 3.0, y - 3.0, 1.6, WHITE);
        }
        "vt.export" => {
            draw_rectangle_lines(x - 6.0, y - 3.0, 9.0, 9.0, 1.4, w);
            let col = Color::new(0.45, 0.85, 1.0, 1.0);
            draw_line(x - 2.0, y + 1.0, x + 6.0, y - 7.0, 1.8, col);
            draw_triangle(vec2(x + 7.0, y - 8.0), vec2(x + 1.5, y - 7.0), vec2(x + 6.0, y - 2.5), col);
        }
        _ => {}
    }
}

fn any_perp3(d: Vec3) -> Vec3 {
    if d.y.abs() < 0.9 { d.cross(Vec3::Y).normalize_or_zero() } else { d.cross(Vec3::X).normalize_or_zero() }
}

fn draw_gpu(gpu: &GpuTree, mats: &[Material], gm: &macroquad::material::Material, light: &Lighting, cam_pos: Vec3) {
    gl_use_material(gm);
    for (mi, meshes) in &gpu.parts {
        // no material (NO_MATERIAL or a removed one): plain grey
        let mut fallback = Material::new("?", 0);
        fallback.color = [0.62, 0.62, 0.62];
        fallback.opacity_from_alpha = false;
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

/// Every triangle edge of the tree as a strip `px_world` wide per unit of distance (the
/// thickness in pixels already folded in), turned to face the camera and lifted a little toward
/// it so the surface doesn't hide it; edges of selected vertices (alpha below 200) in `csel`.
fn draw_thick_wires(parts: &[(usize, Vec<Mesh>)], cam: Vec3, ortho: bool, px_world: f32, c: Color, csel: Color) {
    let (cn, cs): ([u8; 4], [u8; 4]) = (c.into(), csel.into());
    let mut verts: Vec<Vertex> = Vec::with_capacity(32000);
    let mut idx: Vec<u16> = Vec::with_capacity(48000);
    let mut n = 0usize;
    let flush = |verts: &mut Vec<Vertex>, idx: &mut Vec<u16>| {
        if !idx.is_empty() {
            let m = Mesh { vertices: std::mem::take(verts), indices: std::mem::take(idx), texture: None };
            draw_mesh(&m);
        }
    };
    'all: for (_, meshes) in parts {
        for mesh in meshes {
            for t in mesh.indices.chunks(3) {
                if t.len() < 3 {
                    continue;
                }
                for (i0, i1) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    let (va, vb) = (&mesh.vertices[i0 as usize], &mesh.vertices[i1 as usize]);
                    let (a, b) = (va.position, vb.position);
                    let mid = (a + b) * 0.5;
                    let to = cam - mid;
                    let dist = if ortho { 1.0 } else { to.length() };
                    let half = 0.5 * px_world * dist;
                    let side = (b - a).cross(to);
                    if side.length_squared() < 1e-14 {
                        continue;
                    }
                    let side = side.normalize() * half;
                    let lift = to.normalize_or_zero() * half * 2.0;
                    let col = if va.color[3] < 200 && vb.color[3] < 200 { cs } else { cn };
                    let base = verts.len() as u16;
                    for p in [a - side + lift, a + side + lift, b + side + lift, b - side + lift] {
                        verts.push(Vertex { position: p, uv: Vec2::ZERO, color: col, normal: Vec4::ZERO });
                    }
                    idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                    if verts.len() >= 31996 {
                        flush(&mut verts, &mut idx);
                    }
                }
                n += 1;
                if n > 60000 {
                    break 'all;
                }
            }
        }
    }
    flush(&mut verts, &mut idx);
}

/// Spine-only generators' spines: a camera-facing yellow strip 2.5 px wide along each, and a
/// small square at every point (one per segment). `px` = world size of a pixel per unit of
/// distance.
fn draw_spine_lines(lines: &[(u32, Vec<Vec3>)], cam: Vec3, ortho: bool, px: f32, sel: Option<u32>) {
    let mut verts: Vec<Vertex> = Vec::with_capacity(8000);
    let mut idx: Vec<u16> = Vec::with_capacity(12000);
    let mut quad = |verts: &mut Vec<Vertex>, idx: &mut Vec<u16>, q: [Vec3; 4], col: [u8; 4]| {
        let base = verts.len() as u16;
        for p in q {
            verts.push(Vertex { position: p, uv: Vec2::ZERO, color: col, normal: Vec4::ZERO });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        if verts.len() > 7990 {
            draw_mesh(&Mesh { vertices: std::mem::take(verts), indices: std::mem::take(idx), texture: None });
        }
    };
    for (id, pts) in lines {
        let on = sel == Some(*id);
        let line: [u8; 4] = if on { [255, 236, 120, 255] } else { [240, 200, 40, 255] };
        let dot: [u8; 4] = if on { [255, 255, 200, 255] } else { [255, 225, 90, 255] };
        let w = if on { 3.5 } else { 2.5 };
        let scale = |p: Vec3| if ortho { px } else { px * (cam - p).length() };
        for k in 0..pts.len().saturating_sub(1) {
            let (a, b) = (pts[k], pts[k + 1]);
            let to = cam - (a + b) * 0.5;
            let side = (b - a).cross(to);
            if side.length_squared() < 1e-14 {
                continue;
            }
            let s = scale((a + b) * 0.5);
            let side = side.normalize() * 0.5 * w * s;
            let lift = to.normalize_or_zero() * 3.0 * s;
            quad(&mut verts, &mut idx, [a - side + lift, a + side + lift, b + side + lift, b - side + lift], line);
        }
        for p in pts {
            let to = (cam - *p).normalize_or_zero();
            let right = to.cross(Vec3::Y).normalize_or(Vec3::X);
            let up = right.cross(to).normalize_or(Vec3::Y);
            let h = 3.5 * scale(*p);
            let c = *p + to * 4.0 * scale(*p);
            quad(&mut verts, &mut idx, [c - right * h - up * h, c + right * h - up * h, c + right * h + up * h, c - right * h + up * h], dot);
        }
    }
    if !idx.is_empty() {
        draw_mesh(&Mesh { vertices: verts, indices: idx, texture: None });
    }
}

/// The Generation editor's node styles (Preferences > Node Editor, picture 41).
pub const NODE_STYLES: [&str; 5] = ["Rectangle", "Rounded", "Circle", "Ellipse", "Hexagon"];

/// A node's size in a style, at zoom 1.
pub fn node_size(style: usize) -> Vec2 {
    match style {
        2 => vec2(58.0, 58.0),
        3 => vec2(132.0, 56.0),
        4 => vec2(132.0, 50.0),
        _ => vec2(128.0, 46.0),
    }
}

/// One generator node in a style, filling `r`: 0 Rectangle (a coloured header over the body), 1
/// Rounded (a pill with a coloured cap), 2 Circle (a coloured disc, the name above and the count
/// below, like the original's icons), 3 Ellipse (a coloured rim), 4 Hexagon (a coloured badge).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_node_shape(ui: &mut Ui, style: usize, r: Rect, header: Color, body: Color, text: Color, alpha: f32, outline: Option<(Color, f32)>, title: &str, sub: &str, z: f32) {
    let fade = |c: Color, a: f32| Color::new(c.r, c.g, c.b, c.a * a);
    let fs = (12.0 * z.sqrt()).clamp(9.0, 15.0);
    let fs2 = (10.5 * z.sqrt()).clamp(8.0, 13.0);
    let shadow = Color::new(0.0, 0.0, 0.0, 0.35 * alpha);
    let line = |dflt: f32| outline.unwrap_or((Color::new(0.0, 0.0, 0.0, 0.55 * alpha), dflt));
    match style {
        1 => {
            let rad = r.h * 0.5;
            draw_round_rect(Rect::new(r.x + 3.0, r.y + 4.0, r.w, r.h), rad, shadow);
            draw_round_rect(r, rad, fade(body, alpha));
            draw_circle(r.x + rad, r.y + rad, rad - 3.0 * z, fade(header, alpha));
            let (c, t) = line(1.0);
            draw_round_rect_lines(r, rad, t, c);
            let tx = r.x + r.h + 2.0 * z;
            ui.text_fit(title, tx, r.y + r.h * 0.3 - fs * 0.5, r.w - r.h - 8.0 * z, fs, fade(text, alpha));
            ui.text_fit(sub, tx, r.y + r.h * 0.68 - fs2 * 0.5, r.w - r.h - 8.0 * z, fs2, fade(text, 0.6 * alpha));
        }
        2 => {
            let (cx, cy, rad) = (r.x + r.w * 0.5, r.y + r.h * 0.5, r.w.min(r.h) * 0.5);
            draw_circle(cx + 2.0, cy + 3.0, rad, shadow);
            draw_circle(cx, cy, rad, fade(Color::new(0.04, 0.04, 0.05, 1.0), alpha));
            draw_circle(cx, cy, rad * 0.8, fade(header, alpha));
            let (c, t) = line(1.5);
            draw_circle_lines(cx, cy, rad, t, c);
            // the kind's first letter in the disc
            let letter: String = title.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            ui.text_center(&letter, Rect::new(r.x, r.y, r.w, r.h), (rad * 0.9).max(9.0), fade(text, alpha));
            // the name and count beside the disc, clear of the sockets and wires
            ui.text(title, r.x + r.w + 6.0 * z, cy - fs - 1.0, fs, fade(text, alpha));
            ui.text(sub, r.x + r.w + 6.0 * z, cy + 1.0, fs2, fade(text, 0.6 * alpha));
        }
        3 => {
            let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
            draw_ellipse(cx + 2.0, cy + 3.0, r.w * 0.5, r.h * 0.5, 0.0, shadow);
            draw_ellipse(cx, cy, r.w * 0.5, r.h * 0.5, 0.0, fade(header, alpha));
            draw_ellipse(cx, cy, r.w * 0.5 - 4.0 * z, r.h * 0.5 - 4.0 * z, 0.0, fade(body, alpha));
            if let Some((c, t)) = outline {
                draw_ellipse_lines(cx, cy, r.w * 0.5, r.h * 0.5, 0.0, t, c);
            }
            let tw = ui.text_w(title, fs).min(r.w * 0.7);
            ui.text_fit(title, cx - tw * 0.5, cy - fs - 1.0, r.w * 0.7, fs, fade(text, alpha));
            let sw = ui.text_w(sub, fs2).min(r.w * 0.7);
            ui.text_fit(sub, cx - sw * 0.5, cy + 1.0, r.w * 0.7, fs2, fade(text, 0.6 * alpha));
        }
        4 => {
            let hx = |o: f32, d: f32| -> [Vec2; 6] {
                let (x, y, w, h) = (r.x + o + d, r.y + d, r.w - 2.0 * d, r.h - 2.0 * d);
                let k = h * 0.45;
                [vec2(x + k, y), vec2(x + w - k, y), vec2(x + w, y + h * 0.5), vec2(x + w - k, y + h), vec2(x + k, y + h), vec2(x, y + h * 0.5)]
            };
            let fill = |pts: [Vec2; 6], c: Color, dy: f32| {
                let ctr = pts.iter().fold(Vec2::ZERO, |a, p| a + *p) / 6.0 + vec2(0.0, dy);
                for i in 0..6 {
                    draw_triangle(ctr, pts[i] + vec2(0.0, dy), pts[(i + 1) % 6] + vec2(0.0, dy), c);
                }
            };
            fill(hx(3.0, 0.0), shadow, 3.0);
            fill(hx(0.0, 0.0), fade(header, alpha), 0.0);
            fill(hx(0.0, 3.5 * z), fade(body, alpha), 0.0);
            let (c, t) = line(1.0);
            let p = hx(0.0, 0.0);
            for i in 0..6 {
                draw_line(p[i].x, p[i].y, p[(i + 1) % 6].x, p[(i + 1) % 6].y, t, c);
            }
            let k = r.h * 0.45;
            ui.text_fit(title, r.x + k, r.y + r.h * 0.3 - fs * 0.5, r.w - 2.0 * k, fs, fade(text, alpha));
            ui.text_fit(sub, r.x + k, r.y + r.h * 0.68 - fs2 * 0.5, r.w - 2.0 * k, fs2, fade(text, 0.6 * alpha));
        }
        _ => {
            let hh = 20.0 * z;
            draw_round_rect(Rect::new(r.x + 3.0, r.y + 4.0, r.w, r.h), 5.0 * z, shadow);
            draw_round_rect(r, 5.0 * z, fade(body, alpha));
            draw_round_rect(Rect::new(r.x, r.y, r.w, hh), 5.0 * z, fade(header, alpha));
            draw_rectangle(r.x, r.y + hh - 5.0 * z, r.w, 5.0 * z, fade(header, alpha));
            let (c, t) = line(1.0);
            draw_round_rect_lines(r, 5.0 * z, t, c);
            ui.text_fit(title, r.x + 7.0 * z, r.y + (hh - fs) * 0.5 - 1.0, r.w - 12.0 * z, fs, fade(text, alpha));
            ui.text_fit(sub, r.x + 7.0 * z, r.y + hh + (r.h - hh - fs2) * 0.5 - 1.0, r.w - 12.0 * z, fs2, fade(text, 0.6 * alpha));
        }
    }
}

/// A node-graph wire: a vertical S-curve from  (a parent's top socket) up to  (a child's
/// bottom socket).
fn draw_wire(a: Vec2, b: Vec2, w: f32, col: Color, style: usize) {
    // Preferences > Node Editor > Wire style (picture 41): 1 Straight, 2 Angled
    if style == 1 {
        draw_line(a.x, a.y, b.x, b.y, w, col);
        return;
    }
    if style == 2 {
        let my = (a.y + b.y) * 0.5;
        for (p, q) in [(a, vec2(a.x, my)), (vec2(a.x, my), vec2(b.x, my)), (vec2(b.x, my), b)] {
            draw_line(p.x, p.y, q.x, q.y, w, col);
            draw_circle(q.x, q.y, w * 0.5, col);
        }
        return;
    }
    let k = ((b.y - a.y).abs() * 0.5).max(30.0);
    let (c1, c2) = (a - vec2(0.0, k), b + vec2(0.0, k));
    let mut prev = a;
    for i in 1..=28 {
        let t = i as f32 / 28.0;
        let u = 1.0 - t;
        let p = a * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + b * (t * t * t);
        draw_line(prev.x, prev.y, p.x, p.y, w, col);
        draw_circle(p.x, p.y, w * 0.5, col);
        prev = p;
    }
}
