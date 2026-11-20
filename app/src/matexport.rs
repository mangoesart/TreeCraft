//! File > Export material (F10; picture 53): the original's Render Images dialog
//! (CRenderModeImageDialog, strings at 0x141089438-0x141089b40). Options (Keep aspect ratio,
//! Width, Height, Override bkgd, Shadows, Streak colors, Ground, Clip below ground), Sequence
//! (Use timeline, Include wind, Length (s), Frame rate (fps), Frames) and a list of Images, each
//! with a Suffix, an RGB source and an Alpha source (the render modes, as the install's
//! render_modes folder names them) and a background colour / alpha. OK asks for a folder and
//! renders every image from the current view into it as PNG files: <tree><_suffix>.png (and
//! _0001... for a sequence).
use crate::app::App;
use crate::ui::*;
use macroquad::prelude::*;

/// Render modes an image can take its colour or alpha from: (name, the shader's mode number).
pub const SOURCES: [(&str, f32); 33] = [
    ("Standard", 0.0),
    ("Scribed", 1.0),
    ("AO", 2.0),
    ("Albedo Check", 3.0),
    ("Subsurface Check", 4.0),
    ("Lightmap", 5.0),
    ("Lighting / Untextured", 10.0),
    ("Lighting / Diffuse", 11.0),
    ("Lighting / Specular", 12.0),
    ("Lighting / Ambient", 13.0),
    ("Lighting / Subsurface", 14.0),
    ("Lighting / Shadow", 15.0),
    ("Material / Color", 20.0),
    ("Material / Opacity", 21.0),
    ("Material / Gloss", 22.0),
    ("Material / Normal", 23.0),
    ("Material / Specular", 24.0),
    ("Material / Metallic", 25.0),
    ("Material / Subsurface", 26.0),
    ("Material / Subsurface Amount", 27.0),
    ("Material / Custom", 28.0),
    ("Vertex Colors / Color", 30.0),
    ("Vertex Colors / Red", 31.0),
    ("Vertex Colors / Green", 32.0),
    ("Vertex Colors / Blue", 33.0),
    ("Other / Phong", 40.0),
    ("Other / Blinn-Phong", 41.0),
    ("Other / Oren-Nayar", 42.0),
    ("Other / Lambert", 43.0),
    ("Other / Flat", 44.0),
    ("Other / Depth", 46.0),
    ("Other / Texcoord", 48.0),
    ("Other / Texcoord Seams", 49.0),
];

fn src(name: &str) -> usize {
    SOURCES.iter().position(|s| s.0 == name).unwrap_or(0)
}

#[derive(Clone, Debug)]
pub struct ExImage {
    pub suffix: String,
    pub rgb: usize,
    /// 0 None, k = SOURCES[k - 1].
    pub alpha: usize,
    pub bg_rgb: [f32; 3],
    pub bg_alpha: f32,
}

#[derive(Clone, Debug)]
pub struct MatExport {
    pub keep_aspect: bool,
    pub width: i32,
    pub height: i32,
    pub override_bg: bool,
    pub shadows: bool,
    pub streak: bool,
    pub ground: bool,
    pub clip_below: bool,
    pub sequence: bool,
    pub use_timeline: bool,
    pub include_wind: bool,
    pub length: f32,
    pub fps: f32,
    pub frames: i32,
    pub images: Vec<ExImage>,
    pub scroll: f32,
    /// The suffix being typed: (image, text, all selected).
    pub editing: Option<(usize, String, bool)>,
    /// The colour swatch opened for editing: (image, 0 RGB / 1 alpha).
    pub swatch: Option<(usize, u8)>,
    /// Where the window is (picture 54: moved by its title, resized from any corner), what's
    /// being dragged (1 title, 2-5 corners) and where the drag started.
    pub win: Option<Rect>,
    pub win_drag: u8,
    pub win_grab: (Vec2, Rect),
    /// Render from (picture 55): a custom camera's name, or empty for the current view.
    pub camera: String,
}

/// How an export ended (picture 55): shown in a message until the person closes it.
pub struct ExportResult {
    pub ok: bool,
    pub title: String,
    pub lines: Vec<String>,
    pub dir: std::path::PathBuf,
}

impl MatExport {
    /// The original's default list (picture 53).
    pub fn new() -> Self {
        let im = |suffix: &str, rgb: &str, bg: [f32; 3]| ExImage { suffix: suffix.into(), rgb: src(rgb), alpha: 0, bg_rgb: bg, bg_alpha: 0.0 };
        MatExport {
            keep_aspect: false,
            width: 4096,
            height: 4096,
            override_bg: true,
            shadows: false,
            streak: true,
            ground: false,
            clip_below: false,
            sequence: false,
            use_timeline: false,
            include_wind: true,
            length: 10.0,
            fps: 30.0,
            frames: 300,
            images: vec![
                im("", "Material / Color", [0.16, 0.25, 0.12]),
                im("Opacity", "Material / Opacity", [0.0, 0.0, 0.0]),
                im("Normal", "Material / Normal", [0.5, 0.5, 1.0]),
                im("Gloss", "Material / Gloss", [0.8, 0.8, 0.8]),
                im("AO", "AO", [1.0, 1.0, 1.0]),
                im("Subsurface", "Material / Subsurface", [0.8, 0.88, 0.38]),
                im("SubsurfaceAmount", "Material / Subsurface Amount", [1.0, 1.0, 1.0]),
            ],
            scroll: 0.0,
            editing: None,
            swatch: None,
            win: None,
            win_drag: 0,
            win_grab: (Vec2::ZERO, Rect::new(0.0, 0.0, 0.0, 0.0)),
            camera: String::new(),
        }
    }

