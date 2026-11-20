//! 3D viewport: camera, lit material shader, grid, and GPU mesh upload.
use crate::gen::TreeGeo;
use crate::model::Material;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation, PipelineParams, UniformType};
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::f32::consts::PI;

pub const VIEWS: [&str; 7] = ["Perspective", "Front", "Back", "Left", "Right", "Top", "Bottom"];

#[derive(Clone, Copy)]
pub struct Cam {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    pub target: Vec3,
    pub ortho: bool,
    pub view: usize,
    pub fov: f32,
    /// Turn around the view direction (radians): a custom camera's Roll (picture 50).
    pub roll: f32,
    /// A custom camera's clip start and end, instead of the automatic ones.
    pub clip: Option<(f32, f32)>,
}

impl Cam {
    pub fn new() -> Self {
        Cam { yaw: 0.6, pitch: 0.18, dist: 70.0, target: vec3(0.0, 14.0, 0.0), ortho: false, view: 0, fov: 45.0, roll: 0.0, clip: None }
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
        // close in, the view keeps going forward past its centre instead of stopping (picture 57):
        // under half a unit away the centre itself moves ahead, so the camera can pass through
        // (and clip into) what's in front of it
        let nd = self.dist * 0.88f32.powf(steps);
        const NEAR: f32 = 0.5;
        if steps > 0.0 && nd < NEAR {
            let step = (self.dist.max(NEAR) * (1.0 - 0.88f32.powf(steps))).max(0.05);
            self.target -= self.dir() * step;
            self.dist = NEAR;
        } else {
            self.dist = nd.clamp(NEAR, 2000.0);
        }
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
        let mut up = if self.pitch.abs() > 1.5 { vec3(-self.yaw.sin(), 0.0, -self.yaw.cos()) * self.pitch.signum() } else { Vec3::Y };
        if self.roll != 0.0 {
            // turned around the view direction (the camera looks along -dir)
            let fwd = -self.dir();
            let right = fwd.cross(up).normalize_or_zero();
            let up0 = right.cross(fwd).normalize_or_zero();
            up = (up0 * self.roll.cos() + right * self.roll.sin()).normalize_or(Vec3::Y);
        }
        let (auto_near, auto_far) = if self.ortho { (1.0, self.dist.max(1.0) * 60.0) } else { ((self.dist * 0.01).max(0.05), self.dist * 40.0 + 500.0) };
        let (z_near, z_far) = match self.clip {
            // an orthographic camera sits far back (position()): its planes count from the target's distance
            Some((n, f)) if self.ortho => ((self.dist.max(1.0) * 20.0 - self.dist + n).max(0.01), self.dist.max(1.0) * 20.0 - self.dist + f.max(n + 0.01)),
            Some((n, f)) => (n.max(0.001), f.max(n + 0.01)),
            None => (auto_near, auto_far),
        };
        Camera3D {
            position: pos,
            target: self.target,
            up,
            fovy: if self.ortho { self.view_height() } else { self.fov.to_radians() },
            aspect: Some(aspect),
            projection: if self.ortho { Projection::Orthographics } else { Projection::Perspective },
            render_target: rt,
            viewport,
            z_near,
            z_far,
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
uniform float Time;
uniform float Wind;
uniform float IsLeaf;
uniform vec2 WindDir;
void main() {
    vec4 w = Model * vec4(position, 1.0);
    // wind: the tree sways more the higher it is, mostly along the wind direction; leaves flutter
    if (Wind > 0.0) {
        float h = max(w.y, 0.0);
        float along = (0.6 + 0.4 * sin(Time * 1.3 + w.x * 0.05 + w.z * 0.04)) * Wind * 0.006 * h * h / (8.0 + h);
        float across = cos(Time * 1.1 + w.x * 0.05) * Wind * 0.002 * h * h / (8.0 + h);
        w.x += WindDir.x * along - WindDir.y * across;
        w.z += WindDir.y * along + WindDir.x * across;
        w.y += IsLeaf * sin(Time * 7.0 + w.x * 3.1 + w.z * 2.3) * Wind * 0.05;
    }
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
uniform float HasRoughMap;
uniform sampler2D RoughMap;
uniform float Shading;
uniform sampler2D MatCap;
uniform vec4 SelColor;
uniform float Mode;
uniform float Season;
uniform float IsLeaf;
uniform float OneSided;
uniform float Overdraw;
uniform vec3 SunColor;
uniform vec3 AmbSky;
uniform vec3 AmbHorizon;
uniform vec3 AmbGround;
uniform vec4 AOParams;
uniform float Wrap;
uniform float FlipBack;
float grey(float v) { return clamp(v, 0.0, 1.0); }
float hash3(vec3 p) { return fract(sin(dot(p, vec3(127.1, 311.7, 74.7))) * 43758.5453); }
float vnoise(vec3 p) {
    vec3 i = floor(p);
    vec3 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = mix(mix(hash3(i), hash3(i + vec3(1.0, 0.0, 0.0)), f.x), mix(hash3(i + vec3(0.0, 1.0, 0.0)), hash3(i + vec3(1.0, 1.0, 0.0)), f.x), f.y);
    float b = mix(mix(hash3(i + vec3(0.0, 0.0, 1.0)), hash3(i + vec3(1.0, 0.0, 1.0)), f.x), mix(hash3(i + vec3(0.0, 1.0, 1.0)), hash3(i + vec3(1.0, 1.0, 1.0)), f.x), f.y);
    return mix(a, b, f.z);
}
void main() {
    vec4 tex = texture2D(Texture, uv);
    if (tex.a < Cutoff) discard;
    // Welding's texture blend (picture 58): a welded base dissolves into its parent's bark
    // through a ragged, bark-like pattern (three octaves of noise in space), so the parent's own
    // bark shows through and the seam disappears; a thin darker rim follows the edge like a
    // crease in the bark. The fade arrives in the colour's alpha (gen.rs): unselected 1.0 down to
    // 0.92, selected 0.5 down to 0.42.
    float weldShade = 1.0;
    {
        float a = color.a;
        float fade = a < 0.75 ? (0.502 - a) / 0.078 : (1.0 - a) / 0.078;
        fade = clamp(fade, 0.0, 1.0);
        if (fade > 0.02) {
            vec3 q = wp * 2.3;
            float nz = vnoise(q) * 0.55 + vnoise(q * 2.7 + 11.3) * 0.3 + vnoise(q * 7.9 + 3.7) * 0.15;
            if (fade > nz) discard;
            weldShade = mix(0.84, 1.0, smoothstep(0.0, 0.08, nz - fade));
        }
    }
    // Roughness (picture 25): the material's Roughness value (Gloss uniform = 1 - roughness)
    // times its Roughness picture, white = rough, black = glossy; Gl is the gloss it gives
    float Rgh = 1.0 - Gloss;
    if (HasRoughMap > 0.5) Rgh *= texture2D(RoughMap, uv).r;
    float Gl = 1.0 - Rgh;
    if (OneSided > 0.5 && !gl_FrontFacing) discard;
    // Overdraw: every layer adds red at opacity 0.05 (render_modes/Other/Overdraw.fx)
    if (Overdraw > 0.5) { gl_FragColor = vec4(1.0, 0.0, 0.0, 0.05); return; }
    vec3 n = normalize(vn);
    // Preferences > Render > Flip normals on backsides (picture 45)
    if (!gl_FrontFacing && FlipBack > 0.5) {
        n = -n;
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
    vec3 base = tex.rgb * Tint.rgb * color.rgb * weldShade;
    // season (Edit > Season): leaves go from fresh green (spring) through summer to autumn
    // orange and winter brown, keeping their brightness
    if (IsLeaf > 0.5 && Season >= 0.0) {
        vec3 sc; float k;
        if (Season < 0.25) { sc = vec3(0.62, 0.9, 0.35); k = (0.25 - Season) / 0.25 * 0.35; }
        else if (Season < 0.5) { sc = vec3(1.0); k = 0.0; }
        else if (Season < 0.85) { sc = vec3(0.95, 0.5, 0.1); k = (Season - 0.5) / 0.35 * 0.75; }
        else { float t = (Season - 0.85) / 0.15; sc = mix(vec3(0.95, 0.5, 0.1), vec3(0.45, 0.32, 0.18), t); k = 0.75 + t * 0.15; }
        float lum = dot(base, vec3(0.3, 0.59, 0.11));
        base = mix(base, sc * lum * 2.2, k);
    }
    // Render modes, following the install's render_modes/*.fx files
    vec3 L = normalize(LightDir);
    vec3 V = normalize(CamPos - wp);
    float ndl = dot(n, L);
    // Preferences > Lighting > Ambient Occlusion (picture 42): Min..Max, then Contrast and Brightness
    float occ2 = clamp((mix(AOParams.z, AOParams.w, occ) - 0.5) * (1.0 + AOParams.y) + 0.5 + AOParams.x, 0.0, 1.0);
    float aoV = mix(1.0, occ2, AOAmount);
    vec3 albedo = tex.rgb * Tint.rgb;
    // the viewport's shading menu (picture 27): MatCap reads a lit-sphere picture by the
    // normal as the camera sees it; Flat shows the colours with no lighting
    if (Mode < 0.5 && Shading > 0.5) {
        vec3 o;
        // Shading 1 MatCap, 3 MatCap with the texture's colours (picture 29), 2 Flat
        if (Shading < 1.5 || Shading > 2.5) {
            vec3 f = normalize(CamPos - wp);
            vec3 rt = cross(vec3(0.0, 1.0, 0.0), f);
            rt = length(rt) < 0.01 ? vec3(1.0, 0.0, 0.0) : normalize(rt);
            vec3 up = cross(f, rt);
            vec2 mc = vec2(dot(n, rt), dot(n, up)) * 0.48 + 0.5;
            o = texture2D(MatCap, vec2(mc.x, 1.0 - mc.y)).rgb;
            if (Shading > 2.5) o = clamp(o * base * 1.35, 0.0, 1.0);
        } else {
            o = base;
        }
        if (color.a < 0.9) o = mix(o, SelColor.rgb, SelColor.a);
        gl_FragColor = vec4(o, 1.0); return;
    }
    // the original's AmbientColor(normal, k): a sky/ground ambient term
    // neutral (uncoloured) ambient and sun so a material shows its texture's own colours (picture 25)
    // the original's Environment > Ambient (picture 42): Sky above, Horizon level, Ground below,
    // times Ambient Intensity
    vec3 ambLight = n.y >= 0.0 ? mix(AmbHorizon, AmbSky, n.y) : mix(AmbHorizon, AmbGround, -n.y);
    if (Mode > 19.5 && Mode < 29.5) {
        // Material: Color, Opacity (white at its opacity on black), Gloss, Normal (screen space),
        // Specular colour, Metallic, Subsurface colour, Subsurface Amount, Custom
        vec3 o = albedo;
        if (Mode > 20.5 && Mode < 21.5) o = vec3(tex.a);
        else if (Mode > 21.5 && Mode < 22.5) o = vec3(grey(Rgh));
        else if (Mode > 22.5 && Mode < 23.5) {
            vec3 rt = normalize(cross(vec3(0.0, 1.0, 0.0), V));
            vec3 up = cross(V, rt);
            o = vec3(dot(n, rt), dot(n, up), dot(n, V)) * 0.5 + 0.5;
        }
        else if (Mode > 23.5 && Mode < 24.5) o = vec3(grey(Specular));
        else if (Mode > 24.5 && Mode < 25.5) o = vec3(grey(Metallic));
        else if (Mode > 25.5 && Mode < 26.5) o = SubColor.rgb;
        else if (Mode > 26.5 && Mode < 27.5) o = vec3(grey(SubColor.a));
        else if (Mode > 27.5) o = vec3(0.0);
        gl_FragColor = vec4(o, 1.0); return;
    }
    if (Mode > 29.5 && Mode < 34.5) {
        // Vertex Colors: Color, then each channel in its own colour
        vec3 vc = color.rgb;
        if (Mode > 30.5 && Mode < 31.5) vc = vec3(vc.r, 0.0, 0.0);
        else if (Mode > 31.5 && Mode < 32.5) vc = vec3(0.0, vc.g, 0.0);
        else if (Mode > 32.5) vc = vec3(0.0, 0.0, vc.b);
        gl_FragColor = vec4(vc, 1.0); return;
    }
    // Depth: the screen depth
    if (Mode > 45.5 && Mode < 46.5) { gl_FragColor = vec4(vec3(gl_FragCoord.z), 1.0); return; }
    if (Mode > 47.5 && Mode < 48.5) { gl_FragColor = vec4(fract(uv), 0.0, 1.0); return; }
    if (Mode > 48.5 && Mode < 49.5) {
        // Texcoord Seams: saturate((sin(u, v, u + v) * 0.5 + 0.5)^2)
        vec3 s = sin(vec3(uv, uv.x + uv.y)) * 0.5 + 0.5;
        gl_FragColor = vec4(clamp(s * s, 0.0, 1.0), 1.0); return;
    }
    // Billboard Subsurface Amount: black at opacity 1 - sqrt(amount) x AO over white
    if (Mode > 44.5 && Mode < 45.5) { gl_FragColor = vec4(vec3(grey(sqrt(SubColor.a) * occ)), 1.0); return; }
    if (Mode > 1.5 && Mode < 2.5) { gl_FragColor = vec4(vec3(occ), 1.0); return; }
    if (Mode > 2.5 && Mode < 3.5) {
        // Albedo Check: sRGB luminance against 0.05..0.9 (0.25..1.0 for metal); the cube of
        // where it falls (-1..1) blends toward blue when too dark, red when too bright
        vec2 rc = mix(vec2(0.05, 0.9), vec2(0.25, 1.0), Metallic);
        float lum = pow(max(dot(albedo, vec3(0.2126, 0.7152, 0.0722)), 0.0), 1.0 / 2.2);
        float f = clamp((lum - rc.x) / (rc.y - rc.x), 0.0, 1.0) * 2.0 - 1.0;
        f = f * f * f;
        // dark side: f *= -f * f (so the blend is -f^3 of the cubed value), bright side: f x (1 - metallic)
        vec3 o = f < 0.0 ? mix(vec3(lum), vec3(0.0, 0.0, 1.0), -f * f * f) : mix(vec3(lum), vec3(1.0, 0.0, 0.0), f * (1.0 - Metallic));
        gl_FragColor = vec4(o, 1.0); return;
    }
    if (Mode > 3.5 && Mode < 4.5) {
        // Subsurface Check: the same ramp on the subsurface colour x amount, scaled by amount
        float lum = pow(max(dot(SubColor.rgb * SubColor.a, vec3(0.2126, 0.7152, 0.0722)), 0.0), 1.0 / 2.2);
        float f = clamp((lum - 0.05) / 0.85, 0.0, 1.0) * 2.0 - 1.0;
        f = f * f * f * SubColor.a;
        vec3 o = f < 0.0 ? mix(vec3(lum), vec3(0.0, 0.0, 1.0), -f) : mix(vec3(lum), vec3(1.0, 0.0, 0.0), f);
        gl_FragColor = vec4(o, 1.0); return;
    }
    if (Mode > 0.5 && Mode < 1.5) {
        // Scribed: plain white, pinker where see-through, darker toward grazing angles
        float lit = dot(normalize(vn), V);
        vec3 o = mix(vec3(0.8, 0.7, 0.7), vec3(1.0), tex.a) * mix(1.0, 0.8, 1.0 - lit * lit);
        gl_FragColor = vec4(o, 1.0); return;
    }
    if (Mode > 39.5 && Mode < 44.5) {
        // Other: Phong, Blinn-Phong, Oren-Nayar, Lambert, Flat (Blinn-Phong; the original uses
        // a faceted normal from screen derivatives)
        vec3 amb = albedo * ambLight * 0.5 / 0.75;
        float NoL = max(0.0, ndl);
        float sh = max(0.001, Rgh); sh *= sh;
        vec3 o;
        if (Mode < 40.5) {
            float s = (1.0 / sh - 1.0) * 2.0;
            float RoV = max(0.0, dot(reflect(-L, n), V));
            float sp = clamp(pow(RoV, s) * (s + 1.0) / (3.14159 * 3.0), 0.0, 1.0);
            o = albedo * NoL + vec3(Specular) * sp + amb;
        } else if (Mode < 41.5 || Mode > 43.5) {
            float s = (1.0 / sh - 1.0) * 8.0;
            float NoH = max(0.001, dot(n, normalize(L + V)));
            float sp = pow(NoH, s) * (s + 4.0) / (3.14159 * 12.0);
            o = albedo * max(0.001, NoL) + vec3(Specular) * sp + amb;
        } else if (Mode < 42.5) {
            float VoL = dot(V, L);
            float NoV = dot(n, V);
            float sNoV = abs(NoV) + 0.1;
            float r2 = Rgh * Rgh;
            float s = VoL - NoL * sNoV;
            float t = mix(NoL, min(1.0, NoL / sNoV), step(0.0, s));
            float A = 1.0 - 0.5 * r2 / (r2 + 0.33);
            float B = 0.45 * r2 / (r2 + 0.09) * s * t;
            o = amb + albedo * NoL * (A + B);
        } else {
            o = amb + albedo * NoL;
        }
        gl_FragColor = vec4(o * aoV, 1.0); return;
    }
    if (Unlit > 0.5) { vec3 u = base; if (color.a < 0.9) { u = mix(u, SelColor.rgb, SelColor.a); } gl_FragColor = vec4(u, 1.0); return; }
    // Lighting > Untextured: every colour 0.5
    if (Mode > 9.5 && Mode < 10.5) base = vec3(0.5);
    if (Mode > 4.5 && Mode < 5.5) base = vec3(1.0);
    // Shadows > Style's wrap (picture 44): light reaching round onto the shadow side
    float diff = max((ndl + Wrap) / (1.0 + Wrap), 0.0);
    vec3 H = normalize(L + V);
    float sp = pow(max(dot(n, H), 0.0), mix(4.0, 96.0, Gl)) * Specular * (0.4 + Gl);
    float ao = aoV;
    vec3 amb = base * ambLight * 0.75;
    vec3 sun = SunColor;
    vec3 dif = base * sun * diff * (1.0 - Metallic * 0.6);
    vec3 trans = base * SubColor.rgb * SubColor.a * max(-ndl, 0.0) * 1.4 * TwoSided;
    vec3 spec = sp * mix(vec3(1.0), base * 2.0, Metallic) * sun;
    vec3 c = (amb + dif + trans) * ao + spec * ao;
    // Lighting parts on their own, as the original's files keep them: Diffuse and Specular and
    // Subsurface x half AO ((1 + AO) / 2), Ambient x AO; Shadow is the shadow term (TreeCraft has
    // no shadow maps: lit sides 1)
    float halfAO = (1.0 + ao) * 0.5;
    if (Mode > 10.5 && Mode < 11.5) c = dif * halfAO;
    else if (Mode > 11.5 && Mode < 12.5) c = spec * halfAO;
    else if (Mode > 12.5 && Mode < 13.5) c = amb * ao;
    else if (Mode > 13.5 && Mode < 14.5) c = trans * halfAO;
    else if (Mode > 14.5 && Mode < 15.5) c = vec3(1.0);
    else if (Mode > 4.5 && Mode < 5.5) c = (amb + dif) * ao;
    c = pow(c, vec3(0.92));
    // the selected generator: Selection > Solid fill colour at its strength (picture 30)
    if (color.a < 0.9) { c = mix(c, SelColor.rgb, SelColor.a); }
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
        UniformDesc::new("HasRoughMap", UniformType::Float1),
        UniformDesc::new("Shading", UniformType::Float1),
        UniformDesc::new("SelColor", UniformType::Float4),
        UniformDesc::new("Mode", UniformType::Float1),
        UniformDesc::new("Season", UniformType::Float1),
        UniformDesc::new("IsLeaf", UniformType::Float1),
        UniformDesc::new("OneSided", UniformType::Float1),
        UniformDesc::new("Overdraw", UniformType::Float1),
        UniformDesc::new("Time", UniformType::Float1),
        UniformDesc::new("Wind", UniformType::Float1),
        UniformDesc::new("WindDir", UniformType::Float2),
        UniformDesc::new("SunColor", UniformType::Float3),
        UniformDesc::new("AmbSky", UniformType::Float3),
        UniformDesc::new("AmbHorizon", UniformType::Float3),
        UniformDesc::new("AmbGround", UniformType::Float3),
        UniformDesc::new("AOParams", UniformType::Float4),
        UniformDesc::new("Wrap", UniformType::Float1),
        UniformDesc::new("FlipBack", UniformType::Float1),
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
            textures: vec!["NormalMap".to_string(), "RoughMap".to_string(), "MatCap".to_string()],
        },
    )
    .map_err(|e| format!("{:?}", e))
}

/// Selection > Outline (picture 30), first pass: the selected generator's shape in white on clear,
/// drawn into a picture the size of the viewport (same vertex shader, so wind matches).
const FS_MASK: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying mediump vec2 uv;
uniform sampler2D Texture;
uniform float Cutoff;
void main() {
    if (texture2D(Texture, uv).a < Cutoff) discard;
    if (color.a > 0.9) discard;
    gl_FragColor = vec4(1.0);
}"#;

/// Second pass: drawn over the viewport, colouring the pixels just outside that shape.
const VS_2D: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying lowp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
}"#;

const FS_OUTLINE: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec4 OutColor;
uniform vec2 Texel;
uniform float Thick;
void main() {
    if (texture2D(Texture, uv).r > 0.5) discard;
    float m = 0.0;
    for (int i = 0; i < 16; i++) {
        float a = float(i) * 0.3927;
        vec2 o = vec2(cos(a), sin(a)) * Thick * Texel;
        m = max(m, texture2D(Texture, uv + o).r);
        m = max(m, texture2D(Texture, uv + o * 0.5).r);
    }
    if (m < 0.5) discard;
    gl_FragColor = OutColor;
}"#;

pub fn make_mask_material() -> Result<macroquad::material::Material, String> {
    let uniforms = vec![
        UniformDesc::new("Time", UniformType::Float1),
        UniformDesc::new("Wind", UniformType::Float1),
        UniformDesc::new("IsLeaf", UniformType::Float1),
        UniformDesc::new("WindDir", UniformType::Float2),
        UniformDesc::new("Cutoff", UniformType::Float1),
    ];
    load_material(ShaderSource::Glsl { vertex: VS, fragment: FS_MASK }, MaterialParams { pipeline_params: PipelineParams::default(), uniforms, textures: vec![] }).map_err(|e| format!("{:?}", e))
}

pub fn make_outline_material() -> Result<macroquad::material::Material, String> {
    let uniforms = vec![UniformDesc::new("OutColor", UniformType::Float4), UniformDesc::new("Texel", UniformType::Float2), UniformDesc::new("Thick", UniformType::Float1)];
    load_material(
        ShaderSource::Glsl { vertex: VS_2D, fragment: FS_OUTLINE },
        MaterialParams {
            pipeline_params: PipelineParams { color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))), ..Default::default() },
            uniforms,
            textures: vec![],
        },
    )
    .map_err(|e| format!("{:?}", e))
}

