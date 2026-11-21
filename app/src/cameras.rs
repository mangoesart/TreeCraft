//! Custom cameras (picture 50): added from the viewport's view menu where the view is (a
//! perspective camera from the perspective view, an orthographic one locked to Front, Back, Left,
//! Right, Top or Bottom from those), drawn as a yellow camera icon that can be moved and turned
//! in the perspective view, looked through from the view menu, and edited in the Properties
//! panel: name, lens (focal length, field of view, sensor, orthographic scale, clipping, shift),
//! position and rotation, frame (resolution, %, presets, frame overlay), depth of field (focus
//! distance, f-stop, blades, the sharp range) and display (icon size, name, limits).
use crate::app::{App, Sel};
use crate::props::Rows;
use crate::render::{Cam, VIEWS};
use crate::ui::*;
use macroquad::prelude::*;

/// The camera icon's yellow (picture 50).
pub const CAM_YELLOW: Color = Color::new(1.0, 0.82, 0.18, 1.0);

/// The 3D camera (picture 53): modelled on the camera picture the person supplied (picture 52)
/// in its blue: two film reels with five holes and a hub on each face, a body with a side panel
/// and two buttons, a knob at the back and an open lens hood at the front. Local axes: x right,
/// y up, z the way it looks; about 1.4 long, scaled by Icon size.
struct CamMesh {
    pos: Vec<Vec3>,
    nrm: Vec<Vec3>,
    col: Vec<[f32; 3]>,
    idx: Vec<u16>,
}

const BLUE: [f32; 3] = [0.38, 0.64, 0.96];
const BLUE_LIGHT: [f32; 3] = [0.56, 0.78, 1.0];
const BLUE_DARK: [f32; 3] = [0.22, 0.45, 0.80];

impl CamMesh {
    fn quad(&mut self, q: [Vec3; 4], n: Vec3, c: [f32; 3]) {
        let b = self.pos.len() as u16;
        for p in q {
            self.pos.push(p);
            self.nrm.push(n);
            self.col.push(c);
        }
        self.idx.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
    }
    fn cuboid(&mut self, c: Vec3, h: Vec3, col: [f32; 3]) {
        for (axis, sgn) in [(0, 1.0f32), (0, -1.0), (1, 1.0), (1, -1.0), (2, 1.0), (2, -1.0)] {
            let mut n = Vec3::ZERO;
            n[axis] = sgn;
            let (u, v) = match axis {
                0 => (Vec3::Y, Vec3::Z),
                1 => (Vec3::Z, Vec3::X),
                _ => (Vec3::X, Vec3::Y),
            };
            let f = c + n * h;
            let (hu, hv) = (u * h, v * h);
            self.quad([f - hu - hv, f + hu - hv, f + hu + hv, f - hu + hv], n, col);
        }
    }
    /// A disc facing `n` (both ways when `both`).
    fn disc(&mut self, c: Vec3, n: Vec3, r: f32, seg: usize, col: [f32; 3]) {
        let (u, v) = basis_of(n);
        let b = self.pos.len() as u16;
        self.pos.push(c);
        self.nrm.push(n);
        self.col.push(col);
        for k in 0..=seg {
            let a = k as f32 / seg as f32 * std::f32::consts::TAU;
            self.pos.push(c + (u * a.cos() + v * a.sin()) * r);
            self.nrm.push(n);
            self.col.push(col);
        }
        for k in 0..seg as u16 {
            self.idx.extend_from_slice(&[b, b + 1 + k, b + 2 + k]);
        }
    }
    /// A cylinder along `axis` from `c - axis*half` to `c + axis*half`, with caps.
    fn cylinder(&mut self, c: Vec3, axis: Vec3, r: f32, half: f32, seg: usize, side: [f32; 3], cap: [f32; 3]) {
        let (u, v) = basis_of(axis);
        for k in 0..seg {
            let a0 = k as f32 / seg as f32 * std::f32::consts::TAU;
            let a1 = (k + 1) as f32 / seg as f32 * std::f32::consts::TAU;
            let (d0, d1) = (u * a0.cos() + v * a0.sin(), u * a1.cos() + v * a1.sin());
            let n = (d0 + d1).normalize_or_zero();
            self.quad([c - axis * half + d0 * r, c - axis * half + d1 * r, c + axis * half + d1 * r, c + axis * half + d0 * r], n, side);
        }
        self.disc(c + axis * half, axis, r, seg, cap);
        self.disc(c - axis * half, -axis, r, seg, cap);
    }
    /// A film reel: its axis along x, holes and a hub on both faces.
    fn reel(&mut self, c: Vec3, r: f32, half: f32) {
        self.cylinder(c, Vec3::X, r, half, 28, BLUE, BLUE_LIGHT);
        for s in [1.0f32, -1.0] {
            let face = c + Vec3::X * (half + 0.004) * s;
            for k in 0..5 {
                let a = std::f32::consts::FRAC_PI_2 + k as f32 / 5.0 * std::f32::consts::TAU;
                let hc = face + (Vec3::Y * a.sin() + Vec3::Z * a.cos()) * r * 0.58;
                self.disc(hc, Vec3::X * s, r * 0.24, 14, BLUE_DARK);
            }
            self.disc(face + Vec3::X * 0.002 * s, Vec3::X * s, r * 0.22, 14, BLUE);
        }
    }
    fn build() -> CamMesh {
        let mut m = CamMesh { pos: vec![], nrm: vec![], col: vec![], idx: vec![] };
        // body and its side panels with two buttons each
        m.cuboid(vec3(0.0, -0.10, -0.03), vec3(0.26, 0.22, 0.39), BLUE);
        for s in [1.0f32, -1.0] {
            m.cuboid(vec3(0.265 * s, -0.10, -0.12), vec3(0.012, 0.11, 0.17), BLUE_LIGHT);
            for y in [-0.03f32, -0.18] {
                m.cylinder(vec3(0.27 * s, y, 0.19), Vec3::X, 0.045, 0.02, 16, BLUE_LIGHT, BLUE_LIGHT);
            }
        }
        // two reels on top, the front one a little higher
        m.reel(vec3(0.0, 0.40, -0.22), 0.27, 0.11);
        m.reel(vec3(0.0, 0.43, 0.20), 0.28, 0.11);
        // the knob at the back
        m.cylinder(vec3(0.0, -0.10, -0.49), Vec3::Z, 0.12, 0.07, 20, BLUE, BLUE_LIGHT);
        // lens collar and the open hood
        m.cylinder(vec3(0.0, -0.10, 0.41), Vec3::Z, 0.16, 0.05, 24, BLUE, BLUE);
        let (z0, z1, a, b, y) = (0.46f32, 0.78f32, 0.12f32, 0.26f32, -0.10f32);
        let ring = |z: f32, h: f32| [vec3(h, y + h, z), vec3(-h, y + h, z), vec3(-h, y - h, z), vec3(h, y - h, z)];
        let (n0, n1) = (ring(z0, a), ring(z1, b));
        for k in 0..4 {
            let j = (k + 1) % 4;
            let q = [n0[k], n0[j], n1[j], n1[k]];
            let mid = (q[0] + q[1] + q[2] + q[3]) * 0.25 - vec3(0.0, y, (z0 + z1) * 0.5);
            let n = vec3(mid.x, mid.y, 0.0).normalize_or_zero() * 0.8 + Vec3::NEG_Z * 0.2;
            m.quad(q, n.normalize_or_zero(), BLUE);
            // the inside, darker
            let inset = |p: Vec3| vec3(p.x * 0.97, y + (p.y - y) * 0.97, p.z);
            m.quad([inset(q[0]), inset(q[1]), inset(q[2]), inset(q[3])], -n.normalize_or_zero(), BLUE_DARK);
        }
        // the hood's rim
        for k in 0..4 {
            let j = (k + 1) % 4;
            let inner = |p: Vec3| vec3(p.x * 0.85, y + (p.y - y) * 0.85, p.z);
            m.quad([n1[k], n1[j], inner(n1[j]), inner(n1[k])], Vec3::Z, BLUE_LIGHT);
        }
        // centred on its middle: the body's middle sits at the camera's place
        m
    }
}

