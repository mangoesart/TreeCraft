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
    for child in children {
        if child.hidden {
            continue;
        }
        let sel = ctx.highlight && ctx.selected == Some(child.id);
        let col: [u8; 4] = if sel { [255, 235, 210, 128] } else { [255, 255, 255, 255] };
        let p = &child.p;
        let mut new_spines: Vec<ParentRef> = vec![];
        let mut count = 0usize;
        let on_anchors = parents.iter().all(|q| matches!(q, ParentRef::Anchor(_)));
        for (pi, parent) in parents.iter().enumerate() {
            // Generation:Anchors: only the chosen anchor (fns 0x14024b880, 0x1402456e0)
            if let ParentRef::Anchor(ai) = parent {
                if p.anchor_pick > 0 && anchors[*ai].index != p.anchor_pick {
                    continue;
                }
                // fn 0x14026f500: only anchors whose height lies between First and Last
                let h = anchors[*ai].height;
                if h < p.first.min(p.last) || h > p.first.max(p.last) {
                    continue;
                }
            }
            let mut rng = Rng::new((p.seed as u64) * 7919 + pi as u64 * 104729 + child.id as u64 * 31);
            let placements = distribute(p, *parent, spines, tree, &mut rng);
            for (t, roll, base_pos, azim_dir) in placements {
                match child.kind {
                    Kind::Trunk | Kind::Branch => {
                        if let Some(s) = make_spine(child, p, *parent, t, roll, base_pos, azim_dir, spines, &mut rng) {
                            skin(&s, p, col, geo, child.kind == Kind::Trunk);
                            spines.push(s);
                            new_spines.push(ParentRef::Spine(spines.len() - 1));
                            count += 1;
                        }
                    }
                    Kind::Leaf => {
                        let frames = make_leaf(ctx, p, *parent, t, roll, base_pos, spines, anchors, &mut rng, col, geo, cache, leaf_ranges);
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
                let t = first + span * rng.next();
                out.push((t.clamp(0.0, 1.0), r * TURN, Vec3::ZERO, Vec3::X));
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
                    out.push((tj.clamp(0.0, 1.0), turns * TURN, Vec3::ZERO, Vec3::X));
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
                let t = first + span * rng.next();
                out.push((t.clamp(0.0, 1.0), turns * TURN, Vec3::ZERO, Vec3::X));
            }
        }
    }
    out
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
    rng: &mut Rng,
) -> Option<Spine> {
    let angle = (p.start_angle + p.angle_var * rng.sym() + p.angle_profile * (t - 0.5)).clamp(0.0, 1.0) * PI;
    let (start, dir, plen, prad) = match parent {
        ParentRef::Ground => {
            // start angle measured up from the ground: 0.5 = straight up
            let d = (Vec3::Y * angle.sin() + ground_dir * angle.cos()).normalize();
            (ground_pos, d, 0.0, 0.0)
        }
        ParentRef::Spine(i) => {
            let ps = &spines[i];
            let (tn, n, b) = ps.frame(t);
            let perp = n * roll.cos() + b * roll.sin();
            let d = (tn * angle.cos() + perp * angle.sin()).normalize();
            (ps.point(t), d, ps.len, ps.radius(t))
        }
        ParentRef::Anchor(_) => return None, // only leaves spawn on anchors
    };
    let prof = 1.0 - p.len_profile * ((t - p.first) / (p.last - p.first).max(0.01)).clamp(0.0, 1.0);
    let len = ((p.len_abs + p.len_pct * plen) * prof * (1.0 + p.len_var * rng.sym()) * p.size_scalar).max(0.0);
    if len < 0.05 {
        return None;
    }
    let mut r0 = (p.radius_abs + p.radius_pct * prad) * (1.0 + p.radius_var * rng.sym()) * p.size_scalar;
    if p.clamp_parent && prad > 0.0 {
        r0 = r0.min(prad * 0.95);
    }
    r0 = r0.max(0.005);
    let n = p.seg_len.clamp(2, 64) as usize;
    let step = len / n as f32;
    let f = 0.6 + 4.0 * p.noise_turb;
    let ph = [rng.next() * 6.28, rng.next() * 6.28, rng.next() * 6.28];
    let mut pts = Vec::with_capacity(n + 1);
    let mut d = dir;
    let mut pos = start;
    pts.push(pos);
    for k in 0..n {
        let kk = k as f32;
        let noise = vec3((kk * f + ph[0]).sin(), (kk * f * 1.13 + ph[1]).sin() * 0.5, (kk * f * 0.87 + ph[2]).sin());
        d = (d + noise * p.noise_amt * 1.6 / n as f32 + Vec3::NEG_Y * p.gravity * 2.5 / n as f32).normalize();
        if p.jink > 0.0 && rng.next() < p.jink_freq {
            d = (d + vec3(rng.sym(), rng.sym() * 0.3, rng.sym()) * p.jink * 0.5).normalize();
        }
        pos += d * step;
        pts.push(pos);
    }
    let _ = node;
    Some(build_spine(node.id, pts, r0, p))
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
            let mut r = r0 * (1.0 - (1.0 - p.taper) * s.powf(0.9));
            if s < 0.18 {
                let q = 1.0 - s / 0.18;
                r *= 1.0 + p.flare * q * q;
            }
            r
        })
        .collect();
    Spine { node, pts, rad, tan, nrm, cum, len }
}

fn skin(s: &Spine, p: &Params, col: [u8; 4], geo: &mut TreeGeo, trunk: bool) {
    let radial = p.seg_radial.clamp(3, 48) as usize;
    let m = s.pts.len();
    let need = m * (radial + 1) + 1;
    let r0 = s.rad[0].max(0.01);
    let mat = geo.mats.entry(p.material).or_default();
    let ch = mat.chunk_for(need);
    let base = ch.v.len();
    let mut bmin = Vec3::splat(f32::MAX);
    let mut bmax = Vec3::splat(f32::MIN);
    for k in 0..m {
        let tn = s.tan[k];
        let n = s.nrm[k];
        let b = tn.cross(n);
        let sfrac = s.cum[k] / s.len;
        let v = s.cum[k] / (2.0 * PI * r0) * p.v_tile;
        let ao = if trunk { 0.8 + 0.2 * sfrac } else { 0.62 + 0.38 * sfrac };
        for j in 0..=radial {
            let a = j as f32 / radial as f32 * 2.0 * PI;
            let dir = n * a.cos() + b * a.sin();
            let pos = s.pts[k] + dir * s.rad[k];
            bmin = bmin.min(pos);
            bmax = bmax.max(pos);
            ch.v.push(vtx(pos, vec2(j as f32 / radial as f32 * p.u_tile, v), col, dir, ao));
        }
    }
    for k in 0..m - 1 {
        for j in 0..radial {
            let a = (base + k * (radial + 1) + j) as u16;
            let b = a + 1;
            let c = a + (radial + 1) as u16;
            let d = c + 1;
            ch.i.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    // tip cap
    let tip = s.pts[m - 1] + s.tan[m - 1] * s.rad[m - 1] * 0.5;
    let ci = ch.v.len() as u16;
    ch.v.push(vtx(tip, vec2(0.5, s.cum[m - 1] / (2.0 * PI * r0) * p.v_tile), col, s.tan[m - 1], 1.0));
    let last = (base + (m - 1) * (radial + 1)) as u16;
    for j in 0..radial as u16 {
        ch.i.extend_from_slice(&[last + j, ci, last + j + 1]);
    }
    geo.tris += (m - 1) * radial * 2 + radial;
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
