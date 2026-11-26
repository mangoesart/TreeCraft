//! The Export tab (picture 68): the tree written as a mesh file in one of three formats, with
//! its options shown in the Properties panel.
//! - Autodesk FBX (.fbx): binary FBX 7.4 (the layout Blender, Maya, 3ds Max, Unity and Unreal read),
//!   one model per group with normals, UVs and its material (diffuse colour and texture file).
//! - Wavefront OBJ (.obj) with a .mtl beside it.
//! - SpeedTree XML (.xml): TreeCraft's reading of the original's "SpeedTreeRaw" layout. The element
//!   names are the original's (SpeedTreeRaw, Objects, Object, ParentID, Name, Points, Vertices,
//!   NormalX..., TexcoordU/V, Triangles, PointIndices, VertexIndices); how they nest is an estimate.
use crate::app::{App, Sel};
use crate::model::Kind;
use crate::props::Rows;
use macroquad::prelude::*;
use std::collections::HashSet;

pub const FILE_TYPES: [&str; 3] = ["Autodesk FBX (*.fbx)", "Wavefront OBJ (*.obj)", "SpeedTree XML (*.xml)"];
pub const GROUP_BY: [&str; 4] = ["Hierarchy", "Material", "Geometry type", "Generator"];
pub const UNITS: [(&str, f32); 5] = [("Meter", 1.0), ("Centimeter", 100.0), ("Millimeter", 1000.0), ("Inch", 39.370_08), ("Foot", 3.280_84)];
pub const AXES: [&str; 2] = ["Y-up (Maya, Unity)", "Z-up (3ds Max, Unreal, Blender)"];

#[derive(Clone, Debug)]
pub struct ExportOpts {
    pub file_type: usize,
    pub group_by: usize,
    pub geometry: bool,
    pub spines: bool,
    pub unit: usize,
    pub scale: f32,
    pub swap_yz: bool,
    pub flip: [bool; 3],
    pub flip_v: bool,
    pub copy_textures: bool,
    pub axis: usize,
    pub selected_only: bool,
}

impl Default for ExportOpts {
    fn default() -> Self {
        ExportOpts { file_type: 0, group_by: 1, geometry: true, spines: false, unit: 0, scale: 1.0, swap_yz: false, flip: [false; 3], flip_v: false, copy_textures: true, axis: 0, selected_only: false }
    }
}

/// One piece of the export: a named mesh with one material, and its parent piece (Hierarchy).
pub struct Group {
    pub name: String,
    pub material: usize,
    pub parent: Option<usize>,
    pub pos: Vec<Vec3>,
    pub nrm: Vec<Vec3>,
    pub uv: Vec<Vec2>,
    pub tris: Vec<[u32; 3]>,
    /// Polylines (branch spines), OBJ and XML only.
    pub lines: Vec<Vec<Vec3>>,
    /// PivotPainter2's UV set (channel 2): each vertex at its element's pixel; empty when off.
    pub uv2: Vec<Vec2>,
}

