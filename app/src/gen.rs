//! Tree generation: walks the generator graph and builds branch skins and leaf meshes.
use crate::model::*;
use crate::model::Material;
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::f32::consts::PI;

pub const CHUNK_VERTS: usize = 30000;
/// The most triangle corners one piece may hold: the drawing buffer's index room (main.rs
/// draw_call_index_capacity) less a margin. Picture 93: pieces of thin needle tubes held up to
/// 180000 corners against a 120000 buffer, so the drawing cut the rest off silently, and which
/// branches lost their ends moved whenever any generator was hidden or shown.
pub const CHUNK_INDICES: usize = 240_000;

pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) | 1)
    }
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
    }
    /// -1..1
    pub fn sym(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
}

pub struct Spine {
    pub node: u32,
    pub pts: Vec<Vec3>,
    pub rad: Vec<f32>,
    pub tan: Vec<Vec3>,
    pub nrm: Vec<Vec3>,
    pub cum: Vec<f32>,
    pub len: f32,
    /// The material its bark is drawn with (children with an Inherited entry take it).
    pub mat: usize,
    /// The spine it grows on and where along it (for curves reading a higher parent level).
    pub parent_spine: Option<usize>,
    pub pos_on_parent: f32,
    /// An extension of another spine (picture 86): a Skin > Splits fork or a Bifurcation child,
    /// with its number among that spine's extensions (1 = Split 1, 2 = Split 2 ...).
    pub ext_of: Option<(usize, u8)>,
    /// Spine:Break: the share of its whole length left after the break (1 = not broken).
    pub kept: f32,
}

impl Spine {
    fn locate(&self, t: f32) -> (usize, f32) {
        let d = t.clamp(0.0, 1.0) * self.len;
        for k in 0..self.pts.len() - 1 {
            if d <= self.cum[k + 1] || k == self.pts.len() - 2 {
                let seg = (self.cum[k + 1] - self.cum[k]).max(1e-6);
                return (k, ((d - self.cum[k]) / seg).clamp(0.0, 1.0));
            }
        }
        (0, 0.0)
    }
    pub fn point(&self, t: f32) -> Vec3 {
        let (k, f) = self.locate(t);
        self.pts[k].lerp(self.pts[k + 1], f)
    }
    pub fn radius(&self, t: f32) -> f32 {
        let (k, f) = self.locate(t);
        self.rad[k] + (self.rad[k + 1] - self.rad[k]) * f
    }
    pub fn frame(&self, t: f32) -> (Vec3, Vec3, Vec3) {
        let (k, f) = self.locate(t);
        let tn = self.tan[k].lerp(self.tan[k + 1], f).normalize_or_zero();
        let mut n = self.nrm[k].lerp(self.nrm[k + 1], f);
        n = (n - tn * n.dot(tn)).normalize_or_zero();
        if n.length_squared() < 0.5 {
            n = any_perp(tn);
        }
        let b = tn.cross(n);
        (tn, n, b)
    }
}

fn any_perp(t: Vec3) -> Vec3 {
    let a = if t.x.abs() < 0.9 { Vec3::X } else { Vec3::Z };
    t.cross(a).normalize()
}

#[derive(Default)]
pub struct Chunk {
    pub v: Vec<Vertex>,
    pub i: Vec<u16>,
}

#[derive(Default)]
pub struct MatGeo {
    pub chunks: Vec<Chunk>,
}

impl MatGeo {
    fn chunk_for(&mut self, need: usize) -> &mut Chunk {
        // a tube makes at most 6 corners a vertex (two triangles per quad) and a cap 3; a leaf
        // mesh rarely more, so 8 a vertex keeps every piece within the drawing buffer
        let full = match self.chunks.last() {
            Some(c) => c.v.len() + need > CHUNK_VERTS || c.i.len() + need * 8 > CHUNK_INDICES,
            None => true,
        };
        if full {
            self.chunks.push(Chunk::default());
        }
        self.chunks.last_mut().unwrap()
    }
}

#[derive(Default)]
pub struct TreeGeo {
    pub mats: HashMap<usize, MatGeo>,
    pub tris: usize,
    pub verts: usize,
    pub spines: usize,
    pub leaves: usize,
    pub bmin: Vec3,
    pub bmax: Vec3,
    /// The bounds with hidden generators too (picture 92): the leaves' shading reads these, so
    /// hiding a generator changes nothing else.
    pub full_min: Vec3,
    pub full_max: Vec3,
    pub per_node: HashMap<u32, usize>,
    /// Collection meshes built from contributing generators (mesh index -> geometry).
    pub collections: HashMap<usize, MeshGeom>,
    /// Messages for the person (e.g. a generator that found no anchor).
    pub warnings: Vec<String>,
    /// Every branch's spine (the viewport's Show > Bones).
    pub bones: Vec<Vec<Vec3>>,
    /// Leaf centres placed so far, by 1-unit cell (leaf collision removal).
    pub leaf_grid: HashMap<(i32, i32, i32), Vec<Vec3>>,
    /// Leaves removed by leaf collision.
    pub leaves_removed: usize,
    /// Materials whose Color runs as a gradient along each branch (picture 74): base and tip
    /// colours by material.
    pub grad: HashMap<usize, ([f32; 3], [f32; 3], f32)>,
    /// Every node made (for Node edit mode picking): generator, its number in the generator,
    /// where it starts and ends.
    pub nodes: Vec<(u32, usize, Vec3, Vec3)>,
    /// What a click can hit (picture 26): each node's spine points and radii (a leaf: its centre
    /// and a radius), with its generator and number.
    pub hits: Vec<(u32, usize, Vec<Vec3>, Vec<f32>)>,
    /// Spines of Skin:Type "Spine only" generators, drawn as yellow lines (picture 39).
    pub spine_lines: Vec<(u32, Vec<Vec3>)>,
    /// PivotPainter2 (TreeCraft's own tool): every branch and leaf as an element, in the order
    /// made, keyed by (generator, node number) so the same element has the same key in every
    /// generation of this tree.
    pub pp_elems: Vec<PPElem>,
    /// Which drawn vertices belong to which element: (material, chunk, first vertex, end, key).
    pub pp_ranges: Vec<(usize, usize, usize, usize, (u32, usize))>,
    /// The element each spine (by its index in the spine list) became.
    pp_of_spine: HashMap<usize, (u32, usize)>,
}

/// One PivotPainter2 element: a branch (its spine) or a leaf. Its pivot is where it starts, its
/// X axis points along it, its Y axis is across it, its length runs along X, and its parent is
/// the element it grows on (None: a root).
#[derive(Clone, Debug)]
pub struct PPElem {
    pub key: (u32, usize),
    pub pivot: Vec3,
    pub x: Vec3,
    pub y: Vec3,
    pub len: f32,
    /// Its size across Y and Z (the branch's base diameter, or the leaf's width).
    pub width: f32,
    pub parent: Option<(u32, usize)>,
    pub depth: u32,
}

impl TreeGeo {
    /// Where every material's geometry ends now (to tell later which vertices an element added).
    fn pp_mark(&self) -> Vec<(usize, usize, usize)> {
        self.mats.iter().map(|(k, m)| (*k, m.chunks.len(), m.chunks.last().map_or(0, |c| c.v.len()))).collect()
    }
    /// Record that everything added since `mark` belongs to element `key`.
    fn pp_since(&mut self, mark: &[(usize, usize, usize)], key: (u32, usize)) {
        let mut out = vec![];
        for (k, m) in &self.mats {
            let (c0, s0) = match mark.iter().find(|x| x.0 == *k) {
                Some(&(_, n, l)) if n > 0 => (n - 1, l),
                _ => (0, 0),
            };
            for (ci, ch) in m.chunks.iter().enumerate().skip(c0) {
                let start = if ci == c0 { s0 } else { 0 };
                if ch.v.len() > start {
                    out.push((*k, ci, start, ch.v.len(), key));
                }
            }
        }
        self.pp_ranges.extend(out);
    }
    /// Add element `key` growing on the spine `parent_spine` (or a root).
    fn pp_add(&mut self, key: (u32, usize), pivot: Vec3, tip: Vec3, side: Vec3, width: f32, parent_spine: Option<usize>) {
        let d = tip - pivot;
        let x = d.normalize_or(Vec3::Y);
        let mut y = side - x * side.dot(x);
        if y.length_squared() < 1e-8 {
            y = x.any_orthonormal_vector();
        }
        let parent = parent_spine.and_then(|i| self.pp_of_spine.get(&i).copied());
        let depth = parent.and_then(|p| self.pp_elems.iter().rev().find(|e| e.key == p)).map_or(0, |e| e.depth + 1);
        self.pp_elems.push(PPElem { key, pivot, x, y: y.normalize(), len: d.length(), width, parent, depth });
    }
}

/// A curve's multiplier for one property and kind (1 when there's no curve).
pub fn curve_at(p: &Params, name: &str, kind: u8, x: f32) -> f32 {
    match p.curves.get(&format!("{}|{}", name, kind)) {
        Some(c) => c.eval(x),
        // no curve drawn yet: the original's default shape for it (picture 41), else flat
        None => match crate::stcurves::default_curve(name, kind) {
            Some(i) => preset_curves()[i.min(preset_curves().len() - 1)].eval(x),
            None => 1.0,
        },
    }
}

/// The curve editor's presets, made once.
fn preset_curves() -> &'static [Curve] {
    static PRESETS: std::sync::OnceLock<Vec<Curve>> = std::sync::OnceLock::new();
    PRESETS.get_or_init(|| (0..crate::editmodes::CURVE_PRESETS.len()).map(crate::editmodes::preset_curve).collect())
}

/// The generator's values for one child: Parent curves read at `t` (where it sits on its parent),
/// Distribution curves at `d` (its place among the children on that parent, 0 first .. 1 last).
fn curved(p: &Params, ts: [f32; 4], d: f32) -> Option<Params> {
    let keys: Vec<(String, u8)> = p.curves.keys().filter_map(|k| k.rsplit_once('|').map(|(n, c)| (n.to_string(), c.parse::<u8>().unwrap_or(9)))).filter(|(_, c)| *c < 2).collect();
    if keys.is_empty() {
        return None;
    }
    let mut q = p.clone();
    for (name, c) in keys {
        // the curve editor's Parent level: 1 = the direct parent, 2 = its parent ...
        let level = p.curve_levels.get(&format!("{}|{}", name, c)).copied().unwrap_or(1).clamp(1, 4) as usize;
        let m = curve_at(p, &name, c, if c == 0 { ts[level - 1] } else { d });
        crate::stpanel::scale_prop(&mut q, &name, m);
    }
    Some(q)
}

/// One branch's material from its generator's Materials list (picture 25): picked by weight,
/// steady for each node; "Inherited" takes the parent branch's. An empty list uses `material`.
fn pick_material(p: &Params, parent_mat: usize, gen: u32, node: usize) -> usize {
    if p.mat_list.is_empty() {
        return p.material;
    }
    let total: f32 = p.mat_list.iter().map(|e| e.1.max(0.0)).sum();
    let mut rng = Rng::new(gen as u64 * 2654435761 + node as u64 * 40503 + 17);
    let mut x = rng.next() * total.max(1e-6);
    let mut chosen = p.mat_list[0].0;
    for (m, w) in &p.mat_list {
        if x < w.max(0.0) {
            chosen = *m;
            break;
        }
        x -= w.max(0.0);
    }
    if chosen == INHERITED {
        parent_mat
    } else {
        chosen
    }
}

/// A property the original has that TreeCraft keeps in Params::extra (or its default).
pub fn pex(p: &Params, name: &str, def: f32) -> f32 {
    p.extra.get(name).map(|v| *v as f32).unwrap_or(def)
}

/// Spine:Prune (picture 36): General removes that share of branches at random; Up / Down remove
/// branches pointing up / down past 1 - 2 x the amount; Interior removes ones growing back toward the
/// trunk's axis; Ground removes any that dip below the ground.
fn pruned(p: &Params, s: &Spine, gen: u32, node: usize) -> bool {
    let n = s.pts.len();
    if n < 2 {
        return false;
    }
    let dir = (s.pts[n - 1] - s.pts[0]).normalize_or_zero();
    let general = pex(p, "Spine:Prune:General", 0.0);
    if general > 0.0 && hash01(gen as u64, node as u64, 77) * 0.5 + 0.5 < general {
        return true;
    }
    let up = pex(p, "Spine:Prune:Up", 0.0);
    if up > 0.0 && dir.y > 1.0 - 2.0 * up {
        return true;
    }
    let down = pex(p, "Spine:Prune:Down", 0.0);
    if down > 0.0 && dir.y < -(1.0 - 2.0 * down) {
        return true;
    }
    let interior = pex(p, "Spine:Prune:Interior", 0.0);
    if interior > 0.0 {
        let out = vec2(s.pts[0].x, s.pts[0].z).normalize_or_zero();
        let d2 = vec2(dir.x, dir.z).normalize_or_zero();
        if out != Vec2::ZERO && d2 != Vec2::ZERO && -d2.dot(out) > 1.0 - 2.0 * interior {
            return true;
        }
    }
    if pex(p, "Spine:Prune:Ground", 0.0) > 0.5 && s.pts.iter().any(|q| q.y < 0.0) {
        return true;
    }
    false
}

/// Spine:Break: the branch snapped at Spot (a share of its length).
fn broken(s: Spine, spot: f32, p: &Params) -> Spine {
    let cut = s.len * spot;
    let mut pts = vec![s.pts[0]];
    for k in 1..s.pts.len() {
        if s.cum[k] >= cut {
            let f = (cut - s.cum[k - 1]) / (s.cum[k] - s.cum[k - 1]).max(1e-6);
            pts.push(s.pts[k - 1].lerp(s.pts[k], f));
            break;
        }
        pts.push(s.pts[k]);
    }
    if pts.len() < 2 {
        return s;
    }
    let mut b = build_spine(s.node, pts, s.rad[0], p);
    // the radius keeps its taper as if the branch were whole
    for (k, r) in b.rad.iter_mut().enumerate() {
        *r = s.radius((b.cum[k] / s.len).min(1.0));
    }
    b.kept = (b.len / s.len.max(1e-6)).clamp(0.01, 1.0);
    b
}

