//! The Edit Cutout window ("Edit Mesh..."), laid out like the original's (fn 0x1402fc9c0 titles it
//! "Edit Cutout - <mesh>", fn 0x1402f32a0 names its parts): Points, Anchors, Material, Options,
//! the cutout canvas, and High/Med/Low with their arrows. It stays open until Confirm or Cancel.
//! The original's Mask and Color groups ("Paint mask to remove triangles") aren't built: outlines
//! here are kept simple (they can't cross themselves), so concave shapes need no mask.

use crate::app::{App, Sel};
use crate::meshio::{self, CutOpts};
use crate::model::*;
use crate::ui::*;
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    AddPoints,
    RemovePoints,
    AddAnchors,
    RemoveAnchors,
    /// Transform the whole card (points, pivot and anchors) as one island.
    Move,
    Rotate,
    Scale,
    /// Paint vertex colours onto the points (the original's Color group).
    PaintColor,
}

/// A transform drag in progress: start mouse (picture space), island center, and the state
/// before the drag (outline, anchors, anchor angles, pivot angle).
struct Xform {
    start: Vec2,
    center: Vec2,
    outline: Vec<Vec2>,
    anchors: Vec<Vec2>,
    angles: Vec<f32>,
    pivot_angle: f32,
    /// which gizmo handle was grabbed: 0 free (centre or anywhere), 1 X only, 2 Y only
    axis: u8,
}

/// Length of the transform gizmo's handles and radius of its rotate ring, in screen pixels.
const GIZ_L: f32 = 70.0;

fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

/// The gizmo handle under the mouse: 1 = X handle, 2 = Y handle, 0 = anything else (free drag).
fn giz_part(tool: Tool, cs: Vec2, m: Vec2) -> u8 {
    if !matches!(tool, Tool::Move | Tool::Scale) || (m - cs).abs().max_element() <= 8.0 {
        return 0;
    }
    if seg_dist(m, cs, cs + vec2(GIZ_L, 0.0)) < 8.0 {
        1
    } else if seg_dist(m, cs, cs + vec2(0.0, -GIZ_L)) < 8.0 {
        2
    } else {
        0
    }
}

impl CutoutEditor {
    /// For screenshots: a diamond card and the named transform tool (move, rotate or scale).
    pub fn test_transform_tool(&mut self, t: &str) {
        self.outline = vec![vec2(0.5, 0.15), vec2(0.8, 0.5), vec2(0.5, 0.9), vec2(0.2, 0.5)];
        self.colors = vec![[1.0, 1.0, 1.0]; 4];
        if t == "paint" {
            self.colors = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 0.0]];
        }
        self.tool = match t {
            "paint" => Tool::PaintColor,
            "rotate" => Tool::Rotate,
            "scale" => Tool::Scale,
            _ => Tool::Move,
        };
    }
}

/// Where the transform gizmo sits and what Rotate/Scale turn around: the average of the card's
/// points (or of the anchors when it has none), as the original places its manipulator at the
/// mean position of the selected objects (fn 0x140570530: positions summed, then x 1/count).
fn card_center(ed: &CutoutEditor) -> Option<Vec2> {
    let pts = if ed.outline.is_empty() { &ed.anchors } else { &ed.outline };
    if pts.is_empty() {
        return None;
    }
    Some(pts.iter().copied().sum::<Vec2>() / pts.len() as f32)
}

/// Paint colours offered by the Color swatch (red first, the original's default, QColor 7).
const PAINT_COLORS: [(&str, [f32; 3]); 8] = [
    ("Red", [1.0, 0.0, 0.0]),
    ("Green", [0.0, 1.0, 0.0]),
    ("Blue", [0.0, 0.0, 1.0]),
    ("Yellow", [1.0, 1.0, 0.0]),
    ("Cyan", [0.0, 1.0, 1.0]),
    ("Magenta", [1.0, 0.0, 1.0]),
    ("Black", [0.0, 0.0, 0.0]),
    ("White", [1.0, 1.0, 1.0]),
];

pub struct CutoutEditor {
    pub mesh: usize,
    pub name: String,
    pub lods: [MeshLod; 3],
    /// The level the canvas edits (loaded with its left arrow).
    pub target: usize,
    pub outline: Vec<Vec2>,
    pub anchors: Vec<Vec2>,
    pub tool: Tool,
    /// 0 = None, k = material k-1.
    pub material: usize,
    pub show_opacity: bool,
    pub show_all: bool,
    pub opts: CutOpts,
    drag: Option<(bool, usize)>,
    warn: Option<(String, f64)>,
    opacity_tex: Option<(usize, Texture2D)>,
    /// Where the window is (moved by its title bar, resized by its corner); None = default.
    rect: Option<Rect>,
    /// (mouse at press, window at press, sides being dragged: 0 = moving, 1 left, 2 right, 4 top, 8 bottom)
    win_drag: Option<(Vec2, Rect, u8)>,
    /// The anchor the Anchor X/Y controls edit (the last one picked).
    sel_anchor: usize,
    /// Dragging the angle knob of this anchor (0 = the pivot).
    angle_drag: Option<usize>,
    /// Angle of each anchor (degrees, index 0 unused: the pivot's is opts.angle).
    pub anchor_angles: Vec<f32>,
    /// Number of each anchor (index 0 unused), set by right-clicking an anchor.
    pub anchor_ids: Vec<usize>,
    /// Side of each anchor (index 0 unused): 0 Automatic, 1 Left, 2 Right.
    pub anchor_sides: Vec<u8>,
    /// Vertex colour of each outline point.
    pub colors: Vec<[f32; 3]>,
    /// Points inside the outline that are mesh vertices too (picture-17), and their colours.
    pub inner: Vec<Vec2>,
    pub inner_colors: Vec<[f32; 3]>,
    /// The colour the paint tool puts down.
    pub paint: [f32; 3],
    xform: Option<Xform>,
    /// The anchor whose number popup is open.
    id_popup: Option<usize>,
    /// The colour picker panel, while open.
    picker: Option<Rect>,
    /// Canvas view: zoom (1 = fit) and pan (fractions of the canvas), changed with the middle
    /// mouse button as in the original (fn 0x140300880).
    view_zoom: f32,
    view_pan: Vec2,
    last_mouse: Vec2,
    /// Where the colour brush was last frame (it paints along the stroke).
    paint_prev: Option<Vec2>,
}

impl CutoutEditor {
    /// The canvas's green anchors as stored in a level.
    fn green(&self) -> Vec<CutAnchor> {
        (1..self.anchors.len())
            .map(|k| CutAnchor {
                pos: self.anchors[k],
                angle: self.anchor_angles.get(k).copied().unwrap_or(0.0),
                id: self.anchor_ids.get(k).copied().unwrap_or(1),
                side: self.anchor_sides.get(k).copied().unwrap_or(0),
            })
            .collect()
    }
    /// The canvas as a level: outline, its colours and the green anchors.
    fn as_level(&self) -> MeshLod {
        let plain = self.colors.iter().chain(&self.inner_colors).all(|c| *c == [1.0, 1.0, 1.0]);
        MeshLod {
            file: String::new(),
            outline: self.outline.clone(),
            geom: None,
            anchors: self.green(),
            colors: if plain { vec![] } else { self.colors.clone() },
            inner: self.inner.clone(),
            inner_colors: if plain { vec![] } else { self.inner_colors.clone() },
        }
    }
    /// The canvas's mesh: outline, inner points, colours and options.
    fn geom(&self) -> crate::model::MeshGeom {
        meshio::outline_geom_inner(&self.outline, &self.colors, &self.inner, &self.inner_colors, self.anchors[0], self.opts)
    }
    /// Put a level's outline, colours and green anchors on the canvas (the pivot stays).
    fn load_level(&mut self, l: usize) {
        let lv = self.lods[l].clone();
        self.outline = lv.outline.clone();
        self.colors = if lv.colors.len() == lv.outline.len() { lv.colors.clone() } else { vec![[1.0; 3]; lv.outline.len()] };
        self.inner = lv.inner.clone();
        self.inner_colors = if lv.inner_colors.len() == lv.inner.len() { lv.inner_colors.clone() } else { vec![[1.0; 3]; lv.inner.len()] };
        self.anchors.truncate(1);
        self.anchor_angles = vec![0.0];
        self.anchor_ids = vec![0];
        self.anchor_sides = vec![0];
        for a in &lv.anchors {
            self.anchors.push(a.pos);
            self.anchor_angles.push(a.angle);
            self.anchor_ids.push(a.id);
            self.anchor_sides.push(a.side);
        }
        self.sel_anchor = 0;
    }
}

const EMBED_WARNING: &str = "Editing this mesh will convert it to embedded.\nIt will no longer reference files on disk.\n\nDo you wish to continue?";

impl App {
    /// "Edit Mesh...": like the original (fn 0x1400828d0), a mesh that still references files on
    /// disk asks first and becomes embedded; then the Edit Cutout window opens.
    pub fn open_cutout_editor(&mut self, cur: usize) {
        if cur >= self.meshes.len() {
            return;
        }
        if !self.meshes[cur].embedded {
            let go = rfd::MessageDialog::new().set_title("TreeCraft").set_description(EMBED_WARNING).set_buttons(rfd::MessageButtons::YesNo).show();
            if go != rfd::MessageDialogResult::Yes {
                return;
            }
            let m = &mut self.meshes[cur];
            m.embedded = true;
            for l in &mut m.lods {
                l.file.clear(); // the geometry stays, but no longer points at a file
            }
            self.dirty = true;
        }
        let m = &self.meshes[cur];
        let target = self.mesh_lod.min(2);
        // a mesh with no cutout yet starts from a clean slate (no points), at the person's request
        let mut ed = CutoutEditor {
            mesh: cur,
            name: m.name.clone(),
            lods: m.lods.clone(),
            target,
            outline: vec![],
            anchors: vec![m.anchor],
            tool: Tool::AddPoints,
            material: if m.ref_material < self.materials.len() { m.ref_material + 1 } else { 0 },
            show_opacity: false,
            show_all: false,
            opts: CutOpts::of(m),
            drag: None,
            warn: None,
            opacity_tex: None,
            rect: self.cutout_rect,
            win_drag: None,
            sel_anchor: 0,
            angle_drag: None,
            anchor_angles: vec![0.0],
            anchor_ids: vec![0],
            anchor_sides: vec![0],
            colors: vec![],
            inner: vec![],
            inner_colors: vec![],
            paint: PAINT_COLORS[0].1,
            xform: None,
            id_popup: None,
            picker: None,
            view_zoom: 1.0,
            view_pan: Vec2::ZERO,
            last_mouse: Vec2::ZERO,
            paint_prev: None,
        };
        ed.load_level(target);
        self.cutout = Some(ed);
        self.select(Sel::Mesh(cur));
    }

