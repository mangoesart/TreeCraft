//! Mesh files (OBJ) and the processing the Meshes panel applies: cutout → geometry,
//! Orient, Flip, Flip normals, Fix winding, and the Growth axis used to place leaves.
use crate::model::*;
use crate::textures::triangulate;
use macroquad::prelude::*;
use std::collections::HashMap;

/// Largest mesh accepted for a leaf (each leaf copy must fit one draw batch).
pub const MAX_VERTS: usize = 20000;

fn obj_index(s: &str, len: usize) -> Option<usize> {
    let n: i64 = s.parse().ok()?;
    let i = if n < 0 { len as i64 + n } else { n - 1 };
    if i >= 0 && (i as usize) < len {
        Some(i as usize)
    } else {
        None
    }
}

/// Read a Wavefront OBJ file (v, vt, vn, f; polygons are fanned into triangles).
pub fn load_obj(path: &str) -> Result<MeshGeom, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let (mut v, mut vt, mut vn) = (vec![], vec![], vec![]);
    let mut out = MeshGeom::default();
    let mut map: HashMap<(usize, usize, usize), u32> = HashMap::new();
    let mut has_normals = true;
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let a: Vec<f32> = it.take(3).filter_map(|s| s.parse().ok()).collect();
                if a.len() == 3 {
                    v.push(vec3(a[0], a[1], a[2]));
                }
            }
            Some("vt") => {
                let a: Vec<f32> = it.take(2).filter_map(|s| s.parse().ok()).collect();
                if !a.is_empty() {
                    vt.push(vec2(a[0], 1.0 - a.get(1).copied().unwrap_or(0.0)));
                }
            }
            Some("vn") => {
                let a: Vec<f32> = it.take(3).filter_map(|s| s.parse().ok()).collect();
                if a.len() == 3 {
                    vn.push(vec3(a[0], a[1], a[2]).normalize_or_zero());
                }
            }
            Some("f") => {
                let mut face: Vec<u32> = vec![];
                for tok in it {
                    let parts: Vec<&str> = tok.split('/').collect();
                    let pi = match obj_index(parts[0], v.len()) {
                        Some(i) => i,
                        None => continue,
                    };
                    let ti = parts.get(1).and_then(|s| obj_index(s, vt.len())).map(|i| i + 1).unwrap_or(0);
                    let ni = parts.get(2).and_then(|s| obj_index(s, vn.len())).map(|i| i + 1).unwrap_or(0);
                    if ni == 0 {
                        has_normals = false;
                    }
                    let key = (pi, ti, ni);
                    let id = *map.entry(key).or_insert_with(|| {
                        out.pos.push(v[pi]);
                        out.uv.push(if ti > 0 { vt[ti - 1] } else { Vec2::ZERO });
                        out.nrm.push(if ni > 0 { vn[ni - 1] } else { Vec3::ZERO });
                        (out.pos.len() - 1) as u32
                    });
                    face.push(id);
                }
                for k in 1..face.len().saturating_sub(1) {
                    out.idx.extend_from_slice(&[face[0], face[k], face[k + 1]]);
                }
            }
            _ => {}
        }
        if out.pos.len() > MAX_VERTS {
            return Err(format!("mesh has more than {} vertices", MAX_VERTS));
        }
    }
    if out.idx.is_empty() {
        return Err("no faces found".into());
    }
    if !has_normals {
        compute_normals(&mut out);
    }
    Ok(out)
}

/// A file name's stem with trailing digits removed, then a trailing "lod" (any case) removed.
fn lod_base(stem: &str) -> (String, bool) {
    let s = stem.trim_end_matches(|c: char| c.is_ascii_digit());
    let lower = s.to_lowercase();
    if lower.ends_with("lod") {
        (s[..s.len() - 3].to_string(), true)
    } else {
        (s.to_string(), false)
    }
}

