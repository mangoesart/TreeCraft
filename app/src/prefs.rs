//! Edit > Preferences (pictures 20-21): a window laid out like Blender's, with its pages on the
//! left. The pages hold what the original keeps in its default Properties panel (Edit: Wind,
//! Lighting, Global properties, Window properties) plus the viewport colours:
//! - Themes: the 3D viewport's colours (the original's options include "Sets the background
//!   color", "Sets the grid color", "Sets the color of the viewport labels", fn 0x14052e030),
//!   a brightness for the background, a colour picker (hue bar and saturation/value square)
//!   and named colour presets.
//! - Viewport: the original's window properties (fn 0x14052e030 tooltips) that apply here.
//! - Lighting, Wind and Global: the light, wind and the Tree generator's own settings.
//! Settings are saved to TreeCraft/prefs.txt in the user's application-data folder.

use crate::app::App;
use crate::model::Kind;
use crate::ui::*;
use macroquad::prelude::*;

/// (key saved in the file, label, default colour as RGBA)
/// Keys starting with node_ are the Node Editor's colours; the rest the 3D viewport's.
pub const VIEWPORT_COLORS: [(&str, &str, [f32; 4]); 34] = [
    ("bg_top", "Background (top)", [0.26, 0.27, 0.29, 1.0]),
    ("bg_bottom", "Background (bottom)", [0.14, 0.14, 0.15, 1.0]),
    ("grid", "Grid", [1.0, 1.0, 1.0, 0.07]),
    ("grid_major", "Grid (every 5th line)", [1.0, 1.0, 1.0, 0.16]),
    ("grid_x", "Grid X axis", [0.85, 0.3, 0.3, 0.7]),
    ("grid_z", "Grid Z axis", [0.3, 0.45, 0.9, 0.7]),
    ("labels", "Viewport labels", [0.62, 0.63, 0.66, 1.0]),
    ("bones", "Bones", [1.0, 0.85, 0.2, 1.0]),
    ("normals", "Normals", [0.3, 0.6, 1.0, 0.8]),
    ("scribed", "Scribed edges", [0.05, 0.05, 0.05, 0.55]),
    ("forces", "Forces", [0.4, 0.8, 1.0, 0.9]),
    ("collision", "Collision objects", [1.0, 0.45, 0.45, 0.9]),
    ("picked", "Picked object", [1.0, 0.85, 0.2, 1.0]),
    ("axis_x", "Axes: X", [0.9, 0.35, 0.35, 1.0]),
    ("axis_y", "Axes: Y", [0.4, 0.85, 0.4, 1.0]),
    ("axis_z", "Axes: Z", [0.4, 0.55, 1.0, 1.0]),
    // Node Editor (picture 23): muted colours per kind, blue wires
    ("node_bg", "Background", [0.12, 0.12, 0.13, 1.0]),
    ("node_grid", "Grid", [1.0, 1.0, 1.0, 0.05]),
    ("node_body", "Node body", [0.19, 0.19, 0.2, 1.0]),
    ("node_tree", "Header: Tree", [0.30, 0.34, 0.42, 1.0]),
    ("node_trunk", "Header: Trunk", [0.42, 0.33, 0.26, 1.0]),
    ("node_branch", "Header: Branches", [0.30, 0.38, 0.26, 1.0]),
    ("node_leaf", "Header: Leaves", [0.22, 0.42, 0.33, 1.0]),
    ("node_scene", "Header: Forces & objects", [0.38, 0.30, 0.46, 1.0]),
    ("node_text", "Node text", [0.88, 0.88, 0.9, 1.0]),
    ("node_wire", "Wires", [0.18, 0.52, 0.96, 1.0]),
    ("node_selected", "Selected node", [1.0, 0.62, 0.2, 1.0]),
    // Render > WireShaded and Render > Wireframe (picture 24)
    ("wire_shaded", "WireShaded edges", [0.04, 0.04, 0.05, 0.75]),
    ("wire_only", "Wireframe lines", [0.85, 0.88, 0.95, 0.55]),
    // the original's viewport colours (picture 42): sky, horizon, below the horizon, the ground
    ("sky_top", "Sky (top)", [0.58, 0.77, 0.94, 1.0]),
    ("sky_horizon", "Sky (horizon)", [0.75, 0.88, 0.98, 1.0]),
    ("sky_ground", "Below the horizon", [0.64, 0.65, 0.61, 1.0]),
    ("ground", "Ground", [0.55, 0.55, 0.50, 1.0]),
    // the world origin's disc (picture 46)
    // a faint pale yellow (picture 56)
    ("origin", "World origin", [0.90, 0.90, 0.60, 0.20]),
];

/// Viewport backgrounds (Preferences > Themes): TreeCraft's dark gradient, or the original's sky.
pub const BG_STYLES: [&str; 2] = ["Dark gradient (TreeCraft)", "Sky and ground (SpeedTree)"];

/// The pages (the unused ones are gone; the original's Edit items have their own).
/// (Lighting moved to the Properties panel, picture 43, so the viewport stays in view.)
/// (The Node Editor's settings live under Themes > Node Editor, picture 44.)
/// The original's own tabs follow (General, Tree Window, Property Bar, Render, Data; picture 45).
pub const PAGES: [&str; 9] = ["Themes", "Viewport", "Wind", "Global", "General", "Tree Window", "Property Bar", "Render", "Data"];

/// The original's preference choices (its dialog's texts, fn 0x140047920; picture 45).
pub const HIDE_SEL_CHOICES: [&str; 3] = ["Always", "When wind is on", "Never"];
pub const NAV_SCHEMES: [&str; 3] = ["Default", "Autodesk Maya", "Autodesk 3ds Max"];
pub const GROUP_CLICKS: [&str; 2] = ["Slams to top", "Reveals"];
pub const MSAA_CHOICES: [&str; 4] = ["Disabled", "2x", "4x", "8x"];
pub const SHADOW_AA_CHOICES: [&str; 5] = ["1", "5", "9", "17", "25"];
pub const MAX_TEX_CHOICES: [&str; 5] = ["512", "1024", "2048", "4096", "8192"];

/// The Generation editor's grid and wire choices (Preferences > Node Editor, picture 41).
pub const NODE_GRIDS: [&str; 4] = ["Dots", "Lines", "Crosses", "None"];
pub const NODE_WIRES: [&str; 3] = ["Curved", "Straight", "Angled"];

#[derive(Clone)]
pub struct Prefs {
    pub colors: Vec<[f32; 4]>,
    /// Background brightness (multiplies both background colours).
    pub bg_brightness: f32,
    /// Named colour presets.
    pub presets: Vec<(String, Vec<[f32; 4]>)>,
    // Viewport (the original's window properties)
    pub wireframe: bool,
    pub textures: bool,
    pub alpha_test: bool,
    pub labels: bool,
    pub manip_size: f32,
    pub snap_move: bool,
    pub snap_move_unit: f32,
    pub snap_rotate: bool,
    pub snap_rotate_deg: f32,
    pub snap_scale: bool,
    pub snap_scale_amount: f32,
    pub zoom_scale: f32,
    pub line_width: f32,
    pub near_mult: f32,
    pub far_mult: f32,
    /// File > Recent files, newest first.
    pub recent: Vec<String>,
    /// The window as it was last closed: maximized, or its size (picture 23). 0 = never closed.
    pub window_max: bool,
    pub window_w: i32,
    pub window_h: i32,
    /// The viewport's navigation gizmo (picture 27): GIZMO_STYLES index, and its size.
    pub gizmo_style: usize,
    pub gizmo_size: f32,
    /// Whether dragging turns the Front, Back, Left, Right, Top and Bottom views (default no:
    /// they only zoom and pan, picture 27).
    pub rotate_fixed_views: bool,
    /// The viewport's Selection menu (picture 30): SEL_MODES index; Solid fill colour (alpha =
    /// how strongly it covers); Outline colour and thickness in pixels.
    pub sel_mode: usize,
    pub sel_fill: [f32; 4],
    pub sel_outline: [f32; 4],
    pub sel_thickness: f32,
    /// The colour wheel's colours used last, newest first (picture 31).
    pub recent_colors: Vec<[f32; 3]>,
    /// WireShaded / Wireframe lines (picture 38): thickness in pixels, and how much brighter the
    /// selected generator's lines are (0 = the same, 1 = white).
    pub wire_px: f32,
    pub wire_sel_boost: f32,
    /// The Generation editor (picture 41): node style (crate::app::NODE_STYLES), grid style and
    /// spacing, wire style and thickness, node size, and whether nodes show their counts.
    pub node_style: usize,
    pub node_grid: usize,
    pub node_grid_step: f32,
    pub node_wire_style: usize,
    pub node_wire_px: f32,
    pub node_scale: f32,
    pub node_counts: bool,
    /// How dark the soft shadow under each node is, 0 = none (picture 88).
    pub node_shadow: f32,
    /// The viewport's background (BG_STYLES) and whether the ground is drawn (picture 42).
    pub bg_style: usize,
    pub show_ground: bool,
    // the original's General tab (picture 45)
    pub display_scaling: f32,
    pub warn_node_edits: bool,
    pub multi_proc: bool,
    pub native_dialogs: bool,
    pub autosave: bool,
    pub autosave_notify: bool,
    pub autosave_min: f32,
    // Tree Window
    pub hide_sel_on_edit: usize,
    pub show_fan: bool,
    pub height_size: f32,
    pub nav_scheme: usize,
    pub rev_zoom: bool,
    pub rev_rotate: bool,
    pub rev_pan: bool,
    // Property Bar
    pub use_sliders: bool,
    pub group_stacking: bool,
    pub animate_scroll: bool,
    /// Where the mode buttons (Generator, Node, Freehand, Export) sit (picture 69): 0 Top (under
    /// the menu bar), 1 Left, 2 Right or 3 Bottom of the Properties panel.
    pub mode_bar: usize,
    pub group_click: usize,
    // Render
    pub msaa: usize,
    pub shadow_aa: usize,
    pub max_tex: usize,
    pub flip_back: bool,
}

