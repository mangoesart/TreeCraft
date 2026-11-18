//! TreeCraft's icon (picture 23), drawn in code so the build script can put it into the program
//! file and the window can show it too: a rounded indigo-to-teal tile with a cream branching
//! tree (each branch splits in two, five times) and lime / gold leaf dots at its tips.

struct Seg {
    a: (f32, f32),
    b: (f32, f32),
    r: f32,
}

fn grow(out: &mut Vec<Seg>, tips: &mut Vec<(f32, f32, bool)>, p: (f32, f32), ang: f32, len: f32, r: f32, depth: u32, left: bool) {
    let e = (p.0 + len * ang.sin(), p.1 - len * ang.cos());
    out.push(Seg { a: p, b: e, r });
    if depth == 0 {
        tips.push((e.0, e.1, left));
        return;
    }
    let spread = 0.42 + 0.06 * depth as f32;
    grow(out, tips, e, ang - spread, len * 0.78, r * 0.7, depth - 1, true);
    grow(out, tips, e, ang + spread * 0.9, len * 0.74, r * 0.7, depth - 1, false);
}

fn seg_dist(p: (f32, f32), s: &Seg) -> f32 {
    let (dx, dy) = (s.b.0 - s.a.0, s.b.1 - s.a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((p.0 - s.a.0) * dx + (p.1 - s.a.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    let (qx, qy) = (s.a.0 + t * dx - p.0, s.a.1 + t * dy - p.1);
    (qx * qx + qy * qy).sqrt()
}

/// The icon as `size` x `size` RGBA pixels, rows from the top.
pub fn render(size: u32) -> Vec<u8> {
    let mut segs = vec![];
    let mut tips = vec![];
    // a short tapering trunk, then three limbs that keep splitting into a round crown
    segs.push(Seg { a: (0.5, 0.89), b: (0.5, 0.77), r: 0.052 });
    segs.push(Seg { a: (0.5, 0.77), b: (0.5, 0.64), r: 0.040 });
    for (a, l) in [(-0.75f32, 0.13f32), (0.05, 0.14), (0.8, 0.12)] {
        grow(&mut segs, &mut tips, (0.5, 0.64), a, l, 0.03, 3, a < 0.0);
    }
    let leaf_r = 0.062;
    let n = size as usize;
    let ss = if size <= 64 { 4 } else { 3 };
    let mut px = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let mut acc = [0.0f32; 4];
            for sy in 0..ss {
                for sx in 0..ss {
                    let u = (x as f32 + (sx as f32 + 0.5) / ss as f32) / size as f32;
                    let v = (y as f32 + (sy as f32 + 0.5) / ss as f32) / size as f32;
                    // rounded tile
                    let (cr, m) = (0.22, 0.04);
                    let qx = (u - 0.5).abs() - (0.5 - m - cr);
                    let qy = (v - 0.5).abs() - (0.5 - m - cr);
                    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - cr;
                    if outside > 0.0 {
                        continue;
                    }
                    // indigo at the top to teal at the bottom, a soft glow behind the crown
                    let glow = (1.0 - (((u - 0.5).powi(2) + (v - 0.42).powi(2)).sqrt() / 0.42)).max(0.0) * 0.22;
                    let mut c = [0.17 + (0.05 - 0.17) * v + glow, 0.19 + (0.42 - 0.19) * v + glow, 0.44 + (0.44 - 0.44) * v + glow * 0.8];
                    // ground line
                    if (v - 0.905).abs() < 0.012 && (u - 0.5).abs() < 0.26 {
                        c = [0.10, 0.27, 0.30];
                    }
                    // the crown's shade behind the branches
                    let e = ((u - 0.5) / 0.33).powi(2) + ((v - 0.40) / 0.25).powi(2);
                    if e < 1.0 {
                        let k = 1.0 - e * 0.35;
                        c = [0.13 * k, 0.45 * k, 0.36 * k];
                    }
                    let p = (u, v);
                    if segs.iter().any(|s| seg_dist(p, s) < s.r) {
                        c = [0.98, 0.91, 0.76];
                    }
                    for (k, t) in tips.iter().enumerate() {
                        let d = ((u - t.0).powi(2) + (v - t.1).powi(2)).sqrt();
                        if d < leaf_r {
                            let base = if k % 3 == 1 { [1.0, 0.80, 0.30] } else if t.2 { [0.58, 0.92, 0.42] } else { [0.36, 0.82, 0.48] };
                            // a lighter spot towards the top left of each leaf
                            let hl = (1.0 - ((u - t.0 + leaf_r * 0.35).powi(2) + (v - t.1 + leaf_r * 0.35).powi(2)).sqrt() / leaf_r).max(0.0) * 0.25;
                            c = [(base[0] + hl).min(1.0), (base[1] + hl).min(1.0), (base[2] + hl).min(1.0)];
                        }
                    }
                    acc[0] += c[0];
                    acc[1] += c[1];
                    acc[2] += c[2];
                    acc[3] += 1.0;
                }
            }
            let k = (ss * ss) as f32;
            let i = (y * n + x) * 4;
            if acc[3] > 0.0 {
                for j in 0..3 {
                    px[i + j] = ((acc[j] / acc[3]).clamp(0.0, 1.0) * 255.0) as u8;
                }
                px[i + 3] = (acc[3] / k * 255.0) as u8;
            }
        }
    }
    px
}

#[cfg(test)]
mod tests {
    #[test]
    fn icon_is_a_rounded_tile_with_a_tree() {
        let px = super::render(64);
        // transparent corners, opaque middle
        assert_eq!(px[3], 0);
        assert_eq!(px[(32 * 64 + 32) * 4 + 3], 255);
        if let Ok(p) = std::env::var("TC_ICON_PNG") {
            let big = super::render(256);
            let _ = image::save_buffer(p, &big, 256, 256, image::ColorType::Rgba8);
        }
    }
}
