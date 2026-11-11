//! 3D viewport: camera, lit material shader, grid, and GPU mesh upload.
use crate::gen::TreeGeo;
use crate::model::Material;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation, PipelineParams, UniformType};
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::f32::consts::PI;

pub const VIEWS: [&str; 7] = ["Perspective", "Front", "Back", "Left", "Right", "Top", "Bottom"];

pub struct Cam {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub target: Vec3,
    pub ortho: bool,
    pub view: usize,
    pub fov: f32,
}

impl Cam {
    pub fn new() -> Self {
        Cam { yaw: 0.6, pitch: 0.18, dist: 70.0, target: vec3(0.0, 14.0, 0.0), ortho: false, view: 0, fov: 45.0 }
    }
    pub fn set_view(&mut self, v: usize) {
        self.view = v;
        match v {
            0 => {
                self.ortho = false;
                self.yaw = 0.6;
                self.pitch = 0.18;
            }
            1 => self.set_axis(0.0, 0.0),
            2 => self.set_axis(PI, 0.0),
            3 => self.set_axis(-PI * 0.5, 0.0),
            4 => self.set_axis(PI * 0.5, 0.0),
            5 => self.set_axis(0.0, PI * 0.5 - 0.001),
            _ => self.set_axis(0.0, -PI * 0.5 + 0.001),
        }
    }
    fn set_axis(&mut self, yaw: f32, pitch: f32) {
        self.ortho = true;
        self.yaw = yaw;
        self.pitch = pitch;
    }
    pub fn dir(&self) -> Vec3 {
        vec3(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos())
    }
    pub fn position(&self) -> Vec3 {
        self.target + self.dir() * if self.ortho { self.dist.max(1.0) * 20.0 } else { self.dist }
    }
    pub fn view_name(&self) -> String {
        if self.view == 0 {
            if self.ortho { "User (orthographic)".into() } else { "Perspective".into() }
        } else {
            VIEWS[self.view].to_string()
        }
    }
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-1.55, 1.55);
        if self.view != 0 {
            self.view = 0; // leaves the named view; projection kept
        }
    }
    pub fn pan(&mut self, dx: f32, dy: f32, vp_h: f32) {
        let d = self.dir();
        let right = Vec3::Y.cross(d).normalize_or_zero();
        let up = d.cross(right).normalize_or_zero();
        let s = self.view_height() / vp_h.max(1.0);
        self.target += (right * dx + up * dy) * s;
    }
    pub fn zoom(&mut self, steps: f32) {
        self.dist = (self.dist * 0.88f32.powf(steps)).clamp(1.0, 2000.0);
    }
    pub fn view_height(&self) -> f32 {
        2.0 * self.dist * (self.fov.to_radians() * 0.5).tan()
    }
    pub fn frame(&mut self, bmin: Vec3, bmax: Vec3) {
        self.target = (bmin + bmax) * 0.5;
        let r = ((bmax - bmin).length() * 0.5).max(1.0);
        self.dist = r / (self.fov.to_radians() * 0.5).sin() * 1.05;
    }
    pub fn camera3d(&self, viewport: Option<(i32, i32, i32, i32)>, aspect: f32, rt: Option<RenderTarget>) -> Camera3D {
        let pos = self.position();
        let up = if self.pitch.abs() > 1.5 { vec3(-self.yaw.sin(), 0.0, -self.yaw.cos()) * self.pitch.signum() } else { Vec3::Y };
        Camera3D {
            position: pos,
            target: self.target,
            up,
            fovy: if self.ortho { self.view_height() } else { self.fov.to_radians() },
            aspect: Some(aspect),
            projection: if self.ortho { Projection::Orthographics } else { Projection::Perspective },
            render_target: rt,
            viewport,
            z_near: if self.ortho { 1.0 } else { (self.dist * 0.01).max(0.05) },
            z_far: if self.ortho { self.dist.max(1.0) * 60.0 } else { self.dist * 40.0 + 500.0 },
        }
    }
}

