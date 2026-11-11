mod app;
mod cutout;
mod gen;
mod meshdialog;
mod matpanel;
mod meshio;
mod model;
mod props;
mod render;
mod textures;
mod ui;

use app::{App, Sel};
use macroquad::prelude::*;
use model::Kind;
use std::collections::HashMap;
use ui::*;

/// Physical pixels per UI unit: the display's scaling, but never squeezing the layout
/// below ~880 units tall.
fn ui_scale() -> f32 {
    let dpi = screen_dpi_scale().max(0.5);
    let phys_h = screen_height() * dpi;
    dpi.min(phys_h / 880.0).max(0.6)
}

/// Macroquad units (physical / dpi) per UI unit.
fn ui_factor() -> f32 {
    ui_scale() / screen_dpi_scale().max(0.5)
}

fn window_conf() -> macroquad::conf::Conf {
    macroquad::conf::Conf {
        miniquad_conf: miniquad::conf::Conf {
            window_title: "TreeCraft".to_string(),
            window_width: 1600,
            window_height: 900,
            high_dpi: true,
            window_resizable: true,
            sample_count: 4,
            ..Default::default()
        },
        draw_call_vertex_capacity: 32000,
        draw_call_index_capacity: 120000,
        ..Default::default()
    }
}

const MENUS: [(&str, &[&str]); 5] = [
    ("File", &["New tree", "Quit"]),
    ("Edit", &["Delete node", "Duplicate node", "Randomize", "Arrange nodes"]),
    ("View", &["Perspective", "Front", "Back", "Left", "Right", "Top", "Bottom", "Toggle orthographic", "Frame tree"]),
    ("Window", &["Generation", "Materials", "Meshes", "Window properties"]),
    ("Help", &["Controls"]),
];

fn menu_bar(app: &mut App, ui: &mut Ui, w: f32) -> bool {
    draw_rectangle(0.0, 0.0, w, 24.0, C_BG);
    let mut x = 4.0;
    let mut quit = false;
    for (name, items) in MENUS.iter() {
        let key = format!("menu.{}", name);
        let tw = ui.text_w(name, 13.0) + 16.0;
        let r = Rect::new(x, 1.0, tw, 22.0);
        if ui.tool_button(&key, r, name, ui.popup_open_for(&key)) {
            ui.open_popup(hid(&key), r, items.iter().map(|s| s.to_string()).collect(), None);
        }
        if let Some(i) = ui.take_popup_result(&key) {
            let sel_node = if let Sel::Node(id) = app.sel { Some(id) } else { None };
            match (*name, i) {
                ("File", 0) => app.new_tree(),
                ("File", 1) => quit = true,
                ("Edit", 0) => match sel_node {
                    Some(id) => app.delete_node(id),
                    None => app.set_status("Select a node first"),
                },
                ("Edit", 1) => {
                    if let Some(id) = sel_node {
                        app.duplicate_node(id)
                    }
                }
                ("Edit", 2) => app.randomize(sel_node),
                ("Edit", 3) => app.arrange(),
                ("View", v) if v <= 6 => app.cam.set_view(v),
                ("View", 7) => app.cam.ortho = !app.cam.ortho,
                ("View", 8) => app.frame_tree(),
                ("Window", t) if t <= 2 => app.tab = t,
                ("Window", 3) => app.select(Sel::Window),
                ("Help", _) => app.set_status("Left-drag orbit • middle/shift-drag pan • wheel zoom • F frame • Del delete node"),
                _ => {}
            }
        }
        x += tw;
    }
    quit
}

