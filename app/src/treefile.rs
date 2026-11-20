//! File > Save / Save As / Open (picture-21): the whole tree in a readable text file (.tctree):
//! the generators with all their settings (and the original's extra properties), materials (their
//! settings and picture files; pictures are read again from those files), leaf meshes (cutouts
//! with levels, anchors, colours and inner points, and geometry), and scene objects.
//! One `[section]` per item, then `key=value` lines.

use crate::app::{App, Sel};
use crate::model::*;
use crate::model::Material;
use macroquad::prelude::*;
use std::fmt::Write as _;

const HEADER: &str = "TreeCraft tree 1";

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n")
}
fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') => out.push('\n'),
                Some(o) => out.push(o),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}
fn fl(v: &[f32]) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}
fn nums(s: &str) -> Vec<f32> {
    s.split(',').filter_map(|x| x.trim().parse().ok()).collect()
}
fn c3(s: &str) -> [f32; 3] {
    let n = nums(s);
    [n.first().copied().unwrap_or(1.0), n.get(1).copied().unwrap_or(1.0), n.get(2).copied().unwrap_or(1.0)]
}
fn v2s(v: &[Vec2]) -> String {
    v.iter().map(|p| format!("{},{}", p.x, p.y)).collect::<Vec<_>>().join(";")
}
fn s2v(s: &str) -> Vec<Vec2> {
    s.split(';').filter(|x| !x.is_empty()).map(|x| { let n = nums(x); vec2(n.first().copied().unwrap_or(0.0), n.get(1).copied().unwrap_or(0.0)) }).collect()
}
fn c3s(v: &[[f32; 3]]) -> String {
    v.iter().map(|c| fl(c)).collect::<Vec<_>>().join(";")
}
fn s2c3(s: &str) -> Vec<[f32; 3]> {
    s.split(';').filter(|x| !x.is_empty()).map(c3).collect()
}

/// Write and read struct fields by name with one list per struct.
macro_rules! fields {
    ($w:ident, $r:ident, $T:ty, { $($f:ident : $k:ident),* $(,)? }) => {
        fn $w(o: &$T, s: &mut String, pre: &str) {
            $( let _ = writeln!(s, "{}{}={}", pre, stringify!($f), fields!(@put $k, o.$f)); )*
        }
        fn $r(o: &mut $T, key: &str, v: &str) -> bool {
            match key {
                $( stringify!($f) => { fields!(@get $k, o.$f, v); true } )*
                _ => false,
            }
        }
    };
    (@put f, $e:expr) => { $e.to_string() };
    (@put i, $e:expr) => { $e.to_string() };
    (@put u, $e:expr) => { $e.to_string() };
    (@put b, $e:expr) => { $e.to_string() };
    (@put s, $e:expr) => { esc(&$e) };
    (@put c, $e:expr) => { fl(&$e) };
    (@put v2, $e:expr) => { format!("{},{}", $e.x, $e.y) };
    (@put v3, $e:expr) => { format!("{},{},{}", $e.x, $e.y, $e.z) };
    (@get f, $e:expr, $v:ident) => { if let Ok(x) = $v.parse() { $e = x; } };
    (@get i, $e:expr, $v:ident) => { if let Ok(x) = $v.parse() { $e = x; } };
    (@get u, $e:expr, $v:ident) => { if let Ok(x) = $v.parse() { $e = x; } };
    (@get b, $e:expr, $v:ident) => { $e = $v == "true"; };
    (@get s, $e:expr, $v:ident) => { $e = unesc($v); };
    (@get c, $e:expr, $v:ident) => { $e = c3($v); };
    (@get v2, $e:expr, $v:ident) => { { let n = nums($v); if n.len() >= 2 { $e = vec2(n[0], n[1]); } } };
    (@get v3, $e:expr, $v:ident) => { { let n = nums($v); if n.len() >= 3 { $e = vec3(n[0], n[1], n[2]); } } };
}

