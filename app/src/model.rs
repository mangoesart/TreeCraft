//! Scene data: generator nodes, their properties, materials and leaf meshes.
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Tree,
    Trunk,
    Branch,
    Leaf,
}

impl Kind {
    pub fn label(&self) -> &'static str {
        match self {
            Kind::Tree => "Tree",
            Kind::Trunk => "Trunk",
            Kind::Branch => "Branch",
            Kind::Leaf => "Leaf",
        }
    }
    pub fn type_label(&self) -> &'static str {
        match self {
            Kind::Tree => "Tree generator",
            Kind::Trunk | Kind::Branch => "Branch generator",
            Kind::Leaf => "Leaf mesh generator",
        }
    }
    pub fn makes_spines(&self) -> bool {
        matches!(self, Kind::Trunk | Kind::Branch)
    }
    /// Leaf generators can parent other Leaf generators (they spawn on the leaves' anchors).
    pub fn can_parent(&self) -> bool {
        true
    }
}

/// Generation modes offered (the original lists Classic, Proportional, Proportional steps,
/// Absolute, Absolute steps, Phyllotaxy, Interval, Bifurcation, Flood, Parent: fn 0x1403175a0).
pub const GEN_MODES: [&str; 4] = ["Classic", "Absolute", "Interval", "Phyllotaxy"];
pub const STYLES: [&str; 2] = ["Absolute", "Relative"];
pub const MODE_CLASSIC: usize = 0;
pub const MODE_ABSOLUTE: usize = 1;
pub const MODE_INTERVAL: usize = 2;
pub const MODE_PHYLLOTAXY: usize = 3;

#[derive(Clone, Debug)]
pub struct Params {
    // Generation
    pub gen_mode: usize,
    pub frequency: f32,
    pub number: i32,
    pub interval: f32,
    pub per_node: i32,
    pub first: f32,
    pub last: f32,
    pub jumble: f32,
    pub spiral: f32,
    pub roll_var: f32,
    pub balanced: bool,
    pub offset: f32,
    pub sweep: f32,
    pub roll: f32,
    pub relative: bool,
    pub limit: i32,
    pub size_scalar: f32,
    pub seed: i32,
    // Spine
    pub len_abs: f32,
    pub len_pct: f32,
    pub len_var: f32,
    pub len_profile: f32,
    pub start_angle: f32,
    pub angle_var: f32,
    pub angle_profile: f32,
    pub gravity: f32,
    pub noise_amt: f32,
    pub noise_turb: f32,
    pub jink: f32,
    /// Share of spine segments that jink. fn 0x140192780 converts old files with
    /// Early noise Threshold = 1 - Jink frequency; app_data/sketch trunk.stt and branch.stt have
    /// Threshold 0.5, so the default is 0.5.
    pub jink_freq: f32,
    /// Generation:Anchors (fn 0x14024b880): on a parent leaf with anchors, use all of them (0) or
    /// only anchor k (1..5).
    pub anchor_pick: usize,
    // Skin
    pub radius_abs: f32,
    pub radius_pct: f32,
    pub radius_var: f32,
    pub taper: f32,
    pub flare: f32,
    pub clamp_parent: bool,
    pub seg_len: i32,
    pub seg_radial: i32,
    pub material: usize,
    pub u_tile: f32,
    pub v_tile: f32,
    // Leaves
    pub leaf_size: f32,
    pub leaf_size_pct: f32,
    pub leaf_size_var: f32,
    pub leaf_mesh: usize,
    pub leaf_material: usize,
    pub face_sky: f32,
    pub droop: f32,
    pub fold: f32,
    pub curl: f32,
    pub leaf_twist: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    // Tree
    pub tree_radius: f32,
    pub lod_preview: i32,
    pub collect: bool,
}