/// The tree's shadow on the ground (picture 42): every vertex slid along the sun's direction down
/// to the ground (y = Lift), drawn dark and see-through. Depth test "less" with depth writing means
/// overlapping parts of one pass don't darken twice; leaves keep their cut-out shape.
const VS_SHADOW: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying mediump vec2 uv;
varying highp vec2 gxz;
uniform mat4 Model;
uniform mat4 Projection;
uniform vec3 LightDir;
uniform float Lift;
uniform vec2 Shift;
void main() {
    vec4 w = Model * vec4(position, 1.0);
    vec3 L = normalize(LightDir);
    float t = max(w.y, 0.0) / max(L.y, 0.05);
    vec3 p = w.xyz - L * t;
    p.y = Lift;
    p.xz += Shift;
    gl_Position = Projection * vec4(p, 1.0);
    uv = texcoord;
    gxz = p.xz;
}"#;

const FS_SHADOW: &str = r#"#version 100
precision mediump float;
varying mediump vec2 uv;
varying highp vec2 gxz;
uniform sampler2D Texture;
uniform float Cutoff;
uniform vec4 ShadowColor;
uniform vec4 GroundCol;
uniform float Radius;
void main() {
    if (texture2D(Texture, uv).a < Cutoff) discard;
    // solid (picture 44): the colour the ground (or the background past its edge) has there,
    // darkened by ShadowColor.a - so parts of the shadow drawn over each other can't flicker
    float r = length(gxz) / max(Radius, 0.001);
    float g = r < 0.66 ? 1.0 : clamp((1.0 - r) / 0.34, 0.0, 1.0);
    vec3 base = mix(ShadowColor.rgb, GroundCol.rgb, g);
    gl_FragColor = vec4(base * (1.0 - GroundCol.a), 1.0);
}"#;

