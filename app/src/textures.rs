//! Procedural textures, image loading, colour adjustments and automatic cutouts.
use crate::model::{ImageBuf, Material};
use macroquad::prelude::*;

fn hash2(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffffff) as f32 / 16777215.0
}

/// Tileable value noise with period (px, py) lattice cells.
fn vnoise(x: f32, y: f32, px: i32, py: i32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let fx = x - xi as f32;
    let fy = y - yi as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let w = |a: i32, p: i32| a.rem_euclid(p);
    let a = hash2(w(xi, px), w(yi, py), seed);
    let b = hash2(w(xi + 1, px), w(yi, py), seed);
    let c = hash2(w(xi, px), w(yi + 1, py), seed);
    let d = hash2(w(xi + 1, px), w(yi + 1, py), seed);
    let ab = a + (b - a) * sx;
    let cd = c + (d - c) * sx;
    ab + (cd - ab) * sy
}

fn fbm(x: f32, y: f32, px: i32, py: i32, oct: i32, seed: u32) -> f32 {
    let mut s = 0.0;
    let mut amp = 0.5;
    let mut f = 1.0;
    let mut norm = 0.0;
    for o in 0..oct {
        s += amp * vnoise(x * f, y * f, px * f as i32, py * f as i32, seed + o as u32 * 17);
        norm += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    s / norm
}

pub fn bark_image(n: usize) -> ImageBuf {
    let mut px = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let u = x as f32 / n as f32;
            let v = y as f32 / n as f32;
            // vertical fissures: stretched noise
            let warp = fbm(u * 4.0, v * 2.0, 4, 2, 3, 7) * 0.6;
            let ridge = fbm(u * 10.0 + warp * 3.0, v * 2.0, 10, 2, 4, 11);
            let fiss = (1.0 - (ridge * 2.0 - 1.0).abs()).powf(3.0);
            let grain = fbm(u * 24.0, v * 48.0, 24, 48, 2, 23);
            let t = (0.35 + 0.65 * (1.0 - fiss)) * (0.8 + 0.4 * grain);
            let r = 0.18 + 0.30 * t;
            let g = 0.13 + 0.22 * t;
            let b = 0.09 + 0.15 * t;
            let i = (y * n + x) * 4;
            px[i] = (r.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 3] = 255;
        }
    }
    ImageBuf { w: n, h: n, px }
}

pub fn noise_image(n: usize) -> ImageBuf {
    let mut px = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let t = fbm(x as f32 / n as f32 * 8.0, y as f32 / n as f32 * 8.0, 8, 8, 5, 3);
            let c = (t * 255.0) as u8;
            let i = (y * n + x) * 4;
            px[i] = c;
            px[i + 1] = c;
            px[i + 2] = c;
            px[i + 3] = 255;
        }
    }
    ImageBuf { w: n, h: n, px }
}

/// Leaf half-width at height t (0 = stem end at the bottom, 1 = tip).
fn leaf_half_width(t: f32) -> f32 {
    if t <= 0.0 || t >= 1.0 {
        return 0.0;
    }
    let base = (t * std::f32::consts::PI).sin().powf(0.75);
    let tip = (1.0 - t).powf(0.35);
    0.40 * base * (0.55 + 0.45 * tip)
}

pub fn leaf_image(n: usize) -> ImageBuf {
    let mut px = vec![0u8; n * n * 4];
    let stem_end = 0.92; // v where the blade starts (stem below)
    let tip_v = 0.03;
    for y in 0..n {
        for x in 0..n {
            let u = (x as f32 + 0.5) / n as f32;
            let v = (y as f32 + 0.5) / n as f32;
            let i = (y * n + x) * 4;
            let du = u - 0.5;
            // stem
            if v > stem_end && du.abs() < 0.012 {
                px[i] = 90;
                px[i + 1] = 110;
                px[i + 2] = 40;
                px[i + 3] = 255;
                continue;
            }
            let t = (stem_end - v) / (stem_end - tip_v);
            let hw = leaf_half_width(t);
            let edge = hw - du.abs();
            if edge <= 0.0 {
                continue;
            }
            // veins
            let mid = (1.0 - (du.abs() / 0.010)).max(0.0);
            let side = {
                let k = (t * 9.0 - du.abs() * 9.0 * 1.4).fract();
                let d = (k - 0.5).abs();
                ((0.06 - d) / 0.06).max(0.0) * (1.0 - du.abs() / hw.max(0.01)).max(0.0)
            };
            let vein = mid.max(side * 0.6);
            let shade = fbm(u * 12.0, v * 12.0, 12, 12, 3, 5);
            let g0 = 0.30 + 0.12 * t + 0.10 * shade;
            let r = (0.16 + 0.06 * shade) + vein * 0.18;
            let g = g0 + vein * 0.20;
            let b = 0.07 + vein * 0.08;
            let a = (edge * n as f32 * 0.9).clamp(0.0, 1.0);
            px[i] = (r.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 3] = (a * 255.0) as u8;
        }
    }
    ImageBuf { w: n, h: n, px }
}