/// The original's "Attempt to find associated LOD meshes?" search (fn 0x1406003e0 case 8):
/// take the chosen file's name base (trailing digits and a trailing "lod" removed), list
/// "<base>*" mesh files in the same folder in name order, and return the files after the chosen
/// one whose names end in "lod" + digits. This build reads OBJ files only.
pub fn find_lod_files(chosen: &str) -> Vec<String> {
    let p = std::path::Path::new(chosen);
    let dir = match p.parent() {
        Some(d) => d,
        None => return vec![],
    };
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let (base, _) = lod_base(&stem);
    let base_l = base.to_lowercase();
    let mut files: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|q| q.extension().map(|x| x.to_string_lossy().eq_ignore_ascii_case("obj")).unwrap_or(false))
            .filter(|q| q.file_stem().map(|s| s.to_string_lossy().to_lowercase().starts_with(&base_l)).unwrap_or(false))
            .map(|q| q.to_string_lossy().to_string())
            .collect(),
        Err(_) => return vec![],
    };
    files.sort_by_key(|s| s.to_lowercase());
    let mut out = vec![];
    let mut seen_chosen = false;
    for f in files {
        if std::path::Path::new(&f) == p {
            seen_chosen = true;
            continue;
        }
        if seen_chosen {
            let st = std::path::Path::new(&f).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let (b, is_lod) = lod_base(&st);
            if is_lod && b.eq_ignore_ascii_case(&base) {
                out.push(f);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn improve_folding_puts_an_edge_on_the_fold_line() {
        use macroquad::prelude::vec2;
        // a diamond whose top and bottom corners sit on the fold line (straight up from the pivot)
        let d = [vec2(0.5, 0.0), vec2(1.0, 0.5), vec2(0.5, 1.0), vec2(0.0, 0.5)];
        let mut o = super::CutOpts::default();
        o.improve_folding = true;
        let g = super::outline_geom_col(&d, &[], vec2(0.5, 1.0), o);
        let has = g.idx.chunks(3).any(|t| t.contains(&0) && t.contains(&2));
        assert!(has, "the 0-2 diagonal along the fold line should be an edge: {:?}", g.idx);
    }

    #[test]
    fn tessellation_adds_t_squared_times_5000_points() {
        use macroquad::prelude::vec2;
        // an L shape (concave): every added point stays inside its outline
        let l = [vec2(0.1, 0.9), vec2(0.9, 0.9), vec2(0.9, 0.6), vec2(0.4, 0.6), vec2(0.4, 0.1), vec2(0.1, 0.1)];
        let mut o = super::CutOpts::default();
        o.tessellation = 0.1; // (0.1^2 * 5000) = 50 points
        let g = super::outline_geom_col(&l, &[], vec2(0.25, 0.9), o);
        assert_eq!(g.uv.len(), l.len() + 50);
        for p in &g.uv[l.len()..] {
            let in_l = p.x >= 0.1 - 1e-4 && p.x <= 0.9 + 1e-4 && p.y >= 0.1 - 1e-4 && p.y <= 0.9 + 1e-4 && !(p.x > 0.4 + 1e-4 && p.y < 0.6 - 1e-4);
            assert!(in_l, "point {:?} left the outline", p);
        }
    }

    #[test]
    fn finds_lod_files_after_the_chosen_one() {
        let dir = std::env::temp_dir().join(format!("treecraft_lodtest_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for n in ["Leaf_LOD0.obj", "Leaf_LOD1.obj", "Leaf_LOD2.obj", "Other.obj"] {
            std::fs::write(dir.join(n), "v 0 0 0\n").unwrap();
        }
        let chosen = dir.join("Leaf_LOD0.obj").to_string_lossy().to_string();
        let found = super::find_lod_files(&chosen);
        let names: Vec<String> = found.iter().map(|f| std::path::Path::new(f).file_name().unwrap().to_string_lossy().to_string()).collect();
        assert_eq!(names, vec!["Leaf_LOD1.obj", "Leaf_LOD2.obj"]);
        assert_eq!(super::lod_base("Leaf_LOD12"), ("Leaf_".to_string(), true));
    }
}

pub fn save_obj(path: &str, g: &MeshGeom) -> Result<(), String> {
    let mut s = String::from("# exported by TreeCraft\n");
    for p in &g.pos {
        s.push_str(&format!("v {} {} {}\n", p.x, p.y, p.z));
    }
    for t in &g.uv {
        s.push_str(&format!("vt {} {}\n", t.x, 1.0 - t.y));
    }
    for n in &g.nrm {
        s.push_str(&format!("vn {} {} {}\n", n.x, n.y, n.z));
    }
    for f in g.idx.chunks(3) {
        let (a, b, c) = (f[0] + 1, f[1] + 1, f[2] + 1);
        s.push_str(&format!("f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n"));
    }
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn compute_normals(g: &mut MeshGeom) {
    let mut n = vec![Vec3::ZERO; g.pos.len()];
    for f in g.idx.chunks(3) {
        let (a, b, c) = (f[0] as usize, f[1] as usize, f[2] as usize);
        let fnn = (g.pos[b] - g.pos[a]).cross(g.pos[c] - g.pos[a]);
        n[a] += fnn;
        n[b] += fnn;
        n[c] += fnn;
    }
    g.nrm = n.into_iter().map(|v| v.normalize_or_zero()).collect();
}

/// The cutout editor's Options (the original's "Options" group in the Edit Cutout window).
/// From the code (fns 0x1402f4120, 0x1402f83f0): Tessellation is a slider 0..10000 (page step
/// 100; 0..1 here), Angle is a thumb wheel (CThumbWheel, fn 0x140409be0, step 2), Double-sided and
/// Improve folding are check boxes. What each does to the mesh is partly estimated (see MECHANICS.md).
#[derive(Clone, Copy, Default)]
pub struct CutOpts {
    /// 0..1: subdivision passes, round(Tessellation x 3) (estimated); 0 = none.
    pub tessellation: f32,
    /// Degrees: direction of the leaf's growth line in the picture, turned from straight up
    /// (clockwise positive). The mesh is built along that line (estimated).
    pub angle: f32,
    /// Also make the back faces.
    pub double_sided: bool,
    /// Flip edges that cross the fold line (through the pivot along the Angle) so the mesh folds
    /// cleanly (the original's fn 0x1401a4ef0).
    pub improve_folding: bool,
}

impl CutOpts {
    pub fn of(m: &LeafMesh) -> Self {
        CutOpts { tessellation: m.tessellation, angle: m.angle, double_sided: m.double_sided, improve_folding: m.improve_folding }
    }
}

/// Area-weighted centroid of a closed outline (its mean point when the area is ~0).
fn outline_centroid(p: &[Vec2]) -> Vec2 {
    let n = p.len();
    let (mut a, mut c) = (0.0f32, Vec2::ZERO);
    for i in 0..n {
        let (u, v) = (p[i], p[(i + 1) % n]);
        let k = u.perp_dot(v);
        a += k;
        c += (u + v) * k;
    }
    if a.abs() < 1e-9 {
        return p.iter().copied().sum::<Vec2>() / n.max(1) as f32;
    }
    c / (3.0 * a)
}

fn in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let (d1, d2, d3) = ((b - a).perp_dot(p - a), (c - b).perp_dot(p - b), (a - c).perp_dot(p - c));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}

/// A cutout outline as geometry: x = across, y = from the anchor toward the tip, z = 0,
/// facing +Z. Triangles are split once so folding and curling bend smoothly.
pub fn outline_geom(outline: &[Vec2], anchor: Vec2) -> MeshGeom {
    outline_geom_opts(outline, anchor, CutOpts::default())
}

/// outline_geom with the cutout editor's Options.
pub fn outline_geom_opts(outline: &[Vec2], anchor: Vec2, o: CutOpts) -> MeshGeom {
    outline_geom_col(outline, &[], anchor, o)
}

fn avg_col(cs: &[[f32; 3]]) -> [f32; 3] {
    let n = cs.len().max(1) as f32;
    let mut s = [0.0f32; 3];
    for c in cs {
        for k in 0..3 {
            s[k] += c[k] / n;
        }
    }
    s
}

fn circum(a: Vec2, b: Vec2, c: Vec2) -> Option<(Vec2, f32)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        return None;
    }
    let (a2, b2, c2) = (a.length_squared(), b.length_squared(), c.length_squared());
    let p = vec2(
        (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    );
    Some((p, (p - a).length_squared()))
}

/// One tessellation step: a new point in the triangle with the largest circumcircle, at its
/// circumcentre when that lies inside it (the original's choice), otherwise at the middle of its
/// longest edge (splitting the neighbour too), so points never leave a concave outline.
fn circle_size(uvs: &[Vec2], f: [usize; 3]) -> f32 {
    circum(uvs[f[0]], uvs[f[1]], uvs[f[2]]).map(|c| c.1).unwrap_or(0.0)
}

fn refine_largest(uvs: &mut Vec<Vec2>, cols: &mut Vec<[f32; 3]>, faces: &mut Vec<[usize; 3]>, rad: &mut Vec<f32>, use_col: bool) {
    let mut best: Option<(usize, f32)> = None;
    for (k, &r) in rad.iter().enumerate() {
        if best.map_or(true, |b| r > b.1) {
            best = Some((k, r));
        }
    }
    let Some((k, _)) = best else { return };
    let f = faces[k];
    let (a, b, c) = (uvs[f[0]], uvs[f[1]], uvs[f[2]]);
    if let Some((p, _)) = circum(a, b, c) {
        if in_triangle(p, a, b, c) {
            faces.swap_remove(k);
            rad.swap_remove(k);
            uvs.push(p);
            if use_col {
                let col = avg_col(&[cols[f[0]], cols[f[1]], cols[f[2]]]);
                cols.push(col);
            }
            let i = uvs.len() - 1;
            for t in [[f[0], f[1], i], [f[1], f[2], i], [f[2], f[0], i]] {
                rad.push(circle_size(uvs, t));
                faces.push(t);
            }
            return;
        }
    }
    // longest edge (e0, e1); the opposite corner is o
    let e = (0..3).max_by(|&x, &y| {
        let lx = (uvs[f[(x + 1) % 3]] - uvs[f[x]]).length_squared();
        let ly = (uvs[f[(y + 1) % 3]] - uvs[f[y]]).length_squared();
        lx.partial_cmp(&ly).unwrap_or(std::cmp::Ordering::Equal)
    });
    let e = e.unwrap_or(0);
    let (e0, e1) = (f[e], f[(e + 1) % 3]);
    uvs.push((uvs[e0] + uvs[e1]) * 0.5);
    if use_col {
        let col = avg_col(&[cols[e0], cols[e1]]);
        cols.push(col);
    }
    let m = uvs.len() - 1;
    let n = faces.len();
    for k in 0..n {
        let t = faces[k];
        for j in 0..3 {
            let (p, q, r) = (t[j], t[(j + 1) % 3], t[(j + 2) % 3]);
            if (p == e0 && q == e1) || (p == e1 && q == e0) {
                let (t1, t2) = ([p, m, r], [m, q, r]);
                faces[k] = t1;
                rad[k] = circle_size(uvs, t1);
                faces.push(t2);
                rad.push(circle_size(uvs, t2));
                break;
            }
        }
    }
}

/// The original's Improve folding (fn 0x1401a4ef0): for an edge shared by two triangles whose ends
/// lie on opposite sides of the fold line (point c, direction d), flip it to the other diagonal
/// when |edge . d| < limit, the limit being |other diagonal . d| if that diagonal also crosses the
/// line and unlimited (3.4e38, constant 0x14102c810) if not, and only when the two triangles'
/// areas add up the same both ways (a convex pair, within 1e-5, constant 0x14103ecd8); repeat
/// until nothing flips.
fn fold_flips(uvs: &[Vec2], faces: &mut Vec<[usize; 3]>, c: Vec2, d: Vec2) {
    let d = d.normalize_or_zero();
    if d == Vec2::ZERO {
        return;
    }
    let side = |p: Vec2| d.y * (p.x - c.x) - d.x * (p.y - c.y);
    let crosses = |a: f32, b: f32| (a > 0.0 && b < 0.0) || (a < 0.0 && b > 0.0);
    let area = |a: usize, b: usize, e: usize| (uvs[b] - uvs[a]).perp_dot(uvs[e] - uvs[a]);
    for _ in 0..faces.len() * 4 + 16 {
        let mut edges: HashMap<(usize, usize), Vec<(usize, usize)>> = HashMap::new();
        for (k, t) in faces.iter().enumerate() {
            for j in 0..3 {
                let (a, b, opp) = (t[j], t[(j + 1) % 3], t[(j + 2) % 3]);
                edges.entry((a.min(b), a.max(b))).or_default().push((k, opp));
            }
        }
        let mut keys: Vec<_> = edges.keys().copied().collect();
        keys.sort_unstable();
        let mut flipped = false;
        // several flips per pass, each on triangles untouched so far in the pass
        let mut touched = vec![false; faces.len()];
        for (a, b) in keys {
            let v = &edges[&(a, b)];
            if v.len() != 2 || !crosses(side(uvs[a]), side(uvs[b])) {
                continue;
            }
            let ((k1, c1), (k2, c2)) = (v[0], v[1]);
            if touched[k1] || touched[k2] {
                continue;
            }
            let e = (uvs[b] - uvs[a]).normalize_or_zero();
            let f = (uvs[c2] - uvs[c1]).normalize_or_zero();
            let limit = if crosses(side(uvs[c1]), side(uvs[c2])) { f.dot(d).abs() } else { f32::MAX };
            if e.dot(d).abs() >= limit {
                continue;
            }
            let (t1, t2) = (faces[k1], faces[k2]);
            let before = area(t1[0], t1[1], t1[2]).abs() + area(t2[0], t2[1], t2[2]).abs();
            let after = area(c1, c2, a).abs() + area(c1, c2, b).abs();
            if (before - after).abs() >= 1e-5 {
                continue;
            }
            // keep the faces' winding
            let wind = area(t1[0], t1[1], t1[2]) >= 0.0;
            let fix = |t: [usize; 3]| if (area(t[0], t[1], t[2]) >= 0.0) == wind { t } else { [t[0], t[2], t[1]] };
            faces[k1] = fix([c1, c2, a]);
            faces[k2] = fix([c1, c2, b]);
            touched[k1] = true;
            touched[k2] = true;
            flipped = true;
        }
        if !flipped {
            break;
        }
    }
}

thread_local! {
    /// recent cutout meshes by their inputs: the editor draws the same outline several times a frame
    static GEOM_CACHE: std::cell::RefCell<Vec<(u64, MeshGeom)>> = std::cell::RefCell::new(Vec::new());
}

fn geom_key(outline: &[Vec2], colors: &[[f32; 3]], anchor: Vec2, o: CutOpts) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for p in outline {
        (p.x.to_bits(), p.y.to_bits()).hash(&mut h);
    }
    for c in colors {
        (c[0].to_bits(), c[1].to_bits(), c[2].to_bits()).hash(&mut h);
    }
    (anchor.x.to_bits(), anchor.y.to_bits(), o.tessellation.to_bits(), o.angle.to_bits()).hash(&mut h);
    (o.double_sided, o.improve_folding, outline.len(), colors.len()).hash(&mut h);
    h.finish()
}

/// outline_geom_opts with a vertex colour per outline point (empty = none); new vertices get the
/// average of the points they're made from.
pub fn outline_geom_col(outline: &[Vec2], colors: &[[f32; 3]], anchor: Vec2, o: CutOpts) -> MeshGeom {
    let key = geom_key(outline, colors, anchor, o);
    if let Some(g) = GEOM_CACHE.with(|c| c.borrow().iter().find(|e| e.0 == key).map(|e| e.1.clone())) {
        return g;
    }
    let g = outline_geom_build(outline, colors, anchor, o);
    GEOM_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= 8 {
            c.remove(0);
        }
        c.push((key, g.clone()));
    });
    g
}

