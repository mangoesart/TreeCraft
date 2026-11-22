//! Texel density view (picture 75): every generator drawn with a checker picture instead of its
//! material, through its own UVs, so how much texture each part gets shows at a glance. Sizes
//! 256 to 4096 (one cell per 64 texels: 4 to 64 cells across), four patterns (black and white
//! or coloured, plain or with letters and numbers in the cells), or a picture of the person's own.
//! Chosen in the viewport's shading drop-down.
use crate::ui::*;
use macroquad::prelude::*;

pub const SIZES: [usize; 5] = [256, 512, 1024, 2048, 4096];
pub const PATTERNS: [&str; 5] = ["B&W", "B&W labels", "Colour", "Colour labels", "Custom"];

struct Texel {
    on: bool,
    size: usize,
    pattern: usize,
    custom: String,
    tex: Option<Texture2D>,
    key: (usize, usize, String),
}

thread_local! {
    static T: std::cell::RefCell<Texel> = std::cell::RefCell::new(Texel { on: false, size: 2, pattern: 1, custom: String::new(), tex: None, key: (99, 99, String::new()) });
}

pub fn on() -> bool {
    T.with(|t| t.borrow().on)
}

pub fn set_on(v: bool) {
    T.with(|t| t.borrow_mut().on = v);
}

pub fn set_choice(size: usize, pattern: usize) {
    T.with(|t| {
        let mut t = t.borrow_mut();
        t.size = size.min(SIZES.len() - 1);
        t.pattern = pattern.min(PATTERNS.len() - 1);
    });
}

/// The checker picture for the current choice (made when the choice changes).
pub fn texture() -> Option<Texture2D> {
    T.with(|t| {
        let mut t = t.borrow_mut();
        let key = (t.size, t.pattern, t.custom.clone());
        if t.key != key || t.tex.is_none() {
            let img = if t.pattern == 4 && !t.custom.is_empty() {
                crate::textures::load_image_file(&t.custom).ok()
            } else {
                None
            };
            let img = img.unwrap_or_else(|| checker_image(SIZES[t.size], t.pattern.min(3)));
            let tex = Texture2D::from_rgba8(img.w as u16, img.h as u16, &img.px);
            tex.set_filter(FilterMode::Linear);
            crate::textures::set_repeat(&tex);
            t.tex = Some(tex);
            t.key = key;
        }
        t.tex.clone()
    })
}

/// A 3 x 5 pixel font for the cell labels: each glyph's rows, top first, 3 bits each.
fn glyph(c: char) -> [u8; 5] {
    match c {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 3, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 6, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        _ => [0; 5],
    }
}

