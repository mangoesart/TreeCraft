//! The main menu bar, laid out like the original's (fn 0x140294d60 creates its actions):
//! File, Edit, Tools, Window, Assets and Help, with cascading submenus.

use crate::app::{App, Sel};
use crate::model::*;
use crate::ui::{hid, MenuItem, Ui, C_BG};
use macroquad::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const MENU_NAMES: [&str; 6] = ["File", "Edit", "Tools", "Window", "Assets", "Help"];

/// Help items that point at the original's web pages: TreeCraft has none, so they say so.
const WEB_ITEMS: [&str; 11] = [
    "Video tutorials...",
    "Support for modeling issues...",
    "Support for UE4 integration issues...",
    "SpeedTree.com...",
    "Follow us on Twitter...",
    "YouTube channel...",
    "Facebook...",
    "Shop for models...",
    "Purchase a license...",
    "Check for updates...",
    "Video tutorials",
];

const CONTROLS: &str = "Viewport: left-drag orbits, middle-drag or Shift+drag pans, the wheel zooms, F frames the tree.\n\
Generation editor: +Add adds a generator under the selected one; Delete removes it.\n\
Ctrl+N new, Ctrl+O open, Ctrl+S save, Ctrl+Shift+S save as, Ctrl+E export mesh,\n\
Ctrl+Z undo, Ctrl+Y redo, Alt+Z / Alt+Y undo / redo selection, Ctrl+W close.";

fn leaf(s: &str) -> MenuItem {
    MenuItem::leaf(s)
}

fn info(title: &str, text: &str) {
    rfd::MessageDialog::new().set_title(title).set_description(text).set_buttons(rfd::MessageButtons::Ok).show();
}

/// The labels along `path` in `tree`.
fn labels(tree: &[MenuItem], path: &[usize]) -> Vec<String> {
    let mut out = vec![];
    let mut items = tree;
    for &i in path {
        let Some(it) = items.get(i) else { break };
        out.push(it.label.clone());
        items = &it.children;
    }
    out
}

/// The original's incremental name (fn 0x14041c880): digits after the last '_' count up and are
/// written "_%03d"; otherwise "_001" is added. "name_007.tctree" -> "name_008.tctree".
pub fn next_incremental(p: &Path) -> PathBuf {
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "tree".into());
    let ext = p.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "tctree".into());
    let name = match stem.rsplit_once('_') {
        Some((b, d)) if d.chars().all(|c| c.is_ascii_digit()) => format!("{}_{:03}", b, d.parse::<u32>().unwrap_or(0) + 1),
        _ => format!("{}_001", stem),
    };
    p.with_file_name(format!("{}.{}", name, ext))
}