/// A random number in -1..1 from a few integers (steady for the same node).
fn hash01(a: u64, b: u64, c: u64) -> f32 {
    let mut x = a.wrapping_mul(0x9E3779B97F4A7C15) ^ b.wrapping_mul(0xC2B2AE3D27D4EB4F) ^ c.wrapping_mul(0x165667B19E3779F9);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 29;
    ((x >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
}

/// One node's values with every property's Variance Settings applied (the four TreeCraft varies
/// itself are left to make_spine / make_leaf): the value plus up to Variance above or below; the
/// random part is Uniform or Gaussian (bell curve); Cohesion blends a part shared by neighbours
/// (changing smoothly along the parent at Scale, starting at Offset) with each node's own.
fn varied(p: &Params, t: f32, gen: u32, node: usize, parent: usize) -> Option<Params> {
    let own = ["Spine:Length:+ % of parent", "Spine:Orientation:Start angle", "Skin:Radius:+ % of parent", "Leaves:Size"];
    let list: Vec<(&String, &Variance)> = p.variances.iter().filter(|(k, v)| v.amount.abs() > 1e-6 && !own.contains(&k.as_str())).collect();
    if list.is_empty() {
        return None;
    }
    let mut q = p.clone();
    for (name, v) in list {
        let h = name.bytes().fold(1469598103934665603u64, |a, b| (a ^ b as u64).wrapping_mul(1099511628211));
        let mut r = hash01(gen as u64, node as u64, h);
        if v.gaussian {
            r = (r + hash01(gen as u64 + 7, node as u64, h) + hash01(gen as u64 + 13, node as u64, h)) / 3.0 * 1.7;
            r = r.clamp(-1.0, 1.0);
        }
        // the cohesive part: smooth along the parent, the same for neighbours
        let ph = (h % 1000) as f32 * 0.01 + parent as f32 * 1.3;
        let x = t * v.scale.max(0.0) * 6.2832 + v.offset * 6.2832 + ph;
        let shared = (x.sin() * 0.65 + (x * 2.3 + 1.7).sin() * 0.35).clamp(-1.0, 1.0);
        let k = v.cohesion.clamp(0.0, 1.0);
        let r = shared * (1.0 - k) + r * k;
        crate::stpanel::add_prop(&mut q, name, v.amount * r);
    }
    Some(q)
}

/// A node's own offsets (Node edit mode) on top of its generator's values.
fn edited(p: &Params, e: &NodeEdit) -> Params {
    let mut q = p.clone();
    // the node's own values (Node mode, picture 65) first, then the gizmo's scale and turn
    crate::treefile::apply_kv(&mut q, &e.props);
    q.len_abs *= e.length;
    q.len_pct *= e.length;
    q.leaf_size *= e.length;
    q.radius_abs *= e.radius;
    q.radius_pct *= e.radius;
    q.start_angle = (q.start_angle + e.angle).clamp(0.0, 1.0);
    q
}

impl TreeGeo {
    fn grow(&mut self, p: Vec3) {
        self.bmin = self.bmin.min(p);
        self.bmax = self.bmax.max(p);
        self.full_min = self.full_min.min(p);
        self.full_max = self.full_max.max(p);
    }

    /// A place to build a hidden generator's parts out of sight (picture 92): it borrows the
    /// leaves already placed, so leaf collision works as if the parts were drawn.
    fn hidden_scratch(&mut self) -> TreeGeo {
        TreeGeo {
            bmin: Vec3::splat(f32::MAX),
            bmax: Vec3::splat(f32::MIN),
            full_min: Vec3::splat(f32::MAX),
            full_max: Vec3::splat(f32::MIN),
            leaf_grid: std::mem::take(&mut self.leaf_grid),
            ..Default::default()
        }
    }

    /// Take back what a hidden generator's parts leave behind: the leaves placed (for collision)
    /// and how far they reach (for the leaves' shading); nothing is drawn.
    fn hidden_merge(&mut self, s: TreeGeo) {
        self.leaf_grid = s.leaf_grid;
        if s.full_min.x <= s.full_max.x {
            self.full_min = self.full_min.min(s.full_min);
            self.full_max = self.full_max.max(s.full_max);
        }
    }
}

fn vtx(p: Vec3, uv: Vec2, col: [u8; 4], n: Vec3, ao: f32) -> Vertex {
    Vertex { position: p, uv, color: col, normal: vec4(n.x, n.y, n.z, ao) }
}

#[derive(Clone, Copy)]
enum ParentRef {
    Ground,
    Spine(usize),
    /// An anchor of a leaf made by a Leaf generator: its children spawn there.
    Anchor(usize),
}

/// A leaf's anchor in tree space: where it is, the direction its line points, and the leaf's normal.
#[derive(Clone, Copy)]
pub struct AnchorFrame {
    /// Which anchor of its mesh this is (1 = the first after the pivot).
    pub index: usize,
    /// Height along the leaf, compared with the child generator's First/Last.
    pub height: f32,
    pub pos: Vec3,
    pub dir: Vec3,
    pub nrm: Vec3,
}

pub struct GenCtx<'a> {
    pub nodes: &'a [Node],
    pub materials: &'a [Material],
    pub meshes: &'a [LeafMesh],
    pub selected: Option<u32>,
    pub highlight: bool,
    /// Draw trunk and branch skins (the viewport's Show > Branches); their spines still grow
    /// so leaves keep their places.
    pub show_branches: bool,
    /// Scene forces (Direction, Magnet) bending every branch as it grows.
    pub forces: &'a [SceneObj],
    /// Leaf collision removal (the toolbar's Post > Collision): 0 None, 1 Low, 2 Medium, 3 High.
    pub leaf_collision: u8,
    /// Only these generators draw geometry (File > Export selected as mesh); None = all.
    pub only: Option<std::collections::HashSet<u32>>,
}

pub fn generate(ctx: &GenCtx) -> TreeGeo {
    let mut geo = TreeGeo { bmin: Vec3::splat(f32::MAX), bmax: Vec3::splat(f32::MIN), full_min: Vec3::splat(f32::MAX), full_max: Vec3::splat(f32::MIN), ..Default::default() };
    // gradient colours (picture 74): only while no picture is in the material's Color slot
    for (i, m) in ctx.materials.iter().enumerate() {
        if m.grad_on && (m.file.is_empty() || m.source != 4) {
            let (a, b) = if m.grad_flip { (m.grad_color, m.grad_base) } else { (m.grad_base, m.grad_color) };
            // (flipped, the blend point is measured from the other end)
            let mid = if m.grad_flip { 1.0 - m.grad_mid } else { m.grad_mid };
            geo.grad.insert(i, (a, b, mid));
        }
    }
    let mut spines: Vec<Spine> = vec![];
    let root = match ctx.nodes.iter().find(|n| n.kind == Kind::Tree) {
        Some(r) => r,
        None => return geo,
    };
    let tree = root.p.clone();
    let lod = tree.lod_preview.max(0) as usize;
    // processed leaf geometry per mesh choice (0 = plain card, k = mesh k-1)
    let mut cache: Vec<Option<MeshGeom>> = vec![Some(crate::meshio::outline_geom(&[vec2(0.0, 0.0), vec2(0.5, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.5, 1.0), vec2(0.0, 1.0)], vec2(0.5, 1.0)))];
    for mi in 0..ctx.meshes.len() {
        cache.push(crate::meshio::processed_geom(ctx.meshes, mi, lod));
    }
    let mut leaf_ranges: Vec<(usize, usize, usize)> = vec![]; // (material, chunk, vertex start)
    let mut anchors: Vec<AnchorFrame> = vec![];
    process(ctx, root.id, &[ParentRef::Ground], &tree, &mut spines, &mut anchors, &mut geo, &cache, &mut leaf_ranges);
    geo.collections = collect(ctx, &spines);
    place_tree(&mut geo, &tree);
    if geo.bmin.x > geo.bmax.x {
        geo.bmin = vec3(-5.0, 0.0, -5.0);
        geo.bmax = vec3(5.0, 10.0, 5.0);
    }
    // ambient occlusion for leaves: deeper inside the crown is darker
    // (picture 92: over the whole tree, hidden generators included)
    let (amin, amax) = if geo.full_min.x <= geo.full_max.x { (geo.full_min, geo.full_max) } else { (geo.bmin, geo.bmax) };
    let c = (amin + amax) * 0.5;
    let half = ((amax - amin) * 0.5).max(Vec3::splat(0.5));
    for (m, ci, start) in leaf_ranges {
        if let Some(mg) = geo.mats.get_mut(&m) {
            let ch = &mut mg.chunks[ci];
            let end = ch.v.len();
            for v in &mut ch.v[start..end] {
                let d = (v.position - c) / half;
                let r = (d.x * d.x + d.z * d.z).sqrt().min(1.0);
                let h = ((v.position.y - amin.y) / (half.y * 2.0)).clamp(0.0, 1.0);
                let ao = (0.35 + 0.45 * r.powf(0.8) + 0.25 * h).min(1.0);
                v.normal.w = v.normal.w.min(ao);
            }
        }
    }
    geo
}

/// The Tree generator's Generation > Size scalar and Shape > Sink / Roll (picture 49): the whole
/// tree scaled about its root, turned around the up axis (degrees) and lowered into the ground.
fn place_tree(geo: &mut TreeGeo, tree: &Params) {
    let scale = tree.size_scalar.max(0.001);
    let sink = tree.extra.get("Shape:Sink").copied().unwrap_or(0.0) as f32;
    let roll = (tree.extra.get("Shape:Roll").copied().unwrap_or(0.0) as f32).to_radians();
    if (scale - 1.0).abs() < 1e-6 && sink == 0.0 && roll == 0.0 {
        return;
    }
    let rot = Quat::from_rotation_y(roll);
    let pt = |p: Vec3| rot * p * scale - Vec3::Y * sink;
    for mg in geo.mats.values_mut() {
        for ch in &mut mg.chunks {
            for v in &mut ch.v {
                v.position = pt(v.position);
                let n = rot * vec3(v.normal.x, v.normal.y, v.normal.z);
                v.normal = vec4(n.x, n.y, n.z, v.normal.w);
            }
        }
    }
    for b in &mut geo.bones {
        for p in b.iter_mut() {
            *p = pt(*p);
        }
    }
    for n in &mut geo.nodes {
        n.2 = pt(n.2);
        n.3 = pt(n.3);
    }
    for h in &mut geo.hits {
        for p in h.2.iter_mut() {
            *p = pt(*p);
        }
        for r in h.3.iter_mut() {
            *r *= scale;
        }
    }
    for s in &mut geo.spine_lines {
        for p in s.1.iter_mut() {
            *p = pt(*p);
        }
    }
    if geo.bmin.x <= geo.bmax.x {
        let (a, b) = (geo.bmin, geo.bmax);
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for k in 0..8 {
            let c = pt(vec3(if k & 1 == 0 { a.x } else { b.x }, if k & 2 == 0 { a.y } else { b.y }, if k & 4 == 0 { a.z } else { b.z }));
            lo = lo.min(c);
            hi = hi.max(c);
        }
        geo.bmin = lo;
        geo.bmax = hi;
    }
}

#[allow(clippy::too_many_arguments)]
fn process(
    ctx: &GenCtx,
    node_id: u32,
    parents: &[ParentRef],
    tree: &Params,
    spines: &mut Vec<Spine>,
    anchors: &mut Vec<AnchorFrame>,
    geo: &mut TreeGeo,
    cache: &[Option<MeshGeom>],
    leaf_ranges: &mut Vec<(usize, usize, usize)>,
) {
    let mut children: Vec<&Node> = ctx.nodes.iter().filter(|n| n.parent == Some(node_id)).collect();
    // Bifurcation generators first, so their forks exist when siblings Extend parent onto them
    children.sort_by_key(|n| (n.kind.makes_spines() && crate::stpanel::orig_mode(&n.p) == 7) as u8 == 0);
    let parent_gen = ctx.nodes.iter().find(|n| n.id == node_id);
    for child in children {
        // a hidden generator (picture 91) still grows, so every generator on it or beside it stays
        // exactly as it was; only its own bark, leaves and picking are left out
        let shown = !child.hidden && ctx.only.as_ref().map_or(true, |o| o.contains(&child.id));
        // Extend parent (picture 86): this generator's parents with the extensions it may use
        let ext_parents = if child.kind.makes_spines() || child.kind == Kind::Leaf { extend_parents(&child.p, parents, spines) } else { parents.to_vec() };
        let parents: &[ParentRef] = &ext_parents;
        // Proportional modes count against the longest parent
        let max_plen = parents.iter().filter_map(|q| if let ParentRef::Spine(i) = q { Some(spines[*i].len) } else { None }).fold(0.0f32, f32::max);
        let sel = ctx.highlight && ctx.selected == Some(child.id);
        // the selected generator is marked by alpha only; its colour comes from Selection (picture 30)
        let col: [u8; 4] = if sel { [255, 255, 255, 128] } else { [255, 255, 255, 255] };
        let p = &child.p;
        let mut new_spines: Vec<ParentRef> = vec![];
        let mut count = 0usize;
        let on_anchors = parents.iter().all(|q| matches!(q, ParentRef::Anchor(_)));
        // each node's number in its generator, counted the same way every time (Node edit mode)
        let mut inst = 0usize;
        // where this generator's kept children start (Knockout > Proximity)
        let mut kept: Vec<Vec3> = vec![];
        for (pi, parent) in parents.iter().enumerate() {
            // Generation:Anchors: only the chosen anchor (fns 0x14024b880, 0x1402456e0)
            if let ParentRef::Anchor(ai) = parent {
                // (the Gen tab's Anchors row, picture 56, when the Leaves tab's choice is All)
                let pick = if p.anchor_pick > 0 { p.anchor_pick } else { pex(p, "Generation:Anchors", 0.0).max(0.0) as usize };
                if pick > 0 && anchors[*ai].index != pick {
                    continue;
                }
                // fn 0x14026f500: only anchors whose height lies between First and Last
                let h = anchors[*ai].height;
                if h < p.first.min(p.last) || h > p.first.max(p.last) {
                    continue;
                }
            }
            let mut rng = Rng::new((p.seed as u64) * 7919 + pi as u64 * 104729 + child.id as u64 * 31);
            let placements = shared_placement(p, *parent, spines, distribute(p, *parent, spines, tree, &mut rng, max_plen));
            let n_on_parent = placements.len();
            for (k_on_parent, (t, roll, base_pos, azim_dir)) in placements.into_iter().enumerate() {
                let me = inst;
                inst += 1;
                // property curves (Parent, Distribution) and this node's own offsets
                let d = k_on_parent as f32 / (n_on_parent.max(2) - 1) as f32;
                // where this child sits on its parent, its grandparent ...
                // on the ground (trunks) "where on the parent" is the trunk's place among the trunks
                let mut ts = if matches!(*parent, ParentRef::Ground) { [d; 4] } else { [t; 4] };
                if let ParentRef::Spine(i) = *parent {
                    let mut cur = Some(i);
                    for lv in 1..4 {
                        match cur {
                            Some(s) => {
                                ts[lv] = spines[s].pos_on_parent;
                                cur = spines[s].parent_spine;
                            }
                            None => ts[lv] = ts[lv - 1],
                        }
                    }
                }
                let pc = curved(&child.p, ts, d);
                // Variance Settings: each node strays by its own random amount (picture 35)
                let pc = varied(pc.as_ref().unwrap_or(&child.p), t, child.id, me, pi).or(pc);
                let edit = child.p.node_edits.get(&me);
                if edit.map_or(false, |e| e.hidden) {
                    continue;
                }
                // Generation > Knockout (picture 56): removes that share of the children; with
                // Proximity > Range, Weight of it goes to those crowding a kept sibling
                let ko = pex(&child.p, "Generation:Knockout", 0.0).clamp(0.0, 1.0);
                if ko > 0.0 {
                    let at = match *parent {
                        ParentRef::Spine(i) => spines[i].point(t.min(1.0)),
                        ParentRef::Anchor(ai) => anchors[ai].pos,
                        ParentRef::Ground => base_pos,
                    };
                    let range = pex(&child.p, "Generation:Shared:Knockout:Proximity:Range", 0.0).max(0.0) * child.p.size_scalar;
                    let w = if range > 0.0 { pex(&child.p, "Generation:Shared:Knockout:Proximity:Weight", 1.0).clamp(0.0, 1.0) } else { 0.0 };
                    let crowded = range > 0.0 && kept.iter().any(|k| k.distance(at) < range);
                    // (an estimate: random share (1 - Weight) x Knockout, plus Weight x sqrt(Knockout)
                    // for children within Range of a kept one)
                    let chance = (1.0 - w) * ko + if crowded { w * ko.sqrt() } else { 0.0 };
                    if hash01(child.id as u64, me as u64, 7331) * 0.5 + 0.5 < chance.min(1.0) {
                        continue;
                    }
                    kept.push(at);
                }
                let pe = match edit {
                    Some(e) => Some(edited(pc.as_ref().unwrap_or(&child.p), e)),
                    None => pc,
                };
                let p = pe.as_ref().unwrap_or(&child.p);
                let roll = roll + edit.map_or(0.0, |e| e.roll) * 2.0 * PI;
                // Node mode's Move (picture 65): slid along the parent, staying on its surface
                let t = match *parent {
                    ParentRef::Spine(_) => (t + edit.map_or(0.0, |e| e.slide)).clamp(0.0, 1.0),
                    _ => t,
                };
                match child.kind {
                    Kind::Trunk | Kind::Branch => {
                        let roll = roll + pex(p, "Spine:Orientation:Roll", 0.0) * 2.0 * PI;
                        // the parent generator's Shape Control envelope (on when its Size:Absolute
                        // is above 0): a sphere at Position along the parent spine
                        let shape = match (*parent, parent_gen) {
                            (ParentRef::Spine(i), Some(pg)) => {
                                let abs = pex(&pg.p, "Shape Control:Size:Absolute", 0.0);
                                (abs > 0.0).then(|| {
                                    let ps = &spines[i];
                                    let rr = abs + pex(&pg.p, "Shape Control:Size:+ % of size", 1.5) * ps.len * 0.5;
                                    (ps.point(pex(&pg.p, "Shape Control:Position", 0.75).clamp(0.0, 1.0)), rr)
                                })
                            }
                            _ => None,
                        };
                        if let Some(mut s) = make_spine(child, p, *parent, t, roll, base_pos, azim_dir, spines, ctx.forces, &mut rng, shape) {
                            // Spine > Orientation > Roll (picture 74) on a trunk standing on the
                            // ground: the whole trunk turns around the upright through its base
                            // (with a node's own roll from Node mode), bends and all
                            if matches!(*parent, ParentRef::Ground) {
                                let turn = (pex(p, "Spine:Orientation:Roll", 0.0) + edit.map_or(0.0, |e| e.roll)) * 2.0 * PI;
                                if turn != 0.0 {
                                    let q = Quat::from_rotation_y(-turn);
                                    let base = s.pts[0];
                                    for v in &mut s.pts {
                                        *v = base + q * (*v - base);
                                    }
                                    for v in s.tan.iter_mut().chain(s.nrm.iter_mut()) {
                                        *v = q * *v;
                                    }
                                }
                            }
                            // Prune and Break (picture 36): drop or shorten this branch
                            if pruned(p, &s, child.id, me) {
                                continue;
                            }
                            // Skin:Radius:Length kill ratio (fn 0x140141d60): a branch longer than
                            // that many times its radius is removed ("Branch removed (length/radius
                            // larger than kill ratio)"); not for Spine only skins
                            let kill = pex(p, "Skin:Radius:Length kill ratio", 500.0);
                            let skin_type = pex(p, "Skin:Type", 0.0) as i32;
                            if kill > 0.0 && skin_type != 3 && (s.rad[0] <= 0.0 || kill < s.len / s.rad[0]) {
                                continue;
                            }
                            if pex(p, "Spine:Break:Broken", 0.0) > 0.5 {
                                let seed = pex(p, "Spine:Break:Seed", 15000.0) as u64;
                                if hash01(seed, child.id as u64, me as u64) * 0.5 + 0.5 < pex(p, "Spine:Break:Chance", 0.0) {
                                    let spot = pex(p, "Spine:Break:Spot", 0.5).clamp(0.02, 1.0);
                                    s = broken(s, spot, p);
                                }
                            }
                            let parent_mat = if let ParentRef::Spine(i) = *parent { spines[i].mat } else { NO_MATERIAL };
                            s.mat = pick_material(&child.p, parent_mat, child.id, me);
                            if let ParentRef::Spine(i) = *parent {
                                s.parent_spine = Some(i);
                                // a Bifurcation fork extends its parent (Extend parent, picture 86)
                                if crate::stpanel::orig_mode(p) == 7 {
                                    let nth = spines.iter().filter(|q| matches!(q.ext_of, Some((o, _)) if o == i)).count();
                                    s.ext_of = Some((i, (nth + 1).min(255) as u8));
                                }
                            }
                            s.pos_on_parent = t;
                            if shown {
                                geo.bones.push(s.pts.clone());
                                geo.nodes.push((child.id, me, s.pts[0], s.pts[s.pts.len() - 1]));
                                geo.hits.push((child.id, me, s.pts.clone(), s.rad.clone()));
                            }
                            // Skin:Type "Spine only" (3) keeps the spine for its children but draws no
                            // bark; Skin:Visibility draws only that share of the branches (their
                            // children still grow)
                            let visible = hash01(child.id as u64, me as u64, 4049) * 0.5 + 0.5 < pex(p, "Skin:Visibility", 1.0);
                            // Spine only (picture 39): drawn as a yellow line with a mark at each segment
                            if skin_type == 3 && ctx.show_branches && shown {
                                geo.spine_lines.push((child.id, s.pts.clone()));
                            }
                            if ctx.show_branches && !shown && skin_type != 3 && visible {
                                // hidden (picture 92): built out of sight, for the bounds only
                                let mut scratch = geo.hidden_scratch();
                                skin(&s, p, col, &mut scratch, child.kind == Kind::Trunk, None, child.id, me);
                                geo.hidden_merge(scratch);
                            }
                            if ctx.show_branches && shown && skin_type != 3 && visible {
                                // picture 72: the parent is drawn with flat facets, which lie inside its
                                // round radius by r x (1 - cos(pi / sides)); the weld fits the base to
                                // that inner surface, 3% further in, so no crease opens between them
                                let weld = if let ParentRef::Spine(i) = *parent {
                                    let sides = parent_gen.map_or(8, |g| g.p.seg_radial.max(3)) as f32;
                                    let inner = (PI / sides).cos() * 0.97;
                                    Some((spines[i].radius(t.min(1.0)) * inner, spines[i].frame(t.min(1.0)).0, spines[i].point(t.min(1.0))))
                                } else {
                                    None
                                };
                                let mark = geo.pp_mark();
                                skin(&s, p, col, geo, child.kind == Kind::Trunk, weld, child.id, me);
                                geo.pp_since(&mark, (child.id, me));
                            }
                            // PivotPainter2: this branch is an element growing on its parent's
                            let ps = if let ParentRef::Spine(i) = *parent { Some(i) } else { None };
                            geo.pp_add((child.id, me), s.pts[0], s.pts[s.pts.len() - 1], s.nrm[0], s.rad[0] * 2.0, ps);
                            geo.pp_of_spine.insert(spines.len(), (child.id, me));
                            // its Skin > Splits forks, as spines children can grow on with Extend
                            // parent (they're drawn with the bark above)
                            let forks = if skin_type != 3 { split_forks(&s, p, child.id, me) } else { vec![] };
                            spines.push(s);
                            let me_spine = spines.len() - 1;
                            new_spines.push(ParentRef::Spine(me_spine));
                            for mut f in forks {
                                f.ext_of = f.ext_of.map(|(_, k)| (me_spine, k));
                                f.parent_spine = Some(me_spine);
                                spines.push(f);
                            }
                            count += 1;
                        }
                    }
                    Kind::Leaf if !shown => {
                        // hidden (picture 91): its leaves are made out of sight, so leaf generators
                        // under it still find the same anchors
                        let mut scratch = geo.hidden_scratch();
                        if pex(p, "Leaves:Frond", 0.0) <= 0.5 {
                            let mut scratch_ranges = vec![];
                            let frames = make_leaf(ctx, p, *parent, t, roll, base_pos, spines, anchors, &mut rng, col, &mut scratch, cache, &mut scratch_ranges);
                            for fr in frames {
                                anchors.push(fr);
                                new_spines.push(ParentRef::Anchor(anchors.len() - 1));
                            }
                        } else if k_on_parent == 0 {
                            if let ParentRef::Spine(i) = *parent {
                                let (a, b) = (p.first.min(p.last).clamp(0.0, 1.0), p.first.max(p.last).clamp(0.0, 1.0));
                                let b = if b - a < 0.02 { 1.0 } else { b };
                                make_frond(&spines[i], a, b, p, col, &mut scratch);
                            }
                        }
                        geo.hidden_merge(scratch);
                        count += 1;
                    }
                    Kind::Leaf if pex(p, "Leaves:Frond", 0.0) > 0.5 => {
                        // a frond (picture 65): one flat card per parent, from First to Last along
                        // its spine, bending with it
                        if k_on_parent == 0 {
                            if let ParentRef::Spine(i) = *parent {
                                let s = &spines[i];
                                let (a, b) = (p.first.min(p.last).clamp(0.0, 1.0), p.first.max(p.last).clamp(0.0, 1.0));
                                let b = if b - a < 0.02 { 1.0 } else { b };
                                let mark = geo.pp_mark();
                                make_frond(s, a, b, p, col, geo);
                                geo.pp_since(&mark, (child.id, me));
                                geo.pp_add((child.id, me), s.point(a), s.point(b), s.frame(a).1, (p.leaf_size * p.size_scalar).max(0.02), Some(i));
                                let at = s.point(a);
                                geo.nodes.push((child.id, me, at, s.point(b)));
                                geo.hits.push((child.id, me, (0..=8).map(|k| s.point(a + (b - a) * k as f32 / 8.0)).collect(), vec![(p.leaf_size * p.size_scalar).max(0.1) * 0.5; 9]));
                                count += 1;
                            }
                        }
                    }
                    Kind::Leaf => {
                        let mark = geo.pp_mark();
                        let frames = make_leaf(ctx, p, *parent, t, roll, base_pos, spines, anchors, &mut rng, col, geo, cache, leaf_ranges);
                        geo.pp_since(&mark, (child.id, me));
                        let at = match *parent {
                            ParentRef::Spine(i) => spines[i].point(t),
                            ParentRef::Anchor(ai) => anchors[ai].pos,
                            ParentRef::Ground => base_pos,
                        };
                        let tip = frames.first().map_or(at + Vec3::Y * p.leaf_size.max(0.1) * 0.5, |f| f.pos);
                        geo.nodes.push((child.id, me, at, tip));
                        // PivotPainter2: the leaf, from where it sits toward its tip
                        {
                            let ps = if let ParentRef::Spine(i) = *parent { Some(i) } else { None };
                            let side = ps.map_or(Vec3::X, |i| spines[i].frame(t.min(1.0)).0);
                            geo.pp_add((child.id, me), at, tip, side, (p.leaf_size * p.size_scalar).max(0.02), ps);
                        }
                        let lr = (p.leaf_size * p.size_scalar).max(0.1) * 0.8;
                        geo.hits.push((child.id, me, vec![at], vec![lr]));
                        for fr in frames {
                            anchors.push(fr);
                            new_spines.push(ParentRef::Anchor(anchors.len() - 1));
                        }
                        count += 1;
                    }
                    Kind::Tree => {}
                }
            }
        }
        // Freehand: branches drawn by hand belong to this generator too (their points are kept
        // as drawn; children grow on them like on any branch)
        if child.kind.makes_spines() {
            for (k, pts) in child.p.drawn.iter().enumerate() {
                if pts.len() < 2 {
                    continue;
                }
                let r0 = (child.p.radius_abs * child.p.size_scalar).max(0.005);
                let mut s = build_spine(child.id, pts.clone(), r0, &child.p);
                let parent_mat = spines.iter().find(|q| Some(q.node) == child.parent).map_or(NO_MATERIAL, |q| q.mat);
                s.mat = pick_material(&child.p, parent_mat, child.id, usize::MAX - k);
                if shown {
                    geo.bones.push(s.pts.clone());
                    geo.nodes.push((child.id, usize::MAX - k, s.pts[0], s.pts[s.pts.len() - 1]));
                    geo.hits.push((child.id, usize::MAX - k, s.pts.clone(), s.rad.clone()));
                }
                if ctx.show_branches && !shown {
                    let mut scratch = geo.hidden_scratch();
                    skin(&s, &child.p, col, &mut scratch, child.kind == Kind::Trunk, None, child.id, usize::MAX - k);
                    geo.hidden_merge(scratch);
                }
                if ctx.show_branches && shown {
                    let mark = geo.pp_mark();
                    skin(&s, &child.p, col, geo, child.kind == Kind::Trunk, None, child.id, usize::MAX - k);
                    geo.pp_since(&mark, (child.id, usize::MAX - k));
                }
                // PivotPainter2: a hand-drawn branch grows on the nearest spine of its parent
                {
                    let ps = spines.iter().position(|q| Some(q.node) == child.parent);
                    geo.pp_add((child.id, usize::MAX - k), s.pts[0], s.pts[s.pts.len() - 1], s.nrm[0], s.rad[0] * 2.0, ps);
                    geo.pp_of_spine.insert(spines.len(), (child.id, usize::MAX - k));
                }
                spines.push(s);
                new_spines.push(ParentRef::Spine(spines.len() - 1));
                count += 1;
            }
        }
        geo.per_node.insert(child.id, count);
        if child.kind.makes_spines() {
            geo.spines += count;
            if !new_spines.is_empty() {
                process(ctx, child.id, &new_spines, tree, spines, anchors, geo, cache, leaf_ranges);
            }
        } else if child.kind == Kind::Leaf {
            geo.leaves += count;
            // leaf generators under this one spawn on its leaves' anchors
            if ctx.nodes.iter().any(|n| n.parent == Some(child.id)) {
                process(ctx, child.id, &new_spines, tree, spines, anchors, geo, cache, leaf_ranges);
            }
        }
        if child.kind == Kind::Leaf && on_anchors && count == 0 {
            // the original's message when nothing could be placed on a mesh parent (fn 0x1402456e0)
            geo.warnings.push(format!(
                "{}: Unable to find anchor for node (check mesh anchors (including LOD meshes), first/last, and anchor restrictions)",
                child.name
            ));
        }
    }
}

/// Where children go on a parent: (position along parent 0..1, roll angle in radians,
/// ground position, ground outward direction).
///
/// Follows the original's placement routines (see MECHANICS.md):
/// - Classic (fn 0x140477e40): count = whole part of Frequency (Relative style: parent length
///   x Frequency x 0.025); each child at a random spot between First and Last (flat frequency
///   curve, fn 0x1402cad40); roll in turns = (Balanced ? i/n : random) + random(-Offset, Offset)/2,
///   times Sweep, plus Roll.
/// - Absolute (fn 0x1404723f0): count = Number; roll = (i/n + random(-Jumble, Jumble)/2) x Sweep,
///   centred on zero, plus Roll.
/// - Interval (fn 0x14047b6f0): walk from Last down to First in steps of 10 / Frequency length
///   units, putting Count children around each node (1/Count apart, half a turn when fewer than 3),
///   adding Spiral x 0.1 turns per node; Spread jitters each child along the parent.
fn distribute(p: &Params, parent: ParentRef, spines: &[Spine], tree: &Params, rng: &mut Rng, max_plen: f32) -> Vec<(f32, f32, Vec3, Vec3)> {
    const TURN: f32 = 2.0 * PI;
    let mut out = vec![];
    let plen = match parent {
        ParentRef::Spine(i) => spines[i].len,
        ParentRef::Ground => 0.0,
        // one child on each anchor, turned by Roll around the anchor's direction
        ParentRef::Anchor(_) => return vec![(0.0, p.roll * TURN, Vec3::ZERO, Vec3::X)],
    };
    // First and Last with their Parent curves (picture 91), read where this parent sits
    let (f0, l0) = (p.first * place_curve(p, "Generation:First", 0.0, parent, spines, true), p.last * place_curve(p, "Generation:Last", 0.0, parent, spines, true));
    let (first, last) = (f0.min(l0), l0.max(f0));
    let span = last - first;
    let mode = crate::stpanel::orig_mode(p);
    let ext = match parent {
        ParentRef::Spine(i) => ext_amount(p, &spines[i]),
        _ => 0.0,
    };
    if let ParentRef::Ground = parent {
        // children of the Tree node: trunks spread over the Tree's Shape radius
        let n = match mode {
            0 => p.frequency.max(0.0) as usize,
            1 => pex(p, "Generation:Proportional:Number", 2.0).max(0.0) as usize,
            2 => (pex(p, "Generation:Proportional Steps:Steps", 1.0) * pex(p, "Generation:Proportional Steps:Per Step:Number", 1.0)).max(0.0) as usize,
            4 => (pex(p, "Generation:Absolute Steps:Steps", 1.0) * pex(p, "Generation:Absolute Steps:Per Step:Number", 1.0)).max(0.0) as usize,
            7 | 8 | 9 => 1,
            _ => p.number.max(0) as usize,
        }
        .min(500);
        for i in 0..n {
            let base = if p.balanced { i as f32 / n.max(1) as f32 } else { rng.next() };
            let turns = (base + rng.sym() * p.offset * 0.5) * p.sweep + p.roll;
            let ang = turns * TURN;
            let dir = vec3(ang.cos(), 0.0, ang.sin());
            let r = if n <= 1 { 0.0 } else { tree.tree_radius * 0.5 * ((i as f32 + 0.5) / n as f32).sqrt() };
            out.push((0.0, ang, dir * r, dir));
        }
        return out;
    }
    // the golden angle in turns (137.5 degrees): spiral phyllotaxy and Flood
    const GOLD: f32 = 0.381_966;
    // Steps (picture 86): `steps` evenly spaced spots from First to Last (one step sits in the
    // middle), `per` children at each; Spread scatters them along the parent by that share of the
    // gap between steps; each child's roll = (j / per + random(-Jumble, Jumble) / 2) x Sweep + Roll,
    // and every step turns a further Step roll
    let stepped = |out: &mut Vec<(f32, f32, Vec3, Vec3)>, steps: usize, per: usize, spread: f32, step_roll: f32, jumble: f32, sweep: f32, roll: f32, rng: &mut Rng| {
        let steps = steps.clamp(1, 500);
        let per = per.min(200);
        let gap = span / steps as f32;
        for k in 0..steps {
            let tk = if steps == 1 { first + span * 0.5 } else { first + span * k as f32 / (steps - 1) as f32 };
            for j in 0..per {
                let tj = tk + rng.sym() * spread * gap * 0.5;
                let turns = (j as f32 / per.max(1) as f32 + rng.sym() * jumble * 0.5) * sweep + roll + step_roll * k as f32;
                out.push((onto_parent(p, tj, ext), turns * TURN, Vec3::ZERO, Vec3::X));
            }
        }
    };
    match mode {
        // Absolute: exactly Number children
        3 => {
            let n = (p.number.max(0) as usize).min(2000);
            let mut rolls: Vec<f32> = (0..n)
                .map(|i| (rng.sym() * p.jumble * 0.5 + i as f32 / n.max(1) as f32) * p.sweep)
                .collect();
            if let (Some(mn), Some(mx)) = (rolls.iter().cloned().reduce(f32::min), rolls.iter().cloned().reduce(f32::max)) {
                let mid = (mn + mx) * 0.5;
                for r in &mut rolls {
                    *r = *r - mid + p.roll;
                }
            }
            for r in rolls {
                let t = along(p, "Generation:Absolute:Number", first, span, rng);
                // First / Last below 0 or above 1 (picture 74): spots off the parent stay at its
                // base or tip instead of disappearing
                out.push((onto_parent(p, t, ext), r * TURN, Vec3::ZERO, Vec3::X));
            }
        }
        // Proportional (picture 86): Number children on the longest parent of this generator,
        // fewer on shorter ones in proportion to their length (rounded); rolls as Absolute
        1 => {
            let num = pex(p, "Generation:Proportional:Number", 2.0).max(0.0);
            let n = ((num * plen / max_plen.max(1e-4)).round().max(0.0) as usize).min(2000);
            let (jumble, sweep, roll) = (pex(p, "Generation:Proportional:Jumble", 0.0), pex(p, "Generation:Proportional:Sweep", 1.0), pex(p, "Generation:Proportional:Roll", 0.0));
            for i in 0..n {
                let turns = (i as f32 / n.max(1) as f32 + rng.sym() * jumble * 0.5) * sweep + roll;
                let t = along(p, "Generation:Proportional:Number", first, span, rng);
                out.push((onto_parent(p, t, ext), turns * TURN, Vec3::ZERO, Vec3::X));
            }
        }
        // Proportional Steps: Steps on the longest parent, fewer on shorter ones (at least one);
        // Per Step Number children each, Offset scatters them along the parent
        2 => {
            let pre = "Generation:Proportional Steps";
            let pp = |name: &str| place_curve(p, &format!("{}:{}", pre, name), 0.0, parent, spines, true);
            let steps = (pex(p, &format!("{}:Steps", pre), 1.0).max(1.0) * pp("Steps") * plen / max_plen.max(1e-4)).round().max(1.0) as usize;
            let per = (pex(p, &format!("{}:Per Step:Number", pre), 1.0).max(0.0) * pp("Per Step:Number")).round() as usize;
            stepped(
                &mut out,
                steps,
                per,
                pex(p, &format!("{}:Per Step:Offset", pre), 0.0),
                pex(p, &format!("{}:Per Step:Roll", pre), 0.0),
                pex(p, &format!("{}:Jumble", pre), 0.0),
                pex(p, &format!("{}:Sweep", pre), 1.0),
                pex(p, &format!("{}:Roll", pre), 0.0),
                rng,
            );
        }
        // Absolute Steps: Steps on every parent, Per Step Number children each
        4 => {
            let pre = "Generation:Absolute Steps";
            stepped(
                &mut out,
                (pex(p, &format!("{}:Steps", pre), 1.0).max(1.0) * place_curve(p, &format!("{}:Steps", pre), 0.0, parent, spines, true)).round().max(1.0) as usize,
                (pex(p, &format!("{}:Per Step:Number", pre), 1.0).max(0.0) * place_curve(p, &format!("{}:Per Step:Number", pre), 0.0, parent, spines, true)).round() as usize,
                pex(p, &format!("{}:Per Step:Spread", pre), 0.0),
                pex(p, &format!("{}:Per Step:Roll", pre), 0.0),
                pex(p, &format!("{}:Jumble", pre), 0.0),
                pex(p, &format!("{}:Sweep", pre), 1.0),
                pex(p, &format!("{}:Roll", pre), 0.0),
                rng,
            );
        }
        // Phyllotaxy (picture 86): nodes evenly from Last down to First, as many as Internode
        // length (units) fits plus one, so the first and last nodes sit on First and Last with no
        // gap; Limit caps them. Style: Opposite (distichous) 2 a node in one plane, Opposite
        // (decussate) 2 a node turning a quarter each node, Opposite 2 a node turning the golden
        // angle, Alternating (distichous) 1 a node turning half, Alternating 1 a node turning the
        // golden angle, Whorled Children-per-node a node, each whorl turned half a gap. Spiral adds
        // 0.1 turn a node; Spread scatters children along the parent by that share of the gap.
        5 => {
            let (a, b) = (first.clamp(0.0, 1.0), last.clamp(0.0, 1.0));
            let style = pex(p, "Generation:Phyllotaxy:Arrangement:Style", 5.0) as i32;
            let per = match style {
                0..=2 => 2,
                3 | 4 => 1,
                _ => p.per_node.max(1) as usize,
            };
            // the nodes: a walk down from Last, each step Internode length (x its Parent curve
            // where the walk is, picture 91), then stretched so the last node sits on First
            let pc = |name: &str, t: f32| place_curve(p, name, t, parent, spines, false);
            let limit = ((p.limit.max(1) as f32) * place_curve(p, "Generation:Phyllotaxy:Nodes:Limit", 0.0, parent, spines, true)).round().max(1.0) as usize;
            let mut ts = vec![b];
            let mut t = b;
            while ts.len() < limit.min(2000) {
                t -= (p.interval * pc("Generation:Phyllotaxy:Nodes:Internode length", t)).max(0.05) / plen.max(0.001);
                if t < a - 1e-4 {
                    break;
                }
                ts.push(t);
            }
            if ts.len() > 1 {
                let low = *ts.last().unwrap();
                let k = if b - low > 1e-6 { (b - a) / (b - low) } else { 1.0 };
                for t in &mut ts {
                    *t = b - (b - *t) * k;
                }
            }
            let nodes = ts.len();
            let gap = if nodes > 1 { (b - a) / (nodes - 1) as f32 } else { 0.0 };
            let mut spiral = 0.0f32;
            for (k, &t) in ts.iter().enumerate() {
                let kf = k as f32;
                let per = if style >= 5 { ((per as f32) * pc("Generation:Phyllotaxy:Arrangement:Children per node", t)).round().max(1.0) as usize } else { per };
                let node_turn = match style {
                    0 => 0.0,
                    1 => 0.25 * kf,
                    3 => 0.5 * kf,
                    2 | 4 => GOLD * kf,
                    _ => (k % 2) as f32 * 0.5 / per as f32,
                } + spiral;
                spiral += p.spiral * pc("Generation:Phyllotaxy:Nodes:Spiral", t) * 0.1;
                let spread = p.jumble * pc("Generation:Phyllotaxy:Nodes:Spread", t);
                for j in 0..per {
                    let tj = (t + rng.sym() * spread * gap * 0.5).clamp(a, b);
                    let turns = node_turn + j as f32 / per as f32;
                    out.push((tj, turns * TURN, Vec3::ZERO, Vec3::X));
                }
            }
        }
        // Bifurcation (picture 86; estimated): spots from Last down to First every Spacing (a
        // share of the parent); each forks with chance Threshold (the one at Last always does when
        // Threshold is above 0); forks alternate sides. Balance makes the forks smaller and Align
        // turns them to the world's sides (make_spine); they count as the parent's extensions.
        7 => {
            let thr = pex(p, "Generation:Bifurcation:Threshold", 0.5).clamp(0.0, 1.0);
            let spacing = pex(p, "Generation:Bifurcation:Spacing", 0.1).max(0.01);
            let (a, b) = (first.clamp(0.0, 1.0), last.clamp(0.0, 1.0));
            let mut t = b;
            let mut k = 0;
            // (picture 91: Parent curves on Threshold and Spacing, read where the walk is)
            let pc = |name: &str, t: f32| place_curve(p, name, t, parent, spines, false);
            while t >= a - 1e-4 && thr > 0.0 && out.len() < 500 {
                if k == 0 || rng.next() < thr * pc("Generation:Bifurcation:Threshold", t) {
                    let turns = 0.5 * (out.len() % 2) as f32 + p.roll;
                    out.push((t.clamp(0.0, 1.0), turns * TURN, Vec3::ZERO, Vec3::X));
                }
                t -= (spacing * pc("Generation:Bifurcation:Spacing", t)).max(0.01);
                k += 1;
            }
        }
        // Flood (picture 86; estimated): a child on every point of the parent's spine between
        // First and Last, each turned the golden angle from the last
        8 => {
            if let ParentRef::Spine(i) = parent {
                let s = &spines[i];
                let (a, b) = (first.clamp(0.0, 1.0), last.clamp(0.0, 1.0));
                for (k, c) in s.cum.iter().enumerate() {
                    let t = c / s.len.max(1e-6);
                    if t > 1e-4 && t >= a - 1e-4 && t <= b + 1e-4 {
                        out.push((t.min(1.0), (GOLD * k as f32) * TURN, Vec3::ZERO, Vec3::X));
                    }
                }
            }
        }
        // Parent (picture 86; estimated): one child where First sits on each parent, carrying
        // on along it (make_spine points it the parent's way)
        9 => out.push((first.clamp(0.0, 1.0), 0.0, Vec3::ZERO, Vec3::X)),
        6 => {
            // Interval: spacing 10/Frequency units
            // (picture 91: Parent curves on Frequency, Count, Spread and Spiral vary them along
            // the parent, read at each node; Limit's at the parent's own spot)
            let pc = |name: &str, t: f32| place_curve(p, name, t, parent, spines, false);
            let limit = ((p.limit.max(1) as f32) * place_curve(p, "Generation:Interval:Limit", 0.0, parent, spines, true)).round().max(1.0) as i32;
            // (picture 74: the walk keeps to the parent)
            let (first, last) = (first.clamp(0.0, 1.0), last.clamp(0.0, 1.0));
            let mut t = last;
            let mut node = 0i32;
            let mut spiral_turns = 0.0f32;
            while t >= first && node < limit && out.len() < 4000 {
                let step = 10.0 / (p.interval * pc("Generation:Interval:Frequency", t)).max(0.01) / plen.max(0.001);
                let per = ((p.per_node.max(1) as f32) * pc("Generation:Interval:Count", t)).round().max(1.0) as usize;
                let within = if per >= 3 { 1.0 / per as f32 } else { 0.5 };
                let stagger = if p.balanced && node % 2 == 1 { 0.5 / per as f32 } else { 0.0 };
                let spread = p.jumble * pc("Generation:Interval:Spread", t);
                for j in 0..per {
                    let tj = (t + rng.sym() * spread * step * 0.5).clamp(first, last);
                    let turns = (j as f32 * within + stagger + spiral_turns) * p.sweep + p.roll;
                    if (-1e-4..=1.0001).contains(&tj) {
                        out.push((tj.clamp(0.0, 1.0), turns * TURN, Vec3::ZERO, Vec3::X));
                    }
                }
                spiral_turns += p.spiral * pc("Generation:Interval:Spiral", t) * 0.1;
                t -= step;
                node += 1;
            }
        }
        _ => {
            let f = if p.relative { plen * p.frequency * 0.025 } else { p.frequency };
            let n = (f.max(0.0) as usize).min(2000);
            if pex(p, "Generation:Steps:Enabled", 0.0) > 0.5 {
                // Classic > Steps (picture 86): Number evenly spaced steps from First to Last;
                // Per step frequency on puts Frequency children at every step, off shares them
                // out (child i at step i mod steps); Spread scatters them along the parent by that
                // share of the gap between steps; every step turns a further Roll
                let steps = ((pex(p, "Generation:Steps:Number", 5.0).max(1.0) * place_curve(p, "Generation:Steps:Number", 0.0, parent, spines, true)).round().max(1.0) as usize).min(500);
                let per_step = pex(p, "Generation:Steps:Per step frequency", 0.0) > 0.5;
                let spread = pex(p, "Generation:Steps:Spread", 0.0);
                let step_roll = pex(p, "Generation:Steps:Roll", 0.0);
                let gap = span / steps as f32;
                let total = if per_step { n * steps } else { n };
                for i in 0..total.min(4000) {
                    let k = if per_step { i / n.max(1) } else { i % steps };
                    let in_step = if per_step { n } else { (n + steps - 1 - k) / steps };
                    let j = if per_step { i % n.max(1) } else { i / steps };
                    let tk = if steps == 1 { first + span * 0.5 } else { first + span * k as f32 / (steps - 1) as f32 };
                    let base = if p.balanced { j as f32 / in_step.max(1) as f32 } else { rng.next() };
                    let turns = (base + rng.sym() * p.offset * 0.5) * p.sweep + p.roll + step_roll * k as f32;
                    let t = tk + rng.sym() * spread * gap * 0.5;
                    out.push((onto_parent(p, t, ext), turns * TURN, Vec3::ZERO, Vec3::X));
                }
            } else {
                for i in 0..n {
                    let base = if p.balanced { i as f32 / n.max(1) as f32 } else { rng.next() };
                    let turns = (base + rng.sym() * p.offset * 0.5) * p.sweep + p.roll;
                    let t = along(p, "Generation:Frequency", first, span, rng);
                    out.push((onto_parent(p, t, ext), turns * TURN, Vec3::ZERO, Vec3::X));
                }
            }
        }
    }
    out
}

/// Extend parent (picture 86): the original's list (fn 0x140247820) None, Any, Split 1, Split 2,
/// Skinny, Fat, Breaks only, Bottom, stored by number.
pub fn extend_choice(p: &Params) -> usize {
    (pex(p, "Generation:Shared:Extend parent", 0.0).round().max(0.0) as usize).min(7)
}

/// How far past the tip children may go on this parent, as a share of its length: with Extend
/// parent Breaks only, a snapped branch's children carry on over the part that broke off (along
/// the tip's direction); otherwise none.
fn ext_amount(p: &Params, s: &Spine) -> f32 {
    if extend_choice(p) == 6 && s.kept < 1.0 {
        1.0 / s.kept - 1.0
    } else {
        0.0
    }
}

/// The parents a generator's children grow on, with Extend parent (picture 86; estimated): Any
/// adds every extension of each parent (its Skin > Splits forks and Bifurcation children), Split 1
/// / Split 2 only that one, Skinny / Fat the thinnest / thickest; Bottom moves children from an
/// extension onto the spine it extends (down to the first); None and Breaks only keep the parents.
fn extend_parents(p: &Params, parents: &[ParentRef], spines: &[Spine]) -> Vec<ParentRef> {
    let choice = extend_choice(p);
    let mut out: Vec<ParentRef> = vec![];
    for q in parents {
        let ParentRef::Spine(i) = *q else {
            out.push(*q);
            continue;
        };
        if choice == 7 {
            let mut root = i;
            while let Some((up, _)) = spines[root].ext_of {
                if up >= spines.len() || up == root {
                    break;
                }
                root = up;
            }
            if !out.iter().any(|o| matches!(o, ParentRef::Spine(r) if *r == root)) {
                out.push(ParentRef::Spine(root));
            }
            continue;
        }
        out.push(*q);
        if !(1..=5).contains(&choice) {
            continue;
        }
        let exts: Vec<usize> = (0..spines.len()).filter(|&j| matches!(spines[j].ext_of, Some((o, _)) if o == i)).collect();
        let pick: Vec<usize> = match choice {
            1 => exts,
            2 | 3 => exts.into_iter().filter(|&j| spines[j].ext_of.map(|e| e.1 as usize) == Some(choice - 1)).collect(),
            _ => {
                let by = |j: &usize| spines[*j].rad.first().copied().unwrap_or(0.0);
                let best = if choice == 4 { exts.iter().copied().min_by(|a, b| by(a).total_cmp(&by(b))) } else { exts.iter().copied().max_by(|a, b| by(a).total_cmp(&by(b))) };
                best.into_iter().collect()
            }
        };
        for j in pick {
            out.push(ParentRef::Spine(j));
        }
    }
    out
}


/// The Gen tab's shared settings on where children go (picture 56; estimated, see MECHANICS.md):
/// Shared > Position shifts every child along the parent by that share of its length; Shared >
/// Rotation turns them all around it (turns); Extend parent stretches the spots past the
/// parent's tip by that share (t x (1 + extend)); Boundaries > Clip drops children within that
/// share of the parent's tip; Favor splits pulls each child toward the nearest spot where a
/// sibling generator's child already grows.
fn shared_placement(p: &Params, parent: ParentRef, spines: &[Spine], mut out: Vec<(f32, f32, Vec3, Vec3)>) -> Vec<(f32, f32, Vec3, Vec3)> {
    let ParentRef::Spine(i) = parent else { return out };
    let pos = pex(p, "Generation:Shared:Position", 0.0);
    let rot = pex(p, "Generation:Shared:Rotation", 0.0) * 2.0 * PI;
    let ext = ext_amount(p, &spines[i]);
    let clip = pex(p, "Generation:Shared:Boundaries:Clip", 0.025).clamp(0.0, 1.0);
    let favor = match crate::stpanel::orig_mode(p) {
        3 => pex(p, "Generation:Absolute:Favor splits", 0.0),
        1 => pex(p, "Generation:Proportional:Favor splits", 0.0),
        _ => 0.0,
    }
    .clamp(0.0, 1.0);
    // the spots other generators' children already took on this parent
    let splits: Vec<f32> = if favor > 0.0 { spines.iter().filter(|s| s.parent_spine == Some(i)).map(|s| s.pos_on_parent).collect() } else { vec![] };
    out.retain_mut(|(t, roll, _, _)| {
        let mut tt = *t + pos;
        if let Some(near) = splits.iter().copied().min_by(|a, b| (a - tt).abs().total_cmp(&(b - tt).abs())) {
            tt += (near - tt) * favor;
        }
        tt *= 1.0 + ext;
        *roll += rot;
        *t = tt;
        tt >= -1e-4 && tt <= 1.0 + ext + 1e-4 && (ext > 0.0 || tt <= 1.0 - clip + 1e-4 || clip <= 0.0)
    });
    for o in &mut out {
        o.0 = o.0.max(0.0);
    }
    out
}

/// A spot along the parent for a child (picture 74): First / Last beyond the parent's ends put
/// it at the base (below 0) or as close to the tip as Boundaries > Clip lets children grow (above
/// 1), so it stays on the parent instead of disappearing.
fn onto_parent(p: &Params, t: f32, ext: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t > 1.0 {
        let clip = pex(p, "Generation:Shared:Boundaries:Clip", 0.025).clamp(0.0, 1.0);
        if ext > 0.0 { 1.0 } else { (1.0 - clip).max(0.0) }
    } else {
        t
    }
}

/// A random spot between First and Last; a Profile curve on the count property makes children
/// likelier where it's high (picture 24).
fn along(p: &Params, count_prop: &str, first: f32, span: f32, rng: &mut Rng) -> f32 {
    // the Profile curve, and (picture 91) the Parent curve on the count: more children where
    // either is high, along First..Last
    let c2 = p.curves.get(&format!("{}|2", count_prop));
    let c0 = p.curves.get(&format!("{}|0", count_prop));
    if c2.is_none() && c0.is_none() {
        return first + span * rng.next();
    }
    let dens = |x: f32| c2.map_or(1.0, |c| c.eval(x)).max(0.0) * c0.map_or(1.0, |c| c.eval((first + span * x).clamp(0.0, 1.0))).max(0.0);
    let top = (0..=32).map(|k| dens(k as f32 / 32.0)).fold(0.0f32, f32::max).max(1e-3) * 1.05;
    for _ in 0..48 {
        let x = rng.next();
        if rng.next() * top <= dens(x) {
            return first + span * x;
        }
    }
    first + span * rng.next()
}

/// A Parent curve (picture 91) on a property that decides where children go, read where it
/// applies: level 1 at the spot `t` on this parent (or, `per_parent`, where this parent sits on
/// its own parent), each further level one parent up. 1 when no curve is drawn.
fn place_curve(p: &Params, name: &str, t: f32, parent: ParentRef, spines: &[Spine], per_parent: bool) -> f32 {
    let key = format!("{}|0", name);
    let Some(c) = p.curves.get(&key) else { return 1.0 };
    let level = p.curve_levels.get(&key).copied().unwrap_or(1).clamp(1, 4) as usize;
    let mut x = t;
    let mut cur = if let ParentRef::Spine(i) = parent { Some(i) } else { None };
    let climbs = level - 1 + per_parent as usize;
    for _ in 0..climbs {
        match cur {
            Some(i) => {
                x = spines[i].pos_on_parent;
                cur = spines[i].parent_spine;
            }
            None => break,
        }
    }
    c.eval(x.clamp(0.0, 1.0))
}

#[allow(clippy::too_many_arguments)]
fn make_spine(
    node: &Node,
    p: &Params,
    parent: ParentRef,
    t: f32,
    roll: f32,
    ground_pos: Vec3,
    ground_dir: Vec3,
    spines: &[Spine],
    forces: &[SceneObj],
    rng: &mut Rng,
    shape: Option<(Vec3, f32)>,
) -> Option<Spine> {
    // Spine:Orientation:Unify: every child on one parent draws the same random values
    let mut unified;
    let rng: &mut Rng = if pex(p, "Spine:Orientation:Unify", 0.0) > 0.5 {
        let pi = if let ParentRef::Spine(i) = parent { i as u64 } else { 0 };
        unified = Rng::new(p.seed as u64 * 7919 + pi * 104729 + node.id as u64 * 31);
        &mut unified
    } else {
        rng
    };
    let gen_mode = crate::stpanel::orig_mode(p);
    let angle = (p.start_angle + p.angle_var * rng.sym() + p.angle_profile * (t - 0.5)).clamp(0.0, 1.0) * PI;
    // Parent mode (picture 86): the child carries on along its parent
    let angle = if gen_mode == 9 && matches!(parent, ParentRef::Spine(_)) { 0.0 } else { angle };
    let mut anc_dir: Option<Vec3> = None;
    let (mut start, dir, plen, prad) = match parent {
        ParentRef::Ground => {
            // start angle measured up from the ground: 0.5 = straight up
            let d = (Vec3::Y * angle.sin() + ground_dir * angle.cos()).normalize();
            (ground_pos, d, 0.0, 0.0)
        }
        ParentRef::Spine(i) => {
            let ps = &spines[i];
            let (tn, mut n, mut b) = ps.frame(t);
            // Generation > Align (picture 56; estimated): on, the children's roll counts from a
            // fixed reference: Align style 1 the world's up (seen across the parent), 0 the
            // parent's own frame; off, each child turns by a random extra amount
            let omode = crate::stpanel::orig_mode(p);
            let mode_pre = match omode {
                2 => "Generation:Proportional Steps",
                3 => "Generation:Absolute",
                4 => "Generation:Absolute Steps",
                5 => "Generation:Phyllotaxy:Arrangement",
                6 => "Generation:Interval",
                _ => "Generation:Proportional",
            };
            let (align_on, style) = match omode {
                // Bifurcation's Align (picture 86): 1 turns the forks to the world's sides, 0
                // keeps the parent's own frame; Flood and Parent keep the parent's frame
                7 => (true, (pex(p, "Generation:Bifurcation:Align", 0.0) > 0.5) as i32),
                8 | 9 => (true, 0),
                _ => (pex(p, &format!("{}:Align", mode_pre), 1.0) > 0.5, pex(p, &format!("{}:Align style", mode_pre), 1.0) as i32),
            };
            let mut roll = roll;
            if !align_on {
                roll += rng.next() * 2.0 * PI;
            } else if style == 1 {
                let up = Vec3::Y - tn * Vec3::Y.dot(tn);
                if up.length_squared() > 1e-4 {
                    n = up.normalize();
                    b = tn.cross(n);
                }
            }
            let perp = n * roll.cos() + b * roll.sin();
            let mut d = (tn * angle.cos() + perp * angle.sin()).normalize();
            // Spine:Orientation:Alignment: 1 = Start angle measured from the parent's direction,
            // 0 = from straight up (the world's vertical), in between a blend
            let align = pex(p, "Spine:Orientation:Alignment", 1.0).clamp(0.0, 1.0);
            if align < 1.0 {
                let hor = vec3(perp.x, 0.0, perp.z).normalize_or(vec3(d.x, 0.0, d.z).normalize_or(Vec3::X));
                let dw = (Vec3::Y * angle.cos() + hor * angle.sin()).normalize();
                d = dw.lerp(d, align).normalize_or(d);
            }
            // Spine:Shape:Ancestor: start turned that much toward the parent's own direction
            let anc = pex(p, "Spine:Shape:Ancestor", 0.0).clamp(-1.0, 1.0);
            if anc != 0.0 {
                d = d.lerp(tn, anc).normalize_or(d);
            }
            anc_dir = Some(tn);
            // Extend parent: spots past the tip continue along the tip's direction
            let over = (t - 1.0).max(0.0);
            // Generation > Sink (picture 56): the base pushed back into the parent by that many
            // times the parent's radius there
            let sink = pex(p, "Generation:Sink", 0.0).max(0.0);
            let pr = ps.radius(t);
            (ps.point(t) + tn * over * ps.len - d * sink * pr, d, ps.len, pr)
        }
        ParentRef::Anchor(_) => return None, // only leaves spawn on anchors
    };
    let prof = 1.0 - p.len_profile * ((t - p.first) / (p.last - p.first).max(0.01)).clamp(0.0, 1.0);
    // Spine:Length:+ % of radius: longer by that share of the parent's radius where it starts
    let mut len = ((p.len_abs + p.len_pct * plen + pex(p, "Spine:Length:+ % of radius", 0.0) * prad) * prof * (1.0 + p.len_var * rng.sym()) * p.size_scalar).max(0.0);
    // Shape Control (picture 38): the parent's envelope sphere shortens branches that would reach
    // past it, by Shape scale; Stop shape control leaves this generator alone
    if let Some((c, rr)) = shape {
        let k = pex(p, "Spine:Length:Shape scale", 1.0).max(0.0);
        if k > 0.0 && pex(p, "Spine:Length:Stop shape control", 0.0) < 0.5 {
            // how far along `dir` from `start` the sphere's surface is
            let oc = start - c;
            let b = oc.dot(dir);
            let disc = b * b - (oc.length_squared() - rr * rr);
            let reach = if disc >= 0.0 { (-b + disc.sqrt()).max(0.0) } else { 0.0 };
            if len > reach {
                len = (len + (reach - len) * k.min(1.0)).max(0.0);
            }
        }
    }
    let mut r0 = (p.radius_abs + p.radius_pct * prad) * (1.0 + p.radius_var * rng.sym()) * p.size_scalar;
    if p.clamp_parent && prad > 0.0 {
        r0 = r0.min(prad * 0.95);
    }
    r0 = r0.max(0.005);
    // Bifurcation > Balance (picture 86; estimated): 0 = forks as big as the generator makes them,
    // 1 = forks 40% the size
    if gen_mode == 7 {
        let k = 1.0 - 0.6 * pex(p, "Generation:Bifurcation:Balance", 0.0).clamp(0.0, 1.0);
        len *= k;
        r0 = (r0 * k).max(0.005);
    }
    // Generation > Sink on a trunk (picture 57): with no parent branch, the base goes down into
    // the ground by Sink x the trunk's own radius
    if matches!(parent, ParentRef::Ground) {
        let sink = pex(p, "Generation:Sink", 0.0).max(0.0);
        start -= dir * sink * r0;
    }
    // Spine:Length:Match one v tile: exactly as long as one repeat of the bark texture along it
    if pex(p, "Spine:Length:Match one v tile", 0.0) > 0.5 {
        let v_style = pex(p, "UV Mapping:V:Style", 1.0) as i32;
        let u_rep = p.u_tile * pex(p, "UV Mapping:U:+ Relative", 1.0);
        let tile = match v_style {
            0 => 1.0 / p.v_tile.max(0.01),
            1 => 2.0 * PI * r0 / (u_rep * p.v_tile).max(0.01),
            _ => len,
        };
        len = tile;
    }
    if len < 0.05 {
        return None;
    }
    // Skin:Radius:Length compensation (fn 0x140141d60): c = compensation x radius x 2; a branch
    // shorter than c gets the full amount, one longer than 6c none ((length - c) / 5c). TreeCraft
    // uses the amount to thin short, stubby branches (up to half; estimated)
    let lc = pex(p, "Skin:Radius:Length compensation", 1.0);
    if lc > 0.0 {
        let c = lc * r0 * 2.0;
        let x = ((len - c) / (c * 5.0)).clamp(0.0, 1.0);
        r0 *= 1.0 - 0.5 * (1.0 - x);
        r0 = r0.max(0.005);
    }
    // Segments:Accuracy: how many steps the spine grows in (50 -> 12; picture 39); the skin's
    // segments are laid on it afterwards (resample_spine)
    let n = (pex(p, "Segments:Accuracy", 50.0) / 4.0).round().clamp(4.0, 64.0) as usize;
    let step = len / n as f32;
    // Spine:Noise:Late:Smooth slows the late noise down; Spine:Noise:Seed changes its pattern
    let f = (0.6 + 4.0 * p.noise_turb) * (1.0 - 0.8 * pex(p, "Spine:Noise:Late:Smooth", 0.0).clamp(0.0, 1.0));
    let nseed = pex(p, "Spine:Noise:Seed", 15000.0) * 0.0137;
    let ph = [rng.next() * 6.28 + nseed, rng.next() * 6.28 + nseed * 1.7, rng.next() * 6.28 + nseed * 2.3];
    // Spine:Shape:Straightness: less of every bend
    let straight = 1.0 - pex(p, "Spine:Shape:Straightness", 0.0).clamp(0.0, 1.0);
    let turb = pex(p, "Spine:Noise:Early:Turbulence", 0.5).clamp(0.0, 1.0);
    let mut jink_dir = Vec3::ZERO;
    let mut pts = Vec::with_capacity(n + 1);
    let mut d = dir;
    let mut pos = start;
    pts.push(pos);
    for k in 0..n {
        let kk = k as f32;
        let noise = vec3((kk * f + ph[0]).sin(), (kk * f * 1.13 + ph[1]).sin() * 0.5, (kk * f * 0.87 + ph[2]).sin());
        // Profile curves (yellow) along the branch's own length
        let s = (kk + 0.5) / n as f32;
        let (pn, pg) = (curve_at(p, "Spine:Noise:Late:Amount", 2, s), curve_at(p, "Spine:Shape:Gravity", 2, s));
        // Ancestor's second curve (picture 40): keep turning toward the parent's direction along the branch
        if let (Some(at), true) = (anc_dir, p.curves.contains_key("Spine:Shape:Ancestor|2")) {
            let k = pex(p, "Spine:Shape:Ancestor", 0.0).clamp(-1.0, 1.0) * curve_at(p, "Spine:Shape:Ancestor", 2, s) * 2.0 / n as f32;
            d = d.lerp(at, k).normalize_or(d);
        }
        d = (d + (noise * p.noise_amt * pn * 1.6 / n as f32 + Vec3::NEG_Y * p.gravity * pg * 2.5 / n as f32) * straight).normalize();
        // scene forces bend growth like gravity does: Direction pushes along its direction,
        // Magnet pulls toward its centre, weaker with distance (strength 1 ~ gravity 1)
        // Forces:Allow forces off: this generator ignores the scene's forces
        let forces: &[SceneObj] = if pex(p, "Forces:Allow forces", 1.0) > 0.5 { forces } else { &[] };
        for f in forces {
            let push = match f.kind {
                SceneKind::Direction => f.dir.normalize_or_zero(),
                SceneKind::Magnet => {
                    let v = f.pos - pos;
                    v.normalize_or_zero() / (1.0 + v.length() * 0.1)
                }
                _ => Vec3::ZERO,
            };
            let nd = (d + push * f.size * 2.5 / n as f32).normalize_or_zero();
            if nd != Vec3::ZERO {
                d = nd;
            }
        }
        if p.jink > 0.0 && rng.next() < p.jink_freq {
            // Spine:Noise:Early:Turbulence: 0 = every jink turns the same way as the last, 1 = each
            // turns anywhere
            let fresh = vec3(rng.sym(), rng.sym() * 0.3, rng.sym());
            jink_dir = if jink_dir == Vec3::ZERO { fresh } else { (jink_dir * (1.0 - turb) + fresh * turb).normalize_or(fresh) };
            d = (d + jink_dir * p.jink * curve_at(p, "Spine:Noise:Early:Amount", 2, s) * 0.5 * straight).normalize();
        }
        pos += d * step;
        pts.push(pos);
    }
    // Skin:Smooth (and a SubD skin's Subdivision:Smooth) round the spine's corners
    let mut sm = pex(p, "Skin:Smooth", 0.0).clamp(0.0, 1.0);
    let skin_type = pex(p, "Skin:Type", 0.0) as i32;
    if skin_type == 1 || skin_type == 2 {
        sm = sm.max(pex(p, "Skin:Subdivision:Smooth", 0.5).clamp(0.0, 1.0));
    }
    if sm > 0.0 && pts.len() > 2 {
        for _ in 0..(sm * 6.0).ceil() as usize {
            let prev = pts.clone();
            for k in 1..pts.len() - 1 {
                pts[k] = prev[k].lerp((prev[k - 1] + prev[k + 1]) * 0.5, sm * 0.5);
            }
        }
    }
    freehand_bend(&mut pts, p);
    let pts = resample_spine(&pts, p, len);
    Some(build_spine(node.id, pts, r0, p))
}

/// Freehand's bends (pictures 46, 47): every bend kept from earlier (Freehand:Kept<i>:...,
/// Freehand:KeptCount of them) in the order they were made, then the controller's own
/// (Freehand:...). Each one: above its pivot (Lower, a share of the length) the spine turns round
/// its Axis, little by little along its band up to the top (Upper), by Angle degrees in all; past
/// the top it keeps that direction. Shape is how the turning spreads over the band (1 evenly,
/// below 1 sooner near the pivot, above 1 later near the top). Lengths are kept.
fn freehand_bend(pts: &mut Vec<Vec3>, p: &Params) {
    let kept = pex(p, "Freehand:KeptCount", 0.0).max(0.0) as usize;
    for i in 0..kept.min(64) {
        bend_once(pts, p, &format!("Freehand:Kept{}:", i));
    }
    bend_once(pts, p, "Freehand:");
}

fn bend_once(pts: &mut Vec<Vec3>, p: &Params, pre: &str) {
    let get = |k: &str, d: f32| pex(p, &format!("{}{}", pre, k), d);
    let ang = get("Angle", 0.0);
    if ang.abs() < 0.01 || pts.len() < 2 {
        return;
    }
    let axis = vec3(get("AxisX", 0.0), get("AxisY", 0.0), get("AxisZ", 1.0)).normalize_or_zero();
    if axis == Vec3::ZERO {
        return;
    }
    let lo = get("Lower", 0.3).clamp(0.0, 0.95);
    let hi = get("Upper", 1.0).clamp(lo + 0.05, 1.0);
    let shape = get("Shape", 1.0).clamp(0.2, 4.0);
    let mut cum = vec![0.0f32; pts.len()];
    for k in 1..pts.len() {
        cum[k] = cum[k - 1] + pts[k].distance(pts[k - 1]);
    }
    let total = cum[pts.len() - 1].max(1e-5);
    let mut out = vec![pts[0]];
    for k in 1..pts.len() {
        let s = (cum[k - 1] + cum[k]) * 0.5 / total;
        let w = ((s - lo) / (hi - lo)).clamp(0.0, 1.0).powf(shape);
        let q = Quat::from_axis_angle(axis, ang.to_radians() * w);
        let prev = out[k - 1];
        out.push(prev + q * (pts[k] - pts[k - 1]));
    }
    *pts = out;
}

/// The Segments tab's Length settings (picture 39) laid on a grown spine: "+ Relative" segments
/// plus Absolute more per unit of length; Weld scalar crowds them toward the base (above 1) or
/// the tip (below 1); Optimization drops the points where the spine runs nearly straight (up to
/// 20 degrees of turn at 1).
fn resample_spine(pts: &[Vec3], p: &Params, len: f32) -> Vec<Vec3> {
    let m = pts.len();
    if m < 2 {
        return pts.to_vec();
    }
    let mut cum = vec![0.0f32; m];
    for k in 1..m {
        cum[k] = cum[k - 1] + pts[k].distance(pts[k - 1]);
    }
    let total = cum[m - 1].max(1e-5);
    let count = (p.seg_len as f32 + pex(p, "Segments:Length:Absolute", 0.0).max(0.0) * len).round().clamp(2.0, 128.0) as usize;
    let weld = pex(p, "Segments:Length:Weld scalar", 1.0).clamp(0.2, 5.0);
    let at = |d: f32| -> Vec3 {
        let mut k = 0;
        while k + 2 < m && cum[k + 1] < d {
            k += 1;
        }
        let f = ((d - cum[k]) / (cum[k + 1] - cum[k]).max(1e-6)).clamp(0.0, 1.0);
        pts[k].lerp(pts[k + 1], f)
    };
    // the Profile (yellow) curve on Segments:Length:Absolute (picture 40): how densely the
    // segments sit along the branch, base (left) to tip (right); high = more segments there
    let dens = p.curves.get("Segments:Length:Absolute|2");
    let place = |t: f32| -> f32 {
        let Some(c) = dens else { return t };
        const N: usize = 64;
        let mut cdf = [0.0f32; N + 1];
        for k in 0..N {
            let u = (k as f32 + 0.5) / N as f32;
            cdf[k + 1] = cdf[k] + c.eval(u).max(0.03);
        }
        let target = t * cdf[N];
        for k in 0..N {
            if cdf[k + 1] >= target {
                let f = (target - cdf[k]) / (cdf[k + 1] - cdf[k]).max(1e-6);
                return (k as f32 + f) / N as f32;
            }
        }
        1.0
    };
    let us: Vec<f32> = (0..=count).map(|i| place(i as f32 / count as f32).powf(weld)).collect();
    let mut out: Vec<Vec3> = us.iter().map(|u| at(u * total)).collect();
    let opt = pex(p, "Segments:Length:Optimization", 0.0).clamp(0.0, 1.0);
    if opt > 0.0 && out.len() > 3 {
        let limit = opt * 20f32.to_radians();
        let mut kept = vec![out[0]];
        for k in 1..out.len() - 1 {
            let a = (out[k] - *kept.last().unwrap()).normalize_or_zero();
            let b = (out[k + 1] - out[k]).normalize_or_zero();
            // its second curve: how much optimizing happens along the branch
            if a.angle_between(b) >= limit * curve_at(p, "Segments:Length:Optimization", 2, us[k]) {
                kept.push(out[k]);
            }
        }
        kept.push(out[out.len() - 1]);
        out = kept;
    }
    out
}

fn build_spine(node: u32, pts: Vec<Vec3>, r0: f32, p: &Params) -> Spine {
    let m = pts.len();
    let mut cum = vec![0.0; m];
    for k in 1..m {
        cum[k] = cum[k - 1] + pts[k].distance(pts[k - 1]);
    }
    let len = cum[m - 1].max(1e-4);
    let mut tan = vec![Vec3::Y; m];
    for k in 0..m {
        let a = if k == 0 { pts[0] } else { pts[k - 1] };
        let b = if k == m - 1 { pts[m - 1] } else { pts[k + 1] };
        tan[k] = (b - a).normalize_or_zero();
    }
    let mut nrm = vec![Vec3::X; m];
    nrm[0] = any_perp(tan[0]);
    for k in 1..m {
        let n = nrm[k - 1] - tan[k] * nrm[k - 1].dot(tan[k]);
        nrm[k] = if n.length_squared() > 1e-8 { n.normalize() } else { any_perp(tan[k]) };
    }
    let rad = (0..m)
        .map(|k| {
            let s = cum[k] / len;
            let mut r = r0 * (1.0 - (1.0 - p.taper) * s.powf(0.9)) * curve_at(p, "Skin:Radius:Absolute", 2, s) * curve_at(p, "Skin:Radius:+ % of parent", 2, s);
            if s < 0.18 {
                let q = 1.0 - s / 0.18;
                r *= 1.0 + p.flare * q * q;
            }
            r
        })
        .collect();
    Spine { node, pts, rad, tan, nrm, cum, len, mat: p.material, parent_spine: None, pos_on_parent: 0.0, ext_of: None, kept: 1.0 }
}

/// A UV Mapping property kept in Params::extra (or its default).
fn uv_ex(p: &Params, name: &str, def: f32) -> f32 {
    p.extra.get(name).map(|v| *v as f32).unwrap_or(def)
}

/// How a branch's bark is laid on its texture, from the UV Mapping properties (picture 32;
/// choices from fn 0x14012ff90). `r0` is the base radius, `len` the length, `flip` turns the twist
/// the other way (Random flip).
pub struct UvMap {
    /// Repeats around the branch.
    pub u_rep: f32,
    u_off: f32,
    /// Extra turns of U from base to tip.
    pub twist: f32,
    style: i32,
    v_style: i32,
    anchor_end: bool,
    v_scale: f32,
    v_off: f32,
    r0: f32,
    len: f32,
}

impl UvMap {
    pub fn new(p: &Params, r0: f32, len: f32, flip: bool) -> Self {
        let style = uv_ex(p, "UV Mapping:Style", 0.0) as i32;
        let mut u_rep = p.u_tile * uv_ex(p, "UV Mapping:U:+ Relative", 1.0);
        // Fractional: Enable makes Value the share of the texture taken around the branch
        if uv_ex(p, "UV Mapping:U:Fractional:Enable", 0.0) > 0.5 {
            u_rep *= uv_ex(p, "UV Mapping:U:Fractional:Value", 1.0);
        }
        let mut twist = uv_ex(p, "UV Mapping:U:Twist:Amount", 0.0);
        if flip && uv_ex(p, "UV Mapping:U:Twist:Random flip", 1.0) > 0.5 {
            twist = -twist;
        }
        // U locked [0, 1]: once around the branch, no offset or twist
        let locked_u = style >= 1;
        UvMap {
            u_rep: if locked_u { 1.0 } else { u_rep },
            u_off: if locked_u { 0.0 } else { uv_ex(p, "UV Mapping:U:Offset", 0.0) },
            twist: if locked_u { 0.0 } else { twist },
            style,
            v_style: uv_ex(p, "UV Mapping:V:Style", 1.0) as i32,
            anchor_end: uv_ex(p, "UV Mapping:V:Anchor", 0.0) > 0.5,
            v_scale: p.v_tile,
            v_off: uv_ex(p, "UV Mapping:V:Offset", 0.0),
            r0: r0.max(0.01),
            len: len.max(1e-4),
        }
    }

    /// The texture coordinate at `around` (0..1 round the branch) and `dist` along it.
    pub fn uv(&self, around: f32, dist: f32) -> Vec2 {
        let along = dist / self.len;
        let u = around * self.u_rep + self.u_off + self.twist * along;
        let d = if self.anchor_end { self.len - dist } else { dist };
        let mut v = match self.v_style {
            // Absolute: V Scale repeats per unit of length
            0 => d * self.v_scale,
            // Relative: V Scale repeats over the whole branch
            2 => d / self.len * self.v_scale,
            // Relative (keep aspect ratio): square texels, as wide around as along
            _ => d / (2.0 * PI * self.r0) * self.u_rep * self.v_scale,
        } + self.v_off;
        // U and V locked [0, 1]: base to tip once
        if self.style >= 2 {
            v = (d / self.len).clamp(0.0, 1.0);
        }
        vec2(u, v)
    }

    /// How far the texture runs: U at the end of the way round, V at the far end.
    pub fn extent(&self) -> (f32, f32) {
        (self.u_rep, (self.uv(0.0, self.len).y - self.uv(0.0, 0.0).y).abs())
    }

    /// Where the base starts in UV (offsets and the anchor move it).
    pub fn origin(&self) -> Vec2 {
        let a = self.uv(0.0, 0.0);
        let b = self.uv(0.0, self.len);
        vec2(a.x, a.y.min(b.y))
    }
}

/// The Skin tab's settings for one branch (picture 38), read once.
struct SkinLook {
    roll: f32,
    twist: f32,
    squash: f32,
    squash_rot: f32,
    seam: f32,
    // welding: where the branch meets its parent (parent radius and direction there)
    weld: Option<(f32, Vec3, Vec3)>,
    weld_offset: f32,
    spread: f32,
    upper: f32,
    lower: f32,
    blend_tex: f32,
    blend_contrast: f32,
    blend_noise: f32,
    /// Welding > Segments (TreeCraft's own, picture 59): extra rings in the weld zone (0 =
    /// automatic), how tightly they pack toward the parent, and how many more steps the rings
    /// there go round in.
    weld_rings: usize,
    weld_bias: f32,
    weld_radial: f32,
    /// Welding > Push (picture 61): the weld's surface moved into (-) or out of (+) the parent by
    /// this share of its radius, for parents whose surface isn't a round cylinder at the joint.
    weld_push: f32,
    // Intersection > Texture Blend
    pull: f32,
    pull_offset: f32,
    // SubD skins: more segments, texture density, smoothed radius
    subd: bool,
    tex_density: f32,
    relax: f32,
}

impl SkinLook {
    fn new(p: &Params, weld: Option<(f32, Vec3, Vec3)>) -> Self {
        let st = pex(p, "Skin:Type", 0.0) as i32;
        let subd = st == 1 || st == 2;
        // off by default (picture 56)
        let welding = pex(p, "Skin:Welding:Enabled", 0.0) > 0.5;
        let blend = pex(p, "Skin:Intersection:Texture Blend:Enabled", 1.0) > 0.5;
        // a SubD skin's Upper / Lower spread widen the weld on each side (2 = as set)
        let (us, ls) = if subd { (pex(p, "Skin:Subdivision:Upper spread", 2.0) * 0.5, pex(p, "Skin:Subdivision:Lower spread", 2.0) * 0.5) } else { (1.0, 1.0) };
        SkinLook {
            roll: pex(p, "Skin:Roll", 0.0) * 2.0 * PI,
            twist: pex(p, "Spine:Shape:Twist", 0.0) * 2.0 * PI,
            squash: pex(p, "Skin:Squash:Amount", 0.0).clamp(0.0, 0.95),
            squash_rot: pex(p, "Skin:Squash:Rotate", 0.0) * 2.0 * PI,
            seam: pex(p, "Skin:Light seam reduction", 0.0).clamp(0.0, 1.0),
            weld: if welding { weld } else { None },
            weld_offset: pex(p, "Skin:Welding:Offset", 1.0).max(0.0),
            spread: pex(p, "Skin:Welding:Spread:Amount", 0.1).max(0.0),
            upper: pex(p, "Skin:Welding:Spread:Upper scale", 0.25).max(0.0) * us,
            lower: pex(p, "Skin:Welding:Spread:Lower scale", 0.25).max(0.0) * ls,
            blend_tex: pex(p, "Skin:Welding:Blend:Texture", 0.75).clamp(0.0, 1.0),
            blend_contrast: pex(p, "Skin:Welding:Blend:Contrast", 5.0).max(0.0),
            blend_noise: pex(p, "Skin:Welding:Blend:Noise", 3.0).max(0.0),
            weld_rings: pex(p, "Skin:Welding:Segments:Rings", 0.0).max(0.0).round() as usize,
            weld_bias: pex(p, "Skin:Welding:Segments:Ring bias", 1.35).clamp(0.3, 5.0),
            weld_radial: pex(p, "Skin:Welding:Segments:Radial detail", 2.0).clamp(1.0, 4.0),
            weld_push: pex(p, "Skin:Welding:Push", 0.0).clamp(-0.9, 2.0),
            pull: if blend && weld.is_some() { pex(p, "Skin:Intersection:Texture Blend:Pull", 0.2) } else { 0.0 },
            pull_offset: pex(p, "Skin:Intersection:Texture Blend:Offset", 0.0),
            subd,
            tex_density: if subd { pex(p, "Skin:Subdivision:Texture density", 1.0).max(0.01) } else { 1.0 },
            relax: if subd { pex(p, "Skin:Subdivision:Relax scale", 1.0).clamp(0.0, 2.0) } else { 0.0 },
        }
    }
}

/// A branch's bark, with its Skin settings; then any Splits (extra forks) off it.
#[allow(clippy::too_many_arguments)]
fn skin(s: &Spine, p: &Params, col: [u8; 4], geo: &mut TreeGeo, trunk: bool, weld: Option<(f32, Vec3, Vec3)>, gen: u32, node: usize) {
    let mut look = SkinLook::new(p, weld);
    // Welding (picture 57): a branch running almost along its parent can't be welded unless
    // Force ray casts; a failed weld drops the branch's bark unless Keep failed
    if let Some((_, ptan, _)) = look.weld {
        let d0 = (s.pts[1.min(s.pts.len() - 1)] - s.pts[0]).normalize_or(ptan);
        if d0.dot(ptan).abs() > 0.985 && pex(p, "Skin:Welding:Force ray casts", 0.0) < 0.5 {
            look.weld = None;
            if pex(p, "Skin:Welding:Keep failed", 0.0) < 0.5 {
                return;
            }
        }
    }
    skin_tube(s, p, col, geo, trunk, &look);
    let flat = SkinLook { weld: None, pull: 0.0, ..SkinLook::new(p, None) };
    for fork in split_forks(s, p, gen, node) {
        skin_tube(&fork, p, col, geo, trunk, &flat);
    }
}

/// Skin:Splits: by Chance (Seed), the branch forks at two thirds of its length: one fork at
/// Spread 1 (x 90 degrees), another at Spread 2; Balance shares the radius between the main
/// branch and its forks; Smooth Outer bends a fork out gradually, Smooth Inner lets it start
/// inside the branch. Each fork is marked as Split 1 or Split 2 of `s` (Extend parent, picture 86;
/// the caller fills in which spine `s` is).
fn split_forks(s: &Spine, p: &Params, gen: u32, node: usize) -> Vec<Spine> {
    let mut out = vec![];
    let chance = pex(p, "Skin:Splits:Chance", 0.0);
    if chance <= 0.0 || s.pts.len() < 3 {
        return out;
    }
    let seed = pex(p, "Skin:Splits:Seed", 15000.0) as u64;
    if hash01(seed, gen as u64, node as u64) * 0.5 + 0.5 >= chance {
        return out;
    }
    let at = 0.66;
    let (tn, n, b) = s.frame(at);
    let start = s.point(at);
    let rest = s.len * (1.0 - at);
    let rs = s.radius(at);
    let bal = pex(p, "Skin:Splits:Balance", 0.5).clamp(0.05, 0.95);
    let outer = pex(p, "Skin:Splits:Smooth:Outer", 0.5).clamp(0.0, 1.0);
    let inner = pex(p, "Skin:Splits:Smooth:Inner", 0.25).clamp(0.0, 1.0);
    for (k, key) in ["Skin:Splits:Spread 1", "Skin:Splits:Spread 2"].iter().enumerate() {
        let spread = pex(p, key, 0.0);
        if spread <= 0.0 {
            continue;
        }
        let side = if k == 0 { n } else { -n * 0.5 + b * 0.866 };
        let ang = spread.clamp(0.0, 1.0) * PI * 0.5;
        let dir = (tn * ang.cos() + side * ang.sin()).normalize();
        let r = (rs * (1.0 - bal).sqrt()).max(0.005);
        let p0 = start - side * r * inner;
        let ctrl = p0 + (tn * outer + dir * (1.0 - outer)).normalize_or(dir) * rest * 0.5;
        let end = p0 + dir * rest;
        let pts: Vec<Vec3> = (0..=8)
            .map(|i| {
                let u = i as f32 / 8.0;
                p0 * (1.0 - u) * (1.0 - u) + ctrl * 2.0 * u * (1.0 - u) + end * u * u
            })
            .collect();
        let mut fork = build_spine(s.node, pts, r, p);
        fork.mat = s.mat;
        fork.pos_on_parent = at;
        fork.ext_of = Some((usize::MAX, k as u8 + 1));
        out.push(fork);
    }
    out
}

/// Welding's extra rings (pictures 58, 59): the weld zone gets Welding > Segments > Rings rings
/// (0 = automatic: 8 + 6 x Offset, up to 64), packed toward the parent by Ring bias, so the base
/// can bend smoothly onto it; the rest of the branch keeps its own points.
fn densify_base(s: &Spine, zone: f32, offset: f32, rings: usize, bias: f32) -> Spine {
    let n = if rings > 0 { rings.clamp(2, 96) } else { ((8.0 + 6.0 * offset).round() as usize).clamp(6, 64) };
    let mut ds: Vec<f32> = (0..=n).map(|i| zone * (i as f32 / n as f32).powf(bias.clamp(0.3, 5.0))).collect();
    ds.extend(s.cum.iter().copied().filter(|c| *c > zone * 1.0001));
    let len = s.len.max(1e-6);
    let mut out = Spine { node: s.node, pts: vec![], rad: vec![], tan: vec![], nrm: vec![], cum: vec![], len: s.len, mat: s.mat, parent_spine: s.parent_spine, pos_on_parent: s.pos_on_parent, ext_of: s.ext_of, kept: s.kept };
    for d in ds {
        let t = (d / len).clamp(0.0, 1.0);
        let (tn, nn, _) = s.frame(t);
        out.pts.push(s.point(t));
        out.rad.push(s.radius(t));
        out.tan.push(tn);
        out.nrm.push(nn);
        out.cum.push(d.min(s.len));
    }
    out
}

/// The weld fillet's size along the branch and along the parent (picture 73): Spread x the side's
/// scale (Upper toward the parent's tip, Lower toward its base) decides how far the rounded joint
/// reaches, Offset scales it all; at least a fifth of the branch's radius.
fn weld_fillet(r0: f32, spread: f32, scale: f32, offset: f32) -> f32 {
    (r0 * (0.2 + spread.max(0.0) * 6.0 * scale.max(0.0)) * (0.5 + 0.5 * offset.max(0.0))).max(r0 * 0.05).max(1e-3)
}

fn skin_tube(s0: &Spine, p: &Params, col: [u8; 4], geo: &mut TreeGeo, trunk: bool, look: &SkinLook) {
    // the weld zone (picture 59): from the parent's middle to where the branch's lowest side
    // leaves the parent (further for a branch leaving at a shallow angle), plus Offset x the
    // parent radius of fillet
    let zone = look.weld.map_or(0.0, |(pr, ptan, _)| {
        let d0 = if s0.pts.len() > 1 { (s0.pts[1] - s0.pts[0]).normalize_or(Vec3::Y) } else { Vec3::Y };
        let c = d0.dot(ptan.normalize_or(Vec3::Y)).abs();
        let sn = (1.0 - c * c).sqrt().max(0.25);
        // (picture 73) as far as the furthest column leaves the parent, plus its fillet
        let biggest = weld_fillet(s0.rad[0], look.spread * 1.5, look.upper.max(look.lower), look.weld_offset);
        (pr + s0.rad[0] * c) / sn + biggest * 1.1
    });
    let dense;
    let s: &Spine = if zone > 0.0 && s0.len > zone * 0.2 && s0.pts.len() >= 2 {
        dense = densify_base(s0, zone.min(s0.len), look.weld_offset, look.weld_rings, look.weld_bias);
        &dense
    } else {
        s0
    };
    let mat_key = s.mat;
    // Segments:Radial: Absolute, plus "+ Relative" more for every quarter unit of radius (picture 39)
    let rel = (pex(p, "Segments:Radial: + Relative", 2.0).max(0.0) * s.rad[0] * 4.0).round() as i32;
    let mut radial = (p.seg_radial + rel).clamp(3, 64) as usize;
    if look.subd {
        radial = (radial * 2).min(64);
    }
    let m = s.pts.len();
    // Segments:Radial:Absolute's second curve (picture 41): the count around can change along
    // the branch; rings of different counts are stitched together
    // (welded, the rings in the weld zone go round in twice as many steps, picture 58)
    let ring_n: Vec<usize> = (0..m)
        .map(|k| {
            let n = ((radial as f32 * curve_at(p, "Segments:Radial:Absolute", 2, s.cum[k] / s.len.max(1e-6))).round() as usize).clamp(3, 64);
            if zone > 0.0 && s.cum[k] < zone { ((n as f32 * look.weld_radial).round() as usize).clamp(3, 96) } else { n }
        })
        .collect();
    let need: usize = ring_n.iter().map(|r| r + 1).sum::<usize>() + 2;
    let r0 = s.rad[0].max(0.01);
    // each branch flips its twist (Random flip) by where it starts
    let flip = ((s.pts[0].x * 12.9898 + s.pts[0].z * 78.233).sin() * 43758.547).fract().abs() < 0.5;
    let uvm = UvMap::new(p, r0, s.len / look.tex_density, flip);
    // a SubD skin relaxes its radius along the branch (Relax scale)
    let mut rad = s.rad.clone();
    if look.relax > 0.0 && m > 2 {
        for _ in 0..(look.relax * 3.0).ceil() as usize {
            let prev = rad.clone();
            for k in 1..m - 1 {
                let relax = (look.relax * curve_at(p, "Skin:Subdivision:Relax scale", 2, s.cum[k] / s.len)).min(1.0);
                rad[k] = prev[k] + ((prev[k - 1] + prev[k + 1]) * 0.5 - prev[k]) * relax * 0.5;
            }
        }
    }
    let mat = geo.mats.entry(mat_key).or_default();
    let ch = mat.chunk_for(need);
    let base = ch.v.len();
    let mut bmin = Vec3::splat(f32::MAX);
    let mut bmax = Vec3::splat(f32::MIN);
    let mut ring_start: Vec<usize> = Vec::with_capacity(m);
    // the weld solver's cut distance per column (ring step count, column)
    let mut cross_cache: HashMap<(usize, usize), f32> = HashMap::new();
    for k in 0..m {
        let radial = ring_n[k];
        ring_start.push(ch.v.len());
        let tn = s.tan[k];
        let n = s.nrm[k];
        let b = tn.cross(n);
        let sfrac = s.cum[k] / s.len;
        let dist = s.cum[k];
        let mut ao = if trunk { 0.8 + 0.2 * sfrac } else { 0.62 + 0.38 * sfrac };
        // Intersection > Texture Blend: near the parent the bark texture is pulled along the
        // branch (Pull), the pull starting further out with Offset
        let wp = if zone > 0.0 { (1.0 - (dist - look.pull_offset * zone) / zone).clamp(0.0, 1.0) } else { 0.0 };
        let tex_dist = (dist + look.pull * wp * wp * zone * 2.0) / look.tex_density;
        let ao_k = ao;
        // the settings' second curves, read along the branch (picture 40)
        let squash = (look.squash * curve_at(p, "Skin:Squash:Amount", 2, sfrac)).clamp(0.0, 0.95);
        let squash_rot = look.squash_rot * curve_at(p, "Skin:Squash:Rotate", 2, sfrac);
        let spread = look.spread * curve_at(p, "Skin:Welding:Spread:Amount", 2, sfrac);
        let seam = look.seam * curve_at(p, "Skin:Light seam reduction", 2, sfrac);
        for j in 0..=radial {
            ao = ao_k;
            // Spine:Shape:Twist turns the skin around the branch along its length; Skin:Roll
            // turns the whole skin (and its texture seam) around the spine
            let a = j as f32 / radial as f32 * 2.0 * PI + look.twist * sfrac + look.roll;
            // Skin:Squash flattens the cross-section by Amount, turned by Rotate
            let (cr, sr) = (squash_rot.cos(), squash_rot.sin());
            let (ca, sa) = ((a - squash_rot).cos(), (a - squash_rot).sin() * (1.0 - squash));
            let dir = n * (ca * cr - sa * sr) + b * (ca * sr + sa * cr);
            let (na, nb) = ((a - squash_rot).cos() * (1.0 - squash), (a - squash_rot).sin());
            let mut nrm = (n * (na * cr - nb * sr) + b * (na * sr + nb * cr)).normalize_or(dir);
            let mut r = rad[k];
            // Welding > Blend's texture fade (picture 58): how far this spot of the base dissolves
            // into the parent's bark (0 solid .. 1 gone)
            let mut fade = 0.0f32;
            let mut pos = s.pts[k] + dir * r;
            if let (true, Some((pr0, ptan0, pp))) = (dist < zone, look.weld) {
                // (the cut is found on the parent's own surface; Push moves the result, below)
                let pr = pr0;
                // TreeCraft's weld solver (picture 59). The parent is taken as a cylinder along its
                // direction at the joint. Each column of the base (one angle round the branch) is
                // cut where it enters the parent, like a boolean cut: every vertex before the cut
                // sits at the cut point on the parent's surface, so nothing of the branch is left
                // inside the parent. After the cut, the base widens (Spread, more toward the
                // parent's tip by Upper scale or its base by Lower scale) and settles down onto
                // the parent's surface over a fillet as long as Offset x the parent radius, so it
                // meets the parent smoothly.
                let ptan = ptan0.normalize_or(Vec3::Y);
                let radial_of = |q: Vec3| -> (f32, Vec3) {
                    let v = q - pp;
                    let perp = v - ptan * v.dot(ptan);
                    (perp.length(), perp.normalize_or(dir))
                };
                // which side of the branch this column faces: toward the parent's tip (Upper scale)
                // or its base (Lower scale), measured against the most a column can lean that way
                let d_axis = (s.pts[1.min(s.pts.len() - 1)] - s.pts[0]).normalize_or(Vec3::Y);
                let lean_max = (1.0 - d_axis.dot(ptan).powi(2)).max(0.0).sqrt().max(0.05);
                let side = (dir.normalize_or(dir).dot(ptan) / lean_max).clamp(-1.0, 1.0);
                let mid = (look.upper + look.lower) * 0.5;
                let scale = mid + ((if side >= 0.0 { look.upper } else { look.lower }) - mid) * side.abs();
                // the fillet's size (picture 73): Spread x the side's scale, times Offset
                let fsize = weld_fillet(s.rad[0], spread, scale, look.weld_offset);
                // a point of this column (the branch's own surface, not widened) at distance d
                let frame_dir = |d: f32| -> (Vec3, Vec3) {
                    let t = (d / s.len.max(1e-6)).clamp(0.0, 1.0);
                    let (tn2, n2, _) = s.frame(t);
                    let b2 = tn2.cross(n2);
                    (s.point(t), n2 * (ca * cr - sa * sr) + b2 * (ca * sr + sa * cr))
                };
                let col_at = |d: f32| -> Vec3 {
                    let (p0, d2) = frame_dir(d);
                    let t = (d / s.len.max(1e-6)).clamp(0.0, 1.0);
                    p0 + d2 * s.radius(t)
                };
                // where the column leaves the parent: march out, then halve the step
                let key = (radial, j);
                let cut = match cross_cache.get(&key) {
                    Some(c) => *c,
                    None => {
                        let steps = 48;
                        let mut prev = 0.0f32;
                        let mut found = zone;
                        if radial_of(col_at(0.0)).0 >= pr {
                            found = 0.0;
                        } else {
                            for i in 1..=steps {
                                let d = zone * i as f32 / steps as f32;
                                if radial_of(col_at(d)).0 >= pr {
                                    let (mut lo, mut hi) = (prev, d);
                                    for _ in 0..12 {
                                        let mid = (lo + hi) * 0.5;
                                        if radial_of(col_at(mid)).0 >= pr { hi = mid } else { lo = mid }
                                    }
                                    found = hi;
                                    break;
                                }
                                prev = d;
                            }
                        }
                        cross_cache.insert(key, found);
                        found
                    }
                };
                // TreeCraft's weld fillet (picture 73): each column runs from the parent's surface
                // to the branch's side along one smooth curve, tangent to both, like a rounded
                // corner. C is where the column meets the parent; T1 is fsize further up the branch,
                // T2 fsize along the parent's surface away from the branch. The curve is a cubic
                // from T2 to T1 whose handles reach 55% of the way to C, close to a circular arc,
                // so it fills the corner inward on both sides: a gentle flare where the branch
                // leans away from the parent, a round fillet in the tight crotch. Nothing bulges out.
                let surf = |q: Vec3| -> Vec3 {
                    let v = q - pp;
                    let (_, o) = radial_of(q);
                    pp + ptan * v.dot(ptan) + o * (pr * 1.003)
                };
                let corner = surf(col_at(cut));
                let t1 = col_at(cut + fsize);
                let (_, oc) = radial_of(corner);
                let (_, dir_c) = frame_dir(cut);
                let mut away = dir_c - oc * dir_c.dot(oc);
                if away.length_squared() < 1e-8 {
                    let db = (col_at(cut + zone * 0.01) - col_at(cut)).normalize_or(Vec3::Y);
                    away = -db - oc * (-db).dot(oc);
                }
                let away = away.normalize_or(dir_c);
                let t2 = surf(corner + away * fsize);
                let end = cut + fsize;
                let (hug, out) = if dist < end {
                    let u = (dist / end.max(1e-6)).clamp(0.0, 1.0);
                    const K: f32 = 0.55;
                    let (p0, p1, p2, p3) = (t2, t2 + (corner - t2) * K, t1 + (corner - t1) * K, t1);
                    let v = 1.0 - u;
                    pos = p0 * (v * v * v) + p1 * (3.0 * v * v * u) + p2 * (3.0 * v * u * u) + p3 * (u * u * u);
                    // the curve's normal turns from the parent's to the branch's along it
                    let (_, o) = radial_of(pos);
                    let w = u * u * (3.0 - 2.0 * u);
                    nrm = (o * (1.0 - w) + dir * w).normalize_or(nrm);
                    (1.0 - w, o)
                } else {
                    pos = s.pts[k] + dir * r;
                    let (_, o) = radial_of(pos);
                    (0.0, o)
                };
                // never inside the parent
                let (rd, o) = radial_of(pos);
                if rd < pr * 1.003 {
                    pos += o * (pr * 1.003 - rd);
                }
                // Push (pictures 61, 62): the whole edge near the parent moves in (-) or out (+)
                // along the parent's surface direction by Push x its radius, evenly all the way
                // round, fading out over the fillet; pushed in, it may go that far inside
                if look.weld_push != 0.0 {
                    let (_, o2) = radial_of(pos);
                    pos += o2 * (look.weld_push * pr * hug);
                }
                // Welding > Blend: the joint's shading turns to the parent's (its surface
                // direction) by Texture, sharper with Contrast, ragged with Noise; a faint
                // darkening keeps the crease; the fade dissolves the base into the parent's bark
                let jag = 1.0 + look.blend_noise * 0.05 * (a * 7.0 + dist * 13.0).sin();
                let wb = (hug * jag).clamp(0.0, 1.0).powf(1.0 + look.blend_contrast * 0.15);
                nrm = nrm.lerp(out, (look.blend_tex * wb).clamp(0.0, 1.0)).normalize_or(nrm);
                ao = ao + (0.86 - ao) * hug;
                ao *= 1.0 - 0.06 * look.blend_tex * wb;
                // the dissolving texture fade (picture 58) is off for now (picture 60: it left a
                // gap where the base meets the parent; texture blending comes later). The rule it
                // used, kept for then: fade = Texture x smoothstep((hug - 0.5) x (1 + 0.3 Contrast)
                // + 0.5) x ragged(Noise)
                const TEXTURE_FADE: bool = false;
                if TEXTURE_FADE {
                    let c = 1.0 + look.blend_contrast * 0.3;
                    let h2 = ((hug - 0.5) * c + 0.5).clamp(0.0, 1.0);
                    fade = (look.blend_tex * h2 * h2 * (3.0 - 2.0 * h2) * jag.min(1.2)).clamp(0.0, 1.0);
                }
            }
            // Skin:Light seam reduction: near the base the normals lean toward the direction the
            // branch leaves its parent, so the joint's lighting matches
            if seam > 0.0 && sfrac < 0.2 {
                let k2 = seam * (1.0 - sfrac / 0.2);
                nrm = nrm.lerp(s.tan[0], k2 * 0.8).normalize_or(nrm);
            }
            bmin = bmin.min(pos);
            bmax = bmax.max(pos);
            // the fade rides in the colour's alpha, below the selection mark's range: unselected
            // 255 down to 235, selected 128 down to 108 (render.rs reads it back)
            let mut vc = col;
            // the material's gradient (picture 74): base colour to tip colour along the branch
            if let Some((ga, gb, gm)) = geo.grad.get(&mat_key) {
                let f = crate::colorwheel::grad_t(sfrac, *gm);
                for q in 0..3 {
                    vc[q] = ((ga[q] + (gb[q] - ga[q]) * f).clamp(0.0, 1.0) * col[q] as f32).round() as u8;
                }
            }
            if fade > 0.0 {
                vc[3] = col[3].saturating_sub((fade * 20.0).round() as u8);
            }
            ch.v.push(vtx(pos, uvm.uv(j as f32 / radial as f32, tex_dist), vc, nrm, ao));
        }
    }
    let mut tris = 0usize;
    for k in 0..m - 1 {
        let (na, nb) = (ring_n[k], ring_n[k + 1]);
        let (sa, sb) = (ring_start[k], ring_start[k + 1]);
        if na == nb {
            for j in 0..na {
                let a = (sa + j) as u16;
                let b = a + 1;
                let c = (sb + j) as u16;
                let d = c + 1;
                // counter-clockwise seen from outside, so the outer faces are front faces (picture 42:
                // the other way round the shader turned their normals inward, lighting the bark from below)
                ch.i.extend_from_slice(&[a, b, c, b, d, c]);
            }
            tris += na * 2;
        } else {
            // a zipper: walk both rings round, always stepping the one whose next point comes first
            let (mut i, mut j) = (0usize, 0usize);
            while i < na || j < nb {
                let next_a = (i + 1) as f32 / na as f32;
                let next_b = (j + 1) as f32 / nb as f32;
                if j >= nb || (i < na && next_a <= next_b) {
                    ch.i.extend_from_slice(&[(sa + i) as u16, (sa + i + 1) as u16, (sb + j) as u16]);
                    i += 1;
                } else {
                    ch.i.extend_from_slice(&[(sa + i) as u16, (sb + j + 1) as u16, (sb + j) as u16]);
                    j += 1;
                }
                tris += 1;
            }
        }
    }
    let _ = base;
    // the caps take the gradient's colours too (picture 75)
    let grad = geo.grad.get(&mat_key).copied();
    let grad_col = |f: f32| -> [u8; 4] {
        let mut vc = col;
        if let Some((ga, gb, gm)) = grad {
            let f = crate::colorwheel::grad_t(f, gm);
            for q in 0..3 {
                vc[q] = ((ga[q] + (gb[q] - ga[q]) * f).clamp(0.0, 1.0) * col[q] as f32).round() as u8;
            }
        }
        vc
    };
    // ends: Gen > General > Cap tip (or Skin > Cap ends) closes the tip with a low dome half its
    // radius past the last ring (picture 71); open unless asked (picture 85). Cap ends also
    // closes the base (open by default, picture 34, since it sits in the parent)
    if (p.cap_tip || p.cap_ends) && (rad[m - 1] > 1e-4 || s.rad[m - 1] > 1e-4) {
        let tip = s.pts[m - 1] + s.tan[m - 1] * s.rad[m - 1] * 0.5;
        let ci = ch.v.len() as u16;
        ch.v.push(vtx(tip, uvm.uv(0.5, s.cum[m - 1]), grad_col(1.0), s.tan[m - 1], 1.0));
        let last = ring_start[m - 1] as u16;
        for j in 0..ring_n[m - 1] as u16 {
            ch.i.extend_from_slice(&[last + j, last + j + 1, ci]);
        }
        geo.tris += ring_n[m - 1];
    }
    if p.cap_ends {
        let bi = ch.v.len() as u16;
        ch.v.push(vtx(s.pts[0], uvm.uv(0.5, 0.0), grad_col(0.0), -s.tan[0], 1.0));
        let first = ring_start[0] as u16;
        for j in 0..ring_n[0] as u16 {
            ch.i.extend_from_slice(&[first + j + 1, first + j, bi]);
        }
        geo.tris += ring_n[0];
    }
    geo.tris += tris;
    geo.verts += need;
    geo.grow(bmin);
    geo.grow(bmax);
}

/// A frond (picture 65): a flat card laid along the parent's spine from `a` to `b`, in the plane of
/// the spine and its side direction, so it bends wherever the parent bends. Width = Leaves > Size x
/// Size scalar, widest in the middle and 35% of that at both ends (an estimate). Texture: U across,
/// V along the card.
fn make_frond(s: &Spine, a: f32, b: f32, p: &Params, col: [u8; 4], geo: &mut TreeGeo) {
    let steps = ((s.pts.len() as f32 * (b - a)).ceil() as usize).clamp(6, 64);
    let half = (p.leaf_size * p.size_scalar).max(0.02) * 0.5;
    let mat_idx = p.leaf_material;
    let mg = geo.mats.entry(mat_idx).or_default();
    let ch = mg.chunk_for((steps + 1) * 2);
    let start = ch.v.len();
    let mut pts = Vec::with_capacity((steps + 1) * 2);
    for k in 0..=steps {
        let f = k as f32 / steps as f32;
        let t = a + (b - a) * f;
        let (_tn, side, nrm) = s.frame(t);
        let c = s.point(t);
        let w = half * (0.35 + 0.65 * (PI * f).sin()) * p.scale_x.max(0.01);
        let l = c - side * w;
        let r = c + side * w;
        ch.v.push(vtx(l, vec2(0.0, 1.0 - f), col, nrm, 1.0));
        ch.v.push(vtx(r, vec2(1.0, 1.0 - f), col, nrm, 1.0));
        pts.push(l);
        pts.push(r);
    }
    for k in 0..steps {
        let i0 = (start + k * 2) as u16;
        ch.i.extend_from_slice(&[i0, i0 + 1, i0 + 3, i0, i0 + 3, i0 + 2]);
    }
    geo.tris += steps * 2;
    geo.verts += pts.len();
    for q in pts {
        geo.grow(q);
    }
}

#[allow(clippy::too_many_arguments)]
fn make_leaf(
    ctx: &GenCtx,
    p: &Params,
    parent: ParentRef,
    t: f32,
    roll: f32,
    ground_pos: Vec3,
    spines: &[Spine],
    anchors: &[AnchorFrame],
    rng: &mut Rng,
    col: [u8; 4],
    geo: &mut TreeGeo,
    cache: &[Option<MeshGeom>],
    leaf_ranges: &mut Vec<(usize, usize, usize)>,
) -> Vec<AnchorFrame> {
    let angle = (p.start_angle + p.angle_var * rng.sym()).clamp(0.0, 1.0) * PI;
    let (base, tn, perp, plen) = match parent {
        ParentRef::Ground => (ground_pos, Vec3::Y, vec3(roll.cos(), 0.0, roll.sin()), 0.0),
        ParentRef::Spine(i) => {
            let s = &spines[i];
            let (tn, n, b) = s.frame(t);
            let perp = n * roll.cos() + b * roll.sin();
            (s.point(t) + perp * s.radius(t) * 0.8, tn, perp, s.len)
        }
        ParentRef::Anchor(i) => {
            // along the anchor's line (Start angle 0), rolled around it by Roll
            let a = anchors[i];
            let perp = Quat::from_axis_angle(a.dir, roll) * a.nrm;
            (a.pos, a.dir, perp, 0.0)
        }
    };
    let mut out = (tn * angle.cos() + perp * angle.sin()).normalize();
    out = (out + Vec3::NEG_Y * p.droop).normalize_or_zero();
    if out.length_squared() < 0.5 {
        out = perp;
    }
    let side = tn.cross(perp).normalize_or_zero();
    let mut nrm = out.cross(side).normalize_or_zero();
    if nrm.length_squared() < 0.5 {
        nrm = any_perp(out);
    }
    if nrm.y < 0.0 {
        nrm = -nrm;
    }
    let up_o = (Vec3::Y - out * Vec3::Y.dot(out)).normalize_or_zero();
    nrm = (nrm * (1.0 - p.face_sky) + up_o * p.face_sky).normalize_or_zero();
    nrm = (nrm - out * nrm.dot(out)).normalize_or_zero();
    if nrm.length_squared() < 0.5 {
        nrm = any_perp(out);
    }
    if p.leaf_twist != 0.0 {
        let q = Quat::from_axis_angle(out, p.leaf_twist * PI * (0.5 + 0.5 * rng.next()));
        nrm = q * nrm;
    }
    let right = out.cross(nrm).normalize_or_zero();
    let size = ((p.leaf_size + p.leaf_size_pct * plen) * (1.0 + p.leaf_size_var * rng.sym()) * p.size_scalar).max(0.01);
    // leaf collision removal (Post > Collision): a leaf whose centre comes closer than a share of
    // its size to a leaf already placed is left out; High keeps leaves furthest apart
    if ctx.leaf_collision > 0 && !matches!(parent, ParentRef::Anchor(_)) {
        let c = base + out * size * 0.5;
        let reach = size * [0.0, 0.35, 0.6, 0.85][ctx.leaf_collision.min(3) as usize];
        let cell = |v: Vec3| ((v.x).floor() as i32, (v.y).floor() as i32, (v.z).floor() as i32);
        let (cx, cy, cz) = cell(c);
        let r = reach.ceil() as i32;
        let mut hit = false;
        'look: for dx in -r..=r {
            for dy in -r..=r {
                for dz in -r..=r {
                    if let Some(v) = geo.leaf_grid.get(&(cx + dx, cy + dy, cz + dz)) {
                        if v.iter().any(|q| q.distance(c) < reach) {
                            hit = true;
                            break 'look;
                        }
                    }
                }
            }
        }
        if hit {
            geo.leaves_removed += 1;
            return vec![];
        }
        geo.leaf_grid.entry((cx, cy, cz)).or_default().push(c);
    }

    // the mesh, already processed (Orient, Flip, Scale, Fix winding, Flip normals)
    let mut mi = if p.leaf_mesh <= ctx.meshes.len() { p.leaf_mesh } else { 0 };
    // a chosen mesh with no shape yet (no cutout, no loaded file) falls back to the plain card,
    // so a new leaf generator always shows something (picture 65)
    if !cache.get(mi).and_then(|g| g.as_ref()).map_or(false, |g| !g.is_empty()) {
        mi = 0;
    }
    let g = match cache.get(mi).and_then(|g| g.as_ref()) {
        Some(g) if !g.is_empty() => g,
        _ => return vec![],
    };
    // the mesh's Growth axis points along the leaf; its face normal follows the leaf normal
    let growth = if mi == 0 { 1 } else { ctx.meshes[mi - 1].growth };
    let (ax_r, ax_g, ax_n) = crate::meshio::growth_frame(growth);
    let fold = p.fold * PI * 0.5;
    let mut pos: Vec<Vec3> = Vec::with_capacity(g.pos.len());
    let mut src_n: Vec<Vec3> = Vec::with_capacity(g.pos.len());
    for (k, q) in g.pos.iter().enumerate() {
        let x = q.dot(ax_r) * size * p.scale_x;
        let y = q.dot(ax_g) * size * p.scale_y;
        let z0 = q.dot(ax_n) * size;
        let xf = x * fold.cos();
        let mut z = z0 + x.abs() * fold.sin();
        let yn = y / size.max(0.001);
        z += p.curl * yn * yn * size * 0.6;
        pos.push(base + right * xf + out * y + nrm * z);
        let sn = g.nrm[k];
        src_n.push((right * sn.dot(ax_r) + out * sn.dot(ax_g) + nrm * sn.dot(ax_n)).normalize_or_zero());
    }
    // smooth normals from the bent faces, pointing the same way as the mesh's own normals
    let mut ns = vec![Vec3::ZERO; pos.len()];
    for f in g.idx.chunks(3) {
        let (a, b, c) = (f[0] as usize, f[1] as usize, f[2] as usize);
        let fnn = (pos[b] - pos[a]).cross(pos[c] - pos[a]);
        ns[a] += fnn;
        ns[b] += fnn;
        ns[c] += fnn;
    }
    // material variation colour
    let mat_idx = p.leaf_material;
    let mut c = col;
    if let Some(m) = ctx.materials.get(mat_idx) {
        if m.variation > 0.0 {
            let vc = m.var_colors[(rng.next() * 3.0) as usize % 3];
            let mx = vc[0].max(vc[1]).max(vc[2]).max(0.01);
            for k in 0..3 {
                let mult: f32 = 1.0 + (vc[k] / mx - 1.0) * m.variation * 0.6;
                c[k] = (c[k] as f32 * mult.clamp(0.0, 1.0)) as u8;
            }
        }
    }
    let mg = geo.mats.entry(mat_idx).or_default();
    let ch = mg.chunk_for(pos.len());
    let start = ch.v.len();
    for k in 0..pos.len() {
        let mut n = ns[k].normalize_or_zero();
        if src_n[k].length_squared() > 0.0 && n.dot(src_n[k]) < 0.0 {
            n = -n;
        }
        if n.length_squared() < 0.5 {
            n = src_n[k];
        }
        // vertex colours painted in the Edit Cutout window tint the leaf
        let mut ck = c;
        if let Some(vc) = g.col.get(k) {
            for j in 0..3 {
                ck[j] = (ck[j] as f32 * vc[j].clamp(0.0, 1.0)) as u8;
            }
        }
        ch.v.push(vtx(pos[k], g.uv[k], ck, n, 1.0));
    }
    for f in g.idx.chunks(3) {
        ch.i.extend_from_slice(&[(start + f[0] as usize) as u16, (start + f[1] as usize) as u16, (start + f[2] as usize) as u16]);
    }
    let ci = mg.chunks.len() - 1;
    leaf_ranges.push((mat_idx, ci, start));
    geo.tris += g.tris();
    geo.verts += pos.len();
    for q in &pos {
        geo.grow(*q);
    }
    // the mesh's anchors (after the pivot) in tree space, for leaf generators below this one
    let mut frames = vec![];
    if mi > 0 {
        let to_world = |q: Vec3| {
            let x = q.dot(ax_r) * size * p.scale_x;
            let y = q.dot(ax_g) * size * p.scale_y;
            let z0 = q.dot(ax_n) * size;
            let yn = y / size.max(0.001);
            let z = z0 + x.abs() * fold.sin() + p.curl * yn * yn * size * 0.6;
            base + right * (x * fold.cos()) + out * y + nrm * z
        };
        for (q, d, h, id) in crate::meshio::anchor_points_h(&ctx.meshes[mi - 1]) {
            let wd = (right * d.dot(ax_r) * p.scale_x + out * d.dot(ax_g) * p.scale_y + nrm * d.dot(ax_n)).normalize_or_zero();
            if wd.length_squared() > 0.5 {
                frames.push(AnchorFrame { index: id, height: h, pos: to_world(q), dir: wd, nrm });
            }
        }
    }
    frames
}