pub fn make_shadow_material() -> Result<macroquad::material::Material, String> {
    let uniforms = vec![
        UniformDesc::new("LightDir", UniformType::Float3),
        UniformDesc::new("Lift", UniformType::Float1),
        UniformDesc::new("Shift", UniformType::Float2),
        UniformDesc::new("Cutoff", UniformType::Float1),
        UniformDesc::new("ShadowColor", UniformType::Float4),
        UniformDesc::new("GroundCol", UniformType::Float4),
        UniformDesc::new("Radius", UniformType::Float1),
    ];
    load_material(
        ShaderSource::Glsl { vertex: VS_SHADOW, fragment: FS_SHADOW },
        MaterialParams {
            pipeline_params: PipelineParams { depth_test: Comparison::LessOrEqual, depth_write: true, ..Default::default() },
            uniforms,
            textures: vec![],
        },
    )
    .map_err(|e| format!("{:?}", e))
}

/// The ground under the tree (picture 42): a disc of `radius` around the origin in `col`, fading
/// out over its outer third, a little below the grid.
pub fn draw_ground(radius: f32, y: f32, col: Color) {
    let seg = 72;
    let rings = [(0.0f32, 1.0f32), (0.66, 1.0), (1.0, 0.0)];
    let mut v: Vec<Vertex> = vec![];
    let mut idx: Vec<u16> = vec![];
    for (ri, (f, a)) in rings.iter().enumerate() {
        for k in 0..=seg {
            let ang = k as f32 / seg as f32 * std::f32::consts::TAU;
            let c = Color::new(col.r, col.g, col.b, col.a * a);
            v.push(Vertex { position: vec3(ang.cos() * radius * f, y, ang.sin() * radius * f), uv: Vec2::ZERO, color: c.into(), normal: vec4(0.0, 1.0, 0.0, 1.0) });
            if ri > 0 && k < seg {
                let a0 = ((ri - 1) * (seg + 1) + k) as u16;
                let b0 = (ri * (seg + 1) + k) as u16;
                idx.extend_from_slice(&[a0, b0, a0 + 1, a0 + 1, b0, b0 + 1]);
            }
        }
    }
    draw_mesh(&Mesh { vertices: v, indices: idx, texture: None });
}