fn outline_geom_build(outline: &[Vec2], colors: &[[f32; 3]], anchor: Vec2, o: CutOpts) -> MeshGeom {
    let mut uvs: Vec<Vec2> = outline.to_vec();
    let use_col = colors.len() == outline.len() && !colors.is_empty();
    let mut cols: Vec<[f32; 3]> = if use_col { colors.to_vec() } else { vec![] };
    let mut faces: Vec<[usize; 3]> = triangulate(&uvs);
    // Improve folding (fn 0x1401a4ef0, called by fn 0x1402f8980 after triangulating and again
    // after tessellating): edges crossing the fold line (through the pivot along the Angle) are
    // flipped toward it
    let th0 = o.angle.to_radians();
    let fold_dir = vec2(th0.sin(), -th0.cos());
    if o.improve_folding {
        fold_flips(&uvs, &mut faces, anchor, fold_dir);
    }
    // no automatic subdivision (the person's request): only the Tessellation option adds points.
    // The original (fn 0x1402f8980 -> 0x1401a6550 -> 0x1401a4320) adds (int)(t*t*5000) points,
    // one per step, each at the circumcentre of the triangle with the largest circle.
    let t = o.tessellation.clamp(0.0, 1.0);
    let count = (t * t * 5000.0) as usize;
    let mut rad: Vec<f32> = faces.iter().map(|&f| circle_size(&uvs, f)).collect();
    for _ in 0..count {
        refine_largest(&mut uvs, &mut cols, &mut faces, &mut rad, use_col);
    }
    if o.improve_folding && count > 0 {
        fold_flips(&uvs, &mut faces, anchor, fold_dir);
    }
    let mut idx: Vec<u32> = faces.iter().flat_map(|f| f.iter().map(|&i| i as u32)).collect();
    // the growth line: straight up in the picture, turned by Angle (clockwise, degrees)
    let th = o.angle.to_radians();
    let (up, right) = (vec2(th.sin(), th.cos()), vec2(th.cos(), -th.sin()));
    let mut pos: Vec<Vec3> = uvs
        .iter()
        .map(|uv| {
            let p = vec2(uv.x - anchor.x, anchor.y - uv.y);
            vec3(p.dot(right), p.dot(up), 0.0)
        })
        .collect();
    // make the front face (+Z) wind counter-clockwise
    for f in idx.chunks_mut(3) {
        let (a, b, c) = (f[0] as usize, f[1] as usize, f[2] as usize);
        if (pos[b] - pos[a]).cross(pos[c] - pos[a]).z < 0.0 {
            f.swap(1, 2);
        }
    }
    let mut nrm = vec![Vec3::Z; pos.len()];
    if o.double_sided {
        // back faces: their own vertices facing -Z, wound the other way
        let n = pos.len() as u32;
        pos.extend_from_within(..);
        let back_uv = uvs.clone();
        uvs.extend(back_uv);
        if use_col {
            cols.extend_from_within(..);
        }
        nrm.extend(std::iter::repeat(-Vec3::Z).take(n as usize));
        let front = idx.clone();
        for f in front.chunks(3) {
            idx.extend([f[0] + n, f[2] + n, f[1] + n]);
        }
    }
    MeshGeom { nrm, pos, uv: uvs, idx, col: cols }
}

