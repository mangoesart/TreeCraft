//! What each entry of the Generation Add menu builds (picture-20). The values come from the
//! original's template files (install folder templates/*.stt, gzip XML, one or more generators
//! each with properties such as Generation:Mode, Spine:Length:+ % of parent ...), read once and
//! kept here, so TreeCraft needs no SpeedTree files. Kinds TreeCraft doesn't grow (knots, fins,
//! stitches, targets, zones) are made from the nearest thing it has; those choices are marked
//! "estimated". "From file..." reads a .stt file itself (`load_stt`).

use crate::model::*;

/// One generator a template makes. `mode` is the original's Generation:Mode number (0 Classic,
/// 3 Absolute, 5 Phyllotaxy, 6 Interval, 7 Bifurcation ...).
#[derive(Clone, Debug)]
pub struct Gen {
    pub kind: Kind,
    pub name: String,
    pub mode: i32,
    pub number: i32,
    pub interval: f32,
    pub per_node: i32,
    pub first: f32,
    pub last: f32,
    pub len_abs: f32,
    pub len_pct: f32,
    pub angle: f32,
    pub noise: f32,
    pub rad_abs: f32,
    pub rad_pct: f32,
    pub leaf_size: f32,
    /// A tube (no taper, no flare).
    pub tube: bool,
    /// Tip radius share (taper) when set.
    pub taper: Option<f32>,
    /// Grows on the template's previous generator instead of the chosen parent.
    pub child: bool,
    /// A frond: one flat card along each parent (picture 65).
    pub frond: bool,
}

pub enum Make {
    Gen(Gen),
    /// A scene object (a Magnet stands in for the original's targets, a Sphere for zones).
    Scene(SceneKind, String),
    /// Show the ground grid (Terrain).
    Grid,
}

fn g(kind: Kind, name: &str) -> Gen {
    let d = Params::new(kind);
    Gen {
        kind,
        name: name.to_string(),
        mode: -1,
        number: d.number,
        interval: d.interval,
        per_node: d.per_node,
        first: d.first,
        last: d.last,
        len_abs: d.len_abs,
        len_pct: d.len_pct,
        angle: d.start_angle,
        noise: d.noise_amt,
        rad_abs: d.radius_abs,
        rad_pct: d.radius_pct,
        leaf_size: d.leaf_size,
        tube: false,
        taper: None,
        child: false,
        frond: false,
    }
}

/// A spine generator from a template's values (mode, Interval frequency/count, First, Last,
/// length absolute / % of parent, start angle, late noise, radius absolute / % of parent).
#[allow(clippy::too_many_arguments)]
fn spine(kind: Kind, name: &str, mode: i32, n: i32, int_c: i32, first: f32, last: f32, len_abs: f32, len_pct: f32, angle: f32, noise: f32, rad_abs: f32, rad_pct: f32) -> Gen {
    Gen { mode, number: n, interval: 2.0, per_node: int_c, first, last, len_abs, len_pct, angle, noise, rad_abs, rad_pct, ..g(kind, name) }
}

fn leaf(name: &str, mode: i32, n: i32, size: f32) -> Gen {
    Gen { mode, number: n, first: 0.0, last: 1.0, leaf_size: size, ..g(Kind::Leaf, name) }
}