/// A flat disc at the ground (picture 46: the world origin's marker): filled with `fill`, ringed
/// with `ring` of width `ring_w`, at height `y`.
pub fn draw_disc(radius: f32, y: f32, fill: Color, ring: Color, ring_w: f32) {
    let seg = 64;
    let mut v: Vec<Vertex> = vec![Vertex { position: vec3(0.0, y, 0.0), uv: Vec2::ZERO, color: fill.into(), normal: vec4(0.0, 1.0, 0.0, 1.0) }];
    let mut idx: Vec<u16> = vec![];
    for k in 0..=seg {
        let a = k as f32 / seg as f32 * std::f32::consts::TAU;
        let (c, s) = (a.cos(), a.sin());
        v.push(Vertex { position: vec3(c * (radius - ring_w), y, s * (radius - ring_w)), uv: Vec2::ZERO, color: fill.into(), normal: vec4(0.0, 1.0, 0.0, 1.0) });
        v.push(Vertex { position: vec3(c * (radius - ring_w), y, s * (radius - ring_w)), uv: Vec2::ZERO, color: ring.into(), normal: vec4(0.0, 1.0, 0.0, 1.0) });
        v.push(Vertex { position: vec3(c * radius, y, s * radius), uv: Vec2::ZERO, color: ring.into(), normal: vec4(0.0, 1.0, 0.0, 1.0) });
        if k < seg {
            let b = (1 + k * 3) as u16;
            idx.extend_from_slice(&[0, b, b + 3]);
            idx.extend_from_slice(&[b + 1, b + 2, b + 4, b + 2, b + 5, b + 4]);
        }
    }
    draw_mesh(&Mesh { vertices: v, indices: idx, texture: None });
}

