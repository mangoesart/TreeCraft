//! Global and Window properties in the Properties panel (picture 49), laid out like the
//! original's "Properties - Tree" (the Tree generator) and "Properties - <scene>" panels, with the
//! original's group names, property names and choices (strings at 0x141096e30-0x141098a00 and
//! 0x1410b9480-0x1410ba350). Rows with faint names are kept (and saved) but not used yet.
use crate::app::App;
use crate::model::Params;
use crate::props::Rows;
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub const GEOMETRY: [&str; 2] = ["Triangles", "Quads"];
pub const LIGHTMAP_RES: [&str; 7] = ["32", "64", "128", "256", "512", "1024", "2048"];
pub const LIGHTMAP_MULT: [&str; 3] = ["x1 (Same as target)", "x2", "x4"];
pub const LAYOUTS: [&str; 4] = ["One", "Two (side by side)", "Two (over under)", "Four"];
pub const BG_STYLES: [&str; 6] = ["Ambient light", "Solid color", "Full frame image", "Letterboxed image", "Stretched image", "Spherical panorama"];
pub const TRANSPARENCY: [&str; 4] = ["Best available (A2C if possible)", "Alpha test only", "Blending only", "Alpha test and blending"];
pub const ANISO: [&str; 5] = ["x1", "x2", "x4", "x8", "x16"];
pub const MANIP_MODES: [&str; 3] = ["Translate", "Rotate", "Scale"];
pub const COORD_SYSTEMS: [&str; 2] = ["Global", "Local"];
pub const CAPTURE_STYLES: [&str; 3] = ["Disabled", "Interactive", "Sequence render"];