/// What the Add menu entry at `trail` (its submenus, then its name) makes; None for entries
/// handled elsewhere (From file..., Default generator).
pub fn make(trail: &[String]) -> Option<Vec<Make>> {
    use Kind::*;
    let folder = if trail.len() > 1 { trail[0].as_str() } else { "" };
    let sub = if trail.len() > 2 { trail[1].as_str() } else { "" };
    let name = trail.last()?.as_str();
    let v = match (folder, sub, name) {
        // templates/*.stt
        ("", _, "Trunk") => vec![Make::Gen(spine(Trunk, "Trunk", 3, 1, 3, 0.0, 0.0, 25.0, 0.0, 0.5, 0.35, 1.0, 0.0))],
        ("", _, "Trunk Tube") => vec![Make::Gen(Gen { tube: true, ..spine(Trunk, "Trunk Tube", 3, 1, 3, 0.0, 0.0, 25.0, 0.0, 0.5, 0.0, 0.5, 0.0) })],
        ("", _, "Bifurcating Branches") => vec![Make::Gen(spine(Branch, "Bifurcating Branches", 7, 1, 3, 0.2, 0.9, 0.0, 1.0, 0.39343, 0.726, 0.0, 0.97))],
        ("", _, "Big Branches") => vec![Make::Gen(spine(Branch, "Big Branches", 6, 1, 4, 0.214698, 0.925, 0.0, 0.4, 0.35, 0.7, 0.0, 0.661671))],
        ("", _, "Branch Tubes") => vec![Make::Gen(Gen { tube: true, ..spine(Branch, "Branch Tubes", 6, 1, 3, 0.2, 0.9, 0.0, 0.75, 0.5, 0.0, 0.0, 0.75) })],
        ("", _, "Little Branches") => vec![Make::Gen(spine(Branch, "Little Branches", 6, 1, 3, 0.214698, 0.9, 0.0, 0.35, 0.339982, 0.55, 0.0, 0.5))],
        ("", _, "Single Branch") => vec![Make::Gen(spine(Branch, "Single Branch", 3, 1, 3, 0.0, 1.0, 1.0, 0.5, 0.25, 0.5, 0.0, 0.75))],
        ("", _, "Twigs") => vec![Make::Gen(spine(Branch, "Twigs", 6, 1, 3, 0.214698, 0.9, 0.0, 0.227102, 0.339982, 0.5, 0.0, 1.0))],
        // Cap closes the parent's tip: one short piece at the very end, full radius, tapering to
        // a point (estimated shape; the original's cap is its own generator type)
        ("", _, "Cap") => vec![Make::Gen(Gen { taper: Some(0.0), ..spine(Branch, "Cap", 3, 1, 3, 1.0, 1.0, 0.0, 0.03, 0.0, 0.0, 0.0, 1.0) })],
        // Fronds: one flat card along each parent, bending with it (picture 65)
        ("", _, "Fronds") => vec![Make::Gen(Gen { mode: 0, frond: true, ..leaf("Fronds", 0, 1, 1.6) })],
        ("", _, "Leaf Mesh") => vec![Make::Gen(leaf("Leaf Mesh", 3, 1, 1.0))],
        // Decorations: knots and lumps as short stubs, cavities and gashes as cards on the bark,
        // peeling bark as 100 small cards (the template's Absolute number), shell as a tube
        ("Decorations", _, "Knot") => vec![Make::Gen(Gen { taper: Some(0.3), ..spine(Branch, "Knot", 3, 1, 3, 0.0, 1.0, 0.0, 0.04, 0.5, 0.0, 0.0, 0.45) })],
        ("Decorations", _, "Lump") => vec![Make::Gen(Gen { taper: Some(0.2), ..spine(Branch, "Lump", 3, 1, 3, 0.0, 1.0, 0.0, 0.025, 0.5, 0.0, 0.0, 0.7) })],
        ("Decorations", _, "Cavity") => vec![Make::Gen(leaf("Cavity", 3, 1, 0.8))],
        ("Decorations", _, "Gash") => vec![Make::Gen(leaf("Gash", 3, 1, 0.8))],
        ("Decorations", _, "Peeling Bark") => vec![Make::Gen(leaf("Peeling Bark", 3, 100, 0.5))],
        ("Decorations", _, "Roots") => vec![
            Make::Gen(spine(Branch, "Roots", 6, 1, 3, 0.005, 0.03, 0.0, 0.287102, 0.897, 1.13455, 0.0, 0.45)),
            Make::Gen(Gen { child: true, ..spine(Branch, "Root Twigs", 6, 1, 3, 0.214698, 0.9, 0.0, 0.227102, 0.339982, 0.5, 0.0, 1.0) }),
        ],
        ("Decorations", _, "Shell") => vec![Make::Gen(Gen { tube: true, ..spine(Trunk, "Shell", 3, 1, 3, 0.0, 0.0, 25.0, 0.0, 0.5, 0.34, 0.5, 0.0) })],
        // Hand Drawn (the drawn length isn't there to read: trunk 25, branches half the parent)
        ("Hand Drawn", "", "Trunk") => vec![Make::Gen(spine(Trunk, "Trunk", 3, 1, 3, 0.0, 0.0, 25.0, 0.0, 0.5, 0.0, 1.0, 0.0))],
        ("Hand Drawn", "", "Branches") => vec![Make::Gen(spine(Branch, "Branches", 3, 1, 3, 0.0, 1.0, 0.0, 0.5, 0.5, 0.0, 0.0, 0.75))],
        ("Hand Drawn", "Legacy", "Trunk" | "SubD Trunk" | "SubD Setup") => vec![Make::Gen(g(Trunk, name))],
        ("Hand Drawn", "Legacy", "Branch" | "SubD Branch") => vec![Make::Gen(g(Branch, name))],
        ("Hand Drawn", "Legacy", "Palm Frond") => vec![
            Make::Gen(Gen { mode: 0, first: 0.1, last: 1.0, ..g(Branch, "Frond") }),
            Make::Gen(Gen { child: true, ..g(Leaf, "Leaflets") }),
        ],
        // Photogrammetry: targets become Magnets (they pull growth like the original's targets),
        // stitches continue the parent from its tip
        ("Photogrammetry", _, "Single Target") => vec![Make::Scene(SceneKind::Magnet, "Target".into())],
        ("Photogrammetry", _, "Click Placed Branches") => vec![
            Make::Scene(SceneKind::Magnet, "Target".into()),
            Make::Gen(spine(Trunk, "Branch", 3, 1, 3, 0.0, 1.0, 15.0, 0.0, 0.5, 0.35, 0.38, 0.0)),
        ],
        ("Photogrammetry", _, "Click Placed Custom Mesh") => vec![
            Make::Scene(SceneKind::Magnet, "Target".into()),
            Make::Gen(spine(Trunk, "Spine Only Branch", 3, 1, 3, 0.0, 0.0, 15.0, 0.0, 0.5, 0.35, 0.38, 0.0)),
            Make::Gen(Gen { child: true, ..leaf("Mesh", 3, 1, 1.0) }),
        ],
        ("Photogrammetry", _, "Mesh") => vec![Make::Gen(leaf("Mesh", 3, 1, 1.0))],
        ("Photogrammetry", _, "Place a spine in a mesh") => vec![
            Make::Scene(SceneKind::Magnet, "Move me with the gizmo".into()),
            Make::Gen(spine(Trunk, "Spine", 3, 1, 3, 0.0, 1.0, 25.0, 0.0, 0.5, 0.0, 0.25, 0.0)),
        ],
        ("Photogrammetry", _, "Place 3D Anchors") => vec![Make::Scene(SceneKind::Magnet, "Target".into()), Make::Gen(leaf("Leaf Mesh", 3, 1, 1.0))],
        ("Photogrammetry", _, "Procedural Branch Off Mesh") => vec![Make::Gen(spine(Trunk, "Branch", 3, 1, 3, 0.0, 0.0, 25.0, 0.0, 0.5, 0.35, 0.38, 0.0))],
        ("Photogrammetry", _, "Cutting Stitch" | "Texture Blended Extension" | "Vertex Blended Extension") => {
            vec![Make::Gen(Gen { taper: Some(0.2), ..spine(Branch, name, 3, 1, 3, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0, 0.0, 1.0) })]
        }
        // Zones: the terrain shows the ground; a zone is a marker sphere
        ("Zones", _, "Terrain") => vec![Make::Grid],
        ("Zones", _, "Zone") => vec![Make::Scene(SceneKind::Sphere, "Zone".into())],
        _ => return None,
    };
    Some(v)
}