/// The same shader drawing every layer additively without depth (Render > Other > Overdraw):
/// places drawn over many times turn bright.
pub fn make_overdraw_material() -> Result<macroquad::material::Material, String> {
    let uniforms = [
        "Tint", "LightDir", "CamPos", "Cutoff", "Gloss", "Specular", "Metallic", "SubColor", "AOAmount", "TwoSided", "Bump", "Unlit", "HasNormalMap", "HasRoughMap", "Shading", "SelColor", "Mode", "Season", "IsLeaf",
        "OneSided", "Overdraw", "Time", "Wind", "WindDir", "SunColor", "AmbSky", "AmbHorizon", "AmbGround", "AOParams", "Wrap", "FlipBack",
    ]
    .iter()
    .map(|n| {
        let t = match *n {
            "Tint" | "SubColor" | "SelColor" | "AOParams" => UniformType::Float4,
            "LightDir" | "CamPos" | "SunColor" | "AmbSky" | "AmbHorizon" | "AmbGround" => UniformType::Float3,
            "WindDir" => UniformType::Float2,
            _ => UniformType::Float1,
        };
        UniformDesc::new(n, t)
    })
    .collect();
    load_material(
        ShaderSource::Glsl { vertex: VS, fragment: FS },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::Always,
                depth_write: false,
                color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
                ..Default::default()
            },
            uniforms,
            textures: vec!["NormalMap".to_string(), "RoughMap".to_string(), "MatCap".to_string()],
        },
    )
    .map_err(|e| format!("{:?}", e))
}