fn names(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// The Export tab's panel.
pub fn export_rows(r: &mut Rows, app: &mut App) {
    let mut o = app.export_opts.clone();
    if r.group("Export Mesh") {
        r.combo("ex.type", "File type", &names(&FILE_TYPES), &mut o.file_type);
        let sel_name = if let Sel::Node(id) = app.sel { app.node(id).map(|n| n.name.clone()) } else { None };
        if let Some(n) = &sel_name {
            r.b("ex.sel", &format!("Only {} and below", n), &mut o.selected_only);
        } else {
            o.selected_only = false;
        }
    }
    if r.group("Grouping") {
        r.combo("ex.group", "By", &names(&GROUP_BY), &mut o.group_by);
    }
    if r.group("Include") {
        r.b("ex.geo", "Geometry", &mut o.geometry);
        if o.file_type == 0 {
            r.b_faint("ex.spines", "Branch spines (OBJ and XML)", &mut o.spines);
        } else {
            r.b("ex.spines", "Branch spines", &mut o.spines);
        }
    }
    if r.group("Transform") {
        r.info("Scene unit", "Meter");
        let mut u = o.unit;
        r.combo("ex.unit", "Convert to unit", &UNITS.iter().map(|u| u.0.to_string()).collect::<Vec<_>>(), &mut u);
        o.unit = u.min(UNITS.len() - 1);
        r.f("ex.scale", "+Scale", &mut o.scale, 0.01, 100.0, 2);
        o.scale = o.scale.max(1e-4);
        r.b("ex.swap", "Swap YZ", &mut o.swap_yz);
        r.b("ex.fx", "Flip X", &mut o.flip[0]);
        r.b("ex.fy", "Flip Y", &mut o.flip[1]);
        r.b("ex.fz", "Flip Z", &mut o.flip[2]);
        r.b("ex.fv", "Flip V texcoord", &mut o.flip_v);
    }
    if r.group("Textures") {
        r.b("ex.copy", "Copy textures beside the file", &mut o.copy_textures);
    }
    if o.file_type == 0 && r.group("Format-specific") {
        r.combo("ex.axis", "Axis", &names(&AXES), &mut o.axis);
    }
    let stats = app.geo.as_ref().map(|g| (g.tris, g.verts)).unwrap_or((0, 0));
    r.info("Triangles", &stats.0.to_string());
    r.info("Vertices", &stats.1.to_string());
    app.export_opts = o;
    if r.button("ex.go", "Export...") {
        app.export_mesh();
    }
}

impl App {
    /// The Export tab's Export button: picks a file and writes it.
    pub fn export_mesh(&mut self) {
        let o = self.export_opts.clone();
        let (filter, ext) = match o.file_type {
            0 => ("Autodesk FBX", "fbx"),
            1 => ("Wavefront OBJ", "obj"),
            _ => ("SpeedTree XML", "xml"),
        };
        let stem = self.tree_path.as_ref().and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string())).unwrap_or_else(|| "tree".into());
        let Some(path) = crate::prefs::file_dialog().set_title("Export mesh").add_filter(filter, &[ext]).set_file_name(&format!("{}.{}", stem, ext)).save_file() else { return };
        let path = if path.extension().is_none() { path.with_extension(ext) } else { path };
        match self.write_export(&path, &o) {
            Ok(n) => self.set_status(&format!("Exported {} ({} part{})", path.display(), n, if n == 1 { "" } else { "s" })),
            Err(e) => self.set_status(&format!("Export failed: {}", e)),
        }
    }

    /// Writes the export to `path`; returns how many parts it holds.
    pub fn write_export(&self, path: &std::path::Path, o: &ExportOpts) -> Result<usize, String> {
        let groups = self.export_groups(o);
        if groups.is_empty() {
            return Err("nothing to export (no geometry)".into());
        }
        // textures beside the file: their file names only, copied when asked
        let dir = path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
        let mut tex_names: std::collections::HashMap<usize, String> = Default::default();
        for g in &groups {
            if let Some(m) = self.materials.get(g.material) {
                if !m.file.is_empty() && !tex_names.contains_key(&g.material) {
                    let src = std::path::Path::new(&m.file);
                    let name = src.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    if o.copy_textures && src.exists() {
                        let _ = std::fs::copy(src, dir.join(&name));
                        tex_names.insert(g.material, name);
                    } else {
                        tex_names.insert(g.material, m.file.clone());
                    }
                }
            }
        }
        let mats: Vec<(usize, String, [f32; 3], Option<String>)> = {
            let mut seen: Vec<(usize, String, [f32; 3], Option<String>)> = vec![];
            for g in &groups {
                if !seen.iter().any(|(m, ..)| *m == g.material) {
                    let (name, col) = match self.materials.get(g.material) {
                        Some(m) => (m.name.clone(), m.color),
                        None => ("Default".to_string(), [0.6, 0.6, 0.6]),
                    };
                    seen.push((g.material, name, col, tex_names.get(&g.material).cloned()));
                }
            }
            seen
        };
        let bytes = match o.file_type {
            0 => fbx_binary(&groups, &mats, o.axis == 1),
            1 => {
                let mtl = path.with_extension("mtl");
                let mtl_name = mtl.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                std::fs::write(&mtl, mtl_text(&mats)).map_err(|e| e.to_string())?;
                obj_text(&groups, &mats, &mtl_name).into_bytes()
            }
            _ => xml_text(&groups, &mats).into_bytes(),
        };
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
        // PivotPainter2: its textures beside the FBX, named after it
        if self.pp2.with_fbx && o.file_type == 0 {
            let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "tree".into());
            crate::pivotpainter::write_textures(&dir, &stem, &self.pp2_elements(o), &self.pp2)?;
        }
        Ok(groups.len())
    }

    /// PivotPainter2's elements for the whole tree, moved into Unreal's space the way the export
    /// moves the mesh: the export's own transform (unit, +Scale, swaps, flips), then the FBX's axis
    /// setting (Y-up: X, Z, Y; Z-up: X, -Y, Z) and metres to centimetres.
    pub fn pp2_elements(&self, o: &ExportOpts) -> Vec<crate::pivotpainter::PPOut> {
        let full = self.generate_with(None);
        let tf = Xform::new(o);
        let to_ue = |v: Vec3| if o.axis == 1 { vec3(v.x, -v.y, v.z) } else { vec3(v.x, v.z, v.y) };
        let idx: std::collections::HashMap<(u32, usize), usize> = full.pp_elems.iter().enumerate().map(|(i, e)| (e.key, i)).collect();
        full.pp_elems
            .iter()
            .enumerate()
            .map(|(i, e)| {
                // a steady random value per element, from its key
                let mut hsh = (e.key.0 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (e.key.1 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
                hsh ^= hsh >> 31;
                crate::pivotpainter::PPOut {
                    pivot: to_ue(tf.p(e.pivot)) * 100.0,
                    x: to_ue(tf.n(e.x)),
                    y: to_ue(tf.n(e.y)),
                    z: to_ue(tf.n(e.x.cross(e.y))),
                    ext: vec3(e.len, e.width, e.width) * tf.k * 100.0,
                    parent: e.parent.and_then(|p| idx.get(&p).copied()).unwrap_or(i),
                    depth: e.depth,
                    random: (hsh % 10_000) as f32 / 9_999.0,
                }
            })
            .collect()
    }

    /// PivotPainter2 > Bake textures only: the textures into a chosen folder, named after the tree.
    pub fn pp2_bake_only(&mut self) {
        let Some(dir) = crate::prefs::file_dialog().set_title("PivotPainter2: folder for the textures").pick_folder() else { return };
        let stem = self.tree_path.as_ref().and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string())).unwrap_or_else(|| "tree".into());
        let o = ExportOpts { file_type: 0, ..self.export_opts.clone() };
        let els = self.pp2_elements(&o);
        match crate::pivotpainter::write_textures(&dir, &stem, &els, &self.pp2) {
            Ok(files) => self.set_status(&format!("PivotPainter2: baked {} element(s) into {}", els.len(), files.join(", "))),
            Err(e) => self.set_status(&format!("PivotPainter2: {}", e)),
        }
    }

    /// The tree cut into groups by the chosen Grouping, transformed as asked.
    pub fn export_groups(&self, o: &ExportOpts) -> Vec<Group> {
        // which generators take part
        let mut keep: HashSet<u32> = self.nodes.iter().filter(|n| !n.hidden).map(|n| n.id).collect();
        if o.selected_only {
            if let Sel::Node(root) = self.sel {
                let mut set = HashSet::new();
                let mut stack = vec![root];
                while let Some(id) = stack.pop() {
                    set.insert(id);
                    stack.extend(self.nodes.iter().filter(|n| n.parent == Some(id)).map(|n| n.id));
                }
                keep.retain(|id| set.contains(id));
            }
        }
        let gens: Vec<(u32, String, Option<u32>, Kind)> = self.nodes.iter().filter(|n| keep.contains(&n.id) && n.kind != Kind::Tree).map(|n| (n.id, n.name.clone(), n.parent, n.kind)).collect();
        // the sets of generators drawn per piece, and each piece's name and parent piece
        let mut pieces: Vec<(String, HashSet<u32>, Option<u32>)> = vec![];
        match o.group_by {
            1 => pieces.push(("Tree".into(), gens.iter().map(|g| g.0).collect(), None)),
            2 => {
                let br: HashSet<u32> = gens.iter().filter(|g| g.3 != Kind::Leaf).map(|g| g.0).collect();
                let lf: HashSet<u32> = gens.iter().filter(|g| g.3 == Kind::Leaf).map(|g| g.0).collect();
                pieces.push(("Branches".into(), br, None));
                pieces.push(("Leaves".into(), lf, None));
            }
            _ => {
                for g in &gens {
                    pieces.push((g.1.clone(), [g.0].into_iter().collect(), Some(g.0)));
                }
            }
        }
        let tf = Xform::new(o);
        // PivotPainter2 (FBX only): every element's index in the whole tree, and the texture size
        let pp = if self.pp2.with_fbx && o.file_type == 0 {
            let full = self.generate_with(None);
            let idx: std::collections::HashMap<(u32, usize), usize> = full.pp_elems.iter().enumerate().map(|(i, e)| (e.key, i)).collect();
            let (w, h) = crate::pivotpainter::dims(full.pp_elems.len());
            Some((idx, w, h))
        } else {
            None
        };
        let mut out: Vec<Group> = vec![];
        let mut piece_of_gen: std::collections::HashMap<u32, Vec<usize>> = Default::default();
        for (name, set, gen_id) in pieces {
            if set.is_empty() {
                continue;
            }
            let geo = self.generate_with(Some(set.clone()));
            let mut mats: Vec<_> = geo.mats.iter().collect();
            mats.sort_by_key(|(k, _)| **k);
            let multi = mats.len() > 1;
            let first = out.len();
            if o.geometry {
                for (mi, mg) in mats {
                    let mut g = Group { name: if multi && o.group_by != 1 { format!("{}_{}", name, self.materials.get(*mi).map(|m| m.name.as_str()).unwrap_or("Default")) } else if o.group_by == 1 { self.materials.get(*mi).map(|m| m.name.clone()).unwrap_or_else(|| "Default".into()) } else { name.clone() }, material: *mi, parent: None, pos: vec![], nrm: vec![], uv: vec![], tris: vec![], lines: vec![], uv2: vec![] };
                    for (ci, ch) in mg.chunks.iter().enumerate() {
                        let base = g.pos.len() as u32;
                        // PivotPainter2: this chunk's element ranges
                        let ranges: Vec<(usize, usize, (u32, usize))> = if pp.is_some() { geo.pp_ranges.iter().filter(|r| r.0 == *mi && r.1 == ci).map(|r| (r.2, r.3, r.4)).collect() } else { vec![] };
                        for (vi, v) in ch.v.iter().enumerate() {
                            g.pos.push(tf.p(v.position));
                            g.nrm.push(tf.n(vec3(v.normal.x, v.normal.y, v.normal.z)));
                            g.uv.push(if o.flip_v { vec2(v.uv.x, 1.0 - v.uv.y) } else { v.uv });
                            if let Some((idx, w, h)) = &pp {
                                let e = ranges.iter().find(|r| vi >= r.0 && vi < r.1).and_then(|r| idx.get(&r.2)).copied().unwrap_or(0);
                                g.uv2.push(crate::pivotpainter::uv(e, *w, *h));
                            }
                        }
                        for t in ch.i.chunks(3) {
                            if t.len() == 3 {
                                let tri = [base + t[0] as u32, base + t[1] as u32, base + t[2] as u32];
                                g.tris.push(if tf.mirror { [tri[0], tri[2], tri[1]] } else { tri });
                            }
                        }
                    }
                    if !g.tris.is_empty() {
                        out.push(g);
                    }
                }
            }
            if o.spines && o.file_type != 0 {
                let lines: Vec<Vec<Vec3>> = geo.hits.iter().filter(|h| set.contains(&h.0) && h.2.len() > 1).map(|h| h.2.iter().map(|p| tf.p(*p)).collect()).collect();
                if !lines.is_empty() {
                    out.push(Group { name: format!("{}_spines", name), material: usize::MAX, parent: None, pos: vec![], nrm: vec![], uv: vec![], tris: vec![], lines, uv2: vec![] });
                }
            }
            if let Some(id) = gen_id {
                piece_of_gen.insert(id, (first..out.len()).collect());
            }
        }
        // Hierarchy: each generator's pieces hang under its parent generator's first piece
        if o.group_by == 0 {
            let parent_of: std::collections::HashMap<u32, Option<u32>> = gens.iter().map(|g| (g.0, g.2)).collect();
            for (id, idxs) in piece_of_gen.clone() {
                let mut p = parent_of.get(&id).copied().flatten();
                let mut target = None;
                while let Some(pid) = p {
                    if let Some(v) = piece_of_gen.get(&pid).and_then(|v| v.first()) {
                        target = Some(*v);
                        break;
                    }
                    p = parent_of.get(&pid).copied().flatten();
                }
                for i in idxs {
                    out[i].parent = target;
                }
            }
        }
        out
    }
}