impl App {
    fn menu_tree(&self, name: &str) -> Vec<MenuItem> {
        let sep = MenuItem::sep;
        match name {
            "File" => {
                let mut recent: Vec<MenuItem> = self.prefs.recent.iter().map(|r| leaf(r)).collect();
                if recent.is_empty() {
                    recent.push(leaf("(no recent files)"));
                }
                recent.push(sep());
                recent.push(leaf("Clear list"));
                vec![
                    leaf("New").hot("Ctrl+N"),
                    leaf("Open...").hot("Ctrl+O"),
                    MenuItem::sub("Recent files", recent),
                    leaf("Close").hot("Ctrl+W"),
                    sep(),
                    leaf("Save").hot("Ctrl+S"),
                    leaf("Save incremental"),
                    leaf("Save as...").hot("Ctrl+Shift+S"),
                    leaf("Save as with assets...").hot("Ctrl+Alt+Shift+S"),
                    sep(),
                    leaf("Export mesh...").hot("Ctrl+E"),
                    leaf("Export selected as mesh..."),
                    leaf("Export material..."),
                    leaf("Export world building data..."),
                    leaf("Export image..."),
                    leaf("Export image sequence..."),
                    leaf("Render photo..."),
                    sep(),
                    leaf("Import mesh asset..."),
                    leaf("Import material asset..."),
                    leaf("Import cameras from FBX..."),
                    sep(),
                    leaf("Exit"),
                ]
            }
            "Edit" => vec![
                leaf(&self.undo_label()).hot("Ctrl+Z"),
                leaf(&self.redo_label()).hot("Ctrl+Y"),
                leaf("Undo selection").hot("Alt+Z"),
                leaf("Redo selection").hot("Alt+Y"),
                sep(),
                leaf("Delete node").hot("Del"),
                leaf("Duplicate node"),
                leaf("Randomize"),
                leaf("Arrange nodes"),
                sep(),
                leaf("Preferences..."),
                leaf("Edit Tree Window properties..."),
            ],
            "Tools" => vec![
                leaf("Render ambient occlusion"),
                leaf("Clear ambient occlusion"),
                sep(),
                leaf("Generate collision primitives..."),
                leaf("Wind wizard..."),
                leaf("Growth wizard..."),
                sep(),
                MenuItem::sub("Arbitrary scene scale...", ["x 0.1", "x 0.5", "x 2", "x 10"].iter().map(|s| leaf(s)).collect()),
                MenuItem::sub(
                    "Scene unit conversion...",
                    ["Feet to meters", "Meters to feet", "Inches to centimeters", "Centimeters to inches", "Meters to centimeters", "Centimeters to meters"].iter().map(|s| leaf(s)).collect(),
                ),
                sep(),
                leaf("Create collection from selection"),
                leaf("Remove unused assets"),
                leaf("Reset lightmap scalars"),
                leaf("Switch to Substances where possible"),
            ],
            "Window" => vec![
                leaf("Cascade tree windows"),
                leaf("Tile tree windows"),
                sep(),
                leaf("Properties"),
                MenuItem::check("Generation editor", self.tab == 0),
                MenuItem::check("Materials", self.tab == 1),
                MenuItem::check("Meshes", self.tab == 2),
                leaf("Displacements"),
                leaf("Masks"),
                leaf("Output"),
                leaf("Material sets"),
                leaf("Timeline"),
                sep(),
                MenuItem::sub(
                    "Layout",
                    vec![
                        MenuItem::check("Normal", self.layout_mode == 0),
                        MenuItem::check("Full screen", self.layout_mode == 1),
                        MenuItem::check("Full screen no bars", self.layout_mode == 2),
                    ],
                ),
                leaf("Reset layout..."),
            ],
            "Assets" => vec![
                leaf("Add materials from textures..."),
                leaf("Import material asset..."),
                leaf("Import mesh asset..."),
                sep(),
                leaf("Export material..."),
                sep(),
                leaf("Remove unused assets"),
            ],
            _ => vec![
                leaf("User manual..."),
                leaf("Video tutorials..."),
                leaf("Support for modeling issues..."),
                leaf("Support for UE4 integration issues..."),
                sep(),
                leaf("SpeedTree.com..."),
                leaf("Follow us on Twitter..."),
                leaf("YouTube channel..."),
                leaf("Facebook..."),
                sep(),
                leaf("Shop for models..."),
                leaf("Purchase a license..."),
                leaf("Check for updates..."),
                sep(),
                leaf("Controls"),
                leaf("About TreeCraft..."),
            ],
        }
    }

    /// File > New (and Close): ask to save unsaved changes, then start an empty tree.
    pub fn file_new(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        self.new_tree();
        self.tree_path = None;
        self.regenerate();
        self.undo_reset();
    }

