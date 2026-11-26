//! PivotPainter2 (TreeCraft's own tool; the original SpeedTree has nothing like it): bakes every
//! branch and leaf of the tree into the textures Unreal Engine's Pivot Painter 2 wind materials
//! read, and gives the exported FBX a second UV set that points each part at its pixel.
//!
//! Each branch or leaf is an "element" (gen.rs PPElem): its pivot (where it starts), its X axis
//! (along it), Y axis (across it), its size, its parent element and how deep it sits. The elements
//! are laid out one pixel each, row by row from the top left of a W x H texture; every vertex of
//! an element gets the UV of its pixel's centre in UV channel 2.
//!
//! Textures (the layout Unreal's Pivot Painter 2 material functions expect, written by TreeCraft's
//! own code):
//! - Pivot Position (16-bit): RGB = the pivot in Unreal's space and centimetres, as 16-bit floats
//!   in an OpenEXR file; alpha one of Parent Index (Int as Float: the 16-bit float whose bit
//!   pattern is the index + 1024), Parent Index (Float: index + 0.5, up to 2048), a random 0-1
//!   value per element or the hierarchy position 0-1.
//! - X, Y or Z Vector (8-bit): RGB = the element's axis as (v + 1) / 2 in a Targa file; alpha one
//!   of X, Y or Z extent (centimetres / 8, rounded up, 1..256, over 256: up to 2048 cm), a random
//!   value or the hierarchy position.
//!
//! In Unreal: import the EXR as HDR (RGBA16F) and the TGA as VectorDisplacementmap (or with sRGB
//! off), both with Nearest filtering and no mip maps, and use UV channel 2 in the material.

use crate::app::App;
use crate::props::Rows;
use macroquad::prelude::*;

pub const RGB: [&str; 5] = ["Do not render", "Pivot Position (16-bit)", "X Vector (8-bit)", "Y Vector (8-bit)", "Z Vector (8-bit)"];
pub const ALPHA_HDR: [&str; 4] = ["Parent Index (Int as Float)", "Parent Index (Float, up to 2048)", "Random 0-1 per element", "Hierarchy position 0-1"];
pub const ALPHA_LDR: [&str; 5] = ["X Extent (divided by 2048)", "Y Extent (divided by 2048)", "Z Extent (divided by 2048)", "Random 0-1 per element", "Hierarchy position 0-1"];

/// The tool's settings (kept for the session).
#[derive(Clone, Debug)]
pub struct PP2 {
    /// Texture 1 and 2: (RGB choice, alpha choice).
    pub tex: [(usize, usize); 2],
    /// Bake the textures and the second UV set with every FBX export.
    pub with_fbx: bool,
}

impl Default for PP2 {
    fn default() -> Self {
        // Unreal's foliage wind material reads these two
        PP2 { tex: [(1, 0), (2, 0)], with_fbx: true }
    }
}

/// One element ready to bake, already in Unreal's space (centimetres; X forward, Y right, Z up).
#[derive(Clone, Debug)]
pub struct PPOut {
    pub pivot: Vec3,
    pub x: Vec3,
    pub y: Vec3,
    pub z: Vec3,
    /// Size along X, Y and Z in centimetres.
    pub ext: Vec3,
    pub parent: usize,
    pub depth: u32,
    pub random: f32,
}

/// The texture size for `n` elements: as square as possible, row by row.
pub fn dims(n: usize) -> (usize, usize) {
    let n = n.max(1);
    let w = (n as f32).sqrt().ceil() as usize;
    let h = n.div_ceil(w);
    (w.max(1), h.max(1))
}

/// The UV (TreeCraft's convention: v down from the top) of element `i`'s pixel centre.
pub fn uv(i: usize, w: usize, h: usize) -> Vec2 {
    vec2(((i % w) as f32 + 0.5) / w as f32, ((i / w) as f32 + 0.5) / h as f32)
}