fn names(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// Number rows kept in a name -> value map (missing = the original's default).
fn xf(r: &mut Rows, m: &mut BTreeMap<String, f64>, key: &str, label: &str, def: f32, min: f32, max: f32, dec: usize, used: bool) {
    let mut v = m.get(key).map(|x| *x as f32).unwrap_or(def);
    let before = v;
    if used {
        r.f(key, label, &mut v, min, max, dec);
    } else {
        r.f_faint(key, label, &mut v, min, max, dec);
    }
    if v != before {
        m.insert(key.to_string(), v as f64);
    }
}
fn xi(r: &mut Rows, m: &mut BTreeMap<String, f64>, key: &str, label: &str, def: i32, min: i32, max: i32, used: bool) {
    let mut v = m.get(key).map(|x| *x as i32).unwrap_or(def);
    let before = v;
    if used {
        r.i(key, label, &mut v, min, max);
    } else {
        r.i_faint(key, label, &mut v, min, max);
    }
    if v != before {
        m.insert(key.to_string(), v as f64);
    }
}
fn xb(r: &mut Rows, m: &mut BTreeMap<String, f64>, key: &str, label: &str, def: bool, used: bool) {
    let mut v = m.get(key).map(|x| *x > 0.5).unwrap_or(def);
    let before = v;
    if used {
        r.b(key, label, &mut v);
    } else {
        r.b_faint(key, label, &mut v);
    }
    if v != before {
        m.insert(key.to_string(), if v { 1.0 } else { 0.0 });
    }
}
fn xc(r: &mut Rows, m: &mut BTreeMap<String, f64>, key: &str, label: &str, items: &[&str], def: usize) -> usize {
    let mut v = m.get(key).map(|x| *x as usize).unwrap_or(def).min(items.len() - 1);
    let before = v;
    r.combo(key, label, &names(items), &mut v);
    if v != before {
        m.insert(key.to_string(), v as f64);
    }
    v
}

/// A value from a name -> value map, or its default.
pub fn get(m: &BTreeMap<String, f64>, key: &str, def: f32) -> f32 {
    m.get(key).map(|x| *x as f32).unwrap_or(def)
}

/// The Tree generator's panel ("Properties - Tree"): Generation (Shared: Size scalar; Geometry),
/// Shape (Radius, Sink, Roll), Subdivision, Dynamic LOD, SpeedTree SDK, Lightmap, Compute Flags,
/// Vertex Color, Proxy Collision; then TreeCraft's preview LOD and statistics.
pub fn tree_rows(r: &mut Rows, p: &mut Params, stats: (usize, usize, usize, usize)) {
    if r.group("Generation") {
        r.subheading("Shared");
        // the whole tree's size: every length, radius and leaf scaled together
        r.f("size_scalar", "Size scalar", &mut p.size_scalar, 0.01, 10.0, 2);
        p.size_scalar = p.size_scalar.max(0.001);
        r.subheading("Geometry");
        let m = &mut p.extra;
        xc(r, m, "Generation:Geometry:Branches/Caps", "Branches/Caps", &GEOMETRY, 0);
        xc(r, m, "Generation:Geometry:Leaves/Fronds/Meshes", "Leaves/Fronds/Meshes", &GEOMETRY, 0);
        xc(r, m, "Generation:Geometry:Subdivision", "Subdivision", &GEOMETRY, 0);
    }
    if r.group("Shape") {
        // the trunks' spread on the ground (fn 0x140141d60 reads Shape:Radius)
        r.f("tree_radius", "Radius", &mut p.tree_radius, 0.0, 60.0, 1);
        let m = &mut p.extra;
        // Sink lowers the whole tree into the ground; Roll turns it around its up axis (degrees)
        xf(r, m, "Shape:Sink", "Sink", 0.0, -20.0, 20.0, 2, true);
        xf(r, m, "Shape:Roll", "Roll", 0.0, -180.0, 180.0, 1, true);
    }
    if r.group("Subdivision") {
        let m = &mut p.extra;
        xi(r, m, "Subdivision:Level", "Level", 0, 0, 4, false);
        xf(r, m, "Subdivision:Relax", "Relax", 0.25, 0.0, 1.0, 2, false);
    }
    if r.group("Dynamic LOD") {
        let m = &mut p.extra;
        xb(r, m, "Dynamic LOD:Enabled", "Enabled", true, true);
        xi(r, m, "Dynamic LOD:Number", "Number", 3, 1, 8, true);
        // the leaf LOD the viewport previews, up to Number
        let n = get(m, "Dynamic LOD:Number", 3.0).max(1.0) as i32;
        let on = get(m, "Dynamic LOD:Enabled", 1.0) > 0.5;
        if on {
            r.i("lod_preview", "Preview LOD", &mut p.lod_preview, 0, (n - 1).max(0));
        }
        p.lod_preview = if on { p.lod_preview.clamp(0, (n - 1).max(0)) } else { 0 };
        let m = &mut p.extra;
        for nm in ["Branches", "Decorations", "Leaves", "Fronds"] {
            xf(r, m, &format!("Dynamic LOD:{}", nm), nm, 1.0, 0.0, 1.0, 2, false);
        }
        r.subheading("Leaf Settings");
        xf(r, m, "Dynamic LOD:Leaf Settings:Grow scale", "Grow scale", 2.0, 0.0, 10.0, 2, false);
        xf(r, m, "Dynamic LOD:Leaf Settings:Jumble", "Jumble", 0.1, 0.0, 1.0, 2, false);
        xi(r, m, "Dynamic LOD:Leaf Settings:Seed", "Seed", 15000, 0, 99999, false);
    }
    if r.group("SpeedTree SDK") {
        r.info("User data", "(for games using the SDK)");
    }
    if r.group("Lightmap") {
        let m = &mut p.extra;
        xc(r, m, "Lightmap:Target resolution", "Target resolution", &LIGHTMAP_RES, 2);
        xc(r, m, "Lightmap:Display multiplier", "Display multiplier", &LIGHTMAP_MULT, 0);
    }
    if r.group("Compute Flags") {
        let m = &mut p.extra;
        xb(r, m, "Compute Flags:Vertex colors", "Vertex colors", false, false);
    }
    if r.group("Vertex Color") {
        let m = &mut p.extra;
        xf(r, m, "Vertex Color:Red", "Red", 0.0, 0.0, 1.0, 2, false);
        xf(r, m, "Vertex Color:Green", "Green", 0.0, 0.0, 1.0, 2, false);
        xf(r, m, "Vertex Color:Blue", "Blue", 1.0, 0.0, 1.0, 2, false);
    }
    if r.group("Proxy Collision") {
        let m = &mut p.extra;
        xb(r, m, "Proxy Collision:Automatic", "Automatic", false, false);
        xf(r, m, "Proxy Collision:Scale", "Scale", 1.0, 0.0, 4.0, 2, false);
        let _ = r.button("cull", "Cull");
        let _ = r.button("restore", "Restore culled");
    }
    if r.group("Statistics") {
        r.info("Triangles", &stats.0.to_string());
        r.info("Vertices", &stats.1.to_string());
        r.info("Branches", &stats.2.to_string());
        r.info("Leaves", &stats.3.to_string());
    }
}

/// The scene's panel ("Properties - <scene>"): Viewport (Layout, Grid lines color, Grid spacing,
/// Lightmap view %, Background with Panorama Adjustments), General Rendering, Manipulator,
/// Screenshot Safe Frame, User Shader Constant 1, Capture Camera.
pub fn window_rows(r: &mut Rows, app: &mut App) {
    let mut m = std::mem::take(&mut app.winp);
    if r.group("Viewport") {
        xc(r, &mut m, "Viewport:Layout", "Layout", &LAYOUTS, 0);
        // the grid's line colour is the theme's (Preferences > Themes > 3D Viewport > Grid)
        let i = crate::prefs::VIEWPORT_COLORS.iter().position(|c| c.0 == "grid").unwrap_or(0);
        let mut c = [app.prefs.colors[i][0], app.prefs.colors[i][1], app.prefs.colors[i][2]];
        r.color("gridcol", "Grid lines color", &mut c);
        app.prefs.colors[i][0] = c[0];
        app.prefs.colors[i][1] = c[1];
        app.prefs.colors[i][2] = c[2];
        xf(r, &mut m, "Viewport:Grid spacing", "Grid spacing", 10.0, 1.0, 100.0, 1, true);
        xf(r, &mut m, "Viewport:Lightmap view %", "Lightmap view %", 0.3, 0.0, 1.0, 2, false);
        r.subheading("Background");
        let style = xc(r, &mut m, "Viewport:Background:Style", "Style", &BG_STYLES, 0);
        let mut bc = [get(&m, "Viewport:Background:Color:R", 0.40), get(&m, "Viewport:Background:Color:G", 0.47), get(&m, "Viewport:Background:Color:B", 0.56)];
        let before = bc;
        r.color("bgcol", "Color", &mut bc);
        if bc != before {
            for (k, ch) in ["R", "G", "B"].iter().enumerate() {
                m.insert(format!("Viewport:Background:Color:{}", ch), bc[k] as f64);
            }
        }
        let shown = if app.win_image_path.is_empty() {
            "(none)".to_string()
        } else {
            std::path::Path::new(&app.win_image_path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()
        };
        r.info("Image", &shown);
        if r.button("image", "Choose image...") {
            if let Some(f) = crate::prefs::file_dialog().add_filter("Pictures", &["png", "jpg", "jpeg", "tga", "bmp"]).pick_file() {
                app.set_win_image(&f.to_string_lossy());
                if style < 2 {
                    m.insert("Viewport:Background:Style".into(), 2.0);
                }
            }
        }
        if !app.win_image_path.is_empty() && r.button("noimage", "Remove image") {
            app.set_win_image("");
        }
        r.subheading("Panorama Adjustments");
        xf(r, &mut m, "Viewport:Background:Panorama Adjustments:Brightness", "Brightness", 0.0, -1.0, 1.0, 2, true);
        xf(r, &mut m, "Viewport:Background:Panorama Adjustments:Contrast", "Contrast", 0.0, -1.0, 1.0, 2, true);
        xf(r, &mut m, "Viewport:Background:Panorama Adjustments:Rotation", "Rotation", 0.0, -180.0, 180.0, 0, true);
    }
    if r.group("General Rendering") {
        // Transparency: alpha testing on unless "Blending only"
        let t = xc(r, &mut m, "General Rendering:Transparency", "Transparency", &TRANSPARENCY, 0);
        app.prefs.alpha_test = t != 2;
        xc(r, &mut m, "General Rendering:Anisotropic filtering", "Anisotropic filtering", &ANISO, 4);
        xf(r, &mut m, "General Rendering:3D icon scale", "3D icon scale", 2.8, 0.1, 10.0, 1, false);
    }
    if r.group("Manipulator") {
        xc(r, &mut m, "Manipulator:Mode", "Mode", &MANIP_MODES, 1);
        xc(r, &mut m, "Manipulator:Coordinate system", "Coordinate system", &COORD_SYSTEMS, 0);
        r.f("manip", "Size scale", &mut app.prefs.manip_size, 0.25, 4.0, 2);
    }
    if r.group("Screenshot Safe Frame") {
        // a frame in the viewport showing what a picture of that size would hold
        xb(r, &mut m, "Screenshot Safe Frame:Show", "Show", false, true);
        xi(r, &mut m, "Screenshot Safe Frame:Width", "Width", 1024, 16, 8192, true);
        xi(r, &mut m, "Screenshot Safe Frame:Height", "Height", 1024, 16, 8192, true);
    }
    if r.group("User Shader Constant 1") {
        for (k, def) in [("X", 1.0), ("Y", 0.0), ("Z", 0.0), ("W", 0.0)] {
            xf(r, &mut m, &format!("User Shader Constant 1:{}", k), k, def, -10.0, 10.0, 2, false);
        }
    }
    if r.group("Capture Camera") {
        xc(r, &mut m, "Capture Camera:Style", "Style", &CAPTURE_STYLES, 0);
        for nm in ["Left X", "Left Y", "Middle X", "Middle Y", "Light azimuth", "Light pitch"] {
            xf(r, &mut m, &format!("Capture Camera:{}", nm), nm, 0.0, -1000.0, 1000.0, 1, false);
        }
    }
    app.winp = m;
}

impl App {
    /// Window properties > Background > Image: read the picture (empty: none).
    pub fn set_win_image(&mut self, path: &str) {
        self.win_image_path = path.to_string();
        self.win_image = None;
        if path.is_empty() {
            return;
        }
        match crate::textures::load_image_file(path) {
            Ok(img) => {
                let t = Texture2D::from_rgba8(img.w as u16, img.h as u16, &img.px);
                t.set_filter(FilterMode::Linear);
                self.win_image = Some(t);
            }
            Err(e) => self.set_status(&format!("Couldn't read the background image: {}", e)),
        }
    }

    /// One window property, or its default.
    pub fn wp(&self, key: &str, def: f32) -> f32 {
        get(&self.winp, key, def)
    }

    /// The Background's Solid color and image styles (picture 49). True when it drew; false
    /// leaves the theme's sky or gradient (Ambient light, or no image chosen).
    pub fn draw_window_background(&self, r: Rect) -> bool {
        let style = self.wp("Viewport:Background:Style", 0.0) as usize;
        if style == 1 {
            let c = Color::new(self.wp("Viewport:Background:Color:R", 0.40), self.wp("Viewport:Background:Color:G", 0.47), self.wp("Viewport:Background:Color:B", 0.56), 1.0);
            draw_rectangle(r.x, r.y, r.w, r.h, c);
            return true;
        }
        let Some(t) = &self.win_image else { return false };
        if style < 2 {
            return false;
        }
        let bc = Color::new(self.wp("Viewport:Background:Color:R", 0.40), self.wp("Viewport:Background:Color:G", 0.47), self.wp("Viewport:Background:Color:B", 0.56), 1.0);
        draw_rectangle(r.x, r.y, r.w, r.h, bc);
        let (tw, th) = (t.width().max(1.0), t.height().max(1.0));
        let bright = self.wp("Viewport:Background:Panorama Adjustments:Brightness", 0.0);
        let contrast = self.wp("Viewport:Background:Panorama Adjustments:Contrast", 0.0);
        let k = (1.0 + bright.min(0.0)).clamp(0.0, 1.0);
        let tint = Color::new(k, k, k, 1.0);
        match style {
            // Full frame: covers the viewport, cropped to keep its shape
            2 | 3 => {
                let s = if style == 2 { (r.w / tw).max(r.h / th) } else { (r.w / tw).min(r.h / th) };
                let (w, h) = (tw * s, th * s);
                let (x, y) = (r.x + (r.w - w) * 0.5, r.y + (r.h - h) * 0.5);
                // (cut to the viewport by drawing only the part inside it)
                let sx = ((r.x - x) / s).max(0.0);
                let sy = ((r.y - y) / s).max(0.0);
                let sw = (r.w / s).min(tw - sx);
                let sh = (r.h / s).min(th - sy);
                draw_texture_ex(t, x.max(r.x), y.max(r.y), tint, DrawTextureParams { dest_size: Some(vec2(sw * s, sh * s)), source: Some(Rect::new(sx, sy, sw, sh)), ..Default::default() });
            }
            4 => {
                draw_texture_ex(t, r.x, r.y, tint, DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
            }
            // Spherical panorama: turns with the camera (and Rotation), tilts with its pitch
            _ => {
                let fov = self.cam.fov.to_radians();
                let aspect = r.w / r.h.max(1.0);
                let span_u = (fov * aspect / std::f32::consts::TAU).min(1.0);
                let span_v = (fov / std::f32::consts::PI).min(1.0);
                let rot = self.wp("Viewport:Background:Panorama Adjustments:Rotation", 0.0).to_radians();
                let u0 = (-(self.cam.yaw + rot) / std::f32::consts::TAU).rem_euclid(1.0) - span_u * 0.5;
                let v0 = (0.5 - self.cam.pitch / std::f32::consts::PI - span_v * 0.5).clamp(0.0, 1.0 - span_v);
                let u0 = u0.rem_euclid(1.0);
                // two pieces when the view wraps around the picture's edge
                let first = (1.0 - u0).min(span_u);
                let w1 = r.w * first / span_u;
                draw_texture_ex(t, r.x, r.y, tint, DrawTextureParams { dest_size: Some(vec2(w1, r.h)), source: Some(Rect::new(u0 * tw, v0 * th, first * tw, span_v * th)), ..Default::default() });
                if first < span_u {
                    draw_texture_ex(t, r.x + w1, r.y, tint, DrawTextureParams { dest_size: Some(vec2(r.w - w1, r.h)), source: Some(Rect::new(0.0, v0 * th, (span_u - first) * tw, span_v * th)), ..Default::default() });
                }
            }
        }
        if bright > 0.0 {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(1.0, 1.0, 1.0, bright * 0.6));
        }
        if contrast < 0.0 {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.5, 0.5, 0.5, -contrast * 0.7));
        } else if contrast > 0.0 {
            // deepen it a little: darker shades darker (an approximation)
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, contrast * 0.25));
        }
        true
    }

    /// Screenshot Safe Frame (picture 49): the part of the viewport a Width x Height picture holds,
    /// with the rest dimmed.
    pub fn draw_safe_frame(&self, r: Rect) {
        if self.wp("Screenshot Safe Frame:Show", 0.0) < 0.5 {
            return;
        }
        let (w, h) = (self.wp("Screenshot Safe Frame:Width", 1024.0).max(1.0), self.wp("Screenshot Safe Frame:Height", 1024.0).max(1.0));
        let s = (r.w / w).min(r.h / h) * 0.96;
        let (fw, fh) = (w * s, h * s);
        let f = Rect::new(r.x + (r.w - fw) * 0.5, r.y + (r.h - fh) * 0.5, fw, fh);
        let dim = Color::new(0.0, 0.0, 0.0, 0.35);
        draw_rectangle(r.x, r.y, r.w, f.y - r.y, dim);
        draw_rectangle(r.x, f.y + f.h, r.w, r.y + r.h - f.y - f.h, dim);
        draw_rectangle(r.x, f.y, f.x - r.x, f.h, dim);
        draw_rectangle(f.x + f.w, f.y, r.x + r.w - f.x - f.w, f.h, dim);
        draw_rectangle_lines(f.x, f.y, f.w, f.h, 1.5, Color::new(1.0, 0.85, 0.3, 0.9));
    }
}