    /// Draw the Edit Cutout window on top of everything while it's open (modal).
    pub fn draw_cutout_window(&mut self, ui: &mut Ui) {
        let mut ed = match self.cutout.take() {
            Some(e) => e,
            None => return,
        };
        if ed.mesh >= self.meshes.len() {
            return;
        }
        ui.blocked = false;
        let (sw, sh) = (ui.sw, ui.sh);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.45));
        // move by the title bar, resize by the corner grip, double-click the title to fill the app
        let full = Rect::new(0.0, 0.0, sw, sh);
        let mut r = ed.rect.unwrap_or(Rect::new(24.0, 20.0, sw - 48.0, sh - 40.0));
        let m = ui.mouse();
        let grip = Rect::new(r.x + r.w - 18.0, r.y + r.h - 18.0, 18.0, 18.0);
        let title_bar = Rect::new(r.x, r.y, r.w, 24.0);
        // which edges a press here would drag (1 left, 2 right, 4 top, 8 bottom): the four corners
        // (18 px) and the four sides (5 px)
        let edges = |m: Vec2| -> u8 {
            let grown = Rect::new(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0);
            if !grown.contains(m) {
                return 0;
            }
            let (c, b) = (18.0, 5.0);
            let (dl, dr, dt, db) = (m.x - r.x, r.x + r.w - m.x, m.y - r.y, r.y + r.h - m.y);
            let mut e = 0;
            if dl < b || (dl < c && (dt < c || db < c)) {
                e |= 1;
            }
            if dr < b || (dr < c && (dt < c || db < c)) {
                e |= 2;
            }
            if dt < b || (dt < c && (dl < c || dr < c)) {
                e |= 4;
            }
            if db < b || (db < c && (dl < c || dr < c)) {
                e |= 8;
            }
            e
        };
        let hover_edges = edges(m);
        if ui.active == 0 && ui.popup.is_none() && ui.inp.l_pressed {
            if hover_edges != 0 {
                ed.win_drag = Some((m, r, hover_edges));
                ui.active = hid("cutout.window");
            } else if title_bar.contains(m) {
                if ui.inp.l_double {
                    let def = Rect::new(24.0, 20.0, sw - 48.0, sh - 40.0);
                    r = if (r.w - full.w).abs() < 1.0 && (r.h - full.h).abs() < 1.0 { def } else { full };
                } else {
                    ed.win_drag = Some((m, r, 0));
                    ui.active = hid("cutout.window");
                }
            }
        }
        if let Some((start, r0, e)) = ed.win_drag {
            if ui.inp.l_down {
                let d = m - start;
                if e != 0 {
                    // the dragged sides follow the mouse; the opposite sides stay put
                    let (min_w, min_h) = (820.0f32.min(sw), 560.0f32.min(sh));
                    let (mut x0, mut y0, mut x1, mut y1) = (r0.x, r0.y, r0.x + r0.w, r0.y + r0.h);
                    if e & 1 != 0 {
                        x0 = (r0.x + d.x).clamp(0.0, x1 - min_w);
                    }
                    if e & 2 != 0 {
                        x1 = (r0.x + r0.w + d.x).clamp(x0 + min_w, sw);
                    }
                    if e & 4 != 0 {
                        y0 = (r0.y + d.y).clamp(0.0, y1 - min_h);
                    }
                    if e & 8 != 0 {
                        y1 = (r0.y + r0.h + d.y).clamp(y0 + min_h, sh);
                    }
                    r = Rect::new(x0, y0, x1 - x0, y1 - y0);
                } else {
                    r.x = r0.x + d.x;
                    r.y = r0.y + d.y;
                }
            } else {
                ed.win_drag = None;
                ui.active = 0;
            }
        }
        r.w = r.w.min(sw).max(400.0);
        r.h = r.h.min(sh).max(300.0);
        r.x = r.x.clamp(80.0 - r.w, sw - 80.0);
        r.y = r.y.clamp(0.0, sh - 24.0);
        ed.rect = Some(r);
        self.cutout_rect = Some(r);
        draw_rectangle(r.x + 4.0, r.y + 5.0, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.35));
        draw_rectangle(r.x, r.y, r.w, r.h, C_PANEL);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, C_LINE);
        draw_rectangle(r.x, r.y, r.w, 24.0, C_PANEL2);
        let title = format!("Edit Cutout - {}", ed.name);
        ui.text_center(&title, Rect::new(r.x, r.y, r.w, 24.0), 13.0, C_TEXT);
        ui.record("cutout.window", r);
        if title_bar.contains(m) && ed.win_drag.is_none() && hover_edges == 0 {
            ui.tooltip = Some("Drag to move • double-click to fill the TreeCraft window • drag any corner or side to resize".into());
        }
        // show the side(s) a drag would resize
        let show = match ed.win_drag {
            Some((_, _, e)) => e,
            None if ui.active == 0 => hover_edges,
            None => 0,
        };
        if show != 0 {
            let c = C_ACCENT;
            if show & 1 != 0 {
                draw_line(r.x, r.y, r.x, r.y + r.h, 3.0, c);
            }
            if show & 2 != 0 {
                draw_line(r.x + r.w, r.y, r.x + r.w, r.y + r.h, 3.0, c);
            }
            if show & 4 != 0 {
                draw_line(r.x, r.y, r.x + r.w, r.y, 3.0, c);
            }
            if show & 8 != 0 {
                draw_line(r.x, r.y + r.h, r.x + r.w, r.y + r.h, 3.0, c);
            }
        }
        // resize grip
        let gc = Color::new(1.0, 1.0, 1.0, if grip.contains(m) { 0.7 } else { 0.35 });
        for i in 1..4 {
            let o = i as f32 * 4.5;
            draw_line(r.x + r.w - o, r.y + r.h - 2.0, r.x + r.w - 2.0, r.y + r.h - o, 1.0, gc);
        }

        // ---- tool bar
        let ty = r.y + 30.0;
        let th = 92.0;
        let mut x = r.x + 8.0;
        // Points
        group(ui, Rect::new(x, ty, 112.0, 44.0), "Points");
        tool(ui, &mut ed.tool, Tool::AddPoints, Rect::new(x + 6.0, ty + 16.0, 32.0, 24.0), "cut.padd", "+", "Add/Move points");
        tool(ui, &mut ed.tool, Tool::RemovePoints, Rect::new(x + 40.0, ty + 16.0, 32.0, 24.0), "cut.prem", "x", "Remove points");
        let rb = Rect::new(x + 74.0, ty + 16.0, 32.0, 24.0);
        if ui.button("cut.preset", rb, "R") && confirm("Reset Points", "Are you sure you wish to reset the points?\nThis cannot be undone") {
            // fn 0x1402f7c90: asks first, resets, then switches to Add/Move points
            ed.outline.clear(); // back to a clean slate
            ed.colors.clear();
            ed.inner.clear();
            ed.inner_colors.clear();
            ed.tool = Tool::AddPoints;
        }
        tip(ui, rb, "Reset points");
        // Anchors
        group(ui, Rect::new(x, ty + 48.0, 112.0, 44.0), "Anchors");
        tool(ui, &mut ed.tool, Tool::AddAnchors, Rect::new(x + 6.0, ty + 64.0, 32.0, 24.0), "cut.aadd", "+", "Add/Move anchors (green points where leaf generators under this mesh's generator spawn)");
        tool(ui, &mut ed.tool, Tool::RemoveAnchors, Rect::new(x + 40.0, ty + 64.0, 32.0, 24.0), "cut.arem", "x", "Remove anchors");
        let ab = Rect::new(x + 74.0, ty + 64.0, 32.0, 24.0);
        if ui.button("cut.areset", ab, "R") && confirm("Reset Anchors", "Are you sure you wish to reset the Anchors?\nThis cannot be undone") {
            // fn 0x1402f8270: asks first, resets, then switches to Add/Move anchors
            // the green anchors go; the pivot stays where it is
            ed.anchors.truncate(1);
            ed.anchor_angles.truncate(1);
            ed.anchor_ids.truncate(1);
            ed.anchor_sides.truncate(1);
            ed.sel_anchor = 0;
            ed.tool = Tool::AddAnchors;
        }
        tip(ui, ab, "Reset anchors (removes the green anchors; the pivot stays)");
        x += 120.0;
        // Transform: move / rotate / scale the whole card as one island (the person's request)
        group(ui, Rect::new(x, ty, 112.0, 44.0), "Transform");
        tool(ui, &mut ed.tool, Tool::Move, Rect::new(x + 6.0, ty + 16.0, 32.0, 24.0), "cut.move", "M", "Move the whole card (points, pivot and anchors)");
        tool(ui, &mut ed.tool, Tool::Rotate, Rect::new(x + 40.0, ty + 16.0, 32.0, 24.0), "cut.rotate", "R", "Rotate the whole card around its center");
        tool(ui, &mut ed.tool, Tool::Scale, Rect::new(x + 74.0, ty + 16.0, 32.0, 24.0), "cut.scale", "S", "Scale the whole card from its center");
        // Color: paint vertex colours (the original has a Color group here; fn 0x1402f83f0 sets its
        // swatch to red, QColor 7)
        group(ui, Rect::new(x, ty + 48.0, 112.0, 44.0), "Color");
        tool(ui, &mut ed.tool, Tool::PaintColor, Rect::new(x + 6.0, ty + 64.0, 32.0, 24.0), "cut.paint", "P", "Paint vertex colours onto points");
        let sw = Rect::new(x + 40.0, ty + 64.0, 32.0, 24.0);
        ui.record("cut.swatch", sw);
        draw_rectangle(sw.x + 3.0, sw.y + 3.0, sw.w - 6.0, sw.h - 6.0, Color::new(ed.paint[0], ed.paint[1], ed.paint[2], 1.0));
        draw_rectangle_lines(sw.x + 3.0, sw.y + 3.0, sw.w - 6.0, sw.h - 6.0, 1.0, C_LINE);
        // the swatch opens a colour picker beside it (ColorClicked, fn 0x1402f3f90)
        if ui.clicked(sw) {
            ed.picker = if ed.picker.is_some() { None } else { Some(Rect::new(sw.x, sw.y + sw.h + 4.0, 230.0, 118.0)) };
        }
        tip(ui, sw, "Paint colour (click to choose)");
        let cb = Rect::new(x + 74.0, ty + 64.0, 32.0, 24.0);
        if ui.button("cut.colorreset", cb, "R") && confirm("Reset Color", "Are you sure you wish to reset the color?\nThis cannot be undone") {
            // ResetColor (fn 0x1402f8040): asks first, resets every colour, then switches to painting
            // fn 0x1402fd070 fills every colour with the canvas's current paint colour (+0x124,
            // the value the brush uses in fn 0x140300880)
            ed.colors = vec![ed.paint; ed.outline.len()];
            ed.inner_colors = vec![ed.paint; ed.inner.len()];
            ed.tool = Tool::PaintColor;
        }
        tip(ui, cb, "Reset colour (every point takes the swatch colour)");
        x += 120.0;
        // Material
        let opt_w = 300.0;
        let anc_w = 220.0;
        let mat_w = (r.x + r.w - 8.0 - opt_w - 8.0 - anc_w - 8.0 - x).max(160.0);
        // Pivot / Anchor: position and angle of the picked one (the pivot's angle is Options > Angle)
        let gx = x + mat_w + 8.0;
        ed.sel_anchor = ed.sel_anchor.min(ed.anchors.len() - 1);
        let k = ed.sel_anchor;
        group(ui, Rect::new(gx, ty, anc_w, th), if k == 0 { "Pivot" } else { "Anchor" });
        let label = if k == 0 { "Pivot (where the leaf attaches)".to_string() } else { format!("Anchor {} (green)", k) };
        ui.text(&label, gx + 8.0, ty + 16.0, 11.5, C_TEXT_DIM);
        ui.text("X", gx + 8.0, ty + 36.0, 12.0, C_TEXT_DIM);
        ui.slider("cut.ax", Rect::new(gx + 52.0, ty + 34.0, anc_w - 60.0, 17.0), &mut ed.anchors[k].x, 0.0, 1.0, 3);
        ui.text("Y", gx + 8.0, ty + 55.0, 12.0, C_TEXT_DIM);
        ui.slider("cut.ay", Rect::new(gx + 52.0, ty + 53.0, anc_w - 60.0, 17.0), &mut ed.anchors[k].y, 0.0, 1.0, 3);
        ui.text("Angle", gx + 8.0, ty + 74.0, 12.0, C_TEXT_DIM);
        let aw = Rect::new(gx + 52.0, ty + 72.0, anc_w - 60.0, 17.0);
        if k == 0 {
            thumb_wheel(ui, "cut.anchorangle", aw, &mut ed.opts.angle);
            tip(ui, aw, "Pivot angle: the leaf grows along the orange line; drag to turn (or drag its knob); double-click resets");
        } else {
            while ed.anchor_angles.len() < ed.anchors.len() {
                ed.anchor_angles.push(0.0);
            }
            thumb_wheel(ui, "cut.anchorangle", aw, &mut ed.anchor_angles[k]);
            tip(ui, aw, "Anchor angle: leaves spawned here point along the green line; drag to turn (or drag its knob)");
        }
        group(ui, Rect::new(x, ty, mat_w, th), "Material");
        let mut names = vec!["None".to_string()];
        names.extend(self.materials.iter().map(|m| m.name.clone()));
        ed.material = ed.material.min(names.len() - 1);
        ui.combo("cut.material", Rect::new(x + 6.0, ty + 18.0, mat_w - 12.0, 22.0), &names, &mut ed.material);
        if radio(ui, Rect::new(x + 6.0, ty + 46.0, 120.0, 18.0), "Show color", !ed.show_opacity) {
            ed.show_opacity = false;
        }
        if radio(ui, Rect::new(x + 6.0, ty + 68.0, 120.0, 18.0), "Show opacity", ed.show_opacity) {
            ed.show_opacity = true;
        }
        let sa = Rect::new(x + mat_w * 0.5, ty + 47.0, 16.0, 16.0);
        ui.checkbox("cut.showall", sa, &mut ed.show_all);
        ui.text("Show all meshes", sa.x + 22.0, sa.y + 1.0, 12.0, C_TEXT_DIM);
        tip(ui, Rect::new(sa.x, sa.y, 140.0, 16.0), "Also draw the other meshes cut from this material");
        // Options
        let ox = r.x + r.w - 8.0 - opt_w;
        group(ui, Rect::new(ox, ty, opt_w, th), "Options");
        let lx = ox + 100.0;
        let row = |i: f32| ty + 16.0 + i * 19.0;
        ui.text("Tessellation", ox + 8.0, row(0.0) + 2.0, 12.0, C_TEXT_DIM);
        ui.slider("cut.tess", Rect::new(lx, row(0.0), opt_w - 108.0, 17.0), &mut ed.opts.tessellation, 0.0, 1.0, 2);
        ui.text("Angle", ox + 8.0, row(1.0) + 2.0, 12.0, C_TEXT_DIM);
        let ar = Rect::new(lx, row(1.0), opt_w - 108.0, 17.0);
        thumb_wheel(ui, "cut.angle", ar, &mut ed.opts.angle);
        tip(ui, ar, "Drag to turn the leaf's growth line (orange) to match the picture; double-click resets");
        ui.text("Double-sided", ox + 8.0, row(2.0) + 2.0, 12.0, C_TEXT_DIM);
        ui.checkbox("cut.double", Rect::new(lx, row(2.0) + 1.0, 16.0, 16.0), &mut ed.opts.double_sided);
        ui.text("Improve folding", ox + 8.0, row(3.0) + 2.0, 12.0, C_TEXT_DIM);
        let fr = Rect::new(lx, row(3.0) + 1.0, 16.0, 16.0);
        ui.checkbox("cut.folding", fr, &mut ed.opts.improve_folding);
        tip(ui, Rect::new(ox, row(3.0), opt_w, 18.0), "Lines the triangles up with the fold line (from the pivot along the Angle) so the mesh folds cleanly");

        // ---- canvas and the three levels
        let body_y = ty + th + 8.0;
        let bottom = r.y + r.h - 44.0;
        let col_w = 300.0;
        let cv_area = Rect::new(r.x + 8.0, body_y, r.w - 24.0 - col_w, bottom - body_y);
        let col = Rect::new(cv_area.x + cv_area.w + 8.0, body_y, col_w, bottom - body_y);
        // while the colour picker is open, the canvas under it doesn't see the mouse
        let over_picker = ed.picker.map(|p| p.contains(ui.mouse())).unwrap_or(false);
        let saved = ui.inp.clone();
        if over_picker {
            ui.inp.l_pressed = false;
            ui.inp.l_down = false;
            ui.inp.r_pressed = false;
        }
        self.cutout_canvas(ui, &mut ed, cv_area);
        self.cutout_levels(ui, &mut ed, col);
        ui.inp = saved;
        if let Some(p) = ed.picker {
            draw_rectangle(p.x + 3.0, p.y + 4.0, p.w, p.h, Color::new(0.0, 0.0, 0.0, 0.4));
            draw_rectangle(p.x, p.y, p.w, p.h, C_PANEL2);
            draw_rectangle_lines(p.x, p.y, p.w, p.h, 1.0, C_LINE);
            for (i, name) in ["R", "G", "B"].iter().enumerate() {
                let yy = p.y + 8.0 + i as f32 * 22.0;
                ui.text(name, p.x + 8.0, yy + 2.0, 12.0, C_TEXT_DIM);
                ui.slider(&format!("cut.pick{}", i), Rect::new(p.x + 24.0, yy, p.w - 32.0, 18.0), &mut ed.paint[i], 0.0, 1.0, 2);
            }
            for (i, (name, c)) in PAINT_COLORS.iter().enumerate() {
                let b = Rect::new(p.x + 8.0 + i as f32 * 27.0, p.y + 78.0, 22.0, 22.0);
                draw_rectangle(b.x, b.y, b.w, b.h, Color::new(c[0], c[1], c[2], 1.0));
                draw_rectangle_lines(b.x, b.y, b.w, b.h, if *c == ed.paint { 2.0 } else { 1.0 }, if *c == ed.paint { WHITE } else { C_LINE });
                tip(ui, b, name);
                if ui.clicked(b) {
                    ed.paint = *c;
                }
            }
            ui.text_fit("Click outside to close", p.x + 8.0, p.y + p.h - 15.0, p.w - 16.0, 10.5, C_TEXT_FAINT);
            let swatch = ui.rects.get("cut.swatch").copied();
            if ui.inp.l_pressed && !p.contains(ui.mouse()) && !swatch.map(|s| s.contains(ui.mouse())).unwrap_or(false) {
                ed.picker = None;
                ed.tool = Tool::PaintColor;
            }
        }

        // ---- bottom: counts, Confirm, Cancel
        let g = ed.geom();
        let (nv, nf) = if ed.outline.len() >= 3 { (g.pos.len(), g.idx.len() / 3) } else { (0, 0) };
        let label = format!("V:{}  F:{}   editing: {}", nv, nf, LOD_NAMES[ed.target]);
        ui.text(&label, cv_area.x + 4.0, bottom + 14.0, 12.0, C_TEXT_DIM);
        if let Some((w, t)) = &ed.warn {
            if get_time() - t < 3.0 {
                ui.text(w, cv_area.x + 260.0, bottom + 14.0, 12.0, C_ORANGE);
            }
        }
        let cr = Rect::new(r.x + r.w - 24.0 - 110.0, bottom + 8.0, 110.0, 28.0);
        let ok = Rect::new(cr.x - 118.0, cr.y, 110.0, 28.0);
        let mut close = false;
        if ui.button_ex("cut.confirm", ok, "Confirm", true, true) {
            self.apply_cutout(&ed);
            close = true;
        }
        if ui.button("cut.cancel", cr, "Cancel") {
            close = true;
        }
        if !close {
            self.cutout = Some(ed);
        }
        // modal: nothing behind the window reacts. A menu opened in the window (anchor numbers,
        // material list) still needs this frame's click: menus take it at the end of the frame.
        ui.blocked = true;
        if ui.popup.is_none() {
            ui.inp.l_pressed = false;
        }
        ui.inp.r_pressed = false;
        ui.inp.keys.clear();
    }

    fn apply_cutout(&mut self, ed: &CutoutEditor) {
        let mut lods = ed.lods.clone();
        if ed.outline.len() >= 3 || lods[ed.target].geom.is_none() {
            // the canvas becomes the level (a cleared canvas leaves the level empty)
            lods[ed.target] = ed.as_level();
        } else {
            lods[ed.target].anchors = ed.green(); // a level made of mesh geometry keeps it
        }
        let m = &mut self.meshes[ed.mesh];
        m.lods = lods;
        m.anchor = ed.anchors[0];
        m.tessellation = ed.opts.tessellation;
        m.angle = ed.opts.angle;
        m.double_sided = ed.opts.double_sided;
        m.improve_folding = ed.opts.improve_folding;
        m.embedded = true;
        if ed.material > 0 {
            m.ref_material = ed.material - 1;
        }
        self.mesh_lod = ed.target;
        self.dirty = true;
    }

    fn cutout_canvas(&mut self, ui: &mut Ui, ed: &mut CutoutEditor, area: Rect) {
        draw_rectangle(area.x, area.y, area.w, area.h, Color::new(0.16, 0.16, 0.16, 1.0));
        let side = area.w.min(area.h - 24.0) - 16.0;
        let fit = Rect::new(area.x + (area.w - side) * 0.5, area.y + 8.0, side, side);
        // the original's canvas view (fn 0x140300880): middle-drag pans, middle+left drag zooms
        // (0.01 per pixel); double-middle-click fits the picture again
        {
            let m = ui.mouse();
            let d = m - ed.last_mouse;
            if area.contains(m) && ui.inp.m_down && ui.popup.is_none() {
                if ui.inp.l_down {
                    ed.view_zoom = (ed.view_zoom - d.y * 0.01).clamp(0.2, 12.0);
                } else {
                    ed.view_pan += d / (fit.w * ed.view_zoom).max(1.0);
                }
            }
            ed.last_mouse = m;
        }
        let z = ed.view_zoom;
        let cv = Rect::new(
            fit.x + fit.w * 0.5 * (1.0 - z) + ed.view_pan.x * fit.w * z,
            fit.y + fit.h * 0.5 * (1.0 - z) + ed.view_pan.y * fit.h * z,
            fit.w * z,
            fit.h * z,
        );
        ui.set_clip(Some(area));
        ui.record("cutout.canvas", cv);
        // the original's canvas checkerboard uses 7 px squares (constant 0x14107cab0, fn 0x1402fe130)
        checker(cv, 7.0);
        // the material's picture (colour, or its opacity as grey)
        if ed.material > 0 {
            let mi = ed.material - 1;
            if ed.show_opacity {
                if ed.opacity_tex.as_ref().map(|(k, _)| *k != mi).unwrap_or(true) {
                    ed.opacity_tex = self.materials.get(mi).and_then(|m| m.adjusted.as_ref()).map(|img| {
                        let mut px = img.px.clone();
                        for p in px.chunks_mut(4) {
                            let a = p[3];
                            p[0] = a;
                            p[1] = a;
                            p[2] = a;
                            p[3] = 255;
                        }
                        (mi, Texture2D::from_rgba8(img.w as u16, img.h as u16, &px))
                    });
                }
                if let Some((_, t)) = &ed.opacity_tex {
                    draw_texture_ex(t, cv.x, cv.y, WHITE, DrawTextureParams { dest_size: Some(vec2(cv.w, cv.h)), ..Default::default() });
                }
            } else if let Some(t) = self.materials.get(mi).and_then(|m| m.tex.as_ref()) {
                draw_texture_ex(t, cv.x, cv.y, WHITE, DrawTextureParams { dest_size: Some(vec2(cv.w, cv.h)), ..Default::default() });
            }
        }
        let to_s = |p: Vec2| vec2(cv.x + p.x * cv.w, cv.y + p.y * cv.h);
        let to_uv = |s: Vec2| vec2((s.x - cv.x) / cv.w, (s.y - cv.y) / cv.h);
        // other meshes cut from the same material
        if ed.show_all && ed.material > 0 {
            for (k, m) in self.meshes.iter().enumerate() {
                if k == ed.mesh || m.ref_material + 1 != ed.material {
                    continue;
                }
                let o = &m.lods[0].outline;
                for i in 0..o.len() {
                    let a = to_s(o[i]);
                    let b = to_s(o[(i + 1) % o.len()]);
                    draw_line(a.x, a.y, b.x, b.y, 1.0, Color::new(1.0, 1.0, 1.0, 0.35));
                }
            }
        }
        // the growth line through the pivot, turned by its angle; then each green anchor's line
        let a0 = to_s(ed.anchors[0]);
        let th = ed.opts.angle.to_radians();
        let unit = vec2(th.sin(), -th.cos());
        let dir = unit * cv.w * 1.5;
        let orange = Color::new(1.0, 0.55, 0.1, 0.9);
        let green = Color::new(0.25, 0.85, 0.35, 1.0);
        if let Some((p, q)) = clip_segment(a0 - dir, a0 + dir, cv) {
            draw_line(p.x, p.y, q.x, q.y, 1.0, orange);
        }
        // angle handles: a round knob 60 px along each line (k = 0 is the pivot)
        let m0 = ui.mouse();
        let mut hover_knob = None;
        for k in 0..ed.anchors.len() {
            let deg = if k == 0 { ed.opts.angle } else { ed.anchor_angles.get(k).copied().unwrap_or(0.0) };
            let r = deg.to_radians();
            let base = to_s(ed.anchors[k]);
            let knob = base + vec2(r.sin(), -r.cos()) * if k == 0 { 60.0 } else { 40.0 };
            let c = if k == 0 { orange } else { green };
            let hot = knob.distance(m0) < 9.0 || ed.angle_drag == Some(k);
            if knob.distance(m0) < 9.0 {
                hover_knob = Some(k);
            }
            draw_line(base.x, base.y, knob.x, knob.y, 2.5, c);
            draw_circle(knob.x, knob.y, if hot { 8.0 } else { 6.0 }, c);
            draw_circle_lines(knob.x, knob.y, if hot { 8.0 } else { 6.0 }, 1.5, WHITE);
        }
        if let Some(k) = hover_knob {
            if ui.active == 0 && ed.drag.is_none() {
                let deg = if k == 0 { ed.opts.angle } else { ed.anchor_angles.get(k).copied().unwrap_or(0.0) };
                ui.tooltip = Some(if k == 0 { format!("Pivot angle {:.0}° - drag to turn", deg) } else { format!("Anchor {} angle {:.0}° - drag to turn", k, deg) });
                if ui.inp.l_pressed && ui.popup.is_none() {
                    ed.angle_drag = Some(k);
                    ed.sel_anchor = k;
                    ui.active = hid("cutout.angle");
                }
            }
        }
        if let Some(k) = ed.angle_drag {
            if ui.inp.l_down {
                let v = m0 - to_s(ed.anchors[k.min(ed.anchors.len() - 1)]);
                if v.length() > 4.0 {
                    let deg = v.x.atan2(-v.y).to_degrees(); // smooth, no snapping (the person's request)
                    if k == 0 {
                        ed.opts.angle = deg;
                    } else if k < ed.anchor_angles.len() {
                        ed.anchor_angles[k] = deg;
                    }
                }
            } else {
                ed.angle_drag = None;
                ui.active = 0;
            }
        }        let cyan = Color::new(0.2, 0.9, 0.9, 1.0);
        if ed.outline.len() == 2 {
            let (a, b) = (to_s(ed.outline[0]), to_s(ed.outline[1]));
            draw_line(a.x, a.y, b.x, b.y, 2.0, cyan);
        }
        if ed.outline.len() >= 3 {
            if ed.inner_colors.len() != ed.inner.len() {
                ed.inner_colors.resize(ed.inner.len(), [1.0; 3]);
            }
            let g = ed.geom();
            // in colour mode the triangles show the painted vertex colours, blended across each
            // triangle as the leaf will get them (picture-16)
            if ed.tool == Tool::PaintColor && ed.colors.len() == ed.outline.len() {
                let gc = g.clone();
                let n_front = if ed.opts.double_sided { gc.idx.len() / 2 } else { gc.idx.len() };
                let to_v = |i: usize| {
                    let p = to_s(gc.uv[i]);
                    let c = gc.col.get(i).copied().unwrap_or([1.0; 3]);
                    macroquad::models::Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, Color::new(c[0], c[1], c[2], 0.6))
                };
                for chunk in gc.idx[..n_front].chunks(3 * 20000) {
                    let vertices: Vec<_> = chunk.iter().map(|&i| to_v(i as usize)).collect();
                    let indices: Vec<u16> = (0..vertices.len() as u16).collect();
                    draw_mesh(&macroquad::models::Mesh { vertices, indices, texture: None });
                }
            }
            // inner (triangle) edges: yellow and thicker so they read over the picture (picture-18);
            // the outline is drawn over them in cyan
            let tri_c = Color::new(1.0, 0.86, 0.1, 0.85);
            let n_front = if ed.opts.double_sided { g.idx.len() / 2 } else { g.idx.len() };
            for f in g.idx[..n_front].chunks(3) {
                let (a, b, c) = (to_s(g.uv[f[0] as usize]), to_s(g.uv[f[1] as usize]), to_s(g.uv[f[2] as usize]));
                draw_line(a.x, a.y, b.x, b.y, 1.6, tri_c);
                draw_line(b.x, b.y, c.x, c.y, 1.6, tri_c);
                draw_line(c.x, c.y, a.x, a.y, 1.6, tri_c);
            }
            for i in 0..ed.outline.len() {
                let a = to_s(ed.outline[i]);
                let b = to_s(ed.outline[(i + 1) % ed.outline.len()]);
                draw_line(a.x, a.y, b.x, b.y, 2.0, cyan);
            }
        }
        let m = ui.mouse();
        if ed.colors.len() != ed.outline.len() {
            ed.colors.resize(ed.outline.len(), [1.0; 3]);
        }
        if ed.inner_colors.len() != ed.inner.len() {
            ed.inner_colors.resize(ed.inner.len(), [1.0; 3]);
        }
        // painted vertex colours: a coloured halo under each point (outline, then inner points)
        for (p, c) in ed.outline.iter().zip(&ed.colors).chain(ed.inner.iter().zip(&ed.inner_colors)) {
            if *c != [1.0, 1.0, 1.0] {
                let s = to_s(*p);
                draw_circle(s.x, s.y, 9.0, Color::new(c[0], c[1], c[2], 0.85));
            }
        }
        // grab radii from fn 0x140300880: points 7 px (+20 with Remove points), pivot 10 px,
        // anchors 7 px (27 with Remove anchors). Point k >= outline.len() is inner point k - n.
        let pt_r = if ed.tool == Tool::RemovePoints { 27.0 } else { 7.0 };
        let n_out = ed.outline.len();
        let hover_pt = ed.outline.iter().chain(&ed.inner).position(|p| to_s(*p).distance(m) < pt_r);
        let hover_anc = ed.anchors.iter().enumerate().position(|(k, p)| {
            let r = if k == 0 { 10.0 } else if ed.tool == Tool::RemoveAnchors { 27.0 } else { 7.0 };
            to_s(*p).distance(m) < r
        });
        for (k, p) in ed.outline.iter().chain(&ed.inner).enumerate() {
            let s = to_s(*p);
            let hot = hover_pt == Some(k) || ed.drag == Some((false, k));
            draw_circle(s.x, s.y, if hot { 6.0 } else { 4.5 }, if hot { WHITE } else { Color::new(0.9, 0.15, 0.15, 1.0) });
            if k >= n_out && !hot {
                // inner points have a light ring
                draw_circle_lines(s.x, s.y, 5.5, 1.0, Color::new(1.0, 0.8, 0.8, 0.9));
            }
        }
        // the pivot is white, the other anchors green, each with its number below it
        for (k, p) in ed.anchors.iter().enumerate() {
            let s = to_s(*p);
            let hot = hover_anc == Some(k) || ed.drag == Some((true, k));
            let fill = if k == 0 { WHITE } else { Color::new(0.25, 0.85, 0.35, 1.0) };
            draw_circle(s.x, s.y, if hot { 8.0 } else { 6.5 }, fill);
            draw_circle_lines(s.x, s.y, if hot { 8.0 } else { 6.5 }, 1.5, if k == 0 { Color::new(0.2, 0.75, 0.3, 1.0) } else { WHITE });
            if k > 0 {
                let id = ed.anchor_ids.get(k).copied().unwrap_or(k).to_string();
                let tw = ui.text_w(&id, 13.0);
                draw_rectangle(s.x - tw * 0.5 - 3.0, s.y + 9.0, tw + 6.0, 15.0, Color::new(0.0, 0.0, 0.0, 0.6));
                ui.text(&id, s.x - tw * 0.5, s.y + 9.0, 13.0, Color::new(0.6, 1.0, 0.6, 1.0));
            }
        }

        // editing
        let over = cv.contains(m) && area.contains(m) && ui.popup.is_none() && !ui.inp.m_down;
        let inp = ui.inp.clone();
        let uv_raw = to_uv(m);
        let uv = vec2(uv_raw.x.clamp(0.0, 1.0), uv_raw.y.clamp(0.0, 1.0));
        // right-click a green anchor: pick its number (the "Anchors" choice of generators uses it)
        if over && ui.active == 0 && inp.r_pressed {
            if let Some(k) = hover_anc.filter(|k| *k > 0) {
                ed.id_popup = Some(k);
                ed.sel_anchor = k;
                // the original's menu (fn 0x140301500): "ID" 1-5 and "Side" Automatic/Left/Right,
                // each with the current one ticked
                let cur = ed.anchor_ids.get(k).copied().unwrap_or(1);
                let side = ed.anchor_sides.get(k).copied().unwrap_or(0) as usize;
                // as many numbers as there are anchors (at least the original's 5), the person's request
                let nid = id_count(ed);
                let mut items: Vec<String> = (1..=nid).map(|n| format!("{} Anchor number {}", if n == cur { "[x]" } else { "[  ]" }, n)).collect();
                items.extend(ANCHOR_SIDES.iter().enumerate().map(|(i, s)| format!("{} Side: {}", if i == side { "[x]" } else { "[  ]" }, s)));
                ui.open_popup(hid("cut.anchorid"), Rect::new(m.x, m.y, 10.0, 0.0), items, None);
            }
        }
        if let Some(n) = ui.take_popup_result("cut.anchorid") {
            if let Some(k) = ed.id_popup.take() {
                let nid = id_count(ed);
                ed.anchor_ids.resize(ed.anchors.len().max(ed.anchor_ids.len()), 1);
                ed.anchor_sides.resize(ed.anchors.len().max(ed.anchor_sides.len()), 0);
                if k < ed.anchors.len() {
                    if n < nid {
                        ed.anchor_ids[k] = n + 1;
                    } else {
                        ed.anchor_sides[k] = (n - nid) as u8;
                    }
                }
            }
        }
        // transform the whole card (points, pivot and anchors) as one island
        if matches!(ed.tool, Tool::Move | Tool::Rotate | Tool::Scale) && over && ui.active == 0 && inp.l_pressed {
            let center = card_center(ed).unwrap_or(uv_raw);
            ed.xform = Some(Xform {
                start: uv_raw,
                center,
                // the outline then the inner points (split again after transforming)
                outline: ed.outline.iter().chain(&ed.inner).copied().collect(),
                anchors: ed.anchors.clone(),
                angles: ed.anchor_angles.clone(),
                pivot_angle: ed.opts.angle,
                axis: giz_part(ed.tool, to_s(center), m),
            });
            ui.active = hid("cutout.xform");
        }
        if let Some(x) = &ed.xform {
            if inp.l_down {
                let (c, s0) = (x.center, x.start - x.center);
                let (outline0, anchors0, angles0, piv0, axis) = (x.outline.clone(), x.anchors.clone(), x.angles.clone(), x.pivot_angle, x.axis);
                let f: Box<dyn Fn(Vec2) -> Vec2> = match ed.tool {
                    Tool::Move => {
                        let mut d = uv_raw - x.start;
                        match axis {
                            1 => d.y = 0.0,
                            2 => d.x = 0.0,
                            _ => {}
                        }
                        Box::new(move |p| p + d)
                    }
                    Tool::Rotate => {
                        let a = (uv_raw - c).y.atan2((uv_raw - c).x) - s0.y.atan2(s0.x);
                        let (sn, cs) = a.sin_cos();
                        ed.opts.angle = wrap_deg(piv0 + a.to_degrees());
                        for k in 1..ed.anchor_angles.len().min(angles0.len()) {
                            ed.anchor_angles[k] = wrap_deg(angles0[k] + a.to_degrees());
                        }
                        Box::new(move |p| {
                            let v = p - c;
                            c + vec2(v.x * cs - v.y * sn, v.x * sn + v.y * cs)
                        })
                    }
                    _ => {
                        let ratio = |now: f32, was: f32| if was.abs() < 1e-4 { 1.0 } else { (now / was).clamp(0.05, 20.0) };
                        let k = match axis {
                            1 => vec2(ratio((uv_raw - c).x, s0.x), 1.0),
                            2 => vec2(1.0, ratio((uv_raw - c).y, s0.y)),
                            _ => Vec2::splat(((uv_raw - c).length() / s0.length().max(1e-4)).clamp(0.05, 20.0)),
                        };
                        Box::new(move |p| c + (p - c) * k)
                    }
                };
                let moved: Vec<Vec2> = outline0.iter().map(|p| f(*p)).collect();
                let n = ed.outline.len().min(moved.len());
                ed.outline = moved[..n].to_vec();
                ed.inner = moved[n..].to_vec();
                ed.anchors = anchors0.iter().map(|p| f(*p)).collect();
            } else {
                ed.xform = None;
                ui.active = 0;
            }
        }
        // transform gizmo (picture-15): shows which transform tool is on, at the card's centre;
        // red X / green Y handles limit Move and Scale to one axis, the centre square drags freely
        if matches!(ed.tool, Tool::Move | Tool::Rotate | Tool::Scale) {
            if let Some(cu) = card_center(ed) {
                let dragging = ed.xform.as_ref();
                let c_uv = match dragging {
                    Some(x) if ed.tool != Tool::Move => x.center,
                    _ => cu,
                };
                let cs = to_s(c_uv);
                let hot = match dragging {
                    Some(x) => x.axis,
                    None if over => giz_part(ed.tool, cs, m),
                    None => 255,
                };
                let hi = Color::new(1.0, 0.85, 0.2, 1.0);
                let red = if hot == 1 { hi } else { Color::new(0.95, 0.3, 0.3, 1.0) };
                let grn = if hot == 2 { hi } else { Color::new(0.35, 0.9, 0.35, 1.0) };
                let blu = Color::new(0.35, 0.65, 1.0, 1.0);
                let free = if hot == 0 { hi } else { blu };
                let (xe, ye) = (cs + vec2(GIZ_L, 0.0), cs + vec2(0.0, -GIZ_L));
                match ed.tool {
                    Tool::Move => {
                        draw_line(cs.x, cs.y, xe.x - 10.0, xe.y, if hot == 1 { 3.0 } else { 2.0 }, red);
                        draw_triangle(xe, xe + vec2(-12.0, -6.0), xe + vec2(-12.0, 6.0), red);
                        draw_line(cs.x, cs.y, ye.x, ye.y + 10.0, if hot == 2 { 3.0 } else { 2.0 }, grn);
                        draw_triangle(ye, ye + vec2(-6.0, 12.0), ye + vec2(6.0, 12.0), grn);
                        draw_rectangle(cs.x - 7.0, cs.y - 7.0, 14.0, 14.0, Color::new(free.r, free.g, free.b, 0.35));
                        draw_rectangle_lines(cs.x - 7.0, cs.y - 7.0, 14.0, 14.0, 2.0, free);
                        if let Some(x) = dragging {
                            let txt = format!("{:+.3}, {:+.3}", cu.x - x.center.x, cu.y - x.center.y);
                            ui.text(&txt, cs.x + 12.0, cs.y + 22.0, 12.0, hi);
                        }
                    }
                    Tool::Rotate => {
                        draw_circle_lines(cs.x, cs.y, GIZ_L, 2.0, blu);
                        draw_circle(cs.x, cs.y, 3.0, blu);
                        if let Some(x) = dragging {
                            let v0 = to_s(x.start) - cs;
                            let v1 = m - cs;
                            let (a0, a1) = (v0.y.atan2(v0.x), v1.y.atan2(v1.x));
                            let mut da = a1 - a0;
                            while da > std::f32::consts::PI {
                                da -= std::f32::consts::TAU;
                            }
                            while da < -std::f32::consts::PI {
                                da += std::f32::consts::TAU;
                            }
                            let n = 32;
                            for i in 0..n {
                                let (t0, t1) = (a0 + da * i as f32 / n as f32, a0 + da * (i + 1) as f32 / n as f32);
                                let p0 = cs + vec2(t0.cos(), t0.sin()) * GIZ_L;
                                let p1 = cs + vec2(t1.cos(), t1.sin()) * GIZ_L;
                                draw_triangle(cs, p0, p1, Color::new(1.0, 0.85, 0.2, 0.25));
                            }
                            let e0 = cs + vec2(a0.cos(), a0.sin()) * GIZ_L;
                            let e1 = cs + vec2(a1.cos(), a1.sin()) * GIZ_L;
                            draw_line(cs.x, cs.y, e0.x, e0.y, 1.0, C_TEXT_DIM);
                            draw_line(cs.x, cs.y, e1.x, e1.y, 2.0, hi);
                            draw_circle(e1.x, e1.y, 5.0, hi);
                            let deg = wrap_deg(ed.opts.angle - x.pivot_angle);
                            ui.text(&format!("{:+.1}°", deg), cs.x + 12.0, cs.y + 22.0, 12.0, hi);
                        } else {
                            let ring = over && ((m - cs).length() - GIZ_L).abs() < 8.0;
                            if ring {
                                draw_circle_lines(cs.x, cs.y, GIZ_L, 3.0, hi);
                            }
                        }
                    }
                    _ => {
                        draw_line(cs.x, cs.y, xe.x, xe.y, if hot == 1 { 3.0 } else { 2.0 }, red);
                        draw_rectangle(xe.x - 5.0, xe.y - 5.0, 10.0, 10.0, red);
                        draw_line(cs.x, cs.y, ye.x, ye.y, if hot == 2 { 3.0 } else { 2.0 }, grn);
                        draw_rectangle(ye.x - 5.0, ye.y - 5.0, 10.0, 10.0, grn);
                        draw_rectangle(cs.x - 7.0, cs.y - 7.0, 14.0, 14.0, Color::new(free.r, free.g, free.b, 0.35));
                        draw_rectangle_lines(cs.x - 7.0, cs.y - 7.0, 14.0, 14.0, 2.0, free);
                        if let (Some(x), Some(o0)) = (dragging, dragging.and_then(|x| x.outline.first().copied())) {
                            // the scale reached, read from the first point's distance to the centre
                            let (now, was) = (ed.outline.first().copied().unwrap_or(o0) - x.center, o0 - x.center);
                            let r = |a: f32, b: f32| if b.abs() < 1e-5 { 1.0 } else { a / b };
                            let txt = match x.axis {
                                1 => format!("x {:.2}", r(now.x, was.x)),
                                2 => format!("y {:.2}", r(now.y, was.y)),
                                _ => format!("x {:.2}", now.length() / was.length().max(1e-5)),
                            };
                            ui.text(&txt, cs.x + 12.0, cs.y + 22.0, 12.0, hi);
                        }
                    }
                }
            }
        }
        // paint vertex colours onto the points under the brush
        // the original paints along the stroke from the last mouse position with a 20 px brush in
        // the current colour (mode 6 in fn 0x140300880 calls fn 0x1402fd0f0 with radius 20)
        const BRUSH: f32 = 20.0;
        if ed.tool == Tool::PaintColor && over && inp.l_down && !inp.m_down && (ui.active == 0 || ui.active == hid("cutout.paint")) {
            ui.active = hid("cutout.paint");
            // the first dab on press is 10 px (fn 0x140301500), the drag 20 px (fn 0x140300880)
            let r = if ed.paint_prev.is_none() { 10.0 } else { BRUSH };
            let a = ed.paint_prev.unwrap_or(m);
            let n_out = ed.outline.len();
            for k in 0..n_out + ed.inner.len() {
                let p = if k < n_out { ed.outline[k] } else { ed.inner[k - n_out] };
                let s = to_s(p);
                let ab = m - a;
                let t = if ab.length_squared() > 1e-6 { ((s - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
                if (a + ab * t).distance(s) < r {
                    if k < n_out {
                        ed.colors[k] = ed.paint;
                    } else if let Some(c) = ed.inner_colors.get_mut(k - n_out) {
                        *c = ed.paint;
                    }
                }
            }
            ed.paint_prev = Some(m);
        }
        if ed.tool == Tool::PaintColor && !inp.l_down && ui.active == hid("cutout.paint") {
            ui.active = 0;
        }
        if !inp.l_down {
            ed.paint_prev = None;
        }
        if ed.tool == Tool::PaintColor && over {
            draw_circle_lines(m.x, m.y, BRUSH, 1.5, Color::new(ed.paint[0], ed.paint[1], ed.paint[2], 0.9));
        }
        if over && ui.active == 0 && inp.l_pressed {
            match ed.tool {
                Tool::AddPoints => {
                    if let Some(k) = hover_pt {
                        ed.drag = Some((false, k));
                    } else if let Some(k) = hover_anc {
                        // anchors can always be dragged
                        ed.drag = Some((true, k));
                        ed.sel_anchor = k;
                    } else if ed.outline.len() >= 3 && point_in_poly(&ed.outline, uv) {
                        // a click inside the shape adds an inner vertex there, joined into the
                        // triangles (always to its nearest vertex); the outline stays (picture-17)
                        let near = ed.outline.iter().zip(&ed.colors).chain(ed.inner.iter().zip(&ed.inner_colors)).min_by(|a, b| a.0.distance(uv).total_cmp(&b.0.distance(uv)));
                        let c = near.map(|(_, c)| *c).unwrap_or([1.0; 3]);
                        ed.inner.push(uv);
                        ed.inner_colors.push(c);
                        ed.drag = Some((false, ed.outline.len() + ed.inner.len() - 1));
                    } else {
                        let at = insert_index(&ed.outline, uv);
                        let mut next = ed.outline.clone();
                        next.insert(at, uv);
                        if is_simple(&next) {
                            ed.outline = next;
                            // a new point takes the colour between its neighbours
                            let n = ed.colors.len();
                            let c = if n >= 2 {
                                let (a, b) = (ed.colors[(at + n - 1) % n], ed.colors[at % n]);
                                [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5]
                            } else {
                                [1.0; 3]
                            };
                            ed.colors.insert(at.min(n), c);
                            ed.drag = Some((false, at));
                        } else {
                            ed.warn = Some(("A point there would make the outline cross itself".into(), get_time()));
                        }
                    }
                }
                Tool::RemovePoints => {
                    if let Some(k) = hover_pt.filter(|k| *k >= ed.outline.len()) {
                        let i = k - ed.outline.len();
                        ed.inner.remove(i);
                        if i < ed.inner_colors.len() {
                            ed.inner_colors.remove(i);
                        }
                    } else if let Some(k) = hover_pt {
                        let mut next = ed.outline.clone();
                        next.remove(k);
                        if is_simple(&next) {
                            ed.outline = next;
                            if k < ed.colors.len() {
                                ed.colors.remove(k);
                            }
                        } else {
                            ed.warn = Some(("Removing that point would make the outline cross itself".into(), get_time()));
                        }
                    }
                }
                Tool::AddAnchors => {
                    if let Some(k) = hover_anc {
                        ed.drag = Some((true, k));
                        ed.sel_anchor = k;
                    } else if let Some(k) = hover_pt {
                        // pressing a point in anchor mode drags the point (fn 0x140301500)
                        ed.drag = Some((false, k));
                    } else {
                        // fn 0x140301500: a new anchor gets ID 1, Side Automatic, and its line
                        // points away from the pivot (0.05 picture units long)
                        let v = uv - ed.anchors[0];
                        let ang = if v.length_squared() > 1e-10 { v.x.atan2(-v.y).to_degrees() } else { 0.0 };
                        ed.anchors.push(uv);
                        ed.anchor_angles.resize(ed.anchors.len() - 1, 0.0);
                        ed.anchor_angles.push(ang);
                        ed.anchor_ids.resize(ed.anchors.len() - 1, 1);
                        ed.anchor_ids.push(1);
                        ed.anchor_sides.resize(ed.anchors.len() - 1, 0);
                        ed.anchor_sides.push(0);
                        ed.drag = Some((true, ed.anchors.len() - 1));
                        ed.sel_anchor = ed.anchors.len() - 1;
                    }
                }
                Tool::RemoveAnchors => {
                    if let Some(k) = hover_anc {
                        if k > 0 {
                            ed.anchors.remove(k);
                            if k < ed.anchor_angles.len() {
                                ed.anchor_angles.remove(k);
                            }
                            if k < ed.anchor_sides.len() {
                                ed.anchor_sides.remove(k);
                            }
                            if k < ed.anchor_ids.len() {
                                ed.anchor_ids.remove(k);
                            }
                            ed.sel_anchor = 0;
                        } else {
                            ed.warn = Some(("The pivot can't be removed (drag it to move it)".into(), get_time()));
                        }
                    }
                }
                _ => {} // transform and paint tools are handled above
            }
            if ed.drag.is_some() {
                ui.active = hid("cutout.drag");
            }
        }
        if let Some((anchor, k)) = ed.drag {
            if inp.l_down {
                if anchor {
                    if k < ed.anchors.len() {
                        ed.anchors[k] = uv;
                    }
                } else if k >= ed.outline.len() {
                    // an inner point stays inside the shape
                    let i = k - ed.outline.len();
                    if i < ed.inner.len() && point_in_poly(&ed.outline, uv) {
                        ed.inner[i] = uv;
                    }
                } else if k < ed.outline.len() {
                    // a move that would make the outline cross itself is refused
                    let mut next = ed.outline.clone();
                    next[k] = uv;
                    if is_simple(&next) {
                        ed.outline = next;
                    }
                }
            } else {
                ed.drag = None;
            }
        }
        let hint = match ed.tool {
            Tool::AddPoints => "Click outside the shape to add an outline point, inside it to add an inner vertex; drag a point or the white pivot to move it, drag a knob to turn an angle",
            Tool::RemovePoints => "Click a point to remove it",
            Tool::AddAnchors => "Click to add a green anchor, drag one to move it, drag its knob to set its angle (white = pivot, where the leaf attaches)",
            Tool::RemoveAnchors => "Click an anchor to remove it",
            Tool::Move => "Drag to move the whole card (points, pivot and anchors); the red / green arrow moves it only sideways / up-down",
            Tool::Rotate => "Drag around the ring (or anywhere) to rotate the card (angles turn with it)",
            Tool::Scale => "Drag the centre square to scale evenly; the red / green handle scales only the width / height",
            Tool::PaintColor => "Drag over points to paint them with the Color swatch",
        };
        ui.set_clip(None);
        let full_hint = format!("{}  •  middle-drag pans, middle+left drag zooms", hint);
        ui.text_fit(&full_hint, area.x + 6.0, area.y + area.h - 18.0, area.w - 70.0, 11.0, C_TEXT_FAINT);
        let fb = Rect::new(area.x + area.w - 56.0, area.y + area.h - 24.0, 50.0, 20.0);
        if ui.button("cut.fitview", fb, "Fit") {
            ed.view_zoom = 1.0;
            ed.view_pan = Vec2::ZERO;
        }
        tip(ui, fb, "Fit the picture in the window again");
    }

    fn cutout_levels(&mut self, ui: &mut Ui, ed: &mut CutoutEditor, col: Rect) {
        let h = col.h / 3.0;
        for l in 0..3 {
            let y = col.y + l as f32 * h;
            let box_r = Rect::new(col.x + 34.0, y + 18.0, col.w - 34.0, h - 40.0);
            ui.text(LOD_NAMES[l], col.x, y + 2.0, 12.0, if l == ed.target { WHITE } else { C_TEXT_DIM });
            let side = box_r.w.min(box_r.h);
            let pv = Rect::new(box_r.x + (box_r.w - side) * 0.5, box_r.y, side, side);
            checker(pv, 10.0);
            draw_rectangle_lines(pv.x, pv.y, pv.w, pv.h, if l == ed.target { 2.0 } else { 1.0 }, if l == ed.target { C_ACCENT2 } else { C_LINE });
            // what this level holds now (the canvas, for the level being edited)
            let geom = if l == ed.target && ed.outline.len() >= 3 {
                Some(ed.geom())
            } else if let Some(g) = &ed.lods[l].geom {
                Some(g.clone())
            } else if ed.lods[l].outline.len() >= 3 {
                let lv = &ed.lods[l];
                Some(meshio::outline_geom_inner(&lv.outline, &lv.colors, &lv.inner, &lv.inner_colors, ed.anchors[0], ed.opts))
            } else {
                None
            };
            let (mut nv, mut nf) = (0, 0);
            if let Some(g) = &geom {
                nv = g.pos.len();
                nf = g.idx.len() / 3;
                let to_s = |p: Vec2| vec2(pv.x + p.x.clamp(0.0, 1.0) * pv.w, pv.y + p.y.clamp(0.0, 1.0) * pv.h);
                let c = Color::new(0.2, 0.9, 0.9, 0.7);
                for f in g.idx.chunks(3).take(4000) {
                    if let (Some(a), Some(b), Some(cc)) = (g.uv.get(f[0] as usize), g.uv.get(f[1] as usize), g.uv.get(f[2] as usize)) {
                        let (a, b, cc) = (to_s(*a), to_s(*b), to_s(*cc));
                        draw_line(a.x, a.y, b.x, b.y, 1.0, c);
                        draw_line(b.x, b.y, cc.x, cc.y, 1.0, c);
                        draw_line(cc.x, cc.y, a.x, a.y, 1.0, c);
                    }
                }
            }
            // this level's green anchors (the canvas's for the level being edited)
            let anchors = if l == ed.target { ed.green() } else { ed.lods[l].anchors.clone() };
            let green = Color::new(0.25, 0.85, 0.35, 1.0);
            for a in &anchors {
                let s = vec2(pv.x + a.pos.x.clamp(0.0, 1.0) * pv.w, pv.y + a.pos.y.clamp(0.0, 1.0) * pv.h);
                let r = a.angle.to_radians();
                let e = s + vec2(r.sin(), -r.cos()) * 12.0;
                draw_line(s.x, s.y, e.x, e.y, 1.5, green);
                draw_circle(s.x, s.y, 3.5, green);
            }
            let vf = if anchors.is_empty() { format!("V:{} F:{}", nv, nf) } else { format!("V:{} F:{} A:{}", nv, nf, anchors.len()) };
            let tw = ui.text_w(&vf, 11.0);
            ui.text(&vf, pv.x + (pv.w - tw) * 0.5, pv.y + pv.h + 3.0, 11.0, C_TEXT_DIM);
            ui.record(&format!("cutout.level.{}", l), pv);
            // arrows (the original's LoadButton / SaveButton): <- edit this level in the canvas,
            // -> put the canvas's outline, colours and anchors into this level
            let lb = Rect::new(col.x, y + h * 0.35, 28.0, 24.0);
            let rb = Rect::new(col.x, y + h * 0.35 + 30.0, 28.0, 24.0);
            if ui.button(&format!("cut.load{}", l), lb, "<-") && l != ed.target {
                if ed.outline.len() >= 3 || ed.lods[ed.target].geom.is_none() {
                    ed.lods[ed.target] = ed.as_level();
                } else {
                    ed.lods[ed.target].anchors = ed.green();
                }
                ed.target = l;
                ed.load_level(l);
                if ed.lods[l].geom.is_some() && ed.outline.len() < 3 {
                    ed.warn = Some(("This level holds mesh geometry; add points to replace it with a cutout".into(), get_time()));
                }
            }
            tip(ui, lb, "Edit this level in the window (outline, colours and anchors)");
            if ui.button(&format!("cut.send{}", l), rb, "->") {
                if ed.outline.len() >= 3 {
                    ed.lods[l] = ed.as_level();
                } else if !ed.green().is_empty() {
                    ed.lods[l].anchors = ed.green(); // anchors alone go over
                } else {
                    ed.warn = Some(("Draw an outline first (at least 3 points)".into(), get_time()));
                }
            }
            tip(ui, rb, "Copy the window's outline, colours and anchors into this level");
        }
    }
}

/// How many anchor numbers the right-click menu offers: one per green anchor, at least 5 (the
/// original's count), and never fewer than the highest number already used.
fn id_count(ed: &CutoutEditor) -> usize {
    let used = ed.anchor_ids.iter().skip(1).copied().max().unwrap_or(0);
    5usize.max(ed.anchors.len().saturating_sub(1)).max(used)
}

/// An angle in degrees brought into -180..180.
fn wrap_deg(d: f32) -> f32 {
    (d + 180.0).rem_euclid(360.0) - 180.0
}

/// A Yes/No question, as the original's QMessageBox::question (Yes | No).
fn confirm(title: &str, text: &str) -> bool {
    rfd::MessageDialog::new().set_title(title).set_description(text).set_buttons(rfd::MessageButtons::YesNo).show() == rfd::MessageDialogResult::Yes
}

fn group(ui: &mut Ui, r: Rect, title: &str) {
    draw_rectangle_lines(r.x, r.y + 6.0, r.w, r.h - 6.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.14));
    let tw = ui.text_w(title, 12.0);
    draw_rectangle(r.x + 6.0, r.y, tw + 8.0, 12.0, C_PANEL);
    ui.text(title, r.x + 10.0, r.y - 1.0, 12.0, C_TEXT_DIM);
}