const VS: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
varying lowp vec4 color;
varying mediump vec2 uv;
varying mediump vec3 vn;
varying mediump vec3 wp;
varying mediump float occ;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    vec4 w = Model * vec4(position, 1.0);
    gl_Position = Projection * w;
    wp = w.xyz;
    vn = normal.xyz;
    occ = normal.w;
    uv = texcoord;
    color = color0 / 255.0;
}"#;

const FS: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying mediump vec2 uv;
varying mediump vec3 vn;
varying mediump vec3 wp;
varying mediump float occ;
uniform sampler2D Texture;
uniform vec4 Tint;
uniform vec3 LightDir;
uniform vec3 CamPos;
uniform float Cutoff;
uniform float Gloss;
uniform float Specular;
uniform float Metallic;
uniform vec4 SubColor;
uniform float AOAmount;
uniform float TwoSided;
uniform float Bump;
uniform float Unlit;
uniform float HasNormalMap;
uniform sampler2D NormalMap;
void main() {
    vec4 tex = texture2D(Texture, uv);
    if (tex.a < Cutoff) discard;
    vec3 n = normalize(vn);
    if (!gl_FrontFacing) {
        if (TwoSided < 0.5) { n = -n; } else { n = -n; }
    }
    if (HasNormalMap > 0.5) {
        // a loaded Normal map (tangent frame approximated from the surface normal)
        vec3 nm = texture2D(NormalMap, uv).xyz * 2.0 - 1.0;
        vec3 t2 = abs(n.y) < 0.95 ? normalize(cross(n, vec3(0.0, 1.0, 0.0))) : normalize(cross(n, vec3(1.0, 0.0, 0.0)));
        vec3 b2 = cross(n, t2);
        n = normalize(t2 * nm.x * Bump + b2 * nm.y * Bump + n * max(nm.z, 0.05));
    } else if (Bump > 0.0) {
        float e = 1.0 / 256.0;
        float h0 = dot(tex.rgb, vec3(0.33));
        float hx = dot(texture2D(Texture, uv + vec2(e, 0.0)).rgb, vec3(0.33));
        float hy = dot(texture2D(Texture, uv + vec2(0.0, e)).rgb, vec3(0.33));
        vec3 t = abs(n.y) < 0.95 ? normalize(cross(n, vec3(0.0, 1.0, 0.0))) : normalize(cross(n, vec3(1.0, 0.0, 0.0)));
        vec3 b = cross(n, t);
        n = normalize(n - Bump * 6.0 * ((hx - h0) * t + (hy - h0) * b));
    }
    vec3 base = tex.rgb * Tint.rgb * color.rgb;
    if (Unlit > 0.5) { vec3 u = base; if (color.a < 0.9) { u = mix(u, vec3(1.0, 0.62, 0.22), 0.38); } gl_FragColor = vec4(u, 1.0); return; }
    vec3 L = normalize(LightDir);
    float ndl = dot(n, L);
    float diff = max(ndl, 0.0);
    vec3 V = normalize(CamPos - wp);
    vec3 H = normalize(L + V);
    float sp = pow(max(dot(n, H), 0.0), mix(4.0, 96.0, Gloss)) * Specular * (0.4 + Gloss);
    float ao = mix(1.0, occ, AOAmount);
    vec3 sky = vec3(0.50, 0.55, 0.62) * (0.65 + 0.35 * n.y);
    vec3 amb = base * sky * 0.75;
    vec3 sun = vec3(1.0, 0.96, 0.88);
    vec3 dif = base * sun * diff * (1.0 - Metallic * 0.6);
    vec3 trans = base * SubColor.rgb * SubColor.a * max(-ndl, 0.0) * 1.4 * TwoSided;
    vec3 spec = sp * mix(vec3(1.0), base * 2.0, Metallic) * sun;
    vec3 c = (amb + dif + trans) * ao + spec * ao;
    c = pow(c, vec3(0.92));
    if (color.a < 0.9) { c = mix(c, vec3(1.0, 0.62, 0.22), 0.38); }
    gl_FragColor = vec4(c, 1.0);
}"#;