fn basis_of(n: Vec3) -> (Vec3, Vec3) {
    let t = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = t.cross(n).normalize_or(Vec3::X);
    (u, n.cross(u).normalize_or(Vec3::Y))
}

fn cam_mesh() -> &'static CamMesh {
    static M: std::sync::OnceLock<CamMesh> = std::sync::OnceLock::new();
    M.get_or_init(CamMesh::build)
}

/// Frame presets: name, width, height.
pub const FRAME_PRESETS: [(&str, i32, i32); 8] = [
    ("Custom", 0, 0),
    ("HD 1080p (1920 x 1080)", 1920, 1080),
    ("HD 720p (1280 x 720)", 1280, 720),
    ("4K UHD (3840 x 2160)", 3840, 2160),
    ("Square (1080 x 1080)", 1080, 1080),
    ("Portrait (1080 x 1920)", 1080, 1920),
    ("Cinema 2.39:1 (2048 x 858)", 2048, 858),
    ("Photo 3:2 (3000 x 2000)", 3000, 2000),
];

#[derive(Clone, Debug)]
pub struct ViewCam {
    pub name: String,
    /// Orthographic, locked to `view` (1 Front .. 6 Bottom); perspective has view 0.
    pub ortho: bool,
    pub view: usize,
    pub pos: Vec3,
    /// Heading and tilt as the viewport's camera keeps them (radians), and roll (radians).
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    /// How far ahead it looks at (its orbit point when looking through it).
    pub dist: f32,
    pub focal: f32,
    pub sensor: f32,
    pub ortho_scale: f32,
    pub auto_clip: bool,
    pub near: f32,
    pub far: f32,
    pub shift_x: f32,
    pub shift_y: f32,
    pub res_x: i32,
    pub res_y: i32,
    pub res_pct: f32,
    pub show_frame: bool,
    pub passepartout: f32,
    pub dof: bool,
    pub focus_dist: f32,
    pub fstop: f32,
    pub blades: i32,
    pub show_focus: bool,
    pub icon_size: f32,
    pub show_name: bool,
    pub show_limits: bool,
    pub lock_to_view: bool,
    /// Helper lines on the selected camera (picture 51): focal length / field of view along its
    /// view line, the frame's width and height beside its frame.
    pub show_guides: bool,
    /// Hidden from the viewport (Scene objects menu, picture 65); still usable for rendering.
    pub hidden: bool,
}

impl ViewCam {
    /// A camera where the viewport's view is: same place, direction and zoom.
    pub fn from_view(name: String, cam: &Cam, view_aspect: f32) -> ViewCam {
        let aspect = view_aspect.clamp(0.2, 5.0);
        let res_x = 1920;
        let res_y = ((res_x as f32 / aspect).round() as i32).max(16);
        let mut c = ViewCam {
            name,
            ortho: cam.ortho && cam.view != 0,
            view: if cam.ortho { cam.view } else { 0 },
            pos: cam.target + cam.dir() * cam.dist,
            yaw: cam.yaw,
            pitch: cam.pitch,
            roll: 0.0,
            dist: cam.dist,
            focal: 50.0,
            sensor: 36.0,
            ortho_scale: cam.view_height(),
            auto_clip: true,
            near: 0.1,
            far: 1000.0,
            shift_x: 0.0,
            shift_y: 0.0,
            res_x,
            res_y,
            res_pct: 100.0,
            show_frame: true,
            passepartout: 0.5,
            dof: false,
            focus_dist: cam.dist,
            fstop: 2.8,
            blades: 6,
            show_focus: true,
            icon_size: 1.0,
            show_name: true,
            show_limits: false,
            lock_to_view: false,
            show_guides: true,
            hidden: false,
        };
        // a perspective view that was made orthographic by hand stays a perspective camera
        if cam.ortho && cam.view == 0 {
            c.ortho = false;
        }
        c.set_vfov(cam.fov.to_radians());
        c
    }
    pub fn aspect(&self) -> f32 {
        self.res_x.max(1) as f32 / self.res_y.max(1) as f32
    }
    /// Horizontal field of view (radians) from the focal length and sensor width.
    pub fn hfov(&self) -> f32 {
        2.0 * (self.sensor.max(0.1) / (2.0 * self.focal.max(0.1))).atan()
    }
    pub fn vfov(&self) -> f32 {
        2.0 * ((self.hfov() * 0.5).tan() / self.aspect()).atan()
    }
    pub fn set_vfov(&mut self, v: f32) {
        let h = 2.0 * ((v * 0.5).tan() * self.aspect()).atan();
        self.focal = (self.sensor / (2.0 * (h * 0.5).tan().max(1e-4))).clamp(1.0, 2000.0);
    }
    /// The way it looks (unit), right and up, with roll.
    pub fn basis(&self) -> (Vec3, Vec3, Vec3) {
        let fwd = -vec3(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos());
        let up0 = if self.pitch.abs() > 1.5 { vec3(-self.yaw.sin(), 0.0, -self.yaw.cos()) * self.pitch.signum() } else { Vec3::Y };
        let right = fwd.cross(up0).normalize_or(Vec3::X);
        let up = right.cross(fwd).normalize_or(Vec3::Y);
        let (s, c) = self.roll.sin_cos();
        (fwd, right * c - up * s, up * c + right * s)
    }
    /// The viewport's camera that looks through it, with its frame filling `frac_h` of the
    /// viewport's height.
    pub fn to_cam(&self, frac_h: f32) -> Cam {
        let (fwd, _, _) = self.basis();
        let dist = self.dist.max(0.5);
        let target = self.pos + fwd * dist;
        let fov = if self.ortho {
            2.0 * ((self.ortho_scale / frac_h.max(0.05)) / (2.0 * dist)).atan()
        } else {
            2.0 * ((self.vfov() * 0.5).tan() / frac_h.max(0.05)).atan()
        };
        Cam {
            yaw: self.yaw,
            pitch: self.pitch,
            dist,
            target,
            ortho: self.ortho,
            view: if self.ortho { self.view } else { 0 },
            fov: fov.to_degrees().clamp(0.05, 175.0),
            roll: self.roll,
            clip: if self.auto_clip { None } else { Some((self.near, self.far)) },
        }
    }
    /// Take the viewport's place (Lock camera to view, Set to current view); the lens stays.
    pub fn take_view(&mut self, cam: &Cam) {
        self.pos = cam.target + cam.dir() * cam.dist;
        if !self.ortho {
            self.yaw = cam.yaw;
            self.pitch = cam.pitch;
        }
        self.dist = cam.dist;
    }
    /// Depth of field's sharp range (near, far; far may be infinite), with a circle of confusion
    /// of the sensor width / 1500 and scene units taken as metres.
    pub fn sharp_range(&self) -> (f32, f32) {
        let f = self.focal / 1000.0;
        let n = self.fstop.max(0.1);
        let coc = self.sensor / 1500.0 / 1000.0;
        let h = f * f / (n * coc) + f;
        let s = self.focus_dist.max(0.01);
        let near = s * (h - f) / (h + s - 2.0 * f);
        let far = if s >= h { f32::INFINITY } else { s * (h - f) / (h - s) };
        (near, far)
    }
}