    /// The settings as preset text (picture 54): one `key=value` per line, images by their
    /// sources' names.
    pub fn to_preset(&self) -> String {
        let mut s = String::from("# TreeCraft export material preset 1\n");
        for (k, v) in [
            ("keep_aspect", self.keep_aspect.to_string()),
            ("width", self.width.to_string()),
            ("height", self.height.to_string()),
            ("override_bg", self.override_bg.to_string()),
            ("shadows", self.shadows.to_string()),
            ("streak", self.streak.to_string()),
            ("ground", self.ground.to_string()),
            ("clip_below", self.clip_below.to_string()),
            ("sequence", self.sequence.to_string()),
            ("use_timeline", self.use_timeline.to_string()),
            ("include_wind", self.include_wind.to_string()),
            ("length", self.length.to_string()),
            ("fps", self.fps.to_string()),
            ("frames", self.frames.to_string()),
            ("camera", self.camera.replace('\n', " ")),
        ] {
            s.push_str(&format!("{}={}\n", k, v));
        }
        for im in &self.images {
            let alpha = if im.alpha == 0 { "None" } else { SOURCES[im.alpha - 1].0 };
            s.push_str(&format!("image={}|{}|{}|{},{},{}|{}\n", im.suffix.replace('|', " "), SOURCES[im.rgb].0, alpha, im.bg_rgb[0], im.bg_rgb[1], im.bg_rgb[2], im.bg_alpha));
        }
        s
    }

    /// Settings from preset text; None when it isn't one.
    pub fn from_preset(text: &str, keep: &MatExport) -> Option<MatExport> {
        let mut lines = text.lines();
        if !lines.next()?.trim().starts_with("# TreeCraft export material preset") {
            return None;
        }
        let mut d = keep.clone();
        d.images.clear();
        for line in lines {
            let Some((k, v)) = line.trim_end_matches('\r').split_once('=') else { continue };
            let b = v == "true";
            match k {
                "keep_aspect" => d.keep_aspect = b,
                "width" => d.width = v.parse().unwrap_or(d.width),
                "height" => d.height = v.parse().unwrap_or(d.height),
                "override_bg" => d.override_bg = b,
                "shadows" => d.shadows = b,
                "streak" => d.streak = b,
                "ground" => d.ground = b,
                "clip_below" => d.clip_below = b,
                "sequence" => d.sequence = b,
                "use_timeline" => d.use_timeline = b,
                "include_wind" => d.include_wind = b,
                "length" => d.length = v.parse().unwrap_or(d.length),
                "fps" => d.fps = v.parse().unwrap_or(d.fps),
                "frames" => d.frames = v.parse().unwrap_or(d.frames),
                "camera" => d.camera = v.to_string(),
                "image" => {
                    let p: Vec<&str> = v.split('|').collect();
                    if p.len() >= 5 {
                        let c: Vec<f32> = p[3].split(',').filter_map(|x| x.parse().ok()).collect();
                        d.images.push(ExImage {
                            suffix: p[0].to_string(),
                            rgb: src(p[1]),
                            alpha: if p[2] == "None" { 0 } else { src(p[2]) + 1 },
                            bg_rgb: [c.first().copied().unwrap_or(0.0), c.get(1).copied().unwrap_or(0.0), c.get(2).copied().unwrap_or(0.0)],
                            bg_alpha: p[4].parse().unwrap_or(0.0),
                        });
                    }
                }
                _ => {}
            }
        }
        d.editing = None;
        d.swatch = None;
        d.scroll = 0.0;
        Some(d)
    }
}

/// Where export presets are kept: %APPDATA%\TreeCraft\export_presets.
pub fn presets_dir() -> std::path::PathBuf {
    let base = std::env::var_os("APPDATA").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("TreeCraft").join("export_presets")
}

/// The saved presets' names (files *.tcexport in presets_dir), sorted.
pub fn preset_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(presets_dir())
        .map(|it| it.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().map_or(false, |x| x == "tcexport")).filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string())).collect())
        .unwrap_or_default();
    v.sort_by_key(|s| s.to_lowercase());
    v
}

/// Add Image's menu (picture 54): the render modes as the original lists them, grouped like
/// the install's render_modes folder (Lighting, Material, Vertex Colors, Other).
fn add_image_menu() -> Vec<MenuItem> {
    let mut top: Vec<MenuItem> = vec![];
    let mut groups: Vec<(String, Vec<MenuItem>)> = vec![];
    for (name, _) in SOURCES.iter() {
        match name.split_once(" / ") {
            Some((g, item)) => {
                if let Some(e) = groups.iter_mut().find(|e| e.0 == g) {
                    e.1.push(MenuItem::leaf(item));
                } else {
                    groups.push((g.to_string(), vec![MenuItem::leaf(item)]));
                }
            }
            None => top.push(MenuItem::leaf(name)),
        }
    }
    for (g, items) in groups {
        top.push(MenuItem::sub(&g, items));
    }
    top
}

/// The source a path in add_image_menu stands for.
fn add_image_source(path: &[usize]) -> Option<usize> {
    let tops: Vec<&str> = SOURCES.iter().filter(|s| !s.0.contains(" / ")).map(|s| s.0).collect();
    let first = *path.first()?;
    if first < tops.len() {
        return Some(src(tops[first]));
    }
    let mut groups: Vec<&str> = vec![];
    for s in SOURCES.iter() {
        if let Some((g, _)) = s.0.split_once(" / ") {
            if !groups.contains(&g) {
                groups.push(g);
            }
        }
    }
    let g = groups.get(first - tops.len())?;
    let items: Vec<usize> = SOURCES.iter().enumerate().filter(|(_, s)| s.0.starts_with(&format!("{} / ", g))).map(|(i, _)| i).collect();
    items.get(*path.get(1)?).copied()
}

/// One picture still to render: (image, frame number or None, file).
pub struct ExportJob {
    pub dir: std::path::PathBuf,
    pub settings: MatExport,
    pub todo: Vec<(usize, Option<usize>, std::path::PathBuf)>,
    pub done: usize,
    pub total: usize,
    pub failed: Vec<String>,
    /// Frames to wait before the first picture (the tree and its pictures must be ready).
    pub wait: u32,
    /// The picture being made (for the progress window).
    pub current: String,
}

fn names() -> Vec<String> {
    SOURCES.iter().map(|s| s.0.to_string()).collect()
}

impl App {
    pub fn open_material_export(&mut self) {
        if self.mat_export.is_none() {
            self.mat_export = Some(MatExport::new());
        }
        self.mat_export_open = true;
    }

    /// True while the window, its progress or its result message takes the mouse and keys.
    pub fn material_export_takes_input(&self) -> bool {
        self.mat_export_open || self.mat_export_job.is_some() || self.mat_export_result.is_some()
    }

