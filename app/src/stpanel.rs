//! The Properties panel for a selected generator, laid out like the original's (picture-21): tabs
//! (All, Gen, Spine, Shape, Skin, UV, Displacement, Material, Animation, Segments, LOD, Hand Drawn
//! for branches; All, Gen, Leaves, Orientation, Deformation, Lighting, Animation, LOD, Collision
//! for leaves), each showing its groups of the original's properties in the original's order with
//! its defaults (stprops.rs). Properties TreeCraft's generator uses are tied to it; the others are
//! kept with the generator (Params::extra) and drawn with faint names until they're built.

use crate::model::*;
use crate::props::Rows;
use crate::stprops::{StProp, BRANCH_PROPS, LEAF_PROPS};

/// Branch generator tabs: the original's (fn 0x140136e00), each a label and the name patterns
/// (alternatives joined by "|") a property must contain to show; "All" shows everything.
/// Vertex Color has its own tab after Material (the person's request, picture 33).
pub const BRANCH_TABS: [(&str, &str); 13] = [
    ("All", ""),
    ("Gen", "Generation:"),
    ("Spine", "Spine:|Forces:"),
    ("Shape", "Shape Control:"),
    ("Skin", "Skin:"),
    ("UV", "UV Mapping|Lightmap"),
    ("Displacement", "Displacement"),
    ("Material", "Materials:|Season:|Lightmap"),
    ("Vertex Color", "Vertex Color:"),
    ("Animation", "Wind:|Growth:"),
    ("Segments", "Segments:"),
    ("LOD", "Dynamic LOD:"),
    ("Hand Drawn", "Hand Drawn:"),
];

/// Leaf mesh generator tabs: the original's (fn 0x140184db0).
pub const LEAF_TABS: [(&str, &str); 10] = [
    ("All", ""),
    ("Gen", "Generation:"),
    ("Skin", "Leaves:|Lighting:|Deformation:"),
    ("Orientation", "Orientation:|Sensitivity:|Adjustments:|Deformation:"),
    ("Lighting", "Lighting:|Material:|Lightmap"),
    ("Material", "Season|Leaves:Type"),
    ("Vertex Color", "Vertex Color:"),
    ("LOD", "Dynamic LOD:"),
    ("Animation", "Wind|Growth"),
    ("Collision", "Collision:"),
];

/// Whether a property shows on a tab with this pattern (empty = All).
fn on_tab(name: &str, pattern: &str) -> bool {
    pattern.is_empty() || pattern.split('|').any(|p| name.contains(p))
}

/// The original's Generation:Mode names by number (fn 0x1403175a0).
pub const ORIG_MODES: [&str; 10] = ["Classic", "Proportional", "Proportional Steps", "Absolute", "Absolute Steps", "Phyllotaxy", "Interval", "Bifurcation", "Flood", "Parent"];

/// Multiply a property (by its original name) by `m`: what a curve does to it at one place
/// (picture 24). Properties not tied to the generator are scaled in Params::extra.
pub fn scale_prop(p: &mut Params, name: &str, m: f32) {
    match wire(p, name) {
        Some(W::F(v, ..)) => *v *= m,
        Some(W::I(v, ..)) => *v = (*v as f32 * m).round() as i32,
        Some(W::Inv(v)) => *v = 1.0 - (1.0 - *v) * m,
        Some(W::B(_)) => {}
        None => {
            if let Some(d) = default_of(name) {
                *p.extra.entry(name.to_string()).or_insert(d) *= m as f64;
            }
        }
    }
}

/// A property's default from the original's tables (numbers and true / false).
fn default_of(name: &str) -> Option<f64> {
    BRANCH_PROPS.iter().chain(LEAF_PROPS.iter()).find(|t| t.0 == name).and_then(|t| match t.1 {
        "true" => Some(1.0),
        "false" => Some(0.0),
        s => s.parse().ok(),
    })
}