/// f32 to IEEE 16-bit float bits (round to nearest; out of range -> infinity).
pub fn f32_to_f16(v: f32) -> u16 {
    let b = v.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32;
    let mant = b & 0x7f_ffff;
    if exp == 0xff {
        return sign | 0x7c00 | if mant != 0 { 0x200 } else { 0 };
    }
    let e = exp - 127 + 15;
    if e >= 0x1f {
        return sign | 0x7c00;
    }
    if e <= 0 {
        if e < -10 {
            return sign;
        }
        let m = (mant | 0x80_0000) >> (1 - e);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    let h = sign as u32 | ((e as u32) << 10) | (mant >> 13);
    // round half up on the dropped bits
    (h + ((mant >> 12) & 1)) as u16
}

/// A hierarchy depth turned into 0..1 (0 = root, 1 = the deepest element).
fn depth01(d: u32, max: u32) -> f32 {
    if max == 0 { 0.0 } else { d as f32 / max as f32 }
}

/// Write an RGBA image of 16-bit floats (raw half bits, row by row from the top) as an
/// uncompressed scanline OpenEXR file.
pub fn write_exr_half(path: &std::path::Path, w: usize, h: usize, px: &[[u16; 4]]) -> std::io::Result<()> {
    let mut out: Vec<u8> = vec![];
    out.extend_from_slice(&0x0131_2f76u32.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    let attr = |out: &mut Vec<u8>, name: &str, ty: &str, val: &[u8]| {
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        out.extend_from_slice(ty.as_bytes());
        out.push(0);
        out.extend_from_slice(&(val.len() as i32).to_le_bytes());
        out.extend_from_slice(val);
    };
    // channels, sorted by name as the format requires: A, B, G, R (all half)
    let mut ch = vec![];
    for n in ["A", "B", "G", "R"] {
        ch.extend_from_slice(n.as_bytes());
        ch.push(0);
        ch.extend_from_slice(&1i32.to_le_bytes());
        ch.extend_from_slice(&[0, 0, 0, 0]);
        ch.extend_from_slice(&1i32.to_le_bytes());
        ch.extend_from_slice(&1i32.to_le_bytes());
    }
    ch.push(0);
    attr(&mut out, "channels", "chlist", &ch);
    attr(&mut out, "compression", "compression", &[0]);
    let mut bx = vec![];
    for v in [0i32, 0, w as i32 - 1, h as i32 - 1] {
        bx.extend_from_slice(&v.to_le_bytes());
    }
    attr(&mut out, "dataWindow", "box2i", &bx);
    attr(&mut out, "displayWindow", "box2i", &bx);
    attr(&mut out, "lineOrder", "lineOrder", &[0]);
    attr(&mut out, "pixelAspectRatio", "float", &1.0f32.to_le_bytes());
    let mut v2 = vec![];
    v2.extend_from_slice(&0.0f32.to_le_bytes());
    v2.extend_from_slice(&0.0f32.to_le_bytes());
    attr(&mut out, "screenWindowCenter", "v2f", &v2);
    attr(&mut out, "screenWindowWidth", "float", &1.0f32.to_le_bytes());
    out.push(0);
    // the offset table, then one scanline per block
    let line_bytes = w * 4 * 2;
    let table_at = out.len();
    let first = table_at + h * 8;
    for y in 0..h {
        out.extend_from_slice(&((first + y * (8 + line_bytes)) as u64).to_le_bytes());
    }
    for y in 0..h {
        out.extend_from_slice(&(y as i32).to_le_bytes());
        out.extend_from_slice(&(line_bytes as i32).to_le_bytes());
        // channel by channel: A (3), B (2), G (1), R (0)
        for c in [3usize, 2, 1, 0] {
            for x in 0..w {
                let v = px.get(y * w + x).map_or(0, |p| p[c]);
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    std::fs::write(path, out)
}

/// Bake the chosen textures for `els` into `dir`, named T_<base>_<what>_<alpha>.<ext>. Returns
/// the files written.
pub fn write_textures(dir: &std::path::Path, base: &str, els: &[PPOut], s: &PP2) -> Result<Vec<String>, String> {
    if els.is_empty() {
        return Err("the tree has no branches or leaves to bake".into());
    }
    let (w, h) = dims(els.len());
    let max_depth = els.iter().map(|e| e.depth).max().unwrap_or(0);
    let clean: String = base.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect();
    let mut files = vec![];
    for (t, &(rgb, alpha)) in s.tex.iter().enumerate() {
        if rgb == 0 {
            continue;
        }
        if rgb == 1 {
            let a = alpha.min(ALPHA_HDR.len() - 1);
            let mut px = vec![[0u16; 4]; w * h];
            for (i, e) in els.iter().enumerate() {
                let av = match a {
                    // the 16-bit float whose bits are the index + 1024
                    0 => (e.parent as u32 + 1024).min(0x7bff) as u16,
                    1 => f32_to_f16(e.parent as f32 + 0.5),
                    2 => f32_to_f16(e.random),
                    _ => f32_to_f16(depth01(e.depth, max_depth)),
                };
                px[i] = [f32_to_f16(e.pivot.x), f32_to_f16(e.pivot.y), f32_to_f16(e.pivot.z), av];
            }
            let tag = ["ParentIndexInt", "ParentIndexFloat", "Random0-1", "HierarchyPosition0-1"][a];
            let name = format!("T_{}_PivotPos16bit_{}_UV2.exr", clean, tag);
            write_exr_half(&dir.join(&name), w, h, &px).map_err(|e| e.to_string())?;
            files.push(name);
        } else {
            let a = alpha.min(ALPHA_LDR.len() - 1);
            let mut px = vec![0u8; w * h * 4];
            let b = |v: f32| ((v.clamp(0.0, 1.0) * 255.0).round()) as u8;
            let extent = |cm: f32| (cm / 8.0).ceil().clamp(1.0, 256.0) / 256.0;
            for (i, e) in els.iter().enumerate() {
                let v = match rgb {
                    2 => e.x,
                    3 => e.y,
                    _ => e.z,
                };
                let av = match a {
                    0 => extent(e.ext.x),
                    1 => extent(e.ext.y),
                    2 => extent(e.ext.z),
                    3 => e.random,
                    _ => depth01(e.depth, max_depth),
                };
                px[i * 4..i * 4 + 4].copy_from_slice(&[b((v.x + 1.0) * 0.5), b((v.y + 1.0) * 0.5), b((v.z + 1.0) * 0.5), b(av)]);
            }
            let axis = ["X", "Y", "Z"][(rgb - 2).min(2)];
            let tag = ["XExtentDividedby2048", "YExtentDividedby2048", "ZExtentDividedby2048", "Random0-1", "HierarchyPosition0-1"][a];
            let name = format!("T_{}_{}Vector8bit_{}_UV2.tga", clean, axis, tag);
            image::save_buffer(dir.join(&name), &px, w as u32, h as u32, image::ColorType::Rgba8).map_err(|e| e.to_string())?;
            files.push(name);
        }
        let _ = t;
    }
    if files.is_empty() {
        return Err("both textures are set to Do not render".into());
    }
    Ok(files)
}

/// The PivotPainter2 panel (opened from the default panel, under Window properties).
pub fn pp2_rows(r: &mut Rows, app: &mut App) {
    let mut s = app.pp2.clone();
    let n = app.geo.as_ref().map_or(0, |g| g.pp_elems.len());
    let (w, h) = dims(n);
    if r.button("pp2.back", "<  Back to the default panel") {
        app.pp2_open = false;
    }
    if r.group("PivotPainter2") {
        r.note("Bakes each branch and leaf's pivot, direction, size and");
        r.note("parent into textures for Unreal Engine's Pivot Painter 2");
        r.note("wind materials, with a second UV set in the FBX.");
        r.info("Elements", &format!("{} (branches and leaves)", n));
        r.info("Texture size", &format!("{} x {}", w, h));
        r.info("UV channel", "2 (each part points at its pixel)");
    }
    for t in 0..2 {
        if r.group(&format!("Texture {}", t + 1)) {
            let rgb_names: Vec<String> = RGB.iter().map(|x| x.to_string()).collect();
            let before = s.tex[t].0;
            r.combo(&format!("pp2.rgb{}", t), "RGB", &rgb_names, &mut s.tex[t].0);
            let hdr = s.tex[t].0 == 1;
            if (before == 1) != hdr {
                s.tex[t].1 = 0;
            }
            if s.tex[t].0 != 0 {
                let list: Vec<String> = if hdr { ALPHA_HDR.iter().map(|x| x.to_string()).collect() } else { ALPHA_LDR.iter().map(|x| x.to_string()).collect() };
                s.tex[t].1 = s.tex[t].1.min(list.len() - 1);
                r.combo(&format!("pp2.a{}", t), "Alpha", &list, &mut s.tex[t].1);
                r.info("File", if hdr { "OpenEXR, 16-bit float (HDR)" } else { "Targa, 8-bit" });
            }
        }
    }
    if r.group("Export") {
        r.b("pp2.fbx", "Bake with every FBX export", &mut s.with_fbx);
        r.note("The textures are written beside the FBX, named after it.");
        app.pp2 = s.clone();
        if r.button("pp2.bake", "Bake textures only...") {
            app.pp2_bake_only();
        }
    }
    if r.group("In Unreal Engine") {
        r.note("Pivot Position (EXR): Compression HDR (RGBA16F).");
        r.note("Vectors (TGA): VectorDisplacementmap, sRGB off.");
        r.note("Both: Filter Nearest, Mip Gen Settings NoMipmaps.");
        r.note("Material: read them with UV channel 2 (index 1).");
    }
    app.pp2 = s;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_floats_and_layout() {
        assert_eq!(f32_to_f16(1.0), 0x3c00);
        assert_eq!(f32_to_f16(-2.0), 0xc000);
        assert_eq!(f32_to_f16(0.5), 0x3800);
        assert_eq!(f32_to_f16(2236.0), 0x685e);
        assert_eq!(dims(374), (20, 19));
        assert_eq!(dims(1), (1, 1));
        let u = uv(21, 20, 19);
        assert!((u.x - 0.075).abs() < 1e-6 && (u.y - 1.5 / 19.0).abs() < 1e-6);
    }
}