    /// Carry out a main-menu choice; returns true for Exit.
    fn menu_command(&mut self, name: &str, path: &[usize]) -> bool {
        let tree = self.menu_tree(name);
        let l = labels(&tree, path);
        let Some(item) = l.last().cloned() else { return false };
        let parent = if l.len() > 1 { l[0].as_str() } else { "" };
        let sel_node = if let Sel::Node(id) = self.sel { Some(id) } else { None };
        match (name, parent, item.as_str()) {
            ("File", "Recent files", "Clear list") => {
                self.prefs.recent.clear();
                self.prefs.save();
            }
            ("File", "Recent files", "(no recent files)") => {}
            ("File", "Recent files", p) => {
                let p = PathBuf::from(p);
                if !p.exists() {
                    self.set_status(&format!("{} isn't there any more", p.display()));
                    self.prefs.recent.retain(|r| Path::new(r) != p);
                    self.prefs.save();
                } else if self.confirm_discard() {
                    self.open_path(p);
                }
            }
            (_, _, "New") | (_, _, "Close") => self.file_new(),
            (_, _, "Open...") => self.open_tree(),
            (_, _, "Save") => {
                self.save_tree(false);
            }
            (_, _, "Save as...") => {
                self.save_tree(true);
            }
            (_, _, "Save incremental") => self.save_incremental(),
            (_, _, "Save as with assets...") => self.save_with_assets(),
            (_, _, "Export mesh...") => self.export_obj(),
            (_, _, "Export selected as mesh...") => self.export_selected(sel_node),
            (_, _, "Export material...") => self.export_material(),
            (_, _, "Export world building data...") => self.export_world_data(),
            (_, _, "Export image...") => self.export_image(false),
            (_, _, "Render photo...") => self.export_image(true),
            (_, _, "Export image sequence...") => self.export_sequence(),
            (_, _, "Import mesh asset...") => {
                self.add_mesh_from_file();
                self.tab = 2;
            }
            (_, _, "Import material asset...") => self.import_material(),
            (_, _, "Add materials from textures...") => {
                self.add_materials_from_textures();
                self.tab = 1;
            }
            (_, _, "Import cameras from FBX...") => info("Import cameras from FBX", "TreeCraft has no FBX reader yet, so cameras can't be imported. Use the viewport's view menu (Perspective, Front, Top...) instead."),
            (_, _, "Exit") => return true,
            ("Edit", _, i) if i.starts_with("Undo ") || i == "Nothing to undo" => {
                if i == "Undo selection" {
                    self.undo_selection();
                } else {
                    self.undo();
                }
            }
            ("Edit", _, i) if i.starts_with("Redo ") || i == "Nothing to redo" => {
                if i == "Redo selection" {
                    self.redo_selection();
                } else {
                    self.redo();
                }
            }
            (_, _, "Delete node") => match sel_node {
                Some(id) => self.delete_node(id),
                None => self.set_status("Select a node first"),
            },
            (_, _, "Duplicate node") => match sel_node {
                Some(id) => self.duplicate_node(id),
                None => self.set_status("Select a node first"),
            },
            (_, _, "Randomize") => self.randomize(sel_node),
            (_, _, "Arrange nodes") => self.arrange(),
            (_, _, "Preferences...") => self.prefs_open = true,
            (_, _, "Edit Tree Window properties...") => self.select(Sel::Window),
            (_, _, "Render ambient occlusion") => {
                self.ao_on = true;
                self.set_status("Ambient occlusion on");
            }
            (_, _, "Clear ambient occlusion") => {
                self.ao_on = false;
                self.set_status("Ambient occlusion cleared");
            }
            (_, _, "Generate collision primitives...") => self.collision_primitives(),
            (_, _, "Wind wizard...") => {
                self.wind_on = true;
                self.prefs_page = 2;
                self.prefs_open = true;
            }
            (_, _, "Growth wizard...") => info("Growth wizard", "TreeCraft trees don't grow over time yet, so there's no growth animation to set up."),
            (_, "Arbitrary scene scale...", f) => {
                let k: f32 = f.trim_start_matches("x ").parse().unwrap_or(1.0);
                self.scale_scene(k, &format!("Scene scaled {}", f));
            }
            (_, "Scene unit conversion...", what) => {
                let k = match what {
                    "Feet to meters" => 0.3048,
                    "Meters to feet" => 1.0 / 0.3048,
                    "Inches to centimeters" => 2.54,
                    "Centimeters to inches" => 1.0 / 2.54,
                    "Meters to centimeters" => 100.0,
                    _ => 0.01,
                };
                self.scale_scene(k, what);
            }
            (_, _, "Create collection from selection") => self.collection_from_selection(sel_node),
            (_, _, "Remove unused assets") => self.remove_unused_assets(),
            (_, _, "Reset lightmap scalars") => self.set_status("TreeCraft materials have no lightmap scalars; nothing to reset"),
            (_, _, "Switch to Substances where possible") => info("Substances", "TreeCraft doesn't read Substance materials; every material keeps its pictures."),
            (_, _, "Cascade tree windows") | (_, _, "Tile tree windows") => self.set_status("TreeCraft shows one tree window at a time"),
            (_, _, "Properties") => {
                self.layout_mode = self.layout_mode.min(1);
                self.select(Sel::None);
            }
            (_, _, "Generation editor") => self.tab = 0,
            (_, _, "Materials") | (_, _, "Material sets") => self.tab = 1,
            (_, _, "Meshes") => self.tab = 2,
            (_, _, w @ ("Displacements" | "Masks" | "Output" | "Timeline")) => info(w, &format!("TreeCraft has no {} window yet.", w)),
            (_, "Layout", m) => {
                self.layout_mode = match m {
                    "Normal" => 0,
                    "Full screen" => 1,
                    _ => 2,
                };
                set_fullscreen(self.layout_mode != 0);
            }
            (_, _, "Reset layout...") => {
                self.left_w = None;
                self.dock_w = None;
                self.layout_mode = 0;
                self.tab = 0;
                set_fullscreen(false);
            }
            // the tips card in the viewport (picture 43)
            (_, _, "User manual...") | (_, _, "Controls") => self.help_open = true,
            (_, _, "About TreeCraft...") => info("About TreeCraft", "TreeCraft — a tree modeler built with Rust and macroquad.\nIt works like SpeedTree Modeler but is an independent program."),
            (_, _, w) if WEB_ITEMS.contains(&w) => info(w.trim_end_matches("..."), "TreeCraft is an independent program with no website or online service, so this item opens nothing."),
            _ => {}
        }
        false
    }