fn keyboard(app: &mut App, ui: &Ui) {
    if ui.popup.is_some() {
        return;
    }
    let i = &ui.inp;
    let sel_node = if let Sel::Node(id) = app.sel { Some(id) } else { None };
    if i.key(KeyCode::F) {
        app.frame_tree();
    }
    if i.key(KeyCode::Key0) || i.key(KeyCode::Kp0) {
        app.cam.set_view(0);
    }
    if i.key(KeyCode::Key1) || i.key(KeyCode::Kp1) {
        app.cam.set_view(if i.ctrl { 2 } else { 1 });
    }
    if i.key(KeyCode::Key3) || i.key(KeyCode::Kp3) {
        app.cam.set_view(if i.ctrl { 3 } else { 4 });
    }
    if i.key(KeyCode::Key7) || i.key(KeyCode::Kp7) {
        app.cam.set_view(if i.ctrl { 6 } else { 5 });
    }
    if i.key(KeyCode::Key5) || i.key(KeyCode::Kp5) {
        app.cam.ortho = !app.cam.ortho;
    }
    if i.key(KeyCode::Delete) {
        if let Some(id) = sel_node {
            app.delete_node(id);
        }
    }
    if i.key(KeyCode::D) && i.ctrl {
        if let Some(id) = sel_node {
            app.duplicate_node(id);
        }
    }
    if i.key(KeyCode::Escape) {
        app.select(Sel::None);
    }
}

struct Layout {
    left: Rect,
    view: Rect,
    dock: Rect,
}

fn layout(w: f32, h: f32, left_w: Option<f32>, dock_w: Option<f32>) -> Layout {
    let top = 26.0;
    // the panels' widths, dragged by their edges (the viewport keeps at least 200)
    let lw = left_w.unwrap_or((w * 0.22).clamp(300.0, 380.0)).clamp(220.0, (w * 0.45).max(220.0));
    let dw = dock_w.unwrap_or((w * 0.30).clamp(380.0, 560.0)).clamp(300.0, (w - lw - 212.0).max(300.0));
    Layout {
        left: Rect::new(2.0, top, lw, h - top - 2.0),
        view: Rect::new(lw + 6.0, top, w - lw - dw - 12.0, h - top - 2.0),
        dock: Rect::new(w - dw - 2.0, top, dw, h - top - 2.0),
    }
}

/// One frame of the whole app. Returns false to quit.
fn frame(app: &mut App, ui: &mut Ui, inp: Inp) -> bool {
    let scale = ui_scale();
    let f = ui_factor();
    let dpi = screen_dpi_scale().max(0.5);
    ui.scale = scale;
    let w = screen_width() / f;
    let h = screen_height() / f;
    ui.sw = w;
    ui.sh = h;
    ui.begin(inp);
    // a modal dialog takes all input: panels behind it see an idle mouse and no keys
    let dialog_input = if app.mesh_dialog.is_some() || app.mat_dialog.is_some() || app.cutout.is_some() {
        let real = ui.inp.clone();
        ui.inp = Inp { mx: real.mx, my: real.my, ..Default::default() };
        ui.blocked = true;
        Some(real)
    } else {
        None
    };
    // splitters between the panels and the viewport: drag to widen or narrow a panel
    {
        let lay0 = layout(w, h, app.left_w, app.dock_w);
        let (mx, my) = (ui.inp.mx, ui.inp.my);
        let in_y = my > lay0.view.y && my < lay0.view.y + lay0.view.h;
        let on_left = in_y && (mx - (lay0.left.x + lay0.left.w + 2.0)).abs() <= 4.0;
        let on_right = in_y && (mx - (lay0.dock.x - 2.0)).abs() <= 4.0;
        if app.split_drag == 0 && ui.active == 0 && ui.popup.is_none() && ui.inp.l_pressed && (on_left || on_right) {
            app.split_drag = if on_left { 1 } else { 2 };
            ui.inp.l_pressed = false;
        }
        if app.split_drag != 0 {
            if ui.inp.l_down {
                match app.split_drag {
                    1 => app.left_w = Some(mx - lay0.left.x - 2.0),
                    _ => app.dock_w = Some(w - 2.0 - mx - 2.0),
                }
                ui.inp.l_pressed = false;
            } else {
                app.split_drag = 0;
            }
        }
        if app.split_drag != 0 || on_left || on_right {
            let x = if app.split_drag == 1 || (app.split_drag == 0 && on_left) { lay0.left.x + lay0.left.w + 2.0 } else { lay0.dock.x - 2.0 };
            draw_rectangle(x - 1.5, lay0.view.y, 3.0, lay0.view.h, C_ACCENT);
        }
    }
    let lay = layout(w, h, app.left_w, app.dock_w);
    app.viewport = lay.view;

    if app.dirty {
        app.regenerate();
    }
    app.render_preview();
    app.render_mesh_preview();

    clear_background(C_BG);
    // 3D viewport first: background in UI units, scene in physical pixels
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.push_model_matrix(Mat4::from_scale(vec3(f, f, 1.0)));
    app.draw_viewport_bg(lay.view);
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.pop_model_matrix();
    app.draw_viewport_3d(lay.view, scale, screen_height() * dpi);

    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.push_model_matrix(Mat4::from_scale(vec3(f, f, 1.0)));

    let quit = menu_bar(app, ui, w);
    props::draw_panel(app, ui, lay.left);
    app.draw_dock(ui, lay.dock);
    app.draw_viewport_overlay(ui, lay.view);
    app.viewport_input(ui, lay.view);
    keyboard(app, ui);
    if let Some(real) = dialog_input {
        ui.inp = real;
    }
    app.draw_mesh_dialog(ui);
    app.draw_cutout_window(ui);
    ui.end();

    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.pop_model_matrix();
    !quit
}