fn kv(s: &mut String, k: &str, v: impl std::fmt::Display) {
    s.push_str(&format!("{}={}\n", k, v));
}

/// The cameras as tree-file lines, one [camera] section each.
pub fn write(cams: &[ViewCam], s: &mut String) {
    for c in cams {
        s.push_str("\n[camera]\n");
        kv(s, "name", c.name.replace('\n', " "));
        kv(s, "ortho", c.ortho);
        kv(s, "view", c.view);
        kv(s, "pos", format!("{},{},{}", c.pos.x, c.pos.y, c.pos.z));
        for (k, v) in [
            ("yaw", c.yaw), ("pitch", c.pitch), ("roll", c.roll), ("dist", c.dist), ("focal", c.focal), ("sensor", c.sensor),
            ("ortho_scale", c.ortho_scale), ("near", c.near), ("far", c.far), ("shift_x", c.shift_x), ("shift_y", c.shift_y),
            ("res_pct", c.res_pct), ("passepartout", c.passepartout), ("focus_dist", c.focus_dist), ("fstop", c.fstop), ("icon_size", c.icon_size),
        ] {
            kv(s, k, v);
        }
        for (k, v) in [("res_x", c.res_x), ("res_y", c.res_y), ("blades", c.blades)] {
            kv(s, k, v);
        }
        for (k, v) in [("auto_clip", c.auto_clip), ("show_frame", c.show_frame), ("dof", c.dof), ("show_focus", c.show_focus), ("show_name", c.show_name), ("show_limits", c.show_limits), ("lock_to_view", c.lock_to_view), ("show_guides", c.show_guides), ("hidden", c.hidden)] {
            kv(s, k, v);
        }
    }
}

/// One tree-file line of a [camera] section.
pub fn read_line(c: &mut ViewCam, k: &str, v: &str) {
    let f = || v.parse::<f32>().unwrap_or(0.0);
    let i = || v.parse::<i32>().unwrap_or(0);
    let b = || v == "true";
    match k {
        "name" => c.name = v.to_string(),
        "ortho" => c.ortho = b(),
        "view" => c.view = v.parse().unwrap_or(0),
        "pos" => {
            let n: Vec<f32> = v.split(',').filter_map(|x| x.parse().ok()).collect();
            if n.len() >= 3 {
                c.pos = vec3(n[0], n[1], n[2]);
            }
        }
        "yaw" => c.yaw = f(),
        "pitch" => c.pitch = f(),
        "roll" => c.roll = f(),
        "dist" => c.dist = f(),
        "focal" => c.focal = f(),
        "sensor" => c.sensor = f(),
        "ortho_scale" => c.ortho_scale = f(),
        "near" => c.near = f(),
        "far" => c.far = f(),
        "shift_x" => c.shift_x = f(),
        "shift_y" => c.shift_y = f(),
        "res_pct" => c.res_pct = f(),
        "passepartout" => c.passepartout = f(),
        "focus_dist" => c.focus_dist = f(),
        "fstop" => c.fstop = f(),
        "icon_size" => c.icon_size = f(),
        "res_x" => c.res_x = i(),
        "res_y" => c.res_y = i(),
        "blades" => c.blades = i(),
        "auto_clip" => c.auto_clip = b(),
        "show_frame" => c.show_frame = b(),
        "dof" => c.dof = b(),
        "show_focus" => c.show_focus = b(),
        "show_name" => c.show_name = b(),
        "show_limits" => c.show_limits = b(),
        "lock_to_view" => c.lock_to_view = b(),
        "show_guides" => c.show_guides = b(),
        "hidden" => c.hidden = b(),
        _ => {}
    }
}

pub fn blank() -> ViewCam {
    ViewCam::from_view("Camera".into(), &Cam::new(), 16.0 / 9.0)
}

/// Where a looked-through camera's frame sits in the viewport `r`: 90% of the limiting side.
pub fn frame_rect(r: Rect, aspect: f32) -> Rect {
    let va = r.w / r.h.max(1.0);
    let (fw, fh) = if aspect > va { (r.w * 0.9, r.w * 0.9 / aspect) } else { (r.h * 0.9 * aspect, r.h * 0.9) };
    Rect::new(r.x + (r.w - fw) * 0.5, r.y + (r.h - fh) * 0.5, fw, fh)
}

impl App {
    /// View menu > Add camera (picture 50).
    pub fn add_camera(&mut self) {
        let r = self.viewport;
        let mut k = self.cameras.len() + 1;
        while self.cameras.iter().any(|c| c.name == format!("Camera {}", k)) {
            k += 1;
        }
        let aspect = if r.w > 10.0 && r.h > 10.0 { r.w / r.h } else { 16.0 / 9.0 };
        let mut c = ViewCam::from_view(format!("Camera {}", k), &self.cam, aspect);
        // a small icon for the scene's size
        let ext = (self.bounds.1 - self.bounds.0).length();
        c.icon_size = (ext * 0.08).clamp(0.5, 20.0);
        let kind = if c.ortho { format!("orthographic ({})", VIEWS[c.view.min(VIEWS.len() - 1)]) } else { "perspective".to_string() };
        self.set_status(&format!("Added {}, a {} camera where the view is", c.name, kind));
        self.cameras.push(c);
        self.select(Sel::Camera(self.cameras.len() - 1));
    }

    /// Look through camera `i` (None: back to the view as it was before, picture 51: the
    /// camera's lens never stays on the viewport).
    pub fn look_through(&mut self, i: Option<usize>) {
        match i {
            Some(i) if i < self.cameras.len() => {
                if self.looking.is_none() {
                    self.look_saved = Some(self.cam);
                }
                self.looking = Some(i);
                self.look_cam = None;
            }
            _ => {
                if self.looking.take().is_some() {
                    match self.look_saved.take() {
                        Some(s) => self.cam = s,
                        None => {
                            self.cam.roll = 0.0;
                            self.cam.clip = None;
                            self.cam.fov = 45.0;
                        }
                    }
                }
            }
        }
    }

    /// The view was moved while looking through a camera: stay where it is now, with the
    /// viewport's own lens back (field of view, projection), not the camera's.
    fn leave_camera_here(&mut self) {
        self.looking = None;
        let saved = self.look_saved.take();
        let (fov, ortho) = saved.map(|s| (s.fov, s.ortho)).unwrap_or((45.0, false));
        // keep what's in the middle of the view the same size on screen
        if !self.cam.ortho && !ortho {
            let h = self.cam.view_height();
            self.cam.fov = fov;
            self.cam.dist = (h / (2.0 * (fov.to_radians() * 0.5).tan())).clamp(1.0, 2000.0);
        } else {
            self.cam.fov = fov;
        }
        if self.cam.view == 0 {
            self.cam.ortho = ortho;
        }
        self.cam.roll = 0.0;
        self.cam.clip = None;
    }

    pub fn delete_camera(&mut self, i: usize) {
        if i >= self.cameras.len() {
            return;
        }
        let name = self.cameras.remove(i).name;
        match self.looking {
            Some(l) if l == i => self.look_through(None),
            Some(l) if l > i => self.looking = Some(l - 1),
            _ => {}
        }
        self.sel_back.retain(|s| !matches!(s, Sel::Camera(_)));
        self.sel_fwd.retain(|s| !matches!(s, Sel::Camera(_)));
        self.sel = Sel::None;
        self.set_status(&format!("Deleted {}", name));
    }