impl Params {
    pub fn new(kind: Kind) -> Self {
        let mut p = Params {
            gen_mode: MODE_CLASSIC,
            frequency: 1.0,
            number: 1,
            interval: 2.0,
            per_node: 3,
            first: 0.0,
            last: 1.0,
            jumble: 0.0,
            spiral: 0.0,
            roll_var: 0.0,
            balanced: true,
            offset: 0.0,
            sweep: 1.0,
            roll: 0.0,
            relative: false,
            limit: 1000,
            size_scalar: 1.0,
            seed: 1,
            len_abs: 0.0,
            len_pct: 0.0,
            len_var: 0.0,
            len_profile: 0.0,
            start_angle: 0.5,
            angle_var: 0.0,
            angle_profile: 0.0,
            gravity: 0.0,
            noise_amt: 0.0,
            noise_turb: 0.5,
            jink: 0.0,
            jink_freq: 0.5,
            anchor_pick: 0,
            radius_abs: 1.0,
            radius_pct: 0.0,
            radius_var: 0.0,
            taper: 0.2,
            flare: 0.0,
            clamp_parent: true,
            seg_len: 8,
            seg_radial: 6,
            material: 0,
            u_tile: 1.0,
            v_tile: 1.0,
            leaf_size: 1.0,
            leaf_size_pct: 0.0,
            leaf_size_var: 0.0,
            leaf_mesh: 1,
            leaf_material: 1,
            face_sky: 0.0,
            droop: 0.0,
            fold: 0.0,
            curl: 0.0,
            leaf_twist: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            tree_radius: 10.0,
            lod_preview: 0,
            collect: false,
        };
        match kind {
            Kind::Tree => {
                p.tree_radius = 10.0; // Shape:Radius (Blank template)
            }
            Kind::Trunk => {
                // Trunk template: Generation:Mode 3 (Absolute) number 1, Spine length 25,
                // start angle 0.5, radius 1, late noise 0.35
                p.gen_mode = MODE_ABSOLUTE;
                p.number = 1;
                p.first = 0.0;
                p.last = 0.0;
                p.len_abs = 25.0;
                p.start_angle = 0.5;
                p.radius_abs = 1.0;
                p.noise_amt = 0.35;
                p.noise_turb = 0.5;
                p.taper = 0.18;
                p.flare = 0.5;
                p.seg_len = 14;
                p.seg_radial = 10;
                p.material = 0;
                p.seed = 2000;
            }
            Kind::Branch => {
                // Branch template: first 0.215, last 0.925, length 40% of parent (var 0.13),
                // start angle 0.35 (var 0.038), radius 66% of parent (var 0.05), late noise 0.7
                p.gen_mode = MODE_CLASSIC;
                p.frequency = 14.0;
                p.first = 0.215;
                p.last = 0.925;
                p.len_abs = 0.0;
                p.len_pct = 0.4;
                p.len_var = 0.33;
                p.len_profile = 0.55;
                p.start_angle = 0.35;
                p.angle_var = 0.11;
                p.angle_profile = -0.1;
                p.radius_abs = 0.0;
                p.radius_pct = 0.66;
                p.radius_var = 0.08;
                p.noise_amt = 0.7;
                p.noise_turb = 0.5;
                p.gravity = 0.12;
                p.taper = 0.12;
                p.flare = 0.0;
                p.seg_len = 8;
                p.seg_radial = 6;
                // Interval settings from the Branch template: Count 4, Spread 0.5
                p.per_node = 4;
                p.jumble = 0.5;
                p.seed = 3100;
            }
            Kind::Leaf => {
                // Leaf template: size 1, first 0, last 1
                p.gen_mode = MODE_CLASSIC;
                p.frequency = 26.0;
                p.first = 0.1;
                p.last = 1.0;
                p.start_angle = 0.3;
                p.angle_var = 0.1;
                p.leaf_size = 1.4;
                p.leaf_size_var = 0.2;
                p.face_sky = 0.45;
                p.droop = 0.15;
                p.fold = 0.15;
                p.curl = 0.1;
                p.leaf_mesh = 1;
                p.leaf_material = 1;
                p.seed = 4200;
            }
        }
        p
    }
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: u32,
    pub kind: Kind,
    pub name: String,
    pub parent: Option<u32>,
    pub pos: Vec2,
    pub p: Params,
    pub hidden: bool,
}

// ---------------- materials ----------------

pub const MAP_SOURCES: [&str; 5] = ["None", "Procedural bark", "Procedural leaf", "Procedural noise", "Image file"];