/// Collection meshes: skins of the spines whose generators contribute to collections
/// (Spines: Branches / Fronds / Both; Pivot: Origin keeps world positions, First makes them
/// relative to the first collected spine's start).
fn collect(ctx: &GenCtx, spines: &[Spine]) -> HashMap<usize, MeshGeom> {
    let mut out = HashMap::new();
    for (mi, m) in ctx.meshes.iter().enumerate() {
        if !m.collection {
            continue;
        }
        let mut g = MeshGeom::default();
        let mut pivot: Option<Vec3> = None;
        for s in spines {
            let node = match ctx.nodes.iter().find(|n| n.id == s.node) {
                Some(n) => n,
                None => continue,
            };
            // this build has no frond generators, so "Fronds" collects nothing
            if !node.p.collect || m.spine_style == 1 {
                continue;
            }
            let pv = *pivot.get_or_insert(if m.pivot_style == 1 { s.pts[0] } else { Vec3::ZERO });
            let radial = 6usize;
            let base = g.pos.len() as u32;
            for k in 0..s.pts.len() {
                let b = s.tan[k].cross(s.nrm[k]);
                for j in 0..=radial {
            let a = j as f32 / radial as f32 * 2.0 * PI;
                    let d = s.nrm[k] * a.cos() + b * a.sin();
                    g.pos.push(s.pts[k] + d * s.rad[k] - pv);
                    g.nrm.push(d);
                    g.uv.push(vec2(j as f32 / radial as f32, s.cum[k] / s.len));
                }
            }
            for k in 0..s.pts.len() as u32 - 1 {
                for j in 0..radial as u32 {
                    let a = base + k * (radial as u32 + 1) + j;
                    let c = a + radial as u32 + 1;
                    g.idx.extend_from_slice(&[a, c, a + 1, a + 1, c, c + 1]);
                }
            }
            if g.pos.len() > crate::meshio::MAX_VERTS {
                break;
            }
        }
        out.insert(mi, g);
    }
    out
}