/// Settings that take any value, below 0 or above their usual top (picture 38).
fn unlimited(name: &str) -> bool {
    matches!(name, "Generation:Orientation:Sweep" | "Generation:Absolute:Sweep" | "Generation:First" | "Generation:Last")
}

/// Properties shown as a list of choices, stored in Params::extra by number (fn 0x14012ff90 for
/// UV Mapping).
fn choices(name: &str) -> Option<&'static [&'static str]> {
    Some(match name {
        "UV Mapping:Style" => &["Unrestricted", "U locked [0, 1], V unrestricted", "U and V locked [0, 1]"],
        "UV Mapping:V:Style" => &["Absolute", "Relative (keep aspect ratio)", "Relative"],
        "UV Mapping:V:Anchor" => &["Start", "End"],
        // read from the texts each definition stores before its name (picture 36)
        "Skin:Type" => &["Polygons", "SubD surface (rectangular attachments)", "SubD surface (arbitrary attachments)", "Spine only"],
        "Shape Control:Style" => &["Sphere", "Enabled force meshes"],
        "Growth:Timing:Style" => &["In place", "Parent", "Parent's start and end"],
        "Vertex Color:Red:Style" | "Vertex Color:Green:Style" | "Vertex Color:Blue:Style" => &["Set", "Offset from parent"],
        "Generation:Phyllotaxy:Arrangement:Style" => &["Opposite (distichous)", "Opposite (decussate)", "Opposite", "Alternating (distichous)", "Alternating", "Whorled"],
        _ => return None,
    })
}

/// Properties kept in Params::extra that the tree does use (gen.rs UvMap), with their ranges.
fn used_extra(name: &str) -> Option<(f32, f32)> {
    Some(match name {
        "UV Mapping:U:+ Relative" => (0.0, 4.0),
        "UV Mapping:U:Offset" | "UV Mapping:V:Offset" => (-1.0, 1.0),
        "UV Mapping:U:Fractional:Value" => (0.0, 1.0),
        "UV Mapping:U:Twist:Amount" => (-4.0, 4.0),
        "UV Mapping:U:Fractional:Enable" | "UV Mapping:U:Twist:Random flip" => (0.0, 1.0),
        // spine settings gen.rs uses (picture 36)
        "Spine:Length:+ % of radius" => (0.0, 10.0),
        "Spine:Orientation:Roll" | "Spine:Shape:Ancestor" => (-1.0, 1.0),
        "Spine:Shape:Straightness" | "Spine:Noise:Late:Smooth" => (0.0, 1.0),
        "Spine:Shape:Twist" => (-4.0, 4.0),
        "Spine:Prune:Interior" | "Spine:Prune:Up" | "Spine:Prune:Down" | "Spine:Prune:General" => (0.0, 1.0),
        "Spine:Prune:Ground" | "Spine:Break:Broken" => (0.0, 1.0),
        "Spine:Break:Chance" | "Spine:Break:Spot" => (0.0, 1.0),
        "Spine:Noise:Seed" | "Spine:Break:Seed" => (0.0, 99999.0),
        // the rest of the Spine tab (picture 38)
        "Forces:Allow forces" | "Spine:Length:Match one v tile" | "Spine:Length:Stop shape control" | "Spine:Orientation:Unify" => (0.0, 1.0),
        "Spine:Length:Shape scale" => (0.0, 2.0),
        "Spine:Orientation:Alignment" | "Spine:Noise:Early:Turbulence" => (0.0, 1.0),
        "Shape Control:Position" => (0.0, 1.0),
        "Shape Control:Size:Absolute" => (0.0, 50.0),
        "Shape Control:Size:+ % of size" => (0.0, 3.0),
        // the Segments tab (picture 39)
        "Segments:Accuracy" => (4.0, 200.0),
        "Segments:Length:Absolute" => (0.0, 4.0),
        "Segments:Length:Weld scalar" => (0.2, 5.0),
        "Segments:Length:Optimization" => (0.0, 1.0),
        "Segments:Radial: + Relative" => (0.0, 10.0),
        // the Skin tab (picture 38)
        "Skin:Visibility" | "Skin:Light seam reduction" | "Skin:Smooth" => (0.0, 1.0),
        "Skin:Radius:Length compensation" => (0.0, 5.0),
        "Skin:Radius:Length kill ratio" => (0.0, 1000.0),
        "Skin:Roll" | "Skin:Squash:Rotate" | "Skin:Intersection:Texture Blend:Offset" => (-1.0, 1.0),
        "Skin:Welding:Enabled" | "Skin:Intersection:Texture Blend:Enabled" => (0.0, 1.0),
        "Skin:Welding:Offset" => (0.0, 4.0),
        "Skin:Welding:Spread:Amount" | "Skin:Welding:Blend:Texture" | "Skin:Intersection:Texture Blend:Pull" => (0.0, 1.0),
        "Skin:Welding:Spread:Upper scale" | "Skin:Welding:Spread:Lower scale" => (0.0, 2.0),
        "Skin:Welding:Blend:Contrast" | "Skin:Welding:Blend:Noise" => (0.0, 10.0),
        "Skin:Subdivision:Relax scale" => (0.0, 2.0),
        "Skin:Subdivision:Smooth" | "Skin:Squash:Amount" => (0.0, 1.0),
        "Skin:Subdivision:Texture density" => (0.0, 4.0),
        "Skin:Subdivision:Upper spread" | "Skin:Subdivision:Lower spread" => (0.0, 8.0),
        "Skin:Splits:Chance" | "Skin:Splits:Spread 1" | "Skin:Splits:Spread 2" | "Skin:Splits:Balance" => (0.0, 1.0),
        "Skin:Splits:Smooth:Inner" | "Skin:Splits:Smooth:Outer" => (0.0, 1.0),
        "Skin:Splits:Seed" => (0.0, 99999.0),
        _ => return None,
    })
}