#[derive(Clone)]
pub struct ImageBuf {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl ImageBuf {
    pub fn alpha_at(&self, u: f32, v: f32) -> f32 {
        let x = ((u * self.w as f32) as isize).clamp(0, self.w as isize - 1) as usize;
        let y = ((v * self.h as f32) as isize).clamp(0, self.h as isize - 1) as usize;
        self.px[(y * self.w + x) * 4 + 3] as f32 / 255.0
    }
}

#[derive(Clone)]
pub struct Material {
    pub name: String,
    pub two_sided: bool,
    pub color: [f32; 3],
    pub source: usize,
    pub file: String,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub hue: f32,
    pub opacity_from_alpha: bool,
    pub cutoff: f32,
    pub normal_strength: f32,
    pub gloss: f32,
    /// Specular is a colour in the original's Materials panel.
    pub specular_color: [f32; 3],
    pub metallic: f32,
    pub sss_color: [f32; 3],
    pub sss_amount: f32,
    pub ao: f32,
    pub variation: f32,
    pub var_colors: [[f32; 3]; 3],
    pub cutout_mesh: usize,
    // the original's Materials panel (fn 0x140063f80, 8.4.2)
    /// Opacity value (multiplies the alpha).
    pub opacity: f32,
    pub custom_color: [f32; 3],
    pub season_color: [f32; 3],
    pub atlas_scale: f32,
    /// Index into ATLAS_MODES.
    pub atlas_mode: usize,
    /// Index into MAX_RESOLUTIONS.
    pub max_res: usize,
    pub user_data: String,
    /// Index into UNWRAP_MODES.
    pub unwrap: usize,
    /// Supplemental meshes (rows after the first in the Meshes sub-tab; fn 0x140068ed0 reads them
    /// from +0x130). The first row is cutout_mesh. 0 = None, k = mesh k-1.
    pub mesh_rows: Vec<usize>,
    /// A material set (fn 0x14006d8c0 sets +0x6cc): its members are other materials with a weight.
    pub is_set: bool,
    pub set_members: Vec<(usize, f32)>,
    /// The ten texture slots (SLOT_NAMES order).
    pub slots: Vec<MatSlot>,
    pub base: Option<ImageBuf>,
    pub adjusted: Option<ImageBuf>,
    pub tex: Option<Texture2D>,
    pub dirty_base: bool,
    pub dirty_tex: bool,
}

impl Material {
    pub fn new(name: &str, source: usize) -> Self {
        Material {
            name: name.to_string(),
            two_sided: false,
            color: [1.0, 1.0, 1.0],
            source,
            file: String::new(),
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            hue: 0.0,
            opacity_from_alpha: true,
            cutoff: 0.33,
            normal_strength: 0.5,
            // new-material values as the original's panel shows them (person's screenshot):
            // Color white, Opacity 1, Normal 1, Gloss 0.25, Specular grey, Metallic 0,
            // Subsurface white, Subsurface% 0, AO 1, Custom black, Scale 1
            gloss: 0.25,
            specular_color: [0.75, 0.75, 0.75],
            metallic: 0.0,
            sss_color: [1.0, 1.0, 1.0],
            sss_amount: 0.0,
            ao: 1.0,
            variation: 0.0,
            // Variation colours from the original's sample material
            // fn 0x1400a7390 (material constructor): +0x30..+0x50
            var_colors: [[0.28, 0.37, 0.0], [0.59, 0.72, 0.32], [0.46, 0.55, 0.46]],
            is_set: false,
            set_members: Vec::new(),
            cutout_mesh: 0,
            opacity: 1.0,
            custom_color: [0.0, 0.0, 0.0],
            season_color: [0.15, 0.15, 0.15],
            atlas_scale: 1.0,
            atlas_mode: 0,
            max_res: 0,
            user_data: String::new(),
            unwrap: 0,
            mesh_rows: vec![],
            slots: vec![MatSlot { enabled: true, ..Default::default() }; 10],
            base: None,
            adjusted: None,
            tex: None,
            dirty_base: true,
            dirty_tex: true,
        }
    }
    pub fn bark() -> Self {
        let mut m = Material::new("Bark", 1);
        m.normal_strength = 1.0;
        m.gloss = 0.15;
        m.specular_color = [0.1, 0.1, 0.1];
        m
    }
    pub fn leaf() -> Self {
        let mut m = Material::new("Leaf", 2);
        m.two_sided = true;
        m.sss_color = [0.75, 0.85, 0.3];
        m.sss_amount = 0.45;
        m.gloss = 0.45;
        m.specular_color = [0.3, 0.3, 0.3];
        m.normal_strength = 0.3;
        m.variation = 0.35;
        m.cutout_mesh = 1;
        // Broadleaf.spm's leaf material (the constructor default is 0.46 0.55 0.46)
        m.var_colors[2] = [0.46, 0.55, 0.2];
        m
    }
    /// A new material as the original's "Add new" makes it: no textures, panel defaults.
    pub fn blank(name: &str) -> Self {
        Material::new(name, 0)
    }
}

/// One texture slot of a material: on/off, the picture loaded into it, and its GPU copy.
#[derive(Clone, Default)]
pub struct MatSlot {
    pub enabled: bool,
    pub file: String,
    pub image: Option<ImageBuf>,
    pub tex: Option<Texture2D>,
}

/// The original's ten texture slots, in its panel order (fn 0x14006e3c0 / person's screenshot).
pub const SLOT_NAMES: [&str; 10] = ["Color", "Opacity", "Normal", "Gloss", "Specular", "Metallic", "Subsurface", "Subsurface%", "AO", "Custom"];
/// Which slots hold a colour value (the others hold a number).
pub const SLOT_IS_COLOR: [bool; 10] = [true, false, false, false, true, false, true, false, false, true];
/// Choices read from the original's Materials panel (fn 0x140063f80, 8.4.2).
pub const ATLAS_MODES: [&str; 3] = ["Default", "Force in", "Force out"];
pub const MAX_RESOLUTIONS: [&str; 7] = ["Default", "Original", "4096", "2048", "1024", "512", "256"];
pub const UNWRAP_MODES: [&str; 2] = ["Share", "Separate"];

impl Material {
    pub fn slot_color(&mut self, s: usize) -> Option<&mut [f32; 3]> {
        match s {
            0 => Some(&mut self.color),
            4 => Some(&mut self.specular_color),
            6 => Some(&mut self.sss_color),
            9 => Some(&mut self.custom_color),
            _ => None,
        }
    }
    pub fn slot_value(&mut self, s: usize) -> Option<&mut f32> {
        match s {
            1 => Some(&mut self.opacity),
            2 => Some(&mut self.normal_strength),
            3 => Some(&mut self.gloss),
            5 => Some(&mut self.metallic),
            7 => Some(&mut self.sss_amount),
            8 => Some(&mut self.ao),
            _ => None,
        }
    }
}

/// Something the add/remove dialog can list.
pub trait AssetItem: Clone {
    fn asset_name(&self) -> &str;
    fn set_asset_name(&mut self, n: String);
}
impl AssetItem for Material {
    fn asset_name(&self) -> &str {
        &self.name
    }
    fn set_asset_name(&mut self, n: String) {
        self.name = n;
    }
}
impl AssetItem for LeafMesh {
    fn asset_name(&self) -> &str {
        &self.name
    }
    fn set_asset_name(&mut self, n: String) {
        self.name = n;
    }
}

// ---------------- leaf meshes ----------------
//
// Mirrors the original's Meshes panel (fns 0x140076fd0 and 0x140084750 in the 8.4.2 program)
// and the <Mesh> fields stored in its .spm files (FixWinding, FlipNormals, CollisionScale,
// Weight, Wind, Flip, Orient, SpineStyle, PivotStyle, RippleDirectionIsNormal, Collection,
// MoreLodsMeshID, Scale, Lod_1...).

/// Triangle mesh: positions, normals, texture coordinates and triangle indices.
#[derive(Clone, Default)]
pub struct MeshGeom {
    pub pos: Vec<Vec3>,
    pub nrm: Vec<Vec3>,
    pub uv: Vec<Vec2>,
    pub idx: Vec<u32>,
    /// Vertex colours (empty = all white), painted in the Edit Cutout window.
    pub col: Vec<[f32; 3]>,
}

impl MeshGeom {
    pub fn tris(&self) -> usize {
        self.idx.len() / 3
    }
    pub fn is_empty(&self) -> bool {
        self.idx.is_empty()
    }
}

/// One level of detail: either a cutout outline over the reference material's picture,
/// or geometry loaded from a mesh file.
#[derive(Clone, Default)]
pub struct MeshLod {
    /// Mesh file this level was loaded from ("" for a cutout).
    pub file: String,
    /// Closed outline in texture space (u right, v down, 0..1). Empty = no cutout.
    pub outline: Vec<Vec2>,
    /// Geometry from the file, when one was loaded (takes precedence over the outline).
    pub geom: Option<MeshGeom>,
    /// Green anchors of this level (the pivot is the mesh's `anchor`); the original keeps
    /// anchors with each level ("check mesh anchors (including LOD meshes)", fn 0x1402456e0).
    pub anchors: Vec<CutAnchor>,
    /// Vertex colour of each outline point (empty = all white).
    pub colors: Vec<[f32; 3]>,
}

/// A green anchor in a cutout: picture position, angle (degrees clockwise from up) and the number
/// a generator's "Anchors" choice picks it by.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CutAnchor {
    pub pos: Vec2,
    pub angle: f32,
    pub id: usize,
    /// The original's anchor Side (fn 0x140301500): 0 Automatic, 1 Left, 2 Right. Stored only.
    pub side: u8,
}