/// A mesh's anchors after the pivot, in the same space as `processed_geom` (pivot at the origin,
/// built along the pivot's angle, then Orient, Flip and Scale): (position, direction of the
/// anchor's line). Each anchor's angle is measured like the pivot's: degrees clockwise from
/// straight up in the picture.
pub fn anchor_points(m: &LeafMesh) -> Vec<(Vec3, Vec3)> {
    anchor_points_h(m).into_iter().map(|(p, d, _, _)| (p, d)).collect()
}

/// The level whose anchors the tree uses: High, or the first level below it that has anchors.
pub fn anchor_level(m: &LeafMesh) -> usize {
    (0..3).find(|&l| !m.lods[l].anchors.is_empty()).unwrap_or(0)
}

/// anchor_points plus each anchor's height along the leaf (picture units from the pivot along its
/// growth line, before Orient/Scale): the value the original compares with First/Last
/// (fn 0x14026f500 reads the anchor record's second number).
pub fn anchor_points_h(m: &LeafMesh) -> Vec<(Vec3, Vec3, f32, usize)> {
    let th = m.angle.to_radians();
    let (up, right) = (vec2(th.sin(), th.cos()), vec2(th.cos(), -th.sin()));
    let local = |v: Vec2| vec3(v.dot(right), v.dot(up), 0.0);
    let orient = ORIENTS.get(m.orient).map(|o| o.1).unwrap_or(6);
    let s = flip_signs(m.flip);
    m.lods[anchor_level(m)]
        .anchors
        .iter()
        .map(|a| {
            let p = local(vec2(a.pos.x - m.anchor.x, m.anchor.y - a.pos.y));
            let r = a.angle.to_radians();
            let d = local(vec2(r.sin(), r.cos()));
            (orient_point(orient, p) * s * m.scale, (orient_point(orient, d) * s).normalize_or_zero(), p.y, a.id)
        })
        .collect()
}