    /// Each frame before drawing: while looking through a camera the view follows it; moving
    /// the view leaves it (or, with Lock camera to view, carries the camera along).
    pub fn update_camera_view(&mut self) {
        let Some(i) = self.looking else { return };
        if i >= self.cameras.len() {
            self.look_through(None);
            return;
        }
        let r = self.viewport;
        let frac = frame_rect(r, self.cameras[i].aspect()).h / r.h.max(1.0);
        if let Some(last) = self.look_cam {
            let moved = (self.cam.yaw - last.yaw).abs() > 1e-5 || (self.cam.pitch - last.pitch).abs() > 1e-5 || (self.cam.dist - last.dist).abs() > 1e-4 || self.cam.target.distance(last.target) > 1e-4 || self.cam.ortho != last.ortho || self.cam.view != last.view;
            if moved {
                if self.cameras[i].lock_to_view && self.cam.ortho == last.ortho {
                    let c = self.cam;
                    self.cameras[i].take_view(&c);
                } else {
                    self.leave_camera_here();
                    return;
                }
            }
        }
        let c = self.cameras[i].to_cam(frac);
        self.cam = c;
        self.look_cam = Some(c);
    }

    /// The cameras as 3D models in the scene (picture 53), lit from over the viewer's
    /// shoulder; called with the viewport's 3D camera set. The one looked through is left out.
    pub fn draw_cameras_3d(&self) {
        if self.cameras.is_empty() {
            return;
        }
        let m = cam_mesh();
        let vd = self.cam.dir();
        let vr = Vec3::Y.cross(vd).normalize_or(Vec3::X);
        let l = (vd + Vec3::Y * 0.7 - vr * 0.35).normalize_or(Vec3::Y);
        for (i, c) in self.cameras.iter().enumerate() {
            if Some(i) == self.looking || c.hidden {
                continue;
            }
            let (fwd, right, up) = c.basis();
            let k = (c.icon_size * self.icon_scale()).max(0.01);
            let verts: Vec<macroquad::models::Vertex> = m
                .pos
                .iter()
                .zip(&m.nrm)
                .zip(&m.col)
                .map(|((p, n), col)| {
                    let wp = c.pos + (right * p.x + up * p.y + fwd * p.z) * k;
                    let wn = (right * n.x + up * n.y + fwd * n.z).normalize_or_zero();
                    let d = wn.dot(l).max(0.0);
                    let sky = wn.y * 0.5 + 0.5;
                    let shade = 0.42 + 0.5 * d + 0.12 * sky;
                    let to = |v: f32| (v * shade).clamp(0.0, 1.0) * 255.0;
                    macroquad::models::Vertex { position: wp, uv: Vec2::ZERO, color: [to(col[0]) as u8, to(col[1]) as u8, to(col[2]) as u8, 255], normal: vec4(wn.x, wn.y, wn.z, 1.0) }
                })
                .collect();
            draw_mesh(&Mesh { vertices: verts, indices: m.idx.clone(), texture: None });
        }
    }

    /// Window properties > General Rendering > 3D icon scale (2.8 = each camera's own Icon size).
    pub fn icon_scale(&self) -> f32 {
        (self.wp("General Rendering:3D icon scale", 2.8) / 2.8).clamp(0.02, 10.0)
    }

    fn cam_matrix(&self, r: Rect) -> Mat4 {
        self.cam.camera3d(None, r.w / r.h.max(1.0), None).matrix()
    }

    /// The cameras in the viewport (drawn over it): yellow icons, the selected one with its
    /// move arrows and turn ring in the perspective view; clicks on them select, drags move and
    /// turn. Also the looked-through camera's frame.
    pub fn draw_cameras_overlay(&mut self, ui: &mut Ui, r: Rect) {
        // everything here stays inside the viewport (picture 55: guides ran over the panels)
        ui.set_clip(Some(r));
        self.draw_cameras_overlay_in(ui, r);
        ui.set_clip(None);
    }