fields!(put_params, get_params, Params, {
    gen_mode: u, frequency: f, number: i, interval: f, per_node: i, first: f, last: f, jumble: f, spiral: f,
    roll_var: f, balanced: b, offset: f, sweep: f, roll: f, relative: b, limit: i, size_scalar: f, seed: i,
    len_abs: f, len_pct: f, len_var: f, len_profile: f, start_angle: f, angle_var: f, angle_profile: f,
    gravity: f, noise_amt: f, noise_turb: f, jink: f, jink_freq: f, anchor_pick: u, radius_abs: f,
    radius_pct: f, radius_var: f, taper: f, flare: f, clamp_parent: b, seg_len: i, seg_radial: i,
    material: u, u_tile: f, v_tile: f, leaf_size: f, leaf_size_pct: f, leaf_size_var: f, leaf_mesh: u,
    leaf_material: u, face_sky: f, droop: f, fold: f, curl: f, leaf_twist: f, scale_x: f, scale_y: f,
    tree_radius: f, lod_preview: i, collect: b, cap_ends: b,
});

fields!(put_material, get_material, Material, {
    name: s, two_sided: b, color: c, source: u, file: s, brightness: f, contrast: f, saturation: f, hue: f,
    opacity_from_alpha: b, cutoff: f, normal_strength: f, gloss: f, specular_color: c, metallic: f,
    sss_color: c, sss_amount: f, ao: f, variation: f, cutout_mesh: u, opacity: f, custom_color: c,
    season_color: c, atlas_scale: f, atlas_mode: u, max_res: u, user_data: s, unwrap: u, is_set: b,
});

fields!(put_mesh, get_mesh, LeafMesh, {
    name: s, more: u, anchor: v2, ref_material: u, auto_points: i, threshold: f, margin: f, scale: f,
    orient: u, flip: u, flip_normals: b, fix_winding: b, collection: b, spine_style: u, pivot_style: u,
    growth: u, ripple: u, collision_scale: f, weight: f, embedded: b, tessellation: f, angle: f,
    double_sided: b, improve_folding: b,
});

fields!(put_scene, get_scene, SceneObj, { name: s, pos: v3, dir: v3, size: f, length: f });

fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Tree => "Tree",
        Kind::Trunk => "Trunk",
        Kind::Branch => "Branch",
        Kind::Leaf => "Leaf",
    }
}