pub const SEL_MODES: [&str; 3] = ["Solid fill", "Outline", "Both"];

/// Navigation gizmo designs (Preferences > Viewport).
pub const GIZMO_STYLES: [&str; 5] = ["Axis balls (Blender-like)", "Arrows", "View cube", "Minimal dots", "Hidden"];

fn prefs_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
    Some(std::path::PathBuf::from(base).join("TreeCraft").join("prefs.txt"))
}

fn col4(v: &str) -> Option<[f32; 4]> {
    let n: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    (n.len() == 4).then(|| [n[0], n[1], n[2], n[3]])
}

impl Prefs {
    pub fn defaults() -> Self {
        Prefs {
            colors: VIEWPORT_COLORS.iter().map(|c| c.2).collect(),
            bg_brightness: 1.0,
            presets: vec![],
            wireframe: false,
            textures: true,
            alpha_test: true,
            labels: true,
            manip_size: 1.0,
            snap_move: false,
            snap_move_unit: 1.0,
            snap_rotate: false,
            snap_rotate_deg: 15.0,
            snap_scale: false,
            snap_scale_amount: 0.1,
            zoom_scale: 1.0,
            line_width: 1.0,
            near_mult: 1.0,
            far_mult: 1.0,
            recent: vec![],
            window_max: true,
            window_w: 0,
            window_h: 0,
            gizmo_style: 0,
            gizmo_size: 1.0,
            rotate_fixed_views: false,
            sel_mode: 0,
            sel_fill: [1.0, 0.62, 0.22, 0.38],
            sel_outline: [1.0, 0.62, 0.2, 1.0],
            sel_thickness: 3.0,
            recent_colors: vec![],
            wire_px: 1.8,
            wire_sel_boost: 0.35,
            node_style: 0,
            // very faint grid lines (picture 88)
            node_grid: 1,
            node_grid_step: 40.0,
            node_wire_style: 0,
            node_wire_px: 2.4,
            node_scale: 1.0,
            node_counts: true,
            node_shadow: 0.6,
            bg_style: 1,
            show_ground: true,
            display_scaling: 1.0,
            warn_node_edits: true,
            multi_proc: true,
            native_dialogs: false,
            autosave: true,
            autosave_notify: true,
            autosave_min: 5.0,
            hide_sel_on_edit: 1,
            show_fan: true,
            height_size: 1.0,
            nav_scheme: 0,
            rev_zoom: false,
            rev_rotate: false,
            rev_pan: false,
            use_sliders: true,
            group_stacking: false,
            animate_scroll: true,
            mode_bar: 0,
            group_click: 1,
            msaa: 2,
            shadow_aa: 2,
            max_tex: 2,
            flip_back: true,
        }
    }

    /// Put a file at the top of File > Recent files and save. Like the original (fn 0x1402a76e0):
    /// back slashes become '/', an earlier entry for it is removed, at most 16 are kept.
    pub fn add_recent(&mut self, path: &str) {
        let path = path.replace('\\', "/");
        self.recent.retain(|p| *p != path);
        self.recent.insert(0, path);
        self.recent.truncate(16);
        self.save();
    }

    /// A file that couldn't be opened leaves the list (fn 0x1402a7f00).
    pub fn remove_recent(&mut self, path: &str) {
        let path = path.replace('\\', "/");
        self.recent.retain(|p| *p != path);
        self.save();
    }