/// Unit, scale, swap and flips.
struct Xform {
    k: f32,
    swap: bool,
    flip: [bool; 3],
    mirror: bool,
}

impl Xform {
    fn new(o: &ExportOpts) -> Self {
        let flips = o.flip.iter().filter(|f| **f).count() + o.swap_yz as usize;
        Xform { k: UNITS[o.unit.min(UNITS.len() - 1)].1 * o.scale, swap: o.swap_yz, flip: o.flip, mirror: flips % 2 == 1 }
    }
    fn axes(&self, v: Vec3) -> Vec3 {
        let mut v = if self.swap { vec3(v.x, v.z, v.y) } else { v };
        for (k, f) in self.flip.iter().enumerate() {
            if *f {
                v[k] = -v[k];
            }
        }
        v
    }
    fn p(&self, v: Vec3) -> Vec3 {
        self.axes(v) * self.k
    }
    fn n(&self, v: Vec3) -> Vec3 {
        self.axes(v).normalize_or_zero()
    }
}

fn clean(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect()
}

fn mat_name(mats: &[(usize, String, [f32; 3], Option<String>)], mi: usize) -> String {
    mats.iter().find(|m| m.0 == mi).map(|m| clean(&m.1)).unwrap_or_else(|| "Default".into())
}

pub fn mtl_text(mats: &[(usize, String, [f32; 3], Option<String>)]) -> String {
    use std::fmt::Write as _;
    let mut s = String::from("# TreeCraft materials\n");
    for (_, name, c, tex) in mats {
        let _ = writeln!(s, "\nnewmtl {}\nKd {} {} {}\nKa 0 0 0\nKs 0.1 0.1 0.1\nNs 20\nd 1", clean(name), c[0], c[1], c[2]);
        if let Some(t) = tex {
            let _ = writeln!(s, "map_Kd {}", t);
        }
    }
    s
}