#[cfg(test)]
mod weld_tests {
    use super::*;

    /// TreeCraft's weld solver (picture 59): a branch welded onto an upright trunk leaves nothing
    /// inside the trunk, whatever angle it leaves at.
    #[test]
    fn welded_base_stays_out_of_the_parent() {
        for &angle_deg in &[90.0f32, 60.0, 45.0, 30.0] {
            let mut p = Params::new(Kind::Branch);
            p.extra.insert("Skin:Welding:Enabled".into(), 1.0);
            p.extra.insert("Skin:Welding:Spread:Amount".into(), 0.27);
            p.extra.insert("Skin:Welding:Spread:Upper scale".into(), 0.58);
            p.extra.insert("Skin:Welding:Spread:Lower scale".into(), 0.4);
            let pr = 1.0f32;
            let a = angle_deg.to_radians();
            let dir = vec3(a.sin(), a.cos(), 0.0);
            // the branch starts on the trunk's axis at height 5
            let pts: Vec<Vec3> = (0..=20).map(|i| vec3(0.0, 5.0, 0.0) + dir * (i as f32 * 0.5)).collect();
            let s = build_spine(1, pts, 0.6, &p);
            let mut geo = TreeGeo::default();
            skin(&s, &p, [255; 4], &mut geo, false, Some((pr, Vec3::Y, vec3(0.0, 5.0, 0.0))), 1, 0);
            let mut n = 0;
            let mut worst = f32::MAX;
            for mg in geo.mats.values() {
                for ch in &mg.chunks {
                    for v in &ch.v {
                        let q = v.position;
                        let rd = (q.x * q.x + q.z * q.z).sqrt();
                        worst = worst.min(rd);
                        n += 1;
                    }
                }
            }
            assert!(n > 0, "no vertices at {} deg", angle_deg);
            assert!(worst >= pr * 0.999, "a vertex {:.4} from the axis (inside the trunk) at {} deg", worst, angle_deg);
        }
    }