fn tip(ui: &mut Ui, r: Rect, s: &str) {
    if ui.hover(r) {
        ui.tooltip = Some(s.to_string());
    }
}

fn tool(ui: &mut Ui, cur: &mut Tool, me: Tool, r: Rect, key: &str, label: &str, tooltip: &str) {
    if ui.button_ex(key, r, label, *cur == me, true) {
        *cur = me;
    }
    tip(ui, r, tooltip);
}

/// The part of segment a-b inside rectangle r (Liang-Barsky).
fn clip_segment(a: Vec2, b: Vec2, r: Rect) -> Option<(Vec2, Vec2)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-d.x, a.x - r.x), (d.x, r.x + r.w - a.x), (-d.y, a.y - r.y), (d.y, r.y + r.h - a.y)] {
        if p.abs() < 1e-9 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    if t0 > t1 {
        None
    } else {
        Some((a + d * t0, a + d * t1))
    }
}

/// A thumb wheel like the original's CThumbWheel (fn 0x140409be0): drag left or right to turn
/// the value in steps of 2 degrees (the step the Edit Cutout window sets, fn 0x1402f83f0).
fn thumb_wheel(ui: &mut Ui, key: &str, r: Rect, v: &mut f32) -> bool {
    let id = hid(key);
    ui.record(key, r);
    draw_rectangle(r.x, r.y, r.w, r.h, C_FIELD);
    // ridges that slide with the value
    let n = (r.w / 8.0) as i32;
    let off = (*v * 2.0).rem_euclid(8.0);
    for i in 0..=n {
        let x = r.x + i as f32 * 8.0 + off;
        if x < r.x + r.w - 1.0 {
            draw_line(x, r.y + 2.0, x, r.y + r.h - 2.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.12));
        }
    }
    let label = format!("{:.1}°", v);
    let tw = ui.text_w(&label, 11.5);
    ui.text(&label, r.x + r.w - tw - 4.0, r.y + 2.0, 11.5, C_TEXT);
    let mut changed = false;
    if ui.hover(r) && ui.active == 0 && ui.inp.l_pressed {
        if ui.inp.l_double {
            *v = 0.0;
            changed = true;
        } else {
            ui.active = id;
            ui.drag_start_x = ui.inp.mx;
        }
    }
    if ui.active == id {
        if ui.inp.l_down {
            // smooth, no snapping (the person's request): half a degree per pixel dragged
            let dx = ui.inp.mx - ui.drag_start_x;
            if dx != 0.0 {
                // the original's step: 0.005 radians per wheel unit (fn 0x1402f9190), no rounding
                *v = ((*v + (dx * 0.005).to_degrees() + 180.0).rem_euclid(360.0)) - 180.0;
                ui.drag_start_x = ui.inp.mx;
                changed = true;
            }
        } else {
            ui.active = 0;
        }
    }
    changed
}