/// A property tied to TreeCraft's generator.
enum W<'a> {
    F(&'a mut f32, f32, f32, usize),
    I(&'a mut i32, i32, i32),
    B(&'a mut bool),
    /// Stored as 1 - value (Spine:Noise:Early:Threshold = 1 - jink frequency, fn 0x140192780).
    Inv(&'a mut f32),
}

fn wire<'a>(p: &'a mut Params, name: &str) -> Option<W<'a>> {
    Some(match name {
        "Generation:Frequency" => W::F(&mut p.frequency, 0.0, 120.0, 1),
        "Generation:Orientation:Balanced" | "Generation:Interval:Balance" => W::B(&mut p.balanced),
        "Generation:Orientation:Offset" => W::F(&mut p.offset, 0.0, 1.0, 3),
        "Generation:Orientation:Sweep" | "Generation:Absolute:Sweep" => W::F(&mut p.sweep, 0.0, 1.0, 3),
        "Generation:Orientation:Roll" | "Generation:Absolute:Roll" => W::F(&mut p.roll, -1.0, 1.0, 3),
        "Generation:Absolute:Number" => W::I(&mut p.number, 0, 60),
        "Generation:Absolute:Jumble" | "Generation:Interval:Spread" | "Generation:Phyllotaxy:Nodes:Spread" => W::F(&mut p.jumble, 0.0, 1.0, 2),
        "Generation:Interval:Frequency" => W::F(&mut p.interval, 0.05, 20.0, 2),
        "Generation:Phyllotaxy:Nodes:Internode length" => W::F(&mut p.interval, 0.1, 20.0, 2),
        "Generation:Interval:Count" | "Generation:Phyllotaxy:Arrangement:Children per node" => W::I(&mut p.per_node, 1, 12),
        "Generation:Interval:Spiral" | "Generation:Phyllotaxy:Nodes:Spiral" => W::F(&mut p.spiral, -10.0, 10.0, 2),
        "Generation:Interval:Limit" | "Generation:Phyllotaxy:Nodes:Limit" => W::I(&mut p.limit, 1, 1000),
        "Generation:First" => W::F(&mut p.first, 0.0, 1.0, 3),
        "Generation:Last" => W::F(&mut p.last, 0.0, 1.0, 3),
        "Generation:Size scalar" => W::F(&mut p.size_scalar, 0.1, 3.0, 2),
        "Random Seeds:Generation" => W::I(&mut p.seed, 0, 9999),
        "Segments:Radial:Absolute" => W::I(&mut p.seg_radial, 3, 32),
        "Segments:Length: + Relative" => W::I(&mut p.seg_len, 2, 64),
        "Spine:Length:Absolute" => W::F(&mut p.len_abs, 0.0, 100.0, 2),
        "Spine:Length:+ % of parent" => W::F(&mut p.len_pct, 0.0, 1.5, 3),
        "Spine:Orientation:Start angle" => W::F(&mut p.start_angle, 0.0, 1.0, 3),
        "Spine:Shape:Gravity" => W::F(&mut p.gravity, -1.0, 1.0, 2),
        "Spine:Noise:Early:Amount" => W::F(&mut p.jink, 0.0, 1.0, 2),
        "Spine:Noise:Early:Threshold" => W::Inv(&mut p.jink_freq),
        "Spine:Noise:Late:Amount" => W::F(&mut p.noise_amt, 0.0, 3.0, 2),
        "Spine:Noise:Late:Turbulence" => W::F(&mut p.noise_turb, 0.0, 1.0, 2),
        "Skin:Radius:Absolute" => W::F(&mut p.radius_abs, 0.0, 10.0, 3),
        "Skin:Radius:+ % of parent" => W::F(&mut p.radius_pct, 0.0, 1.5, 3),
        "Skin:Radius:Clamp to parent" => W::B(&mut p.clamp_parent),
        "UV Mapping:U:Absolute" => W::F(&mut p.u_tile, 0.1, 8.0, 2),
        "UV Mapping:V:Scale" => W::F(&mut p.v_tile, 0.1, 8.0, 2),
        "Leaves:Size" => W::F(&mut p.leaf_size, 0.0, 10.0, 2),
        "Local Orientation:Sky influence" => W::F(&mut p.face_sky, 0.0, 1.0, 2),
        "Deformation:Fold" => W::F(&mut p.fold, -1.0, 1.0, 2),
        "Deformation:Curl" => W::F(&mut p.curl, -1.0, 1.0, 2),
        "Deformation:Twist" => W::F(&mut p.leaf_twist, -1.0, 1.0, 2),
        "Deformation:Scale:X" => W::F(&mut p.scale_x, 0.1, 3.0, 2),
        "Deformation:Scale:Y" => W::F(&mut p.scale_y, 0.1, 3.0, 2),
        _ => return None,
    })
}

/// Add `delta` to a property (by its original name): one node's variance (picture 35).
pub fn add_prop(p: &mut Params, name: &str, delta: f32) {
    match wire(p, name) {
        Some(W::F(v, ..)) => *v += delta,
        Some(W::I(v, ..)) => *v = (*v as f32 + delta).round() as i32,
        Some(W::Inv(v)) => *v = (*v - delta).clamp(0.0, 1.0),
        Some(W::B(_)) => {}
        None => {
            if let Some(d) = default_of(name) {
                *p.extra.entry(name.to_string()).or_insert(d) += delta as f64;
            }
        }
    }
}

/// The variance (+/-) of a tied spline property that TreeCraft keeps in its own field.
pub(crate) fn wire_var<'a>(p: &'a mut Params, name: &str) -> Option<&'a mut f32> {
    Some(match name {
        "Spine:Length:+ % of parent" => &mut p.len_var,
        "Spine:Orientation:Start angle" => &mut p.angle_var,
        "Skin:Radius:+ % of parent" => &mut p.radius_var,
        "Leaves:Size" => &mut p.leaf_size_var,
        _ => return None,
    })
}

