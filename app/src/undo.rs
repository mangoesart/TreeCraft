//! Undo / Redo with no limit (picture-20). After each finished edit (mouse released, nothing being
//! dragged) the tree's state is compared with the last one kept; when it differs, the old state
//! goes on the undo stack. Kept: the generators, scene objects (forces, collision objects), leaf
//! meshes (cutouts) and materials. Materials are kept without their picture buffers (those can be
//! large); a restored material takes its pictures back from the current one with the same file,
//! or rebuilds them, so the stacks stay small however long they grow.

use crate::app::{App, Sel};
use crate::model::*;
use std::fmt::Write as _;

pub struct Snapshot {
    nodes: Vec<Node>,
    next_id: u32,
    scene: Vec<SceneObj>,
    materials: Vec<Material>,
    meshes: Vec<LeafMesh>,
    /// Custom cameras (picture 51: Undo takes them back too).
    cameras: Vec<crate::cameras::ViewCam>,
    sig: String,
    /// What the edit that led to this state was ("Add Big Branches", "Edit Trunk" ...), shown in
    /// the Edit menu as "Undo <what>" like the original (fn 0x1402a93a0).
    what: String,
}

/// Name the edit between two kept states.
fn describe(prev: &Snapshot, app: &App) -> String {
    if app.nodes.len() > prev.nodes.len() {
        if let Some(n) = app.nodes.iter().find(|n| !prev.nodes.iter().any(|p| p.id == n.id)) {
            return format!("Add {}", n.name);
        }
    }
    if app.nodes.len() < prev.nodes.len() {
        if let Some(n) = prev.nodes.iter().find(|p| !app.nodes.iter().any(|n| n.id == p.id)) {
            return format!("Delete {}", n.name);
        }
    }
    for n in &app.nodes {
        if let Some(p) = prev.nodes.iter().find(|p| p.id == n.id) {
            if format!("{:?}", p) != format!("{:?}", n) {
                return format!("Edit {}", n.name);
            }
        }
    }
    if app.scene.len() != prev.scene.len() || format!("{:?}", app.scene) != format!("{:?}", prev.scene) {
        return if app.scene.len() > prev.scene.len() { format!("Add {}", app.scene.last().map(|o| o.name.as_str()).unwrap_or("object")) } else { "Edit scene objects".into() };
    }
    if app.cameras.len() > prev.cameras.len() {
        return format!("Add {}", app.cameras.last().map(|c| c.name.as_str()).unwrap_or("camera"));
    }
    if app.cameras.len() < prev.cameras.len() {
        return "Delete camera".into();
    }
    for (a, b) in app.cameras.iter().zip(&prev.cameras) {
        if format!("{:?}", a) != format!("{:?}", b) {
            return if a.pos != b.pos || a.yaw != b.yaw || a.pitch != b.pitch { format!("Move {}", a.name) } else { format!("Edit {}", a.name) };
        }
    }
    if app.meshes.len() != prev.meshes.len() {
        return "Meshes".into();
    }
    if app.materials.len() != prev.materials.len() {
        return "Materials".into();
    }
    // a material or mesh setting
    for (a, b) in app.meshes.iter().zip(&prev.meshes) {
        if a.name != b.name || a.anchor != b.anchor || a.lods.iter().zip(&b.lods).any(|(x, y)| x.outline != y.outline || x.anchors != y.anchors) {
            return format!("Edit {}", a.name);
        }
    }
    "Edit".into()
}

/// A text fingerprint of everything Undo keeps (no pictures): equal text, same state.
fn signature(app: &App) -> String {
    let mut s = format!("{:?}|{:?}|{:?}|", app.nodes, app.scene, app.cameras);
    for m in &app.materials {
        let _ = write!(
            s,
            "M{}|{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}|{:?}|{}|{}|{}|{:?}|{}|{}|{:?}|{:?}|{}|{}|{}|{}|{}|{:?}|{}|{:?}|",
            m.name,
            m.two_sided,
            m.source,
            m.color,
            m.file,
            m.brightness,
            m.contrast,
            m.saturation,
            m.hue,
            m.opacity_from_alpha,
            m.cutoff,
            m.normal_strength,
            m.specular_color,
            m.gloss,
            m.sss_color,
            m.metallic,
            m.sss_amount,
            m.ao,
            m.var_colors,
            m.variation,
            m.cutout_mesh,
            m.custom_color,
            m.season_color,
            m.opacity,
            m.atlas_scale,
            m.atlas_mode,
            m.max_res,
            m.user_data,
            m.mesh_rows,
            m.is_set,
            m.set_members,
        );
        for sl in &m.slots {
            let _ = write!(s, "{}{}|", sl.enabled, sl.file);
        }
    }
    for m in &app.meshes {
        let _ = write!(
            s,
            "L{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|",
            m.name,
            m.more,
            m.anchor,
            m.ref_material,
            m.auto_points,
            m.threshold,
            m.margin,
            m.scale,
            m.orient,
            m.flip,
            m.flip_normals,
            m.fix_winding,
            m.collection,
            m.spine_style,
            m.pivot_style,
            m.growth,
            m.ripple,
            m.collision_scale,
            m.weight,
            m.embedded,
            m.tessellation,
            m.angle,
            m.double_sided,
            m.improve_folding,
        );
        for l in &m.lods {
            let _ = write!(
                s,
                "{}|{:?}|{:?}|{:?}|{:?}|{:?}|{}|",
                l.file,
                l.outline,
                l.anchors,
                l.colors,
                l.inner,
                l.inner_colors,
                l.geom.as_ref().map(|g| g.pos.len()).unwrap_or(0)
            );
        }
    }
    s
}