/// Put a template generator's values into a generator's parameters.
pub fn apply(p: &mut Params, t: &Gen) {
    if t.frond {
        p.extra.insert("Leaves:Frond".to_string(), 1.0);
    }
    match t.mode {
        3 | 4 => {
            p.gen_mode = MODE_ABSOLUTE;
            p.number = t.number.max(1);
        }
        5 => p.gen_mode = MODE_PHYLLOTAXY,
        6 => {
            p.gen_mode = MODE_INTERVAL;
            p.interval = t.interval;
            p.per_node = t.per_node;
            p.jumble = 0.5;
        }
        // Bifurcation splits the parent near its end: two children per parent here (estimated)
        7 => {
            p.gen_mode = MODE_CLASSIC;
            p.frequency = 2.0;
        }
        0..=2 => p.gen_mode = MODE_CLASSIC,
        _ => {}
    }
    if t.kind != Kind::Leaf {
        p.first = t.first;
        p.last = t.last;
        p.len_abs = t.len_abs;
        p.len_pct = t.len_pct;
        p.start_angle = t.angle;
        p.noise_amt = t.noise;
        p.radius_abs = t.rad_abs;
        p.radius_pct = t.rad_pct;
        if t.tube {
            p.taper = 1.0;
            p.flare = 0.0;
        }
        if let Some(tp) = t.taper {
            p.taper = tp;
        }
    } else {
        p.first = t.first;
        p.last = t.last;
        p.leaf_size = t.leaf_size;
    }
}