    /// Welding > Push (picture 62): pushed in, the edge goes into the trunk on its upper side and
    /// its lower side alike.
    #[test]
    fn push_moves_the_whole_edge() {
        let mut p = Params::new(Kind::Branch);
        p.extra.insert("Skin:Welding:Enabled".into(), 1.0);
        p.extra.insert("Skin:Welding:Push".into(), -0.2);
        let pr = 1.0f32;
        let a = 50.0f32.to_radians();
        let dir = vec3(a.sin(), a.cos(), 0.0);
        let pts: Vec<Vec3> = (0..=20).map(|i| vec3(0.0, 5.0, 0.0) + dir * (i as f32 * 0.5)).collect();
        let s = build_spine(1, pts, 0.6, &p);
        let mut geo = TreeGeo::default();
        skin(&s, &p, [255; 4], &mut geo, false, Some((pr, Vec3::Y, vec3(0.0, 5.0, 0.0))), 1, 0);
        // the two sides split at the height where the branch's centre line leaves the trunk
        let split = 5.0 + (pr / a.sin()) * a.cos();
        let (mut upper, mut lower) = (f32::MAX, f32::MAX);
        for mg in geo.mats.values() {
            for ch in &mg.chunks {
                for v in &ch.v {
                    let q = v.position;
                    let rd = (q.x * q.x + q.z * q.z).sqrt();
                    if q.y > split { upper = upper.min(rd) } else { lower = lower.min(rd) }
                }
            }
        }
        assert!(upper < pr * 0.9 && lower < pr * 0.9, "pushed edge: upper {:.3}, lower {:.3}", upper, lower);
    }
}