/// A material without its picture buffers (its GPU textures are shared handles, kept).
fn light_material(m: &Material) -> Material {
    let mut c = m.clone();
    c.base = None;
    c.adjusted = None;
    for sl in &mut c.slots {
        sl.image = None;
    }
    c
}

impl App {
    fn snapshot(&self, sig: String, what: String) -> Snapshot {
        Snapshot {
            nodes: self.nodes.clone(),
            next_id: self.next_id,
            scene: self.scene.clone(),
            materials: self.materials.iter().map(light_material).collect(),
            meshes: self.meshes.clone(),
            cameras: self.cameras.clone(),
            sig,
            what,
        }
    }

    /// The Edit menu's labels, as the original words them: "Undo <edit>" or "Nothing to undo",
    /// "Redo <edit>" or "Nothing to redo".
    pub fn undo_label(&self) -> String {
        match (&self.undo_cur, self.undo_stack.is_empty()) {
            (Some(c), false) => format!("Undo {}", c.what),
            _ => "Nothing to undo".into(),
        }
    }
    pub fn redo_label(&self) -> String {
        match self.redo_stack.last() {
            Some(n) => format!("Redo {}", n.what),
            None => "Nothing to redo".into(),
        }
    }

    /// Forget all history and start it from the current state.
    pub fn undo_reset(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        let sig = signature(self);
        self.saved_sig = sig.clone();
        self.undo_cur = Some(self.snapshot(sig, String::new()));
    }

    /// The tree differs from when it was last saved, opened or started.
    pub fn is_modified(&self) -> bool {
        signature(self) != self.saved_sig
    }
    pub fn mark_saved(&mut self) {
        self.saved_sig = signature(self);
    }

    /// Called once a frame; `idle` is true when no edit is in progress (mouse up, nothing dragged).
    pub fn undo_track(&mut self, idle: bool) {
        if !idle {
            return;
        }
        let sig = signature(self);
        if self.undo_cur.as_ref().map(|c| c.sig == sig).unwrap_or(false) {
            return;
        }
        let what = self.undo_cur.as_ref().map(|p| describe(p, self)).unwrap_or_default();
        let snap = self.snapshot(sig, what);
        if let Some(prev) = self.undo_cur.replace(snap) {
            self.undo_stack.push(prev);
            self.redo_stack.clear();
        }
    }

    pub fn undo(&mut self) {
        self.undo_track(true);
        match self.undo_stack.pop() {
            Some(prev) => {
                let mut what = String::new();
                if let Some(cur) = self.undo_cur.take() {
                    what = cur.what.clone();
                    self.redo_stack.push(cur);
                }
                self.restore(&prev);
                self.undo_cur = Some(prev);
                self.set_status(&format!("Undo {}", what));
            }
            None => self.set_status("Nothing to undo"),
        }
    }

    pub fn redo(&mut self) {
        self.undo_track(true);
        match self.redo_stack.pop() {
            Some(next) => {
                if let Some(cur) = self.undo_cur.take() {
                    self.undo_stack.push(cur);
                }
                let what = next.what.clone();
                self.restore(&next);
                self.undo_cur = Some(next);
                self.set_status(&format!("Redo {}", what));
            }
            None => self.set_status("Nothing to redo"),
        }
    }

    fn restore(&mut self, s: &Snapshot) {
        self.nodes = s.nodes.clone();
        self.next_id = s.next_id;
        self.scene = s.scene.clone();
        self.meshes = s.meshes.clone();
        self.cameras = s.cameras.clone();
        if self.looking.map_or(false, |l| l >= self.cameras.len()) {
            self.look_through(None);
        } else if self.looking.is_some() {
            // the view follows the restored camera
            self.look_cam = None;
        }
        let mut mats = s.materials.clone();
        for m in &mut mats {
            // pictures back from the current material with the same picture source, else rebuilt
            if let Some(cur) = self.materials.iter().find(|c| c.name == m.name && c.file == m.file && c.source == m.source) {
                m.base = cur.base.clone();
                for (sl, cs) in m.slots.iter_mut().zip(&cur.slots) {
                    if sl.file == cs.file {
                        sl.image = cs.image.clone();
                    }
                }
            }
            m.dirty_base = m.base.is_none();
            m.dirty_tex = true;
        }
        self.materials = mats;
        // a selection that no longer exists is dropped
        match self.sel {
            Sel::Node(id) if !self.nodes.iter().any(|n| n.id == id) => self.sel = Sel::None,
            Sel::Material(k) if k >= self.materials.len() => self.sel = Sel::None,
            Sel::Mesh(k) if k >= self.meshes.len() => self.sel = Sel::None,
            Sel::Camera(k) if k >= self.cameras.len() => self.sel = Sel::None,
            _ => {}
        }
        if self.scene_sel.map(|k| k >= self.scene.len()).unwrap_or(false) {
            self.scene_sel = None;
        }
        self.mat_cur = self.mat_cur.min(self.materials.len().saturating_sub(1));
        self.dirty = true;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