pub fn obj_text(groups: &[Group], mats: &[(usize, String, [f32; 3], Option<String>)], mtl: &str) -> String {
    use std::fmt::Write as _;
    let mut s = format!("# TreeCraft export\nmtllib {}\n", mtl);
    let mut base = 1usize;
    for g in groups {
        let _ = writeln!(s, "o {}", clean(&g.name));
        if !g.tris.is_empty() {
            let _ = writeln!(s, "usemtl {}", mat_name(mats, g.material));
            for p in &g.pos {
                let _ = writeln!(s, "v {} {} {}", p.x, p.y, p.z);
            }
            for t in &g.uv {
                let _ = writeln!(s, "vt {} {}", t.x, 1.0 - t.y);
            }
            for n in &g.nrm {
                let _ = writeln!(s, "vn {} {} {}", n.x, n.y, n.z);
            }
            for t in &g.tris {
                let (a, b, c) = (base + t[0] as usize, base + t[1] as usize, base + t[2] as usize);
                let _ = writeln!(s, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}");
            }
            base += g.pos.len();
        }
        for l in &g.lines {
            for p in l {
                let _ = writeln!(s, "v {} {} {}", p.x, p.y, p.z);
            }
            let idx: Vec<String> = (0..l.len()).map(|k| (base + k).to_string()).collect();
            let _ = writeln!(s, "l {}", idx.join(" "));
            base += l.len();
        }
    }
    s
}