#[derive(Clone, Copy)]
pub struct Lighting {
    pub azimuth: f32,
    pub elevation: f32,
    pub unlit: bool,
    /// The viewport's render mode number (see the Render menu), wind sway (0 = still) and its
    /// clock, season (0 spring .. 1 winter), back faces hidden, AO forced on, overdraw view.
    pub mode: f32,
    pub wind: f32,
    pub time: f32,
    pub season: f32,
    pub one_sided: bool,
    pub ao_on: bool,
    pub overdraw: bool,
    /// Alpha testing off (Preferences > Viewport): cutout edges aren't clipped.
    pub no_alpha_test: bool,
    /// Wind direction in degrees around the vertical.
    pub wind_dir: f32,
    /// The viewport's shading menu: 0 Studio, 1 MatCap, 2 Flat (picture 27).
    pub shading: f32,
    /// Selection > Solid fill: colour and how strongly it covers the selected generator (0 = none).
    pub sel_fill: [f32; 4],
    /// Preferences > Lighting (the original's light, fn 0x140292fb0; picture 42): the sun's colour
    /// and intensity, the ambient Sky / Horizon / Ground colours and intensity, Ambient
    /// Occlusion brightness, contrast, min and max, shadow quality (SHADOW_QUALITIES) and how
    /// strongly the ground shows the shadow.
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub amb_sky: [f32; 3],
    pub amb_horizon: [f32; 3],
    pub amb_ground: [f32; 3],
    pub amb_intensity: f32,
    pub ao_brightness: f32,
    pub ao_contrast: f32,
    pub ao_min: f32,
    pub ao_max: f32,
    pub shadow_quality: usize,
    pub ground_weight: f32,
    /// How far the sun's light wraps round onto the shadow side (0 = a hard terminator; picture 44).
    pub wrap: f32,
    /// Preferences > Render > Flip normals on backsides (picture 45).
    pub flip_back: bool,
}