pub fn white_image() -> ImageBuf {
    ImageBuf { w: 4, h: 4, px: vec![255u8; 64] }
}

/// Preferences > Render > Max texture resolution (picture 45): larger pictures are scaled down.
static MAX_TEX: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(2048);

pub fn set_max_texture(px: u32) {
    MAX_TEX.store(px.clamp(64, 16384), std::sync::atomic::Ordering::Relaxed);
}

/// Pictures already read by `preload` (path -> image), used once.
static PRELOADED: std::sync::Mutex<Vec<(String, ImageBuf)>> = std::sync::Mutex::new(Vec::new());

/// Preferences > General > Use multiple processors (picture 45): read several pictures at once,
/// one per processor, before the materials ask for them.
pub fn preload(paths: Vec<String>) {
    let paths: Vec<String> = paths.into_iter().filter(|p| !p.is_empty()).collect();
    if paths.len() < 2 {
        return;
    }
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get()).min(paths.len());
    let chunks: Vec<Vec<String>> = (0..threads).map(|t| paths.iter().skip(t).step_by(threads).cloned().collect()).collect();
    let done: Vec<(String, ImageBuf)> = std::thread::scope(|sc| {
        let hs: Vec<_> = chunks.into_iter().map(|c| sc.spawn(move || c.into_iter().filter_map(|p| decode_file(&p).ok().map(|i| (p, i))).collect::<Vec<_>>())).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    });
    if let Ok(mut v) = PRELOADED.lock() {
        v.extend(done);
    }
}