/// The original's mode number for a generator.
fn orig_mode(p: &Params) -> usize {
    if let Some(m) = p.extra.get("Generation:Mode") {
        return *m as usize;
    }
    match p.gen_mode {
        MODE_ABSOLUTE => 3,
        MODE_PHYLLOTAXY => 5,
        MODE_INTERVAL => 6,
        _ => 0,
    }
}

/// Whether a Generation property shows for the current mode (the original shows only the
/// chosen mode's group; Frequency, Style, Orientation and Steps belong to Classic).
fn shown_for_mode(name: &str, mode: usize) -> bool {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.first() != Some(&"Generation") || parts.len() < 2 {
        return true;
    }
    let sub = parts[1];
    match sub {
        "Mode" => false, // drawn as the mode list
        "Style" | "Frequency" | "Orientation" | "Steps" => mode == 0,
        "Proportional" => mode == 1,
        "Proportional Steps" => mode == 2,
        "Absolute" => mode == 3,
        "Absolute Steps" => mode == 4,
        "Phyllotaxy" => mode == 5,
        "Interval" => mode == 6,
        "Bifurcation" => mode == 7,
        _ => true,
    }
}

fn is_int(name: &str, v: &str) -> bool {
    if v.contains('.') || v.contains('e') {
        return false;
    }
    let keys = ["Style", "Mode", "Number", "Count", "Limit", "Seed", "Accuracy", "Bones", "Anchor", "Group", "Source", "Mapping", "Alignment", "Steps", "Children", "Type", "Material", "offset", "Align"];
    keys.iter().any(|k| name.contains(k)) || v.parse::<i64>().map(|n| n.abs() >= 2).unwrap_or(false)
}