    /// The settings saved last time, or the defaults.
    pub fn load() -> Self {
        let mut p = Self::defaults();
        let Some(path) = prefs_path() else { return p };
        let Ok(text) = std::fs::read_to_string(path) else { return p };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            if let Some(name) = k.strip_prefix("preset.") {
                let mut cols: Vec<[f32; 4]> = v.split(';').filter_map(col4).collect();
                // presets saved before the Node Editor colours existed get their defaults
                for c in VIEWPORT_COLORS.iter().skip(cols.len()) {
                    cols.push(c.2);
                }
                if cols.len() == VIEWPORT_COLORS.len() {
                    p.presets.push((name.to_string(), cols));
                }
                continue;
            }
            if k == "recent" {
                p.recent.push(v.to_string());
                continue;
            }
            if let Some(i) = VIEWPORT_COLORS.iter().position(|c| c.0 == k) {
                if let Some(c) = col4(v) {
                    // the old, faint WireShaded default gives way to the new one (picture 38)
                    if k == "wire_shaded" && c == [0.0, 0.0, 0.0, 0.45] {
                        continue;
                    }
                    // the old orange world-origin default gives way to the pale one (picture 56)
                    if k == "origin" && (c[0] - 0.80).abs() < 0.005 && (c[1] - 0.42).abs() < 0.005 && (c[2] - 0.26).abs() < 0.005 {
                        continue;
                    }
                    p.colors[i] = c;
                }
                continue;
            }
            let f = v.parse::<f32>().ok();
            let b = v == "true";
            match k {
                "bg_brightness" => p.bg_brightness = f.unwrap_or(1.0),
                "wireframe" => p.wireframe = b,
                "textures" => p.textures = b,
                "alpha_test" => p.alpha_test = b,
                "labels" => p.labels = b,
                "manip_size" => p.manip_size = f.unwrap_or(1.0),
                "snap_move" => p.snap_move = b,
                "snap_move_unit" => p.snap_move_unit = f.unwrap_or(1.0),
                "snap_rotate" => p.snap_rotate = b,
                "snap_rotate_deg" => p.snap_rotate_deg = f.unwrap_or(15.0),
                "snap_scale" => p.snap_scale = b,
                "snap_scale_amount" => p.snap_scale_amount = f.unwrap_or(0.1),
                "zoom_scale" => p.zoom_scale = f.unwrap_or(1.0),
                "line_width" => p.line_width = f.unwrap_or(1.0),
                "near_mult" => p.near_mult = f.unwrap_or(1.0),
                "far_mult" => p.far_mult = f.unwrap_or(1.0),
                "window_max" => p.window_max = b,
                "gizmo_style" => p.gizmo_style = v.parse::<usize>().unwrap_or(0).min(GIZMO_STYLES.len() - 1),
                "gizmo_size" => p.gizmo_size = f.unwrap_or(1.0),
                "rotate_fixed_views" => p.rotate_fixed_views = b,
                "sel_mode" => p.sel_mode = v.parse::<usize>().unwrap_or(0).min(SEL_MODES.len() - 1),
                "sel_fill" => p.sel_fill = col4(v).unwrap_or(p.sel_fill),
                "sel_outline" => p.sel_outline = col4(v).unwrap_or(p.sel_outline),
                "sel_thickness" => p.sel_thickness = f.unwrap_or(3.0),
                "wire_px" => p.wire_px = f.unwrap_or(1.8).clamp(0.5, 8.0),
                "wire_sel_boost" => p.wire_sel_boost = f.unwrap_or(0.35).clamp(0.0, 1.0),
                "node_style" => p.node_style = v.parse::<usize>().unwrap_or(0).min(crate::app::NODE_STYLES.len() - 1),
                // (picture 88: grid lines by default, so a grid saved before is read again under
                // a new key and the old dots setting is left behind)
                "node_grid2" => p.node_grid = v.parse::<usize>().unwrap_or(1).min(NODE_GRIDS.len() - 1),
                "node_shadow" => p.node_shadow = f.unwrap_or(0.6).clamp(0.0, 1.0),
                "node_grid_step" => p.node_grid_step = f.unwrap_or(40.0).clamp(10.0, 200.0),
                "node_wire_style" => p.node_wire_style = v.parse::<usize>().unwrap_or(0).min(NODE_WIRES.len() - 1),
                "node_wire_px" => p.node_wire_px = f.unwrap_or(2.4).clamp(0.5, 8.0),
                "node_scale" => p.node_scale = f.unwrap_or(1.0).clamp(0.5, 2.0),
                "node_counts" => p.node_counts = b,
                "bg_style" => p.bg_style = v.parse::<usize>().unwrap_or(1).min(BG_STYLES.len() - 1),
                "show_ground" => p.show_ground = b,
                "display_scaling" => p.display_scaling = f.unwrap_or(1.0).clamp(0.5, 2.5),
                "warn_node_edits" => p.warn_node_edits = b,
                "multi_proc" => p.multi_proc = b,
                "native_dialogs" => p.native_dialogs = b,
                "autosave" => p.autosave = b,
                "autosave_notify" => p.autosave_notify = b,
                "autosave_min" => p.autosave_min = f.unwrap_or(5.0).clamp(0.5, 120.0),
                "hide_sel_on_edit" => p.hide_sel_on_edit = v.parse::<usize>().unwrap_or(1).min(2),
                "show_fan" => p.show_fan = b,
                "height_size" => p.height_size = f.unwrap_or(1.0).clamp(0.5, 3.0),
                "nav_scheme" => p.nav_scheme = v.parse::<usize>().unwrap_or(0).min(2),
                "rev_zoom" => p.rev_zoom = b,
                "rev_rotate" => p.rev_rotate = b,
                "rev_pan" => p.rev_pan = b,
                "use_sliders" => p.use_sliders = b,
                "group_stacking" => p.group_stacking = b,
                "animate_scroll" => p.animate_scroll = b,
                "mode_bar" => p.mode_bar = v.parse::<usize>().unwrap_or(0).min(3),
                "group_click" => p.group_click = v.parse::<usize>().unwrap_or(1).min(1),
                "msaa" => p.msaa = v.parse::<usize>().unwrap_or(2).min(3),
                "shadow_aa" => p.shadow_aa = v.parse::<usize>().unwrap_or(2).min(4),
                "max_tex" => p.max_tex = v.parse::<usize>().unwrap_or(2).min(4),
                "flip_back" => p.flip_back = b,
                "recent_color" => {
                    let n: Vec<f32> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
                    if n.len() == 3 && p.recent_colors.len() < 12 {
                        p.recent_colors.push([n[0], n[1], n[2]]);
                    }
                }
                "window_w" => p.window_w = v.parse().unwrap_or(0),
                "window_h" => p.window_h = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        p
    }

    pub fn save(&self) {
        let Some(path) = prefs_path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let c4 = |c: &[f32; 4]| format!("{},{},{},{}", c[0], c[1], c[2], c[3]);
        let mut s = String::from("# TreeCraft preferences\n");
        for (k, c) in VIEWPORT_COLORS.iter().zip(&self.colors) {
            s.push_str(&format!("{}={}\n", k.0, c4(c)));
        }
        s.push_str(&format!(
            "bg_brightness={}\nwireframe={}\ntextures={}\nalpha_test={}\nlabels={}\nmanip_size={}\nsnap_move={}\nsnap_move_unit={}\nsnap_rotate={}\nsnap_rotate_deg={}\nsnap_scale={}\nsnap_scale_amount={}\nzoom_scale={}\nline_width={}\nnear_mult={}\nfar_mult={}\n",
            self.bg_brightness, self.wireframe, self.textures, self.alpha_test, self.labels, self.manip_size, self.snap_move, self.snap_move_unit, self.snap_rotate, self.snap_rotate_deg, self.snap_scale, self.snap_scale_amount, self.zoom_scale, self.line_width, self.near_mult, self.far_mult
        ));
        for (name, cols) in &self.presets {
            s.push_str(&format!("preset.{}={}\n", name.replace('=', "-"), cols.iter().map(c4).collect::<Vec<_>>().join(";")));
        }
        for r in &self.recent {
            s.push_str(&format!("recent={}\n", r));
        }
        s.push_str(&format!("window_max={}\nwindow_w={}\nwindow_h={}\n", self.window_max, self.window_w, self.window_h));
        s.push_str(&format!("gizmo_style={}\ngizmo_size={}\nrotate_fixed_views={}\n", self.gizmo_style, self.gizmo_size, self.rotate_fixed_views));
        s.push_str(&format!("sel_mode={}\nsel_fill={}\nsel_outline={}\nsel_thickness={}\n", self.sel_mode, c4(&self.sel_fill), c4(&self.sel_outline), self.sel_thickness));
        s.push_str(&format!("wire_px={}\nwire_sel_boost={}\n", self.wire_px, self.wire_sel_boost));
        s.push_str(&format!(
            "node_style={}\nnode_grid2={}\nnode_grid_step={}\nnode_wire_style={}\nnode_wire_px={}\nnode_scale={}\nnode_counts={}\nnode_shadow={}\n",
            self.node_style, self.node_grid, self.node_grid_step, self.node_wire_style, self.node_wire_px, self.node_scale, self.node_counts, self.node_shadow
        ));
        s.push_str(&format!("bg_style={}\nshow_ground={}\n", self.bg_style, self.show_ground));
        s.push_str(&format!(
            "display_scaling={}\nwarn_node_edits={}\nmulti_proc={}\nnative_dialogs={}\nautosave={}\nautosave_notify={}\nautosave_min={}\n",
            self.display_scaling, self.warn_node_edits, self.multi_proc, self.native_dialogs, self.autosave, self.autosave_notify, self.autosave_min
        ));
        s.push_str(&format!(
            "hide_sel_on_edit={}\nshow_fan={}\nheight_size={}\nnav_scheme={}\nrev_zoom={}\nrev_rotate={}\nrev_pan={}\n",
            self.hide_sel_on_edit, self.show_fan, self.height_size, self.nav_scheme, self.rev_zoom, self.rev_rotate, self.rev_pan
        ));
        s.push_str(&format!("use_sliders={}\ngroup_stacking={}\nanimate_scroll={}\ngroup_click={}\nmode_bar={}\n", self.use_sliders, self.group_stacking, self.animate_scroll, self.group_click, self.mode_bar));
        s.push_str(&format!("msaa={}\nshadow_aa={}\nmax_tex={}\nflip_back={}\n", self.msaa, self.shadow_aa, self.max_tex, self.flip_back));
        for c in &self.recent_colors {
            s.push_str(&format!("recent_color={},{},{}\n", c[0], c[1], c[2]));
        }
        let _ = std::fs::write(path, s);
    }

    /// A viewport colour by its key (background colours with the brightness applied).
    pub fn color(&self, key: &str) -> Color {
        let i = VIEWPORT_COLORS.iter().position(|c| c.0 == key).unwrap_or(0);
        let c = self.colors.get(i).copied().unwrap_or(VIEWPORT_COLORS[i].2);
        let k = if key.starts_with("bg_") { self.bg_brightness } else { 1.0 };
        Color::new((c[0] * k).min(1.0), (c[1] * k).min(1.0), (c[2] * k).min(1.0), c[3])
    }
}

pub(crate) fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let h = if d < 1e-6 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if mx == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, if mx > 0.0 { d / mx } else { 0.0 }, mx)
}

pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
    let (r, g, b) = match h6 as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r + m, g + m, b + m)
}