fn radio(ui: &mut Ui, r: Rect, label: &str, on: bool) -> bool {
    let c = vec2(r.x + 7.0, r.y + r.h * 0.5);
    draw_circle_lines(c.x, c.y, 6.0, 1.2, C_TEXT_DIM);
    if on {
        draw_circle(c.x, c.y, 3.0, C_TEXT);
    }
    ui.text(label, r.x + 18.0, r.y + 2.0, 12.0, if on { C_TEXT } else { C_TEXT_DIM });
    ui.clicked(r)
}

/// Where a new point goes: next to the closest existing point (on whichever side keeps the
/// outline from crossing itself and adds the shorter edge), so it connects to that point.
fn insert_index(poly: &[Vec2], p: Vec2) -> usize {
    if poly.len() < 3 {
        return poly.len();
    }
    let n = poly.len();
    // a click inside the shape cuts a notch into it (picture-16): the point goes into the edge
    // closest to it, or the next closest that keeps the outline from crossing itself
    if point_in_poly(poly, p) {
        let mut edges: Vec<(f32, usize)> = (0..n).map(|k| (seg_point_dist(poly[k], poly[(k + 1) % n], p), k + 1)).collect();
        edges.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (_, at) in edges {
            let mut next = poly.to_vec();
            next.insert(at, p);
            if is_simple(&next) {
                return at;
            }
        }
    }
    let k = (0..n).min_by(|&a, &b| poly[a].distance(p).total_cmp(&poly[b].distance(p))).unwrap_or(0);
    // before k (between k-1 and k) or after k (between k and k+1)
    let cost = |at: usize| {
        let (a, b) = (poly[(at + n - 1) % n], poly[at % n]);
        a.distance(p) + p.distance(b) - a.distance(b)
    };
    let mut opts = [(k, cost(k)), (k + 1, cost(k + 1))];
    opts.sort_by(|x, y| x.1.total_cmp(&y.1));
    for (at, _) in opts {
        let mut next = poly.to_vec();
        next.insert(at, p);
        if is_simple(&next) {
            return at;
        }
    }
    // otherwise the cheapest edge (least added length) that keeps the outline simple
    let mut all: Vec<(f32, usize)> = (1..=n).map(|at| (cost(at), at)).collect();
    all.sort_by(|x, y| x.0.total_cmp(&y.0));
    for (_, at) in all {
        let mut next = poly.to_vec();
        next.insert(at, p);
        if is_simple(&next) {
            return at;
        }
    }
    insert_nearest_edge(poly, p)
}