    fn save_incremental(&mut self) {
        // the original's messages (fn 0x14041c880)
        match self.tree_path.clone() {
            Some(p) => {
                let next = next_incremental(&p);
                if next.exists() {
                    self.set_status("Incremented file already exists. File not saved.");
                } else {
                    self.save_tree_to(next);
                }
            }
            None => self.set_status("Cannot save incremental version with no current save file."),
        }
    }

    /// File > Save as with assets: copy every picture and mesh file the tree uses into
    /// "<name>_assets" beside the tree file, point the tree at the copies, then save.
    pub(crate) fn save_with_assets(&mut self) {
        let Some(path) = crate::prefs::file_dialog().set_title("Save tree with assets").add_filter("TreeCraft tree", &["tctree"]).set_file_name("tree.tctree").save_file() else { return };
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "tree".into());
        let dir = path.with_file_name(format!("{}_assets", stem));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.set_status(&format!("Couldn't make {}: {}", dir.display(), e));
            return;
        }
        let mut copied = 0;
        let mut copy = |f: &mut String| {
            if f.is_empty() {
                return;
            }
            let src = PathBuf::from(&*f);
            let Some(name) = src.file_name() else { return };
            let dst = dir.join(name);
            if src != dst && std::fs::copy(&src, &dst).is_ok() {
                *f = dst.display().to_string();
                copied += 1;
            }
        };
        for m in &mut self.materials {
            copy(&mut m.file);
            for sl in &mut m.slots {
                copy(&mut sl.file);
            }
        }
        for m in &mut self.meshes {
            for lod in &mut m.lods {
                copy(&mut lod.file);
            }
        }
        if self.save_tree_to(path) {
            self.set_status(&format!("Saved with {} asset file(s) in {}", copied, dir.display()));
        }
    }

    /// File > Export selected as mesh: the selected generator and the ones under it.
    fn export_selected(&mut self, sel: Option<u32>) {
        let Some(id) = sel else {
            self.set_status("Select a generator first");
            return;
        };
        let mut only: HashSet<u32> = HashSet::from([id]);
        loop {
            let more: Vec<u32> = self.nodes.iter().filter(|n| n.parent.map_or(false, |p| only.contains(&p)) && !only.contains(&n.id)).map(|n| n.id).collect();
            if more.is_empty() {
                break;
            }
            only.extend(more);
        }
        let name = self.node(id).map(|n| n.name.clone()).unwrap_or_default();
        let Some(path) = crate::prefs::file_dialog().set_title("Export selected as mesh").add_filter("Wavefront OBJ", &["obj"]).set_file_name(&format!("{}.obj", name)).save_file() else { return };
        let geo = self.generate_with(Some(only));
        let gpu = crate::render::upload(&geo);
        let s = self.obj_text(&gpu);
        match std::fs::write(&path, s) {
            Ok(()) => self.set_status(&format!("Exported \"{}\" to {}", name, path.display())),
            Err(e) => self.set_status(&format!("Export failed: {}", e)),
        }
    }

    fn export_material(&mut self) {
        let Some(m) = self.materials.get(self.mat_cur) else {
            self.set_status("There's no material to export");
            return;
        };
        let text = crate::treefile::material_text(m);
        let name = m.name.clone();
        let Some(path) = crate::prefs::file_dialog().set_title("Export material").add_filter("TreeCraft material", &["tcmat"]).set_file_name(&format!("{}.tcmat", name)).save_file() else { return };
        match std::fs::write(&path, text) {
            Ok(()) => self.set_status(&format!("Exported material \"{}\"", name)),
            Err(e) => self.set_status(&format!("Export failed: {}", e)),
        }
    }

    fn import_material(&mut self) {
        let Some(path) = crate::prefs::file_dialog().set_title("Import material asset").add_filter("TreeCraft material", &["tcmat"]).pick_file() else { return };
        let Some(mut m) = std::fs::read_to_string(&path).ok().and_then(|t| crate::treefile::material_from_text(&t)) else {
            self.set_status("That isn't a TreeCraft material file");
            return;
        };
        let names: Vec<String> = self.materials.iter().map(|x| x.name.clone()).collect();
        m.name = crate::meshdialog::unique_name_in(&names, &m.name);
        crate::textures::refresh_material(&mut m);
        self.materials.push(m);
        self.tab = 1;
        self.select(Sel::Material(self.materials.len() - 1));
        self.set_status("Imported material");
    }

    /// File > Export world building data: the tree's size and its collision objects as text.
    fn export_world_data(&mut self) {
        let Some(path) = crate::prefs::file_dialog().set_title("Export world building data").add_filter("Text", &["txt"]).set_file_name("tree_world.txt").save_file() else { return };
        let (a, b) = self.bounds;
        let mut s = String::from("# TreeCraft world building data\n");
        s.push_str(&format!("height={}\nwidth_x={}\nwidth_z={}\nbounds_min={},{},{}\nbounds_max={},{},{}\n", b.y - a.y, b.x - a.x, b.z - a.z, a.x, a.y, a.z, b.x, b.y, b.z));
        let (v, t, _, _) = self.geo_stats;
        s.push_str(&format!("vertices={}\ntriangles={}\n", v, t));
        for o in self.scene.iter().filter(|o| !o.kind.is_force()) {
            s.push_str(&format!("collision={} {} pos={},{},{} axis={},{},{} radius={} length={}\n", o.kind.name(), o.name, o.pos.x, o.pos.y, o.pos.z, o.dir.x, o.dir.y, o.dir.z, o.size, o.length));
        }
        match std::fs::write(&path, s) {
            Ok(()) => self.set_status(&format!("Exported {}", path.display())),
            Err(e) => self.set_status(&format!("Export failed: {}", e)),
        }
    }

    /// File > Export image / Render photo: the viewport as a PNG after the next frame (a photo
    /// hides the viewport's own controls).
    fn export_image(&mut self, photo: bool) {
        let title = if photo { "Render photo" } else { "Export image" };
        let Some(path) = crate::prefs::file_dialog().set_title(title).add_filter("PNG image", &["png"]).set_file_name(if photo { "photo.png" } else { "tree.png" }).save_file() else { return };
        self.shot_queue.push((path, None, photo));
    }

    /// File > Export image sequence: 36 pictures turning once around the tree.
    fn export_sequence(&mut self) {
        let Some(dir) = crate::prefs::file_dialog().set_title("Export image sequence (pick a folder)").pick_folder() else { return };
        let yaw0 = self.cam.yaw;
        for k in 0..36 {
            self.shot_queue.push((dir.join(format!("tree_{:03}.png", k)), Some(yaw0 + k as f32 * std::f32::consts::TAU / 36.0), true));
        }
        // back to where the camera was
        self.shot_queue.push((PathBuf::new(), Some(yaw0), false));
    }

    pub(crate) fn undo_selection(&mut self) {
        let Some(s) = self.sel_back.pop() else {
            self.set_status("No earlier selection");
            return;
        };
        let (back, mut fwd) = (std::mem::take(&mut self.sel_back), std::mem::take(&mut self.sel_fwd));
        fwd.push(self.sel);
        self.select(s);
        self.sel_back = back;
        self.sel_fwd = fwd;
    }

    pub(crate) fn redo_selection(&mut self) {
        let Some(s) = self.sel_fwd.pop() else {
            self.set_status("No later selection");
            return;
        };
        let fwd = std::mem::take(&mut self.sel_fwd);
        self.select(s);
        self.sel_fwd = fwd;
    }

    /// Tools > Generate collision primitives: capsules along the trunk's bones.
    fn collision_primitives(&mut self) {
        let Some(chain) = self.geo.as_ref().and_then(|g| g.bones.first()).filter(|c| c.len() >= 2).cloned() else {
            self.set_status("Add a trunk first");
            return;
        };
        let r = self.nodes.iter().find(|n| n.kind == Kind::Trunk).map(|n| n.p.radius_abs).unwrap_or(0.5).max(0.05);
        let parts = 3.min(chain.len() - 1);
        self.scene.retain(|o| !o.name.starts_with("Trunk capsule"));
        for k in 0..parts {
            let i0 = k * (chain.len() - 1) / parts;
            let i1 = (k + 1) * (chain.len() - 1) / parts;
            let (a, b) = (chain[i0], chain[i1]);
            let len = (b - a).length();
            if len < 1e-4 {
                continue;
            }
            let shrink = 1.0 - 0.3 * k as f32 / parts as f32;
            self.scene.push(SceneObj { kind: SceneKind::Capsule, name: format!("Trunk capsule {}", k + 1), pos: (a + b) * 0.5, dir: (b - a) / len, size: r * shrink, length: len });
        }
        self.show_coll = true;
        self.set_status("Made collision capsules along the trunk");
    }

    /// Tools > Arbitrary scene scale / Scene unit conversion: every size times `k`.
    fn scale_scene(&mut self, k: f32, what: &str) {
        for n in &mut self.nodes {
            n.p.len_abs *= k;
            n.p.radius_abs *= k;
            n.p.tree_radius *= k;
            n.p.leaf_size *= k;
        }
        for o in &mut self.scene {
            o.pos *= k;
            if !o.kind.is_force() {
                o.size *= k;
                o.length *= k;
            }
        }
        self.dirty = true;
        self.regenerate();
        self.frame_tree();
        self.set_status(what);
    }

    fn collection_from_selection(&mut self, sel: Option<u32>) {
        let Some(id) = sel else {
            self.set_status("Select a generator first");
            return;
        };
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
            n.p.collect = true;
        }
        if !self.meshes.iter().any(|m| m.collection) {
            let mut m = LeafMesh::card("Collection");
            m.collection = true;
            self.meshes.push(m);
        }
        self.dirty = true;
        self.set_status("The selected generator now goes into the collection mesh (Meshes tab)");
    }

    /// Tools > Remove unused assets: materials no generator, mesh or material set uses.
    fn remove_unused_assets(&mut self) {
        let n = self.materials.len();
        let mut used = vec![false; n];
        let mut mark = |i: usize| {
            if i < n {
                used[i] = true;
            }
        };
        for nd in &self.nodes {
            mark(nd.p.material);
            mark(nd.p.leaf_material);
            for (k, _) in nd.p.mat_list.iter().chain(nd.p.ext_list.iter()) {
                mark(*k);
            }
        }
        for m in &self.meshes {
            mark(m.ref_material);
            mark(m.ref_material.wrapping_sub(1));
        }
        for m in &self.materials {
            for (k, _) in &m.set_members {
                mark(*k);
            }
        }
        let removed = used.iter().filter(|u| !**u).count();
        if removed == 0 {
            self.set_status("Every material is in use");
            return;
        }
        let mut remap = vec![NO_MATERIAL; n];
        let mut next = 0;
        for i in 0..n {
            if used[i] {
                remap[i] = next;
                next += 1;
            }
        }
        let fix = |v: &mut usize| {
            if *v < n {
                *v = remap[*v];
            }
        };
        for nd in &mut self.nodes {
            fix(&mut nd.p.material);
            fix(&mut nd.p.leaf_material);
            for (k, _) in nd.p.mat_list.iter_mut().chain(nd.p.ext_list.iter_mut()) {
                fix(k);
            }
        }
        let mut k = 0;
        self.materials.retain(|_| {
            k += 1;
            used[k - 1]
        });
        for m in &mut self.materials {
            m.set_members.retain(|(i, _)| *i < n && used[*i]);
            for (i, _) in &mut m.set_members {
                *i = remap[*i];
            }
        }
        self.mat_cur = 0;
        self.select(Sel::None);
        self.dirty = true;
        self.set_status(&format!("Removed {} unused material(s)", removed));
    }
}

/// Draw the menu bar and carry out what's chosen. Returns true for File > Exit.
pub fn menu_bar(app: &mut App, ui: &mut Ui, w: f32) -> bool {
    draw_rectangle(0.0, 0.0, w, 24.0, C_BG);
    let mut x = 4.0;
    let mut quit = false;
    for name in MENU_NAMES {
        let key = format!("menu.{}", name);
        let tw = ui.text_w(name, 13.0) + 16.0;
        let r = Rect::new(x, 1.0, tw, 22.0);
        if ui.tool_button(&key, r, name, ui.popup_open_for(&key)) {
            let tree = app.menu_tree(name);
            ui.open_menu(hid(&key), r, tree);
        }
        if let Some(path) = ui.take_menu_result(&key) {
            quit |= app.menu_command(name, &path);
        }
        x += tw;
    }
    quit
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incremental_names() {
        assert_eq!(next_incremental(Path::new("a/tree.tctree")), PathBuf::from("a/tree_001.tctree"));
        assert_eq!(next_incremental(Path::new("a/tree_007.tctree")), PathBuf::from("a/tree_008.tctree"));
        assert_eq!(next_incremental(Path::new("a/oak_v2.tctree")), PathBuf::from("a/oak_v2_001.tctree"));
    }
}