pub fn make_material() -> Result<macroquad::material::Material, String> {
    let uniforms = vec![
        UniformDesc::new("Tint", UniformType::Float4),
        UniformDesc::new("LightDir", UniformType::Float3),
        UniformDesc::new("CamPos", UniformType::Float3),
        UniformDesc::new("Cutoff", UniformType::Float1),
        UniformDesc::new("Gloss", UniformType::Float1),
        UniformDesc::new("Specular", UniformType::Float1),
        UniformDesc::new("Metallic", UniformType::Float1),
        UniformDesc::new("SubColor", UniformType::Float4),
        UniformDesc::new("AOAmount", UniformType::Float1),
        UniformDesc::new("TwoSided", UniformType::Float1),
        UniformDesc::new("Bump", UniformType::Float1),
        UniformDesc::new("Unlit", UniformType::Float1),
        UniformDesc::new("HasNormalMap", UniformType::Float1),
    ];
    let _ = BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha));
    load_material(
        ShaderSource::Glsl { vertex: VS, fragment: FS },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            },
            uniforms,
            textures: vec!["NormalMap".to_string()],
        },
    )
    .map_err(|e| format!("{:?}", e))
}

pub struct Lighting {
    pub azimuth: f32,
    pub elevation: f32,
    pub unlit: bool,
}

impl Lighting {
    pub fn dir(&self) -> Vec3 {
        let a = self.azimuth.to_radians();
        let e = self.elevation.to_radians();
        vec3(a.sin() * e.cos(), e.sin(), a.cos() * e.cos())
    }
}

pub fn apply_material(gm: &macroquad::material::Material, m: &Material, light: &Lighting, cam_pos: Vec3) {
    gm.set_uniform("Tint", vec4(m.color[0], m.color[1], m.color[2], 1.0));
    gm.set_uniform("LightDir", light.dir());
    gm.set_uniform("CamPos", cam_pos);
    gm.set_uniform("Cutoff", if m.opacity_from_alpha { m.cutoff.max(0.01) } else { 0.0 });
    gm.set_uniform("Gloss", m.gloss);
    let sc = m.specular_color;
    gm.set_uniform("Specular", sc[0].max(sc[1]).max(sc[2]));
    gm.set_uniform("Metallic", m.metallic);
    gm.set_uniform("SubColor", vec4(m.sss_color[0], m.sss_color[1], m.sss_color[2], m.sss_amount));
    gm.set_uniform("AOAmount", m.ao);
    gm.set_uniform("TwoSided", if m.two_sided { 1.0f32 } else { 0.0f32 });
    gm.set_uniform("Bump", m.normal_strength);
    gm.set_uniform("Unlit", if light.unlit { 1.0f32 } else { 0.0f32 });
    // Normal slot: use its picture when one is loaded and the slot is on
    let nmap = m.slots.get(2).filter(|s| s.enabled).and_then(|s| s.tex.clone());
    gm.set_uniform("HasNormalMap", if nmap.is_some() { 1.0f32 } else { 0.0f32 });
    let t = nmap.unwrap_or_else(flat_normal);
    gm.set_texture("NormalMap", t);
}

thread_local! {
    static FLAT_NORMAL: std::cell::RefCell<Option<Texture2D>> = const { std::cell::RefCell::new(None) };
}

/// A 1×1 flat normal map used when a material has none.
fn flat_normal() -> Texture2D {
    FLAT_NORMAL.with(|c| {
        let mut b = c.borrow_mut();
        if b.is_none() {
            *b = Some(Texture2D::from_rgba8(1, 1, &[128, 128, 255, 255]));
        }
        b.clone().unwrap()
    })
}