impl App {
    /// The Preferences window (a modal window over the app).
    pub fn draw_prefs_window(&mut self, ui: &mut Ui) {
        if !self.prefs_open {
            return;
        }
        ui.blocked = false;
        let (sw, sh) = (ui.sw, ui.sh);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.45));
        // movable by its title bar and resizable from any of its four corners (picture 40)
        let w0 = (sw - 60.0).min(1040.0);
        let h0 = (sh - 50.0).min(720.0);
        let mut r = self.prefs_win.unwrap_or(Rect::new((sw - w0) * 0.5, (sh - h0) * 0.5, w0, h0));
        {
            let m = ui.mouse();
            let (min_w, min_h) = (620.0f32.min(sw - 8.0), 420.0f32.min(sh - 8.0));
            let cs = 12.0;
            let corners = [Rect::new(r.x - 4.0, r.y - 4.0, cs, cs), Rect::new(r.x + r.w - cs + 4.0, r.y - 4.0, cs, cs), Rect::new(r.x - 4.0, r.y + r.h - cs + 4.0, cs, cs), Rect::new(r.x + r.w - cs + 4.0, r.y + r.h - cs + 4.0, cs, cs)];
            let title = Rect::new(r.x, r.y, r.w, 26.0);
            if self.prefs_win_drag == 0 && ui.inp.l_pressed && ui.popup.is_none() {
                if let Some(k) = corners.iter().position(|c| c.contains(m)) {
                    self.prefs_win_drag = 2 + k as u8;
                } else if title.contains(m) {
                    self.prefs_win_drag = 1;
                }
                if self.prefs_win_drag != 0 {
                    self.prefs_win_grab = (m, r);
                    ui.inp.l_pressed = false;
                }
            }
            if self.prefs_win_drag != 0 {
                if ui.inp.l_down {
                    let (start, r0) = self.prefs_win_grab;
                    let d = m - start;
                    r = match self.prefs_win_drag {
                        1 => Rect::new(r0.x + d.x, r0.y + d.y, r0.w, r0.h),
                        k => {
                            let (left, top) = (k == 2 || k == 4, k == 2 || k == 3);
                            let (mut x0, mut y0, mut x1, mut y1) = (r0.x, r0.y, r0.x + r0.w, r0.y + r0.h);
                            if left { x0 = (x0 + d.x).min(x1 - min_w); } else { x1 = (x1 + d.x).max(x0 + min_w); }
                            if top { y0 = (y0 + d.y).min(y1 - min_h); } else { y1 = (y1 + d.y).max(y0 + min_h); }
                            Rect::new(x0, y0, x1 - x0, y1 - y0)
                        }
                    };
                } else {
                    self.prefs_win_drag = 0;
                }
            }
            // keep the title bar on screen
            r.x = r.x.clamp(-r.w + 80.0, sw - 80.0);
            r.y = r.y.clamp(0.0, sh - 30.0);
            self.prefs_win = Some(r);
            // the corners show as small grips; the hovered one lights up
            self.prefs_corners = corners.to_vec();
            if title.contains(m) && self.prefs_win_drag == 0 {
                ui.tooltip = Some("Drag to move; drag a corner to resize".into());
            }
        }
        draw_rectangle(r.x + 4.0, r.y + 5.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(r, 6.0, C_PANEL);
        draw_round_rect(Rect::new(r.x, r.y, r.w, 26.0), 6.0, C_PANEL2);
        ui.text_center("Preferences", Rect::new(r.x, r.y, r.w, 26.0), 13.0, C_TEXT);
        ui.record("prefs.window", r);
        // the pages
        let side = Rect::new(r.x + 8.0, r.y + 34.0, 150.0, r.h - 76.0);
        draw_round_rect(side, 6.0, C_BG);
        self.prefs_page = self.prefs_page.min(PAGES.len() - 1);
        // Global properties moved to the Properties panel (picture 49): its page is gone from the list
        if self.prefs_page == 3 {
            self.prefs_page = 1;
        }
        let mut row = 0usize;
        for (i, name) in PAGES.iter().enumerate() {
            if i == 3 {
                continue;
            }
            let b = Rect::new(side.x + 4.0, side.y + 4.0 + row as f32 * 26.0, side.w - 8.0, 24.0);
            row += 1;
            let on = self.prefs_page == i;
            if on {
                draw_round_rect(b, 4.0, Color::new(0.30, 0.47, 0.78, 1.0));
            } else if b.contains(ui.mouse()) {
                draw_round_rect(b, 4.0, Color::new(1.0, 1.0, 1.0, 0.08));
            }
            ui.text(name, b.x + 10.0, b.y + 5.0, 12.5, if on { WHITE } else { C_TEXT });
            ui.record(&format!("prefs.page.{}", name), b);
            if ui.clicked(b) {
                self.prefs_page = i;
                self.prefs_pick = None;
            }
        }
        let page = Rect::new(side.x + side.w + 10.0, side.y, r.x + r.w - (side.x + side.w + 10.0) - 8.0, side.h);
        let before = (self.prefs.colors.clone(), self.prefs.bg_brightness, self.prefs.presets.len());
        let opts_before = self.prefs_opts_sig();
        match self.prefs_page {
            0 => self.prefs_themes(ui, page),
            1 => self.prefs_viewport(ui, page),
            2 => self.prefs_wind(ui, page),
            4 => self.prefs_general(ui, page),
            5 => self.prefs_tree_window(ui, page),
            6 => self.prefs_property_bar(ui, page),
            7 => self.prefs_render(ui, page),
            8 => self.prefs_data(ui, page),
            _ => self.prefs_global(ui, page),
        }
        if before != (self.prefs.colors.clone(), self.prefs.bg_brightness, self.prefs.presets.len()) || opts_before != self.prefs_opts_sig() {
            self.prefs.save();
        }
        // the corner grips, drawn over the window (the hovered one lit)
        for (k, c) in self.prefs_corners.clone().iter().enumerate() {
            let hot = c.contains(ui.mouse()) || self.prefs_win_drag == 2 + k as u8;
            let col = if hot { C_ACCENT } else { Color::new(1.0, 1.0, 1.0, 0.28) };
            let (left, top) = (k == 0 || k == 2, k < 2);
            let (cx, cy) = (if left { c.x + 4.0 } else { c.x + c.w - 4.0 }, if top { c.y + 4.0 } else { c.y + c.h - 4.0 });
            let (dx, dy) = (if left { 1.0 } else { -1.0 }, if top { 1.0 } else { -1.0 });
            for i in 1..3 {
                let o = i as f32 * 3.5;
                draw_line(cx + dx * o, cy, cx, cy + dy * o, 1.2, col);
            }
        }
        // bottom: Reset (this page's colours), Close
        let by = r.y + r.h - 34.0;
        if self.prefs_page == 0 && ui.button("prefs.reset", Rect::new(r.x + 8.0, by, 150.0, 24.0), "Reset to defaults") {
            let presets = std::mem::take(&mut self.prefs.presets);
            let opts = self.prefs.clone();
            self.prefs = Prefs { colors: Prefs::defaults().colors, bg_brightness: 1.0, presets, ..opts };
            self.prefs.save();
        }
        if ui.button("prefs.close", Rect::new(r.x + r.w - 100.0, by, 92.0, 24.0), "Close") || (ui.inp.key(KeyCode::Escape) && !self.prefs_name_focus) {
            self.prefs_open = false;
            self.prefs_pick = None;
        }
        // the window takes all input
        ui.blocked = true;
        if ui.popup.is_none() {
            ui.inp.l_pressed = false;
        }
        ui.inp.r_pressed = false;
        ui.inp.keys.clear();
        ui.inp.chars.clear();
    }

    fn prefs_opts_sig(&self) -> String {
        let p = &self.prefs;
        format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}|{}{}{}|{}{}|{}{}{}{}{}{}{}|{}{}|{}",
            p.wireframe, p.textures, p.alpha_test, p.labels, p.manip_size, p.snap_move, p.snap_move_unit, p.snap_rotate, p.snap_rotate_deg, p.snap_scale, p.snap_scale_amount, p.zoom_scale, p.line_width, p.near_mult, p.far_mult, p.gizmo_style, p.gizmo_size, p.rotate_fixed_views, p.wire_px, p.wire_sel_boost, p.node_style, p.node_grid, p.node_grid_step, p.node_wire_style, p.node_wire_px, p.node_scale, p.node_counts, p.bg_style, p.show_ground, p.app_sig()
        )
    }

    /// Themes > 3D Viewport: colours in two columns, background brightness, the picked colour's
    /// picker, and presets.
    fn prefs_themes(&mut self, ui: &mut Ui, page: Rect) {
        // two sets of colours: the 3D viewport's and the Node Editor's (picture 23)
        ui.text("Themes  >", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let mut tx = page.x + 4.0 + ui.text_w("Themes  >", 13.0) + 10.0;
        for (k, nm) in ["3D Viewport", "Node Editor"].iter().enumerate() {
            let tw = ui.text_w(nm, 12.5) + 20.0;
            let tr = Rect::new(tx, page.y - 1.0, tw, 20.0);
            let on = self.prefs_theme_tab == k;
            draw_round_rect(tr, 4.0, if on { Color::new(0.30, 0.47, 0.78, 1.0) } else { C_BG });
            ui.text_center(nm, tr, 12.5, if on { WHITE } else { C_TEXT_DIM });
            ui.record(&format!("prefs.theme.{}", nm), tr);
            if ui.clicked(tr) && !on {
                self.prefs_theme_tab = k;
                self.prefs_pick = None;
            }
            tx += tw + 6.0;
        }
        // the Node Editor's colours are the node_ ones; the rest belong to the 3D viewport
        let shown: Vec<usize> = (0..VIEWPORT_COLORS.len()).filter(|&i| VIEWPORT_COLORS[i].0.starts_with("node_") == (self.prefs_theme_tab == 1)).collect();
        let col_w = (page.w - 16.0) * 0.5;
        let rows = (shown.len() + 1) / 2;
        for (j, &i) in shown.iter().enumerate() {
            let label = VIEWPORT_COLORS[i].1;
            let (cx, cy) = (page.x + (j / rows) as f32 * (col_w + 16.0), page.y + 26.0 + (j % rows) as f32 * 26.0);
            let lw = ui.text_w(label, 12.5);
            let label_w = col_w * 0.48;
            ui.text(label, cx + label_w - lw - 8.0, cy + 3.0, 12.5, C_TEXT_DIM);
            let sw_r = Rect::new(cx + label_w, cy, col_w - label_w, 21.0);
            let c = self.prefs.colors[i];
            checker(sw_r, 6.0);
            draw_rectangle(sw_r.x, sw_r.y, sw_r.w, sw_r.h, Color::new(c[0], c[1], c[2], c[3]));
            draw_rectangle_lines(sw_r.x, sw_r.y, sw_r.w, sw_r.h, if self.prefs_pick == Some(i) { 2.0 } else { 1.0 }, if self.prefs_pick == Some(i) { C_ACCENT } else { C_LINE });
            ui.record(&format!("prefs.color.{}", i), sw_r);
            if ui.clicked(sw_r) {
                self.prefs_pick = if self.prefs_pick == Some(i) { None } else { Some(i) };
            }
        }
        let mut y = page.y + 30.0 + rows as f32 * 26.0;
        // background brightness (both background colours)
        if self.prefs_theme_tab == 0 {
            ui.text("Background brightness", page.x + 4.0, y + 3.0, 12.5, C_TEXT_DIM);
            ui.slider("prefs.bgb", Rect::new(page.x + 170.0, y, page.w - 180.0, 20.0), &mut self.prefs.bg_brightness, 0.0, 3.0, 2);
            y += 30.0;
            // the background: TreeCraft's dark gradient or the original's sky and ground (picture 42)
            ui.text("Background", page.x + 4.0, y + 3.0, 12.5, C_TEXT_DIM);
            let styles: Vec<String> = BG_STYLES.iter().map(|s| s.to_string()).collect();
            let before = self.prefs.bg_style;
            ui.combo("prefs.bgstyle", Rect::new(page.x + 170.0, y, (page.w - 180.0).min(320.0), 20.0), &styles, &mut self.prefs.bg_style);
            if before != self.prefs.bg_style {
                self.prefs.save();
            }
            y += 30.0;
        }
        // the Node Editor's look under its colours (picture 44: one place for the node editor)
        if self.prefs_theme_tab == 1 {
            y = self.prefs_node_editor(ui, Rect::new(page.x, y, page.w, page.h)) + 8.0;
        }
        // the picked colour: hue bar, saturation/value square, R G B A sliders
        if let Some(i) = self.prefs_pick {
            let ed = Rect::new(page.x, y, page.w, 150.0);
            draw_round_rect(ed, 6.0, C_BG);
            ui.text(VIEWPORT_COLORS[i].1, ed.x + 10.0, ed.y + 6.0, 12.5, C_TEXT);
            let c = self.prefs.colors[i];
            let (mut hh, mut ss, mut vv) = rgb_to_hsv(c[0], c[1], c[2]);
            let sq = Rect::new(ed.x + 10.0, ed.y + 26.0, 116.0, 116.0);
            let hb = Rect::new(sq.x + sq.w + 8.0, sq.y, 16.0, sq.h);
            // the square: white -> hue across, black down
            let n = 16;
            for a in 0..n {
                for b in 0..n {
                    let (s0, v0) = (a as f32 / (n - 1) as f32, 1.0 - b as f32 / (n - 1) as f32);
                    let (rr, gg, bb) = hsv_to_rgb(hh, s0, v0);
                    draw_rectangle(sq.x + a as f32 * sq.w / n as f32, sq.y + b as f32 * sq.h / n as f32, sq.w / n as f32 + 0.5, sq.h / n as f32 + 0.5, Color::new(rr, gg, bb, 1.0));
                }
            }
            for k in 0..48 {
                let (rr, gg, bb) = hsv_to_rgb(k as f32 / 48.0, 1.0, 1.0);
                draw_rectangle(hb.x, hb.y + k as f32 * hb.h / 48.0, hb.w, hb.h / 48.0 + 0.5, Color::new(rr, gg, bb, 1.0));
            }
            draw_circle_lines(sq.x + ss * sq.w, sq.y + (1.0 - vv) * sq.h, 4.0, 1.5, WHITE);
            draw_rectangle_lines(hb.x - 1.0, hb.y + hh * hb.h - 2.0, hb.w + 2.0, 4.0, 1.5, WHITE);
            ui.record("prefs.picker", sq);
            let m = ui.mouse();
            let mut changed = false;
            if ui.inp.l_down && ui.popup.is_none() {
                if sq.contains(m) || self.prefs_drag == 1 {
                    if ui.inp.l_pressed || self.prefs_drag == 1 {
                        self.prefs_drag = 1;
                        ss = ((m.x - sq.x) / sq.w).clamp(0.0, 1.0);
                        vv = 1.0 - ((m.y - sq.y) / sq.h).clamp(0.0, 1.0);
                        changed = true;
                    }
                } else if hb.contains(m) || self.prefs_drag == 2 {
                    if ui.inp.l_pressed || self.prefs_drag == 2 {
                        self.prefs_drag = 2;
                        hh = ((m.y - hb.y) / hb.h).clamp(0.0, 0.999);
                        changed = true;
                    }
                }
            } else {
                self.prefs_drag = 0;
            }
            if changed {
                let (rr, gg, bb) = hsv_to_rgb(hh, ss, vv);
                self.prefs.colors[i] = [rr, gg, bb, c[3]];
            }
            let names = ["Red", "Green", "Blue", "Alpha"];
            let sx = hb.x + hb.w + 14.0;
            for (k, nm) in names.iter().enumerate() {
                let yy = ed.y + 28.0 + k as f32 * 24.0;
                ui.text(nm, sx, yy + 3.0, 12.0, C_TEXT_DIM);
                let mut v = self.prefs.colors[i][k];
                if ui.slider(&format!("prefs.c{}.{}", i, k), Rect::new(sx + 50.0, yy, ed.x + ed.w - sx - 160.0, 20.0), &mut v, 0.0, 1.0, 3) {
                    self.prefs.colors[i][k] = v;
                }
            }
            let c = self.prefs.colors[i];
            let prev = Rect::new(ed.x + ed.w - 96.0, ed.y + 28.0, 86.0, 92.0);
            checker(prev, 8.0);
            draw_rectangle(prev.x, prev.y, prev.w, prev.h, Color::new(c[0], c[1], c[2], c[3]));
            if ui.button("prefs.c.default", Rect::new(ed.x + ed.w - 200.0, ed.y + 4.0, 96.0, 20.0), "Default") {
                self.prefs.colors[i] = VIEWPORT_COLORS[i].2;
            }
            y += 158.0;
        }
        // where the mode buttons sit (picture 70: here under Themes)
        ui.text("Mode buttons", page.x + 4.0, y + 3.0, 13.0, C_TEXT);
        let mr = Rect::new(page.x + 230.0, y, 220.0, 22.0);
        combo_of(ui, "prefs.modebar", mr, &MODE_BAR_PLACES, &mut self.prefs.mode_bar);
        if mr.contains(ui.mouse()) {
            ui.tooltip = Some("Where Generator, Node, Freehand and Export sit in the Properties panel: a row along its Top or Bottom, or tabs down its Left or Right side".into());
        }
        y += 32.0;
        // presets: save the current colours under a name; load or delete saved ones
        ui.text("Presets", page.x + 4.0, y + 3.0, 13.0, C_TEXT);
        y += 24.0;
        let mut name = std::mem::take(&mut self.prefs_name);
        let mut focus = self.prefs_name_focus;
        crate::matpanel::text_field(ui, "prefs.preset.name", Rect::new(page.x + 4.0, y, 220.0, 22.0), &mut name, &mut focus);
        if name.is_empty() && !focus {
            ui.text("preset name", page.x + 9.0, y + 4.0, 12.0, C_TEXT_FAINT);
        }
        if ui.button("prefs.preset.save", Rect::new(page.x + 230.0, y, 110.0, 22.0), "Save preset") {
            let nm = if name.trim().is_empty() { format!("Preset {}", self.prefs.presets.len() + 1) } else { name.trim().to_string() };
            match self.prefs.presets.iter_mut().find(|p| p.0 == nm) {
                Some(p) => p.1 = self.prefs.colors.clone(),
                None => self.prefs.presets.push((nm.clone(), self.prefs.colors.clone())),
            }
            self.prefs.save();
            self.set_status(&format!("Saved the viewport colours as \"{}\"", nm));
            name.clear();
            focus = false;
        }
        self.prefs_name = name;
        self.prefs_name_focus = focus;
        y += 28.0;
        let mut load = None;
        let mut del = None;
        for (k, (nm, cols)) in self.prefs.presets.iter().enumerate() {
            let row = Rect::new(page.x + 4.0, y, page.w - 8.0, 22.0);
            if y + 22.0 > page.y + page.h {
                break;
            }
            // a strip of the preset's first colours
            for (j, c) in cols.iter().take(8).enumerate() {
                draw_rectangle(row.x + j as f32 * 12.0, row.y + 3.0, 11.0, 16.0, Color::new(c[0], c[1], c[2], 1.0));
            }
            ui.text(nm, row.x + 104.0, row.y + 4.0, 12.5, C_TEXT);
            if ui.button(&format!("prefs.preset.load.{}", k), Rect::new(row.x + row.w - 150.0, row.y, 70.0, 22.0), "Load") {
                load = Some(k);
            }
            if ui.button(&format!("prefs.preset.del.{}", k), Rect::new(row.x + row.w - 74.0, row.y, 70.0, 22.0), "Delete") {
                del = Some(k);
            }
            y += 26.0;
        }
        if let Some(k) = load {
            self.prefs.colors = self.prefs.presets[k].1.clone();
        }
        if let Some(k) = del {
            self.prefs.presets.remove(k);
            self.prefs.save();
        }
    }

    /// The original's window properties (fn 0x14052e030) that apply to TreeCraft's viewport.
    fn prefs_viewport(&mut self, ui: &mut Ui, page: Rect) {
        // (the original's Window properties are in the Properties panel now, picture 49)
        ui.text("Viewport display", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let mut y = page.y + 30.0;
        let lw = 250.0;
        let fw = (page.w - lw - 20.0).min(360.0);
        let mut row = |ui: &mut Ui, y: &mut f32, label: &str, tip: &str| -> Rect {
            // right-aligned against its control (picture 57)
            let tw = ui.text_w(label, 12.5);
            ui.text(label, page.x + lw - 10.0 - tw, *y + 3.0, 12.5, C_TEXT_DIM);
            let r = Rect::new(page.x + lw, *y, fw, 20.0);
            if Rect::new(page.x, *y, page.w, 22.0).contains(ui.mouse()) {
                ui.tooltip = Some(tip.to_string());
            }
            *y += 25.0;
            r
        };
        let mut lit = !self.light.unlit;
        let r = row(ui, &mut y, "Lighting", "Enables lighting");
        ui.checkbox("pv.lit", r, &mut lit);
        self.light.unlit = !lit;
        let r = row(ui, &mut y, "Wireframe", "Enables wireframe (lines only) rendering");
        ui.checkbox("pv.wire", r, &mut self.prefs.wireframe);
        // the wire's look in Render > WireShaded and Wireframe (picture 38)
        let r = row(ui, &mut y, "  Wire thickness (px)", "How thick the WireShaded and Wireframe lines are drawn, in pixels");
        ui.slider("pv.wirepx", r, &mut self.prefs.wire_px, 0.5, 6.0, 1);
        self.prefs.wire_px = self.prefs.wire_px.clamp(0.5, 8.0);
        for (key, label, tip) in [("wire_shaded", "  WireShaded colour", "The colour of the edges drawn over the shaded tree (A = how strongly)"), ("wire_only", "  Wireframe colour", "The colour of the lines in Wireframe (A = how strongly)")] {
            let r = row(ui, &mut y, label, tip);
            let i = VIEWPORT_COLORS.iter().position(|c| c.0 == key).unwrap_or(0);
            let c = &mut self.prefs.colors[i];
            draw_rectangle(r.x, r.y, 26.0, r.h, Color::new(c[0], c[1], c[2], 1.0));
            draw_rectangle_lines(r.x, r.y, 26.0, r.h, 1.0, C_LINE);
            let sw = (r.w - 32.0) / 4.0;
            for (k, nm) in ["r", "g", "b", "a"].iter().enumerate() {
                let rr = Rect::new(r.x + 30.0 + k as f32 * (sw + 1.0), r.y, sw - 1.0, r.h);
                ui.slider(&format!("pv.{}.{}", key, nm), rr, &mut c[k], 0.0, 1.0, 2);
                c[k] = c[k].clamp(0.0, 1.0);
            }
        }
        let r = row(ui, &mut y, "  Selected wire brightness", "How much brighter the selected generator's wires are in WireShaded and Wireframe (0 = the same)");
        ui.slider("pv.wiresel", r, &mut self.prefs.wire_sel_boost, 0.0, 1.0, 2);
        self.prefs.wire_sel_boost = self.prefs.wire_sel_boost.clamp(0.0, 1.0);
        let r = row(ui, &mut y, "Texture mapping", "Enables texture mapping");
        ui.checkbox("pv.tex", r, &mut self.prefs.textures);
        let r = row(ui, &mut y, "Hide back faces", "Enables rendering optimizations based on polygon visibility");
        ui.checkbox("pv.cull", r, &mut self.one_sided);
        let r = row(ui, &mut y, "Alpha testing", "Enables alpha testing");
        ui.checkbox("pv.alpha", r, &mut self.prefs.alpha_test);
        let r = row(ui, &mut y, "Show grid", "Enables the rendering of grid lines");
        ui.checkbox("pv.grid", r, &mut self.show_grid);
        let r = row(ui, &mut y, "Show ground", "A ground under the grid that takes the tree's shadow (picture 42)");
        ui.checkbox("pv.ground", r, &mut self.prefs.show_ground);
        let r = row(ui, &mut y, "Height indicator", "A measuring staff beside the tree with its height (also in Show)");
        ui.checkbox("pv.height", r, &mut self.show_height);
        let r = row(ui, &mut y, "Label viewports", "Controls when viewports are labelled");
        ui.checkbox("pv.labels", r, &mut self.prefs.labels);
        let r = row(ui, &mut y, "Field of view", "The perspective camera's field of view (degrees)");
        ui.slider("pv.fov", r, &mut self.cam.fov, 10.0, 100.0, 0);
        let r = row(ui, &mut y, "Zoom distance scale", "Scales how far away cameras are placed from target objects during zoom operations");
        ui.slider("pv.zoom", r, &mut self.prefs.zoom_scale, 0.25, 4.0, 2);
        let r = row(ui, &mut y, "Manipulator size", "Controls the size of the manipulator");
        ui.slider("pv.manip", r, &mut self.prefs.manip_size, 0.25, 4.0, 2);
        let r = row(ui, &mut y, "Translation snapping", "Enables translation snapping");
        ui.checkbox("pv.snapm", r, &mut self.prefs.snap_move);
        let r = row(ui, &mut y, "  Snap unit", "Smallest unit of movement when snapping is enabled");
        ui.slider("pv.snapmu", r, &mut self.prefs.snap_move_unit, 0.05, 10.0, 2);
        let r = row(ui, &mut y, "Rotation snapping", "Enables rotation snapping");
        ui.checkbox("pv.snapr", r, &mut self.prefs.snap_rotate);
        let r = row(ui, &mut y, "  Snap degrees", "Smallest degrees of rotation when snap is enabled");
        ui.slider("pv.snaprd", r, &mut self.prefs.snap_rotate_deg, 1.0, 90.0, 0);
        let r = row(ui, &mut y, "Scale snapping", "Enables scale snapping");
        ui.checkbox("pv.snaps", r, &mut self.prefs.snap_scale);
        let r = row(ui, &mut y, "  Snap amount", "Smallest amount of scaling when snap is enabled");
        ui.slider("pv.snapsa", r, &mut self.prefs.snap_scale_amount, 0.01, 1.0, 2);
        // navigation (picture 27)
        let r = row(ui, &mut y, "Navigation gizmo", "The axis gizmo at the viewport's top right: click an axis to look along it");
        let styles: Vec<String> = GIZMO_STYLES.iter().map(|s| s.to_string()).collect();
        ui.combo("pv.gizmo", r, &styles, &mut self.prefs.gizmo_style);
        let r = row(ui, &mut y, "  Gizmo size", "How big the navigation gizmo is drawn");
        ui.slider("pv.gizmosz", r, &mut self.prefs.gizmo_size, 0.6, 2.0, 2);
        let r = row(ui, &mut y, "Rotate fixed views", "Let a drag turn the Front, Back, Left, Right, Top and Bottom views (off: they only zoom and pan)");
        ui.checkbox("pv.rotfixed", r, &mut self.prefs.rotate_fixed_views);
        let _ = &mut y;
    }

    /// The Generation editor's look (pictures 41, 44; drawn under Themes > Node Editor's
    /// colours from `page.y`): five node styles shown as samples to click, grid, wires, node size
    /// and counts. Returns where it ends.
    fn prefs_node_editor(&mut self, ui: &mut Ui, page: Rect) -> f32 {
        use crate::app::{draw_node_shape, node_size, NODE_STYLES};
        let page = Rect::new(page.x, page.y - 26.0, page.w, page.h);
        ui.text("Node style", page.x + 4.0, page.y + 30.0, 12.5, C_TEXT_DIM);
        // the samples: a Trunk node in each style; click one to use it
        let n = NODE_STYLES.len();
        let sw = ((page.w - 16.0) / n as f32).min(170.0);
        let sh = 96.0;
        let (head, bodyc, text) = (self.prefs.color("node_trunk"), self.prefs.color("node_body"), self.prefs.color("node_text"));
        crate::app::set_node_shadow(self.prefs.node_shadow);
        for (k, name) in NODE_STYLES.iter().enumerate() {
            let cell = Rect::new(page.x + 8.0 + k as f32 * sw, page.y + 50.0, sw - 8.0, sh);
            let on = self.prefs.node_style == k;
            let hot = cell.contains(ui.mouse());
            draw_round_rect(cell, 6.0, if on { Color::new(0.30, 0.47, 0.78, 0.35) } else if hot { Color::new(1.0, 1.0, 1.0, 0.07) } else { C_BG });
            draw_round_rect_lines(cell, 6.0, if on { 2.0 } else { 1.0 }, if on { C_ACCENT } else { C_LINE });
            let z = 0.72;
            let size = node_size(k) * z;
            // the Circle style writes its name beside the disc: start it further left
            let c = vec2(cell.x + cell.w * if k == 2 { 0.3 } else { 0.5 }, cell.y + 14.0 + (sh - 34.0) * 0.5);
            let r = Rect::new(c.x - size.x * 0.5, c.y - size.y * 0.5, size.x, size.y);
            crate::nodeicons::set_next(crate::nodeicons::NodeIcon::Trunk);
            draw_node_shape(ui, k, r, head, bodyc, text, 1.0, if on { Some((self.prefs.color("node_selected"), 2.0)) } else { None }, "Trunk", "1 trunk", z);
            ui.text_center(name, Rect::new(cell.x, cell.y + cell.h - 20.0, cell.w, 16.0), 12.0, if on { WHITE } else { C_TEXT_DIM });
            ui.record(&format!("prefs.nodestyle.{}", name), cell);
            if ui.clicked(cell) {
                self.prefs.node_style = k;
            }
        }
        let mut y = page.y + 50.0 + sh + 16.0;
        let lw = 170.0;
        let fw = (page.w - lw - 20.0).min(360.0);
        let mut row = |ui: &mut Ui, y: &mut f32, label: &str, tip: &str| -> Rect {
            // right-aligned against its control (picture 57)
            let tw = ui.text_w(label, 12.5);
            ui.text(label, page.x + lw - 10.0 - tw, *y + 3.0, 12.5, C_TEXT_DIM);
            let r = Rect::new(page.x + lw, *y, fw, 20.0);
            if Rect::new(page.x, *y, page.w, 22.0).contains(ui.mouse()) {
                ui.tooltip = Some(tip.to_string());
            }
            *y += 25.0;
            r
        };
        let r = row(ui, &mut y, "Grid", "The pattern behind the nodes");
        let grids: Vec<String> = NODE_GRIDS.iter().map(|s| s.to_string()).collect();
        ui.combo("pn.grid", r, &grids, &mut self.prefs.node_grid);
        let r = row(ui, &mut y, "  Grid spacing", "How far apart the grid's lines or dots are");
        ui.slider("pn.gridstep", r, &mut self.prefs.node_grid_step, 10.0, 120.0, 0);
        self.prefs.node_grid_step = self.prefs.node_grid_step.clamp(10.0, 200.0);
        let r = row(ui, &mut y, "Wire style", "How the wires between nodes are drawn");
        let wires: Vec<String> = NODE_WIRES.iter().map(|s| s.to_string()).collect();
        ui.combo("pn.wire", r, &wires, &mut self.prefs.node_wire_style);
        let r = row(ui, &mut y, "  Wire thickness", "How thick the wires are");
        ui.slider("pn.wirepx", r, &mut self.prefs.node_wire_px, 0.5, 6.0, 1);
        self.prefs.node_wire_px = self.prefs.node_wire_px.clamp(0.5, 8.0);
        let r = row(ui, &mut y, "Node size", "How big the nodes are drawn");
        ui.slider("pn.scale", r, &mut self.prefs.node_scale, 0.5, 2.0, 2);
        self.prefs.node_scale = self.prefs.node_scale.clamp(0.5, 2.0);
        let r = row(ui, &mut y, "Show counts", "The count of trunks, branches or leaves beside each wire's circle");
        ui.checkbox("pn.counts", r, &mut self.prefs.node_counts);
        let r = row(ui, &mut y, "Node shadow", "How dark the soft shadow under each node is (0 = none)");
        ui.slider("pn.shadow", r, &mut self.prefs.node_shadow, 0.0, 1.0, 2);
        self.prefs.node_shadow = self.prefs.node_shadow.clamp(0.0, 1.0);
        y
    }

    fn prefs_wind(&mut self, ui: &mut Ui, page: Rect) {
        ui.text("Wind", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let fw = (page.w - 200.0).min(360.0);
        ui.text("Wind on", page.x + 8.0, page.y + 33.0, 12.5, C_TEXT_DIM);
        ui.checkbox("pw.on", Rect::new(page.x + 190.0, page.y + 30.0, fw, 20.0), &mut self.wind_on);
        ui.text("Strength", page.x + 8.0, page.y + 58.0, 12.5, C_TEXT_DIM);
        ui.slider("pw.str", Rect::new(page.x + 190.0, page.y + 55.0, fw, 20.0), &mut self.wind_strength, 0.0, 3.0, 2);
        ui.text("Direction", page.x + 8.0, page.y + 83.0, 12.5, C_TEXT_DIM);
        ui.slider("pw.dir", Rect::new(page.x + 190.0, page.y + 80.0, fw, 20.0), &mut self.wind_dir, -180.0, 180.0, 0);
    }

    /// Global properties: the Tree generator's own settings.
    fn prefs_global(&mut self, ui: &mut Ui, page: Rect) {
        ui.text("Global properties  (the Tree generator)", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let fw = (page.w - 200.0).min(360.0);
        let stats = self.geo_stats;
        if let Some(n) = self.nodes.iter_mut().find(|n| n.kind == Kind::Tree) {
            ui.text("Shape radius", page.x + 8.0, page.y + 33.0, 12.5, C_TEXT_DIM);
            if ui.slider("pg.rad", Rect::new(page.x + 190.0, page.y + 30.0, fw, 20.0), &mut n.p.tree_radius, 0.0, 60.0, 1) {
                self.dirty = true;
            }
            ui.text("Preview leaf LOD", page.x + 8.0, page.y + 58.0, 12.5, C_TEXT_DIM);
            if ui.islider("pg.lod", Rect::new(page.x + 190.0, page.y + 55.0, fw, 20.0), &mut n.p.lod_preview, 0, 3) {
                self.dirty = true;
            }
        }
        let lines = [("Triangles", stats.0), ("Vertices", stats.1), ("Branches", stats.2), ("Leaves", stats.3)];
        for (k, (l, v)) in lines.iter().enumerate() {
            let y = page.y + 92.0 + k as f32 * 22.0;
            ui.text(l, page.x + 8.0, y, 12.5, C_TEXT_DIM);
            ui.text(&v.to_string(), page.x + 190.0, y, 12.5, C_TEXT);
        }
    }
}

/// Places for the mode buttons (Preferences > Property Bar > Mode buttons, picture 69).
pub const MODE_BAR_PLACES: [&str; 4] = ["Top", "Left", "Right", "Bottom"];

impl Prefs {
    /// The original's own tabs' settings in one string, to notice changes (and save them).
    pub fn app_sig(&self) -> String {
        format!(
            "{}{}{}{}{}{}{}|{}{}{}{}{}{}{}|{}{}{}{}{}|{}{}{}{}",
            self.display_scaling, self.warn_node_edits, self.multi_proc, self.native_dialogs, self.autosave, self.autosave_notify, self.autosave_min,
            self.hide_sel_on_edit, self.show_fan, self.height_size, self.nav_scheme, self.rev_zoom, self.rev_rotate, self.rev_pan,
            self.use_sliders, self.group_stacking, self.animate_scroll, self.group_click, self.mode_bar,
            self.msaa, self.shadow_aa, self.max_tex, self.flip_back
        )
    }
}

/// A row on one of the original's preference tabs: label, tooltip, and the control's place.
fn pref_row(ui: &mut Ui, page: Rect, y: &mut f32, label: &str, tip: &str) -> Rect {
    let lw = 230.0;
    let fw = (page.w - lw - 20.0).min(320.0);
    // right-aligned against its control (picture 57)
    let tw = ui.text_w(label, 12.5);
    ui.text(label, page.x + lw - 10.0 - tw, *y + 3.0, 12.5, C_TEXT_DIM);
    let r = Rect::new(page.x + lw, *y, fw, 20.0);
    if Rect::new(page.x, *y, page.w, 22.0).contains(ui.mouse()) {
        ui.tooltip = Some(tip.to_string());
    }
    *y += 25.0;
    r
}

fn pref_heading(ui: &mut Ui, page: Rect, y: &mut f32, text: &str) {
    *y += 4.0;
    ui.text(text, page.x + 4.0, *y, 12.5, C_TEXT);
    draw_line(page.x + 4.0, *y + 18.0, page.x + page.w - 8.0, *y + 18.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.06));
    *y += 24.0;
}

fn combo_of(ui: &mut Ui, key: &str, r: Rect, items: &[&str], v: &mut usize) {
    let names: Vec<String> = items.iter().map(|s| s.to_string()).collect();
    ui.combo(key, r, &names, v);
}

impl App {
    /// The original's General tab (fn 0x140047920; picture 45): Display scaling, Warn about node
    /// edit losses, Use multiple processors, Always use native file dialogs, Autosave (Enable,
    /// Notify in console, Interval).
    fn prefs_general(&mut self, ui: &mut Ui, page: Rect) {
        ui.text("General", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let mut y = page.y + 30.0;
        let p = &mut self.prefs;
        let r = pref_row(ui, page, &mut y, "Display scaling", "How big the whole window's text and controls are drawn");
        ui.slider("pg.scaling", r, &mut p.display_scaling, 0.5, 2.5, 2);
        p.display_scaling = p.display_scaling.clamp(0.5, 2.5);
        let r = pref_row(ui, page, &mut y, "Warn about node edit losses", "A message when a change to a generator may undo the edits made to its single nodes (Node mode)");
        ui.checkbox("pg.warn", r, &mut p.warn_node_edits);
        let r = pref_row(ui, page, &mut y, "Use multiple processors", "Pictures (textures) are read on all the computer's processors at once when a tree opens");
        ui.checkbox("pg.multi", r, &mut p.multi_proc);
        let r = pref_row(ui, page, &mut y, "Always use native file dialogs", "On: the system's file windows open where the system likes. Off: they open in the folder used last");
        ui.checkbox("pg.native", r, &mut p.native_dialogs);
        pref_heading(ui, page, &mut y, "Autosave");
        let r = pref_row(ui, page, &mut y, "Enable", "Keep a copy of an unsaved tree every few minutes (Welcome > Recover Last Session brings it back)");
        ui.checkbox("pg.autosave", r, &mut p.autosave);
        let r = pref_row(ui, page, &mut y, "Notify in console", "Say so in the status line (and the console) each time it saves");
        ui.checkbox("pg.notify", r, &mut p.autosave_notify);
        let r = pref_row(ui, page, &mut y, "Interval (min)", "Minutes between autosaves");
        ui.slider("pg.interval", r, &mut p.autosave_min, 0.5, 60.0, 2);
        p.autosave_min = p.autosave_min.clamp(0.5, 120.0);
        y += 10.0;
        if ui.button("pg.reset", Rect::new(page.x + 4.0, y, 210.0, 24.0), "Reset General Preferences") {
            let d = Prefs::defaults();
            let p = &mut self.prefs;
            (p.display_scaling, p.warn_node_edits, p.multi_proc, p.native_dialogs, p.autosave, p.autosave_notify, p.autosave_min) = (d.display_scaling, d.warn_node_edits, d.multi_proc, d.native_dialogs, d.autosave, d.autosave_notify, d.autosave_min);
        }
    }

    /// The original's Tree Window tab: Hide selection indicator on edit, Show fan, Size of height
    /// indicator, Navigation (Control scheme, Reverse zooming / rotating / panning).
    fn prefs_tree_window(&mut self, ui: &mut Ui, page: Rect) {
        ui.text("Tree Window", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let mut y = page.y + 30.0;
        let p = &mut self.prefs;
        let r = pref_row(ui, page, &mut y, "Hide selection indicator on edit", "Hide the selected generator's fill and outline while one of its values is being changed");
        combo_of(ui, "pt.hidesel", r, &HIDE_SEL_CHOICES, &mut p.hide_sel_on_edit);
        let r = pref_row(ui, page, &mut y, "Show fan", "The fan button under the light controller (the wind controller to come)");
        ui.checkbox("pt.fan", r, &mut p.show_fan);
        let r = pref_row(ui, page, &mut y, "Size of height indicator", "How big the height line's marks, numbers and tag are drawn");
        ui.slider("pt.height", r, &mut p.height_size, 0.5, 3.0, 2);
        p.height_size = p.height_size.clamp(0.5, 3.0);
        pref_heading(ui, page, &mut y, "Navigation");
        let r = pref_row(ui, page, &mut y, "Control scheme", "Default: left-drag orbits, middle pans, right zooms. Maya: Alt + left / middle / right. 3ds Max: middle pans, Alt + middle orbits, Ctrl + Alt + middle zooms");
        combo_of(ui, "pt.scheme", r, &NAV_SCHEMES, &mut p.nav_scheme);
        let r = pref_row(ui, page, &mut y, "Reverse zooming", "Zoom the other way with the wheel and zoom drags");
        ui.checkbox("pt.rz", r, &mut p.rev_zoom);
        let r = pref_row(ui, page, &mut y, "Reverse rotating", "Orbit the other way");
        ui.checkbox("pt.rr", r, &mut p.rev_rotate);
        let r = pref_row(ui, page, &mut y, "Reverse panning", "Pan the other way");
        ui.checkbox("pt.rp", r, &mut p.rev_pan);
        y += 10.0;
        if ui.button("pt.reset", Rect::new(page.x + 4.0, y, 230.0, 24.0), "Reset Tree Window Preferences") {
            let d = Prefs::defaults();
            let p = &mut self.prefs;
            (p.hide_sel_on_edit, p.show_fan, p.height_size, p.nav_scheme, p.rev_zoom, p.rev_rotate, p.rev_pan) = (d.hide_sel_on_edit, d.show_fan, d.height_size, d.nav_scheme, d.rev_zoom, d.rev_rotate, d.rev_pan);
        }
    }

    /// The original's Property Bar tab: Use sliders where possible, Use group stacking, Animate
    /// scrolling, Group click (Slams to top / Reveals).
    fn prefs_property_bar(&mut self, ui: &mut Ui, page: Rect) {
        ui.text("Property Bar  (the Properties panel)", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let mut y = page.y + 30.0;
        let p = &mut self.prefs;
        let r = pref_row(ui, page, &mut y, "Use sliders where possible", "Number fields show the dial to drag; off: just the number with its arrows (drag the number itself)");
        ui.checkbox("pp.sliders", r, &mut p.use_sliders);
        let r = pref_row(ui, page, &mut y, "Use group stacking", "Opening a group closes the others, so one group shows at a time");
        ui.checkbox("pp.stack", r, &mut p.group_stacking);
        let r = pref_row(ui, page, &mut y, "Animate scrolling", "The panel glides to where it scrolls instead of jumping");
        ui.checkbox("pp.anim", r, &mut p.animate_scroll);
        let r = pref_row(ui, page, &mut y, "Group click", "Opening a group: Slams to top scrolls it to the top of the panel; Reveals scrolls just enough to show all of it");
        combo_of(ui, "pp.click", r, &GROUP_CLICKS, &mut p.group_click);

        y += 10.0;
        if ui.button("pp.reset", Rect::new(page.x + 4.0, y, 230.0, 24.0), "Reset Property Bar Preferences") {
            let d = Prefs::defaults();
            let p = &mut self.prefs;
            (p.use_sliders, p.group_stacking, p.animate_scroll, p.group_click) = (d.use_sliders, d.group_stacking, d.animate_scroll, d.group_click);
        }
    }

    /// The original's Render tab: Multi-sampling (for windows opened after; restart), Shadow
    /// anti-aliasing samples, Max texture resolution, Flip normals on backsides.
    fn prefs_render(&mut self, ui: &mut Ui, page: Rect) {
        ui.text("Render", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        let mut y = page.y + 30.0;
        let p = &mut self.prefs;
        let r = pref_row(ui, page, &mut y, "Multi-sampling (1, 2)", "Smooths the edges of everything drawn. 1 - applies to newly created windows: takes effect when TreeCraft starts next");
        combo_of(ui, "pr.msaa", r, &MSAA_CHOICES, &mut p.msaa);
        let r = pref_row(ui, page, &mut y, "Shadow anti-aliasing samples", "How smooth the ground shadow's edges are (the shadow is drawn this much finer, then smoothed)");
        combo_of(ui, "pr.shaa", r, &SHADOW_AA_CHOICES, &mut p.shadow_aa);
        let before = p.max_tex;
        let r = pref_row(ui, page, &mut y, "Max texture resolution", "Pictures larger than this are scaled down when loaded");
        combo_of(ui, "pr.maxtex", r, &MAX_TEX_CHOICES, &mut p.max_tex);
        let changed_tex = before != p.max_tex;
        let r = pref_row(ui, page, &mut y, "Flip normals on backsides", "Light the back of a leaf or card as its own side (off: both sides lit like the front)");
        ui.checkbox("pr.flip", r, &mut p.flip_back);
        y += 6.0;
        ui.text("1 - Applies to newly created windows.", page.x + 8.0, y, 11.5, C_TEXT_FAINT);
        ui.text("2 - Must be on for alpha-to-coverage transparency to work.", page.x + 8.0, y + 16.0, 11.5, C_TEXT_FAINT);
        y += 44.0;
        if ui.button("pr.reset", Rect::new(page.x + 4.0, y, 210.0, 24.0), "Reset Render Preferences") {
            let d = Prefs::defaults();
            let p = &mut self.prefs;
            (p.msaa, p.shadow_aa, p.max_tex, p.flip_back) = (d.msaa, d.shadow_aa, d.max_tex, d.flip_back);
        }
        crate::textures::set_max_texture(512 << self.prefs.max_tex.min(4));
        if changed_tex {
            // read the pictures again at the new size
            for m in &mut self.materials {
                m.dirty_base = true;
                crate::textures::refresh_material(m);
            }
            self.set_status(&format!("Pictures reloaded at up to {} pixels", 512 << self.prefs.max_tex.min(4)));
        }
    }

    /// The original's Data tab asks to report anonymous usage data; TreeCraft never sends
    /// anything anywhere, so there's nothing to switch.
    fn prefs_data(&mut self, ui: &mut Ui, page: Rect) {
        ui.text("Data", page.x + 4.0, page.y + 2.0, 13.0, C_TEXT);
        ui.text("TreeCraft never collects or sends any data about how it's used.", page.x + 8.0, page.y + 32.0, 12.5, C_TEXT_DIM);
        ui.text("Everything stays on this computer, so there's no reporting to turn on or off.", page.x + 8.0, page.y + 52.0, 12.5, C_TEXT_DIM);
    }
}

/// Preferences > General > Always use native file dialogs (picture 45): off, TreeCraft's file
/// windows open in the folder used last; on, wherever the system chooses.
static DIALOG_NATIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DIALOG_DIR: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

/// Called every frame with the setting and the folder used last.
pub fn set_dialog_state(native: bool, dir: Option<std::path::PathBuf>) {
    DIALOG_NATIVE.store(native, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut d) = DIALOG_DIR.lock() {
        *d = dir;
    }
}

/// A file window, opening in the folder used last unless native dialogs are asked for.
pub fn file_dialog() -> rfd::FileDialog {
    let fd = rfd::FileDialog::new();
    if DIALOG_NATIVE.load(std::sync::atomic::Ordering::Relaxed) {
        return fd;
    }
    match DIALOG_DIR.lock().ok().and_then(|d| d.clone()).filter(|d| d.is_dir()) {
        Some(d) => fd.set_directory(d),
        None => fd,
    }
}