pub fn xml_text(groups: &[Group], mats: &[(usize, String, [f32; 3], Option<String>)]) -> String {
    use std::fmt::Write as _;
    let join = |v: &mut dyn Iterator<Item = String>| v.collect::<Vec<_>>().join(" ");
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<SpeedTreeRaw>\n");
    let _ = writeln!(s, "  <Materials Count=\"{}\">", mats.len());
    for (k, (_, name, c, tex)) in mats.iter().enumerate() {
        let _ = writeln!(s, "    <Material ID=\"{}\" Name=\"{}\" ColorR=\"{}\" ColorG=\"{}\" ColorB=\"{}\" TexFilename=\"{}\"/>", k, esc(name), c[0], c[1], c[2], esc(tex.as_deref().unwrap_or("")));
    }
    s.push_str("  </Materials>\n");
    let _ = writeln!(s, "  <Objects Count=\"{}\">", groups.len());
    for (k, g) in groups.iter().enumerate() {
        let mid = mats.iter().position(|m| m.0 == g.material).map(|v| v as i64).unwrap_or(-1);
        let _ = writeln!(s, "    <Object ID=\"{}\" ParentID=\"{}\" Name=\"{}\" MaterialID=\"{}\">", k + 1, g.parent.map(|p| p + 1).unwrap_or(0), esc(&g.name), mid);
        let mut pts = g.pos.clone();
        for l in &g.lines {
            pts.extend(l.iter().copied());
        }
        let _ = writeln!(s, "      <Points Count=\"{}\">", pts.len());
        let _ = writeln!(s, "        <X>{}</X>", join(&mut pts.iter().map(|p| p.x.to_string())));
        let _ = writeln!(s, "        <Y>{}</Y>", join(&mut pts.iter().map(|p| p.y.to_string())));
        let _ = writeln!(s, "        <Z>{}</Z>", join(&mut pts.iter().map(|p| p.z.to_string())));
        s.push_str("      </Points>\n");
        let _ = writeln!(s, "      <Vertices Count=\"{}\">", g.pos.len());
        for (tag, f) in [("NormalX", 0usize), ("NormalY", 1), ("NormalZ", 2)] {
            let _ = writeln!(s, "        <{0}>{1}</{0}>", tag, join(&mut g.nrm.iter().map(|n| n[f].to_string())));
        }
        let _ = writeln!(s, "        <TexcoordU>{}</TexcoordU>", join(&mut g.uv.iter().map(|t| t.x.to_string())));
        let _ = writeln!(s, "        <TexcoordV>{}</TexcoordV>", join(&mut g.uv.iter().map(|t| (1.0 - t.y).to_string())));
        s.push_str("      </Vertices>\n");
        let _ = writeln!(s, "      <Triangles Count=\"{}\">", g.tris.len());
        let flat = join(&mut g.tris.iter().flat_map(|t| t.iter().map(|i| i.to_string())));
        let _ = writeln!(s, "        <PointIndices>{}</PointIndices>", flat);
        let _ = writeln!(s, "        <VertexIndices>{}</VertexIndices>", flat);
        s.push_str("      </Triangles>\n");
        if !g.lines.is_empty() {
            let mut at = g.pos.len();
            let _ = writeln!(s, "      <Spines Count=\"{}\">", g.lines.len());
            for l in &g.lines {
                let _ = writeln!(s, "        <Spine>{}</Spine>", join(&mut (at..at + l.len()).map(|i| i.to_string())));
                at += l.len();
            }
            s.push_str("      </Spines>\n");
        }
        s.push_str("    </Object>\n");
    }
    s.push_str("  </Objects>\n</SpeedTreeRaw>\n");
    s
}

// ------------------------------------------------------------------ binary FBX 7.4

enum P {
    I32(i32),
    I64(i64),
    F64(f64),
    Str(String),
    Raw(Vec<u8>),
    ArrF64(Vec<f64>),
    ArrI32(Vec<i32>),
}

struct FNode {
    name: &'static str,
    props: Vec<P>,
    kids: Vec<FNode>,
}

fn n(name: &'static str, props: Vec<P>) -> FNode {
    FNode { name, props, kids: vec![] }
}
fn nk(name: &'static str, props: Vec<P>, kids: Vec<FNode>) -> FNode {
    FNode { name, props, kids }
}
fn s(v: &str) -> P {
    P::Str(v.to_string())
}