pub fn write(app: &App) -> String {
    let mut s = format!("{}\n", HEADER);
    for n in &app.nodes {
        let _ = writeln!(s, "\n[node]\nid={}\nkind={}\nname={}\nparent={}\npos={},{}\nhidden={}", n.id, kind_name(n.kind), esc(&n.name), n.parent.map(|p| p.to_string()).unwrap_or_default(), n.pos.x, n.pos.y, n.hidden);
        put_params(&n.p, &mut s, "p.");
        for (k, v) in &n.p.extra {
            let _ = writeln!(s, "extra.{}={}", k, v);
        }
        // property curves, single-node offsets and hand-drawn branches (picture 24)
        for (k, c) in &n.p.curves {
            let _ = writeln!(s, "curve.{}={}", k, v2s(&c.pts));
            if c.kinds.iter().any(|x| *x != 0) {
                let _ = writeln!(s, "curvekind.{}={}", k, c.kinds.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
            }
        }
        for (k, l) in &n.p.curve_levels {
            let _ = writeln!(s, "curvelevel.{}={}", k, l);
        }
        for (k, v) in &n.p.variances {
            let _ = writeln!(s, "variance.{}={},{},{},{},{}", k, v.amount, v.gaussian as u8, v.cohesion, v.scale, v.offset);
        }
        for (i, e) in &n.p.node_edits {
            let _ = writeln!(s, "edit.{}={},{},{},{},{}", i, e.length, e.radius, e.angle, e.roll, e.hidden as u8);
        }
        for (key, list) in [("mats", &n.p.mat_list), ("exts", &n.p.ext_list)] {
            if !list.is_empty() {
                let _ = writeln!(s, "{}={}", key, list.iter().map(|(m, w)| format!("{}:{}", m, w)).collect::<Vec<_>>().join(","));
            }
        }
        for d in &n.p.drawn {
            let _ = writeln!(s, "drawn={}", d.iter().map(|p| format!("{},{},{}", p.x, p.y, p.z)).collect::<Vec<_>>().join(";"));
        }
    }
    for m in &app.materials {
        s.push_str("\n[material]\n");
        write_material(m, &mut s);
    }
    for m in &app.meshes {
        s.push_str("\n[mesh]\n");
        put_mesh(m, &mut s, "");
        for (l, lod) in m.lods.iter().enumerate() {
            let _ = writeln!(s, "lod{}.file={}", l, esc(&lod.file));
            let _ = writeln!(s, "lod{}.outline={}", l, v2s(&lod.outline));
            let _ = writeln!(s, "lod{}.anchors={}", l, lod.anchors.iter().map(|a| format!("{},{},{},{},{}", a.pos.x, a.pos.y, a.angle, a.id, a.side)).collect::<Vec<_>>().join(";"));
            let _ = writeln!(s, "lod{}.colors={}", l, c3s(&lod.colors));
            let _ = writeln!(s, "lod{}.inner={}", l, v2s(&lod.inner));
            let _ = writeln!(s, "lod{}.inner_colors={}", l, c3s(&lod.inner_colors));
            if let Some(g) = &lod.geom {
                let p3 = |v: &[Vec3]| v.iter().map(|p| format!("{},{},{}", p.x, p.y, p.z)).collect::<Vec<_>>().join(";");
                let _ = writeln!(s, "lod{}.geom.pos={}", l, p3(&g.pos));
                let _ = writeln!(s, "lod{}.geom.nrm={}", l, p3(&g.nrm));
                let _ = writeln!(s, "lod{}.geom.uv={}", l, v2s(&g.uv));
                let _ = writeln!(s, "lod{}.geom.idx={}", l, g.idx.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","));
                let _ = writeln!(s, "lod{}.geom.col={}", l, c3s(&g.col));
            }
        }
    }
    for o in &app.scene {
        let _ = writeln!(s, "\n[scene]\nkind={}", o.kind.name());
        put_scene(o, &mut s, "");
    }
    // custom cameras (picture 50)
    crate::cameras::write(&app.cameras, &mut s);
    // Window properties (picture 49)
    if !app.winp.is_empty() || !app.win_image_path.is_empty() {
        s.push_str("\n[window]\n");
        let _ = writeln!(s, "image={}", esc(&app.win_image_path));
        for (k, v) in &app.winp {
            let _ = writeln!(s, "{}={}", k, v);
        }
    }
    s
}