/// GPU-ready meshes per material, rebuilt whenever the tree changes.
pub struct GpuTree {
    pub parts: Vec<(usize, Vec<Mesh>)>,
}

pub fn upload(geo: &TreeGeo) -> GpuTree {
    let mut parts = vec![];
    let mut keys: Vec<usize> = geo.mats.keys().cloned().collect();
    keys.sort();
    for k in keys {
        let mg = &geo.mats[&k];
        let meshes = mg
            .chunks
            .iter()
            .filter(|c| !c.i.is_empty())
            .map(|c| Mesh { vertices: c.v.clone(), indices: c.i.clone(), texture: None })
            .collect();
        parts.push((k, meshes));
    }
    GpuTree { parts }
}

pub fn draw_tree(gpu: &GpuTree, mats: &[Material], gm: &macroquad::material::Material, light: &Lighting, cam_pos: Vec3) {
    gl_use_material(gm);
    for (mi, meshes) in &gpu.parts {
        let fallback = Material::new("?", 0);
        let m = mats.get(*mi).unwrap_or(&fallback);
        apply_material(gm, m, light, cam_pos);
        for mesh in meshes {
            let tmp = Mesh { vertices: mesh.vertices.clone(), indices: mesh.indices.clone(), texture: m.tex.clone() };
            draw_mesh(&tmp);
        }
        unsafe { get_internal_gl() }.flush();
    }
    gl_use_default_material();
}

pub fn draw_grid(size: f32, step: f32) {
    let n = (size / step) as i32;
    for i in -n..=n {
        let x = i as f32 * step;
        let c = if i == 0 { Color::new(1.0, 1.0, 1.0, 0.0) } else if i % 5 == 0 { Color::new(1.0, 1.0, 1.0, 0.16) } else { Color::new(1.0, 1.0, 1.0, 0.07) };
        draw_line_3d(vec3(x, 0.0, -size), vec3(x, 0.0, size), c);
        draw_line_3d(vec3(-size, 0.0, x), vec3(size, 0.0, x), c);
    }
    draw_line_3d(vec3(-size, 0.0, 0.0), vec3(size, 0.0, 0.0), Color::new(0.85, 0.3, 0.3, 0.7));
    draw_line_3d(vec3(0.0, 0.0, -size), vec3(0.0, 0.0, size), Color::new(0.3, 0.45, 0.9, 0.7));
}