    /// The window (drawn over everything); while exporting, the progress; after, the result.
    pub fn draw_material_export(&mut self, ui: &mut Ui) {
        if self.mat_export_job.is_some() || self.mat_export_result.is_some() {
            self.draw_export_progress(ui);
            return;
        }
        if !self.mat_export_open {
            return;
        }
        let Some(mut d) = self.mat_export.take() else { return };
        // a modal window: the app blocks everything else; this window takes the input
        ui.blocked = false;
        let (sw, sh) = (ui.sw, ui.sh);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.35));
        // movable by its title bar and resizable from any of its four corners (picture 54)
        let w0 = (sw - 80.0).min(780.0);
        let h0 = (sh - 60.0).min(720.0);
        let mut r = d.win.unwrap_or(Rect::new((sw - w0) * 0.5, (sh - h0) * 0.5, w0, h0));
        {
            let m = ui.mouse();
            let (min_w, min_h) = (560.0f32.min(sw - 8.0), 470.0f32.min(sh - 8.0));
            let cs = 12.0;
            let corners = [Rect::new(r.x - 4.0, r.y - 4.0, cs, cs), Rect::new(r.x + r.w - cs + 4.0, r.y - 4.0, cs, cs), Rect::new(r.x - 4.0, r.y + r.h - cs + 4.0, cs, cs), Rect::new(r.x + r.w - cs + 4.0, r.y + r.h - cs + 4.0, cs, cs)];
            let title = Rect::new(r.x, r.y, r.w - 36.0, 28.0);
            if d.win_drag == 0 && ui.inp.l_pressed && ui.popup.is_none() {
                if let Some(k) = corners.iter().position(|c| c.contains(m)) {
                    d.win_drag = 2 + k as u8;
                } else if title.contains(m) {
                    d.win_drag = 1;
                }
                if d.win_drag != 0 {
                    d.win_grab = (m, r);
                    ui.inp.l_pressed = false;
                }
            }
            if d.win_drag != 0 {
                if ui.inp.l_down {
                    let (start, r0) = d.win_grab;
                    let dd = m - start;
                    r = match d.win_drag {
                        1 => Rect::new(r0.x + dd.x, r0.y + dd.y, r0.w, r0.h),
                        k => {
                            let (left, top) = (k == 2 || k == 4, k == 2 || k == 3);
                            let (mut x0, mut y0, mut x1, mut y1) = (r0.x, r0.y, r0.x + r0.w, r0.y + r0.h);
                            if left { x0 = (x0 + dd.x).min(x1 - min_w); } else { x1 = (x1 + dd.x).max(x0 + min_w); }
                            if top { y0 = (y0 + dd.y).min(y1 - min_h); } else { y1 = (y1 + dd.y).max(y0 + min_h); }
                            Rect::new(x0, y0, x1 - x0, y1 - y0)
                        }
                    };
                } else {
                    d.win_drag = 0;
                }
            }
            // keep the title bar on screen
            r.x = r.x.clamp(-r.w + 80.0, sw - 80.0);
            r.y = r.y.clamp(0.0, sh - 30.0);
            d.win = Some(r);
            if title.contains(m) && d.win_drag == 0 && ui.popup.is_none() {
                ui.tooltip = Some("Drag to move; drag a corner to resize".into());
            }
            // the corner grips, the hovered or dragged one lit
            for (k, c) in corners.iter().enumerate() {
                let hot = c.contains(m) || d.win_drag == 2 + k as u8;
                if hot {
                    draw_rectangle(c.x, c.y, c.w, c.h, C_ACCENT);
                }
            }
        }
        draw_rectangle(r.x + 4.0, r.y + 5.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(r, 6.0, C_PANEL);
        draw_round_rect(Rect::new(r.x, r.y, r.w, 28.0), 6.0, C_PANEL2);
        ui.text("Export Material", r.x + 12.0, r.y + 6.0, 13.5, C_TEXT);
        ui.record("matexp.window", r);
        let mut close = false;
        if ui.button("matexp.x", Rect::new(r.x + r.w - 30.0, r.y + 3.0, 24.0, 22.0), "x") {
            close = true;
        }
        let lw = 130.0;
        // Options (left) and Sequence (right)
        let top = r.y + 38.0;
        let colw = (r.w - 36.0) * 0.5;
        let ox = r.x + 12.0;
        let sx = ox + colw + 12.0;
        let boxh = 226.0;
        draw_round_rect(Rect::new(ox, top, colw, boxh), 4.0, C_BG);
        draw_round_rect(Rect::new(sx, top, colw, boxh), 4.0, C_BG);
        ui.text("Options", ox + 8.0, top + 5.0, 12.5, C_TEXT);
        let row = |k: usize, x: f32| Rect::new(x + lw, top + 28.0 + k as f32 * 24.0, colw - lw - 10.0, 20.0);
        let label = |ui: &mut Ui, k: usize, x: f32, t: &str, on: bool| {
            let tw = ui.text_w(t, 12.0);
            ui.text(t, x + lw - tw - 8.0, top + 31.0 + k as f32 * 24.0, 12.0, if on { C_TEXT_DIM } else { C_TEXT_FAINT });
        };
        let aspect = (self.viewport.w / self.viewport.h.max(1.0)).clamp(0.1, 10.0);
        label(ui, 0, ox, "Keep aspect ratio", true);
        ui.checkbox("matexp.aspect", row(0, ox), &mut d.keep_aspect);
        label(ui, 1, ox, "Width", true);
        let w0 = d.width;
        ui.islider("matexp.w", row(1, ox), &mut d.width, 16, 8192);
        label(ui, 2, ox, "Height", true);
        let h0 = d.height;
        ui.islider("matexp.h", row(2, ox), &mut d.height, 16, 8192);
        d.width = d.width.clamp(16, 8192);
        d.height = d.height.clamp(16, 8192);
        if d.keep_aspect {
            // the view's shape: height follows width, or width follows height
            if d.width != w0 {
                d.height = ((d.width as f32 / aspect).round() as i32).clamp(16, 8192);
            } else if d.height != h0 {
                d.width = ((d.height as f32 * aspect).round() as i32).clamp(16, 8192);
            }
        }
        label(ui, 3, ox, "Override bkgd", true);
        ui.checkbox("matexp.bg", row(3, ox), &mut d.override_bg);
        label(ui, 4, ox, "Shadows", false);
        ui.checkbox("matexp.shadows", row(4, ox), &mut d.shadows);
        label(ui, 5, ox, "Streak colors", true);
        ui.checkbox("matexp.streak", row(5, ox), &mut d.streak);
        label(ui, 6, ox, "Ground", true);
        ui.checkbox("matexp.ground", row(6, ox), &mut d.ground);
        // (shown with Options' last row: Clip below ground)
        let cb = Rect::new(ox + 8.0, top + boxh - 2.0, 0.0, 0.0);
        let _ = cb;
        // Sequence
        let mut seq = d.sequence;
        ui.checkbox("matexp.seq", Rect::new(sx + 8.0, top + 4.0, 18.0, 18.0), &mut seq);
        d.sequence = seq;
        ui.text("Sequence", sx + 32.0, top + 5.0, 12.5, C_TEXT);
        let on = d.sequence;
        label(ui, 0, sx, "Use timeline", false);
        ui.checkbox("matexp.timeline", row(0, sx), &mut d.use_timeline);
        label(ui, 1, sx, "Include wind", on);
        ui.checkbox("matexp.wind", row(1, sx), &mut d.include_wind);
        label(ui, 2, sx, "Length (s)", on);
        let (l0, f0) = (d.length, d.fps);
        ui.slider("matexp.len", row(2, sx), &mut d.length, 0.1, 120.0, 1);
        label(ui, 3, sx, "Frame rate (fps)", on);
        ui.slider("matexp.fps", row(3, sx), &mut d.fps, 1.0, 120.0, 0);
        d.length = d.length.max(0.04);
        d.fps = d.fps.clamp(1.0, 240.0);
        if d.length != l0 || d.fps != f0 {
            d.frames = (d.length * d.fps).round().max(1.0) as i32;
        }
        label(ui, 4, sx, "Frames", on);
        let fr0 = d.frames;
        ui.islider("matexp.frames", row(4, sx), &mut d.frames, 1, 3600);
        d.frames = d.frames.clamp(1, 100000);
        if d.frames != fr0 {
            d.length = d.frames as f32 / d.fps;
        }
        // Render from (picture 55): the current view or one of the scene's cameras
        let mut cams = vec!["Current view".to_string()];
        cams.extend(self.cameras.iter().map(|c| c.name.clone()));
        let mut ci = self.cameras.iter().position(|c| c.name == d.camera).map(|i| i + 1).unwrap_or(0);
        if ci == 0 {
            d.camera.clear();
        }
        let crow = Rect::new(sx + lw, top + 28.0 + 6.0 * 24.0 + 4.0, colw - lw - 10.0, 22.0);
        let ctw = ui.text_w("Render from", 12.0);
        ui.text("Render from", sx + lw - ctw - 8.0, crow.y + 4.0, 12.0, C_TEXT);
        draw_line(sx + 8.0, crow.y - 10.0, sx + colw - 8.0, crow.y - 10.0, 1.0, C_LINE);
        let c0 = ci;
        ui.combo("matexp.camera", crow, &cams, &mut ci);
        if ci != c0 {
            d.camera = if ci == 0 { String::new() } else { cams[ci].clone() };
            // a camera's own frame shape, when the aspect ratio is kept
            if let (true, Some(c)) = (d.keep_aspect, self.cameras.iter().find(|c| c.name == d.camera)) {
                d.height = ((d.width as f32 / c.aspect()).round() as i32).clamp(16, 8192);
            }
        }
        if ui.hover(crow) {
            ui.tooltip = Some("Which view the pictures are rendered from: the viewport as it is now, or one of the scene's cameras (View menu > Add camera)".into());
        }
        label(ui, 7, ox, "Clip below ground", false);
        ui.checkbox("matexp.clip", row(7, ox), &mut d.clip_below);
        // Images
        let it = top + boxh + 34.0;
        ui.text("Images", r.x + 14.0, it - 20.0, 12.5, C_TEXT);
        let list = Rect::new(r.x + 12.0, it, r.w - 24.0, r.y + r.h - it - 82.0);
        draw_round_rect(list, 4.0, C_BG);
        if list.contains(ui.mouse()) && ui.inp.wheel != 0.0 && ui.popup.is_none() {
            d.scroll = (d.scroll - ui.inp.wheel * 50.0).max(0.0);
        }
        let entry_h = 92.0;
        let content = d.images.len() as f32 * (entry_h + 8.0);
        d.scroll = d.scroll.min((content - list.h + 8.0).max(0.0));
        ui.set_clip(Some(list));
        let src_names = names();
        let mut alpha_names = vec!["None".to_string()];
        alpha_names.extend(src_names.iter().cloned());
        let mut delete = None;
        for k in 0..d.images.len() {
            let ey = list.y + 6.0 + k as f32 * (entry_h + 8.0) - d.scroll;
            if ey + entry_h < list.y || ey > list.y + list.h {
                continue;
            }
            let e = Rect::new(list.x + 40.0, ey, list.w - 50.0, entry_h);
            draw_rectangle_lines(e.x, e.y, e.w, e.h, 1.0, C_LINE);
            if ui.button(&format!("matexp.del.{}", k), Rect::new(list.x + 8.0, ey + 4.0, 24.0, 22.0), "X") {
                delete = Some(k);
            }
            ui.text(&format!("{}", k + 1), e.x + 8.0, ey + 3.0, 12.0, C_TEXT);
            let fx = e.x + 58.0;
            let sw_w = 150.0;
            let fw = e.w - 58.0 - sw_w - 16.0;
            // Suffix (click to type; the whole text selected first)
            ui.text("Suffix", e.x + 10.0, ey + 24.0, 12.0, C_TEXT_DIM);
            let sr = Rect::new(fx, ey + 21.0, e.w - 66.0, 20.0);
            match d.editing.clone() {
                Some((i, mut text, mut all)) if i == k => {
                    if all && !ui.inp.chars.is_empty() {
                        text.clear();
                        all = false;
                    }
                    for c in &ui.inp.chars {
                        if !c.is_control() && text.chars().count() < 48 && (c.is_alphanumeric() || *c == '_' || *c == '-' || *c == ' ') {
                            text.push(*c);
                        }
                    }
                    if ui.inp.key(KeyCode::Backspace) {
                        if all {
                            text.clear();
                            all = false;
                        } else {
                            text.pop();
                        }
                    }
                    draw_rectangle(sr.x, sr.y, sr.w, sr.h, C_PANEL);
                    draw_rectangle_lines(sr.x, sr.y, sr.w, sr.h, 1.0, C_ACCENT2);
                    if all && !text.is_empty() {
                        let tw = ui.text_w(&text, 12.0).min(sr.w - 10.0);
                        draw_rectangle(sr.x + 3.0, sr.y + 3.0, tw + 4.0, sr.h - 6.0, C_ACCENT);
                    }
                    ui.text_fit(&text, sr.x + 5.0, sr.y + 3.0, sr.w - 10.0, 12.0, WHITE);
                    if !all && (get_time() * 2.0) as i64 % 2 == 0 {
                        let cx = sr.x + 6.0 + ui.text_w(&text, 12.0).min(sr.w - 12.0);
                        draw_line(cx, sr.y + 3.0, cx, sr.y + sr.h - 3.0, 1.0, WHITE);
                    }
                    let done = ui.inp.key(KeyCode::Enter) || ui.inp.key(KeyCode::KpEnter) || ui.inp.key(KeyCode::Tab) || (ui.inp.l_pressed && !sr.contains(ui.mouse()));
                    if ui.inp.key(KeyCode::Escape) {
                        d.editing = None;
                    } else if done {
                        d.images[k].suffix = text.trim().to_string();
                        d.editing = None;
                    } else {
                        d.editing = Some((k, text, all));
                    }
                }
                _ => {
                    let hot = sr.contains(ui.mouse()) && ui.popup.is_none();
                    draw_rectangle(sr.x, sr.y, sr.w, sr.h, if hot { C_BTN_HOT } else { C_BTN });
                    let s = d.images[k].suffix.clone();
                    ui.text_fit(&s, sr.x + 5.0, sr.y + 3.0, sr.w - 10.0, 12.0, C_TEXT);
                    if hot && ui.inp.l_pressed {
                        d.editing = Some((k, s, true));
                    }
                }
            }
            // RGB and Alpha sources, each with its background swatch
            ui.text("RGB", e.x + 10.0 + 18.0, ey + 47.0, 12.0, C_TEXT_DIM);
            let mut v = d.images[k].rgb;
            ui.combo(&format!("matexp.rgb.{}", k), Rect::new(fx, ey + 44.0, fw, 20.0), &src_names, &mut v);
            d.images[k].rgb = v.min(SOURCES.len() - 1);
            ui.text("Alpha", e.x + 10.0 + 6.0, ey + 70.0, 12.0, C_TEXT_DIM);
            let mut a = d.images[k].alpha;
            ui.combo(&format!("matexp.alpha.{}", k), Rect::new(fx, ey + 67.0, fw, 20.0), &alpha_names, &mut a);
            d.images[k].alpha = a.min(SOURCES.len());
            for (which, y) in [(0u8, ey + 44.0), (1u8, ey + 67.0)] {
                let swr = Rect::new(fx + fw + 8.0, y, sw_w, 20.0);
                let c = if which == 0 { let b = d.images[k].bg_rgb; Color::new(b[0], b[1], b[2], 1.0) } else { let a = d.images[k].bg_alpha; Color::new(a, a, a, 1.0) };
                if d.swatch == Some((k, which)) {
                    // its sliders in place of the swatch
                    if which == 0 {
                        let cw = (sw_w - 4.0) / 3.0;
                        for ch in 0..3 {
                            let mut x = d.images[k].bg_rgb[ch];
                            ui.slider(&format!("matexp.bg.{}.{}", k, ch), Rect::new(swr.x + ch as f32 * (cw + 2.0), y, cw, 20.0), &mut x, 0.0, 1.0, 2);
                            d.images[k].bg_rgb[ch] = x.clamp(0.0, 1.0);
                        }
                    } else {
                        let mut x = d.images[k].bg_alpha;
                        ui.slider(&format!("matexp.bga.{}", k), swr, &mut x, 0.0, 1.0, 2);
                        d.images[k].bg_alpha = x.clamp(0.0, 1.0);
                    }
                    if ui.inp.l_pressed && !Rect::new(swr.x - 4.0, swr.y - 4.0, swr.w + 8.0, swr.h + 8.0).contains(ui.mouse()) {
                        d.swatch = None;
                    }
                } else {
                    draw_rectangle(swr.x, swr.y, swr.w, swr.h, c);
                    draw_rectangle_lines(swr.x, swr.y, swr.w, swr.h, 1.0, C_LINE);
                    if swr.contains(ui.mouse()) && ui.popup.is_none() {
                        ui.tooltip = Some(if which == 0 { "The background colour where nothing is drawn (with Override bkgd). Click to change it." } else { "The background alpha where nothing is drawn (with Override bkgd). Click to change it." }.into());
                        if ui.inp.l_pressed {
                            d.swatch = Some((k, which));
                        }
                    }
                }
            }
        }
        ui.set_clip(None);
        if let Some(k) = delete {
            d.images.remove(k);
            d.editing = None;
            d.swatch = None;
        }
        // Add Image (picture 54): a menu of the render modes, as the original offers them; the
        // picked one becomes a new image named after it
        let ar = Rect::new(r.x + 12.0, r.y + r.h - 74.0, r.w - 24.0, 26.0);
        if ui.button("matexp.add", ar, "Add Image") {
            ui.open_menu(hid("matexp.add"), ar, add_image_menu());
        }
        if ui.hover(ar) {
            ui.tooltip = Some("Add another picture to render: pick which render mode it shows".into());
        }
        if let Some(p) = ui.take_menu_result("matexp.add") {
            if let Some(k) = add_image_source(&p) {
                let name = SOURCES[k].0;
                let short = name.rsplit(" / ").next().unwrap_or(name).replace(' ', "");
                // backgrounds like the defaults: a flat normal for Normal, white for AO-like
                // pictures, black otherwise
                let bg = match short.as_str() {
                    "Normal" => [0.5, 0.5, 1.0],
                    "AO" | "SubsurfaceAmount" => [1.0, 1.0, 1.0],
                    _ => [0.0, 0.0, 0.0],
                };
                d.images.push(ExImage { suffix: short, rgb: k, alpha: 0, bg_rgb: bg, bg_alpha: 0.0 });
                d.scroll = 1.0e6;
                d.editing = None;
            }
        }
        // presets (picture 54): pick a saved one, save the current settings, load a file
        let by = r.y + r.h - 38.0;
        let mut px = r.x + 12.0;
        ui.text("Preset", px, by + 5.0, 12.0, C_TEXT_DIM);
        px += 48.0;
        let mut names = preset_names();
        names.insert(0, "(choose)".to_string());
        let mut pick = 0usize;
        let pr = Rect::new(px, by, 170.0, 26.0);
        ui.combo("matexp.preset", pr, &names, &mut pick);
        if pick > 0 {
            let path = presets_dir().join(format!("{}.tcexport", names[pick]));
            match std::fs::read_to_string(&path).ok().and_then(|t| MatExport::from_preset(&t, &d)) {
                Some(n) => {
                    d = n;
                    self.set_status(&format!("Loaded the export preset \"{}\"", names[pick]));
                }
                None => self.set_status(&format!("Couldn't read the export preset \"{}\"", names[pick])),
            }
        }
        if ui.hover(pr) {
            ui.tooltip = Some(format!("Saved export presets (kept in {})", presets_dir().display()));
        }
        px += 176.0;
        if ui.button("matexp.save", Rect::new(px, by, 96.0, 26.0), "Save preset...") {
            let dir = presets_dir();
            let _ = std::fs::create_dir_all(&dir);
            if let Some(path) = crate::prefs::file_dialog().set_title("Save export preset").set_directory(&dir).add_filter("Export preset", &["tcexport"]).set_file_name("My preset.tcexport").save_file() {
                let path = if path.extension().is_none() { path.with_extension("tcexport") } else { path };
                match std::fs::write(&path, d.to_preset()) {
                    Ok(()) => self.set_status(&format!("Saved the export preset {}", path.display())),
                    Err(e) => self.set_status(&format!("Couldn't save the preset: {}", e)),
                }
            }
        }
        px += 100.0;
        if ui.button("matexp.load", Rect::new(px, by, 96.0, 26.0), "Load preset...") {
            let dir = presets_dir();
            let _ = std::fs::create_dir_all(&dir);
            if let Some(path) = crate::prefs::file_dialog().set_title("Load export preset").set_directory(&dir).add_filter("Export preset", &["tcexport"]).pick_file() {
                match std::fs::read_to_string(&path).ok().and_then(|t| MatExport::from_preset(&t, &d)) {
                    Some(n) => {
                        d = n;
                        self.set_status(&format!("Loaded the export preset {}", path.display()));
                    }
                    None => self.set_status("That file isn't an export preset"),
                }
            }
        }
        let okr = Rect::new(r.x + r.w - 190.0, by, 84.0, 26.0);
        let cr = Rect::new(r.x + r.w - 98.0, by, 84.0, 26.0);
        let count = d.images.len() * if d.sequence { d.frames.max(1) as usize } else { 1 };
        // the summary, right of the Images label
        let from = if d.camera.is_empty() { "the current view".to_string() } else { d.camera.clone() };
        let summary = format!("{} picture{}, {} x {} px, from {}", count, if count == 1 { "" } else { "s" }, d.width, d.height, from);
        let tw = ui.text_w(&summary, 12.0);
        ui.text(&summary, r.x + r.w - 14.0 - tw, it - 20.0, 12.0, C_TEXT_DIM);
        let ok = ui.button("matexp.ok", okr, "OK");
        if ui.button("matexp.cancel", cr, "Cancel") || (ui.inp.key(KeyCode::Escape) && d.editing.is_none() && ui.popup.is_none()) {
            close = true;
        }
        if ok && !d.images.is_empty() {
            if let Some(dir) = crate::prefs::file_dialog().set_title("Export material: choose where the pictures go").pick_folder() {
                self.start_material_export(dir, d.clone());
                close = true;
            }
        }
        self.mat_export = Some(d);
        if close {
            self.mat_export_open = false;
        }
        // modal: nothing behind the window reacts (its menus still get their clicks)
        ui.blocked = true;
        if ui.popup.is_none() {
            ui.inp.l_pressed = false;
        }
        ui.inp.r_pressed = false;
    }

    /// For checking pictures: export the default list at `size` px into `dir` (no folder dialog).
    pub fn test_material_export(&mut self, dir: std::path::PathBuf, size: i32) {
        let mut s = self.mat_export.clone().unwrap_or_else(MatExport::new);
        s.width = size;
        s.height = size;
        self.start_material_export(dir, s);
    }

    /// Queue every picture; they are rendered one a frame (run_material_export).
    fn start_material_export(&mut self, dir: std::path::PathBuf, s: MatExport) {
        let base = self.tree_path.as_ref().and_then(|p| p.file_stem()).map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "tree".into());
        let mut todo = vec![];
        for (k, im) in s.images.iter().enumerate() {
            let stem = if im.suffix.is_empty() { base.clone() } else { format!("{}_{}", base, im.suffix) };
            if s.sequence {
                for f in 0..s.frames.max(1) as usize {
                    todo.push((k, Some(f), dir.join(format!("{}_{:04}.png", stem, f + 1))));
                }
            } else {
                todo.push((k, None, dir.join(format!("{}.png", stem))));
            }
        }
        todo.reverse();
        let total = todo.len();
        self.mat_export_job = Some(ExportJob { dir, settings: s, todo, done: 0, total, failed: vec![], wait: 3, current: String::new() });
        self.mat_export_result = None;
    }

    /// While exporting (picture 55): a window with a progress bar, the picture being made and
    /// Cancel; afterwards, whether it worked, where the pictures went, Open folder and OK.
    fn draw_export_progress(&mut self, ui: &mut Ui) {
        ui.blocked = false;
        let (sw, sh) = (ui.sw, ui.sh);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.35));
        let w = 480.0f32.min(sw - 20.0);
        let h = 168.0;
        let r = Rect::new((sw - w) * 0.5, (sh - h) * 0.5, w, h);
        draw_rectangle(r.x + 4.0, r.y + 5.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_round_rect(r, 6.0, C_PANEL);
        draw_round_rect(Rect::new(r.x, r.y, r.w, 28.0), 6.0, C_PANEL2);
        ui.record("matexp.progress", r);
        let br = Rect::new(r.x + r.w - 104.0, r.y + r.h - 38.0, 92.0, 26.0);
        if let Some(job) = &self.mat_export_job {
            ui.text("Exporting material", r.x + 12.0, r.y + 6.0, 13.5, C_TEXT);
            let frac = if job.total == 0 { 0.0 } else { job.done as f32 / job.total as f32 };
            let line = if job.done == 0 && job.current.is_empty() { "Getting the tree ready...".to_string() } else { format!("Picture {} of {}: {}", (job.done + 1).min(job.total), job.total, job.current) };
            ui.text_fit(&line, r.x + 14.0, r.y + 44.0, r.w - 28.0, 12.5, C_TEXT);
            let bar = Rect::new(r.x + 14.0, r.y + 70.0, r.w - 28.0, 18.0);
            draw_rectangle(bar.x, bar.y, bar.w, bar.h, C_BG);
            draw_rectangle(bar.x, bar.y, bar.w * frac, bar.h, C_ACCENT);
            draw_rectangle_lines(bar.x, bar.y, bar.w, bar.h, 1.0, C_LINE);
            ui.text_center(&format!("{:.0}%", frac * 100.0), bar, 12.0, WHITE);
            ui.text_fit(&format!("Into {}", job.dir.display()), r.x + 14.0, r.y + 98.0, r.w - 28.0, 12.0, C_TEXT_DIM);
            if ui.button("matexp.stop", br, "Cancel") {
                // stop after the picture in hand; what's saved stays
                let job = self.mat_export_job.take().unwrap();
                let saved = job.done - job.failed.len().min(job.done);
                self.mat_export_result = Some(ExportResult {
                    ok: false,
                    title: "Export cancelled".into(),
                    lines: vec![format!("{} of {} pictures were saved before it stopped.", saved, job.total), format!("Folder: {}", job.dir.display())],
                    dir: job.dir,
                });
            }
        } else if let Some(res) = &self.mat_export_result {
            let (ok, title, lines, dir) = (res.ok, res.title.clone(), res.lines.clone(), res.dir.clone());
            // a green tick or an orange mark beside the title
            let (mx, my) = (r.x + 18.0, r.y + 14.0);
            if ok {
                draw_line(mx - 6.0, my, mx - 2.0, my + 5.0, 2.5, Color::new(0.4, 0.85, 0.4, 1.0));
                draw_line(mx - 2.0, my + 5.0, mx + 6.0, my - 6.0, 2.5, Color::new(0.4, 0.85, 0.4, 1.0));
            } else {
                draw_circle(mx, my, 7.0, Color::new(0.95, 0.55, 0.2, 1.0));
                ui.text("!", mx - 2.0, my - 8.0, 13.0, WHITE);
            }
            ui.text(&title, r.x + 32.0, r.y + 6.0, 13.5, C_TEXT);
            for (k, l) in lines.iter().enumerate() {
                ui.text_fit(l, r.x + 14.0, r.y + 42.0 + k as f32 * 22.0, r.w - 28.0, 12.5, if k == 0 { C_TEXT } else { C_TEXT_DIM });
            }
            let fr = Rect::new(r.x + 14.0, r.y + r.h - 38.0, 120.0, 26.0);
            if ui.button("matexp.openfolder", fr, "Open folder") {
                #[cfg(windows)]
                let _ = std::process::Command::new("explorer").arg(&dir).spawn();
                #[cfg(not(windows))]
                let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
            }
            if ui.button("matexp.done", br, "OK") || ui.inp.key(KeyCode::Enter) || ui.inp.key(KeyCode::Escape) {
                self.mat_export_result = None;
            }
        }
        ui.blocked = true;
        if ui.popup.is_none() {
            ui.inp.l_pressed = false;
        }
        ui.inp.r_pressed = false;
    }

    /// Called each frame before the viewport is drawn: render and save the next picture.
    pub fn run_material_export(&mut self) {
        let Some(mut job) = self.mat_export_job.take() else { return };
        // wait for a built tree whose pictures are loaded and that has been drawn once
        let loading = self.dirty || self.gpu.is_none() || self.materials.iter().any(|m| m.dirty_tex || m.dirty_base);
        if job.wait > 0 || loading {
            job.wait = job.wait.saturating_sub(1);
            self.mat_export_job = Some(job);
            return;
        }
        let Some((k, frame, path)) = job.todo.pop() else {
            // finished: say how it went (picture 55)
            let saved = job.total - job.failed.len();
            let ok = job.failed.is_empty();
            let mut lines = vec![if ok { format!("All {} picture{} were saved.", job.total, if job.total == 1 { "" } else { "s" }) } else { format!("{} of {} pictures were saved.", saved, job.total) }];
            lines.push(format!("Folder: {}", job.dir.display()));
            if !ok {
                lines.push(format!("Not saved: {}", job.failed.join(", ")));
            }
            self.set_status(&lines[0].clone());
            self.mat_export_result = Some(ExportResult { ok, title: if ok { "Export finished".into() } else { "Export finished with problems".into() }, lines, dir: job.dir });
            return;
        };
        job.current = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let s = job.settings.clone();
        let im = s.images[k].clone();
        let time = match frame {
            Some(f) => f as f32 / s.fps.max(1.0),
            None => get_time() as f32,
        };
        let (w, h) = (s.width.max(16) as u32, s.height.max(16) as u32);
        let rgb = self.render_mode_pixels(SOURCES[im.rgb].1, w, h, time, &s);
        let alpha = if im.alpha > 0 { Some(self.render_mode_pixels(SOURCES[im.alpha - 1].1, w, h, time, &s)) } else { None };
        match rgb {
            Some(px) => {
                let out = compose(&px, alpha.as_ref().and_then(|a| a.as_ref()), w as usize, h as usize, &im, &s);
                if image::save_buffer(&path, &out, w, h, image::ColorType::Rgba8).is_err() {
                    job.failed.push(path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                }
            }
            None => job.failed.push("nothing to draw".into()),
        }
        job.done += 1;
        self.set_status(&format!("Exporting material pictures: {} of {}...", job.done, job.total));
        self.mat_export_job = Some(job);
    }

    /// The tree drawn in one render mode from the current view into a picture `w` x `h`
    /// (RGBA, top row first); alpha 0 where nothing is drawn.
    fn render_mode_pixels(&mut self, mode: f32, w: u32, h: u32, time: f32, s: &MatExport) -> Option<Vec<u8>> {
        let gm = self.gm.clone()?;
        let rt = render_target(w, h);
        rt.texture.set_filter(FilterMode::Nearest);
        // the current view, or the chosen camera (its own lens, filling the picture's height)
        let cam = match self.cameras.iter().find(|c| !s.camera.is_empty() && c.name == s.camera) {
            Some(c) => c.to_cam(1.0),
            None => self.cam,
        };
        set_camera(&cam.camera3d(None, w as f32 / h as f32, Some(rt.clone())));
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        if s.ground {
            let ext = (self.bounds.1 - self.bounds.0).length();
            crate::render::draw_ground((ext * 0.9).clamp(12.0, 400.0), -(self.cam.dist * 0.004).max(0.02), self.prefs.color("ground"));
        }
        let mut light = self.light.clone();
        light.mode = mode;
        light.time = time;
        light.wind = if s.sequence && s.include_wind { if self.wind_on { self.wind_strength } else { 1.0 } } else { 0.0 };
        light.shading = 0.0;
        light.overdraw = false;
        light.sel_fill = [0.0; 4];
        let gpu = self.gpu.as_ref()?;
        crate::app::draw_gpu(gpu, &self.materials, &gm, &light, cam.position());
        unsafe { get_internal_gl() }.flush();
        gl_use_default_material();
        set_default_camera();
        let img = rt.texture.get_texture_data();
        // the picture comes back bottom row first
        let row = w as usize * 4;
        let mut out = vec![0u8; row * h as usize];
        for y in 0..h as usize {
            let src = (h as usize - 1 - y) * row;
            out[y * row..(y + 1) * row].copy_from_slice(&img.bytes[src..src + row]);
        }
        Some(out)
    }
}