/// Reorient a point: the original's axis matrix for each Orient value, read from the mesh
/// processing switch in fn 0x1400ab6c0 (8.4.2). Values 1, 2, 5, 8 and 9 mirror the mesh.
pub fn orient_point(value: i32, p: Vec3) -> Vec3 {
    match value {
        1 | 9 => vec3(p.x, p.z, p.y),  // XZY, Y-up left-handed
        2 => vec3(p.y, p.x, p.z),      // YXZ
        3 => vec3(p.y, -p.z, -p.x),    // YZX
        4 => vec3(-p.z, p.x, -p.y),    // ZXY
        5 => vec3(p.z, p.y, p.x),      // ZYX
        7 => vec3(p.x, -p.z, p.y),     // Y-up right-handed
        8 => vec3(p.x, p.y, -p.z),     // Z-up left-handed
        _ => p,                        // 0 XYZ, 6 Z-up right-handed: unchanged
    }
}

pub fn flip_signs(flip: usize) -> Vec3 {
    let label = FLIPS.get(flip).copied().unwrap_or("X Y Z");
    let mut s = Vec3::ONE;
    for tok in label.split_whitespace() {
        let neg = tok.starts_with('-');
        let v = if neg { -1.0 } else { 1.0 };
        match tok.trim_start_matches('-') {
            "X" => s.x = v,
            "Y" => s.y = v,
            _ => s.z = v,
        }
    }
    s
}

