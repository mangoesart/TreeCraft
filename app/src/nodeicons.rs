//! Icons inside the Generation editor's node discs (picture 66): TreeCraft's own drawings, made
//! from shapes at draw time (no picture files), white on the kind's colour, so each node reads at a
//! glance: a tree with a layered crown, a trunk with flared roots and a cut top, a forked twig with
//! buds, a veined leaf on its stalk, an arching frond with leaflets, an arrow for forces and a
//! sphere for collision objects.
use macroquad::prelude::*;
use std::cell::Cell;
use std::f32::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeIcon {
    None,
    Tree,
    Trunk,
    Branch,
    Leaf,
    Frond,
    Force,
    Collision,
}

thread_local! {
    static NEXT: Cell<NodeIcon> = const { Cell::new(NodeIcon::None) };
}

/// The icon the next node drawn shows (taken by the drawing, so it applies once).
pub fn set_next(i: NodeIcon) {
    NEXT.with(|c| c.set(i));
}

/// The icon set for this node (and clears it).
pub fn take_next() -> NodeIcon {
    NEXT.with(|c| c.replace(NodeIcon::None))
}

/// Space for one icon: centre and size, y pointing up.
struct Pen {
    c: Vec2,
    s: f32,
    fg: Color,
    shade: Color,
}

impl Pen {
    fn p(&self, x: f32, y: f32) -> Vec2 {
        vec2(self.c.x + x * self.s, self.c.y - y * self.s)
    }
    fn line(&self, a: (f32, f32), b: (f32, f32), w: f32, col: Color) {
        let (a, b) = (self.p(a.0, a.1), self.p(b.0, b.1));
        draw_line(a.x, a.y, b.x, b.y, (w * self.s).max(1.0), col);
        // round ends
        draw_circle(a.x, a.y, (w * self.s * 0.5).max(0.5), col);
        draw_circle(b.x, b.y, (w * self.s * 0.5).max(0.5), col);
    }
    /// A tapering stroke from width `w0` at `a` to `w1` at `b`.
    fn taper(&self, a: (f32, f32), b: (f32, f32), w0: f32, w1: f32, col: Color) {
        let (pa, pb) = (self.p(a.0, a.1), self.p(b.0, b.1));
        let d = (pb - pa).normalize_or_zero();
        let n = vec2(-d.y, d.x);
        let (h0, h1) = (w0 * self.s * 0.5, w1 * self.s * 0.5);
        draw_triangle(pa + n * h0, pa - n * h0, pb + n * h1, col);
        draw_triangle(pa - n * h0, pb - n * h1, pb + n * h1, col);
        draw_circle(pa.x, pa.y, h0, col);
        draw_circle(pb.x, pb.y, h1, col);
    }
    fn circle(&self, x: f32, y: f32, r: f32, col: Color) {
        let q = self.p(x, y);
        draw_circle(q.x, q.y, r * self.s, col);
    }
    fn poly(&self, pts: &[(f32, f32)], col: Color) {
        if pts.len() < 3 {
            return;
        }
        let v: Vec<Vec2> = pts.iter().map(|q| self.p(q.0, q.1)).collect();
        let c = v.iter().fold(Vec2::ZERO, |a, b| a + *b) / v.len() as f32;
        for k in 0..v.len() {
            draw_triangle(c, v[k], v[(k + 1) % v.len()], col);
        }
    }
    /// A leaf blade from `base` to `tip`, `w` wide at its widest (a little nearer the base).
    fn blade(&self, base: (f32, f32), tip: (f32, f32), w: f32, col: Color) {
        let (b, t) = (vec2(base.0, base.1), vec2(tip.0, tip.1));
        let d = t - b;
        let n = vec2(-d.y, d.x).normalize_or_zero();
        let steps = 10;
        let mut pts = vec![];
        for k in 0..=steps {
            let u = k as f32 / steps as f32;
            let half = w * 0.5 * (PI * u).sin().powf(0.75) * (1.0 - 0.25 * u);
            let q = b + d * u + n * half;
            pts.push((q.x, q.y));
        }
        for k in (1..steps).rev() {
            let u = k as f32 / steps as f32;
            let half = w * 0.5 * (PI * u).sin().powf(0.75) * (1.0 - 0.25 * u);
            let q = b + d * u - n * half;
            pts.push((q.x, q.y));
        }
        // fan from a point inside the blade (its middle) so the outline stays filled
        let v: Vec<Vec2> = pts.iter().map(|q| self.p(q.0, q.1)).collect();
        let m = self.p(b.x + d.x * 0.45, b.y + d.y * 0.45);
        for k in 0..v.len() {
            draw_triangle(m, v[k], v[(k + 1) % v.len()], col);
        }
    }
}

