# Handoff – TreeCraft (rebuild of a game tree modeler)

_Last updated 2026-11-11._

## Project
- Rust + macroquad desktop app in `app/`; program `target/debug/treecraft.exe`, window "TreeCraft".
- Original studied (Deep mode): `C:\Program Files\SpeedTree\SpeedTree Games Indie v8.4.2\win64\SpeedTree Modeler Games Indie.exe`.
- Values, rules and their function addresses: `MECHANICS.md` / `mechanics.json` (110 values: 94 sourced, 16 estimated). Original files used: `CREDITS.md`.

## Status
Builds with no errors and passes every Craft Anything check (build, screenshot, self-test, values sheet, credits). `cargo test` runs 6 unit tests (point insertion, anchor directions and numbers, painted colours, tessellation count in a concave outline, fold-line edge flip, LOD file search). The self-test doesn't open the Edit Cutout window.

## Done
- **Generation tab:** Tree/Trunk/Branch/Leaf node graph; Classic, Absolute and Interval placement from the original (Phyllotaxy approximate); Jink frequency (default 0.5). A Leaf generator under a Leaf generator spawns one leaf per green anchor of each parent leaf, filtered by its "Anchors" choice (All anchors, 1-5 or up to the highest number any mesh uses, matched to anchor numbers) and First/Last (anchor height), with the original's "Unable to find anchor" warning.
- **Properties panel (left):** shows the clicked node, material or mesh.
- **Viewport:** perspective/orthographic, orbit/pan/zoom/frame, grid, lighting.
- **Meshes tab** (pictures 1-3, 7): +/- Meshes window, empty new meshes, High/Med/Low files, "Embedded" label, Orient/Flip/winding processing from the original; wireframe preview with a Material switch and right-click Lit/Scribed.
- **Edit Cutout window** (pictures 9-13): embed warning; modal, movable/resizable (inside the app window), Confirm/Cancel only; starts with no points; new points join the closest point; outlines can't cross themselves (no masks); Transform group (Move/Rotate/Scale the whole card, with an on-canvas gizmo per tool; red/green handles limit Move/Scale to X/Y, picture 15); Pivot (white, orange angle line) and green anchors with angle lines and numbers (right-click; no upper limit, the list grows with the anchor count, picture 14); Color group paints vertex colours (picker, Reset asks first) that tint the leaves; Reset Points/Anchors/Color ask first; Options Tessellation (original's rule: t²×5000 points at largest-triangle circumcentres, meshes cached in `meshio.rs`), smooth Angle wheel (original's 0.005 rad step, no snapping), Double-sided, Improve folding (original's edge flips along the fold line); each level keeps its own outline, colours and anchors, carried by the Load/Save arrows and shown in the level previews. Canvas follows the original's mouse code (fn 0x140300880): middle-drag pans, middle+left drag zooms (Fit button resets), original grab radii, 20 px stroke brush, Reset Color fills with the swatch colour.
- **Materials tab** (pictures 4-6, 8): picker (hides sets), +/- window, + from texture files, copy/paste, Settings/Meshes sub-tabs, ten texture slots, original's defaults, Make New Set...; preview shapes Sphere/Plane/Cylinder/Custom, turns only when dragged, wheel zoom.
- **New tree:** starts with no materials, textures or meshes (the start-up demo tree has Bark, Leaf and Leaf cutout).

## Outstanding
1. Only the Color, Opacity and Normal slot pictures affect rendering.
2. Material sets draw as their first member; the Material Sets panel (members, weights) isn't built.
3. Branch bending (gravity, noise, jink) and radius taper are estimated (unknown unk_cb364103d504cff6). Ruled out: fns 0x140192780, 0x140196510 (old-file converters), 0x1403154a0 (image processing).
4. Inferred, not confirmed: that the anchor value First/Last compare is its height, "Edit..." on an empty material mesh row. Not used: the anchor "side" value the original stores. The tree spawns on High's anchors (or the first lower level that has some), not on the level being shown.
5. Not built: Cull backfaces / Show unique uvs in the mesh preview menu.
6. Stored but inactive: mesh Ripple, Proxy collision, Collection "Fronds"; material Season, Atlas, Max resolution, Unwrap, User data.
7. Not built: Displacements tab, Node/Freehand edit modes, six generation modes (Proportional, Proportional steps, Absolute steps, Bifurcation, Flood, Parent), property curves, save/load/export, wind. The Edit Cutout window can't move to another monitor (single OS window).

## Important files (`app/src/`)
- `app.rs`: app state, dock tabs, viewport, Meshes tab, node graph rules
- `cutout.rs`: Edit Cutout window (+ unit tests)
- `matpanel.rs`: Materials tab
- `meshdialog.rs`: shared add/remove window for meshes and materials
- `props.rs`: Properties panel
- `gen.rs`: tree growth, anchor spawning, vertex-colour tint
- `model.rs`: data types and defaults (levels hold anchors and colours)
- `render.rs`: shader, camera, preview shapes
- `textures.rs`: picture loading, adjustments, auto-cutout
- `meshio.rs`: OBJ files, cutout geometry (with colours), anchor points, mesh processing
- `ui.rs`: UI toolkit
- `main.rs`: menus, keys, `--screenshot`, `--self-test`

## Next tasks
1. Use the remaining slot pictures (Gloss, Specular, Metallic, Subsurface, AO) in the shader (`render.rs`).
2. Build the Material Sets panel (original fn 0x1400757b0).
3. Save/load trees (including per-level anchors and colours), then export the whole tree (OBJ/FBX).
4. Property curves (Frequency, Length, Radius along the parent).
5. Find the original's branch-bending code: look for code reading the Spine:Shape:Gravity property (index 130) from the branch node; replace the estimates in `gen.rs`.
6. Add Edit Cutout and anchor-spawning steps to the self-test.

## Useful tricks
- Screenshot start states: `TM_TAB=0..2`, `TM_SEL=<node name>|mat:N|mesh:N`, `TM_MATTAB=1`, `TM_DIALOG=1|mat|cut` (+ `TM_CUTTOOL=move|rotate|scale`), `TM_NEW=1`.
- UI scaling: `ui_scale()` / `ui_factor()` in `main.rs`.
- Real defaults live in the install's gzip XML files: `app_data\sketch\trunk.stt` / `branch.stt` and the `.spm` trees.
- Qt slot dispatchers: read the class's meta-object (stringdata lists slots in case order; extradata points at the dispatcher). Virtual methods: find the class's RTTI type descriptor, its complete-object locator, then the vtable (e.g. CLeafMeshNode vtable 0x141064e68).
- If an open TreeCraft window locks `target/debug/treecraft.exe`, rename the file and rebuild; the `treecraft-running-old*.exe` files in `target/debug` are such leftovers and can be removed once no copy is running.
- Ghidra fails on very large functions (e.g. fn 0x1400b19b0); use smaller callees.
- PowerShell: use absolute paths with `[IO.File]`; long replacement scripts can trip the safety check, so prefer the Edit tool.