/// The groups of a property table, in order of first appearance.
fn groups(table: &[StProp]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = vec![];
    for (n, ..) in table {
        let g = n.split(':').next().unwrap_or("");
        if !out.contains(&g) {
            out.push(g);
        }
    }
    out
}

/// Draw the tab bar; returns the chosen tab.
pub fn tab_bar(r: &mut Rows, kind: Kind, tab: usize) -> usize {
    let names: Vec<&str> = if kind == Kind::Leaf { LEAF_TABS.iter().map(|t| t.0).collect() } else { BRANCH_TABS.iter().map(|t| t.0).collect() };
    let mut tab = tab.min(names.len() - 1);
    let (mut x, mut y) = (r.x - 4.0, r.y);
    for (i, n) in names.iter().enumerate() {
        let w = r.ui.text_w(n, 12.0) + 14.0;
        if x + w > r.x + r.w + 4.0 {
            x = r.x - 4.0;
            y += 24.0;
        }
        let b = macroquad::prelude::Rect::new(x, y, w, 22.0);
        if r.ui.button_ex(&format!("ptab.{}", n), b, n, i == tab, true) {
            tab = i;
        }
        x += w + 2.0;
    }
    r.y = y + 30.0;
    tab
}

/// The generator's properties for one tab. `extra_rows` draws TreeCraft's own settings that have
/// no property in the original (taper, flare, materials by name ...) at the end of their group.
pub fn draw(r: &mut Rows, p: &mut Params, kind: Kind, tab: usize, extra_rows: &mut dyn FnMut(&mut Rows, &mut Params, &str)) {
    let table: &[StProp] = if kind == Kind::Leaf { LEAF_PROPS } else { BRANCH_PROPS };
    let tabs: Vec<(&str, &str)> = if kind == Kind::Leaf { LEAF_TABS.to_vec() } else { BRANCH_TABS.to_vec() };
    let pattern = tabs.get(tab).map(|t| t.1).unwrap_or("");
    if tab == 0 {
        r.note("Faint names are kept with the generator but don't change the tree yet.");
    }
    let mode_now = orig_mode(p);
    // on the Material tab the Materials group comes first (picture 33)
    let mut gs = groups(table);
    if tabs.get(tab).map(|t| t.0) == Some("Material") {
        if let Some(i) = gs.iter().position(|g| *g == "Materials") {
            let g = gs.remove(i);
            gs.insert(0, g);
        }
    }
    for g in gs {
        // a group shows when any of its properties is on this tab
        if !table.iter().any(|t| t.0.split(':').next() == Some(g) && on_tab(t.0, pattern) && shown_for_mode(t.0, mode_now)) && !(g == "Generation" && on_tab("Generation:", pattern)) {
            continue;
        }
        if !r.group(g) {
            continue;
        }
        if g == "Generation" && on_tab("Generation:Mode", pattern) {
            // the original's ten modes; TreeCraft grows Classic, Absolute, Phyllotaxy and Interval
            let names: Vec<String> = ORIG_MODES.iter().map(|s| s.to_string()).collect();
            let mut m = orig_mode(p);
            let before = m;
            r.combo("orig_mode", "Mode", &names, &mut m);
            if m != before {
                p.gen_mode = match m {
                    3 | 4 => MODE_ABSOLUTE,
                    5 => MODE_PHYLLOTAXY,
                    6 => MODE_INTERVAL,
                    _ => MODE_CLASSIC,
                };
                if matches!(m, 0 | 3 | 5 | 6) {
                    p.extra.remove("Generation:Mode");
                } else {
                    p.extra.insert("Generation:Mode".into(), m as f64);
                    r.note("This mode isn't built yet: the generator grows as Classic.");
                }
            }
        }
        let mode = orig_mode(p);
        let mut last_sub = String::new();
        for (name, def, spline, _var) in table.iter().filter(|t| t.0.split(':').next() == Some(g)) {
            if !shown_for_mode(name, mode) || !on_tab(name, pattern) {
                continue;
            }
            // the Materials entries are drawn as the original's list instead (picture 25)
            if name.starts_with("Materials:Branch:") {
                continue;
            }
            let parts: Vec<&str> = name.split(':').collect();
            let label = parts.last().copied().unwrap_or(name).trim();
            // sections (picture 32): the first part below the group (U, V, Absolute ...) as a band,
            // deeper parts (Fractional, Twist ...) as indented headings under it
            let first = if parts.len() > 2 { parts[1].to_string() } else { String::new() };
            let rest = if parts.len() > 3 { parts[2..parts.len() - 1].join(" > ") } else { String::new() };
            let sub = format!("{}|{}", first, rest);
            if sub != last_sub {
                let (prev_first, prev_rest) = last_sub.split_once('|').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or_default();
                if first != prev_first && !first.is_empty() {
                    r.section(&first);
                }
                if !rest.is_empty() && (rest != prev_rest || first != prev_first) {
                    r.subheading_in(&rest);
                }
                last_sub = sub;
            }
            let key = name.replace([' ', ':', '+', '%'], "_");
            if *name == "Generation:Style" {
                // Absolute or Relative frequency (TreeCraft's Style)
                let styles: Vec<String> = STYLES.iter().map(|s| s.to_string()).collect();
                let mut st = if p.relative { 1 } else { 0 };
                r.combo(&key, label, &styles, &mut st);
                p.relative = st == 1;
                continue;
            }
            if *name == "Generation:Anchors" && kind == Kind::Leaf {
                let mut choices: Vec<String> = ANCHOR_CHOICES.iter().map(|s| s.to_string()).collect();
                for k in 6..=p.anchor_pick.max(5) {
                    choices.push(k.to_string());
                }
                r.combo(&key, label, &choices, &mut p.anchor_pick);
                continue;
            }
            // lists of choices (picture 32)
            if let Some(opts) = choices(name) {
                let names: Vec<String> = opts.iter().map(|s| s.to_string()).collect();
                let d: usize = def.parse().unwrap_or(0);
                let mut k = p.extra.get(*name).map(|v| *v as usize).unwrap_or(d).min(names.len() - 1);
                let before = k;
                r.combo(&key, label, &names, &mut k);
                if k != before {
                    p.extra.insert(name.to_string(), k as f64);
                }
                continue;
            }
            // kept in extra but used by the tree: drawn like any working property
            if let Some((lo, hi)) = used_extra(name) {
                let d: f64 = match *def {
                    "true" => 1.0,
                    "false" => 0.0,
                    s => s.parse().unwrap_or(0.0),
                };
                let cur = p.extra.get(*name).copied().unwrap_or(d);
                if *def == "true" || *def == "false" {
                    let mut b = cur > 0.5;
                    let before = b;
                    r.b(&key, label, &mut b);
                    if b != before {
                        p.extra.insert(name.to_string(), if b { 1.0 } else { 0.0 });
                    }
                } else {
                    let mut v = cur as f32;
                    let before = v;
                    // seeds (and the kill ratio) are whole numbers
                    let whole = name.contains("Seed") || name.contains("kill ratio") || name.contains("Accuracy");
                    r.with_curves(name, crate::stcurves::curve_kinds(name, *spline), crate::stcurves::has_variance(name, *spline), |r| r.f(&key, label, &mut v, lo, hi, if whole { 0 } else { 2 }));
                    if whole {
                        v = v.round();
                    }
                    if v != before {
                        p.extra.insert(name.to_string(), v as f64);
                    }
                }
                continue;
            }
            // every number gets the Parent / Distribution / Profile curve buttons (picture 24)
            let numeric = *def != "true" && *def != "false" && !matches!(wire(p, name), Some(W::B(_)));
            let mut body = |r: &mut Rows| match wire(p, name) {
                Some(W::F(v, lo, hi, dec)) => r.f(&key, label, v, lo, hi, dec),
                Some(W::I(v, lo, hi)) => r.i(&key, label, v, lo, hi),
                Some(W::B(v)) => r.b(&key, label, v),
                Some(W::Inv(v)) => {
                    let mut t = 1.0 - *v;
                    r.f(&key, label, &mut t, 0.0, 1.0, 2);
                    *v = 1.0 - t;
                }
                None => {
                    // kept with the generator, not used by the tree yet
                    let d: f64 = match *def {
                        "true" => 1.0,
                        "false" => 0.0,
                        s => s.parse().unwrap_or(0.0),
                    };
                    let cur = p.extra.get(*name).copied().unwrap_or(d);
                    if *def == "true" || *def == "false" {
                        let mut b = cur > 0.5;
                        if r.b_faint(&key, label, &mut b) {
                            p.extra.insert(name.to_string(), if b { 1.0 } else { 0.0 });
                        }
                    } else if is_int(name, def) {
                        let mut v = cur as i64;
                        let hi = if name.contains("Seed") || v.abs() > 100_000 { i32::MAX as i64 } else { (v.abs() * 4).max(10) };
                        let mut vi = v.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                        if r.i_faint(&key, label, &mut vi, (-hi).max(i32::MIN as i64) as i32 * (cur < 0.0) as i32, hi as i32) {
                            v = vi as i64;
                            p.extra.insert(name.to_string(), v as f64);
                        }
                    } else {
                        let mut v = cur as f32;
                        let hi = (d.abs() as f32 * 3.0).max(1.0);
                        let lo = if d < 0.0 { -hi } else { 0.0 };
                        if r.f_faint(&key, label, &mut v, lo, hi, 3) {
                            p.extra.insert(name.to_string(), v as f64);
                        }
                    }
                }
            };
            r.ui.unclamped = unlimited(name);
            if numeric {
                r.with_curves(name, crate::stcurves::curve_kinds(name, *spline), crate::stcurves::has_variance(name, *spline), &mut body);
            } else {
                body(r);
            }
            r.ui.unclamped = false;
            // (variance is the +/- button at the end of the row now, picture 35)
        }
        extra_rows(r, p, g);
    }
}