#[cfg(test)]
mod dup_tests {
    use super::*;

    fn node(id: u32, kind: Kind, parent: Option<u32>, name: &str) -> Node {
        Node { id, kind, name: name.into(), parent, pos: Vec2::ZERO, p: Params::new(kind), hidden: false }
    }

    /// A duplicated generator (picture 64) grows like the original on the same parent.
    #[test]
    fn duplicate_generator_grows() {
        let mut nodes = vec![node(1, Kind::Tree, None, "Tree"), node(2, Kind::Trunk, Some(1), "Trunk"), node(3, Kind::Branch, Some(2), "Branch")];
        let mut copy = nodes[2].clone();
        copy.id = 4;
        copy.name = "Branch copy".into();
        copy.p.seed += 97;
        nodes.push(copy);
        let ctx = GenCtx { nodes: &nodes, materials: &[], meshes: &[], selected: None, highlight: false, show_branches: true, forces: &[], leaf_collision: 0, only: None };
        let geo = generate(&ctx);
        let a = geo.per_node.get(&3).copied().unwrap_or(0);
        let b = geo.per_node.get(&4).copied().unwrap_or(0);
        assert!(a > 0, "the original made no branches");
        assert!(b > 0, "the copy made no branches (original {})", a);
    }

    /// The person's own pine (picture 93): hiding any one generator leaves every other
    /// generator's branches exactly where they were.
    #[test]
    fn hiding_in_the_pine_changes_nothing_else() {
        let text = include_str!("../testdata/pine_hd_fruit.tcm");
        let nodes = crate::treefile::parse_nodes(text);
        let spines_of = |nodes: &[Node]| -> Vec<(u32, usize, Vec<Vec3>)> {
            let ctx = GenCtx { nodes, materials: &[], meshes: &[], selected: None, highlight: false, show_branches: true, forces: &[], leaf_collision: 0, only: None };
            let mut v: Vec<(u32, usize, Vec<Vec3>)> = generate(&ctx).hits.into_iter().map(|h| (h.0, h.1, h.2)).collect();
            v.sort_by_key(|h| (h.0, h.1));
            v
        };
        // every piece fits one draw, so nothing is cut off (the real cause in picture 93)
        {
            let ctx = GenCtx { nodes: &nodes, materials: &[], meshes: &[], selected: None, highlight: false, show_branches: true, forces: &[], leaf_collision: 0, only: None };
            let geo = generate(&ctx);
            for mg in geo.mats.values() {
                for c in &mg.chunks {
                    assert!(c.v.len() <= CHUNK_VERTS && c.i.len() <= CHUNK_INDICES, "a piece holds {} vertices, {} corners", c.v.len(), c.i.len());
                    assert!(c.i.iter().all(|&k| (k as usize) < c.v.len()), "a corner points past its piece");
                }
            }
        }
        let all = spines_of(&nodes);
        for hide in nodes.iter().filter(|n| n.kind != Kind::Tree).map(|n| n.id).collect::<Vec<_>>() {
            let mut n2 = nodes.clone();
            n2.iter_mut().find(|n| n.id == hide).unwrap().hidden = true;
            let got = spines_of(&n2);
            let want: Vec<_> = all.iter().filter(|h| h.0 != hide).collect();
            assert_eq!(want.len(), got.len(), "hiding {} changed how many branches the others have", hide);
            for (a, b) in want.iter().zip(&got) {
                assert_eq!((a.0, a.1), (b.0, b.1));
                let d = a.2.iter().zip(&b.2).map(|(p, q)| p.distance(*q)).fold(0.0f32, f32::max);
                assert!(a.2.len() == b.2.len() && d < 1e-4, "hiding {} moved generator {} branch {} by {}", hide, a.0, a.1, d);
            }
        }
    }

