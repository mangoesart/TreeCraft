//! Tree generation: walks the generator graph and builds branch skins and leaf meshes.
use crate::model::*;
use crate::model::Material;
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::f32::consts::PI;

pub const CHUNK_VERTS: usize = 30000;

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
        let full = match self.chunks.last() {
            Some(c) => c.v.len() + need > CHUNK_VERTS,
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
    /// Every node made (for Node edit mode picking): generator, its number in the generator,
    /// where it starts and ends.
    pub nodes: Vec<(u32, usize, Vec3, Vec3)>,
    /// What a click can hit (picture 26): each node's spine points and radii (a leaf: its centre
    /// and a radius), with its generator and number.
    pub hits: Vec<(u32, usize, Vec<Vec3>, Vec<f32>)>,
    /// Spines of Skin:Type "Spine only" generators, drawn as yellow lines (picture 39).
    pub spine_lines: Vec<(u32, Vec<Vec3>)>,
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
    let mut geo = TreeGeo { bmin: Vec3::splat(f32::MAX), bmax: Vec3::splat(f32::MIN), ..Default::default() };
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
    let c = (geo.bmin + geo.bmax) * 0.5;
    let half = ((geo.bmax - geo.bmin) * 0.5).max(Vec3::splat(0.5));
    for (m, ci, start) in leaf_ranges {
        if let Some(mg) = geo.mats.get_mut(&m) {
            let ch = &mut mg.chunks[ci];
            let end = ch.v.len();
            for v in &mut ch.v[start..end] {
                let d = (v.position - c) / half;
                let r = (d.x * d.x + d.z * d.z).sqrt().min(1.0);
                let h = ((v.position.y - geo.bmin.y) / (half.y * 2.0)).clamp(0.0, 1.0);
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
    let children: Vec<&Node> = ctx.nodes.iter().filter(|n| n.parent == Some(node_id)).collect();
    let parent_gen = ctx.nodes.iter().find(|n| n.id == node_id);
    for child in children {
        if child.hidden {
            continue;
        }
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
            let placements = shared_placement(p, *parent, spines, distribute(p, *parent, spines, tree, &mut rng));
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
                            }
                            s.pos_on_parent = t;
                            geo.bones.push(s.pts.clone());
                            geo.nodes.push((child.id, me, s.pts[0], s.pts[s.pts.len() - 1]));
                            geo.hits.push((child.id, me, s.pts.clone(), s.rad.clone()));
                            // Skin:Type "Spine only" (3) keeps the spine for its children but draws no
                            // bark; Skin:Visibility draws only that share of the branches (their
                            // children still grow)
                            let visible = hash01(child.id as u64, me as u64, 4049) * 0.5 + 0.5 < pex(p, "Skin:Visibility", 1.0);
                            // Spine only (picture 39): drawn as a yellow line with a mark at each segment
                            if skin_type == 3 && ctx.show_branches && ctx.only.as_ref().map_or(true, |o| o.contains(&child.id)) {
                                geo.spine_lines.push((child.id, s.pts.clone()));
                            }
                            if ctx.show_branches && ctx.only.as_ref().map_or(true, |o| o.contains(&child.id)) && skin_type != 3 && visible {
                                let weld = if let ParentRef::Spine(i) = *parent { Some((spines[i].radius(t.min(1.0)), spines[i].frame(t.min(1.0)).0, spines[i].point(t.min(1.0)))) } else { None };
                                skin(&s, p, col, geo, child.kind == Kind::Trunk, weld, child.id, me);
                            }
                            spines.push(s);
                            new_spines.push(ParentRef::Spine(spines.len() - 1));
                            count += 1;
                        }
                    }
                    Kind::Leaf if ctx.only.as_ref().map_or(false, |o| !o.contains(&child.id)) => {}
                    Kind::Leaf => {
                        let frames = make_leaf(ctx, p, *parent, t, roll, base_pos, spines, anchors, &mut rng, col, geo, cache, leaf_ranges);
                        let at = match *parent {
                            ParentRef::Spine(i) => spines[i].point(t),
                            ParentRef::Anchor(ai) => anchors[ai].pos,
                            ParentRef::Ground => base_pos,
                        };
                        geo.nodes.push((child.id, me, at, frames.first().map_or(at + Vec3::Y * p.leaf_size.max(0.1) * 0.5, |f| f.pos)));
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
                geo.bones.push(s.pts.clone());
                geo.nodes.push((child.id, usize::MAX - k, s.pts[0], s.pts[s.pts.len() - 1]));
                geo.hits.push((child.id, usize::MAX - k, s.pts.clone(), s.rad.clone()));
                if ctx.show_branches && ctx.only.as_ref().map_or(true, |o| o.contains(&child.id)) {
                    skin(&s, &child.p, col, geo, child.kind == Kind::Trunk, None, child.id, usize::MAX - k);
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
fn distribute(p: &Params, parent: ParentRef, spines: &[Spine], tree: &Params, rng: &mut Rng) -> Vec<(f32, f32, Vec3, Vec3)> {
    const TURN: f32 = 2.0 * PI;
    let mut out = vec![];
    let plen = match parent {
        ParentRef::Spine(i) => spines[i].len,
        ParentRef::Ground => 0.0,
        // one child on each anchor, turned by Roll around the anchor's direction
        ParentRef::Anchor(_) => return vec![(0.0, p.roll * TURN, Vec3::ZERO, Vec3::X)],
    };
    let (first, last) = (p.first.min(p.last), p.last.max(p.first));
    let span = last - first;
    if let ParentRef::Ground = parent {
        // children of the Tree node: trunks spread over the Tree's Shape radius
        let n = match p.gen_mode {
            MODE_CLASSIC => p.frequency.max(0.0) as usize,
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
    match p.gen_mode {
        MODE_ABSOLUTE => {
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
                // First / Last may lie below 0 or above 1 (picture 38): spots off the parent are skipped
                if (-1e-4..=1.0001).contains(&t) {
                    out.push((t.clamp(0.0, 1.0), r * TURN, Vec3::ZERO, Vec3::X));
                }
            }
        }
        MODE_INTERVAL | MODE_PHYLLOTAXY => {
            // Interval: spacing 10/Frequency units. Phyllotaxy: Internode length in units.
            let spacing_units = if p.gen_mode == MODE_INTERVAL { 10.0 / p.interval.max(0.01) } else { p.interval.max(0.05) };
            let step = spacing_units / plen.max(0.001);
            let per = p.per_node.max(1) as usize;
            let mut t = last;
            let mut node = 0i32;
            let mut spiral_turns = 0.0f32;
            while t >= first && node < p.limit.max(1) && out.len() < 4000 {
                let within = if per >= 3 { 1.0 / per as f32 } else { 0.5 };
                let stagger = if p.balanced && node % 2 == 1 { 0.5 / per as f32 } else { 0.0 };
                for j in 0..per {
                    let tj = (t + rng.sym() * p.jumble * step * 0.5).clamp(first, last);
                    let turns = (j as f32 * within + stagger + spiral_turns) * p.sweep + p.roll;
                    if (-1e-4..=1.0001).contains(&tj) {
                        out.push((tj.clamp(0.0, 1.0), turns * TURN, Vec3::ZERO, Vec3::X));
                    }
                }
                spiral_turns += p.spiral * 0.1;
                t -= step;
                node += 1;
            }
        }
        _ => {
            let f = if p.relative { plen * p.frequency * 0.025 } else { p.frequency };
            let n = (f.max(0.0) as usize).min(2000);
            for i in 0..n {
                let base = if p.balanced { i as f32 / n.max(1) as f32 } else { rng.next() };
                let turns = (base + rng.sym() * p.offset * 0.5) * p.sweep + p.roll;
                let t = along(p, "Generation:Frequency", first, span, rng);
                if (-1e-4..=1.0001).contains(&t) {
                    out.push((t.clamp(0.0, 1.0), turns * TURN, Vec3::ZERO, Vec3::X));
                }
            }
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
    let ext = pex(p, "Generation:Shared:Extend parent", 0.0).max(0.0);
    let clip = pex(p, "Generation:Shared:Boundaries:Clip", 0.025).clamp(0.0, 1.0);
    let favor = match p.gen_mode {
        MODE_ABSOLUTE => pex(p, "Generation:Absolute:Favor splits", 0.0),
        MODE_CLASSIC => pex(p, "Generation:Proportional:Favor splits", 0.0),
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

/// A random spot between First and Last; a Profile curve on the count property makes children
/// likelier where it's high (picture 24).
fn along(p: &Params, count_prop: &str, first: f32, span: f32, rng: &mut Rng) -> f32 {
    let Some(c) = p.curves.get(&format!("{}|2", count_prop)) else { return first + span * rng.next() };
    let top = c.pts.iter().map(|q| q.y).fold(0.0f32, f32::max).max(1e-3);
    for _ in 0..32 {
        let x = rng.next();
        if rng.next() * top <= c.eval(x) {
            return first + span * x;
        }
    }
    first + span * rng.next()
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
    let angle = (p.start_angle + p.angle_var * rng.sym() + p.angle_profile * (t - 0.5)).clamp(0.0, 1.0) * PI;
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
            let mode_pre = match p.gen_mode {
                MODE_ABSOLUTE => "Generation:Absolute",
                MODE_INTERVAL => "Generation:Interval",
                MODE_PHYLLOTAXY => "Generation:Phyllotaxy:Arrangement",
                _ => "Generation:Proportional",
            };
            let align_on = pex(p, &format!("{}:Align", mode_pre), 1.0) > 0.5;
            let style = pex(p, &format!("{}:Align style", mode_pre), 1.0) as i32;
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
    Spine { node, pts, rad, tan, nrm, cum, len, mat: p.material, parent_spine: None, pos_on_parent: 0.0 }
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
    // Skin:Splits: by Chance (Seed), the branch forks at two thirds of its length: one fork at
    // Spread 1 (x 90 degrees), another at Spread 2; Balance shares the radius between the main
    // branch and its forks; Smooth Outer bends a fork out gradually, Smooth Inner lets it start
    // inside the branch
    let chance = pex(p, "Skin:Splits:Chance", 0.0);
    if chance <= 0.0 || s.pts.len() < 3 {
        return;
    }
    let seed = pex(p, "Skin:Splits:Seed", 15000.0) as u64;
    if hash01(seed, gen as u64, node as u64) * 0.5 + 0.5 >= chance {
        return;
    }
    let at = 0.66;
    let (tn, n, b) = s.frame(at);
    let start = s.point(at);
    let rest = s.len * (1.0 - at);
    let rs = s.radius(at);
    let bal = pex(p, "Skin:Splits:Balance", 0.5).clamp(0.05, 0.95);
    let outer = pex(p, "Skin:Splits:Smooth:Outer", 0.5).clamp(0.0, 1.0);
    let inner = pex(p, "Skin:Splits:Smooth:Inner", 0.25).clamp(0.0, 1.0);
    let flat = SkinLook { weld: None, pull: 0.0, ..SkinLook::new(p, None) };
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
        skin_tube(&fork, p, col, geo, trunk, &flat);
    }
}

/// Welding's extra rings (picture 58): the weld zone gets `8 + 6 x Offset` rings (6..32), closer
/// together toward the parent, so the base can bend smoothly onto it; the rest of the branch
/// keeps its own points.
fn densify_base(s: &Spine, zone: f32, offset: f32) -> Spine {
    let n = ((8.0 + 6.0 * offset).round() as usize).clamp(6, 32);
    let mut ds: Vec<f32> = (0..=n).map(|i| zone * (i as f32 / n as f32).powf(1.35)).collect();
    ds.extend(s.cum.iter().copied().filter(|c| *c > zone * 1.0001));
    let len = s.len.max(1e-6);
    let mut out = Spine { node: s.node, pts: vec![], rad: vec![], tan: vec![], nrm: vec![], cum: vec![], len: s.len, mat: s.mat, parent_spine: s.parent_spine, pos_on_parent: s.pos_on_parent };
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

fn skin_tube(s0: &Spine, p: &Params, col: [u8; 4], geo: &mut TreeGeo, trunk: bool, look: &SkinLook) {
    // the weld zone: from the parent's middle out past its surface by Offset x the parent radius
    let zone = look.weld.map_or(0.0, |(pr, _, _)| pr * (1.0 + look.weld_offset));
    let dense;
    let s: &Spine = if zone > 0.0 && s0.len > zone * 0.2 && s0.pts.len() >= 2 {
        dense = densify_base(s0, zone.min(s0.len), look.weld_offset);
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
            if zone > 0.0 && s.cum[k] < zone { (n * 2).min(64) } else { n }
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
    for k in 0..m {
        let radial = ring_n[k];
        ring_start.push(ch.v.len());
        let tn = s.tan[k];
        let n = s.nrm[k];
        let b = tn.cross(n);
        let sfrac = s.cum[k] / s.len;
        let dist = s.cum[k];
        let mut ao = if trunk { 0.8 + 0.2 * sfrac } else { 0.62 + 0.38 * sfrac };
        let w = if zone > 0.0 { (1.0 - dist / zone).clamp(0.0, 1.0) } else { 0.0 };
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
            let mut weld_to: Option<(Vec3, Vec3, f32)> = None;
            // Welding > Blend's texture fade (picture 58): how far this spot of the base dissolves
            // into the parent's bark (0 solid .. 1 gone)
            let mut fade = 0.0f32;
            if w > 0.0 {
                if let Some((pr, ptan, pp)) = look.weld {
                    // Welding > Spread (picture 57): the base widens over the parent, more toward
                    // the parent's tip (Upper scale) or its base (Lower scale) ...
                    let side = dir.normalize_or(dir).dot(ptan);
                    let scale = if side >= 0.0 { look.upper } else { look.lower } * side.abs() + 0.25 * (1.0 - side.abs());
                    r *= 1.0 + spread * 1.6 * w * w * (scale / 0.25);
                    // ... and wraps onto the parent's surface there (the parent taken as a cylinder
                    // along its direction at the joint): near the parent the ring lies on it, so
                    // the joint never stands out of the parent; further out it eases back into the
                    // branch's own tube
                    let tube = s.pts[k] + dir * r;
                    let v = tube - pp;
                    let along = v.dot(ptan);
                    let perp = v - ptan * along;
                    let out = perp.normalize_or(dir);
                    let surface = pp + ptan * along + out * (pr * 1.012);
                    // how much it hugs the parent: fully at the parent's surface, none at the zone's end
                    let hug = ((zone - dist) / (zone - pr * 0.999).max(1e-4)).clamp(0.0, 1.0);
                    let hug = hug * hug * (3.0 - 2.0 * hug);
                    weld_to = Some((surface, out, hug));
                    // Welding > Blend: the joint's shading turns to the parent's (its surface
                    // direction) by Texture, sharper with Contrast, ragged with Noise, so the joint
                    // is hard to see; a faint darkening keeps the crease
                    let jag = 1.0 + look.blend_noise * 0.05 * (a * 7.0 + dist * 13.0).sin();
                    let wb = (hug * jag).clamp(0.0, 1.0).powf(1.0 + look.blend_contrast * 0.15);
                    nrm = nrm.lerp(out, (look.blend_tex * wb).clamp(0.0, 1.0)).normalize_or(nrm);
                    // the parent's own shading where it hugs it (a trunk's bark is lighter than a
                    // branch's base), with a faint crease
                    ao = ao + (0.86 - ao) * hug;
                    ao *= 1.0 - 0.06 * look.blend_tex * wb;
                    // the fade: strongest where the base lies on the parent, ending sooner with
                    // more Contrast; the shader turns it into a ragged, bark-like edge
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
            let mut pos = s.pts[k] + dir * r;
            if let Some((surface, _, hug)) = weld_to {
                pos = pos.lerp(surface, hug);
            }
            bmin = bmin.min(pos);
            bmax = bmax.max(pos);
            // the fade rides in the colour's alpha, below the selection mark's range: unselected
            // 255 down to 235, selected 128 down to 108 (render.rs reads it back)
            let mut vc = col;
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
    // ends: open by default (picture 34); Skin > Cap ends closes the tip and the base
    if p.cap_ends {
        let tip = s.pts[m - 1] + s.tan[m - 1] * s.rad[m - 1] * 0.5;
        let ci = ch.v.len() as u16;
        ch.v.push(vtx(tip, uvm.uv(0.5, s.cum[m - 1]), col, s.tan[m - 1], 1.0));
        let last = ring_start[m - 1] as u16;
        for j in 0..ring_n[m - 1] as u16 {
            ch.i.extend_from_slice(&[last + j, last + j + 1, ci]);
        }
        let bi = ch.v.len() as u16;
        ch.v.push(vtx(s.pts[0], uvm.uv(0.5, 0.0), col, -s.tan[0], 1.0));
        let first = ring_start[0] as u16;
        for j in 0..ring_n[0] as u16 {
            ch.i.extend_from_slice(&[first + j + 1, first + j, bi]);
        }
        geo.tris += ring_n[0] + ring_n[m - 1];
    }
    geo.tris += tris;
    geo.verts += need;
    geo.grow(bmin);
    geo.grow(bmax);
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
    let mi = if p.leaf_mesh <= ctx.meshes.len() { p.leaf_mesh } else { 0 };
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