fn decode_file(path: &str) -> Result<ImageBuf, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let max = MAX_TEX.load(std::sync::atomic::Ordering::Relaxed);
    let img = if img.width() > max || img.height() > max {
        img.resize(max, max, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    Ok(ImageBuf { w: rgba.width() as usize, h: rgba.height() as usize, px: rgba.into_raw() })
}

pub fn load_image_file(path: &str) -> Result<ImageBuf, String> {
    if let Ok(mut v) = PRELOADED.lock() {
        if let Some(i) = v.iter().position(|(p, _)| p == path) {
            return Ok(v.swap_remove(i).1);
        }
    }
    decode_file(path)
}

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let h = if d < 1e-6 {
        0.0
    } else if mx == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    let s = if mx <= 0.0 { 0.0 } else { d / mx };
    (h, s, mx)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
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

/// Let a texture repeat past 0..1 both ways, so bark tiles along and around branches by its UVs
/// (picture 33; textures start out clamped at their edges).
pub fn set_repeat(t: &Texture2D) {
    let id = t.raw_miniquad_id();
    unsafe { get_internal_gl() }.quad_context.texture_set_wrap(id, macroquad::miniquad::TextureWrap::Repeat, macroquad::miniquad::TextureWrap::Repeat);
}

/// Rebuild a material's base picture and GPU texture when its settings changed.
pub fn refresh_material(m: &mut Material) {
    if m.dirty_base {
        // the Color slot's tick box: off means the colour value alone (plain white picture)
        let color_on = m.slots.first().map(|s| s.enabled).unwrap_or(true);
        m.base = Some(match if color_on { m.source } else { 0 } {
            1 => bark_image(256),
            2 => leaf_image(256),
            3 => noise_image(256),
            4 => {
                if m.file.is_empty() {
                    white_image()
                } else {
                    match load_image_file(&m.file) {
                        Ok(i) => i,
                        Err(_) => white_image(),
                    }
                }
            }
            _ => white_image(),
        });
        m.dirty_base = false;
        m.dirty_tex = true;
    }
    if m.dirty_tex {
        let base = m.base.as_ref().unwrap();
        let mut out = base.clone();
        // the Color slot's Edit Map adjustments (picture 74) come first
        if let Some(a) = m.slots.first().map(|s| s.adjust.clone()) {
            if a != crate::model::MapAdjust::default() {
                out = adjust_image(&out, &a, map_kind(0));
            }
        }
        let br = m.brightness;
        let ct = 1.0 + m.contrast;
        let sat = 1.0 + m.saturation;
        let hue = m.hue;
        let plain = br == 0.0 && m.contrast == 0.0 && m.saturation == 0.0 && hue == 0.0;
        for p in out.px.chunks_mut(4) {
            if !plain {
                let mut r = p[0] as f32 / 255.0;
                let mut g = p[1] as f32 / 255.0;
                let mut b = p[2] as f32 / 255.0;
                r = (r - 0.5) * ct + 0.5 + br;
                g = (g - 0.5) * ct + 0.5 + br;
                b = (b - 0.5) * ct + 0.5 + br;
                let (h, s, v) = rgb_to_hsv(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
                let (r2, g2, b2) = hsv_to_rgb(h + hue, (s * sat).clamp(0.0, 1.0), v);
                p[0] = (r2.clamp(0.0, 1.0) * 255.0) as u8;
                p[1] = (g2.clamp(0.0, 1.0) * 255.0) as u8;
                p[2] = (b2.clamp(0.0, 1.0) * 255.0) as u8;
            }
            if !m.opacity_from_alpha {
                p[3] = 255;
            }
        }
        // Opacity slot: a loaded opacity picture (its brightness) replaces the colour picture's alpha;
        // the Opacity value multiplies it
        let op_img = m.slots.get(1).filter(|s| s.enabled).and_then(|s| s.image.as_ref().map(|i| adjust_image(i, &s.adjust, map_kind(1))));
        if op_img.is_some() || (m.opacity - 1.0).abs() > 1e-4 {
            let (w, h) = (out.w, out.h);
            for y in 0..h {
                for x in 0..w {
                    let i = (y * w + x) * 4;
                    let mut a = out.px[i + 3] as f32 / 255.0;
                    if let Some(op) = &op_img {
                        let ox = (x * op.w / w.max(1)).min(op.w - 1);
                        let oy = (y * op.h / h.max(1)).min(op.h - 1);
                        let j = (oy * op.w + ox) * 4;
                        a = (op.px[j] as f32 + op.px[j + 1] as f32 + op.px[j + 2] as f32) / (3.0 * 255.0);
                    }
                    out.px[i + 3] = ((a * m.opacity).clamp(0.0, 1.0) * 255.0) as u8;
                }
            }
        }
        let tex = Texture2D::from_rgba8(out.w as u16, out.h as u16, &out.px);
        tex.set_filter(FilterMode::Linear);
        set_repeat(&tex);
        m.tex = Some(tex);
        m.adjusted = Some(out);
        m.dirty_tex = false;
    }
}

/// What a texture slot holds, for its Edit Map adjustments (picture 74): 0 a colour (Color,
/// Subsurface, Custom), 1 a normal map, 2 a single value (Opacity, Roughness, Specular, Metallic,
/// Subsurface%, AO).
pub fn map_kind(slot: usize) -> u8 {
    match slot {
        0 | 6 | 9 => 0,
        2 => 1,
        _ => 2,
    }
}

/// A slot's picture with its Edit Map adjustments applied (picture 74).
/// - Colour maps: sRGB to Linear off takes the picture as linear values and shows them as sRGB
///   (power 1/2.2); Invert / Invert R, G, B; Brightness adds; Contrast scales around the middle by
///   (1 + Contrast); Saturation scales it by (1 + Saturation); Red, Green and Blue add to their
///   channel; then the result is laid into Minimum..Maximum.
/// - Normal maps: Invert R / G / B flip that axis (green: up and down), Invert flips all three,
///   Normalize rescales each texel to unit length.
/// - Value maps: the value is read from Channel (luminance, red, green, blue or alpha), inverted,
///   brightened, contrasted, laid into Minimum..Maximum and written to red, green and blue.
pub fn adjust_image(img: &ImageBuf, a: &crate::model::MapAdjust, kind: u8) -> ImageBuf {
    let mut out = img.clone();
    let range = |v: f32| (a.min + v.clamp(0.0, 1.0) * (a.max - a.min)).clamp(0.0, 1.0);
    let tone = |v: f32| ((v - 0.5) * (1.0 + a.contrast) + 0.5 + a.brightness).clamp(0.0, 1.0);
    for p in out.px.chunks_mut(4) {
        let mut c = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
        match kind {
            0 => {
                if !a.srgb {
                    for v in &mut c {
                        *v = v.powf(1.0 / 2.2);
                    }
                }
                for k in 0..3 {
                    if a.invert ^ a.invert_rgb[k] {
                        c[k] = 1.0 - c[k];
                    }
                    c[k] = tone(c[k]);
                }
                if a.saturation != 0.0 {
                    let (h, s, v) = rgb_to_hsv(c[0], c[1], c[2]);
                    let (r, g, b) = hsv_to_rgb(h, (s * (1.0 + a.saturation)).clamp(0.0, 1.0), v);
                    c = [r, g, b];
                }
                c[0] += a.red;
                c[1] += a.green;
                c[2] += a.blue;
                for v in &mut c {
                    *v = range(*v);
                }
            }
            1 => {
                for k in 0..3 {
                    if a.invert ^ a.invert_rgb[k] {
                        c[k] = 1.0 - c[k];
                    }
                }
                if a.normalize {
                    let v = vec3(c[0] * 2.0 - 1.0, c[1] * 2.0 - 1.0, c[2] * 2.0 - 1.0).normalize_or(Vec3::Z);
                    c = [v.x * 0.5 + 0.5, v.y * 0.5 + 0.5, v.z * 0.5 + 0.5];
                }
            }
            _ => {
                let mut v = match a.channel {
                    1 => c[0],
                    2 => c[1],
                    3 => c[2],
                    4 => p[3] as f32 / 255.0,
                    _ => c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722,
                };
                if a.invert {
                    v = 1.0 - v;
                }
                v = range(tone(v));
                c = [v, v, v];
            }
        }
        for k in 0..3 {
            p[k] = (c[k].clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

/// Rebuild one slot's GPU picture from its loaded picture and adjustments (not the Color slot,
/// which the material builds).
pub fn refresh_slot(sl: &mut crate::model::MatSlot, slot: usize) {
    if slot == 0 {
        return;
    }
    if let Some(img) = &sl.image {
        let out = adjust_image(img, &sl.adjust, map_kind(slot));
        let t = Texture2D::from_rgba8(out.w as u16, out.h as u16, &out.px);
        t.set_filter(FilterMode::Linear);
        set_repeat(&t);
        sl.tex = Some(t);
    }
}

fn cross2(o: Vec2, a: Vec2, b: Vec2) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn convex_hull(mut pts: Vec<Vec2>) -> Vec<Vec2> {
    pts.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap().then(a.y.partial_cmp(&b.y).unwrap()));
    pts.dedup_by(|a, b| (*a - *b).length() < 1e-6);
    if pts.len() < 3 {
        return pts;
    }
    let mut lower: Vec<Vec2> = vec![];
    for p in &pts {
        while lower.len() >= 2 && cross2(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<Vec2> = vec![];
    for p in pts.iter().rev() {
        while upper.len() >= 2 && cross2(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// Reduce a convex outline to `n` points: repeatedly drop the corner whose removal
/// adds the least area when its neighbours' edges are extended to meet (keeps coverage).
pub fn reduce_outline(mut poly: Vec<Vec2>, n: usize) -> Vec<Vec2> {
    let n = n.max(3);
    while poly.len() > n {
        let m = poly.len();
        let mut best = (f32::MAX, 0usize, Vec2::ZERO);
        for i in 0..m {
            // replace edge (i, i+1) by extending edges (i-1,i) and (i+1,i+2)
            let a0 = poly[(i + m - 1) % m];
            let a1 = poly[i];
            let b0 = poly[(i + 1) % m];
            let b1 = poly[(i + 2) % m];
            let d1 = a1 - a0;
            let d2 = b0 - b1;
            let den = d1.x * d2.y - d1.y * d2.x;
            if den.abs() < 1e-9 {
                continue;
            }
            let t = ((b1.x - a0.x) * d2.y - (b1.y - a0.y) * d2.x) / den;
            if t < 1.0 {
                continue; // edges diverge; extension would go backwards
            }
            let p = a0 + d1 * t;
            let area = cross2(a1, p, b0).abs() * 0.5;
            if area < best.0 {
                best = (area, i, p);
            }
        }
        if best.0 == f32::MAX {
            // fall back: drop the point with the smallest triangle
            let mut bi = 0;
            let mut ba = f32::MAX;
            for i in 0..m {
                let a = cross2(poly[(i + m - 1) % m], poly[i], poly[(i + 1) % m]).abs();
                if a < ba {
                    ba = a;
                    bi = i;
                }
            }
            poly.remove(bi);
            continue;
        }
        let (_, i, p) = best;
        let j = (i + 1) % m;
        poly[i] = p;
        poly.remove(j);
    }
    poly
}

/// Build a cutout outline around the opaque part of a picture.
pub fn auto_cutout(img: &ImageBuf, points: usize, threshold: f32, margin: f32) -> Vec<Vec2> {
    let res = 96usize;
    let mut pts = vec![];
    for y in 0..res {
        for x in 0..res {
            let u = (x as f32 + 0.5) / res as f32;
            let v = (y as f32 + 0.5) / res as f32;
            if img.alpha_at(u, v) > threshold {
                let h = 0.5 / res as f32;
                pts.push(vec2(u - h, v - h));
                pts.push(vec2(u + h, v - h));
                pts.push(vec2(u + h, v + h));
                pts.push(vec2(u - h, v + h));
            }
        }
    }
    if pts.len() < 3 {
        return crate::model::card_outline();
    }
    let hull = convex_hull(pts);
    let c = hull.iter().fold(Vec2::ZERO, |a, b| a + *b) / hull.len() as f32;
    let grown: Vec<Vec2> = hull.iter().map(|p| *p + (*p - c).normalize_or_zero() * margin).collect();
    let red = reduce_outline(grown, points);
    red.iter().map(|p| vec2(p.x.clamp(-0.05, 1.05), p.y.clamp(-0.05, 1.05))).collect()
}

/// Ear-clipping triangulation of a simple polygon; returns index triples.
pub fn triangulate(poly: &[Vec2]) -> Vec<[usize; 3]> {
    let n = poly.len();
    let mut out = vec![];
    if n < 3 {
        return out;
    }
    let area: f32 = (0..n).map(|i| {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        a.x * b.y - b.x * a.y
    }).sum();
    let mut idx: Vec<usize> = (0..n).collect();
    if area < 0.0 {
        idx.reverse();
    }
    let mut guard = 0;
    while idx.len() > 3 && guard < 10000 {
        guard += 1;
        let m = idx.len();
        let mut found = false;
        for i in 0..m {
            let ia = idx[(i + m - 1) % m];
            let ib = idx[i];
            let ic = idx[(i + 1) % m];
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            if cross2(a, b, c) <= 1e-9 {
                continue;
            }
            let mut inside = false;
            for &j in &idx {
                if j == ia || j == ib || j == ic {
                    continue;
                }
                let p = poly[j];
                if cross2(a, b, p) >= 0.0 && cross2(b, c, p) >= 0.0 && cross2(c, a, p) >= 0.0 {
                    inside = true;
                    break;
                }
            }
            if inside {
                continue;
            }
            out.push([ia, ib, ic]);
            idx.remove(i);
            found = true;
            break;
        }
        if !found {
            // degenerate: fan the rest
            for k in 1..idx.len() - 1 {
                out.push([idx[0], idx[k], idx[k + 1]]);
            }
            return out;
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}