/// Read a SpeedTree template (.stt: gzip XML) into generators: each <Generator Type="..."> with
/// its <Name> and the property values TreeCraft uses. Branch-type generators become trunks when
/// they have an absolute length and no length from the parent; leaf-type ones (Leaf Mesh, Frond,
/// BranchMesh) become leaf generators on the last spine generator; targets become Magnets.
pub fn load_stt(path: &std::path::Path) -> Result<Vec<Make>, String> {
    use std::io::Read;
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut text = String::new();
    let mut gz = flate2::read::GzDecoder::new(f);
    if gz.read_to_string(&mut text).is_err() {
        // not compressed: read it as plain XML
        text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    }
    if !text.contains("<SpeedTree") {
        return Err("not a SpeedTree template".into());
    }
    let mut out = vec![];
    let mut spines = 0;
    for block in text.split("<Generator Type=\"").skip(1) {
        let ty = block.split('"').next().unwrap_or("");
        let name = between(block, "<Name>", "</Name>").unwrap_or(ty).to_string();
        let val = |key: &str| -> Option<f32> {
            let at = block.find(&format!("<Name>{}</Name>", key))?;
            between(&block[at..], "<Value>", "</Value>")?.trim().parse().ok()
        };
        let num = |key: &str, d: f32| val(key).unwrap_or(d);
        match ty {
            "Target" => out.push(Make::Scene(SceneKind::Magnet, name)),
            "Zone" => out.push(Make::Scene(SceneKind::Sphere, name)),
            "Leaf Mesh" | "Frond" | "BranchMesh" | "Fin" => {
                let mut l = leaf(&name, num("Generation:Mode", 3.0) as i32, num("Generation:Absolute:Number", 1.0) as i32, num("Leaves:Size", 1.0));
                l.first = num("Generation:First", 0.0);
                l.last = num("Generation:Last", 1.0);
                l.child = spines > 0;
                out.push(Make::Gen(l));
            }
            _ => {
                let len_abs = num("Spine:Length:Absolute", 0.0);
                let len_pct = num("Spine:Length:+ % of parent", 0.5);
                let kind = if spines == 0 && len_abs > 0.0 && len_pct == 0.0 { Kind::Trunk } else { Kind::Branch };
                let mut s = spine(
                    kind,
                    &name,
                    num("Generation:Mode", 0.0) as i32,
                    num("Generation:Absolute:Number", 1.0) as i32,
                    num("Generation:Interval:Count", 3.0) as i32,
                    num("Generation:First", 0.0),
                    num("Generation:Last", 1.0),
                    len_abs,
                    len_pct,
                    num("Spine:Orientation:Start angle", 0.5),
                    num("Spine:Noise:Late:Amount", 0.0),
                    num("Skin:Radius:Absolute", 0.0),
                    num("Skin:Radius:+ % of parent", 0.0),
                );
                s.interval = num("Generation:Interval:Frequency", 2.0);
                if kind == Kind::Trunk && s.rad_abs <= 0.0 {
                    s.rad_abs = 1.0;
                }
                if kind == Kind::Branch && s.rad_abs <= 0.0 && s.rad_pct <= 0.0 {
                    s.rad_pct = 0.5;
                }
                s.child = spines > 0;
                spines += 1;
                out.push(Make::Gen(s));
            }
        }
    }
    if out.is_empty() {
        return Err("no generators in the file".into());
    }
    Ok(out)
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    let j = s[i..].find(b)? + i;
    Some(&s[i..j])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real template file reads into generators (only where SpeedTree is installed).
    #[test]
    fn reads_an_installed_template() {
        let p = std::path::Path::new(r"C:\Program Files\SpeedTree\SpeedTree Games Indie v8.4.2\templates\Decorations\Roots.stt");
        if !p.exists() {
            return;
        }
        let v = load_stt(p).expect("reads");
        let names: Vec<String> = v.iter().filter_map(|m| if let Make::Gen(g) = m { Some(g.name.clone()) } else { None }).collect();
        assert_eq!(names, vec!["Roots".to_string(), "Root Twigs".to_string()]);
    }

    /// Every entry of the Add menu (except From file... and Default generator) makes something.
    #[test]
    fn every_add_menu_entry_makes_something() {
        let menu = crate::app::App::gen_add_menu_for_tests();
        fn walk(items: &[crate::ui::MenuItem], trail: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
            for it in items {
                if it.sep {
                    continue;
                }
                trail.push(it.label.clone());
                if it.children.is_empty() {
                    out.push(trail.clone());
                } else {
                    walk(&it.children, trail, out);
                }
                trail.pop();
            }
        }
        let mut all = vec![];
        walk(&menu, &mut vec![], &mut all);
        for t in all {
            if t[0] == "From file..." || t[0] == "Default generator" {
                continue;
            }
            assert!(make(&t).map(|v| !v.is_empty()).unwrap_or(false), "nothing made for {:?}", t);
        }
    }
}
