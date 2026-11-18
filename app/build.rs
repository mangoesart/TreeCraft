//! Puts TreeCraft's icon and its name ("TreeCraft") into the Windows program file: the icon
//! drawn by src/icon_art.rs at seven sizes, written as a compiled resource file that the linker
//! takes directly.

#[path = "src/icon_art.rs"]
mod icon_art;

use std::path::PathBuf;

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16().chain(std::iter::once(0)).flat_map(|c| c.to_le_bytes()).collect()
}

fn pad4(v: &mut Vec<u8>) {
    while v.len() % 4 != 0 {
        v.push(0);
    }
}

/// One icon image as the resource holds it: a 32-bit bitmap (height doubled for the mask).
fn icon_bitmap(size: u32) -> Vec<u8> {
    let px = icon_art::render(size);
    let mut d = vec![];
    for v in [40u32, size, size * 2] {
        d.extend_from_slice(&v.to_le_bytes());
    }
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&32u16.to_le_bytes());
    for v in [0u32, 0, 0, 0, 0, 0] {
        d.extend_from_slice(&v.to_le_bytes());
    }
    let n = size as usize;
    // colours bottom-up as BGRA
    for y in (0..n).rev() {
        for x in 0..n {
            let i = (y * n + x) * 4;
            d.extend_from_slice(&[px[i + 2], px[i + 1], px[i], px[i + 3]]);
        }
    }
    // the transparency mask (unused with alpha, all zero), rows padded to 4 bytes
    let row = ((n + 31) / 32) * 4;
    d.extend(std::iter::repeat(0u8).take(row * n));
    d
}

/// One resource in a .res file.
fn resource(out: &mut Vec<u8>, kind: u16, id: u16, data: &[u8]) {
    let mut h = vec![];
    h.extend_from_slice(&(data.len() as u32).to_le_bytes());
    h.extend_from_slice(&32u32.to_le_bytes());
    h.extend_from_slice(&[0xFF, 0xFF]);
    h.extend_from_slice(&kind.to_le_bytes());
    h.extend_from_slice(&[0xFF, 0xFF]);
    h.extend_from_slice(&id.to_le_bytes());
    h.extend_from_slice(&0u32.to_le_bytes()); // data version
    h.extend_from_slice(&0x1030u16.to_le_bytes()); // moveable, pure, discardable
    h.extend_from_slice(&0x0409u16.to_le_bytes()); // English (US)
    h.extend_from_slice(&0u32.to_le_bytes());
    h.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&h);
    out.extend_from_slice(data);
    pad4(out);
}

/// A VS_VERSIONINFO block: key, value, children (each starting on 4 bytes).
fn vnode(key: &str, value: &[u8], value_len: u16, text: bool, children: &[Vec<u8>]) -> Vec<u8> {
    let mut v = vec![0, 0];
    v.extend_from_slice(&value_len.to_le_bytes());
    v.extend_from_slice(&(text as u16).to_le_bytes());
    v.extend_from_slice(&utf16z(key));
    pad4(&mut v);
    v.extend_from_slice(value);
    for c in children {
        pad4(&mut v);
        v.extend_from_slice(c);
    }
    let len = v.len() as u16;
    v[0..2].copy_from_slice(&len.to_le_bytes());
    v
}

fn version_info() -> Vec<u8> {
    let s = |k: &str, val: &str| {
        let w = utf16z(val);
        vnode(k, &w, (w.len() / 2) as u16, true, &[])
    };
    let strings = vec![
        s("CompanyName", "TreeCraft"),
        s("FileDescription", "TreeCraft"),
        s("FileVersion", "0.1.0"),
        s("InternalName", "TreeCraft"),
        s("OriginalFilename", "TreeCraft.exe"),
        s("ProductName", "TreeCraft"),
        s("ProductVersion", "0.1.0"),
    ];
    let table = vnode("040904B0", &[], 0, true, &strings);
    let sfi = vnode("StringFileInfo", &[], 0, true, &[table]);
    let tr: Vec<u8> = [0x0409u16, 0x04B0].iter().flat_map(|x| x.to_le_bytes()).collect();
    let var = vnode("Translation", &tr, 4, false, &[]);
    let vfi = vnode("VarFileInfo", &[], 0, true, &[var]);
    let mut fixed = vec![];
    for v in [0xFEEF04BDu32, 0x0001_0000, 0x0000_0001, 0, 0x0000_0001, 0, 0x3F, 0, 0x0004_0004, 1, 0, 0, 0] {
        fixed.extend_from_slice(&v.to_le_bytes());
    }
    vnode("VS_VERSION_INFO", &fixed, 52, false, &[sfi, vfi])
}

fn main() {
    println!("cargo:rerun-if-changed=src/icon_art.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") || std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let mut res = vec![];
    // the empty first entry every .res file starts with
    res.extend_from_slice(&[0, 0, 0, 0, 0x20, 0, 0, 0, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0, 0]);
    res.extend_from_slice(&[0u8; 16]);
    let mut group = vec![];
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    for (k, &sz) in sizes.iter().enumerate() {
        let bmp = icon_bitmap(sz);
        let id = k as u16 + 1;
        resource(&mut res, 3, id, &bmp); // RT_ICON
        group.push(if sz >= 256 { 0 } else { sz as u8 });
        group.push(if sz >= 256 { 0 } else { sz as u8 });
        group.extend_from_slice(&[0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&32u16.to_le_bytes());
        group.extend_from_slice(&(bmp.len() as u32).to_le_bytes());
        group.extend_from_slice(&id.to_le_bytes());
    }
    resource(&mut res, 14, 1, &group); // RT_GROUP_ICON
    resource(&mut res, 16, 1, &version_info()); // RT_VERSION
    let path = out.join("treecraft_icon.res");
    std::fs::write(&path, res).unwrap();
    println!("cargo:rustc-link-arg-bins={}", path.display());
}