/// Draws `icon` in a disc of radius `rad` at (`cx`, `cy`) whose fill is `bg`.
pub fn draw(icon: NodeIcon, cx: f32, cy: f32, rad: f32, bg: Color, alpha: f32) {
    let fg = Color::new(0.97, 0.97, 0.95, 0.96 * alpha);
    // the disc's colour, darker, for details cut into the white shapes
    let shade = Color::new(bg.r * 0.62, bg.g * 0.62, bg.b * 0.62, alpha);
    let pen = Pen { c: vec2(cx, cy), s: rad * 0.9, fg, shade };
    match icon {
        NodeIcon::None => {}
        NodeIcon::Tree => tree(&pen),
        NodeIcon::Trunk => trunk(&pen),
        NodeIcon::Branch => branch(&pen),
        NodeIcon::Leaf => leaf(&pen),
        NodeIcon::Frond => frond(&pen),
        NodeIcon::Force => force(&pen),
        NodeIcon::Collision => collision(&pen),
    }
}

/// A whole tree: a crown of overlapping rounded clumps over a straight trunk, on a ground line.
fn tree(p: &Pen) {
    let (fg, sh) = (p.fg, p.shade);
    // ground and trunk
    p.line((-0.58, -0.68), (0.58, -0.68), 0.07, fg);
    p.taper((0.0, -0.68), (0.0, 0.05), 0.2, 0.09, fg);
    // two limbs into the crown
    p.taper((0.0, -0.18), (-0.24, 0.08), 0.08, 0.04, fg);
    p.taper((0.0, -0.08), (0.22, 0.14), 0.07, 0.035, fg);
    // the crown: clumps, then darker gaps between them so it reads as layered foliage
    for (x, y, r) in [(-0.36, 0.14, 0.27), (0.36, 0.18, 0.26), (0.0, 0.22, 0.32), (-0.18, 0.5, 0.26), (0.2, 0.5, 0.25)] {
        p.circle(x, y, r, fg);
    }
    for (a, b) in [((-0.36, 0.0), (-0.12, 0.18)), ((0.36, 0.04), (0.14, 0.22)), ((-0.04, 0.4), (0.08, 0.56))] {
        p.line(a, b, 0.035, sh);
    }
}

/// A trunk: tapering, with flared roots at the ground, one stub branch and a cut top showing
/// its rings.
fn trunk(p: &Pen) {
    let (fg, sh) = (p.fg, p.shade);
    p.poly(&[(-0.2, -0.5), (-0.13, 0.62), (0.13, 0.62), (0.2, -0.5)], fg);
    // roots
    p.poly(&[(-0.2, -0.36), (-0.6, -0.7), (-0.06, -0.62)], fg);
    p.poly(&[(0.2, -0.36), (0.6, -0.7), (0.06, -0.62)], fg);
    p.poly(&[(-0.12, -0.5), (0.0, -0.74), (0.12, -0.5)], fg);
    // a stub branch
    p.taper((0.1, 0.2), (0.44, 0.44), 0.12, 0.06, fg);
    // the cut top: an oval with a ring
    let top = p.p(0.0, 0.62);
    draw_ellipse(top.x, top.y, 0.13 * p.s, 0.055 * p.s, 0.0, fg);
    draw_ellipse_lines(top.x, top.y, 0.07 * p.s, 0.028 * p.s, 0.0, (0.02 * p.s).max(1.0), sh);
    // bark grooves
    p.line((-0.06, -0.36), (-0.04, 0.4), 0.03, sh);
    p.line((0.07, -0.3), (0.05, 0.12), 0.03, sh);
}