pub fn growth_axis(g: usize) -> Vec3 {
    match g {
        0 => Vec3::X,
        1 => Vec3::Y,
        2 => Vec3::Z,
        3 => -Vec3::X,
        4 => -Vec3::Y,
        _ => -Vec3::Z,
    }
}

/// The leaf frame implied by a mesh's Growth axis: (right, growth, face normal).
pub fn growth_frame(growth: usize) -> (Vec3, Vec3, Vec3) {
    let g = growth_axis(growth);
    let n = if g.z.abs() > 0.5 { Vec3::Y } else { Vec3::Z };
    let r = g.cross(n).normalize();
    (r, g, n)
}

/// Raw (unprocessed) geometry of one level: collection, loaded file, or cutout.
fn raw_geom(m: &LeafMesh, l: usize) -> Option<MeshGeom> {
    if m.collection {
        return m.collected.clone().filter(|g| !g.is_empty());
    }
    let lod = &m.lods[l];
    if let Some(g) = &lod.geom {
        return Some(g.clone());
    }
    if lod.outline.len() >= 3 {
        return Some(outline_geom_col(&lod.outline, &lod.colors, m.anchor, CutOpts::of(m)));
    }
    None
}

/// Finished geometry for a level after Orient, Flip, Scale, Flip normals and Fix winding,
/// following the original's mesh processing (fn 0x1400ab6c0, 8.4.2).
/// Levels 3 and up come from the "More" mesh's levels.
pub fn processed_geom(meshes: &[LeafMesh], mi: usize, lod: usize) -> Option<MeshGeom> {
    let m = meshes.get(mi)?;
    if lod >= 3 && m.more >= 1 && m.more - 1 != mi {
        return processed_geom(meshes, m.more - 1, lod - 3);
    }
    let l = m.lod_index(lod);
    let mut g = raw_geom(m, l)?;
    let orient = ORIENTS.get(m.orient).map(|o| o.1).unwrap_or(6);
    let s = flip_signs(m.flip);
    for p in &mut g.pos {
        *p = orient_point(orient, *p) * s * m.scale;
    }
    // Flip normals scales the normals by -1 in the same pass (fn 0x1400ab6c0, flag at +0x8d)
    let nflip = if m.flip_normals { -1.0 } else { 1.0 };
    for n in &mut g.nrm {
        *n = (orient_point(orient, *n) * s).normalize_or_zero() * nflip;
    }
    // The original counts mirrors: orient 1/2/5/8/9 count one, each flipped axis one, and Flip
    // normals one; when the count is odd every face is reversed (fn 0x1400ab6c0).
    let flips = (s.x < 0.0) as u32 + (s.y < 0.0) as u32 + (s.z < 0.0) as u32;
    let count = flips + matches!(orient, 1 | 2 | 5 | 8 | 9) as u32 + m.flip_normals as u32;
    let reverse = count % 2 == 1;
    // per face: reverse if needed, drop degenerate faces (cross product length² < 1e-12),
    // rebuild weak normals (length² < 0.9) from face normals × 0.01, and with Fix winding
    // swap faces whose winding is more than 90° from their vertex normals.
    let weak: Vec<bool> = g.nrm.iter().map(|n| n.length_squared() < 0.9).collect();
    let mut idx: Vec<u32> = Vec::with_capacity(g.idx.len());
    let mut add = vec![Vec3::ZERO; g.pos.len()];
    for f in g.idx.chunks(3) {
        let (a, mut b, mut c) = (f[0] as usize, f[1] as usize, f[2] as usize);
        if reverse {
            std::mem::swap(&mut b, &mut c);
        }
        let fnn = (g.pos[a] - g.pos[b]).cross(g.pos[a] - g.pos[c]);
        if fnn.length_squared() < 1e-12 {
            continue;
        }
        if weak[a] || weak[b] || weak[c] {
            for v in [a, b, c] {
                add[v] += fnn * 0.01;
            }
        }
        if m.fix_winding {
            let vn = g.nrm[a] + g.nrm[b] + g.nrm[c] + add[a] + add[b] + add[c];
            let cosv = fnn.dot(vn) / (fnn.length() * vn.length()).max(1e-20);
            if cosv < 1.0 && (cosv <= -1.0 || cosv.clamp(-1.0, 1.0).acos().to_degrees() > 90.0) {
                std::mem::swap(&mut b, &mut c);
            }
        }
        idx.extend_from_slice(&[a as u32, b as u32, c as u32]);
    }
    for (i, n) in g.nrm.iter_mut().enumerate() {
        if weak[i] {
            *n = (*n + add[i]).normalize_or_zero();
        }
    }
    g.idx = idx;
    Some(g)
}

pub fn bounds(g: &MeshGeom) -> (Vec3, Vec3) {
    let mut a = Vec3::splat(f32::MAX);
    let mut b = Vec3::splat(f32::MIN);
    for p in &g.pos {
        a = a.min(*p);
        b = b.max(*p);
    }
    if a.x > b.x {
        (Vec3::splat(-0.5), Vec3::splat(0.5))
    } else {
        (a, b)
    }
}