    /// Hiding a generator (picture 92) leaves every other generator's leaves exactly as they were:
    /// same places and same shading, with leaf collision on.
    #[test]
    fn hiding_changes_nothing_else() {
        let mut nodes = vec![node(1, Kind::Tree, None, "Tree"), node(2, Kind::Trunk, Some(1), "Trunk"), node(3, Kind::Branch, Some(2), "A"), node(4, Kind::Branch, Some(2), "B"), node(5, Kind::Leaf, Some(4), "Leaves")];
        nodes[3].p.seed += 31;
        for n in &mut nodes {
            n.p.material = 0;
        }
        nodes[4].p.leaf_material = 7;
        let leaves = |nodes: &[Node]| -> Vec<[f32; 4]> {
            let ctx = GenCtx { nodes, materials: &[], meshes: &[], selected: None, highlight: false, show_branches: true, forces: &[], leaf_collision: 2, only: None };
            let geo = generate(&ctx);
            geo.mats.get(&7).map(|m| m.chunks.iter().flat_map(|c| c.v.iter().map(|v| [v.position.x, v.position.y, v.position.z, v.normal.w])).collect()).unwrap_or_default()
        };
        let before = leaves(&nodes);
        assert!(!before.is_empty(), "the leaf generator made no leaves");
        nodes[2].hidden = true;
        let after = leaves(&nodes);
        assert_eq!(before.len(), after.len(), "hiding A changed B's leaf count");
        for (a, b) in before.iter().zip(&after) {
            for k in 0..4 {
                assert!((a[k] - b[k]).abs() < 1e-5, "hiding A changed B's leaves: {:?} vs {:?}", a, b);
            }
        }
    }
}

