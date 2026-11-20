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
mod undo;
mod prefs;
mod templates;
mod stprops;
mod stpanel;
mod treefile;
mod appmenu;
mod icon_art;
mod winstate;
mod editmodes;
mod viewnav;
mod colorwheel;
mod stcurves;
mod splash;
mod wintitle;
mod winprops;
mod cameras;
mod matexport;

use app::{App, Sel};
use macroquad::prelude::*;
use model::Kind;
use std::collections::HashMap;
use ui::*;

/// Physical pixels per UI unit: the display's scaling, but never squeezing the layout
/// below ~880 units tall.
/// Preferences > General > Display scaling (picture 45), as f32 bits.
static DISPLAY_SCALING: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3f80_0000);

fn ui_scale() -> f32 {
    let dpi = screen_dpi_scale().max(0.5);
    let phys_h = screen_height() * dpi;
    let k = f32::from_bits(DISPLAY_SCALING.load(std::sync::atomic::Ordering::Relaxed)).clamp(0.5, 2.5);
    (dpi.min(phys_h / 880.0).max(0.6) * k).max(0.3)
}

/// Macroquad units (physical / dpi) per UI unit.
fn ui_factor() -> f32 {
    ui_scale() / screen_dpi_scale().max(0.5)
}

/// A scripted run (--screenshot / --self-test) keeps the fixed 1600x900 window.
fn scripted_run() -> bool {
    std::env::args().any(|a| a == "--screenshot" || a == "--self-test")
}

fn window_icon() -> miniquad::conf::Icon {
    let fill = |dst: &mut [u8], size: u32| dst.copy_from_slice(&icon_art::render(size));
    let mut icon = miniquad::conf::Icon { small: [0; 16 * 16 * 4], medium: [0; 32 * 32 * 4], big: [0; 64 * 64 * 4] };
    fill(&mut icon.small, 16);
    fill(&mut icon.medium, 32);
    fill(&mut icon.big, 64);
    icon
}

fn window_conf() -> macroquad::conf::Conf {
    // the size the window had when it was last closed un-maximized (picture 23)
    let p = prefs::Prefs::load();
    let (w, h) = if !scripted_run() && !p.window_max && p.window_w >= 400 && p.window_h >= 300 { (p.window_w, p.window_h) } else { (1600, 900) };
    macroquad::conf::Conf {
        miniquad_conf: miniquad::conf::Conf {
            window_title: "TreeCraft".to_string(),
            window_width: w,
            window_height: h,
            high_dpi: true,
            window_resizable: true,
            // Preferences > Render > Multi-sampling (applies when TreeCraft starts; picture 45)
            sample_count: [1, 2, 4, 8][p.msaa.min(3)],
            icon: Some(window_icon()),
            ..Default::default()
        },
        draw_call_vertex_capacity: 32000,
        draw_call_index_capacity: 120000,
        ..Default::default()
    }
}

use appmenu::menu_bar;