pub fn sphere_mesh(r: f32) -> Vec<(Vec<Vertex>, Vec<u16>)> {
    let (nu, nv) = (40usize, 24usize);
    let mut v = vec![];
    let mut idx = vec![];
    for j in 0..=nv {
        let th = j as f32 / nv as f32 * PI;
        for i in 0..=nu {
            let ph = i as f32 / nu as f32 * 2.0 * PI;
            let n = vec3(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin());
            v.push(Vertex { position: n * r, uv: vec2(i as f32 / nu as f32 * 2.0, j as f32 / nv as f32), color: [255; 4], normal: vec4(n.x, n.y, n.z, 1.0) });
        }
    }
    for j in 0..nv {
        for i in 0..nu {
            let a = (j * (nu + 1) + i) as u16;
            let b = a + 1;
            let c = a + (nu + 1) as u16;
            let d = c + 1;
            idx.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    vec![(v, idx)]
}

/// A capped cylinder along Y (material preview).
pub fn cylinder_mesh(r: f32, h: f32) -> Vec<(Vec<Vertex>, Vec<u16>)> {
    let nu = 40usize;
    let mut v = vec![];
    let mut idx: Vec<u16> = vec![];
    for i in 0..=nu {
        let ph = i as f32 / nu as f32 * 2.0 * PI;
        let n = vec3(ph.cos(), 0.0, ph.sin());
        for (k, y) in [h * 0.5, -h * 0.5].iter().enumerate() {
            v.push(Vertex { position: vec3(n.x * r, *y, n.z * r), uv: vec2(i as f32 / nu as f32 * 2.0, k as f32), color: [255; 4], normal: vec4(n.x, 0.0, n.z, 1.0) });
        }
    }
    for i in 0..nu {
        let a = (i * 2) as u16;
        idx.extend_from_slice(&[a, a + 1, a + 2, a + 2, a + 1, a + 3]);
    }
    for (y, ny) in [(h * 0.5, 1.0f32), (-h * 0.5, -1.0)] {
        let c = v.len() as u16;
        v.push(Vertex { position: vec3(0.0, y, 0.0), uv: vec2(0.5, 0.5), color: [255; 4], normal: vec4(0.0, ny, 0.0, 1.0) });
        for i in 0..=nu {
            let ph = i as f32 / nu as f32 * 2.0 * PI;
            v.push(Vertex { position: vec3(ph.cos() * r, y, ph.sin() * r), uv: vec2(0.5 + ph.cos() * 0.5, 0.5 + ph.sin() * 0.5), color: [255; 4], normal: vec4(0.0, ny, 0.0, 1.0) });
        }
        for i in 0..nu as u16 {
            if ny > 0.0 {
                idx.extend_from_slice(&[c, c + 2 + i, c + 1 + i]);
            } else {
                idx.extend_from_slice(&[c, c + 1 + i, c + 2 + i]);
            }
        }
    }
    vec![(v, idx)]
}

/// A loaded mesh (custom material preview), centred and scaled to fit a radius of 1, split into
/// pieces small enough for 16-bit indices.
pub fn geom_preview_mesh(g: &crate::model::MeshGeom) -> Vec<(Vec<Vertex>, Vec<u16>)> {
    if g.pos.is_empty() {
        return vec![];
    }
    let (mut a, mut b) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in &g.pos {
        a = a.min(*p);
        b = b.max(*p);
    }
    let c = (a + b) * 0.5;
    let s = 1.0 / ((b - a).length() * 0.5).max(1e-6);
    let mut out = vec![];
    let mut map: std::collections::HashMap<u32, u16> = std::collections::HashMap::new();
    let (mut v, mut idx): (Vec<Vertex>, Vec<u16>) = (vec![], vec![]);
    for tri in g.idx.chunks(3) {
        if tri.len() < 3 {
            break;
        }
        if v.len() + 3 > 65000 {
            out.push((std::mem::take(&mut v), std::mem::take(&mut idx)));
            map.clear();
        }
        for &i in tri {
            let k = *map.entry(i).or_insert_with(|| {
                let iu = i as usize;
                let n = g.nrm.get(iu).copied().unwrap_or(Vec3::Y);
                v.push(Vertex { position: (g.pos[iu] - c) * s, uv: g.uv.get(iu).copied().unwrap_or(Vec2::ZERO), color: [255; 4], normal: vec4(n.x, n.y, n.z, 1.0) });
                (v.len() - 1) as u16
            });
            idx.push(k);
        }
    }
    if !idx.is_empty() {
        out.push((v, idx));
    }
    out
}

pub fn card_mesh(s: f32) -> Vec<(Vec<Vertex>, Vec<u16>)> {
    let n = vec4(0.0, 0.0, 1.0, 1.0);
    let v = vec![
        Vertex { position: vec3(-s, s, 0.0), uv: vec2(0.0, 0.0), color: [255; 4], normal: n },
        Vertex { position: vec3(s, s, 0.0), uv: vec2(1.0, 0.0), color: [255; 4], normal: n },
        Vertex { position: vec3(s, -s, 0.0), uv: vec2(1.0, 1.0), color: [255; 4], normal: n },
        Vertex { position: vec3(-s, -s, 0.0), uv: vec2(0.0, 1.0), color: [255; 4], normal: n },
    ];
    vec![(v, vec![0, 2, 1, 0, 3, 2])]
}