fn write_material(m: &Material, s: &mut String) {
    put_material(m, s, "");
    let _ = writeln!(s, "var_colors={}", c3s(&m.var_colors));
    let _ = writeln!(s, "mesh_rows={}", m.mesh_rows.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
    let _ = writeln!(s, "set_members={}", m.set_members.iter().map(|(k, w)| format!("{}:{}", k, w)).collect::<Vec<_>>().join(","));
    for (i, sl) in m.slots.iter().enumerate() {
        let _ = writeln!(s, "slot{}={}|{}", i, sl.enabled, esc(&sl.file));
    }
}

fn material_line(m: &mut Material, k: &str, v: &str) {
    if get_material(m, k, v) {
        return;
    }
    match k {
        "var_colors" => {
            let c = s2c3(v);
            for (i, x) in c.iter().take(3).enumerate() {
                m.var_colors[i] = *x;
            }
        }
        "mesh_rows" => m.mesh_rows = v.split(',').filter_map(|x| x.parse().ok()).collect(),
        "set_members" => m.set_members = v.split(',').filter_map(|x| { let (a, b) = x.split_once(':')?; Some((a.parse().ok()?, b.parse().ok()?)) }).collect(),
        _ => {
            if let Some(i) = k.strip_prefix("slot").and_then(|x| x.parse::<usize>().ok()) {
                if let (Some(sl), Some((en, f))) = (m.slots.get_mut(i), v.split_once('|')) {
                    sl.enabled = en == "true";
                    sl.file = unesc(f);
                }
            }
        }
    }
}

const MAT_HEADER: &str = "# TreeCraft material 1";

/// File > Export material: one material as text.
pub fn material_text(m: &Material) -> String {
    let mut s = format!("{}\n", MAT_HEADER);
    write_material(m, &mut s);
    s
}

/// File > Import material asset: a material written by material_text.
pub fn material_from_text(text: &str) -> Option<Material> {
    let mut lines = text.lines();
    if lines.next().map(|l| l.trim()) != Some(MAT_HEADER) {
        return None;
    }
    let mut m = Material::new("Material", 0);
    for line in lines {
        if let Some((k, v)) = line.trim_end_matches('\r').split_once('=') {
            material_line(&mut m, k, v);
        }
    }
    m.dirty_base = true;
    m.dirty_tex = true;
    Some(m)
}

fn s2v3(s: &str) -> Vec<Vec3> {
    s.split(';').filter(|x| !x.is_empty()).map(|x| { let n = nums(x); vec3(n.first().copied().unwrap_or(0.0), n.get(1).copied().unwrap_or(0.0), n.get(2).copied().unwrap_or(0.0)) }).collect()
}

/// Read a tree file into the app (replacing the current tree).
pub fn read(app: &mut App, text: &str) -> Result<(), String> {
    let mut lines = text.lines();
    if lines.next().map(|l| l.trim()) != Some(HEADER) {
        return Err("not a TreeCraft tree file".into());
    }
    let mut nodes: Vec<Node> = vec![];
    let mut mats: Vec<Material> = vec![];
    let mut meshes: Vec<LeafMesh> = vec![];
    let mut scene: Vec<SceneObj> = vec![];
    let mut winp: std::collections::BTreeMap<String, f64> = Default::default();
    let mut win_image = String::new();
    let mut cams: Vec<crate::cameras::ViewCam> = vec![];
    let mut section = "";
    for line in lines {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = match line {
                "[node]" => {
                    nodes.push(Node { id: 0, kind: Kind::Tree, name: String::new(), parent: None, pos: Vec2::ZERO, p: Params::new(Kind::Tree), hidden: false });
                    "node"
                }
                "[material]" => {
                    mats.push(Material::new("Material", 0));
                    "material"
                }
                "[mesh]" => {
                    meshes.push(LeafMesh::card("Mesh"));
                    "mesh"
                }
                "[scene]" => {
                    scene.push(SceneObj { kind: SceneKind::Sphere, name: String::new(), pos: Vec3::ZERO, dir: Vec3::Y, size: 1.0, length: 0.0 });
                    "scene"
                }
                "[window]" => "window",
                "[camera]" => {
                    cams.push(crate::cameras::blank());
                    "camera"
                }
                _ => "",
            };
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match section {
            "node" => {
                let Some(n) = nodes.last_mut() else { continue };
                match k {
                    "id" => n.id = v.parse().unwrap_or(0),
                    "kind" => {
                        n.kind = match v {
                            "Trunk" => Kind::Trunk,
                            "Branch" => Kind::Branch,
                            "Leaf" => Kind::Leaf,
                            _ => Kind::Tree,
                        };
                        n.p = Params::new(n.kind);
                    }
                    "name" => n.name = unesc(v),
                    "parent" => n.parent = v.parse().ok(),
                    "pos" => {
                        let x = nums(v);
                        if x.len() >= 2 {
                            n.pos = vec2(x[0], x[1]);
                        }
                    }
                    "hidden" => n.hidden = v == "true",
                    _ => {
                        if let Some(f) = k.strip_prefix("p.") {
                            get_params(&mut n.p, f, v);
                        } else if let Some(e) = k.strip_prefix("extra.") {
                            if let Ok(x) = v.parse() {
                                n.p.extra.insert(e.to_string(), x);
                            }
                        } else if let Some(c) = k.strip_prefix("curve.") {
                            n.p.curves.entry(c.to_string()).or_insert_with(Curve::flat).pts = s2v(v);
                        } else if let Some(c) = k.strip_prefix("curvekind.") {
                            n.p.curves.entry(c.to_string()).or_insert_with(Curve::flat).kinds = v.split(',').filter_map(|x| x.parse().ok()).collect();
                        } else if let Some(c) = k.strip_prefix("variance.") {
                            let x = nums(v);
                            if x.len() >= 5 {
                                n.p.variances.insert(c.to_string(), Variance { amount: x[0], gaussian: x[1] > 0.5, cohesion: x[2], scale: x[3], offset: x[4] });
                            }
                        } else if let Some(c) = k.strip_prefix("curvelevel.") {
                            if let Ok(l) = v.parse::<u8>() {
                                n.p.curve_levels.insert(c.to_string(), l);
                            }
                        } else if let Some(i) = k.strip_prefix("edit.").and_then(|i| i.parse::<usize>().ok()) {
                            let x = nums(v);
                            if x.len() >= 5 {
                                n.p.node_edits.insert(i, NodeEdit { length: x[0], radius: x[1], angle: x[2], roll: x[3], hidden: x[4] > 0.5 });
                            }
                        } else if k == "mats" || k == "exts" {
                            let list: Vec<(usize, f32)> = v.split(',').filter_map(|x| { let (a, b) = x.split_once(':')?; Some((a.parse().ok()?, b.parse().ok()?)) }).collect();
                            if k == "mats" { n.p.mat_list = list } else { n.p.ext_list = list }
                        } else if k == "drawn" {
                            n.p.drawn.push(s2v3(v));
                        }
                    }
                }
            }
            "material" => {
                let Some(m) = mats.last_mut() else { continue };
                material_line(m, k, v);
            }
            "mesh" => {
                let Some(m) = meshes.last_mut() else { continue };
                if get_mesh(m, k, v) {
                    continue;
                }
                let Some((l, field)) = k.strip_prefix("lod").and_then(|x| x.split_once('.')) else { continue };
                let Some(lod) = l.parse::<usize>().ok().and_then(|l| m.lods.get_mut(l)) else { continue };
                match field {
                    "file" => lod.file = unesc(v),
                    "outline" => lod.outline = s2v(v),
                    "anchors" => {
                        lod.anchors = v
                            .split(';')
                            .filter(|x| !x.is_empty())
                            .map(|x| {
                                let n = nums(x);
                                CutAnchor { pos: vec2(n.first().copied().unwrap_or(0.0), n.get(1).copied().unwrap_or(0.0)), angle: n.get(2).copied().unwrap_or(0.0), id: n.get(3).copied().unwrap_or(1.0) as usize, side: n.get(4).copied().unwrap_or(0.0) as u8 }
                            })
                            .collect()
                    }
                    "colors" => lod.colors = s2c3(v),
                    "inner" => lod.inner = s2v(v),
                    "inner_colors" => lod.inner_colors = s2c3(v),
                    g => {
                        let geom = lod.geom.get_or_insert_with(MeshGeom::default);
                        match g {
                            "geom.pos" => geom.pos = s2v3(v),
                            "geom.nrm" => geom.nrm = s2v3(v),
                            "geom.uv" => geom.uv = s2v(v),
                            "geom.idx" => geom.idx = v.split(',').filter_map(|x| x.parse().ok()).collect(),
                            "geom.col" => geom.col = s2c3(v),
                            _ => {}
                        }
                    }
                }
            }
            "scene" => {
                let Some(o) = scene.last_mut() else { continue };
                if k == "kind" {
                    o.kind = match v {
                        "Direction" => SceneKind::Direction,
                        "Magnet" => SceneKind::Magnet,
                        "Capsule" => SceneKind::Capsule,
                        _ => SceneKind::Sphere,
                    };
                } else {
                    get_scene(o, k, v);
                }
            }
            "camera" => {
                if let Some(c) = cams.last_mut() {
                    crate::cameras::read_line(c, k, v);
                }
            }
            "window" => {
                if k == "image" {
                    win_image = unesc(v);
                } else if let Ok(x) = v.parse() {
                    winp.insert(k.to_string(), x);
                }
            }
            _ => {}
        }
    }
    if !nodes.iter().any(|n| n.kind == Kind::Tree) {
        return Err("the file has no Tree generator".into());
    }
    // pictures are made again from each material's source or file
    for m in &mut mats {
        m.dirty_base = true;
        m.dirty_tex = true;
        // the other slots' pictures (Opacity, Normal ...) are read again from their files
        for (s, sl) in m.slots.iter_mut().enumerate() {
            if s == 0 || sl.file.is_empty() {
                continue;
            }
            if let Ok(img) = crate::textures::load_image_file(&sl.file) {
                let t = Texture2D::from_rgba8(img.w as u16, img.h as u16, &img.px);
                t.set_filter(FilterMode::Linear);
                crate::textures::set_repeat(&t);
                sl.tex = Some(t);
                sl.image = Some(img);
            }
        }
    }
    app.next_id = nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
    app.nodes = nodes;
    app.materials = mats;
    app.meshes = meshes;
    app.scene = scene;
    app.winp = winp;
    app.look_through(None);
    app.cameras = cams;
    app.set_win_image(&win_image);
    app.scene_sel = None;
    app.sel = Sel::None;
    app.mat_cur = 0;
    app.dirty = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A generator's settings come back the same after writing and reading them.
    #[test]
    fn generator_settings_round_trip() {
        let mut p = Params::new(Kind::Branch);
        p.frequency = 23.5;
        p.gen_mode = MODE_INTERVAL;
        p.balanced = false;
        p.seed = 4242;
        let mut s = String::new();
        put_params(&p, &mut s, "");
        let mut q = Params::new(Kind::Trunk);
        for line in s.lines() {
            let (k, v) = line.split_once('=').unwrap();
            assert!(get_params(&mut q, k, v), "unknown field {}", k);
        }
        assert_eq!(format!("{:?}", p), format!("{:?}", q));
        assert_eq!(unesc(&esc("a\nb\\c")), "a\nb\\c");
    }
}

impl App {
    /// File > Save: to the file it came from, or ask where (Save As).
    /// Returns true when the tree was written.
    pub fn save_tree(&mut self, ask: bool) -> bool {
        let path = match (&self.tree_path, ask) {
            (Some(p), false) => Some(p.clone()),
            _ => crate::prefs::file_dialog().set_title("Save tree").add_filter("TreeCraft tree", &["tctree"]).set_file_name("tree.tctree").save_file(),
        };
        let Some(path) = path else { return false };
        self.save_tree_to(path)
    }

    pub fn save_tree_to(&mut self, path: std::path::PathBuf) -> bool {
        match std::fs::write(&path, write(self)) {
            Ok(()) => {
                self.set_status(&format!("Saved {}", path.display()));
                self.prefs.add_recent(&path.display().to_string());
                self.tree_path = Some(path);
                self.mark_saved();
                true
            }
            Err(e) => {
                self.set_status(&format!("Couldn't save: {}", e));
                false
            }
        }
    }

    /// Before the tree is closed or replaced: when it has unsaved changes, ask
    /// "This tree has been modified, do you wish to save your changes?". Yes opens the save
    /// window; returns false when the person cancels (or the save is cancelled).
    pub fn confirm_discard(&mut self) -> bool {
        if !self.is_modified() {
            return true;
        }
        // the original's wording (fn 0x1402b3f80): the file's name when it has one, then
        // "Do you wish to save your changes?"; Yes saves, No closes without saving, Cancel stays
        let first = match self.tree_path.as_ref().and_then(|p| p.file_name()) {
            Some(n) => format!("The tree [{}] has been modified.", n.to_string_lossy()),
            None => "This tree has been modified.".to_string(),
        };
        let answer = rfd::MessageDialog::new()
            .set_title("TreeCraft")
            .set_description(format!("{}\n\nDo you wish to save your changes?", first))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => self.save_tree(false),
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    /// File > Open.
    pub fn open_tree(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        let Some(path) = crate::prefs::file_dialog().set_title("Open tree").add_filter("TreeCraft tree", &["tctree"]).pick_file() else { return };
        self.open_path(path);
    }

    /// Open a tree file (File > Recent files too).
    pub fn open_path(&mut self, path: std::path::PathBuf) {
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                self.set_status(&format!("Couldn't open: {}", e));
                self.prefs.remove_recent(&path.display().to_string());
                return;
            }
        };
        match read(self, &text) {
            Ok(()) => {
                // Preferences > General > Use multiple processors (picture 45)
                if self.prefs.multi_proc {
                    let paths: Vec<String> = self.materials.iter().flat_map(|m| std::iter::once(m.file.clone()).chain(m.slots.iter().map(|s| s.file.clone()))).collect();
                    crate::textures::preload(paths);
                }
                for m in &mut self.materials {
                    crate::textures::refresh_material(m);
                }
                self.regenerate();
                self.frame_tree();
                self.undo_reset();
                self.set_status(&format!("Opened {}", path.display()));
                self.prefs.add_recent(&path.display().to_string());
                self.tree_path = Some(path);
            }
            Err(e) => {
                self.set_status(&format!("Couldn't open: {}", e));
                self.prefs.remove_recent(&path.display().to_string());
            }
        }
    }

    /// Where the tree is kept when TreeCraft closes, for the welcome window's Recover Last
    /// Session (picture 37).
    pub fn session_path() -> Option<std::path::PathBuf> {
        let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME"))?;
        Some(std::path::PathBuf::from(base).join("TreeCraft").join("last_session.tctree"))
    }

    /// Preferences > General > Autosave (picture 45): every Interval minutes an unsaved tree is
    /// kept as the last session (Welcome > Recover Last Session brings it back); Notify in console
    /// says so in the status line and the console.
    pub fn autosave_tick(&mut self) {
        let now = get_time();
        if !self.prefs.autosave || self.last_autosave == 0.0 {
            self.last_autosave = now;
            return;
        }
        if now - self.last_autosave < self.prefs.autosave_min.max(0.5) as f64 * 60.0 {
            return;
        }
        self.last_autosave = now;
        if !self.is_modified() {
            return;
        }
        self.keep_session();
        if self.prefs.autosave_notify {
            let p = Self::session_path().map(|p| p.display().to_string()).unwrap_or_default();
            println!("TreeCraft autosaved to {}", p);
            self.set_status(&format!("Autosaved ({})", p));
        }
    }

    /// Keep the tree as it is now, to recover next time.
    pub fn keep_session(&self) {
        if let Some(p) = Self::session_path() {
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(p, write(self));
        }
    }

    /// Bring back the tree TreeCraft had when it last closed (unsaved, untitled).
    pub fn recover_session(&mut self) {
        let Some(p) = Self::session_path() else { return };
        let Ok(text) = std::fs::read_to_string(&p) else {
            self.set_status("There's no last session to recover yet");
            return;
        };
        if !self.confirm_discard() {
            return;
        }
        match read(self, &text) {
            Ok(()) => {
                for m in &mut self.materials {
                    crate::textures::refresh_material(m);
                }
                self.regenerate();
                self.frame_tree();
                self.undo_reset();
                self.tree_path = None;
                self.set_status("Recovered the last session");
            }
            Err(e) => self.set_status(&format!("Couldn't recover the last session: {}", e)),
        }
    }
}