/// A checker picture for `size` texels (made at most 2048 pixels across, with as many cells as
/// the full size would have: one per 64 texels). Patterns (picture 76):
/// 0 grey checks (dark 0.24 / light 0.62, like Blender's default checker);
/// 1 the grey checks with a label in each cell (column letter, row number) whose colour turns
///   along the diagonals, so neighbouring cells always differ;
/// 2 coloured checks: a soft rainbow sweeping across the picture diagonally, light and dark cells
///   alternating, each cell outlined, a stronger line every four cells;
/// 3 the coloured checks with labels (white on dark cells, near-black on light ones).
pub fn checker_image(size: usize, pattern: usize) -> crate::model::ImageBuf {
    let cells = (size / 64).max(2);
    let px = size.min(2048);
    let cell = (px / cells).max(1);
    let px = cell * cells;
    let mut buf = vec![0u8; px * px * 4];
    let colour = pattern >= 2;
    let fill = |cx: usize, cy: usize| -> [f32; 3] {
        let dark = (cx + cy) % 2 == 0;
        if colour {
            let h = ((cx as f32 + cy as f32) / (2.0 * cells as f32)) * 0.85;
            let (r, g, b) = crate::prefs::hsv_to_rgb(h, 0.55, if dark { 0.66 } else { 0.9 });
            [r, g, b]
        } else if dark {
            [0.24, 0.24, 0.24]
        } else {
            [0.62, 0.62, 0.62]
        }
    };
    let line = (cell / 32).max(1);
    for y in 0..px {
        for x in 0..px {
            let (cx, cy) = (x / cell, y / cell);
            let mut c = fill(cx, cy);
            if colour {
                // each cell outlined; every fourth line stronger
                let (lx, ly) = (x % cell, y % cell);
                let edge = lx < line || ly < line;
                let strong = (lx < line * 2 && cx % 4 == 0) || (ly < line * 2 && cy % 4 == 0);
                let k = if strong { 0.55 } else if edge { 0.8 } else { 1.0 };
                for v in &mut c {
                    *v *= k;
                }
            }
            let i = (y * px + x) * 4;
            buf[i] = (c[0] * 255.0) as u8;
            buf[i + 1] = (c[1] * 255.0) as u8;
            buf[i + 2] = (c[2] * 255.0) as u8;
            buf[i + 3] = 255;
        }
    }
    if pattern == 1 || pattern == 3 {
        // labels: column letter(s) and row number, scaled to the cell
        for cy in 0..cells {
            for cx in 0..cells {
                let col = if cx < 26 { ((b'A' + cx as u8) as char).to_string() } else { format!("{}{}", (b'A' + (cx / 26 - 1) as u8) as char, (b'A' + (cx % 26) as u8) as char) };
                let label = format!("{}{}", col, cy + 1);
                // picture 78: the letters shrink until the label fits its cell (long labels such as
                // "AF32" at 2048 and 4096 were left out before)
                let scale = (cell / 14).min((cell.saturating_sub(4)) / (label.len() * 4)).max(1);
                let w = label.len() * 4 * scale;
                if w + 2 > cell {
                    continue;
                }
                let (ox, oy) = (cx * cell + (cell - w) / 2, cy * cell + (cell - 5 * scale) / 2);
                let dark = (cx + cy) % 2 == 0;
                let ink: [u8; 3] = if pattern == 1 {
                    // a colour turning along the diagonals (8 steps), bright on dark cells
                    let h = ((cx + cy * 3) % 8) as f32 / 8.0;
                    let (r, g, b) = crate::prefs::hsv_to_rgb(h, 0.85, if dark { 1.0 } else { 0.55 });
                    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
                } else if dark {
                    [250, 250, 250]
                } else {
                    [25, 25, 25]
                };
                for (k, ch) in label.chars().enumerate() {
                    let g = glyph(ch);
                    for (row, bits) in g.iter().enumerate() {
                        for bit in 0..3 {
                            if bits & (4 >> bit) != 0 {
                                for sy in 0..scale {
                                    for sx in 0..scale {
                                        // (mirrored in the picture: the trees' UVs run right to left across,
                                        // so the labels read the right way round on branches and leaves)
                                        let x = ox + w - 1 - ((k * 4 + bit) * scale + sx);
                                        let y = oy + row * scale + sy;
                                        if x < px && y < px {
                                            let i = (y * px + x) * 4;
                                            buf[i] = ink[0];
                                            buf[i + 1] = ink[1];
                                            buf[i + 2] = ink[2];
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    crate::model::ImageBuf { w: px, h: px, px: buf }
}
thread_local! {
    /// Small previews of the four patterns for the menu's tiles, and of the custom picture.
    static THUMBS: std::cell::RefCell<Vec<Texture2D>> = const { std::cell::RefCell::new(Vec::new()) };
    static CUSTOM_THUMB: std::cell::RefCell<Option<(String, Texture2D)>> = const { std::cell::RefCell::new(None) };
}

fn thumbs() -> Vec<Texture2D> {
    THUMBS.with(|t| {
        let mut t = t.borrow_mut();
        if t.is_empty() {
            for p in 0..4 {
                // 512 texels: 8 cells, enough to read the pattern (and a label or two) small
                let img = checker_image(512, p);
                let tex = Texture2D::from_rgba8(img.w as u16, img.h as u16, &img.px);
                tex.set_filter(FilterMode::Linear);
                t.push(tex);
            }
        }
        t.clone()
    })
}

/// The Texel density part of the shading drop-down at `r` (its height is MENU_H, picture 76): on
/// / off, the size as one rounded row of segments, the patterns as picture tiles (a preview of
/// each checker, and a Custom tile showing the person's own picture), and Choose... for it.
pub const MENU_H: f32 = 112.0;

pub fn menu(ui: &mut Ui, r: Rect) {
    let (mut on, mut size, mut pattern, custom) = T.with(|t| {
        let t = t.borrow();
        (t.on, t.size, t.pattern, t.custom.clone())
    });
    let m = ui.mouse();
    draw_line(r.x, r.y, r.x + r.w, r.y, 1.0, C_LINE);
    let cb = Rect::new(r.x, r.y + 8.0, 16.0, 16.0);
    ui.checkbox("vp.texel.on", cb, &mut on);
    ui.text("Texel density (checker)", cb.x + 22.0, cb.y + 1.0, 12.0, C_TEXT);
    ui.record("vp.texel", cb);
    // size: a segmented control
    let sr = Rect::new(r.x, r.y + 30.0, r.w, 22.0);
    draw_round_rect(sr, 5.0, Color::new(0.13, 0.13, 0.14, 1.0));
    let sw = sr.w / SIZES.len() as f32;
    for (k, s) in SIZES.iter().enumerate() {
        let br = Rect::new(sr.x + k as f32 * sw, sr.y, sw, sr.h);
        let active = size == k;
        let hot = br.contains(m) && !ui.blocked;
        if active {
            draw_round_rect(Rect::new(br.x + 1.0, br.y + 1.0, br.w - 2.0, br.h - 2.0), 4.0, Color::new(0.30, 0.47, 0.78, 1.0));
        } else if hot {
            draw_round_rect(Rect::new(br.x + 1.0, br.y + 1.0, br.w - 2.0, br.h - 2.0), 4.0, Color::new(1.0, 1.0, 1.0, 0.08));
        }
        if k > 0 && !active && size != k - 1 {
            draw_line(br.x, br.y + 5.0, br.x, br.y + br.h - 5.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.1));
        }
        let label = s.to_string();
        let tw = ui.text_w(&label, 12.0);
        ui.text(&label, br.x + (br.w - tw) * 0.5, br.y + 4.0, 12.0, if active { WHITE } else { C_TEXT_DIM });
        ui.record(&format!("vp.texel.size{}", k), br);
        if hot {
            ui.tooltip = Some(format!("{0} x {0} texels: {1} checker cells across one texture", s, s / 64));
            if ui.inp.l_pressed {
                ui.inp.l_pressed = false;
                size = k;
                on = true;
            }
        }
    }
    // patterns: picture tiles
    let th = thumbs();
    let gap = 6.0;
    let tile = ((r.w - gap * 4.0) / 5.0).min(44.0);
    let ty = r.y + 60.0;
    let names = ["Grey checks", "Grey checks with labels", "Coloured checks", "Coloured checks with labels", "Your own checker picture"];
    for k in 0..5 {
        let tr = Rect::new(r.x + k as f32 * (tile + gap), ty, tile, tile);
        let active = pattern == k;
        let hot = tr.contains(m) && !ui.blocked;
        checker(tr, 6.0);
        let pic = if k < 4 {
            th.get(k).cloned()
        } else if !custom.is_empty() {
            CUSTOM_THUMB.with(|c| {
                let mut c = c.borrow_mut();
                if c.as_ref().map_or(true, |(p, _)| *p != custom) {
                    *c = crate::textures::load_image_file(&custom).ok().map(|img| {
                        let t = Texture2D::from_rgba8(img.w as u16, img.h as u16, &img.px);
                        t.set_filter(FilterMode::Linear);
                        (custom.clone(), t)
                    });
                }
                c.as_ref().map(|(_, t)| t.clone())
            })
        } else {
            None
        };
        match pic {
            Some(t) => draw_texture_ex(&t, tr.x, tr.y, WHITE, DrawTextureParams { dest_size: Some(vec2(tr.w, tr.h)), ..Default::default() }),
            None => {
                // the Custom tile with nothing chosen: a picture sign with a plus
                draw_rectangle(tr.x, tr.y, tr.w, tr.h, Color::new(0.16, 0.16, 0.17, 1.0));
                let c = tr.center();
                let col = C_TEXT_DIM;
                draw_rectangle_lines(c.x - 11.0, c.y - 8.0, 22.0, 16.0, 1.5, col);
                draw_triangle(vec2(c.x - 8.0, c.y + 6.0), vec2(c.x - 2.0, c.y - 2.0), vec2(c.x + 3.0, c.y + 6.0), col);
                draw_circle(c.x + 5.0, c.y - 3.0, 2.0, col);
                draw_line(c.x + 10.0, c.y + 9.0, c.x + 16.0, c.y + 9.0, 1.5, col);
                draw_line(c.x + 13.0, c.y + 6.0, c.x + 13.0, c.y + 12.0, 1.5, col);
            }
        }
        draw_rectangle_lines(tr.x, tr.y, tr.w, tr.h, if active { 2.0 } else { 1.0 }, if active { C_ACCENT } else if hot { WHITE } else { C_LINE });
        ui.record(&format!("vp.texel.pat{}", k), tr);
        if hot {
            ui.tooltip = Some(if k == 4 && !custom.is_empty() { format!("{}: {}\nRight-click to pick another picture", names[k], std::path::Path::new(&custom).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()) } else { names[k].to_string() });
            if k == 4 && ui.inp.r_pressed {
                ui.inp.r_pressed = false;
                if let Some(f) = crate::props::pick_image() {
                    T.with(|t| t.borrow_mut().custom = f);
                    pattern = 4;
                    on = true;
                }
            }
            if ui.inp.l_pressed {
                ui.inp.l_pressed = false;
                pattern = k;
                on = true;
                if k == 4 && custom.is_empty() {
                    if let Some(f) = crate::props::pick_image() {
                        T.with(|t| t.borrow_mut().custom = f);
                    }
                }
            }
        }
    }
    // (picture 77: no line or Choose... under the tiles; the Custom tile picks the picture,
    // right-click on it picks another)
    T.with(|t| {
        let mut t = t.borrow_mut();
        t.on = on;
        t.size = size;
        t.pattern = pattern;
    });
}