// ------------------------------------------------------------------ self-test

struct SelfTest {
    path: String,
    frame: usize,
    steps: Vec<(String, bool, String)>,
    snap: HashMap<String, f32>,
    phase: usize,
    wait: usize,
    hud: Option<(bool, String)>,
}

fn center(r: &Rect) -> (f32, f32) {
    (r.x + r.w * 0.5, r.y + r.h * 0.5)
}

impl SelfTest {
    /// Produce scripted input for this frame from the previous frame's widget rectangles.
    fn input(&mut self, app: &mut App, rects: &HashMap<String, Rect>, texts: &[String]) -> (Inp, bool) {
        self.frame += 1;
        let mut i = Inp::default();
        let vp = app.viewport;
        let (vx, vy) = (vp.x + vp.w * 0.5, vp.y + vp.h * 0.5);
        i.mx = vx;
        i.my = vy;
        if self.wait > 0 {
            self.wait -= 1;
            return (i, false);
        }
        let click = |i: &mut Inp, r: Option<&Rect>| -> bool {
            if let Some(r) = r {
                let (x, y) = center(r);
                i.mx = x;
                i.my = y;
                i.l_pressed = true;
                i.l_down = true;
                true
            } else {
                false
            }
        };
        match self.phase {
            0 => {
                self.snap.insert("yaw".into(), app.cam.yaw);
                self.snap.insert("dist".into(), app.cam.dist);
                i.l_pressed = true;
                i.l_down = true;
                self.phase = 1;
            }
            1..=20 => {
                i.mx = vx + self.phase as f32 * 6.0;
                i.my = vy + self.phase as f32 * 1.5;
                i.l_down = true;
                self.phase += 1;
            }
            21 => {
                i.mx = vx + 126.0;
                i.l_released = true;
                self.phase += 1;
            }
            22..=24 => {
                i.wheel = 1.0;
                self.phase += 1;
            }
            25 => {
                let dyaw = (app.cam.yaw - self.snap["yaw"]).to_degrees().abs();
                let dd = self.snap["dist"] - app.cam.dist;
                let ok = dyaw > 5.0 && dd > 0.5;
                self.steps.push(("move".into(), ok, format!("camera orbited {:.0} degrees and zoomed in {:.1} units with scripted mouse drag and wheel", dyaw, dd)));
                self.phase += 1;
            }
            // ----- act: select Branch node, drag its Frequency slider, add a Leaf, toggle ortho
            26 => {
                self.snap.insert("spines".into(), app.geo_stats.2 as f32);
                self.snap.insert("nodes".into(), app.nodes.len() as f32);
                if !click(&mut i, rects.get("node:Branch")) {
                    self.steps.push(("act".into(), false, "Branch node not found in the graph".into()));
                    self.phase = 40;
                } else {
                    self.phase += 1;
                    self.wait = 2;
                }
            }
            27 => {
                let ok = matches!(app.sel, Sel::Node(id) if app.node(id).map(|n| n.kind == Kind::Branch).unwrap_or(false));
                self.snap.insert("sel_ok".into(), if ok { 1.0 } else { 0.0 });
                let key = rects.keys().find(|k| k.starts_with("sl:n") && k.ends_with(".frequency")).cloned();
                if let Some(r) = key.and_then(|k| rects.get(&k).cloned()) {
                    i.mx = r.x + r.w * 0.25;
                    i.my = r.y + r.h * 0.5;
                    i.l_pressed = true;
                    i.l_down = true;
                }
                self.phase += 1;
            }
            28 => {
                i.l_released = true;
                self.phase += 1;
                self.wait = 2;
            }
            29 => {
                click(&mut i, rects.get("btn:gen.add"));
                self.phase += 1;
                self.wait = 1;
            }
            30 => {
                click(&mut i, rects.get("pop:Leaf"));
                self.phase += 1;
                self.wait = 2;
            }
            31 => {
                click(&mut i, rects.get("btn:vp.ortho"));
                self.phase += 1;
                self.wait = 2;
            }
            32 => {
                let sel_ok = self.snap["sel_ok"] > 0.5;
                let sp0 = self.snap["spines"] as usize;
                let sp1 = app.geo_stats.2;
                let n0 = self.snap["nodes"] as usize;
                let n1 = app.nodes.len();
                let ok = sel_ok && sp1 != sp0 && n1 == n0 + 1 && app.cam.ortho && app.geo_stats.3 > 0;
                self.steps.push((
                    "act".into(),
                    ok,
                    format!(
                        "clicked Branch node (selected: {}), dragged Frequency: branches {} -> {}; Add > Leaf: nodes {} -> {}; orthographic view: {}; {} leaves, {} triangles generated",
                        sel_ok, sp0, sp1, n0, n1, app.cam.ortho, app.geo_stats.3, app.geo_stats.0
                    ),
                ));
                click(&mut i, rects.get("tab:Materials"));
                self.phase += 1;
                self.wait = 3;
            }
            33 => {
                let mat_ok = ["cmb:mat.pick", "matpreview", "matslot:Color", "mattab:Settings", "mattab:Meshes", "btn:mat.addremove"].iter().all(|k| rects.contains_key(*k));
                self.snap.insert("mat_ok".into(), if mat_ok { 1.0 } else { 0.0 });
                click(&mut i, rects.get("matpreview"));
                self.phase += 1;
                self.wait = 2;
            }
            34 => {
                let leaf_props = texts.iter().any(|t| t == "Alpha cutoff");
                self.snap.insert("matprops".into(), if leaf_props { 1.0 } else { 0.0 });
                click(&mut i, rects.get("tab:Meshes"));
                self.phase += 1;
                self.wait = 3;
            }
            35 => {
                let mesh_ok = rects.contains_key("meshcanvas");
                let props_ok = rects.keys().any(|k| k.starts_with("hdr:"));
                let groups = texts.iter().any(|t| t == "Properties");
                let stats = texts.iter().any(|t| t.starts_with("Triangles:"));
                let tabs = ["tab:Generation", "tab:Materials", "tab:Meshes"].iter().all(|k| rects.contains_key(*k));
                let ok = mesh_ok && props_ok && groups && stats && tabs && self.snap["mat_ok"] > 0.5 && self.snap["matprops"] > 0.5;
                self.hud = Some((
                    ok,
                    format!(
                        "properties panel with header: {}, triangle/leaf statistics: {}, tabs Generation/Materials/Meshes: {}, material picker, preview, Settings/Meshes tabs and texture slots: {}, material properties shown on click: {}, mesh cutout editor: {}",
                        props_ok, stats, tabs, self.snap["mat_ok"] > 0.5, self.snap["matprops"] > 0.5, mesh_ok
                    ),
                ));
                self.snap.insert("meshes0".into(), app.meshes.len() as f32);
                // open the Meshes dialog with the +/- button
                click(&mut i, rects.get("btn:mesh.addremove"));
                self.phase = 36;
                self.wait = 2;
            }
            36 => {
                self.snap.insert("dlg_open".into(), if rects.contains_key("dlg:Meshes") { 1.0 } else { 0.0 });
                click(&mut i, rects.get("btn:dlg.addnew"));
                self.phase = 37;
                self.wait = 2;
            }
            37 => {
                click(&mut i, rects.get("btn:dlg.duplicate"));
                self.phase = 38;
                self.wait = 2;
            }
            38 => {
                // double-click the duplicate's name to rename it
                if click(&mut i, rects.get("dlg.row:Mesh Copy")) {
                    i.l_double = true;
                }
                self.phase = 39;
                self.wait = 2;
            }
            39 => {
                // clear the old name first: the editor starts with it
                self.phase = 41;
                self.wait = 0;
                let n = "Mesh Copy".chars().count();
                self.snap.insert("backspaces".into(), n as f32);
            }
            41 => {
                let left = self.snap["backspaces"];
                if left > 0.0 {
                    i.keys.push(KeyCode::Backspace);
                    self.snap.insert("backspaces".into(), left - 1.0);
                } else {
                    i.chars = "Twig".chars().collect();
                    self.phase = 42;
                }
            }
            42 => {
                i.keys.push(KeyCode::Enter);
                self.phase = 43;
                self.wait = 2;
            }
            43 => {
                click(&mut i, rects.get("btn:dlg.up"));
                self.phase = 44;
                self.wait = 2;
            }
            44 => {
                click(&mut i, rects.get("btn:dlg.ok"));
                self.phase = 45;
                self.wait = 3;
            }
            45 => {
                let names: Vec<String> = app.meshes.iter().map(|m| m.name.clone()).collect();
                let n0 = self.snap["meshes0"] as usize;
                let dlg_ok = self.snap["dlg_open"] > 0.5
                    && app.mesh_dialog.is_none()
                    && names.len() == n0 + 2
                    && names.get(n0).map(|s| s == "Twig").unwrap_or(false)
                    && names.get(n0 + 1).map(|s| s == "Mesh").unwrap_or(false)
                    // a mesh made with Add new starts empty (no levels, blank preview)
                    && meshio::processed_geom(&app.meshes, n0 + 1, 0).is_none();
                let (ok, detail) = self.hud.take().unwrap_or((false, String::new()));
                self.steps.push((
                    "hud".into(),
                    ok && dlg_ok,
                    format!(
                        "{}, Meshes dialog from +/-: opened {}, Add new + Duplicate + rename by double-click + move up + OK gave {:?}, new mesh empty: {}",
                        detail,
                        self.snap["dlg_open"] > 0.5,
                        names,
                        meshio::processed_geom(&app.meshes, self.snap["meshes0"] as usize + 1, 0).is_none()
                    ),
                ));
                self.phase = 40;
            }
            _ => {
                return (i, true);
            }
        }
        (i, false)
    }