/// Anchor Side choices, in the original's order (fn 0x140301500).
pub const ANCHOR_SIDES: [&str; 3] = ["Automatic", "Left", "Right"];

impl MeshLod {
    pub fn is_empty(&self) -> bool {
        self.geom.is_none() && self.outline.len() < 3
    }
}

pub const LOD_NAMES: [&str; 3] = ["High", "Med", "Low"];
/// Generation:Anchors choices, in the original's order (fn 0x14024b880).
pub const ANCHOR_CHOICES: [&str; 6] = ["All anchors", "1", "2", "3", "4", "5"];
/// Orient choices in the original's order, with the value each stores (fn 0x140084750).
pub const ORIENTS: [(&str, i32); 10] = [
    ("Z-Up Right-handed", 6),
    ("Y-Up Right-handed", 7),
    ("Z-Up Left-handed", 8),
    ("Y-Up Left-handed", 9),
    ("XYZ", 0),
    ("XZY", 1),
    ("YXZ", 2),
    ("YZX", 3),
    ("ZXY", 4),
    ("ZYX", 5),
];
pub const FLIPS: [&str; 8] = ["X Y Z", "-X Y Z", "X -Y Z", "X Y -Z", "-X -Y Z", "-X Y -Z", "X -Y -Z", "-X -Y -Z"];
pub const GROWTHS: [&str; 6] = ["+X", "+Y", "+Z", "-X", "-Y", "-Z"];
pub const RIPPLES: [&str; 4] = ["Normal", "X", "Y", "Z"];
pub const SPINE_STYLES: [&str; 3] = ["Branches", "Fronds", "Both"];
pub const PIVOT_STYLES: [&str; 2] = ["Origin", "First"];