    fn draw_cameras_overlay_in(&mut self, ui: &mut Ui, r: Rect) {
        let m = ui.mouse();
        let isc = self.icon_scale();
        let mat = self.cam_matrix(r);
        // points closer than this (in front of the viewer) are left out; lines are cut there
        const NEAR: f32 = 0.05;
        let to2 = move |c: Vec4| vec2(r.x + (c.x / c.w * 0.5 + 0.5) * r.w, r.y + (0.5 - c.y / c.w * 0.5) * r.h);
        let proj = |p: Vec3| -> Option<Vec2> {
            let c = mat * p.extend(1.0);
            if c.w <= NEAR {
                return None;
            }
            Some(to2(c))
        };
        let seg = |a: Vec3, b: Vec3| -> Option<(Vec2, Vec2)> {
            let (mut ca, mut cb) = (mat * a.extend(1.0), mat * b.extend(1.0));
            if ca.w <= NEAR && cb.w <= NEAR {
                return None;
            }
            if ca.w < NEAR {
                ca = ca.lerp(cb, (NEAR - ca.w) / (cb.w - ca.w));
            } else if cb.w < NEAR {
                cb = cb.lerp(ca, (NEAR - cb.w) / (ca.w - cb.w));
            }
            Some((to2(ca), to2(cb)))
        };
        // the frame of the camera looked through
        if let Some(i) = self.looking {
            if let Some(c) = self.cameras.get(i).cloned() {
                let f = frame_rect(r, c.aspect());
                let dim = Color::new(0.0, 0.0, 0.0, c.passepartout.clamp(0.0, 1.0));
                if c.show_frame {
                    draw_rectangle(r.x, r.y, r.w, f.y - r.y, dim);
                    draw_rectangle(r.x, f.y + f.h, r.w, r.y + r.h - f.y - f.h, dim);
                    draw_rectangle(r.x, f.y, f.x - r.x, f.h, dim);
                    draw_rectangle(f.x + f.w, f.y, r.x + r.w - f.x - f.w, f.h, dim);
                }
                draw_rectangle_lines(f.x, f.y, f.w, f.h, 1.5, CAM_YELLOW);
                let (w, h) = ((c.res_x as f32 * c.res_pct / 100.0).round(), (c.res_y as f32 * c.res_pct / 100.0).round());
                let lens = if c.ortho { format!("orthographic {:.1}", c.ortho_scale) } else { format!("{:.0} mm", c.focal) };
                // the label is a button (picture 51): click it to show the camera's properties
                let label = format!("{}   {} x {}   {}   (click to select)", c.name, w, h, lens);
                let lw = ui.text_w(&label, 12.5);
                let lr = Rect::new(f.x + 1.0, f.y + 1.0, lw + 14.0, 20.0);
                let hot = lr.contains(m) && !ui.blocked && ui.popup.is_none();
                let on = self.sel == Sel::Camera(i);
                draw_rectangle(lr.x, lr.y, lr.w, lr.h, if hot { Color::new(0.30, 0.47, 0.78, 0.9) } else if on { Color::new(0.35, 0.3, 0.05, 0.75) } else { Color::new(0.0, 0.0, 0.0, 0.55) });
                ui.text(&label, f.x + 7.0, f.y + 4.0, 12.5, if hot { WHITE } else { CAM_YELLOW });
                ui.record("vp.camlabel", lr);
                self.vp_ui.push(lr);
                if hot {
                    ui.tooltip = Some("Show this camera's settings in the Properties panel".into());
                    if ui.inp.l_pressed {
                        ui.inp.l_pressed = false;
                        self.select(Sel::Camera(i));
                    }
                }
            }
        }
        let perspective = !self.cam.ortho;
        let sel = if let Sel::Camera(i) = self.sel { Some(i) } else { None };
        let blocked = ui.blocked || ui.popup.is_some() || self.vp_ui.iter().any(|b| b.contains(m)) || !r.contains(m);
        let hs = 70.0 * self.prefs.manip_size.clamp(0.25, 4.0);
        // the selected camera's handles: 1..3 move along X / Y / Z, 4 turn (ring), 5 move freely
        let mut handles: Vec<(u8, Vec2, Vec2)> = vec![];
        let mut centre = None;
        if let Some(i) = sel.filter(|i| Some(*i) != self.looking && *i < self.cameras.len() && !self.cameras[*i].hidden) {
            if perspective {
                let c = &self.cameras[i];
                if let Some(o) = proj(c.pos) {
                    centre = Some(o);
                    for (k, ax) in [Vec3::X, Vec3::Y, Vec3::Z].iter().enumerate() {
                        let unit = (c.dist.max(1.0) * 0.02).max(0.05);
                        if let Some(e) = proj(c.pos + *ax * unit) {
                            let d = e - o;
                            if d.length() > 0.5 {
                                handles.push((k as u8 + 1, o, o + d.normalize() * hs));
                            }
                        }
                    }
                }
            }
        }
        let hit = |p: Vec2| -> u8 {
            for (k, a, b) in &handles {
                let ab = *b - *a;
                let t = ((p - *a).dot(ab) / ab.length_squared().max(1e-4)).clamp(0.0, 1.0);
                if (*a + ab * t).distance(p) < 11.0 && t > 0.15 {
                    return *k;
                }
            }
            if let Some(o) = centre {
                let d = o.distance(p);
                if (d - hs * 0.62).abs() < 10.0 {
                    return 4;
                }
                if d < 12.0 {
                    return 5;
                }
            }
            0
        };
        let hover = if blocked && self.cam_drag.is_none() { 0 } else { hit(m) };
        // start, carry on or end a drag
        if let Some((h, last)) = self.cam_drag {
            if !ui.inp.l_down {
                self.cam_drag = None;
            } else if let Some(i) = sel.filter(|i| *i < self.cameras.len()) {
                let d = m - last;
                let viewer = self.cam.position();
                let c = &mut self.cameras[i];
                match h {
                    1..=3 => {
                        let ax = [Vec3::X, Vec3::Y, Vec3::Z][(h - 1) as usize];
                        if let (Some(a), Some(b)) = (proj(c.pos), proj(c.pos + ax)) {
                            let sd = b - a;
                            let amt = d.dot(sd) / sd.length_squared().max(1e-6);
                            c.pos += ax * amt;
                        }
                    }
                    4 => {
                        if !c.ortho {
                            c.yaw -= d.x * 0.01;
                            c.pitch = (c.pitch + d.y * 0.01).clamp(-1.55, 1.55);
                        }
                    }
                    _ => {
                        let vd = self.cam.dir();
                        let right = Vec3::Y.cross(vd).normalize_or(Vec3::X);
                        let up = vd.cross(right).normalize_or(Vec3::Y);
                        let depth = viewer.distance(c.pos).max(0.1);
                        let s = 2.0 * depth * (self.cam.fov.to_radians() * 0.5).tan() / r.h.max(1.0);
                        c.pos += (right * d.x - up * d.y) * s;
                    }
                }
                self.cam_drag = Some((h, m));
            }
        } else if ui.inp.l_pressed && !blocked && !ui.inp.alt {
            if hover == 5 && ui.inp.l_double {
                // a double-click on the selected camera's centre also centres the view on it
                if let Some(i) = sel.filter(|i| *i < self.cameras.len()) {
                    let p = self.cameras[i].pos;
                    self.centre_on(p);
                    self.set_status("Centred on the camera; the view turns around it");
                }
                ui.inp.l_pressed = false;
            } else if hover != 0 {
                self.cam_drag = Some((hover, m));
                ui.inp.l_pressed = false;
            } else {
                // a click on a camera icon selects it
                let mut best: Option<(usize, f32)> = None;
                let view_right = Vec3::Y.cross(self.cam.dir()).normalize_or(Vec3::X);
                for (i, c) in self.cameras.iter().enumerate() {
                    if Some(i) == self.looking || c.hidden {
                        continue;
                    }
                    let (fwd, _, _) = c.basis();
                    // on the 3D camera (within its selection ring)
                    if let Some(o) = proj(c.pos) {
                        let px = proj(c.pos + view_right * (c.icon_size * isc).max(0.01)).map(|q| q.distance(o)).unwrap_or(30.0);
                        let half = (px * 0.95).max(14.0);
                        let d = o.distance(m);
                        if d < half && best.map_or(true, |x| d < x.1) {
                            best = Some((i, d));
                        }
                    }
                    if let (Some(a), Some(b)) = (proj(c.pos), proj(c.pos + fwd * c.icon_size * isc * 0.6)) {
                        let ab = b - a;
                        let t = ((m - a).dot(ab) / ab.length_squared().max(1e-4)).clamp(0.0, 1.0);
                        let d = (a + ab * t).distance(m);
                        let reach = 16.0f32.max(ab.length() * 0.6);
                        if d < reach && best.map_or(true, |x| d < x.1) {
                            best = Some((i, d));
                        }
                    }
                }
                if let Some((i, _)) = best {
                    self.select(Sel::Camera(i));
                    // a double-click centres the view on the camera, like on a generator (picture 65)
                    if ui.inp.l_double {
                        let p = self.cameras[i].pos;
                        self.centre_on(p);
                        self.set_status("Centred on the camera; the view turns around it");
                    }
                    ui.inp.l_pressed = false;
                }
            }
        }
        // the icons (picture 52): the camera picture at the camera's place, facing the way the
        // camera looks, with a thin yellow view cone from its lens
        let view_right = Vec3::Y.cross(self.cam.dir()).normalize_or(Vec3::X);
        for (i, c) in self.cameras.iter().enumerate() {
            if Some(i) == self.looking || c.hidden {
                continue;
            }
            let on = sel == Some(i);
            let col = if on { Color::new(1.0, 0.93, 0.45, 1.0) } else { CAM_YELLOW };
            let th = if on { 1.6 } else { 1.0 };
            let (fwd, right, up) = c.basis();
            let s = (c.icon_size * isc).max(0.01);
            let line = |a: Vec3, b: Vec3, t: f32, cl: Color| {
                if let Some((p, q)) = seg(a, b) {
                    draw_line(p.x, p.y, q.x, q.y, t, cl);
                }
            };
            let dashed = |a: Vec3, b: Vec3, t: f32, cl: Color| {
                if let Some((p, q)) = seg(a, b) {
                    let n = ((p.distance(q) / 9.0) as usize).clamp(2, 200);
                    for k in (0..n).step_by(2) {
                        let (x, y) = (p.lerp(q, k as f32 / n as f32), p.lerp(q, (k + 1) as f32 / n as f32));
                        draw_line(x.x, x.y, y.x, y.y, t, cl);
                    }
                }
            };
            // the view's mouth just ahead of the lens (where the guides start; not drawn itself:
            // picture 53 took the yellow box away)
            let (hh, hw) = if c.ortho {
                let k = s * 0.45;
                (k, k * c.aspect())
            } else {
                let t = (c.vfov() * 0.5).tan().clamp(0.12, 1.2);
                (s * t, s * t * c.aspect())
            };
            let fc = c.pos + fwd * s;
            let corners = [fc + right * hw + up * hh, fc - right * hw + up * hh, fc - right * hw - up * hh, fc + right * hw - up * hh];
            let _ = th;
            // the 3D camera itself is drawn with the scene (draw_cameras_3d); here its selection
            // ring and name
            if let Some(o) = proj(c.pos) {
                let px = proj(c.pos + view_right * s).map(|q| q.distance(o)).unwrap_or(30.0);
                let rad = (px * 0.95).max(14.0);
                if on {
                    draw_circle_lines(o.x, o.y, rad, 2.0, Color::new(col.r, col.g, col.b, 0.85));
                }
                if c.show_name {
                    let nw = ui.text_w(&c.name, 12.0);
                    draw_rectangle(o.x - nw * 0.5 - 4.0, o.y + rad + 2.0, nw + 8.0, 17.0, Color::new(0.0, 0.0, 0.0, 0.45));
                    ui.text(&c.name, o.x - nw * 0.5, o.y + rad + 4.0, 12.0, col);
                }
            }
            // clip limits along the view line, and the focus plane
            if c.show_limits {
                let (n, f) = if c.auto_clip { (0.05 * c.dist, c.dist * 40.0) } else { (c.near, c.far) };
                line(c.pos + fwd * n, c.pos + fwd * f.min(n + 4000.0), 1.0, col);
                for d in [n, f.min(n + 4000.0)] {
                    if let Some(p) = proj(c.pos + fwd * d) {
                        draw_circle(p.x, p.y, 3.0, col);
                    }
                }
            }
            if on && c.dof && c.show_focus {
                let fd = c.focus_dist.max(0.01);
                let (fh, fw) = if c.ortho { (c.ortho_scale * 0.5, c.ortho_scale * 0.5 * c.aspect()) } else { let t = (c.vfov() * 0.5).tan(); (fd * t, fd * t * c.aspect()) };
                let q = c.pos + fwd * fd;
                let pc = [q + right * fw + up * fh, q - right * fw + up * fh, q - right * fw - up * fh, q + right * fw - up * fh];
                let pts: Vec<Vec2> = pc.iter().filter_map(|p| proj(*p)).collect();
                if pts.len() == 4 {
                    let fill = Color::new(col.r, col.g, col.b, 0.10);
                    draw_triangle(pts[0], pts[1], pts[2], fill);
                    draw_triangle(pts[0], pts[2], pts[3], fill);
                    for k in 0..4 {
                        draw_line(pts[k].x, pts[k].y, pts[(k + 1) % 4].x, pts[(k + 1) % 4].y, 1.0, Color::new(col.r, col.g, col.b, 0.6));
                    }
                    ui.text("focus", pts[0].x + 4.0, pts[0].y - 16.0, 11.5, col);
                }
            }
            // guides (pictures 51, 52): what the camera sees out to its target distance, drawn at
            // full size whatever the icon's size: the view volume's edges, the picture frame there
            // with its width and height, and the view line with the focal length and field of view
            if on && c.show_guides {
                let gc = Color::new(1.0, 0.95, 0.65, 0.95);
                let faint = Color::new(gc.r, gc.g, gc.b, 0.55);
                let reach = c.dist.max(s * 2.0);
                let (fh, fw) = if c.ortho { (c.ortho_scale * 0.5, c.ortho_scale * 0.5 * c.aspect()) } else { let t = (c.vfov() * 0.5).tan(); (reach * t, reach * t * c.aspect()) };
                let q = c.pos + fwd * reach;
                let far = [q + right * fw + up * fh, q - right * fw + up * fh, q - right * fw - up * fh, q + right * fw - up * fh];
                for k in 0..4 {
                    dashed(corners[k], far[k], 1.1, faint);
                    line(far[k], far[(k + 1) % 4], 1.4, gc);
                }
                // the view line to the target, with a cross there
                dashed(c.pos, q, 1.3, gc);
                if let Some(p) = proj(q) {
                    draw_line(p.x - 7.0, p.y, p.x + 7.0, p.y, 1.5, gc);
                    draw_line(p.x, p.y - 7.0, p.x, p.y + 7.0, 1.5, gc);
                }
                let label = |ui: &mut Ui, p: Vec2, t: &str| {
                    let w = ui.text_w(t, 11.5);
                    draw_rectangle(p.x - w * 0.5 - 5.0, p.y - 9.0, w + 10.0, 18.0, Color::new(0.0, 0.0, 0.0, 0.65));
                    ui.text(t, p.x - w * 0.5, p.y - 7.0, 11.5, gc);
                };
                let lens = if c.ortho { format!("Orthographic  |  scale {:.1}", c.ortho_scale) } else { format!("{:.0} mm  |  FOV {:.1}\u{b0} x {:.1}\u{b0}", c.focal, c.hfov().to_degrees(), c.vfov().to_degrees()) };
                if let Some((a, b)) = seg(c.pos, q) {
                    label(ui, a.lerp(b, 0.45), &lens);
                }
                // the field of view: an arc between the two side edges near the lens
                if !c.ortho {
                    let rr = reach * 0.25;
                    let n = 16;
                    let hf = c.hfov() * 0.5;
                    let pts: Vec<Vec2> = (0..=n).filter_map(|k| {
                        let a = -hf + 2.0 * hf * k as f32 / n as f32;
                        proj(c.pos + (fwd * a.cos() + right * a.sin()) * rr)
                    }).collect();
                    for w2 in pts.windows(2) {
                        draw_line(w2[0].x, w2[0].y, w2[1].x, w2[1].y, 1.3, gc);
                    }
                    if let Some(p) = pts.last() {
                        label(ui, *p + vec2(0.0, -14.0), &format!("{:.1}\u{b0}", c.hfov().to_degrees()));
                    }
                }
                // width and height as dimension lines beside the far frame (offset in screen pixels)
                let (w, h) = ((c.res_x as f32 * c.res_pct / 100.0).round(), (c.res_y as f32 * c.res_pct / 100.0).round());
                let sensor_h = c.sensor / c.aspect();
                let wt = if c.ortho { format!("W {} px  ({:.1} units)", w, fw * 2.0) } else { format!("W {} px  ({:.1} mm)", w, c.sensor) };
                let ht = if c.ortho { format!("H {} px  ({:.1} units)", h, fh * 2.0) } else { format!("H {} px  ({:.1} mm)", h, sensor_h) };
                let centre2 = proj(q);
                let dim = |ui: &mut Ui, p0: Vec3, p1: Vec3, t: &str| {
                    if let (Some(a), Some(b), Some(cc)) = (proj(p0), proj(p1), centre2) {
                        // push the line outward, away from the frame's middle
                        let mid = a.lerp(b, 0.5);
                        let out = (mid - cc).normalize_or(vec2(0.0, 1.0)) * 16.0;
                        let (x, y) = (a + out, b + out);
                        draw_line(a.x, a.y, x.x + out.x * 0.3, x.y + out.y * 0.3, 1.0, faint);
                        draw_line(b.x, b.y, y.x + out.x * 0.3, y.y + out.y * 0.3, 1.0, faint);
                        draw_line(x.x, x.y, y.x, y.y, 1.4, gc);
                        let d = (y - x).normalize_or_zero();
                        let nrm = vec2(-d.y, d.x);
                        draw_triangle(x, x + d * 8.0 + nrm * 3.5, x + d * 8.0 - nrm * 3.5, gc);
                        draw_triangle(y, y - d * 8.0 + nrm * 3.5, y - d * 8.0 - nrm * 3.5, gc);
                        label(ui, x.lerp(y, 0.5) + out * 0.9, t);
                    }
                };
                dim(ui, far[2], far[3], &wt);
                dim(ui, far[3], far[0], &ht);
            }
        }
        // the handles over everything
        if let Some(o) = centre {
            let active = self.cam_drag.map(|d| d.0).unwrap_or(hover);
            let axis_cols = [Color::new(0.95, 0.3, 0.3, 1.0), Color::new(0.4, 0.9, 0.35, 1.0), Color::new(0.35, 0.55, 1.0, 1.0)];
            for (k, a, b) in &handles {
                let mut cl = axis_cols[(*k - 1) as usize];
                if active == *k {
                    cl = Color::new(1.0, 1.0, 0.6, 1.0);
                }
                draw_line(a.x, a.y, b.x, b.y, if active == *k { 3.5 } else { 2.5 }, cl);
                let d = (*b - *a).normalize_or_zero();
                let n = vec2(-d.y, d.x);
                draw_triangle(*b + d * 10.0, *b + n * 5.0, *b - n * 5.0, cl);
            }
            let ring = if active == 4 { Color::new(1.0, 1.0, 0.6, 1.0) } else { Color::new(CAM_YELLOW.r, CAM_YELLOW.g, CAM_YELLOW.b, 0.8) };
            draw_circle_lines(o.x, o.y, hs * 0.62, if active == 4 { 3.0 } else { 1.8 }, ring);
            draw_circle(o.x, o.y, if active == 5 { 7.0 } else { 5.0 }, if active == 5 { WHITE } else { CAM_YELLOW });
            let tip = match active {
                1..=3 => Some("Drag to move the camera along this axis"),
                4 => Some("Drag to turn the camera: left/right turns, up/down tilts"),
                5 => Some("Drag to move the camera across the view"),
                _ => None,
            };
            if let Some(t) = tip {
                if self.cam_drag.is_none() {
                    ui.tooltip = Some(t.into());
                }
            }
            // keep the viewport from orbiting under the handles
            self.vp_ui.push(Rect::new(o.x - hs - 12.0, o.y - hs - 12.0, (hs + 12.0) * 2.0, (hs + 12.0) * 2.0).intersect(r).unwrap_or(Rect::new(o.x, o.y, 1.0, 1.0)));
        }
        if self.cam_drag.is_some() {
            self.vp_ui.push(r);
        }
    }
}