#[cfg(test)]
mod leaf_on_trunk_tests {
    use super::*;

    /// Leaves straight on a trunk (picture 65) make geometry, not only a count.
    #[test]
    fn leaves_on_a_trunk_draw() {
        let mk = |id: u32, kind: Kind, parent: Option<u32>| Node { id, kind, name: format!("{:?}", kind), parent, pos: Vec2::ZERO, p: Params::new(kind), hidden: false };
        let nodes = vec![mk(1, Kind::Tree, None), mk(2, Kind::Trunk, Some(1)), mk(3, Kind::Leaf, Some(2))];
        let ctx = GenCtx { nodes: &nodes, materials: &[], meshes: &[], selected: None, highlight: false, show_branches: true, forces: &[], leaf_collision: 0, only: None };
        let geo = generate(&ctx);
        let leaves = geo.per_node.get(&3).copied().unwrap_or(0);
        let verts: usize = geo.mats.values().flat_map(|m| m.chunks.iter()).map(|c| c.v.len()).sum();
        let trunk_only = {
            let n2 = vec![mk(1, Kind::Tree, None), mk(2, Kind::Trunk, Some(1))];
            let c2 = GenCtx { nodes: &n2, materials: &[], meshes: &[], selected: None, highlight: false, show_branches: true, forces: &[], leaf_collision: 0, only: None };
            generate(&c2).mats.values().flat_map(|m| m.chunks.iter()).map(|c| c.v.len()).sum::<usize>()
        };
        assert!(leaves > 0, "no leaves counted");
        assert!(verts > trunk_only, "leaves counted ({}) but no leaf geometry: {} vs trunk {}", leaves, verts, trunk_only);
        // and they sit on the trunk, not far off
        let leaf_pts: Vec<Vec3> = geo.nodes.iter().filter(|n| n.0 == 3).map(|n| n.2).collect();
        let far = leaf_pts.iter().map(|p| (p.x * p.x + p.z * p.z).sqrt()).fold(0.0f32, f32::max);
        assert!(far < 20.0, "leaves {:.1} units off the trunk", far);
        eprintln!("leaves {}, verts {} (trunk {}), first leaf at {:?}", leaves, verts, trunk_only, leaf_pts.first());
    }
}

#[cfg(test)]
mod tip_tests {
    use super::*;

    /// Open edges near the tip of a welded and an unwelded branch (picture 71).
    fn tip_open_edges(weld: bool, cap: bool) -> (usize, usize) {
        let mut p = Params::new(Kind::Branch);
        p.cap_ends = cap;
        // the tip is closed only when asked (picture 85); with cap false the test asks with Cap tip
        p.cap_tip = !cap;
        if weld {
            p.extra.insert("Skin:Welding:Enabled".into(), 1.0);
        }
        let dir = vec3(0.8, 0.6, 0.0).normalize();
        let pts: Vec<Vec3> = (0..=20).map(|i| vec3(0.0, 5.0, 0.0) + dir * (i as f32 * 0.5)).collect();
        let s = build_spine(1, pts, 0.6, &p);
        let mut geo = TreeGeo::default();
        skin(&s, &p, [255; 4], &mut geo, false, Some((1.0, Vec3::Y, vec3(0.0, 5.0, 0.0))), 1, 0);
        let tip = s.pts[s.pts.len() - 1];
        let mut open_tip = 0;
        let mut tris = 0;
        for mg in geo.mats.values() {
            for ch in &mg.chunks {
                // edges keyed by position (the texture seam doubles its points, which isn't a hole)
                let key = |v: Vec3| ((v.x * 1000.0).round() as i64, (v.y * 1000.0).round() as i64, (v.z * 1000.0).round() as i64);
                let mut count: HashMap<((i64, i64, i64), (i64, i64, i64)), u32> = HashMap::new();
                let mut at: HashMap<((i64, i64, i64), (i64, i64, i64)), (Vec3, Vec3)> = HashMap::new();
                for t in ch.i.chunks(3) {
                    tris += 1;
                    for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                        let (pa, pb) = (ch.v[a as usize].position, ch.v[b as usize].position);
                        let (ka, kb) = (key(pa), key(pb));
                        if ka == kb {
                            continue;
                        }
                        let k = if ka < kb { (ka, kb) } else { (kb, ka) };
                        *count.entry(k).or_default() += 1;
                        at.insert(k, (pa, pb));
                    }
                }
                for (k, c) in count {
                    if c == 1 {
                        let (pa, pb) = at[&k];
                        if pa.distance(tip) < 1.5 && pb.distance(tip) < 1.5 {
                            open_tip += 1;
                        }
                    }
                }
            }
        }
        (open_tip, tris)
    }

    #[test]
    fn tip_edges() {
        for (w, c) in [(false, false), (true, false), (false, true), (true, true)] {
            let (o, t) = tip_open_edges(w, c);
            eprintln!("weld {} cap {}: open tip edges {}, tris {}", w, c, o, t);
            assert_eq!(o, 0, "the tip of a branch (weld {}, cap ends {}) is open", w, c);
        }
    }

    /// Every generation mode places children (picture 86).
    #[test]
    fn every_mode_places_children() {
        let pts: Vec<Vec3> = (0..=20).map(|i| vec3(0.0, i as f32 * 0.5, 0.0)).collect();
        let base = Params::new(Kind::Branch);
        let s = build_spine(1, pts.clone(), 0.5, &base);
        let short = build_spine(1, pts[..11].to_vec(), 0.5, &base);
        let spines = vec![s, short];
        let tree = Params::new(Kind::Tree);
        let place = |p: &Params, i: usize| distribute(p, ParentRef::Spine(i), &spines, &tree, &mut Rng::new(5), 10.0);
        let mode = |m: f64| {
            let mut p = Params::new(Kind::Branch);
            p.first = 0.1;
            p.last = 0.9;
            p.extra.insert("Generation:Mode".into(), m);
            p
        };
        // Phyllotaxy: nodes reach both First and Last (no gap)
        let mut p = mode(5.0);
        p.interval = 2.0;
        p.jumble = 0.0;
        let ts: Vec<f32> = place(&p, 0).iter().map(|o| o.0).collect();
        let (lo, hi) = (ts.iter().cloned().fold(1.0, f32::min), ts.iter().cloned().fold(0.0, f32::max));
        assert!((lo - 0.1).abs() < 1e-3 && (hi - 0.9).abs() < 1e-3, "phyllotaxy spans First..Last: {} {}", lo, hi);
        // Classic Steps: 4 steps of Frequency each with Per step frequency
        let mut p = mode(0.0);
        p.frequency = 3.0;
        p.extra.insert("Generation:Steps:Enabled".into(), 1.0);
        p.extra.insert("Generation:Steps:Number".into(), 4.0);
        p.extra.insert("Generation:Steps:Per step frequency".into(), 1.0);
        let o = place(&p, 0);
        assert_eq!(o.len(), 12);
        let mut spots: Vec<i32> = o.iter().map(|q| (q.0 * 1000.0).round() as i32).collect();
        spots.dedup();
        spots.sort();
        spots.dedup();
        assert_eq!(spots.len(), 4, "children sit on 4 steps");
        // Proportional: Number on the longest parent, half on one half as long
        let mut p = mode(1.0);
        p.extra.insert("Generation:Proportional:Number".into(), 8.0);
        assert_eq!(place(&p, 0).len(), 8);
        assert_eq!(place(&p, 1).len(), 4);
        // Proportional Steps and Absolute Steps
        let mut p = mode(2.0);
        p.extra.insert("Generation:Proportional Steps:Steps".into(), 4.0);
        p.extra.insert("Generation:Proportional Steps:Per Step:Number".into(), 2.0);
        assert_eq!(place(&p, 0).len(), 8);
        assert_eq!(place(&p, 1).len(), 4);
        let mut p = mode(4.0);
        p.extra.insert("Generation:Absolute Steps:Steps".into(), 3.0);
        p.extra.insert("Generation:Absolute Steps:Per Step:Number".into(), 2.0);
        assert_eq!(place(&p, 1).len(), 6);
        // Bifurcation, Flood, Parent
        assert!(!place(&mode(7.0), 0).is_empty());
        assert!(place(&mode(8.0), 0).len() > 5);
        assert_eq!(place(&mode(9.0), 0).len(), 1);
        // Extend parent: Any adds the parent's forks, None doesn't, Bottom goes back to the parent
        let mut spines2 = spines;
        let mut fork = build_spine(1, (0..=6).map(|i| vec3(i as f32 * 0.4, 7.0, 0.0)).collect(), 0.2, &base);
        fork.ext_of = Some((0, 1));
        spines2.push(fork);
        let mut p = mode(0.0);
        assert_eq!(extend_parents(&p, &[ParentRef::Spine(0)], &spines2).len(), 1);
        p.extra.insert("Generation:Shared:Extend parent".into(), 1.0);
        assert_eq!(extend_parents(&p, &[ParentRef::Spine(0)], &spines2).len(), 2);
        p.extra.insert("Generation:Shared:Extend parent".into(), 3.0);
        assert_eq!(extend_parents(&p, &[ParentRef::Spine(0)], &spines2).len(), 1, "no Split 2 here");
        p.extra.insert("Generation:Shared:Extend parent".into(), 7.0);
        assert!(matches!(extend_parents(&p, &[ParentRef::Spine(2)], &spines2)[..], [ParentRef::Spine(0)]));
    }

    /// A Parent curve on Phyllotaxy's Internode length changes the nodes (picture 91).
    #[test]
    fn internode_curve_takes_effect() {
        let pts: Vec<Vec3> = (0..=20).map(|i| vec3(0.0, i as f32 * 0.5, 0.0)).collect();
        let base = Params::new(Kind::Branch);
        let spines = vec![build_spine(1, pts, 0.5, &base)];
        let tree = Params::new(Kind::Tree);
        let mut p = Params::new(Kind::Branch);
        p.extra.insert("Generation:Mode".into(), 5.0);
        p.extra.insert("Generation:Phyllotaxy:Arrangement:Style".into(), 4.0);
        p.first = 0.0;
        p.last = 1.0;
        p.interval = 1.0;
        p.jumble = 0.0;
        let flat = distribute(&p, ParentRef::Spine(0), &spines, &tree, &mut Rng::new(1), 10.0).len();
        let mut c = Curve::flat();
        for q in &mut c.pts {
            q.y = 0.5;
        }
        p.curves.insert("Generation:Phyllotaxy:Nodes:Internode length|0".into(), c);
        let half = distribute(&p, ParentRef::Spine(0), &spines, &tree, &mut Rng::new(1), 10.0).len();
        assert!(half > flat + 5, "halving the internode length doubles the nodes: {} -> {}", flat, half);
    }

    /// Without Cap tip or Cap ends the tube's tip stays open (picture 85).
    #[test]
    fn tip_open_by_default() {
        let mut p = Params::new(Kind::Branch);
        assert!(!p.cap_tip && !p.cap_ends);
        p.cap_tip = false;
        let pts: Vec<Vec3> = (0..=10).map(|i| vec3(0.0, i as f32 * 0.5, 0.0)).collect();
        let s = build_spine(1, pts, 0.6, &p);
        let mut open = TreeGeo::default();
        skin(&s, &p, [255; 4], &mut open, false, None, 1, 0);
        p.cap_tip = true;
        let mut capped = TreeGeo::default();
        skin(&s, &p, [255; 4], &mut capped, false, None, 1, 0);
        assert!(capped.tris > open.tris, "Cap tip adds the dome");
    }
}

#[cfg(test)]
mod fillet_tests {
    use super::*;

    /// Picture 73: along every column of a welded base, the distance from the branch's own axis
    /// shrinks steadily from the parent to the branch's radius: no bulge (it would grow again) and
    /// no fold (it would dip under the branch's radius).
    #[test]
    fn weld_profile_is_a_smooth_fillet() {
        for &angle_deg in &[90.0f32, 60.0, 45.0] {
            let mut p = Params::new(Kind::Branch);
            // a branch of even thickness, so "the branch's radius" is one number
            p.neutralize();
            p.extra.insert("Skin:Welding:Enabled".into(), 1.0);
            p.extra.insert("Skin:Welding:Spread:Amount".into(), 0.13);
            p.extra.insert("Skin:Welding:Spread:Upper scale".into(), 0.46);
            p.extra.insert("Skin:Welding:Spread:Lower scale".into(), 1.45);
            p.extra.insert("Skin:Welding:Offset".into(), 1.2);
            let a = angle_deg.to_radians();
            let dir = vec3(a.sin(), a.cos(), 0.0);
            let r0 = 0.6;
            let pts: Vec<Vec3> = (0..=20).map(|i| vec3(0.0, 5.0, 0.0) + dir * (i as f32 * 0.5)).collect();
            let s = build_spine(1, pts, r0, &p);
            let mut geo = TreeGeo::default();
            skin(&s, &p, [255; 4], &mut geo, false, Some((1.0, Vec3::Y, vec3(0.0, 5.0, 0.0))), 1, 0);
            let ch = &geo.mats.values().next().unwrap().chunks[0];
            // rings: consecutive runs of radial + 1 vertices; columns by index within a ring
            let mut rings: Vec<Vec<Vec3>> = vec![];
            let mut cur: Vec<Vec3> = vec![];
            let mut last_len = 0usize;
            for v in &ch.v {
                cur.push(v.position);
                let _ = last_len;
                // a ring closes when its last point meets its first again (the seam doubles it)
                if cur.len() > 3 && cur.last().unwrap().distance(cur[0]) < 1e-4 {
                    last_len = cur.len();
                    rings.push(std::mem::take(&mut cur));
                }
            }
            let n = rings[0].len();
            let axis_dist = |q: Vec3| -> f32 {
                let v = q - vec3(0.0, 5.0, 0.0);
                (v - dir * v.dot(dir)).length()
            };
            for j in 0..n - 1 {
                let col: Vec<f32> = rings.iter().filter(|r| r.len() == n).map(|r| axis_dist(r[j])).collect();

                for w in col.windows(2) {
                    assert!(w[1] <= w[0] + r0 * 0.02, "{} deg, column {}: the base bulges ({:.3} after {:.3})", angle_deg, j, w[1], w[0]);
                }
                let least = col.iter().cloned().fold(f32::MAX, f32::min);
                assert!(least >= r0 * 0.97, "{} deg, column {}: the base folds in to {:.3} (radius {})", angle_deg, j, least, r0);
            }
        }
    }
}

#[cfg(test)]
mod gradient_tests {
    use super::*;

    /// Picture 75: a material's gradient shows on the branch, base colour at its start and tip
    /// colour at its end.
    #[test]
    fn gradient_runs_base_to_tip() {
        let mk = |id: u32, kind: Kind, parent: Option<u32>| Node { id, kind, name: format!("{:?}", kind), parent, pos: Vec2::ZERO, p: Params::new(kind), hidden: false };
        let mut nodes = vec![mk(1, Kind::Tree, None), mk(2, Kind::Trunk, Some(1))];
        nodes[1].p.material = 0;
        let mut m = Material::new("Grad", 0);
        m.grad_on = true;
        m.grad_base = [1.0, 0.0, 0.0];
        m.grad_color = [0.0, 0.0, 1.0];
        let mats = vec![m];
        let ctx = GenCtx { nodes: &nodes, materials: &mats, meshes: &[], selected: None, highlight: false, show_branches: true, forces: &[], leaf_collision: 0, only: None };
        let geo = generate(&ctx);
        let vs: Vec<&Vertex> = geo.mats.get(&0).expect("trunk drawn with the material").chunks.iter().flat_map(|c| c.v.iter()).collect();
        let low = vs.iter().min_by(|a, b| a.position.y.total_cmp(&b.position.y)).unwrap();
        let high = vs.iter().max_by(|a, b| a.position.y.total_cmp(&b.position.y)).unwrap();
        assert!(low.color[0] > 200 && low.color[2] < 60, "base colour {:?}", low.color);
        assert!(high.color[2] > 200 && high.color[0] < 60, "tip colour {:?}", high.color);
    }
}