/// Shadow looks (picture 44), styles after well-known games (TreeCraft's own settings, not taken
/// from those games): name, wrap, ambient intensity, ground weight, AO min, AO contrast.
pub const SHADOW_PRESETS: [(&str, f32, f32, f32, f32, f32); 6] = [
    // TreeCraft's starting look (picture 45)
    ("Default", 0.3, 0.4, 0.3, 0.25, 0.0),
    ("Uncharted style (soft shadows)", 0.55, 0.75, 0.25, 0.45, 0.0),
    ("Red Dead style (realistic shadows)", 0.12, 0.45, 0.5, 0.25, 0.2),
    ("Zelda style (bright, painterly)", 0.75, 0.95, 0.2, 0.6, -0.2),
    ("The Last of Us style (moody, deep)", 0.18, 0.3, 0.6, 0.15, 0.35),
    ("Ghost of Tsushima style (warm, airy)", 0.4, 0.6, 0.35, 0.4, 0.1),
];

/// Shadows > Quality, as the original lists them (table at 0x14178e050); Medium is its default.
pub const SHADOW_QUALITIES: [&str; 5] = ["Disabled", "Low quality", "Medium quality", "High quality", "Very high quality"];

/// The original's lighting presets (app_data/light_presets.xml): name, sun intensity, ambient
/// intensity, altitude (0 overhead .. 1 at the horizon), sun colour, sky, horizon, ground.
pub const LIGHT_PRESETS: [(&str, f32, f32, f32, u32, u32, u32, u32); 5] = [
    ("Daylight", 2.0, 0.4, 0.5, 0xffffff, 0x5391d3, 0xb2e3ff, 0x8a8570),
    ("Bright sun", 4.0, 0.3, 0.5, 0xffffff, 0x0070b5, 0xa2d0f1, 0x8a8570),
    ("Dusk", 2.0, 0.3, 0.8, 0xffb291, 0x2d3459, 0xffa869, 0x433832),
    ("Overcast", 0.75, 1.0, 0.5, 0xffffff, 0x787878, 0xa7a7a7, 0x58584b),
    ("Moonlight", 0.5, 2.0, 0.5, 0xacd9ff, 0x181f26, 0x404350, 0x201e19),
];

pub fn hex_rgb(c: u32) -> [f32; 3] {
    [((c >> 16) & 255) as f32 / 255.0, ((c >> 8) & 255) as f32 / 255.0, (c & 255) as f32 / 255.0]
}

impl Lighting {
    /// Put a preset (LIGHT_PRESETS index) in place; Altitude sets the sun's height.
    pub fn apply_preset(&mut self, k: usize) {
        let (_, i, ai, alt, c, sky, hor, gr) = LIGHT_PRESETS[k.min(LIGHT_PRESETS.len() - 1)];
        self.sun_intensity = i;
        self.amb_intensity = ai;
        self.elevation = (90.0 * (1.0 - alt)).clamp(5.0, 90.0);
        self.sun_color = hex_rgb(c);
        self.amb_sky = hex_rgb(sky);
        self.amb_horizon = hex_rgb(hor);
        self.amb_ground = hex_rgb(gr);
    }

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
    gm.set_uniform("Cutoff", if m.opacity_from_alpha && !light.no_alpha_test { m.cutoff.max(0.01) } else { 0.0 });
    let wa = light.wind_dir.to_radians();
    gm.set_uniform("WindDir", vec2(wa.cos(), wa.sin()));
    gm.set_uniform("Gloss", m.gloss);
    let sc = m.specular_color;
    gm.set_uniform("Specular", sc[0].max(sc[1]).max(sc[2]));
    gm.set_uniform("Metallic", m.metallic);
    gm.set_uniform("SubColor", vec4(m.sss_color[0], m.sss_color[1], m.sss_color[2], m.sss_amount));
    gm.set_uniform("AOAmount", if light.ao_on { 1.0f32 } else { m.ao });
    gm.set_uniform("Mode", light.mode);
    gm.set_uniform("Season", light.season);
    gm.set_uniform("IsLeaf", if m.opacity_from_alpha { 1.0f32 } else { 0.0f32 });
    gm.set_uniform("OneSided", if light.one_sided { 1.0f32 } else { 0.0f32 });
    gm.set_uniform("Overdraw", if light.overdraw { 1.0f32 } else { 0.0f32 });
    gm.set_uniform("Time", light.time);
    gm.set_uniform("Wind", light.wind);
    gm.set_uniform("TwoSided", if m.two_sided { 1.0f32 } else { 0.0f32 });
    gm.set_uniform("Bump", m.normal_strength);
    gm.set_uniform("Unlit", if light.unlit { 1.0f32 } else { 0.0f32 });
    // Normal slot: use its picture when one is loaded and the slot is on
    let nmap = m.slots.get(2).filter(|s| s.enabled).and_then(|s| s.tex.clone());
    gm.set_uniform("HasNormalMap", if nmap.is_some() { 1.0f32 } else { 0.0f32 });
    let t = nmap.unwrap_or_else(flat_normal);
    gm.set_texture("NormalMap", t);
    // Roughness slot (slot 3): its picture when loaded and on
    let rmap = m.slots.get(3).filter(|s| s.enabled).and_then(|s| s.tex.clone());
    gm.set_uniform("HasRoughMap", if rmap.is_some() { 1.0f32 } else { 0.0f32 });
    gm.set_texture("RoughMap", rmap.unwrap_or_else(flat_normal));
    gm.set_uniform("Shading", light.shading);
    gm.set_uniform("SelColor", vec4(light.sel_fill[0], light.sel_fill[1], light.sel_fill[2], light.sel_fill[3]));
    gm.set_texture("MatCap", MATCAP.with(|c| c.borrow().clone()).unwrap_or_else(flat_normal));
    // the original's Daylight intensity 2 gives TreeCraft's sun of 1; Ambient 0.4 its 0.56
    let s = light.sun_intensity * 0.5;
    gm.set_uniform("SunColor", vec3(light.sun_color[0] * s, light.sun_color[1] * s, light.sun_color[2] * s));
    let a = light.amb_intensity * 1.4;
    let v3 = |c: [f32; 3]| vec3(c[0] * a, c[1] * a, c[2] * a);
    gm.set_uniform("AmbSky", v3(light.amb_sky));
    gm.set_uniform("AmbHorizon", v3(light.amb_horizon));
    gm.set_uniform("AmbGround", v3(light.amb_ground));
    gm.set_uniform("AOParams", vec4(light.ao_brightness, light.ao_contrast, light.ao_min, light.ao_max));
    gm.set_uniform("Wrap", light.wrap);
    gm.set_uniform("FlipBack", if light.flip_back { 1.0f32 } else { 0.0 });
}