thread_local! {
    /// The selected camera's live preview for the left panel (picture 65).
    static PREVIEW: std::cell::RefCell<Option<RenderTarget>> = const { std::cell::RefCell::new(None) };
}

/// Width of the camera preview picture, in pixels.
const PREVIEW_W: u32 = 480;

impl App {
    /// Each frame: while a camera is selected in the perspective view (and not looked through),
    /// draw what it sees, with its own lens, into the preview picture the camera panel shows.
    pub fn render_camera_preview(&mut self) {
        let Sel::Camera(i) = self.sel else { return };
        if self.cam.ortho || self.looking == Some(i) || i >= self.cameras.len() {
            return;
        }
        let (Some(gm), Some(gpu)) = (self.gm.clone(), self.gpu.as_ref()) else { return };
        let c = self.cameras[i].clone();
        let w = PREVIEW_W;
        let h = ((w as f32 / c.aspect().clamp(0.2, 5.0)).round() as u32).clamp(32, 1200);
        let rt = PREVIEW.with(|p| {
            let mut p = p.borrow_mut();
            let fresh = match p.as_ref() {
                Some(t) => t.texture.width() as u32 != w || t.texture.height() as u32 != h,
                None => true,
            };
            if fresh {
                let t = render_target_ex(w, h, RenderTargetParams { depth: true, ..Default::default() });
                t.texture.set_filter(FilterMode::Linear);
                *p = Some(t);
            }
            p.clone().unwrap()
        });
        let cam = c.to_cam(1.0);
        let mut c3 = cam.camera3d(None, w as f32 / h as f32, Some(rt.clone()));
        c3.viewport = None;
        set_camera(&c3);
        let (bt, bb) = (self.prefs.color("bg_top"), self.prefs.color("bg_bottom"));
        let bg = Color::new((bt.r + bb.r) * 0.5, (bt.g + bb.g) * 0.5, (bt.b + bb.b) * 0.5, 1.0);
        clear_background(Color::new(bg.r, bg.g, bg.b, 1.0));
        if self.prefs.show_ground {
            let ext = (self.bounds.1 - self.bounds.0).length();
            crate::render::draw_ground((ext * 0.9).clamp(12.0, 400.0), -(cam.dist * 0.004).max(0.02), self.prefs.color("ground"));
        }
        let mut light = self.light.clone();
        light.sel_fill = [0.0; 4];
        light.overdraw = false;
        crate::app::draw_gpu(gpu, &self.materials, &gm, &light, cam.position());
        unsafe { get_internal_gl() }.flush();
        gl_use_default_material();
        set_default_camera();
    }
}