/// The finished picture: colour from `rgb`, alpha from `alpha`'s brightness (or where anything
/// was drawn); where nothing was drawn, Streak colors spreads the nearest colours outward and
/// Override bkgd sets the image's background colour (when not streaked) and alpha.
fn compose(rgb: &[u8], alpha: Option<&Vec<u8>>, w: usize, h: usize, im: &ExImage, s: &MatExport) -> Vec<u8> {
    let n = w * h;
    let mut out = vec![0u8; n * 4];
    let covered: Vec<bool> = (0..n).map(|i| rgb[i * 4 + 3] > 0).collect();
    let bg = [(im.bg_rgb[0] * 255.0) as u8, (im.bg_rgb[1] * 255.0) as u8, (im.bg_rgb[2] * 255.0) as u8];
    let bga = if s.override_bg { (im.bg_alpha * 255.0) as u8 } else { 0 };
    for i in 0..n {
        if covered[i] {
            out[i * 4..i * 4 + 3].copy_from_slice(&rgb[i * 4..i * 4 + 3]);
            out[i * 4 + 3] = match alpha {
                Some(a) => {
                    let l = 0.3 * a[i * 4] as f32 + 0.59 * a[i * 4 + 1] as f32 + 0.11 * a[i * 4 + 2] as f32;
                    l.round().clamp(0.0, 255.0) as u8
                }
                None => 255,
            };
        } else {
            if s.override_bg {
                out[i * 4..i * 4 + 3].copy_from_slice(&bg);
            }
            out[i * 4 + 3] = bga;
        }
    }
    if s.streak && covered.iter().any(|c| *c) {
        streak(&mut out, &covered, w, h);
    }
    out
}