thread_local! {
    /// The MatCap picture the viewport's shading menu chose.
    static MATCAP: std::cell::RefCell<Option<Texture2D>> = const { std::cell::RefCell::new(None) };
}

/// Use this MatCap picture from now on.
pub fn set_matcap(t: Texture2D) {
    MATCAP.with(|c| *c.borrow_mut() = Some(t));
}

/// MatCap pictures TreeCraft makes itself (the original has no MatCap shading; picture 27).
pub const MATCAPS: [&str; 5] = ["Clay", "Chrome", "Jade", "Red wax", "Ceramic"];

/// A lit-sphere picture (RGBA, rows from the top) for a MatCap or a ball preview: kind 0-4 as
/// MATCAPS, 10 = studio light from `sun`, 11 = flat grey. MatCaps fill the corners with the
/// edge colour; previews leave them clear.
pub fn matcap_image(kind: usize, size: usize, sun: Vec3) -> Vec<u8> {
    // kind + 20: a MatCap drawn as a ball preview (clear corners)
    let preview = kind >= 20;
    let kind = kind % 20;
    let mut px = vec![0u8; size * size * 4];
    let l = vec3(-0.45, 0.55, 0.7).normalize();
    for y in 0..size {
        for x in 0..size {
            let fx = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            let fy = 1.0 - (y as f32 + 0.5) / size as f32 * 2.0;
            let r2 = fx * fx + fy * fy;
            let inside = r2 <= 1.0;
            let (nx, ny) = if inside { (fx, fy) } else { (fx / r2.sqrt(), fy / r2.sqrt()) };
            let n = vec3(nx, ny, (1.0 - nx * nx - ny * ny).max(0.0).sqrt());
            let h = (l + Vec3::Z).normalize();
            let ndl = n.dot(l);
            let spec = |p: f32| n.dot(h).max(0.0).powf(p);
            let rim = (1.0 - n.z).powf(2.5);
            let c: Vec3 = match kind {
                0 => vec3(0.78, 0.74, 0.70) * (ndl.max(0.0) * 0.7 + 0.3) + Vec3::splat(spec(24.0) * 0.12),
                1 => {
                    let sky = vec3(0.82, 0.88, 0.98).lerp(vec3(0.2, 0.18, 0.17), ((0.15 - n.y) * 2.5).clamp(0.0, 1.0));
                    sky * (0.85 + 0.15 * (n.y * 9.0).sin()) + Vec3::splat(spec(80.0) * 0.9)
                }
                2 => vec3(0.30, 0.70, 0.50) * (ndl * 0.5 + 0.5).powf(1.4) + vec3(0.2, 0.45, 0.35) * rim + Vec3::splat(spec(40.0) * 0.3),
                3 => vec3(0.72, 0.16, 0.13) * (ndl.max(0.0) * 0.75 + 0.25) + vec3(0.9, 0.5, 0.4) * rim * 0.4 + Vec3::splat(spec(30.0) * 0.35),
                4 => vec3(0.92, 0.92, 0.95) * (ndl.max(0.0) * 0.6 + 0.4) + Vec3::splat(spec(120.0)),
                10 => {
                    let s = sun.normalize_or_zero();
                    let sl = vec3(-s.x, s.y, s.z.abs().max(0.3)).normalize();
                    Vec3::splat(0.8) * (n.dot(sl).max(0.0) * 0.75 + 0.25 * (0.65 + 0.35 * n.y))
                }
                _ => Vec3::splat(0.7),
            };
            let i = (y * size + x) * 4;
            px[i] = (c.x.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 1] = (c.y.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 2] = (c.z.clamp(0.0, 1.0) * 255.0) as u8;
            px[i + 3] = if inside || (kind < 10 && !preview) { 255 } else { 0 };
        }
    }
    px
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

/// The ground grid: `minor` lines every `step`, `major` every fifth, and the X / Z axis lines
/// (colours from Preferences > Themes > 3D Viewport).
pub fn draw_grid(size: f32, step: f32, minor: Color, major: Color, axis_x: Color, axis_z: Color) {
    let n = (size / step) as i32;
    for i in -n..=n {
        if i == 0 {
            continue;
        }
        let x = i as f32 * step;
        let c = if i % 5 == 0 { major } else { minor };
        draw_line_3d(vec3(x, 0.0, -size), vec3(x, 0.0, size), c);
        draw_line_3d(vec3(-size, 0.0, x), vec3(size, 0.0, x), c);
    }
    draw_line_3d(vec3(-size, 0.0, 0.0), vec3(size, 0.0, 0.0), axis_x);
    draw_line_3d(vec3(0.0, 0.0, -size), vec3(0.0, 0.0, size), axis_z);
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