fn keyboard(app: &mut App, ui: &Ui) {
    // no shortcuts while a menu is open or a name/text is being typed
    if ui.popup.is_some() || app.mat_text_focus || app.mesh_editing || ui.editing() {
        return;
    }
    let i = &ui.inp;
    let sel_node = if let Sel::Node(id) = app.sel { Some(id) } else { None };
    if i.key(KeyCode::F) {
        app.frame_tree();
    }
    // F10: Export material (the original's hot key; picture 53)
    if i.key(KeyCode::F10) {
        app.open_material_export();
    }
    if i.key(KeyCode::Key0) || i.key(KeyCode::Kp0) {
        app.cam.set_view(0);
    }
    // views on the number pad
    if i.key(KeyCode::Kp1) {
        app.cam.set_view(if i.ctrl { 2 } else { 1 });
    }
    if i.key(KeyCode::Kp3) {
        app.cam.set_view(if i.ctrl { 3 } else { 4 });
    }
    if i.key(KeyCode::Kp7) {
        app.cam.set_view(if i.ctrl { 6 } else { 5 });
    }
    if i.key(KeyCode::Kp5) {
        app.cam.ortho = !app.cam.ortho;
    }
    // the original's toolbar hot keys (fns 0x1405325b0, 0x140537430): Show > Leaves 1,
    // Fronds 2, Branches 3, Zones 4, Targets 5, Proxies 9; Visibility > Toggle selected H,
    // Toggle isolation I; Zoom Z; 6 / 7 / 8 toggle forces, collision objects and wind
    for (k, show_index) in [(KeyCode::Key1, 0), (KeyCode::Key2, 1), (KeyCode::Key3, 2), (KeyCode::Key4, 3), (KeyCode::Key5, 4), (KeyCode::Key9, 5)] {
        if i.key(k) && !i.ctrl && !i.alt {
            app.toolbar_command("vt.show", &[show_index]);
        }
    }
    for (k, key, item) in [(KeyCode::Key6, "vt.forces", 2), (KeyCode::Key7, "vt.coll", 2), (KeyCode::Key8, "vt.wind", 0)] {
        if i.key(k) && !i.ctrl && !i.alt {
            app.toolbar_command(key, &[item]);
        }
    }
    // O toggles ambient occlusion; Alt+S steps to the next season (the original's hot keys)
    if i.key(KeyCode::O) && !i.ctrl {
        app.ao_on = !app.ao_on;
    }
    if i.key(KeyCode::S) && i.alt {
        app.season = if app.season >= 1.0 { -1.0 } else if app.season < 0.0 { 0.0 } else { (app.season + 0.2).min(1.0) };
    }
    if i.key(KeyCode::H) && !i.ctrl {
        app.toolbar_command("vt.vis", &[0]);
    }
    if i.key(KeyCode::I) && !i.ctrl {
        app.toolbar_command("vt.vis", &[1]);
    }
    if i.key(KeyCode::Z) && !i.ctrl && !i.alt {
        app.toolbar_command("vt.zoom", &[0]);
    }
    // File (the original's hot keys, fn 0x140294d60): Ctrl+N new, Ctrl+O open, Ctrl+W close,
    // Ctrl+S save, Ctrl+Shift+S save as, Ctrl+Alt+Shift+S save with assets, Ctrl+E export mesh
    if i.ctrl && i.key(KeyCode::S) {
        if i.alt && i.shift {
            app.save_with_assets();
        } else {
            app.save_tree(i.shift);
        }
    }
    if i.ctrl && i.key(KeyCode::O) {
        app.open_tree();
    }
    if i.ctrl && (i.key(KeyCode::N) || i.key(KeyCode::W)) {
        app.file_new();
    }
    if i.ctrl && i.key(KeyCode::E) {
        app.export_obj();
    }
    // Edit: Alt+Z / Alt+Y undo and redo the selection
    if i.alt && !i.ctrl && i.key(KeyCode::Z) {
        app.undo_selection();
    }
    if i.alt && !i.ctrl && i.key(KeyCode::Y) {
        app.redo_selection();
    }
    // Undo / Redo (no limit): Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z
    if i.ctrl && i.key(KeyCode::Z) {
        if i.shift {
            app.redo();
        } else {
            app.undo();
        }
    }
    if i.ctrl && i.key(KeyCode::Y) {
        app.redo();
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
    // Tab: Generator / Node edit mode (the original's hot key)
    if i.key(KeyCode::Tab) && !i.ctrl {
        app.set_edit_mode(if app.edit_mode == 1 { 0 } else { 1 });
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
    // the original's preference tabs that work every frame (picture 45)
    DISPLAY_SCALING.store(app.prefs.display_scaling.to_bits(), std::sync::atomic::Ordering::Relaxed);
    ui.plain_numbers = !app.prefs.use_sliders;
    textures::set_max_texture(512 << app.prefs.max_tex.min(4));
    let last_dir = app.tree_path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())).or_else(|| app.prefs.recent.first().and_then(|r| std::path::Path::new(r).parent().map(|d| d.to_path_buf())));
    prefs::set_dialog_state(app.prefs.native_dialogs, last_dir);
    if !scripted_run() {
        app.autosave_tick();
    }
    // the title bar: the scene's name and path (picture 49), checked twice a second
    if get_time() - app.title_t > 0.5 {
        app.title_t = get_time();
        let t = wintitle::title_for(app.tree_path.as_deref(), app.is_modified());
        if t != app.title_shown && wintitle::set(&t) {
            app.title_shown = t;
        }
    }
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
    let dialog_input = if app.mesh_dialog.is_some() || app.mat_dialog.is_some() || app.cutout.is_some() || app.prefs_open || app.curve_takes_input(ui.inp.mx, ui.inp.my) || app.var_takes_input(ui.inp.mx, ui.inp.my) || app.welcome_takes_input() || app.material_export_takes_input() {
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
        let (on_left, on_right) = (on_left && app.layout_mode != 2, on_right && app.layout_mode != 2);
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
    let mut lay = layout(w, h, app.left_w, app.dock_w);
    // Window > Layout > Full screen no bars: the viewport takes the whole window
    let bars = app.layout_mode != 2;
    if !bars {
        lay.view = Rect::new(2.0, 26.0, w - 4.0, h - 28.0);
    }
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
    // a looked-through custom camera sets the view (picture 50)
    app.update_camera_view();
    // Export material: the next picture, rendered off screen
    app.run_material_export();
    app.draw_viewport_bg(lay.view);
    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.pop_model_matrix();
    app.draw_viewport_3d(lay.view, scale, screen_height() * dpi);
    // Selection > Outline: the ring around the selected generator (picture 30)
    app.draw_selection_outline(lay.view, f);

    let gl = unsafe { get_internal_gl() };
    gl.quad_gl.push_model_matrix(Mat4::from_scale(vec3(f, f, 1.0)));

    let quit = menu_bar(app, ui, w);
    if bars {
        props::draw_panel(app, ui, lay.left);
        app.draw_dock(ui, lay.dock);
    }
    // a Render photo hides the viewport's own controls for its picture
    if !app.shot_queue.first().map_or(false, |s| s.2) {
        app.draw_viewport_overlay(ui, lay.view);
    }
    // Node and Freehand modes take viewport clicks first (picture 24)
    if app.edit_mode_input(ui, lay.view) {
        ui.inp.l_pressed = false;
        ui.inp.l_down = false;
    }
    app.draw_edit_overlay();
    app.viewport_pick(ui, lay.view);
    app.viewport_input(ui, lay.view);
    keyboard(app, ui);
    if let Some(real) = dialog_input {
        ui.inp = real;
    }
    app.draw_mesh_dialog(ui);
    app.draw_cutout_window(ui);
    app.draw_prefs_window(ui);
    app.draw_curve_window(ui);
    app.draw_variance_window(ui);
    app.draw_welcome(ui);
    // File > Export material (F10; picture 53)
    app.draw_material_export(ui);
    // undo history: keep the state after each finished edit (outside the modal windows)
    let idle = !ui.inp.l_down && ui.active == 0 && app.cutout.is_none() && app.mesh_dialog.is_none() && app.mat_dialog.is_none() && !app.prefs_open && app.curve_drag.is_none() && !app.var_cohesion_drag && app.welcome.is_none();
    app.undo_track(idle);
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
                let key = rects.keys().find(|k| k.starts_with("sl:n") && (k.ends_with(".frequency") || k.ends_with(".Generation_Frequency"))).cloned();
                // press on the spinner dial (picture 32), then drag it to the right
                if let Some(r) = key.and_then(|k| rects.get(&k).cloned()) {
                    i.mx = r.x + 20.0;
                    i.my = r.y + r.h * 0.5;
                    i.l_pressed = true;
                    i.l_down = true;
                    self.snap.insert("dial_x".into(), i.mx);
                    self.snap.insert("dial_y".into(), i.my);
                }
                self.phase += 1;
            }
            28 => {
                if !self.snap.contains_key("dragged") {
                    i.mx = self.snap.get("dial_x").copied().unwrap_or(0.0) + 90.0;
                    i.my = self.snap.get("dial_y").copied().unwrap_or(0.0);
                    i.l_down = true;
                    self.snap.insert("dragged".into(), 1.0);
                } else {
                    i.l_released = true;
                    self.phase += 1;
                    self.wait = 2;
                }
            }
            29 => {
                click(&mut i, rects.get("btn:gen.add"));
                self.phase += 1;
                self.wait = 1;
            }
            30 => {
                click(&mut i, rects.get("pop:Leaf Mesh"));
                self.phase += 1;
                self.wait = 2;
            }
            31 => {
                // Orthographic now sits in the view menu below the toolbar: open it, then pick it
                if let Some(r) = rects.get("pop:Orthographic") {
                    click(&mut i, Some(r));
                    self.phase += 1;
                    self.wait = 2;
                } else {
                    click(&mut i, rects.get("btn:vp.view"));
                    self.wait = 2;
                }
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
                // the editor starts with the whole name selected (picture 48): typing replaces it
                self.phase = 41;
                self.wait = 0;
                self.snap.insert("backspaces".into(), 0.0);
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
    save_region(path, None);
}

/// Save the screen (or `region`: a rectangle in UI units and the window's width in UI units)
/// as a PNG.
fn save_region(path: &str, region: Option<(Rect, f32)>) -> bool {
    let img = get_screen_data();
    let w = img.width as usize;
    let h = img.height as usize;
    let r = match region {
        Some((r, ui_w)) => {
            let k = w as f32 / ui_w.max(1.0);
            Rect::new(r.x * k, r.y * k, r.w * k, r.h * k)
        }
        None => Rect::new(0.0, 0.0, w as f32, h as f32),
    };
    let x0 = (r.x.max(0.0) as usize).min(w.saturating_sub(1));
    let y0 = (r.y.max(0.0) as usize).min(h.saturating_sub(1));
    let rw = (r.w as usize).clamp(1, w - x0);
    let rh = (r.h as usize).clamp(1, h - y0);
    let mut out = vec![0u8; rw * rh * 4];
    for y in 0..rh {
        // the screen data runs bottom-up
        let sy = h - 1 - (y0 + y);
        let src = &img.bytes[(sy * w + x0) * 4..(sy * w + x0 + rw) * 4];
        out[y * rw * 4..(y + 1) * rw * 4].copy_from_slice(src);
    }
    for p in out.chunks_mut(4) {
        p[3] = 255;
    }
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    image::save_buffer(path, &out, rw as u32, rh as u32, image::ColorType::Rgba8).is_ok()
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
    // the program starts empty; the self-test (and TM_DEMO=1 / TM_EXTRAS=1 screenshots) start
    // from a small demo tree
    if test.is_some() || std::env::var("TM_DEMO").as_deref() == Ok("1") || std::env::var("TM_EXTRAS").as_deref() == Ok("1") {
        app.load_demo();
    }
    // optional start state for checking pictures: TM_TAB=0..2, TM_SEL=node name | mat:N | mesh:N
    if let Ok(t) = std::env::var("TM_TAB") {
        app.tab = t.parse().unwrap_or(0).min(2);
    }
    if let Ok(s) = std::env::var("TM_SEL") {
        // TM_SEL=light: the light's settings in Properties; TM_HELP=1 opens the tips card (picture 43)
        if s == "light" {
            app.select(Sel::Light);
        }
        // TM_SEL=matexport: the Export material window open (picture 53); with
        // TM_MATEXPORT_DIR=<folder> its pictures (512 px) are rendered there at once
        if s == "matexport" {
            app.open_material_export();
            // TM_MATEXPORT_CAM=1: render from a camera added where the view is (picture 55)
            if std::env::var("TM_MATEXPORT_CAM").as_deref() == Ok("1") {
                app.add_camera();
                let name = app.cameras.last().map(|c| c.name.clone()).unwrap_or_default();
                if let Some(d) = app.mat_export.as_mut() {
                    d.camera = name;
                }
                app.cam.yaw += 1.0;
            }
            if let Ok(dir) = std::env::var("TM_MATEXPORT_DIR") {
                app.test_material_export(std::path::PathBuf::from(dir), 512);
            }
        }
        // TM_SEL=camera: a custom camera added where the view is and selected (picture 50);
        // TM_SEL=camlook: also looked through
        if s == "camera" || s == "camlook" {
            app.add_camera();
            if let Some(c) = app.cameras.last_mut() {
                // moved aside so its icon shows in the view
                c.dof = true;
            }
            // the view steps back and turns so the new camera's icon shows
            if s == "camera" {
                if let Some(c) = app.cameras.last_mut() {
                    c.focus_dist = c.dist * 0.8;
                    app.cam.target = (c.pos + app.cam.target) * 0.5;
                }
                app.cam.yaw += 1.3;
                app.cam.dist *= 1.3;
            }
            if s == "camlook" {
                app.look_through(Some(app.cameras.len() - 1));
            }
        }
        // TM_SEL=window: Window properties (picture 49); TM_SAFEFRAME=1 shows the safe frame
        if s == "window" {
            app.select(Sel::Window);
            if std::env::var("TM_SAFEFRAME").as_deref() == Ok("1") {
                app.winp.insert("Screenshot Safe Frame:Show".into(), 1.0);
                app.winp.insert("Screenshot Safe Frame:Width".into(), 1920.0);
                app.winp.insert("Screenshot Safe Frame:Height".into(), 1080.0);
            }
        }
        if std::env::var("TM_HELP").as_deref() == Ok("1") {
            app.help_open = true;
        }
        if let Some(n) = s.strip_prefix("mat:") {
            app.select(Sel::Material(n.parse().unwrap_or(0)));
        } else if let Some(n) = s.strip_prefix("mesh:") {
            app.select(Sel::Mesh(n.parse().unwrap_or(0)));
        } else if let Some(id) = app.nodes.iter().find(|n| n.name == s).map(|n| n.id) {
            app.select(Sel::Node(id));
        }
    }
    // TM_WELD=<node name>: Welding on for that generator, with the spread of picture 57, and the
    // view close on the trunk (pictures)
    if let Ok(s) = std::env::var("TM_WELD") {
        if let Some(n) = app.nodes.iter_mut().find(|n| n.name == s) {
            for (k, v) in [("Skin:Welding:Enabled", 1.0), ("Skin:Welding:Spread:Amount", 0.27), ("Skin:Welding:Spread:Upper scale", 0.58), ("Skin:Welding:Spread:Lower scale", 0.4)] {
                n.p.extra.insert(k.into(), v);
            }
        }
        // TM_WELDNEAR=1: closer, on one joint (picture 58)
        let near = std::env::var("TM_WELDNEAR").as_deref() == Ok("1");
        app.cam.target = if near { vec3(0.0, 6.5, 0.0) } else { vec3(0.0, 9.0, 0.0) };
        app.cam.dist = if near { 9.0 } else { 22.0 };
        app.cam.pitch = 0.1;
        app.dirty = true;
    }
    // TM_SPINEONLY=<node name>: that generator's Skin:Type set to Spine only (pictures)
    if let Ok(s) = std::env::var("TM_SPINEONLY") {
        if let Some(n) = app.nodes.iter_mut().find(|n| n.name == s) {
            n.p.extra.insert("Skin:Type".into(), 3.0);
            // TM_SEGPROFILE=1: a falling Profile curve on Segments:Length:Absolute (more
            // segments at the base, picture 40)
            if std::env::var("TM_SEGPROFILE").as_deref() == Ok("1") {
                n.p.curves.insert("Segments:Length:Absolute|2".into(), model::Curve { pts: vec![vec2(0.0, 1.0), vec2(1.0, 0.0)], kinds: vec![] });
            }
            app.dirty = true;
        }
    }
    // TM_RADIALPROFILE=<node>: a Linear decay second curve on its Segments:Radial:Absolute;
    // TM_NODESTYLE=<n>: the Generation editor's node style; TM_PREFSPAGE=<n>: the Preferences page
    if let Ok(s) = std::env::var("TM_RADIALPROFILE") {
        if let Some(n) = app.nodes.iter_mut().find(|n| n.name == s) {
            n.p.curves.insert("Segments:Radial:Absolute|2".into(), editmodes::preset_curve(4));
            app.dirty = true;
        }
    }
    if let Some(k) = std::env::var("TM_NODESTYLE").ok().and_then(|s| s.parse::<usize>().ok()) {
        app.prefs.node_style = k.min(app::NODE_STYLES.len() - 1);
    }
    if let Some(k) = std::env::var("TM_PREFSPAGE").ok().and_then(|s| s.parse::<usize>().ok()) {
        app.prefs_page = k;
    }
    // TM_FHBEND=<node>: Freehand mode on that generator, bent 35 degrees (picture 46)
    if let Ok(s) = std::env::var("TM_FHBEND") {
        if let Some(n) = app.nodes.iter_mut().find(|n| n.name == s) {
            for (k, v) in [("Angle", 35.0), ("AxisX", 1.0), ("AxisY", 0.0), ("AxisZ", 0.0), ("Lower", 0.35), ("Upper", 0.8)] {
                n.p.extra.insert(format!("Freehand:{}", k), v);
            }
            // TM_FHKEPT=1: an earlier bend near the top, kept (picture 47)
            if std::env::var("TM_FHKEPT").as_deref() == Ok("1") {
                for (k, v) in [("Angle", 50.0), ("AxisX", 0.0), ("AxisY", 0.0), ("AxisZ", 1.0), ("Lower", 0.7), ("Upper", 0.95), ("Shape", 1.0)] {
                    n.p.extra.insert(format!("Freehand:Kept0:{}", k), v);
                }
                n.p.extra.insert("Freehand:KeptCount".into(), 1.0);
                n.p.extra.insert("Freehand:Lower".into(), 0.2);
                n.p.extra.insert("Freehand:Upper".into(), 0.45);
            }
            let id = n.id;
            app.select(Sel::Node(id));
            app.edit_mode = 2;
            app.dirty = true;
        }
    }
    // TM_THEMETAB=1: Themes > Node Editor (picture 44)
    if let Some(k) = std::env::var("TM_THEMETAB").ok().and_then(|s| s.parse::<usize>().ok()) {
        app.prefs_theme_tab = k.min(1);
    }
    if std::env::var("TM_PREFS").as_deref() == Ok("1") {
        app.prefs_open = true;
        app.prefs_pick = Some(0);
    }
    if std::env::var("TM_NEW").is_ok() {
        app.new_tree();
    }
    // TM_SHADING=0|1|2 (Studio, MatCap, Flat), TM_GIZMO=<style>, TM_SHADEMENU=1: for pictures
    if let Some(s) = std::env::var("TM_SHADING").ok().and_then(|s| s.parse::<u8>().ok()) {
        app.shading = s.min(2);
    }
    if let Some(g) = std::env::var("TM_GIZMO").ok().and_then(|s| s.parse::<usize>().ok()) {
        app.prefs.gizmo_style = g.min(prefs::GIZMO_STYLES.len() - 1);
    }
    if std::env::var("TM_SHADEMENU").as_deref() == Ok("1") {
        app.shading_open = true;
    }
    // TM_SELMODE=0|1|2 (Solid fill, Outline, Both), TM_SELMENU=1: for pictures
    if let Some(s) = std::env::var("TM_SELMODE").ok().and_then(|s| s.parse::<usize>().ok()) {
        app.prefs.sel_mode = s.min(2);
    }
    if std::env::var("TM_SELMENU").as_deref() == Ok("1") {
        app.sel_open = true;
        // TM_WHEEL=1: with the colour wheel open on the fill colour
        if std::env::var("TM_WHEEL").as_deref() == Ok("1") {
            let c = app.prefs.sel_fill;
            app.wheel = Some(colorwheel::Wheel::new("sel.fill", vec2(560.0, 210.0), [c[0], c[1], c[2]]));
        }
    }
    // TM_CURVE=1: the curve editor open on the selected generator's Spine:Length:Absolute parent
    // curve, set to the Bell preset (for pictures)
    if std::env::var("TM_CURVE").as_deref() == Ok("1") {
        if let Sel::Node(id) = app.sel {
            let prop = "Spine:Length:Absolute".to_string();
            if let Some(n) = app.nodes.iter_mut().find(|n| n.id == id) {
                n.p.curves.insert(format!("{}|0", prop), editmodes::preset_curve(10));
            }
            app.curve_edit = Some((id, prop, 0));
            app.curve_pin = true;
            app.dirty = true;
        }
    }
    // TM_VAR=1: Variance Settings open for the selected generator's Spine:Shape:Gravity
    if std::env::var("TM_VAR").as_deref() == Ok("1") {
        if let Sel::Node(id) = app.sel {
            app.var_edit = Some((id, "Spine:Shape:Gravity".into(), Rect::new(300.0, 640.0, 26.0, 16.0)));
            app.var_just_opened = true;
        }
    }
    // TM_PTAB=<n>: the branch Properties tab to start on (5 = UV), for pictures
    if let Ok(t) = std::env::var("TM_PTAB") {
        app.prop_tab = t.parse().unwrap_or(0);
    }
    // TM_SUBTAB=<n>: the sub-tab inside the Skin tab (picture 61), for pictures
    if let Ok(t) = std::env::var("TM_SUBTAB") {
        app.prop_sub_tabs.insert("branch:Skin".into(), t.parse().unwrap_or(0));
    }
    if let Ok(t) = std::env::var("TM_MATTAB") {
        app.mat_subtab = t.parse().unwrap_or(0).min(1);
    }
    // TM_RENDER=<n>: start in render mode n (screenshots); TM_EXTRAS=1: bones, wind, a force
    if let Some(n) = std::env::var("TM_RENDER").ok().and_then(|s| s.parse::<f32>().ok()) {
        app.light.mode = n;
    }
    if std::env::var("TM_EXTRAS").as_deref() == Ok("1") {
        app.show_bones = true;
        app.season = 0.8;
        app.scene.push(model::SceneObj { kind: model::SceneKind::Sphere, name: "Sphere 1".into(), pos: vec3(0.0, 8.0, 0.0), dir: Vec3::Y, size: 3.0, length: 0.0 });
        app.scene.push(model::SceneObj { kind: model::SceneKind::Direction, name: "Direction 1".into(), pos: vec3(10.0, 12.0, 0.0), dir: vec3(-1.0, 0.0, 0.0), size: 0.6, length: 0.0 });
        app.dirty = true;
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
    // closing the window with unsaved changes asks to save first (not in scripted runs)
    let interactive = st.is_none() && shot.is_none();
    if interactive {
        prevent_quit();
    }
    // opens maximized unless it was last closed at a smaller size (picture 23)
    let mut maximized = !interactive || !app.prefs.window_max;
    // the welcome window over the loaded app (picture 37), and TreeCraft's Start-menu shortcut
    // with its icon. TM_SPLASH=1 with --screenshot: a picture of the welcome window.
    // (TM_NOWELCOME=1: no welcome window, for pictures of the viewport)
    if interactive && std::env::var("TM_NOWELCOME").is_err() {
        splash::ensure_start_menu_shortcut();
        app.welcome = Some(get_time());
    } else if shot.is_some() && std::env::var("TM_SPLASH").as_deref() == Ok("1") {
        app.welcome = Some(get_time() - 2.0);
    }
    loop {
        if !maximized {
            maximized = winstate::maximize();
        }
        if interactive && is_quit_requested() && app.confirm_discard() {
            winstate::remember(&mut app.prefs);
            app.keep_session();
            break;
        }
        // File > Export image sequence turns the camera for each picture
        if let Some((_, Some(yaw), _)) = app.shot_queue.first() {
            app.cam.yaw = *yaw;
        }
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
        if !frame(&mut app, &mut ui, inp) && (!interactive || app.confirm_discard()) {
            if interactive {
                winstate::remember(&mut app.prefs);
            app.keep_session();
            }
            break;
        }
        frames += 1;
        if !app.shot_queue.is_empty() {
            let (path, _, _) = app.shot_queue.remove(0);
            if !path.as_os_str().is_empty() {
                let v = app.viewport;
                let ok = save_region(&path.display().to_string(), Some((v, screen_width() / scale)));
                app.set_status(&if ok { format!("Saved {}", path.display()) } else { format!("Couldn't save {}", path.display()) });
            }
        }
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