#[derive(Clone)]
pub struct LeafMesh {
    pub name: String,
    /// High, Med and Low levels of detail.
    pub lods: [MeshLod; 3],
    /// Another mesh that supplies further levels of detail ("More"); 0 = none, k = mesh k-1.
    pub more: usize,
    /// Where the leaf attaches to its parent (texture space, cutouts only).
    pub anchor: Vec2,
    pub ref_material: usize,
    pub auto_points: i32,
    pub threshold: f32,
    pub margin: f32,
    pub scale: f32,
    // Transform
    /// Index into ORIENTS.
    pub orient: usize,
    /// Index into FLIPS.
    pub flip: usize,
    pub flip_normals: bool,
    pub fix_winding: bool,
    // Collection
    pub collection: bool,
    pub spine_style: usize,
    pub pivot_style: usize,
    /// Geometry gathered from generators that contribute to collections (built on compute).
    pub collected: Option<MeshGeom>,
    // Wind direction
    pub growth: usize,
    pub ripple: usize,
    // Proxy collision
    pub collision_scale: f32,
    pub weight: f32,
    // Edit Cutout window (fns 0x1400828d0, 0x1402f32a0)
    /// The geometry lives in the tree, not in files on disk ("Embedded").
    pub embedded: bool,
    pub tessellation: f32,
    pub angle: f32,
    pub double_sided: bool,
    pub improve_folding: bool,
}

pub fn card_outline() -> Vec<Vec2> {
    vec![vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)]
}

impl LeafMesh {
    pub fn card(name: &str) -> Self {
        LeafMesh {
            name: name.to_string(),
            lods: [MeshLod { file: String::new(), outline: card_outline(), geom: None, ..Default::default() }, MeshLod::default(), MeshLod::default()],
            more: 0,
            anchor: vec2(0.5, 1.0),
            ref_material: 1,
            auto_points: 10,
            threshold: 0.2,
            margin: 0.02,
            scale: 1.0,
            orient: 0,
            flip: 0,
            flip_normals: false,
            fix_winding: false,
            collection: false,
            spine_style: 0,
            pivot_style: 0,
            collected: None,
            growth: 1,
            ripple: 0,
            collision_scale: 1.0,
            weight: 1.0,
            embedded: true,
            tessellation: 0.0,
            angle: 0.0,
            double_sided: false,
            improve_folding: false,
        }
    }
    /// A new mesh as the original's "Add new" makes it: no geometry in High, Med or Low.
    pub fn empty(name: &str) -> Self {
        let mut m = LeafMesh::card(name);
        m.lods = [MeshLod::default(), MeshLod::default(), MeshLod::default()];
        m
    }
    /// The level shown for `lod`, falling back to the nearest finer level that has content.
    pub fn lod_index(&self, lod: usize) -> usize {
        let mut l = lod.min(2);
        while l > 0 && self.lods[l].is_empty() {
            l -= 1;
        }
        l
    }
}

pub fn mesh_names(meshes: &[LeafMesh]) -> Vec<String> {
    let mut v = vec!["Card (plain quad)".to_string()];
    for m in meshes {
        v.push(m.name.clone());
    }
    v
}