fn seg_point_dist(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
    (a + ab * t).distance(p)
}

/// Even-odd test: is p inside the closed outline?
fn point_in_poly(poly: &[Vec2], p: Vec2) -> bool {
    let n = poly.len();
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// Fallback: after the start of the nearest edge.
fn insert_nearest_edge(poly: &[Vec2], p: Vec2) -> usize {
    let mut best = (f32::MAX, 0usize);
    for k in 0..poly.len() {
        let a = poly[k];
        let b = poly[(k + 1) % poly.len()];
        let ab = b - a;
        let t = ((p - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
        let d = (a + ab * t).distance(p);
        if d < best.0 {
            best = (d, k);
        }
    }
    best.1 + 1
}

fn cross(o: Vec2, a: Vec2, b: Vec2) -> f32 {
    (a - o).perp_dot(b - o)
}

fn segments_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let (d1, d2) = (cross(c, d, a), cross(c, d, b));
    let (d3, d4) = (cross(a, b, c), cross(a, b, d));
    (d1 * d2 < 0.0) && (d3 * d4 < 0.0)
}

/// True when no two edges of the closed outline cross (concave is fine, overlapping isn't).
pub fn is_simple(poly: &[Vec2]) -> bool {
    let n = poly.len();
    if n < 4 {
        return true;
    }
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        for j in i + 1..n {
            if j == i || (j + 1) % n == i || (i + 1) % n == j {
                continue;
            }
            let (c, d) = (poly[j], poly[(j + 1) % n]);
            if segments_cross(a, b, c, d) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_point_joins_the_closest_point() {
        // a tall thin strip; a point added just left of the middle of the left side must sit
        // between the two left corners, next to the closest one
        let strip = vec![vec2(0.1, 0.0), vec2(0.2, 0.0), vec2(0.2, 1.0), vec2(0.1, 1.0)];
        let p = vec2(0.08, 0.6);
        let at = insert_index(&strip, p);
        let mut next = strip.clone();
        next.insert(at, p);
        assert!(is_simple(&next));
        let i = next.iter().position(|q| *q == p).unwrap();
        let n = next.len();
        let nb = [next[(i + n - 1) % n], next[(i + 1) % n]];
        assert!(nb.contains(&vec2(0.1, 1.0)), "should join the closest corner: {:?}", nb);
    }

    #[test]
    fn an_inner_point_joins_the_mesh_at_its_nearest_vertex() {
        let sq = vec![vec2(0.1, 0.1), vec2(0.9, 0.1), vec2(0.9, 0.9), vec2(0.1, 0.9)];
        // near the top-right corner (index 1), off the triangles' shared diagonal
        let p = vec2(0.75, 0.3);
        let g = meshio::outline_geom_inner(&sq, &[], &[p], &[], vec2(0.5, 0.9), CutOpts::default());
        assert_eq!(g.uv.len(), 5, "the outline stays, plus one inner vertex");
        assert_eq!(g.idx.len() / 3, 4, "a square with one inner point has 4 triangles");
        let edge = g.idx.chunks(3).any(|t| t.contains(&4) && t.contains(&1));
        assert!(edge, "the inner point is joined to its nearest corner");
        assert!(point_in_poly(&sq, p));
    }

    #[test]
    fn painted_colours_reach_the_mesh() {
        let card = vec![vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)];
        let cols = vec![[1.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let o = CutOpts { tessellation: 1.0 / 3.0, ..Default::default() };
        let g = meshio::outline_geom_col(&card, &cols, vec2(0.5, 1.0), o);
        assert_eq!(g.col.len(), g.pos.len());
        assert_eq!(g.col[0], [1.0, 0.0, 0.0]);
        // a point added between red and blue corners is a mix of both
        assert!(g.col[card.len()..].iter().any(|c| c[0] > 0.01 && c[2] > 0.01 && (c[0] + c[2] - 1.0).abs() < 1e-4));
    }

    #[test]
    fn anchor_points_follow_their_angles() {
        let mut m = LeafMesh::card("t");
        m.anchor = vec2(0.5, 1.0);
        m.lods[0].anchors = vec![CutAnchor { pos: vec2(0.5, 0.5), angle: 0.0, id: 1, side: 0 }, CutAnchor { pos: vec2(0.5, 0.5), angle: 90.0, id: 3, side: 2 }];
        assert_eq!(meshio::anchor_points_h(&m)[1].3, 3); // numbers travel with the anchors
        let a = meshio::anchor_points(&m);
        assert!((a[0].0 - vec3(0.0, 0.5, 0.0)).length() < 1e-4);
        assert!((a[0].1 - vec3(0.0, 1.0, 0.0)).length() < 1e-4); // straight up = along the leaf
        assert!((a[1].1 - vec3(1.0, 0.0, 0.0)).length() < 1e-4); // 90 degrees = to the right
    }
}