/// A branch: a forked twig rising to the right, with side shoots ending in buds.
fn branch(p: &Pen) {
    let fg = p.fg;
    p.taper((-0.62, -0.6), (0.56, 0.5), 0.22, 0.07, fg);
    p.taper((-0.16, -0.17), (-0.5, 0.36), 0.11, 0.05, fg);
    p.taper((0.16, 0.13), (0.6, -0.02), 0.1, 0.045, fg);
    p.taper((-0.36, 0.13), (-0.2, 0.44), 0.06, 0.03, fg);
    // buds: small blades at the tips
    p.blade((0.56, 0.5), (0.72, 0.7), 0.17, fg);
    p.blade((-0.5, 0.36), (-0.63, 0.58), 0.15, fg);
    p.blade((0.6, -0.02), (0.8, 0.05), 0.14, fg);
    p.blade((-0.2, 0.44), (-0.13, 0.64), 0.13, fg);
}

/// A leaf: a pointed blade on a short stalk with its midrib and veins.
fn leaf(p: &Pen) {
    let (fg, sh) = (p.fg, p.shade);
    let (b, t) = ((-0.48, -0.48), (0.6, 0.6));
    p.line((-0.72, -0.72), b, 0.06, fg);
    p.blade(b, t, 0.62, fg);
    // midrib and veins, curving toward the tip
    p.line((-0.44, -0.44), (0.5, 0.5), 0.035, sh);
    let d = vec2(t.0 - b.0, t.1 - b.1);
    let n = vec2(-d.y, d.x).normalize_or_zero();
    for u in [0.25f32, 0.42, 0.59, 0.75] {
        let on = vec2(b.0, b.1) + d * u;
        let reach = 0.22 * (PI * (u + 0.1)).sin();
        for side in [1.0f32, -1.0] {
            let end = vec2(b.0, b.1) + d * (u + 0.12) + n * reach * side;
            p.line((on.x, on.y), (end.x, end.y), 0.025, sh);
        }
    }
}

/// A frond: an arching stem with paired leaflets shrinking toward its tip.
fn frond(p: &Pen) {
    let fg = p.fg;
    // the stem: a curve from the lower left, rising and bending over to the right
    let (a, c, b) = (vec2(-0.62, -0.66), vec2(-0.42, 0.62), vec2(0.62, 0.5));
    let at = |t: f32| a * (1.0 - t) * (1.0 - t) + c * 2.0 * t * (1.0 - t) + b * t * t;
    let tan = |t: f32| ((c - a) * 2.0 * (1.0 - t) + (b - c) * 2.0 * t).normalize_or_zero();
    let steps = 14;
    for k in 0..steps {
        let (u, v) = (k as f32 / steps as f32, (k + 1) as f32 / steps as f32);
        let (q0, q1) = (at(u), at(v));
        p.taper((q0.x, q0.y), (q1.x, q1.y), 0.09 - 0.05 * u, 0.09 - 0.05 * v, fg);
    }
    // leaflets: in pairs, swept toward the tip, shorter further out
    let mut t = 0.2;
    while t < 0.97 {
        let q = at(t);
        let d = tan(t);
        let n = vec2(-d.y, d.x);
        let len = 0.42 * (1.0 - 0.58 * t);
        for side in [1.0f32, -1.0] {
            let dir = (n * side * 0.8 + d * 0.6).normalize_or_zero();
            let tip = q + dir * len;
            p.blade((q.x, q.y), (tip.x, tip.y), len * 0.46, fg);
        }
        t += 0.105;
    }
}

/// A force: a bold curved push arrow.
fn force(p: &Pen) {
    let fg = p.fg;
    let pts: Vec<Vec2> = (0..=10).map(|k| {
        let t = k as f32 / 10.0;
        vec2(-0.6 + 1.0 * t, -0.35 + 0.6 * t * t)
    }).collect();
    for w in pts.windows(2) {
        p.taper((w[0].x, w[0].y), (w[1].x, w[1].y), 0.14, 0.14, fg);
    }
    p.poly(&[(0.62, 0.42), (0.22, 0.42), (0.5, 0.0)], fg);
}

/// A collision object: a shaded sphere with its equator.
fn collision(p: &Pen) {
    let (fg, sh) = (p.fg, p.shade);
    p.circle(0.0, 0.0, 0.58, fg);
    let c = p.p(0.0, 0.0);
    draw_ellipse_lines(c.x, c.y, 0.58 * p.s, 0.2 * p.s, 0.0, (0.04 * p.s).max(1.0), sh);
    p.line((0.0, -0.58), (0.0, 0.58), 0.035, sh);
    p.circle(-0.2, 0.24, 0.1, Color::new(1.0, 1.0, 1.0, fg.a));
}