/// Streak colors: fill every empty pixel's colour from the drawn ones around it (a push-pull
/// over halving sizes), so edges don't pick up the background when the picture is filtered.
fn streak(px: &mut [u8], covered: &[bool], w: usize, h: usize) {
    // level 0: colour sums and weights
    let mut levels: Vec<(usize, usize, Vec<[f32; 4]>)> = vec![];
    let mut cur: Vec<[f32; 4]> = (0..w * h).map(|i| if covered[i] { [px[i * 4] as f32, px[i * 4 + 1] as f32, px[i * 4 + 2] as f32, 1.0] } else { [0.0; 4] }).collect();
    let (mut cw, mut ch) = (w, h);
    levels.push((cw, ch, cur.clone()));
    while cw > 1 || ch > 1 {
        let (nw, nh) = (cw.div_ceil(2), ch.div_ceil(2));
        let mut next = vec![[0.0f32; 4]; nw * nh];
        for y in 0..ch {
            for x in 0..cw {
                let v = cur[y * cw + x];
                let t = &mut next[(y / 2) * nw + x / 2];
                for c in 0..4 {
                    t[c] += v[c];
                }
            }
        }
        levels.push((nw, nh, next.clone()));
        cur = next;
        cw = nw;
        ch = nh;
    }
    // pull: each level's empty cells take the coarser level's average
    for li in (0..levels.len() - 1).rev() {
        let (pw, _, ref parent) = levels[li + 1].clone();
        let (lw, lh, ref mut cells) = levels[li];
        for y in 0..lh {
            for x in 0..lw {
                let c = &mut cells[y * lw + x];
                if c[3] <= 0.0 {
                    let p = parent[(y / 2) * pw + x / 2];
                    if p[3] > 0.0 {
                        *c = [p[0] / p[3], p[1] / p[3], p[2] / p[3], 1.0];
                    }
                } else if c[3] != 1.0 {
                    *c = [c[0] / c[3], c[1] / c[3], c[2] / c[3], 1.0];
                }
            }
        }
    }
    let base = &levels[0].2;
    for i in 0..w * h {
        if !covered[i] && base[i][3] > 0.0 {
            for c in 0..3 {
                px[i * 4 + c] = base[i][c].round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Settings come back the same after saving and loading a preset.
    #[test]
    fn preset_round_trip() {
        let mut a = MatExport::new();
        a.width = 1024;
        a.sequence = true;
        a.images[2].alpha = src("AO") + 1;
        a.images[0].bg_alpha = 0.25;
        let b = MatExport::from_preset(&a.to_preset(), &MatExport::new()).expect("a preset");
        assert_eq!(a.to_preset(), b.to_preset());
        assert!(MatExport::from_preset("not a preset", &a).is_none());
    }

    /// Every entry of the Add Image menu names a render mode, and they all are reachable.
    #[test]
    fn add_image_menu_covers_every_source() {
        let menu = add_image_menu();
        let mut seen = vec![];
        for (i, it) in menu.iter().enumerate() {
            if it.children.is_empty() {
                seen.push(add_image_source(&[i]).expect("top entry"));
            } else {
                for j in 0..it.children.len() {
                    seen.push(add_image_source(&[i, j]).expect("sub entry"));
                }
            }
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), SOURCES.len());
        let mat = menu.iter().position(|m| m.label == "Material").unwrap();
        let normal = menu[mat].children.iter().position(|m| m.label == "Normal").unwrap();
        assert_eq!(add_image_source(&[mat, normal]), Some(src("Material / Normal")));
    }
}