/// The camera preview picture (when there is one this frame) at `x, y`, `w` wide; returns the
/// height used.
fn draw_camera_preview(r: &mut Rows, aspect: f32) -> f32 {
    let Some(t) = PREVIEW.with(|p| p.borrow().clone()) else { return 0.0 };
    let w = r.w - 12.0;
    let h = (w / aspect.clamp(0.2, 5.0)).min(320.0);
    let w2 = h * aspect.clamp(0.2, 5.0);
    let x = r.x + 6.0 + (w - w2) * 0.5;
    draw_texture_ex(&t.texture, x, r.y + 4.0, WHITE, DrawTextureParams { dest_size: Some(vec2(w2, h)), flip_y: true, ..Default::default() });
    draw_rectangle_lines(x, r.y + 4.0, w2, h, 1.5, CAM_YELLOW);
    h + 10.0
}

/// The camera's panel (picture 50).
pub fn camera_rows(r: &mut Rows, app: &mut App, i: usize) {
    if i >= app.cameras.len() {
        return;
    }
    let looking = app.looking == Some(i);
    // picture 65: what the camera sees, live, while it's selected in the perspective view
    if !app.cam.ortho && !looking {
        let a = app.cameras[i].aspect();
        let used = draw_camera_preview(r, a);
        r.y += used;
    }
    if r.group("Camera") {
        // Name: click to rename (the whole name selected, typing replaces it)
        let lw = (r.w * 0.46).floor();
        r.ui.text_fit("Name", r.x + 6.0, r.y + 5.0, lw - 10.0, 12.5, C_TEXT_DIM);
        let fr = Rect::new(r.x + lw, r.y + 2.0, r.w - lw, 20.0);
        let editing = app.cam_rename.is_some();
        if editing {
            let (mut text, mut all) = app.cam_rename.clone().unwrap();
            if all && !r.ui.inp.chars.is_empty() {
                text.clear();
                all = false;
            }
            for c in &r.ui.inp.chars {
                if text.chars().count() < 48 && !c.is_control() {
                    text.push(*c);
                }
            }
            if r.ui.inp.key(KeyCode::Backspace) {
                if all {
                    text.clear();
                    all = false;
                } else {
                    text.pop();
                }
            }
            if r.ui.inp.key(KeyCode::Left) || r.ui.inp.key(KeyCode::Right) {
                all = false;
            }
            draw_rectangle(fr.x, fr.y, fr.w, fr.h, C_BG);
            draw_rectangle_lines(fr.x, fr.y, fr.w, fr.h, 1.0, C_ACCENT2);
            if all && !text.is_empty() {
                let tw = r.ui.text_w(&text, 12.5).min(fr.w - 10.0);
                draw_rectangle(fr.x + 3.0, fr.y + 3.0, tw + 4.0, fr.h - 6.0, C_ACCENT);
            }
            r.ui.text_fit(&text, fr.x + 5.0, fr.y + 3.0, fr.w - 10.0, 12.5, WHITE);
            if !all && (get_time() * 2.0) as i64 % 2 == 0 {
                let cx = fr.x + 6.0 + r.ui.text_w(&text, 12.5).min(fr.w - 12.0);
                draw_line(cx, fr.y + 3.0, cx, fr.y + fr.h - 3.0, 1.0, WHITE);
            }
            let done = r.ui.inp.key(KeyCode::Enter) || r.ui.inp.key(KeyCode::KpEnter) || (r.ui.inp.l_pressed && !fr.contains(r.ui.mouse()));
            if r.ui.inp.key(KeyCode::Escape) {
                app.cam_rename = None;
            } else if done {
                let t = text.trim().to_string();
                if !t.is_empty() {
                    app.cameras[i].name = t;
                }
                app.cam_rename = None;
            } else {
                app.cam_rename = Some((text, all));
            }
        } else {
            let name = app.cameras[i].name.clone();
            let key = format!("{}.camname", r.prefix);
            if r.ui.button(&key, fr, &name) {
                app.cam_rename = Some((name, true));
            }
            if r.ui.hover(fr) {
                r.ui.tooltip = Some("Click to rename the camera".into());
            }
        }
        r.y += 24.0;
        let c = &app.cameras[i];
        let kind = if c.ortho { format!("Orthographic ({})", VIEWS[c.view.min(VIEWS.len() - 1)]) } else { "Perspective".to_string() };
        r.info("Type", &kind);
        if r.button("look", if looking { "Stop looking through" } else { "Look through camera" }) {
            app.look_through(if looking { None } else { Some(i) });
        }
        if r.button("setview", "Set to current view") {
            let cam = app.cam;
            app.cameras[i].take_view(&cam);
            if !app.cameras[i].ortho && !cam.ortho {
                let a = app.cameras[i].aspect();
                let _ = a;
                app.cameras[i].set_vfov(cam.fov.to_radians());
            }
        }
        if r.button("dup", "Duplicate") {
            let mut c = app.cameras[i].clone();
            c.name = format!("{} copy", c.name);
            app.cameras.push(c);
            let n = app.cameras.len() - 1;
            app.select(Sel::Camera(n));
            return;
        }
        if r.button("del", "Delete camera") {
            app.delete_camera(i);
            return;
        }
    }
    let c = &mut app.cameras[i];
    if r.group("Lens") {
        if c.ortho {
            r.f("oscale", "Orthographic scale", &mut c.ortho_scale, 0.1, 500.0, 2);
            c.ortho_scale = c.ortho_scale.max(0.01);
        } else {
            r.f("focal", "Focal length (mm)", &mut c.focal, 1.0, 300.0, 1);
            c.focal = c.focal.clamp(0.5, 5000.0);
            let mut fov = c.vfov().to_degrees();
            let before = fov;
            r.f("fov", "Field of view (deg)", &mut fov, 1.0, 170.0, 1);
            if (fov - before).abs() > 1e-4 {
                c.set_vfov(fov.clamp(0.5, 175.0).to_radians());
            }
            r.f("sensor", "Sensor width (mm)", &mut c.sensor, 1.0, 100.0, 1);
            c.sensor = c.sensor.max(0.1);
        }
        r.f_faint("shiftx", "Shift X", &mut c.shift_x, -1.0, 1.0, 2);
        r.f_faint("shifty", "Shift Y", &mut c.shift_y, -1.0, 1.0, 2);
        r.b("autoclip", "Automatic clipping", &mut c.auto_clip);
        if !c.auto_clip {
            r.f("near", "Clip start", &mut c.near, 0.001, 100.0, 3);
            r.f("far", "Clip end", &mut c.far, 1.0, 10000.0, 0);
            c.near = c.near.max(0.001);
            c.far = c.far.max(c.near + 0.01);
        }
    }
    if r.group("Transform") {
        r.f("px", "Position X", &mut c.pos.x, -500.0, 500.0, 2);
        r.f("py", "Position Y", &mut c.pos.y, -500.0, 500.0, 2);
        r.f("pz", "Position Z", &mut c.pos.z, -500.0, 500.0, 2);
        if !c.ortho {
            // the way it looks: heading around the up axis, tilt up / down
            let mut yaw = (c.yaw.to_degrees() + 180.0).rem_euclid(360.0) - 180.0;
            let mut tilt = -c.pitch.to_degrees();
            let (y0, t0) = (yaw, tilt);
            r.f("yaw", "Heading (deg)", &mut yaw, -180.0, 180.0, 1);
            r.f("tilt", "Tilt (deg)", &mut tilt, -89.0, 89.0, 1);
            if yaw != y0 {
                c.yaw = yaw.to_radians();
            }
            if tilt != t0 {
                c.pitch = (-tilt).clamp(-89.0, 89.0).to_radians();
            }
        }
        let mut roll = c.roll.to_degrees();
        let r0 = roll;
        r.f("roll", "Roll (deg)", &mut roll, -180.0, 180.0, 1);
        if roll != r0 {
            c.roll = roll.to_radians();
        }
        r.f("dist", "Target distance", &mut c.dist, 0.5, 500.0, 1);
        c.dist = c.dist.max(0.5);
    }
    if r.group("Frame") {
        let names: Vec<String> = FRAME_PRESETS.iter().map(|p| p.0.to_string()).collect();
        let mut pi = FRAME_PRESETS.iter().position(|p| p.1 == c.res_x && p.2 == c.res_y).unwrap_or(0);
        let before = pi;
        r.combo("preset", "Preset", &names, &mut pi);
        if pi != before && pi > 0 {
            let v = c.vfov();
            c.res_x = FRAME_PRESETS[pi].1;
            c.res_y = FRAME_PRESETS[pi].2;
            if !c.ortho {
                c.set_vfov(v);
            }
        }
        r.i("resx", "Resolution X", &mut c.res_x, 16, 8192);
        r.i("resy", "Resolution Y", &mut c.res_y, 16, 8192);
        c.res_x = c.res_x.clamp(1, 16384);
        c.res_y = c.res_y.clamp(1, 16384);
        r.f("respct", "Resolution %", &mut c.res_pct, 1.0, 400.0, 0);
        c.res_pct = c.res_pct.clamp(1.0, 1000.0);
        let (w, h) = ((c.res_x as f32 * c.res_pct / 100.0).round(), (c.res_y as f32 * c.res_pct / 100.0).round());
        r.info("Picture size", &format!("{} x {} px  (aspect {:.3})", w, h, c.aspect()));
        r.b("frame", "Show frame", &mut c.show_frame);
        r.f("pp", "Passepartout", &mut c.passepartout, 0.0, 1.0, 2);
        c.passepartout = c.passepartout.clamp(0.0, 1.0);
    }
    if r.group("Depth of Field") {
        r.b("dof", "Enabled", &mut c.dof);
        r.f("focus", "Focus distance", &mut c.focus_dist, 0.1, 500.0, 2);
        c.focus_dist = c.focus_dist.max(0.01);
        r.f("fstop", "F-stop", &mut c.fstop, 0.5, 32.0, 1);
        c.fstop = c.fstop.clamp(0.1, 128.0);
        r.i("blades", "Blades", &mut c.blades, 0, 16);
        r.b("showfocus", "Show focus plane", &mut c.show_focus);
        if c.dof && !c.ortho {
            let (n, f) = c.sharp_range();
            let far = if f.is_finite() { format!("{:.2}", f) } else { "infinity".to_string() };
            r.info("Sharp from", &format!("{:.2} to {}", n, far));
        }
    }
    if r.group("Display") {
        r.f("icon", "Icon size", &mut c.icon_size, 0.1, 30.0, 2);
        c.icon_size = c.icon_size.max(0.01);
        r.b("hidden", "Hide in viewport", &mut c.hidden);
        r.b("showname", "Show name", &mut c.show_name);
        r.b("limits", "Show limits", &mut c.show_limits);
        r.b("guides", "Show guides", &mut c.show_guides);
        r.b("lock", "Lock camera to view", &mut c.lock_to_view);
        r.note("Lock: moving the view while looking through it moves the camera.");
    }
    if looking {
        // property edits show at once in the view
        app.look_cam = None;
    }
}