fn write_props(out: &mut Vec<u8>, props: &[P]) {
    for p in props {
        match p {
            P::I32(v) => {
                out.push(b'I');
                out.extend_from_slice(&v.to_le_bytes());
            }
            P::I64(v) => {
                out.push(b'L');
                out.extend_from_slice(&v.to_le_bytes());
            }
            P::F64(v) => {
                out.push(b'D');
                out.extend_from_slice(&v.to_le_bytes());
            }
            P::Str(v) => {
                out.push(b'S');
                out.extend_from_slice(&(v.len() as u32).to_le_bytes());
                out.extend_from_slice(v.as_bytes());
            }
            P::Raw(v) => {
                out.push(b'R');
                out.extend_from_slice(&(v.len() as u32).to_le_bytes());
                out.extend_from_slice(v);
            }
            P::ArrF64(v) => {
                out.push(b'd');
                out.extend_from_slice(&(v.len() as u32).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&((v.len() * 8) as u32).to_le_bytes());
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
            P::ArrI32(v) => {
                out.push(b'i');
                out.extend_from_slice(&(v.len() as u32).to_le_bytes());
                out.extend_from_slice(&0u32.to_le_bytes());
                out.extend_from_slice(&((v.len() * 4) as u32).to_le_bytes());
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
        }
    }
}

/// One node record (FBX 7.4: 32-bit offsets, a 13-byte empty record after nested lists).
fn write_node(out: &mut Vec<u8>, node: &FNode) {
    let start = out.len();
    out.extend_from_slice(&[0u8; 12]);
    out.push(node.name.len() as u8);
    out.extend_from_slice(node.name.as_bytes());
    let pstart = out.len();
    write_props(out, &node.props);
    let plen = out.len() - pstart;
    if !node.kids.is_empty() {
        for k in &node.kids {
            write_node(out, k);
        }
        out.extend_from_slice(&[0u8; 13]);
    }
    let end = out.len() as u32;
    out[start..start + 4].copy_from_slice(&end.to_le_bytes());
    out[start + 4..start + 8].copy_from_slice(&(node.props.len() as u32).to_le_bytes());
    out[start + 8..start + 12].copy_from_slice(&(plen as u32).to_le_bytes());
}

/// A Properties70 "P" entry.
fn pp(name: &str, ty: &str, label: &str, flags: &str, vals: Vec<P>) -> FNode {
    let mut props = vec![s(name), s(ty), s(label), s(flags)];
    props.extend(vals);
    n("P", props)
}

pub fn fbx_binary(groups: &[Group], mats: &[(usize, String, [f32; 3], Option<String>)], z_up: bool) -> Vec<u8> {
    let mut id: i64 = 1_000_000;
    let mut next = || {
        id += 1;
        id
    };
    let (up, front, coord) = if z_up { (2, 1, 0) } else { (1, 2, 0) };
    let header = nk(
        "FBXHeaderExtension",
        vec![],
        vec![n("FBXHeaderVersion", vec![P::I32(1003)]), n("FBXVersion", vec![P::I32(7400)]), n("Creator", vec![s("TreeCraft")])],
    );
    let global = nk(
        "GlobalSettings",
        vec![],
        vec![
            n("Version", vec![P::I32(1000)]),
            nk(
                "Properties70",
                vec![],
                vec![
                    pp("UpAxis", "int", "Integer", "", vec![P::I32(up)]),
                    pp("UpAxisSign", "int", "Integer", "", vec![P::I32(1)]),
                    pp("FrontAxis", "int", "Integer", "", vec![P::I32(front)]),
                    pp("FrontAxisSign", "int", "Integer", "", vec![P::I32(1)]),
                    pp("CoordAxis", "int", "Integer", "", vec![P::I32(coord)]),
                    pp("CoordAxisSign", "int", "Integer", "", vec![P::I32(1)]),
                    pp("OriginalUpAxis", "int", "Integer", "", vec![P::I32(up)]),
                    pp("OriginalUpAxisSign", "int", "Integer", "", vec![P::I32(1)]),
                    pp("UnitScaleFactor", "double", "Number", "", vec![P::F64(100.0)]),
                    pp("OriginalUnitScaleFactor", "double", "Number", "", vec![P::F64(100.0)]),
                ],
            ),
        ],
    );
    let mut objects = vec![];
    let mut conns = vec![];
    // materials and their textures
    let mut mat_ids: std::collections::HashMap<usize, i64> = Default::default();
    for (mi, name, c, tex) in mats {
        let mid = next();
        mat_ids.insert(*mi, mid);
        objects.push(nk(
            "Material",
            vec![P::I64(mid), s(&format!("{}\u{0}\u{1}Material", clean(name))), s("")],
            vec![
                n("Version", vec![P::I32(102)]),
                n("ShadingModel", vec![s("phong")]),
                n("MultiLayer", vec![P::I32(0)]),
                nk(
                    "Properties70",
                    vec![],
                    vec![
                        pp("DiffuseColor", "Color", "", "A", vec![P::F64(c[0] as f64), P::F64(c[1] as f64), P::F64(c[2] as f64)]),
                        pp("DiffuseFactor", "Number", "", "A", vec![P::F64(1.0)]),
                        pp("SpecularColor", "Color", "", "A", vec![P::F64(0.1), P::F64(0.1), P::F64(0.1)]),
                        pp("Shininess", "Number", "", "A", vec![P::F64(20.0)]),
                    ],
                ),
            ],
        ));
        if let Some(t) = tex {
            let tid = next();
            objects.push(nk(
                "Texture",
                vec![P::I64(tid), s(&format!("{}\u{0}\u{1}Texture", clean(name))), s("")],
                vec![
                    n("Type", vec![s("TextureVideoClip")]),
                    n("Version", vec![P::I32(202)]),
                    n("TextureName", vec![s(&format!("{}\u{0}\u{1}Texture", clean(name)))]),
                    n("Media", vec![s("")]),
                    n("FileName", vec![s(t)]),
                    n("RelativeFilename", vec![s(t)]),
                ],
            ));
            conns.push(n("C", vec![s("OP"), P::I64(tid), P::I64(mid), s("DiffuseColor")]));
        }
    }
    // one model and geometry per group
    let mut model_ids = vec![];
    for g in groups {
        let gid = next();
        let mid = next();
        model_ids.push(mid);
        if g.tris.is_empty() {
            // spines aren't written to FBX; an empty transform keeps the hierarchy
            objects.push(nk("Model", vec![P::I64(mid), s(&format!("{}\u{0}\u{1}Model", clean(&g.name))), s("Null")], vec![n("Version", vec![P::I32(232)])]));
            continue;
        }
        let verts: Vec<f64> = g.pos.iter().flat_map(|p| [p.x as f64, p.y as f64, p.z as f64]).collect();
        let mut pvi = Vec::with_capacity(g.tris.len() * 3);
        let mut nrm = Vec::with_capacity(g.tris.len() * 9);
        let mut uvi = Vec::with_capacity(g.tris.len() * 3);
        for t in &g.tris {
            pvi.push(t[0] as i32);
            pvi.push(t[1] as i32);
            pvi.push(-(t[2] as i32) - 1);
            for &k in t {
                let v = g.nrm[k as usize];
                nrm.extend_from_slice(&[v.x as f64, v.y as f64, v.z as f64]);
                uvi.push(k as i32);
            }
        }
        let uv: Vec<f64> = g.uv.iter().flat_map(|t| [t.x as f64, 1.0 - t.y as f64]).collect();
        let mut geo_kids = vec![
            n("Vertices", vec![P::ArrF64(verts)]),
            n("PolygonVertexIndex", vec![P::ArrI32(pvi)]),
            n("GeometryVersion", vec![P::I32(124)]),
            nk(
                "LayerElementNormal",
                vec![P::I32(0)],
                vec![n("Version", vec![P::I32(101)]), n("Name", vec![s("")]), n("MappingInformationType", vec![s("ByPolygonVertex")]), n("ReferenceInformationType", vec![s("Direct")]), n("Normals", vec![P::ArrF64(nrm)])],
            ),
            nk(
                "LayerElementUV",
                vec![P::I32(0)],
                vec![n("Version", vec![P::I32(101)]), n("Name", vec![s("UVMap")]), n("MappingInformationType", vec![s("ByPolygonVertex")]), n("ReferenceInformationType", vec![s("IndexToDirect")]), n("UV", vec![P::ArrF64(uv)]), n("UVIndex", vec![P::ArrI32(uvi.clone())])],
            ),
        ];
        // PivotPainter2: the second UV set (channel 2), each vertex at its element's pixel
        let has_uv2 = g.uv2.len() == g.pos.len() && !g.uv2.is_empty();
        if has_uv2 {
            let uv2: Vec<f64> = g.uv2.iter().flat_map(|t| [t.x as f64, 1.0 - t.y as f64]).collect();
            geo_kids.push(nk(
                "LayerElementUV",
                vec![P::I32(1)],
                vec![n("Version", vec![P::I32(101)]), n("Name", vec![s("PivotPainter")]), n("MappingInformationType", vec![s("ByPolygonVertex")]), n("ReferenceInformationType", vec![s("IndexToDirect")]), n("UV", vec![P::ArrF64(uv2)]), n("UVIndex", vec![P::ArrI32(uvi)])],
            ));
        }
        geo_kids.push(nk(
            "LayerElementMaterial",
            vec![P::I32(0)],
            vec![n("Version", vec![P::I32(101)]), n("Name", vec![s("")]), n("MappingInformationType", vec![s("AllSame")]), n("ReferenceInformationType", vec![s("IndexToDirect")]), n("Materials", vec![P::ArrI32(vec![0])])],
        ));
        geo_kids.push(nk(
            "Layer",
            vec![P::I32(0)],
            vec![
                n("Version", vec![P::I32(100)]),
                nk("LayerElement", vec![], vec![n("Type", vec![s("LayerElementNormal")]), n("TypedIndex", vec![P::I32(0)])]),
                nk("LayerElement", vec![], vec![n("Type", vec![s("LayerElementUV")]), n("TypedIndex", vec![P::I32(0)])]),
                nk("LayerElement", vec![], vec![n("Type", vec![s("LayerElementMaterial")]), n("TypedIndex", vec![P::I32(0)])]),
            ],
        ));
        if has_uv2 {
            geo_kids.push(nk("Layer", vec![P::I32(1)], vec![n("Version", vec![P::I32(100)]), nk("LayerElement", vec![], vec![n("Type", vec![s("LayerElementUV")]), n("TypedIndex", vec![P::I32(1)])])]));
        }
        objects.push(nk("Geometry", vec![P::I64(gid), s(&format!("{}\u{0}\u{1}Geometry", clean(&g.name))), s("Mesh")], geo_kids));
        objects.push(nk(
            "Model",
            vec![P::I64(mid), s(&format!("{}\u{0}\u{1}Model", clean(&g.name))), s("Mesh")],
            vec![n("Version", vec![P::I32(232)]), nk("Properties70", vec![], vec![pp("DefaultAttributeIndex", "int", "Integer", "", vec![P::I32(0)])]), n("Shading", vec![P::Raw(vec![1])]), n("Culling", vec![s("CullingOff")])],
        ));
        conns.push(n("C", vec![s("OO"), P::I64(gid), P::I64(mid)]));
        if let Some(m) = mat_ids.get(&g.material) {
            conns.push(n("C", vec![s("OO"), P::I64(*m), P::I64(mid)]));
        }
    }
    for (k, g) in groups.iter().enumerate() {
        let parent = g.parent.and_then(|p| model_ids.get(p).copied()).unwrap_or(0);
        conns.push(n("C", vec![s("OO"), P::I64(model_ids[k]), P::I64(parent)]));
    }
    let count = |name: &str| objects.iter().filter(|o| o.name == name).count() as i32;
    let defs = nk(
        "Definitions",
        vec![],
        vec![
            n("Version", vec![P::I32(100)]),
            n("Count", vec![P::I32(objects.len() as i32 + 1)]),
            nk("ObjectType", vec![s("GlobalSettings")], vec![n("Count", vec![P::I32(1)])]),
            nk("ObjectType", vec![s("Model")], vec![n("Count", vec![P::I32(count("Model"))])]),
            nk("ObjectType", vec![s("Geometry")], vec![n("Count", vec![P::I32(count("Geometry"))])]),
            nk("ObjectType", vec![s("Material")], vec![n("Count", vec![P::I32(count("Material"))])]),
            nk("ObjectType", vec![s("Texture")], vec![n("Count", vec![P::I32(count("Texture"))])]),
        ],
    );
    let top = vec![header, global, defs, nk("Objects", vec![], objects), nk("Connections", vec![], conns)];
    let mut out = Vec::with_capacity(1 << 16);
    out.extend_from_slice(b"Kaydara FBX Binary  \x00\x1a\x00");
    out.extend_from_slice(&7400u32.to_le_bytes());
    for t in &top {
        write_node(&mut out, t);
    }
    out.extend_from_slice(&[0u8; 13]);
    // footer
    out.extend_from_slice(&[0xfa, 0xbc, 0xab, 0x09, 0xd0, 0xc8, 0xd4, 0x66, 0xb1, 0x76, 0xfb, 0x83, 0x1c, 0xf7, 0x26, 0x7e]);
    out.extend_from_slice(&[0u8; 4]);
    let pad = ((out.len() + 15) & !15) - out.len();
    out.extend(std::iter::repeat(0u8).take(if pad == 0 { 16 } else { pad }));
    out.extend_from_slice(&7400u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 120]);
    out.extend_from_slice(&[0xf8, 0x5a, 0x8c, 0x6a, 0xde, 0xf5, 0xd9, 0x7e, 0xec, 0xe9, 0x0c, 0xe3, 0x75, 0x8f, 0x29, 0x0b]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tri_group() -> Vec<Group> {
        vec![Group { name: "Trunk".into(), material: 0, parent: None, pos: vec![Vec3::ZERO, Vec3::X, Vec3::Y], nrm: vec![Vec3::Z; 3], uv: vec![Vec2::ZERO, Vec2::X, Vec2::Y], tris: vec![[0, 1, 2]], lines: vec![], uv2: vec![] }]
    }

    /// The binary FBX reads back as a well-formed node tree: every record's end offset lands on the
    /// next record, and the top level ends with the empty record before the footer.
    #[test]
    fn fbx_records_are_well_formed() {
        let mats = vec![(0usize, "Bark".to_string(), [0.5, 0.4, 0.3], Some("bark.png".to_string()))];
        let b = fbx_binary(&tri_group(), &mats, false);
        assert_eq!(&b[0..21], b"Kaydara FBX Binary  \x00");
        fn walk(b: &[u8], mut at: usize, end: usize) -> usize {
            loop {
                let e = u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize;
                if e == 0 {
                    return at + 13;
                }
                assert!(e > at && e <= end, "bad end offset {} at {}", e, at);
                let plen = u32::from_le_bytes(b[at + 8..at + 12].try_into().unwrap()) as usize;
                let nl = b[at + 12] as usize;
                let kids = at + 13 + nl + plen;
                if kids < e {
                    let after = walk(b, kids, e);
                    assert_eq!(after, e, "nested list doesn't end at the record's end");
                }
                at = e;
            }
        }
        let after = walk(&b, 27, b.len());
        assert_eq!(&b[after..after + 4], &[0xfa, 0xbc, 0xab, 0x09]);
    }

    #[test]
    fn obj_and_xml_hold_the_triangle() {
        let mats = vec![(0usize, "Bark".to_string(), [0.5, 0.4, 0.3], None)];
        let o = obj_text(&tri_group(), &mats, "t.mtl");
        assert!(o.contains("f 1/1/1 2/2/2 3/3/3"));
        assert!(o.contains("usemtl Bark"));
        let x = xml_text(&tri_group(), &mats);
        assert!(x.contains("<SpeedTreeRaw>") && x.contains("<PointIndices>0 1 2</PointIndices>"));
    }
}