    fn write(&self) {
        let mut s = String::from("{\"version\": 1, \"steps\": [");
        for (k, (name, ok, detail)) in self.steps.iter().enumerate() {
            if k > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{{\"step\": \"{}\", \"passed\": {}, \"detail\": \"{}\"}}", name, ok, detail.replace('"', "'")));
        }
        s.push_str("]}\n");
        let _ = std::fs::write(&self.path, s);
    }
}

fn save_screenshot(path: &str) {
    let img = get_screen_data();
    let w = img.width as usize;
    let h = img.height as usize;
    let mut flipped = vec![0u8; w * h * 4];
    for y in 0..h {
        let src = &img.bytes[(h - 1 - y) * w * 4..(h - y) * w * 4];
        flipped[y * w * 4..(y + 1) * w * 4].copy_from_slice(src);
    }
    for p in flipped.chunks_mut(4) {
        p[3] = 255;
    }
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = image::save_buffer(path, &flipped, w as u32, h as u32, image::ColorType::Rgba8);
}

#[macroquad::main(window_conf)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut shot: Option<String> = None;
    let mut test: Option<String> = None;
    let mut k = 1;
    while k < args.len() {
        if args[k] == "--screenshot" && k + 1 < args.len() {
            shot = Some(args[k + 1].clone());
            k += 1;
        } else if args[k] == "--self-test" && k + 1 < args.len() {
            test = Some(args[k + 1].clone());
            k += 1;
        }
        k += 1;
    }
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/InterVariable.ttf")).ok();
    let mut ui = Ui::new(font);
    let mut app = App::new();
    // optional start state for checking pictures: TM_TAB=0..2, TM_SEL=node name | mat:N | mesh:N
    if let Ok(t) = std::env::var("TM_TAB") {
        app.tab = t.parse().unwrap_or(0).min(2);
    }
    if let Ok(s) = std::env::var("TM_SEL") {
        if let Some(n) = s.strip_prefix("mat:") {
            app.select(Sel::Material(n.parse().unwrap_or(0)));
        } else if let Some(n) = s.strip_prefix("mesh:") {
            app.select(Sel::Mesh(n.parse().unwrap_or(0)));
        } else if let Some(id) = app.nodes.iter().find(|n| n.name == s).map(|n| n.id) {
            app.select(Sel::Node(id));
        }
    }
    if std::env::var("TM_NEW").is_ok() {
        app.new_tree();
    }
    if let Ok(t) = std::env::var("TM_MATTAB") {
        app.mat_subtab = t.parse().unwrap_or(0).min(1);
    }
    if std::env::var("TM_DIALOG").as_deref() == Ok("cut") {
        app.open_cutout_editor(0);
        // TM_CUTTOOL=move|rotate|scale: start with a card and a transform tool (screenshots)
        if let Ok(t) = std::env::var("TM_CUTTOOL") {
            if let Some(ed) = app.cutout.as_mut() {
                ed.test_transform_tool(&t);
            }
        }
    } else if std::env::var("TM_DIALOG").as_deref() == Ok("mat") {
        app.open_material_dialog(screen_width() / ui_factor(), screen_height() / ui_factor());
    } else if std::env::var("TM_DIALOG").is_ok() {
        app.open_mesh_dialog(screen_width() / ui_factor(), screen_height() / ui_factor());
    }
    let mut real = RealInput::new();
    let mut st = test.map(|p| SelfTest { path: p, frame: 0, steps: vec![], snap: HashMap::new(), phase: 0, wait: 20, hud: None });
    let mut frames = 0usize;
    loop {
        let scale = ui_factor();
        let rects = ui.rects.clone();
        let texts = ui.texts.clone();
        let inp = match st.as_mut() {
            Some(t) => {
                let (i, done) = t.input(&mut app, &rects, &texts);
                if done || t.frame > 2000 {
                    t.write();
                    break;
                }
                i
            }
            None => real.read(scale),
        };
        if !frame(&mut app, &mut ui, inp) {
            break;
        }
        frames += 1;
        if frames == 5 && std::env::var("TM_DEBUG").is_ok() {
            eprintln!("screen {}x{} dpi {} scale {}", screen_width(), screen_height(), screen_dpi_scale(), ui_scale());
        }
        if let Some(p) = &shot {
            if frames == 120 {
                save_screenshot(p);
                break;
            }
        }
        next_frame().await;
    }
}
