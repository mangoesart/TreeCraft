# TreeCraft – how it works and where the numbers come from

This is a tool, not a game: the "mechanics" are the generator rules and default values.
Sources: the original's node templates (gzip XML in `templates\combined\*.stt`), the Blank
tree template (`tree_templates\Games\Blank.spm`), the original's style sheet
(`app_data\style.css`) and its code (function 0x1403175a0 lists the generation modes).
Anything marked *estimated* was looked for and not found, and was chosen by eye.

## Window layout (from a picture of the original running, checks/original/start.png)
- Menu bar: File, Edit, View, Window, Help.
- Properties panel on the left: header with the selection's name and the Generator / Node / Freehand
  mode buttons (only Generator works in this version), then collapsible groups of properties.
- Viewport in the middle (perspective by default; Front/Back/Left/Right/Top/Bottom views are orthographic).
- Dock on the right with tabs Generation (node graph), Materials, Meshes, and a toolbar
  (Add, Actions, Randomize). The original also has a Displacements tab: not in this version.
- Colours: #292929 background, #3b3b3b panels, #474747/#4f4f4f buttons, #5875b9 accent, #da9747 highlight (style.css).
- Font: Inter (the original's own font, app_data\fonts).

## Generators (nodes)
| Node | Default | Source |
|---|---|---|
| Tree | Shape radius 10 | Blank.spm `Shape:Radius` |
| Trunk | Mode Absolute, number 1 | Trunk.stt `Generation:Mode=3`, `Absolute:Number=1` |
| Trunk | Length 25, start angle 0.5 (straight up), radius 1 | Trunk.stt `Spine:Length:Absolute`, `Spine:Orientation:Start angle`, `Skin:Radius:Absolute` |
| Trunk | Noise amount 0.35, turbulence 0.5 | Trunk.stt `Spine:Noise:Late:*` |
| Trunk | Tip radius 0.18, base flare 0.5, 14 × 10 segments | estimated |
| Branch | First 0.215, last 0.925 | Branch.stt `Generation:First/Last` |
| Branch | Length 40 % of parent (variance 0.13 → 0.33 relative) | Branch.stt `Spine:Length:+ % of parent` |
| Branch | Start angle 0.35 (variance 0.038 → 0.11 relative) | Branch.stt `Spine:Orientation:Start angle` |
| Branch | Radius 66 % of parent (variance 0.05 → 0.08 relative) | Branch.stt `Skin:Radius:+ % of parent` |
| Branch | Noise 0.7, turbulence 0.5 | Branch.stt `Spine:Noise:Late:*` |
| Branch | Interval count 4, spread 0.5; Balanced on, Offset 0, Sweep 1, Roll 0 | Branch.stt `Generation:Interval:*`, `Generation:Orientation:*` |
| Branch | Frequency 14, gravity 0.12, shorter-toward-end 0.55, tip radius 0.12 | estimated |
| Leaf | Size 1 in the template; 1.4 here, first 0 → 0.1, last 1 | Leaf.stt `Leaves:Size`, `Generation:First/Last` (size/first adjusted, estimated) |
| Leaf | Frequency 26, face sky 0.45, droop 0.15, fold 0.15, curl 0.1 | estimated |

Generation modes in the original (0x1403175a0): Classic, Proportional, Proportional steps, Absolute,
Absolute steps, Phyllotaxy, Interval, Bifurcation, Flood, Parent. This version has Classic
(count per parent), Absolute (exact count), Interval (spacing along the parent) and Phyllotaxy
(nodes with N children each).

## Placement rules read from the original's code (Ghidra pseudo-code)
Constants were read directly: 1.0 at 0x141f409b4, 0.5 at 0x141f407a8, 0.025 at 0x141f4040c,
10.0 at 0x141f40d74, 0.1 at 0x141f404b4, 0.25 at 0x141f405e0. Roll is measured in turns (1.0 = 360°).
Random numbers come from a 10000-entry table indexed by seeds that wrap at 9999 (0x141eb5fa0);
this build uses its own random generator instead (same kind of result, different exact trees).

- **Classic** (fn 0x140477e40, properties wired in fn 0x1404766f0):
  - count = whole part of Frequency (Style Absolute), or parent length × Frequency × 0.025
    (Style Relative). Fractions are dropped, not rounded.
  - each child sits at a random spot between First and Last, drawn from the frequency curve
    (fn 0x1402cad40; a flat curve gives evenly random spots).
  - roll (turns) = (Balanced ? i / n : random 0..1) + random(−Offset, +Offset) × 0.5, all × Sweep,
    then + Roll. With "Centered align" the rolls are shifted by −(max+min)/2.
- **Absolute** (fn 0x1404723f0): count = Number; roll = (i / n + random(−Jumble, +Jumble) × 0.5) × Sweep,
  centred on zero, then + Roll; positions random between First and Last.
- **Interval** (fn 0x14047b6f0): start at Last and step down to First by (10 / Frequency) length
  units; each node gets Count children 1/Count turn apart (½ turn when Count < 3), every node adds
  Spiral × 0.1 turns, Spread jitters children along the parent, Limit caps the node count.
- Switching to Interval converts Frequency: new = max(0, (int)(old + 0.5) − 1) or old × 0.9 (fn 0x1404799a0).
- Phyllotaxy here reuses the Interval walk with "Internode length" in units (the original's
  phyllotaxy routine wasn't read; estimated).

## Old-file conversion of spine settings (fn 0x140192780)
When the original opens an older tree file it maps the old spine settings onto the new ones:
- "Spine:Jink amount" -> Spine:Noise:Early:Amount; "Spine:Jink frequency" -> Spine:Noise:Early:Threshold = 1.0 - frequency (constant 0x141543260 = 1.0). So Jink is the Early noise, and its frequency is 1 - Threshold.
- "Spine:Disturbance" -> Spine:Noise:Late:Amount, then multiplied by 1.5 (constant 0x141040de8). Without Perturbance both noise amounts are set to 0.
- Length (Absolute or % of parent by "Length style"), Start angle (made non-negative), Alignment, Roll and Break chance/spot copy across to their new names.
- Defaults in the install's own app_data/sketch trunk.stt and branch.stt: Early Threshold 0.5, Early Amount 0. The build's new "Jink frequency" (share of segments that jink) therefore defaults to 1 - 0.5 = 0.5 (it was a fixed 0.25 before); Jink amount stays 0.
- The spine-walk itself (how Gravity, Noise and Jink bend each step) is still not found: this function only renames settings.
- fn 0x140196510 is the matching converter for older Branch files (Texture Coordinates -> UV Mapping, Segments lengths, Branch:Flares -> Displacement:Flares, Branch:Intersection -> Skin:Subdivision spreads); it doesn't bend spines either. fn 0x1403154a0 (once thought to evaluate children) is an image-processing worker run in parallel by fn 0x1403157a0, so the spine builder is still to be found.
## Property limits read from the original's property tables
Spine table fn 0x14053b8c0, skin table fn 0x140540c10 (each entry: type, suggested slider top,
hard min, hard max). Property numbers: Length:Absolute = 3, Start angle = 4, Gravity = 130, Jink:Amount = 180.
| Property | Range in original | Suggested slider top |
|---|---|---|
| Spine:Length:Absolute | 0 … ∞ | 20 |
| Spine:Length:+ % of parent | 0 … ∞ | 0.5 |
| Spine:Orientation:Start angle | 0 … 1 | — |
| Spine:Orientation:Roll, Spine:Shape:Ancestor | −1 … 1 | — |
| Spine:Shape:Gravity, Spine:Shape:Twist | unbounded | — |
| Spine:Shape:Straightness, Spine:Jink:Amount, Jink:Uniformity | 0 … 1 | uniformity 0.1 |
| Spine:Jink:+ Relative | 0 … ∞ | 0.1 |
| Segments:Radial:Absolute | 0 … 100000 (whole) | 3 |
| Skin:Radius:+ % of parent | 0 … ∞ | 0.75 |
| Skin:Radius:Length compensation | 0 … ∞ | 1 |
| Skin:Radius:Length kill ratio | 0 … ∞ | 500 |
| Skin:Roll | −1 … 1 | — |
| Skin:Smooth, Skin:Squash:Amount | 0 … 1 | — |

## Rules this build adds itself (not found in the code yet; estimated)
- The spine walk (how gravity, noise and jink bend a branch) and the radius taper: the routine that
  applies them wasn't located (it reads properties through cached pointers, not by name). Gravity
  bends the spine down by gravity × 2.5 / segments per step; noise is a smooth wobble; both estimated.
- Start angle 0..1 maps to 0..180° from the parent's direction (0.5 = at right angles). For trunks it is
  measured up from the ground, so 0.5 is vertical.
- Length = absolute + % of parent length, times (1 − shorter-toward-end × position), times variance.
- Radius = absolute + % of parent radius at the spot, clamped to the parent; tapers to "tip radius"
  and swells by "base flare" near the base.
- Gravity bends the spine down (negative bends up); noise adds smooth wobble; jink adds sudden kinks.
- Leaves use the chosen mesh outline (Meshes tab) and material; fold bends the two halves along the
  midrib, curl bends along the length, face sky turns leaves toward the sky.

## Materials (channels from the original's .spm files)
Color, Opacity, Normal, Gloss, Specular, Metallic, Subsurface color/amount, AO, Variation (3 colours,
defaults 0.28/0.37/0.00, 0.59/0.72/0.32, 0.46/0.55/0.20 from Broadleaf.spm), Two sided, Cutout mesh.
Height, Custom and Custom2 channels are not in this version. Default alpha cutoff 0.33 (estimated).

### Materials panel (fn 0x140063f80, pictures of the original)
Top row: drag handle, picker ("No materials" when empty), +/- ("Manage the materials." opens the Materials window: Add new, Duplicate, Rename, up/down, Remove, OK/Cancel), + (browse for textures, one new material each), copy/paste menu. Settings sub-tab: Main (Two-sided "Face culling.", Season colour, Make New Set...), Variations (3 colours, Amount), Atlas (Scale, Atlas: Default/Force in/Force out), Other (Max resolution: Default/Original/4096/2048/1024/512/256, User data). Meshes sub-tab: Unwrap Share/Separate, Add, rows of mesh picker + Edit... + go-to arrow + remove. Ten texture slots: Color, Opacity, Normal, Gloss, Specular, Metallic, Subsurface, Subsurface%, AO, Custom; defaults seen in the original: Opacity 1, Normal 1, Gloss 0.25, AO 1, Subsurface white, Custom black.

### Material defaults and sets (read from the original's code)
- New material (fn 0x1400a7390, the material constructor): ten slots of 0x78 bytes from +0x160. Color, Opacity, Normal, Subsurface = 1,1,1 (constant 0x141796e08); Gloss 0.25; Specular 0.75,0.75,0.75; AO 1,1,1; Custom black; Metallic and Subsurface% 0. Colour slots (flag 2): Color, Specular, Subsurface, Custom. Variation colours 0.28/0.37/0.00, 0.59/0.72/0.32, 0.46/0.55/0.46.
- Make New Set... (fn 0x14006d8c0, undo label "add material set"): name = current name + " Set" (fn 0x140056690), made unique and appended with a fresh id (fn 0x1400af740); the new material gets the set flag (+0x6cc = 1) and one member: the current material's id with weight 1.0 (constant 0x1415432f4). The build draws a set with its first member's look and lists the members on the preview; the original's separate Material Sets panel (fn 0x1400757b0) for editing members and weights is not built yet.

### Materials panel refresh rules (fn 0x140068ed0)
- The material picker lists only materials that are not sets (+0x6cc == 0); with none it shows "No materials" and disables the panel. A new set therefore doesn't appear in the picker (the build keeps showing the current material and says the set was made).
- Meshes sub-tab: rows = supplemental meshes (+0x130 list) + 1. Row 0 is the material's cutout mesh (+0x128, slot "CutoutMeshChanged"); its delete button is disabled with no text, every other row's says "X" ("DeleteSupplementalMesh"). Each row: mesh combo ("None", then every mesh), "Edit..." ("GenerateMesh", called 'Generate mesh' in the help text), go-to arrow (goto.png, "GoToMesh"), delete. Add appends a supplemental row. What GenerateMesh does with an empty row is estimated (the build makes a new cutout mesh named after the material and opens the cutout editor).
- Variation Amount slider position = Amount² × 10000 (constant 0x141031c10), so the slider moves on a squared scale.
- Two-sided reads +0x2c; Atlas Scale +0x58 (default 1.0 from fn 0x1400a7390); User data +0x60.

### Preview widget (shared by the Materials and Meshes panels)
- Setting a material (fn 0x1402ec790) sets mode 0 and the view angles to -pi/2 and -1.728 rad; setting a mesh (fn 0x1402ec7e0) sets mode 1 and an identity orientation. Nothing in these or the input handlers turns the view over time: it moves only with the mouse (the build's preview no longer spins on its own).
- Reset view (fn 0x1402ecdd0, double-click in the build): orientation back to its start and zoom to 0.
- Mouse wheel (fn 0x1402ece70): zoom += wheel delta x -0.001 (constant 0x14107b4bc); one notch (120) = 0.12. Image modes 2-4 also scale by it, never below 1.0.
- Right-click on a mesh preview (fn 0x1402ecb70, mode 1 only) opens a menu of check items: Lit, Cull backfaces, Scribed (edges drawn over the shaded mesh), Show unique uvs. The build has Lit and Scribed in that menu, and keeps the person's Wireframe (default) / Material switch; Cull backfaces and Show unique uvs aren't built.
- The material preview's shapes (Sphere, Plane, Cylinder, Custom) are the person's request, not read from the original.
## Meshes (Meshes tab, matching the original's Meshes panel)

### Edit Cutout window (person's picture-9 plus code)
- Edit Mesh... (fn 0x1400828d0): if the mesh isn't embedded (+0x8e == 0) it asks "Editing this mesh will convert it to embedded. It will no longer reference files on disk. Do you wish to continue?" (Yes/No). No cancels; Yes marks it embedded ("embed mesh") and opens the editor. An embedded mesh opens straight away.
- Loading a file into an embedded mesh asks "Loading a file will delete this asset's embedded data. Do you wish to continue?" (fn 0x1400805d0); the build then clears the embedded levels. Every level of an embedded mesh is labelled "Embedded" (fn 0x14007e690).
- Window title "Edit Cutout - <mesh>" (fn 0x1402fc9c0). Parts and tooltips (fn 0x1402f32a0): Points (Add/Move points, Remove points, Reset points), Anchors (three buttons), Mask (Paint mask to remove triangles, Erase mask, Reset mask), Color (Paint mask to remove triangles, colour, Reset mask), Material (combo, Show color, Show opacity, Show all meshes), Options (Tessellation, Angle, Double-sided, Improve folding), High/Med/Low each with two arrow buttons and "v: f:".
- The build leaves out Mask and Color at the person's request; instead an outline can't be made to cross itself, so concave shapes triangulate without overlap. Confirm / Cancel (the person's request) are the only way to close the window.
- Controls (fns 0x1402f4120, 0x1402f83f0, class COptimalMeshDialog): Tessellation is a QSlider 0..10000, page step 100 (0..1 in the build); Angle is a CThumbWheel (fn 0x140409be0; the window sets its step to 2) sending AngleChanged; Double-sided and Improve folding are check boxes. Per level, the first arrow is LoadButton and the second SaveButton: load = edit that level in the window, save = put the window's shape into that level (as built). The canvas and the level column start at 600:100 in a splitter. The (unbuilt) mask colour defaults to red (QColor 7).
- At the person's request (picture-11): a new point is inserted next to the closest existing point (on the side that keeps the outline simple and adds the shorter edge). The first anchor is the Pivot (white, orange angle line: where the leaf attaches and grows). Other anchors are green, each with its own angle line (degrees clockwise from up in the picture). A Leaf generator placed under a Leaf generator spawns one leaf on each green anchor of each parent leaf, pointing along that anchor's line (its Start angle starts at 0; Roll turns it around the line; size, droop, twist etc. come from its own properties). The anchors go through the same Pivot angle, Orient, Flip, Scale, leaf size and fold as the mesh (curl is applied to position only). This follows the person's design, and the original confirms the shape of it: when a generator's parent is a mesh with anchors, the placement routine (fn 0x1402456e0) skips the generation modes and asks the parent mesh to place children on its anchors (filtered by the generator's First/Last and its "Generation:Anchors" choice); if none were placed it reports "Unable to find anchor for node (check mesh anchors (including LOD meshes), first/last, and anchor restrictions)". "Generation:Anchors" (fn 0x14024b880, next to Generation:Mode) offers All anchors, 1, 2, 3, 4, 5. The build has the Anchors choice on Leaf generators and shows that message in the status bar.
- The parent leaf mesh's placement (CLeafMeshNode vtable 0x141064e68, slot +0x4f8 = fn 0x14026f500) walks the mesh's anchors and keeps one when the restriction is 0 or equals the anchor's id, and when First <= v <= Last, where v is the anchor record's second number (read as its height along the leaf); the child's position is (v - First) / (Last - First) clamped to 0..1 (v itself when First = Last), and its Roll and other values are read there. The build filters anchors the same way using the anchor's height along the pivot's growth line in picture units (0 at the pivot, about 1 at the top of a full-height card). A Leaf generator added under a Leaf generator starts with First 0 and Last 1 so every anchor is used.
- At the person's request (pictures 12-13): each level (High/Med/Low) keeps its own green anchors (position, angle, number) and point colours, as the original keeps anchors per level ("check mesh anchors (including LOD meshes)"); the -> arrow (SaveButton) copies the canvas's outline, colours and anchors into a level, <- (LoadButton) loads them, and each level preview draws its anchors in green with an "A:" count. The tree uses High's anchors (or the first level below that has some). Right-clicking a green anchor picks its number 1-5 (shown under it); the Anchors choice of generators matches that number (the original matches its anchor id, fn 0x14026f500). A Transform group moves, rotates or scales the whole card (points, pivot and anchors; rotation turns the angles too). The Color group (the original's place for it; swatch red by default, fn 0x1402f83f0) paints vertex colours on points with a brush; new vertices take the average of their source points; the colours multiply the leaf's colour on the tree. In the original the Color group's tooltip is "Paint mask to remove triangles", so painting colours is the person's design. Its buttons follow the original: ModeColor (fn 0x1402f7f90) puts the canvas in colour mode (6) and un-checks the other tools; ColorClicked (fn 0x1402f3f90) opens a colour picker beside the swatch (the build's picker has R, G, B and quick colours); ResetColor (fn 0x1402f8040) asks "Are you sure you wish to reset the color? This cannot be undone" (title "Reset Color"), then sets every colour to the canvas's current paint colour (fn 0x1402fd070 copies +0x124, the value the brush paints with) and returns to colour mode.
- Canvas drawing (COptimalMeshWidget paint, fn 0x1402fe130, from the vtable 0x14107cbd8): a checkerboard of 7 px squares (constant 0x14107cab0) drawn as GL quads, then the material picture; the mesh triangles drawn as GL_TRIANGLES in line mode; the mask layer (a sqrt(n) x sqrt(n) texture, masked cells transparent) always; the painted-colour layer (its own RGBA texture) only while the canvas is in colour mode (mode 6); the pivot's angle line, the anchors with their numbers, and the cursor ring in the paint colour (bytes / 255, constant 0x1415433b4) in mode 6, red / yellow-ish / green in the mask and remove modes. The build uses the 7 px checkerboard and shows painted colours only in Color mode (on the triangles, blended between points, since the build stores colours per point). Show color / Show opacity (ColorTypeChanged, fn 0x1402f7620) switch the canvas picture between the material's colour and opacity maps; Show all meshes passes the other meshes' index (-1 when off).
- Render modes (read from the install's render_modes/*.fx files, each a RenderMode() function; the build's shader branches in render.rs follow them): Albedo Check: luminance in sRGB, range (0.05, 0.9) blended to (0.25, 1.0) by metallic, f = (saturate((lum - lo) / (hi - lo)) - 0.5) x 2, cubed; f < 0 blends toward blue by -f^3 of that, otherwise toward red by f x (1 - metallic); background black. Subsurface Check: the same on subsurface colour x amount with range (0.05, 0.9) and f scaled by the amount. AO: the AO value (background white). Scribed / Standard view: white, blended to (0.8, 0.7, 0.7) where see-through, x lerp(1, 0.8, 1 - (N.V)^2) (Scribed also draws the triangle edges). Lighting > Untextured: colour, specular and subsurface colours 0.5 in the full lighting; Diffuse, Specular and Subsurface parts x half AO = (1 + AO) / 2; Ambient x AO; Shadow = the shadow term (the build has no shadow maps: 1). Material > Color, Opacity (white at its opacity on black), Gloss, Normal (screen-space normal x 0.5 + 0.5), Specular colour, Metallic, Subsurface colour, Subsurface Amount, Custom. Vertex Colors > Color; Red (r, 0, 0), Green (0, g, 0), Blue (0, 0, b). Other > Phong: shininess s = (1 / (1 - gloss)^2 - 1) x 2, specular = saturate(R.V^s x (s + 1) / 3pi); Blinn-Phong: s = (1 / (1 - gloss)^2 - 1) x 8, specular = N.H^s x (s + 4) / 12pi; Oren-Nayar: rough = 1 - gloss, A = 1 - 0.5 r^2 / (r^2 + 0.33), B = 0.45 r^2 / (r^2 + 0.09) x s x t with s = V.L - N.L x (|N.V| + 0.1), t = lerp(N.L, min(1, N.L / (|N.V| + 0.1)), step(0, s)); Lambert: ambient (k 0.5) + colour x N.L; Flat: Blinn-Phong on a faceted normal from screen derivatives (the build uses the smooth normal); Billboard Subsurface Amount: black at opacity 1 - sqrt(amount) x AO over white; Depth: the screen depth; Overdraw: red at opacity 0.05 per layer, no depth test (background green); Texcoord: frac(uv); Texcoord Seams: saturate((sin(u, v, u + v) x 0.5 + 0.5)^2). Lightmap colours lightmap texel density (ideal 0.015, max 0.5) from lightmap UVs, which the build doesn't have: it shows lighting on white.
- Toolbar functions built at the person's request (picture 19; estimated behaviour, the original's own code for these wasn't read): Show: Bones (every spine, over the bark), Normals, Disable two sided rendering (back faces discarded); Fronds/Zones/Targets/Proxies toggle but TreeCraft grows none. Scene objects: Direction and Magnet forces bend every branch while it grows like Gravity does (strength 1 ~ Gravity 1; Magnet weakens as 1 / (1 + distance x 0.1)); Sphere and Capsule collision objects are drawn only (the original's help: primitives for collision at run time). Gizmo (Translate / Rotate / Scale): pressing a shown scene object in the viewport picks it; dragging moves it in the view plane, turns its direction, or scales its strength/size by exp(-dy x 0.01). Wind sways the tree by height and flutters leaves (Calm 0.25, Breeze 0.5, Strong 1, Gale 2). Season 0..1 tints leaf materials (cutout materials): fresh green before 0.25, unchanged to 0.5, toward orange to 0.85, then brown. AO (O) applies the per-vertex occlusion fully. Collision (leaf): a leaf whose centre comes within 0.35 / 0.6 / 0.85 of its size (Low / Medium / High) of a leaf already placed is left out. Export writes an OBJ (v, vt, vn, one group per material). Nodes (per-node offsets) isn't built. The toolbar buttons are rounded with drawn icons.
- Viewport toolbar (picture 18; fn 0x1405325b0 builds it, fn 0x140537430 the tree window's menus, fn 0x140531610 the button names): groups View (Render, Show, Zoom), Edit (Generators / Nodes, Add, Visibility, Season, Gizmo), Post (AO, Collision, Export) and Scene (Forces, Collision Objects, Wind, Light), plus a button that collapses the toolbar (preferences/m_bCompressedToolbar); the build orders the groups View, Scene, Edit, Post as in the person's picture and uses the original's help texts as tooltips. Show: Leaves 1, Fronds 2, Branches 3, Zones 4, Proxies 9, Targets 5 (fn 0x1405325b0), Bones, Normals, Disable two sided rendering (fn 0x140537430). Zoom: All, All (all views), Selected, Selected (all views), Copy camera, Paste camera, Reset camera. Visibility: Toggle selected (H), Toggle isolation (I), Show all. Manipulator: Translate, Scale, Rotate. Add collision object: Sphere, Capsule. Leaf Collision quality: High, Medium, Low, None. Render lists the install's render_modes folder by the template naming rule: Standard, Scribed, AO, Albedo Check, Subsurface Check, Lighting (Untextured, Diffuse, Specular, Ambient, Subsurface, Shadow), Lightmap, Material (Color, Opacity, Gloss, Normal, Specular, Metallic, Subsurface, Subsurface Amount, Custom), Vertex Colors (Color, Red, Green, Blue), Other (Phong, Blinn-Phong, Oren-Nayar, Lambert, Flat, Billboard Subsurface Amount, Depth, Overdraw, Texcoord, Texcoord Seams). The original's viewport menu "Viewport" holds Perspective, XY/XZ/YZ Plane and Demo camera. Working in the build: Render Standard / Other > Flat (unlit), Show Leaves / Branches / Grid, Zoom (Selected frames the whole tree), Light (Lit, Properties...), Add (the Add menu), Visibility; the rest say they aren't built yet. Frame, Grid and Lighting left the viewport bar; Orthographic is in the view menu below the toolbar (the person's request). Hot keys as the original labels them (menu texts "Leaves\t1", "Fronds\t2", "Branches\t3", "Zones\t4", "Targets\t5", "Proxies\t9" in fn 0x1405325b0; "Toggle selected\tH", "Toggle isolation\tI" in fn 0x140537430; tooltips "Hot key: Z" for Zoom, "6 (toggles forces in the viewport)", "7 (toggles collision objects in the viewport)", "8 (toggles wind)", "Tab" for Generators/Nodes, "Alt+S" for Season, "O" for AO): the build's number-row keys 1-5 and 9, 6, 7, 8 and H, I, Z do the same; its camera views moved to the number pad (0/1/3/7, 5 ortho).
- Template values (read from the install's templates/*.stt, gzip XML; kept in templates.rs). Generation:Mode numbers: 0 Classic, 1 Proportional, 2 Proportional steps, 3 Absolute, 4 Absolute steps, 5 Phyllotaxy, 6 Interval, 7 Bifurcation (fn 0x1403175a0 order). Trunk: Absolute 1, length 25, angle 0.5, noise 0.35, radius 1. Trunk Tube: length 25, noise 0, radius 0.5. Bifurcating Branches: mode 7, first 0.2, last 0.9, length 100 % of parent, angle 0.39343, noise 0.726, radius 97 %. Big Branches: Interval (frequency 2, count 4), first 0.214698, last 0.925, length 40 %, angle 0.35, noise 0.7, radius 66.17 %. Branch Tubes: Interval 2/3, 0.2-0.9, 75 %, angle 0.5, noise 0, radius 75 %. Little Branches: Interval 2/3, 0.214698-0.9, 35 %, angle 0.339982, noise 0.55, radius 50 %. Single Branch: Absolute 1, 0-1, length 1 + 50 %, angle 0.25, noise 0.5, radius 75 %. Twigs: Interval 2/3, 0.214698-0.9, 22.71 %, angle 0.339982, noise 0.5, radius 100 %. Leaf Mesh: Absolute 1, size 1. Roots: Roots (Interval, first 0.005, last 0.03, 28.71 %, angle 0.897, noise 1.13455, radius 45 %) + Root Twigs. Peeling Bark: Fin, Absolute 100. Shell: length 25, radius 0.5, noise 0.34. Templates holding several generators: Roots, Palm Frond (Frond + Leaflets), Click Placed Branches / Custom Mesh, Place a spine in a mesh, Place 3D Anchors (with Targets). Estimated stand-ins (TreeCraft has no such generator types): Cap = a short full-radius piece at the parent's tip tapering to 0; Fronds = classic leaves; Knot / Lump = short stubs (radius 45 % / 70 %); Cavity / Gash = a card on the bark; Peeling Bark = 100 small cards; Targets = Magnet forces; Stitch / extensions = a piece continuing the parent from its tip; Bifurcation = 2 children per parent; Zone = a marker sphere; Terrain = the ground grid. From file... reads any .stt with the same rules.
- Property tabs (read): each tab is a label and name patterns; a property shows when its name contains one of them. Branch generators (fn 0x140136e00): Gen "Generation:" (only when the program's level setting is below 4), Spine "Spine:|Forces:", Shape "Shape Control:", Skin "Skin:", UV "UV Mapping|Lightmap", Displacement "Displacement", Material "Materials:|Season:|Vertex Color:|Lightmap", Animation "Wind:|Growth:", Segments "Segments:", LOD "Dynamic LOD:", Hand Drawn "Hand Drawn:". Leaf mesh generators (fn 0x140184db0): Gen, Skin "Leaves:|Lighting:|Deformation:", Orientation "Orientation:|Sensitivity:|Adjustments:|Deformation:", Lighting "Lighting:|Material:|Lightmap", Material "Season|Leaves:Type|Vertex Color:", LOD "Dynamic LOD:", Animation "Wind|Growth", Collision "Collision:". Random Seeds, Physics and Decorator appear only under All. The build uses these exact tabs and patterns.
- Properties panel for generators (picture 21): the original's tabs and every property the original's branch and leaf mesh generators have, in its order, with its defaults: 256 branch properties (groups Generation, Forces, Hand Drawn, Random Seeds, Segments, Spine, Shape Control, Skin, UV Mapping, Materials, Displacement, Dynamic LOD, Wind, Season, Growth, Physics, Vertex Color, Lightmap, Decorator) and 175 leaf mesh properties (Generation, Forces, Random Seeds, Leaves, Local Orientation, Sensitivity, Final Adjustments, Deformation, Lighting, Wind, Growth, Season, Dynamic LOD, Vertex Color, Collision, Lightmap), read from templates/__Big_Branches.stt and _Leaf_Mesh.stt (stprops.rs; curve/list values left out). The Generation group shows only the chosen mode's sub-group, as the original does (Frequency, Style, Orientation, Steps for Classic). Properties TreeCraft's generator uses are tied to it (Frequency, Absolute Number/Jumble/Sweep/Roll, Interval Frequency/Count/Spread/Spiral/Limit/Balance, Phyllotaxy internode length/children/spread/spiral/limit, First, Last, Size scalar, Random Seeds:Generation, Segments:Radial:Absolute, Spine Length absolute/% of parent (+ variance), Start angle (+ variance), Gravity, Noise Early amount (jink) and threshold (= 1 - jink frequency), Late amount/turbulence, Skin Radius absolute/% of parent (+ variance)/clamp, UV U absolute, V scale, Leaves:Size (+ variance), Sky influence, Deformation Fold/Curl/Twist/Scale X/Y); the others are kept with the generator (and in saved files) with faint names until built.
- File > Open / Save / Save As (Ctrl+O, Ctrl+S, Ctrl+Shift+S): TreeCraft's own text format (.tctree) with every generator setting and kept original property, materials (settings and picture files), leaf meshes (levels, cutouts, anchors, colours, inner points, geometry) and scene objects.
- Preferences (picture 21): pages Themes, Viewport, Lighting, Wind, Global (the original's default panel items Wind, Lighting, Global properties and Window properties; the unused pages are gone). Themes adds a background brightness (multiplies both background colours, 0-3), a colour picker (hue bar and saturation/value square beside R, G, B, A) and named colour presets (save, load, delete; kept in prefs.txt). Viewport holds the original's window properties that apply (fn 0x14052e030 tooltips): Lighting, Wireframe (lines only), Texture mapping, polygon visibility (hide back faces), Alpha testing, Grid, Label viewports, Zoom distance scale (camera distance when framing), Manipulator size (how near a press picks a scene object), Translation / Rotation / Scale snapping with their unit, degrees and amount (applied while dragging scene objects), plus field of view. Lighting: sun direction and height, AO, lit. Wind: on, strength, direction (the sway follows it). Global: the Tree generator's shape radius and preview LOD with statistics.
- The original's Edit menu (fn 0x1402a93a0, run when menus refresh): the Undo action reads "Undo " + the name of the next undo step and is enabled, or "Nothing to undo" and disabled; Redo likewise ("Redo " + name / "Nothing to redo"); it also has Undo selection / Redo selection actions (actionUndo_selection, slots SlotUndo, SlotRedo, UndoSelection, RedoSelection, fn 0x1402a64c0), and refuses objects added to the undo buffer during a save or load (fn 0x14041f6b0). The build words its Edit menu the same way, naming each step from what changed (Add / Delete / Edit <generator>, Add <object>, Edit scene objects, Edit <mesh>); selection undo isn't built.
- Startup, Undo, Preferences (the person's requests, picture 20): the program starts with only the Tree generator (the self-test still uses a demo tree). Undo / Redo (Edit menu, Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z) has no limit: after each finished edit the generators, scene objects, leaf meshes and materials (without their picture buffers) are kept when they differ from the last kept state. Edit > Preferences: a Blender-style window; Themes > 3D Viewport sets the viewport colours (background top/bottom, grid, every-5th grid line, grid X/Z axes, labels, bones, normals, scribed edges, forces, collision objects, picked object, corner axes X/Y/Z); the original's own viewport options here are background colour, grid colour and viewport label colour (fn 0x14052e030). Saved to %APPDATA%/TreeCraft/prefs.txt.
- Generation Add menu (fn 0x140228270): built from the install's templates folder, read recursively; entries sorted by name, so leading underscores come first; a folder becomes a submenu named with its underscores removed; a .stt file becomes an entry with its leading underscores removed, the other underscores turned into spaces and ".stt" dropped; then "From file..." (Templates (*.stt)), a separator and "Default generator" (the generator types by name, with types 1, 2, 4, 5 and 6 under a "Legacy" submenu). The build lists the same entries (Trunk ... Leaf Mesh; Decorations, Hand Drawn with Legacy, Photogrammetry, Zones) and makes the closest generator it has, named after the entry (trunks, branches, leaf meshes); the others report that they aren't built yet, and the templates' own settings (e.g. Big Branches uses Generation:Mode 6) aren't read.
- At the person's request (picture 17): a click inside the drawn shape adds an inner vertex there instead of changing the outline; the mesh splits the triangle holding it and flips edges toward a Delaunay triangulation (never the outline's edges), which always joins it to its nearest vertex. Inner points can be dragged (inside the shape), removed, painted and transformed, and are kept per level. (The original stores all points in one list and appends new ones, fn 0x140301500.)
- At the person's request (picture 16): outside clicks still join the closest point, falling back to the edge that adds the least length. In Color mode the triangles show the painted vertex colours blended across them. The Edit Cutout window resizes from any corner or side, and the Properties and Generation/Materials/Meshes panels widen or narrow by dragging the gap between them and the viewport.
- Transform gizmo (picture 15, the person's design; the original's cutout window has no Transform group): each transform tool shows a gizmo on the card (Move: red X and green Y arrows with a free centre square; Rotate: a ring with the swept angle and its degrees; Scale: red/green square-ended handles for width/height and a centre square for even scaling), with the change shown as text as the original's manipulator can ("Enables text displaying transformation information on the manipulator", viewport options fn 0x14052e030; settings Manipulator:Mode, Coordinate system, Show text, Smooth lines, Size scale, fn 0x14053c190). Placement follows the original's manipulator (fn 0x140570530): it sits at the average of the selected objects' positions (summed, then x 1/count), with world axes when several objects are selected; the build puts it at the average of the card's points and rotates/scales around that point. Its size (70 px handles) is the build's choice; the original's default Size scale isn't stored in the program.
- Canvas mouse handling (COptimalMeshWidget vtable 0x14107cbd8, mouse-move fn 0x140300880): middle-drag pans (delta / canvas size), middle+left drag zooms by 0.01 per pixel; grab radii (constants 0x14107caac.. 0x14107cb18): points 7 px, point 0 (the pivot) 10 px, +20 px in Remove points mode, anchors 7 px and 27 px in Remove anchors mode; the colour brush (mode 6) paints along the stroke from the last mouse position with radius 20 px (fn 0x1402fd0f0); mask brushes use 10 px (paint) and 20 px (erase). Anchors are stored as pairs (position, line end) with parallel id and side lists ("AnchorIds", "AnchorSides" in the saved mesh); removing an anchor removes the pair and its id and side. The build has the pan/zoom (plus a Fit button), the radii and the stroke brush.
- Canvas mouse press (fn 0x140301500, the slot before mouse-move): right-click on an anchor in anchor mode opens a menu with "ID" (1, 2, 3, 4, 5, the current one ticked) and "Side" (Automatic, Left, Right); a new anchor gets ID 1 and Side Automatic and its line end at position + (position - pivot) normalised x 0.05 (constant 0x14102c9e8), i.e. pointing away from the pivot; pressing within 7 px (49 squared, constant 0x14107cb48) of an anchor while adding points switches to dragging that anchor, and of a point while adding anchors to dragging the point; in points mode a click with nothing under it appends the point at the end of the list (the build inserts it next to the closest point, the person's request); the colour brush's first dab on press has radius 10 px. The build has all of this; Side is stored with each anchor but doesn't change generation (what it does isn't read). At the person's request (picture 14) the numbers aren't limited to 1-5: the right-click list and the generators' Anchors choice offer at least 5 and as many numbers as there are green anchors (or the highest number used), and the Angle wheel and knob turn smoothly with no snapping (shown to one decimal).
- Slots (COptimalMeshDialog meta-object, dispatcher fn 0x1406038b0, in order): IntChanged, AngleChanged, SpinnerChange, Other, ModeAddPoints (fn 0x1402f7b30), ModeRemovePoints, ResetPoints (fn 0x1402f7c90), ModeAddMask, ModeRemoveMask, ResetMask, ModeColor, ColorClicked, ColorChanged, ResetColor, ModeAddAnchors (fn 0x1402f8110), ModeRemoveAnchors, ResetAnchors (fn 0x1402f8270), ColorTypeChanged, MaterialChanged, SaveButton, LoadButton, TimerFirstShow, UpdatePreviews.
- Reset Points (fn 0x1402f7c90) asks "Are you sure you wish to reset the points? This cannot be undone" (title "Reset Points", Yes/No); Yes resets the canvas (fn 0x140301210) and switches to Add points (mode 0). Reset Anchors (fn 0x1402f8270) asks "Are you sure you wish to reset the Anchors? This cannot be undone" and then switches to Add anchors (mode 4; canvas reset fn 0x1403013f0). Both clear the generated mesh. The canvas reset of points leaves one entry at (0.5, 0.25, 0) (constants 0x1415433a0, 0x141543440) whose role isn't settled; the build clears the outline (clean slate, the person's request) and resets anchors to (0.5, 1.0).
- Improve folding (read; checkbox m_pImproveFolding stored at +0x128 by fn 0x1402f4120): fn 0x1402f8980 calls fn 0x1401a4ef0(pivot, direction) after triangulating and again after tessellating. The fold line runs through the pivot along (0, 1) turned by the Angle, i.e. (-sin a, cos a). For each edge shared by two triangles whose ends lie on opposite sides of the line, it flips the edge to the other diagonal of the pair when |edge . dir| < limit, where limit = |other diagonal . dir| if that diagonal also crosses the line, else 3.4e38 (constant 0x14102c810); only when both triangles' areas add up the same both ways (within 1e-5, constant 0x14103ecd8); it repeats until nothing flips. It adds no points. (The "adds a center point" help belongs to the Simplify dialog's own option, fn 0x14033b570.) The build does the same flips; it applies several flips per pass, each on triangles not yet changed in that pass.
- At the person's request (picture-10): a mesh with no cutout opens with no points (Reset points clears), there is no automatic subdivision, the window moves by its title bar and resizes by its corner (it stays inside the TreeCraft window), and an Anchor group plus a knob on the growth line set the anchor's position and angle.
- Tessellation (read, fn 0x1402f8980): t = slider value x 0.0001 (constant 0x14102d4a4; slider 0..10000); when t > 0 it calls fn 0x1401a6550 with (int)(t x t x 5000) (constant 0x14107c9ac), which repeats fn 0x1401a4320 that many times: each repeat takes the triangle with the largest circle, adds a point at its circumcentre and re-fits the triangles whose circles hold it. The build adds the same number of points at the same place, and uses the middle of the longest edge instead when the circumcentre falls outside its triangle (so points never leave a concave outline). The window's "V:%1  F:%2" label counts vertices and faces.
- Angle (read, AngleChanged slot fn 0x1402f9190): angle (radians, canvas +0x238) -= wheel value x 0.005 (constant 0x141044064), with no rounding; fn 0x1402f8980 turns the growth direction (0, 1) by it to (-sin a, cos a), from the pivot, when the Angle box is checked. The build's wheel turns 0.005 radians (about 0.29 degrees) per pixel dragged, smoothly, as the person asked.
- Estimated meanings: Angle (double-click resets) turns the leaf's growth line from straight up in the picture, and the mesh is built along it; Double-sided adds back faces. Extra anchors are stored (the first is the attachment point).

Read from the 8.4.2 program: panel layout, labels, tooltips and choices in fn 0x140076fd0,
Orient choices and their stored values plus Copy/Paste/Duplicate in fn 0x140084750. Fields match the
original's <Mesh> data in .spm files (FixWinding, FlipNormals, CollisionScale, Weight, Wind, Flip,
Orient, SpineStyle, PivotStyle, Collection, MoreLodsMeshID, Scale, Lod_1/Lod_2).
- Top row: drag handle (drop on the viewport: the selected Leaf generator, or the first one, uses
  the mesh), mesh picker, +/- (add card / add cutout from the reference material / remove),
  + (load an OBJ file as a new mesh), Copy / Paste / Duplicate.
- Geometry: Edit Mesh... opens the cutout editor (points and anchor over the picture). High / Med /
  Low are three levels; each is a cutout or a loaded OBJ file, with Browse..., Reload and Export (OBJ).
  An empty level uses the level above. More: another mesh supplies levels 3 and lower.
- +/- opens the "Meshes" dialog (the original's CAssetAddRemoveDialog; texts and tooltips in
  fn 0x14004e3a0, 8.4.2). It has:
  - Add new ("Add new asset")
  - Duplicate ("Duplicate selected"; the copy goes right below)
  - Rename ("Rename selected"; double-clicking a name or F2 also renames, Enter keeps, Esc cancels)
  - Move up / Move down arrows to reorder
  - Remove ("Remove selected assets"; Ctrl/Shift-click selects several)
  - OK applies everything; Cancel / Esc / × discards it.
  On OK, leaf generators, "More" links and material cutouts keep pointing at the same meshes after
  reordering; removed meshes fall back to the plain card. The dialog is modal while open.
  Rules read from the original's code:
  - The +/- handler (fn 0x1400860d0) opens the dialog titled "Meshes" with default name "Mesh"; changes
    apply only on OK (undo text "meshes change"), copying settings into duplicates, and the panel stays
    on the mesh that was showing before.
  - The slots (dispatcher fn 0x1405ff5d0: MoveUp, MoveDown, Add, Rename, Duplicate, Remove,
    ItemChanged, SelectionChanged) work like this:
    - Move up / Move down move every selected item one place and keep them selected.
    - Add new appends "Mesh" (made unique) at the end. The new mesh is empty: nothing in High, Med or
      Low and a blank preview (V:0 F:0), as in the person's screenshot of the original. Edit Mesh… on an
      empty High level offers "Start with a card"; "..." loads a mesh file.
      The original's new-mesh routine (fn 0x1400b1000) sets these fields; TreeCraft's `LeafMesh::empty`
      uses the same values:
      | Offset | Field | Value |
      |---|---|---|
      | +0x00 | wind vector | (0, 1, 0), i.e. Growth +Y |
      | +0x0C | ripple uses the normal | on |
      | +0x10 | ripple direction | (0, 0, 1) |
      | +0x1C | Flip | 0 (X Y Z) |
      | +0x20 | Orient | 6 (Z-Up Right-handed) |
      | +0x58 | collision scale | 1.0 |
      | +0x5C | weight | 1.0 |
      | +0x68 | detail levels | three, empty |
      | +0x8C | Fix winding | off |
      | +0x8D | Flip normals | off |
      | +0xA4 | More | −1 (none) |
    - Rename edits the first selected item.
    - Duplicate appends "<name> Copy" (made unique) at the end and opens its name for editing at once.
    - Remove deletes every selected item.
  - Unique names (fn 0x1400509b0): while a name is taken, if its last word is a number it goes up by
    one, otherwise " 2" is appended ("Mesh" → "Mesh 2" → "Mesh 3").
- Panel handlers read from the original's dispatcher (fn 0x1406003e0, 8.4.2):
  - **Copy / Paste / Duplicate** (case 9):
    - Copy stores the current mesh's settings.
    - Paste overwrites the current mesh with them but keeps its name and id (undo text "paste mesh").
    - Duplicate makes "<name> Copy" (made unique), adds it at the end with the same settings, and
      shows it in the panel (undo text "duplicate mesh").
  - **Browse…** (case 3): if the mesh holds embedded data, it first asks "Loading a file will delete this
    asset's embedded data. Do you wish to continue?".
  - **Loading a High file** (case 8) asks "Attempt to find associated LOD meshes?". On Yes:
    - the file name's trailing digits, then a trailing "lod" (any case), are removed to get a base;
    - "<base>*" mesh files in the same folder are listed in name order;
    - the "…lod<digits>" files after the chosen one become the next levels.

    The original looks at .stm, .obj, .srt, .fbx and .xml files; this build reads OBJ only.
  - **Export** uses a "Save Mesh" dialog with a "Wavefront object (*.obj)" filter.
  - **Count label** reads "V:%1  F:%2", plus "  A:%1" for anchors (TreeCraft counts one anchor for a
    cutout).
  - **Edit Mesh… on a file-based mesh** asks "Editing this mesh will convert it to embedded…" in the
    original. TreeCraft's cutout editor edits cutouts only, so for a file level it offers "Replace with
    a cutout" instead (differs).
- Show: which level the preview and the cutout editor use. Alignment: draws the intended
  orientation (a leaf in X/Y growing up +Y from the origin, facing +Z). V:F = vertices and faces.
- Transform: Orient (Z-Up Right-handed = 6 default, Y-Up Right-handed 7, Z-Up Left-handed 8,
  Y-Up Left-handed 9, XYZ 0, XZY 1, YXZ 2, YZX 3, ZXY 4, ZYX 5), Flip (X Y Z … -X -Y -Z),
  Flip normals, Fix winding.
  - The Orient handler (fn 0x1400823a0, undo text "orient change") stores the choice's value; the Flip
    handler (fn 0x1400822a0, "flip change") stores the list index. Both then reprocess the mesh in
    fn 0x1400ab6c0.
  - Axis matrix per Orient value, read from the switch in fn 0x1400ab6c0 (base matrix = identity at
    0x141796e18). Result (x′, y′, z′) from source (x, y, z):
    | Value | Choice | Result | Mirror |
    |---|---|---|---|
    | 0, 6 | XYZ, Z-up RH | (x, y, z) | no |
    | 1, 9 | XZY, Y-up LH | (x, z, y) | yes |
    | 2 | YXZ | (y, x, z) | yes |
    | 3 | YZX | (y, −z, −x) | no |
    | 4 | ZXY | (−z, x, −y) | no |
    | 5 | ZYX | (z, y, x) | yes |
    | 7 | Y-up RH | (x, −z, y) | no |
    | 8 | Z-up LH | (x, y, −z) | yes |
  - Flip index 1/2/3 negates X/Y/Z, 4 = X+Y, 5 = X+Z, 6 = Y+Z, 7 = all three (−1.0 at 0x14102c874).
  - Each mirror, each flipped axis, and Flip normals add one to a count; the faces are reversed when the
    count is odd. Normals go through the same matrix, scaled by −1 when Flip normals is on (flag at +0x8d).
  - Then, per face (fn 0x1400ab6c0):
    - faces whose cross product has length² below 1e-12 (0x141040780) are dropped;
    - vertex normals shorter than length² 0.9 (0x14103f968) get each face's normal × 0.01
      (0x14103fbbc) added;
    - with Fix winding (flag at +0x8c), a face is swapped when the angle between its winding normal and
      the average of its vertex normals is over 90° (0x14102d4a0; radians → degrees × 57.2958 at 0x14102c808).
    - Afterwards tangents and binormals are rebuilt from the texture coordinates (not used by this build).
  - Wind direction handler (fn 0x14007e100): the wind vector is zeroed, then axis (choice mod 3) is set to
    +1.0 for choices 0–2 or −1.0 for 3–5 (constants 0x1415432f4 = 1.0, 0x14102c874 = −1.0), so the list
    order is +X, +Y, +Z, −X, −Y, −Z.
  - Collision scale and weight have their own handlers (fns 0x14007ddc0 / 0x14007de60).
- Collection: when on, the mesh is built when the tree computes from every Trunk/Branch generator
  marked "Contribute to collections" (Spines: Branches / Fronds / Both — this build has no fronds;
  Pivot: Origin keeps world positions, First makes them relative to the first collected spine).
- Wind direction: Growth (+X +Y +Z -X -Y -Z) is the mesh axis that points along the leaf when placed
  (default +Y, from the sample's WindY = 1); Ripple (Normal, X, Y, Z) is stored for the wind system
  (this build has no wind yet).
- Proxy collision: Scale and Weight are stored (used for world-building proxies in the original;
  not used by anything in this build yet).
- "Generate from material" makes an outline around the opaque pixels with the chosen number of
  points (own method: convex outline, then corners merged while keeping the leaf covered).

## Main menu bar (action texts read from fn 0x140294d60)
- File: New Ctrl+N, Open Ctrl+O, Recent files, Close Ctrl+W, Save Ctrl+S, Save incremental, Save as
  Ctrl+Shift+S, Save as with assets Ctrl+Alt+Shift+S, Export mesh Ctrl+E, Export selected as mesh,
  Export material, Export world building data, Export image, Export image sequence, Render photo,
  Import mesh asset, Import material asset, Import cameras from FBX, Exit.
- Edit: Undo Ctrl+Z, Redo Ctrl+Y, Undo selection Alt+Z, Redo selection Alt+Y, Preferences, Edit Tree
  Window properties. Tools: Render / Clear ambient occlusion, Generate collision primitives, Wind
  wizard, Growth wizard, Arbitrary scene scale, Scene unit conversion, Create collection from
  selection, Remove unused assets, Reset lightmap scalars, Switch to Substances. Window: Cascade /
  Tile tree windows, Properties, Generation editor, Materials, Meshes, Displacements, Masks, Output,
  Material sets, Timeline, Layout (Normal, Full screen, Full screen no bars), Reset layout. Help: the
  original's manual / web links, About.
- Save incremental (fn 0x14041c880): needs a current file, else "Cannot save incremental version with
  no current save file."; digits after the last '_' of the name count up, written "_%03d"; with no
  such digits "_001" is added; if that file exists: "Incremented file already exists. File not saved."
- Recent files (fns 0x1402a76e0, 0x1402a7f00): settings key recentFileList; back slashes become '/',
  the opened file moves to the top, at most 16 kept (while count > 0x10 drop the last); a file that
  fails to open is removed from the list.
- Save prompt (fn 0x1402b3f80): "This tree has been modified." or, when the tree has a file, "The tree
  [<file name after the last / or \>] has been modified.", then "\n\nDo you wish to save your
  changes?"; warning icon, buttons Yes | No | Cancel (0x414000), default Cancel (0x400000); Yes saves
  (fn 0x1402b0820), No closes without saving, Cancel keeps the tree open.
- Which menu each action sits in is inferred from their order in that function. This build's own
  choices: image sequence = 36 pictures in one turn; collision
  primitives = 3 capsules along the trunk; unit factors 0.3048 ft→m, 2.54 in→cm, 100 m→cm. Items with
  no TreeCraft feature (FBX cameras, Growth wizard, Substances, Displacements, Masks, Output,
  Timeline, web links) say so in a message.
- Closing, New, Open or Close with unsaved changes shows that prompt (the person's request, picture 22).

## New generators start neutral (the person's request, picture 22)
- A generator added from +Add has no noise, jink, gravity, length/angle/radius variation or profiles,
  no flare, droop, fold, curl or twist, taper 1, and no material (Material "None"): it draws plain grey.

## Property curves and the curve editor (picture 34)
- Each property definition sets a flags byte at its name + 0x5C (fns 0x14010ef60, 0x14010f240):
  0x02 = Parent curve; 0x01 = a second curve, a Profile when 0x04 is also set (Lightmap:Weight 0x07,
  shown yellow) and otherwise a Distribution (Season:Time scale 0x03, shown blue). Read for 206
  properties by scanning the program's code (src/stcurves.rs), combined over every generator type;
  others fall back to Parent only when the table marks them as splines.
- Curve editor (SplineEditorDialog, fn 0x1403a3870): title "Curve - generator:property:kind", Parent
  level, Draw, Presets (Max, Min, Half, Linear growth, Linear decay, Exp growth, Exp decay, Rise, Droop,
  Round, Bell, S-Curve, Conical, Fade in/out, Season - blossom/spring/summer/transition to
  autumn/autumn/dead, Resolution - standard/aggressive/draft knockout), X, Y, Pin. Right-click menu
  (fn 0x1400dba00): Insert point / step point / collapsed point here, Delete selected control points
  (DEL), From curve max/average/middle/min, Scale curve, Flip curve vertically / horizontally, Select
  all (Ctrl-A), Copy (Ctrl-C), Paste (Ctrl-V); "The last two control points cannot be deleted".
- This build's own choices: preset shapes; smooth points use cubic Hermite slopes from their
  neighbours, step points hold their value, collapsed points are corners; Scale curve = 1.25x about
  the max / average / 0.5 / min; values stay within 0..1. Branch ends are open unless Skin > Cap ends.

## Choices, working spine settings, number arrows (picture 36)
- Choice lists read from the texts each definition stores just before its property name (the
  compiler keeps a function's texts in reverse order of use): Skin:Type = Polygons / SubD surface
  (rectangular attachments) / SubD surface (arbitrary attachments) / Spine only; Shape Control:Style =
  Sphere / Enabled force meshes; Growth:Timing:Style = In place / Parent / Parent's start and end;
  Vertex Color:Red/Green/Blue:Style = Set / Offset from parent; Phyllotaxy Arrangement:Style =
  Opposite (distichous) / Opposite (decussate) / Opposite / Alternating (distichous) / Alternating /
  Whorled (order inferred).
- Now used by the tree (this build's formulas): + % of radius (length + amount x parent radius),
  Orientation:Roll (turns added), Shape:Ancestor (start turned toward the parent's direction),
  Straightness (bends x (1 - amount)), Twist (skin turned along the length), Prune General (random
  share), Up / Down (direction past 1 - 2 x amount), Interior (pointing back toward the trunk axis),
  Ground (any point below 0), Break Broken / Chance / Spot / Seed, Noise Late Smooth and Seed,
  Skin:Type Spine only (no bark).
- Curves: the second curve (flag 0x01) acts along the node's own length; on trunks the Parent curve
  reads the trunk's place among the trunks.
- Number arrows: whole numbers step 1; others a round step (1/2/5 x 10^n) near 1/50 of the range.

## Variance (picture 35)
- Flag 0x10 in the property definitions = the property has variance (cleared on choices like UV
  Mapping:Style); the +/- button shows on rows with 0x10 or any curve.
- Variance Settings (CVarianceSettingsDialog, fn 0x1404f57d0): Variance, Distribution (Gaussian (bell
  curve) / Uniform (random)), Cohesion group: Cohesive .. Random slider, Scale, Offset.
- This build: value + Variance x r per node; r uniform in -1..1 or the mean of three (x1.7, clamped)
  for Gaussian; Cohesion blends a shared part (two sines along the parent at Scale, phase Offset)
  with each node's own random. Defaults: Uniform, Random (1), Scale 1, Offset 0. Length + % of parent,
  Start angle, Radius + % of parent and Leaf size keep their amount in TreeCraft's own variance fields.
- Number boxes: a click selects the whole number so typing replaces it (Ctrl+A or double-click
  selects again; arrows deselect).

## UV Mapping (fn 0x14012ff90; picture 32)
- Choices read from the original: Style = Unrestricted / "U locked [0, 1], V unrestricted" /
  "U and V locked [0, 1]" (default 0); V Style = Absolute / Relative (keep aspect ratio) / Relative
  (default 1); V Anchor = Start / End (default 0). Defaults: U Absolute 1, + Relative 1, Offset 0,
  Twist 0, Random flip on, Fractional off (Value 1), V Scale 1, V Offset 0.
- This build's formulas (estimated, gen.rs UvMap): U repeats = Absolute x Relative (x Fractional
  Value when Fractional is on); u = around x repeats + Offset + Twist x (distance / length), the twist
  flipped on about half the branches when Random flip is on; V from the base (Start) or tip (End):
  Absolute = distance x Scale, Relative (keep aspect) = distance / (2 pi r0) x U repeats x Scale,
  Relative = distance / length x Scale; + V Offset. U locked: one repeat, no offset or twist; U and V
  locked: V = distance / length.
- Number fields everywhere: a horizontal spinner dial (300 units of drag ~ the usual range, Shift
  x0.1) and a box to type a value (ranges above 1 can go to 100 x their top).

## Materials on generators, roughness, viewport picking (picture 25)
- The original's generator Material tab has a Materials group with a Branch list and an Extension
  list (- / +), each entry a Material choice ("Inherited" first) and a Weight (picture 25 of the
  original; property names Materials:Branch:0:Material / Weight in its tables). Materials are not
  assigned automatically. This build picks one entry per branch by weight (steady per node);
  Inherited takes the parent branch's material; an empty list draws plain grey.
- Roughness = 1 - Gloss (the person's request): white = rough, black = glossy; a Roughness picture
  multiplies the value in the shader. Seasons tint leaves only when a season is chosen (default None);
  ambient and sun light are uncoloured so textures keep their colours.
- A click without dragging (under 4 units of movement) picks the generator whose node line is within
  12 units of the mouse; a double-click moves the camera target to the nearest bone point (16 units)
  and halves the camera distance (at least 1).

## Edit modes and property curves (picture 24)
- Original texts (fn 0x1405325b0): Generator mode — "All property edits apply to the selected
  generator(s) ... allows parent curve editing. Hot key: Tab"; Node mode — "Property edits in this
  mode are offsets to the value computed by the generator and should be used for fine tuning only.
  Hot key: Tab". Field hint "Ctrl = parent curve, Shift = variance" (fn 0x140543dd0).
- This build: Node offsets = length/size x, radius x, start-angle +, roll + (turns), hidden; nodes are
  numbered in the order the generator makes them. Freehand strokes are projected onto the plane
  facing the camera through the branch point they start on (14-unit grab) or the view target, and
  resampled to 16 even steps; a new Hand Drawn generator has Frequency 0 so only drawn branches exist.
- Curves map 0..1 to a multiplier 0..1 (piecewise linear). Parent: x = place on the parent;
  Distribution: x = order among the children on one parent; Profile: x = along the branch (gravity,
  late/early noise, radius) or, on Frequency / Absolute Number, child density along the parent
  (rejection sampling, 32 tries). The person named the three kinds; the colours are theirs.
- Render > WireShaded and Render > Wireframe are the person's (Blender-like), not the original's.

## Node graph and window (the person's design, picture 23 — not the original's look)
- Generators are boxes (128 x 46 units, 20-unit header coloured by kind, body with the instance count),
  a socket below (from the parent) and above (to children), blue S-curve wires (cubic, control points
  half the height apart, at least 30 units); sibling spacing 150 units. Forces and collision objects
  are boxes of their own beside the tree. Colours: Preferences > Themes > Node Editor.
- The window opens maximized; closed un-maximized, it reopens at that inside size (prefs.txt
  window_max / window_w / window_h).

## Controls
- Viewport: left-drag orbit, middle-drag or Shift+left-drag pan, right-drag or wheel zoom, F frame.
- Keys: 0 perspective, 1 front (Ctrl: back), 3 right (Ctrl: left), 7 top (Ctrl: bottom), 5 toggle orthographic,
  Delete removes the selected node, Ctrl+D duplicates, Esc deselects.
- Graph: click selects, drag moves, Shift+drag onto another node re-links, right-click for a menu,
  wheel zooms, drag empty space pans. All estimated (the original's exact bindings weren't readable).

## Skin and Spine tabs, wires, copy/paste (picture 38)
- Skin:Radius:Length kill ratio (fn 0x140141d60, read at +0x488): when the ratio is above 0, the skin type isn't Spine only (3) and (radius <= 0 or ratio < length / radius), the branch is removed ("1Branch removed (length/radius larger than kill ratio)"). Default 500. Used in gen.rs process.
- Skin:Radius:Length compensation (fn 0x140141d60, read at +0x480): c = compensation x radius x 2.0 (const 0x14102c9a0 = 2.0); amount = 1 - clamp((length - c) / (c x 5.0), 0, 1) (const 0x1415434c8 = 5.0). Where the original applies the amount wasn't found; TreeCraft thins the radius by up to half (estimated).
- Estimated (no formula found in the original yet): Visibility, Light seam reduction, Skin Roll, Skin Smooth, Welding (Offset, Spread, Blend), Subdivision for SubD skin types, Intersection Texture Blend, Squash, Splits, Spine Alignment, Unify, Early Turbulence, Allow forces, Match one v tile, Shape Control (on when its Size:Absolute is above 0) with Shape scale and Stop shape control.
- Generation Sweep, First and Last take any value; spots off the parent (below 0 or above 1) are skipped.
- WireShaded / Wireframe: lines 1.8 px by default (Preferences > Viewport > Wire thickness), colours editable there, selected generator's lines 35% toward white (Selected wire brightness).
- Every property group header has Copy and Paste buttons.

## Segments tab, Spine only, UV preview (picture 39)
- Property tables (fn 0x140140870): Segments:Radial:Absolute is a whole number (type 3, 0..100000, flags 0x02|0x01 = parent and second curve); Segments:Radial: + Relative is a number (type 4, 0..max float). The value table index for a definition at offset D is (D - 0x40) / 0xA8 x 8 (checked against Skin:Type 0x820 -> 0x60, Length compensation 0x5EC0 -> 0x480, Length kill ratio 0x5F68 -> 0x488 read in fn 0x140141d60).
- Estimated: Accuracy = spine growth steps (Accuracy / 4, 4..64; 50 -> 12); Length segments = "+ Relative" (TreeCraft's length segment count) + Absolute x length; Weld scalar crowds segments toward the base (u^weld); Optimization drops points turning less than Optimization x 20 degrees; Radial = Absolute + round(+ Relative x base radius x 4).
- Skin:Type "Spine only": no bark; each spine is drawn as a yellow line with a mark at every segment; its node has a black header and a black badge with a yellow chain (after the original's spine-only node icon).
- UV preview: the material shown follows an Inherited Materials entry up to the parent, as the bark does.

## Curves along the branch, wire balls, movable Preferences (picture 40)
- Second curves read along each branch's own length (0 base .. 1 tip), as SpeedTree's Profile curve: Segments:Length:Absolute (yellow) sets how densely the length segments sit (points placed by the curve's running total; high at the left = more at the base); Segments:Length:Optimization, Skin:Squash Amount / Rotate, Skin:Welding:Spread:Amount, Skin:Light seam reduction, Skin:Subdivision:Relax scale and Spine:Shape:Ancestor (keeps turning toward the parent's direction) scale their value along the branch. Already along the branch: Gravity, Noise Early / Late Amount, Skin:Radius. Parent (green) curves scale every setting by where the branch sits on its parent (extras now start from the original's default, so their curves work too).
- Viewport corner, right to left: shading balls, wire-mode balls (Scribed, WireShaded, Wireframe; click again = Standard; still in Render), Selection bar (Solid fill / Outline / Both at one click, colour chip opens colours and amounts).
- Preferences: drag the title bar to move, any corner to resize (at least 620 x 420); kept for the session.

## Default curves, radial curve, Node Editor preferences (picture 41)
- Every property definition writes two default curve presets at +0x48 (Parent) and +0x4C (second curve), numbered from 1 in the curve editor's preset order (seen in fn 0x140140870: Skin:Radius:Absolute +0xA60 = 1, +0xA64 = 5; Segments:Radial:Absolute +0x910 = 1, +0x914 = 1). Read for every property by %TEMP%\tc_scan\presets.ps1 into stcurves.rs DEFAULT_CURVES (e.g. Skin:Radius second curve 5 = Linear decay - the original's radius taper; Welding second curves 5; Growth:Transitions 12 = S-Curve; Spine:Break:Chance parent 9 = Droop; Wind:Scalar 6 = Exp growth).
- A new curve starts as its default; a property with a non-flat default uses it even before a curve is drawn. Skin:Radius is left out (TreeCraft's Tip radius does that taper). Spine:Noise:Late:Amount's second curve defaults to Linear growth (the person's request; the original writes none there).
- Segments:Radial:Absolute's second curve changes the count around the branch along it; rings of different counts are stitched with a zipper.
- Preferences > Node Editor: node style (Rectangle, Rounded, Circle, Ellipse, Hexagon, samples to click), background colour, grid (Dots, Lines, Crosses, None) and spacing, wire style (Curved, Straight, Angled) and thickness, node size, show counts.

## Lighting, sky, ground and shadows, light controller, height indicator (picture 42)
- The original's light (fn 0x140292fb0): Environment Preset, Directional Color (default white) and Intensity, Ambient Sky 0x5391d3, Horizon 0xb2e3ff, Ground 0x8a8570 and Intensity; Ambient Occlusion Brightness and Contrast (default 0), Min 0.25, Max 1, Computation Render / Clear; Shadows Quality (Disabled, Low, Medium, High, Very high quality - table 0x14178e050, default Medium) and Ground weight 0.3. Presets are read from app_data/light_presets.xml (fn 0x1402922b0): Daylight (2.0, 0.4, altitude 0.5), Bright sun (4.0, 0.3), Dusk (2.0, 0.3, altitude 0.8, sun #ffb291), Overcast (0.75, 1.0), Moonlight (0.5, 2.0, sun #acd9ff), with their sky / horizon / ground colours.
- TreeCraft: sun = colour x intensity x 0.5 (Daylight gives the old sun of 1); ambient = Sky above, Horizon level, Ground below (by the normal) x intensity x 1.4; AO = Min..Max, then Contrast and Brightness; Altitude a -> sun height 90 x (1 - a) degrees (estimated). Default sun 45 degrees up, 35 degrees round (a 10 am / 2 pm light).
- Fixed: branch triangles were wound clockwise, so the shader flipped their normals inward and lit the bark from below.
- Ground: a disc under the grid (Preferences > Viewport > Show ground, also Show > Ground) taking a planar shadow of the tree (vertices slid along the sun to the ground; depth test "less" keeps overlaps from darkening twice). Quality: 1, 3, 5 or 8 passes from slightly different suns; Ground weight x 1.5 = how dark.
- Background: Themes > Background "Sky and ground (SpeedTree)" (default) - sky above the horizon, ground colour below, the horizon placed by the camera's tilt - or TreeCraft's dark gradient.
- Light controller under the gizmo: click the bulb, then drag in the viewport to move the sun; wheel = intensity (shown under it); right-click = Lighting page. The fan is a placeholder for the wind system.
- Height indicator (Show > Height indicator): an orange and white staff beside the tree banded every round step, heights written beside it, the full height on a tag at the top.

## Light in Properties, one soft shadow, height line, tips card (picture 43)
- The light's settings (all of picture 42's) are now the Properties panel's "Light" page (Sel::Light), opened by Edit > Lighting, Light > Properties and right-clicking the light controller; Preferences has no Lighting page (pages: Themes, Viewport, Node Editor, Wind, Global).
- Shadow: one shadow (the sun isn't jittered any more); Quality softens its edge with copies shifted sideways by 0.4 / 0.6 / 0.8 % of the tree's height; the shadow sits 0.3 % of the camera distance above the grid and the ground 0.4 % below it.
- Height indicator: a thin soft line with ticks and heights, a small tag with the full height, and a hiker 1.8 units tall (TreeCraft's own figure) at its foot.
- Light controller and fan: upper left, under the view menu. Tips: the "?" at the viewport's bottom left (and Help > Controls) opens a card of controls by area.

## Solid hard shadow, shadow presets, person, Node Editor in Themes (picture 44)
- Shadow: one pass, hard, solid - drawn in the ground's own colour (or the background's past the ground's faded edge) darkened by Ground weight x 1.6, so overlapping parts can't double up and flicker; it sits 0.2 % of the camera distance below the grid (ground 0.4 % below), so the grid shows over it.
- Shadow side light ("wrap"): diffuse = max((N.L + wrap) / (1 + wrap), 0); default 0.3 so the shadow side isn't too dark.
- Shadow presets (Light > Shadows > Style), TreeCraft's own settings in the style of games: Uncharted style (soft), Red Dead style (realistic), Zelda style (bright, painterly), The Last of Us style (moody), Ghost of Tsushima style (warm, airy) - each sets wrap, ambient intensity, ground weight, AO min and contrast.
- Person for scale: an ordinary adult standing at ease (TreeCraft's own figure), 1.8 units tall.
- Preferences: the Node Editor settings are under Themes > Node Editor with its colours; pages are Themes, Viewport, Wind, Global.

## The original's preference tabs, shadow picture, reference figure (picture 45)
- The original's Preferences dialog (fn 0x140047920; its texts at 0x14102dca0..0x14102e470): General (Display scaling, Warn about node edit losses, Use multiple processors, Always use native file dialogs, Autosave: Enable / Notify in console / Interval (min)), Tree Window (Hide selection indicator on edit: Always / When wind is on / Never, Show fan, Size of height indicator, Navigation: Control scheme Default / Autodesk Maya / Autodesk 3ds Max, Reverse zooming / rotating / panning), Property Bar (Use sliders where possible, Use group stacking, Animate scrolling, Group click: Slams to top / Reveals), Render (Multi-sampling Disabled / 2x / 4x / 8x - applies to new windows, Shadow anti-aliasing samples 1 / 5 / 9 / 17 / 25, Max texture resolution 512..8192, Flip normals on backsides), Data (usage reporting).
- TreeCraft: all are working: display scaling multiplies the UI scale; node-edit warning when a change moves a generator's nodes; multiple processors decode a tree's pictures in parallel when it opens; non-native dialogs open in the folder used last; autosave keeps the last session every Interval minutes (status + console note); hide selection while a value is being changed; fan shown or not; height indicator marks scaled; Maya (Alt + left / middle / right) and 3ds Max (middle pans, Alt + middle orbits, Ctrl + Alt + middle zooms) controls and reversals; number fields without the dial; group stacking (opening one closes the others); glide scrolling; a group opened by a click scrolls to the top or into view; multi-sampling from the next start; the shadow drawn into a picture 1 / 1.5 / 2 / 2.5 / 3 x the viewport for 1..25 samples and laid over the ground once (smooth edges, no flicker); pictures scaled to the max resolution (reloaded when changed); back faces lit as their own side or like the front. Data: TreeCraft never sends anything, so there's nothing to switch. Defaults are TreeCraft's choices (the original's weren't read).
- Shadow presets: Default (TreeCraft's starting look) at the top, "(custom)" last.
- The person for scale is the person's own reference figure (assets/human.png, cut from picture 45).

## Freehand bend controller, world origin (picture 46)
- Freehand mode puts a controller on the selected trunk or branch generator (drawn on its first node): a green top handle, a blue band, a pink pivot bar. Dragging the top aims the band from the pivot at the mouse: the spine above the pivot turns round the axis between its straight and aimed directions, a little more at each step along the band (share (s - pivot) / (top - pivot), raised to Bend shape), keeping its length; past the top it keeps that direction. Dragging the band slides pivot and top together along the generator; dragging the pivot moves only it (the band stretches or shrinks). Kept as Freehand:Lower / Upper / Angle / AxisX-Z / Shape in the generator's settings; every node of the generator bends the same way in the world. Panel: the same values as sliders (direction as degrees round the vertical), Reset bend, Reset generator to default; Ctrl + drag still draws new branches.
- World origin: a disc at (0, 0, 0) in Themes "World origin" colour (radius a tenth of the tree's size, at least 2.5), lit with the picked colour while the Tree node is selected.

## Kept bends, new controller look (picture 47)
- Starting to move the band or the pivot while the controller holds a bend keeps that bend (copied to Freehand:Kept<i>:..., Freehand:KeptCount) and the controller starts a new straight one; the spine applies the kept bends in the order made, then the controller's. Panel: Kept bends count, Remove the last kept bend, Reset all bends.
- Controller look: a glowing band shading from cyan (pivot) to green (top) with beads, a pink pill-shaped hinge at the pivot, a ringed green knob with bend arrows at the top, all lighting up under the mouse or while dragged; a tag shows the bend and how many earlier bends are kept.

## Rename select-all, click-away, back arrow (picture 48)
- Renaming a material or mesh (double-click or Rename) starts with the whole name selected and highlighted: the first typed letter replaces it, Backspace clears it; arrows/Home/End drop the selection, Ctrl+A selects all again.
- A click on empty viewport space (Generator mode) returns the Properties panel to its default view from any panel (material, mesh, light, window), not only from a generator.
- Properties header: a back arrow left of the title goes to the panel shown before (the selection history, same as Alt+Z), or to the default view when there is none.

## Global and Window properties in the Properties panel, title bar (picture 49)
- Global properties = the Tree generator's panel, laid out like the original's "Properties - Tree" (names from the original's strings at 0x141096e30-0x141098a00): Generation (Shared: Size scalar; Geometry: Branches/Caps, Leaves/Fronds/Meshes, Subdivision = Triangles | Quads), Shape (Radius, Sink, Roll), Subdivision (Level 0, Relax 0.25), Dynamic LOD (Enabled, Number 3, Branches/Decorations/Leaves/Fronds, Leaf Settings: Grow scale 2, Jumble 0.1, Seed 15000), SpeedTree SDK (User data), Lightmap (Target resolution 128 of 32..2048, Display multiplier "x1 (Same as target)"), Compute Flags (Vertex colors), Vertex Color (0, 0, 1), Proxy Collision (Automatic, Scale 1, Cull, Restore culled). Working: Size scalar scales the whole tree, Sink lowers it, Roll turns it (degrees), Radius spreads trunks, Dynamic LOD Enabled/Number limit the preview LOD. Faint rows are kept and saved, not used yet.
- Window properties = the original's "Properties - <scene>" panel (strings at 0x1410b9480-0x1410ba350): Viewport (Layout One | Two (side by side) | Two (over under) | Four; Grid lines color = the theme's grid colour; Grid spacing 10 = 2.5 TreeCraft units; Lightmap view % 0.3; Background Style Ambient light | Solid color | Full frame image | Letterboxed image | Stretched image | Spherical panorama, Color, Image, Panorama Adjustments Brightness / Contrast / Rotation), General Rendering (Transparency Best available (A2C if possible) | Alpha test only | Blending only | Alpha test and blending; Anisotropic filtering x16; 3D icon scale 2.8), Manipulator (Mode Translate | Rotate | Scale, default Rotate; Coordinate system Global | Local; Size scale), Screenshot Safe Frame (Show, Width 1024, Height 1024: drawn as a frame with the outside dimmed), User Shader Constant 1 (1, 0, 0, 0), Capture Camera (Style Disabled | Interactive | Sequence render, Left X/Y, Middle X/Y, Light azimuth / pitch). Saved with the scene ([window] section); New resets them.
- The home panel's Actions section is left out for now; Preferences no longer lists the Global page (its Viewport page is TreeCraft's own display settings).
- Title bar: "<name> [<full path>] - TreeCraft <version> Beta" like Blender, "* " in front while unsaved changes exist, "(Unsaved)" before the first save.

## Custom cameras (picture 50)
- View menu > Add camera: a camera where the view is (same position, direction and zoom). From the perspective view it is a perspective camera; from Front/Back/Left/Right/Top/Bottom it is an orthographic camera locked to that view (no heading/tilt, only roll). The menu then lists "Camera: <name>" to look through each one.
- Lens: perspective focal length (mm) with sensor width (36 mm default), field of view = 2 atan(sensor / 2 focal) across the width, vertical from the frame aspect; editing the field of view sets the focal length. Orthographic scale = the view's height. Shift X/Y kept (not applied yet). Automatic clipping, or Clip start/end.
- Transform: Position X/Y/Z, Heading, Tilt, Roll (degrees), Target distance (its orbit point when looking through it).
- Frame: presets (HD 1080p, 720p, 4K, Square, Portrait, Cinema 2.39:1, Photo 3:2), Resolution X/Y, Resolution %, picture size, Show frame, Passepartout. Looking through it puts its frame at 90% of the viewport's limiting side, with the outside dimmed by Passepartout and a label (name, size, lens).
- Depth of field: Enabled, Focus distance, F-stop (2.8), Blades (6), Show focus plane; the sharp range uses circle of confusion = sensor / 1500 and scene units as metres: H = f^2/(N c) + f, near = s(H-f)/(H+s-2f), far = s(H-f)/(H-s) (infinite at s >= H). The viewport doesn't blur yet; the focus plane is drawn.
- Display: Icon size (yellow icon, resizable), Show name, Show limits, Lock camera to view (moving the view while looking through it carries the camera; otherwise moving the view leaves it).
- In the perspective view the selected camera shows X/Y/Z move arrows (red/green/blue), a yellow turn ring (left/right turns, up/down tilts) and a centre dot (free move across the view); arrow length follows Window properties > Manipulator > Size scale. Click an icon to select it; Name is renamed in the panel (all selected, typing replaces it). Saved with the scene ([camera] sections).

## Cameras: lens kept off the viewport, guides, undo, select while looking (picture 51)
- Looking through a camera saves the viewport's own view first. Leaving from the menu or "Stop looking through" puts that view back exactly. Leaving by moving the view keeps the new place but restores the viewport's field of view and projection, scaling the distance so the middle stays the same size. The camera's lens never stays on the viewport.
- Guides on the selected camera (Display > Show guides, on by default): a dashed view line with focal length and horizontal x vertical field of view, plus width and height dimension lines with arrows (pixels, and the sensor's size in mm: width = sensor, height = sensor / aspect). When the icon is small on screen, the numbers move to a card beside it.
- Undo / Redo keep the cameras (add, delete, move, every setting); edit names read "Add Camera 1", "Move Camera 1", "Edit Camera 1".
- While looking through a camera, its frame label is a button that selects it, and the view menu has "Select active camera".

## Camera picture icon, guides at full size (picture 52)
- The camera icon is the camera picture the person supplied (attachments/picture-52.png, white background cut away, assets/camera_icon.png), drawn at the camera's place as wide as Icon size on screen (28-170 px) and flipped so its lens points the way the camera looks. A thin yellow view cone with its up mark stays at the lens to show direction; the selected camera gets a soft yellow ring. Clicking the picture selects it.
- Guides no longer depend on the icon's size on screen: the view volume is drawn out to the target distance (dashed edges, the picture frame there, a cross at the target). There is a dashed view line with "focal mm | FOV h x v", an arc showing the horizontal field of view near the lens, and width/height dimension lines beside the far frame (pixels, plus the sensor's mm; orthographic cameras show scene units).

## 3D camera, Export material window (picture 53)
- The camera is now a 3D model drawn with the scene (depth-tested, lit from over the viewer's shoulder). It follows the supplied picture's design in its blue: two film reels with five holes and a hub on each face, a body with a side panel and two buttons, a back knob, a lens collar and an open hood that is darker inside. It is about 1.4 x Icon size long, turned with the camera (lens along its view). The old yellow box in front of it is gone; the selection ring, name, move/turn handles and guides stay.
- File > Export material (F10) opens the original's Render Images window (CRenderModeImageDialog; strings at 0x141089438-0x141089b40).
  - Options: Keep aspect ratio (follows the viewport's shape), Width / Height (4096), Override bkgd (on), Shadows (kept, not used), Streak colors (on), Ground, Clip below ground (kept, not used).
  - Sequence: Use timeline (kept, not used), Include wind (on), Length (s) 10, Frame rate 30, Frames 300 (= length x rate).
  - Images, defaults as the original: "" Material / Color (background 0.16, 0.25, 0.12); Opacity; Normal (0.5, 0.5, 1); Gloss (0.8); AO (white); Subsurface (0.8, 0.88, 0.38); SubsurfaceAmount (white). Every alpha is None with background alpha 0.
  - Sources are the install's render_modes names (Standard, Scribed, AO, Albedo Check, Subsurface Check, Lightmap, Lighting / ..., Material / ..., Vertex Colors / ..., Other / ...).
- OK asks for a folder, then renders each image from the current view off screen, one per frame, and saves <tree>[_suffix].png (sequences add _0001...). Alpha None = opaque where drawn; another source = its brightness. Where nothing is drawn: Override bkgd sets the background colour and alpha. Streak colors fills the colour outward from the drawn pixels (push-pull over halving sizes) so filtering doesn't pick up the background.
- Assets > Export material asset... still writes one material as a .tcmat file.

## Export material window: moves, resizes, works, presets (picture 54)
- The window is moved by its title bar and resized from any of its four corners (min 560 x 470); it stays where it was left.
- Its buttons and drop-downs work: the app blocks the screen behind modal windows, and this window now takes the input itself, as the Preferences window does, while nothing behind it reacts.
- Add Image opens a menu of the render modes, as the original offers them, grouped like the install's render_modes folder: Standard, Scribed, AO, Albedo Check, Subsurface Check, Lightmap, then Lighting / Material / Vertex Colors / Other submenus. The picked one becomes a new image named after it (suffix without spaces), with a background suited to it (flat normal for Normal, white for AO and Subsurface Amount, black otherwise).
- Presets: a Preset drop-down lists the saved ones (%APPDATA%\TreeCraft\export_presets\*.tcexport; picking one loads it), Save preset... writes the current settings and images, and Load preset... reads any preset file. Format: "# TreeCraft export material preset 1", key=value lines, and image=suffix|RGB source|Alpha source|r,g,b|alpha.

## Export material: render from a camera, progress and result (picture 55)
- "Render from" (under Sequence): Current view, or any of the scene's cameras by name (saved in presets as camera=<name>). A camera renders through its own lens with its field of view filling the picture's height; with Keep aspect ratio, picking a camera sets the height from its frame shape. The summary line says which view it renders from.
- After OK, a progress window shows "Picture k of N: <file>", a bar with the percentage, the folder, and Cancel (stops after the picture in hand; saved files stay).
- When it ends, a message says "Export finished" with a green tick (all saved) or "Export finished with problems" / "Export cancelled" with an orange mark. It lists how many pictures were saved, the folder, and any that failed, with Open folder (opens it in Explorer) and OK.
- Camera guides and icons are clipped to the viewport (they no longer run over the panels).

## Gen tab all active, welding off, handles, V + drag light, origin (picture 56)
- Generation tab rows that were kept but unused now act (estimated behaviour; the original's code for them wasn't traced):
  - Shared > Position: shifts every child along the parent by that share.
  - Shared > Rotation: turns them all around the parent (turns).
  - Extend parent: spots stretched past the tip, t x (1 + extend); a child past the tip starts along the tip's direction.
  - Boundaries > Clip (0.025): drops children within that share of the parent's tip.
  - Favor splits (Absolute / Proportional): pulls each child toward the nearest spot where another generator's child grows on the same parent.
  - Align (on) with Align style 1 (default): children's roll counts from the world's up, seen across the parent; style 0 uses the parent's own frame; Align off adds a random turn to each child.
  - Sink: the base pushed back into the parent by Sink x the parent's radius there.
  - Knockout: removes children; chance = (1 - Weight) x Knockout, plus Weight x sqrt(Knockout) for children within Proximity Range of a kept sibling.
  - Anchors: picks the anchor on a mesh parent when the Leaves tab's choice is All.
  - With the defaults (Clip 0.025, Align style 1), the demo tree changed slightly: 1350 -> 1323 leaves.
- Skin > Welding > Enabled is off by default (the original's template has it on); gen.rs reads it with default 0.
- Curve editor handles: grabbed within 18 px (was 12), also from just outside the graph for the end points on its edge; the grabbed point follows the mouse exactly instead of moving by the mouse's movement, so it can't lag. Wider grabs for the camera arrows and ring (11 / 10 px) and the Freehand controller (18 / 17 / 11 px).
- Hold V and left-drag in the viewport: the directional light turns around the tree horizontally only (azimuth, the same rate as the light controller: 0.4 deg per pixel); its height stays. Listed in the viewport's tips card.
- World origin: default colour 0.9, 0.9, 0.6 with alpha 0.2 (pale and faint; an old saved orange default is replaced), no ring around it. Selected (Tree node), it's a little brighter and stronger.

## Trunk sink, zoom through, welding redone, right-aligned labels (picture 57)
- Generation > Sink on a trunk (no parent branch): the base goes down into the ground by Sink x the trunk's own radius.
- Mouse-wheel zoom no longer stops near what it looks at: under 0.5 units away the view's centre itself moves ahead (0.12 of the distance per wheel step), so the camera passes through and can clip into the tree; the near clip plane stays at 1% of the distance.
- Welding (estimated; the original's code wasn't traced), with the original's rows Enabled, Keep failed, Force ray casts, Offset, Spread (Amount, Upper scale, Lower scale) and Blend (Texture, Contrast, Noise). The Blend > Lighting curve isn't there yet.
  - Base: inside the weld zone (parent radius x (1 + Offset)) it widens by Spread Amount x 1.6, more toward the parent's tip (Upper scale) or base (Lower scale).
  - Hugging: it eases onto the parent's surface (the parent taken as a cylinder along its direction at the joint, 1.2% outside it), fully at the surface and none at the zone's end (smoothstep), so the joint never stands out of the parent.
  - Shading: Blend turns the joint's normals to the parent's surface direction (Texture, sharpened by Contrast, ragged by Noise) and its shading to the parent's, with a faint crease.
  - A branch leaving almost along its parent (|cos| > 0.985) can't be welded unless Force ray casts; a failed weld drops its bark unless Keep failed.
- Property names in the Properties panel and on the Preferences pages are right-aligned against their controls, as in the original; group and section headings keep their alignment.

## Welding: dissolving texture blend and denser joint (picture 58)
- Texture blend (TreeCraft's own method):
  - A welded base dissolves into its parent's bark: every vertex of the weld zone carries a fade, strongest where it lies on the parent, sharpened by Contrast, ragged by Noise, scaled by Texture.
  - The fade rides in the colour's alpha below the selection mark (unselected 255..235, selected 128..108).
  - The shader compares it with three octaves of 3D value noise (world position x 2.3, 6.2 and 18.2) and leaves out the pixels where the fade wins. The parent's own bark shows through in an irregular, bark-like pattern, with no blending or sorting.
  - A thin rim along the edge is shaded down to 84%, like a crease in the bark.
- Joint segmentation:
  - With welding on, the weld zone (parent radius x (1 + Offset)) gets 8 + 6 x Offset extra rings (6..32), packed toward the parent (spacing ^1.35), and the rings there go round in twice as many steps (up to 64), joined to the rest by the existing ring zipper.
  - A bigger Offset gives a longer, smoother weld with more rings.

## TreeCraft's weld solver: boolean cut, fillet, segment controls (picture 59)
- The parent is taken as a cylinder along its direction at the joint.
- Weld zone: from the parent's middle to where the branch's lowest side leaves it, (pr + r0 |cos|) / max(sin, 0.25), plus Offset x pr of fillet. A shallow-angled branch gets a longer zone.
- Boolean cut: each column of the base (one angle round the branch) is cut where it enters the parent. The crossing is found by marching 40 steps along the column, then halving the step 10 times, and is cached per column.
  - Every vertex before the cut sits at the cut point on the parent's surface (0.3% outside it), so no mesh is left inside the parent.
  - A final check pushes any vertex that would still be inside back out to the surface.
- Fillet: after the cut the base widens (Spread x 1.6, shaped by Upper / Lower scale) and settles 90% of the way down onto the surface. It eases out over Offset-based length pr x (0.35 + 0.6 Offset), with the falloff (1-u)^2 (1+2u).
- Blend (normals, shading, the dissolving fade) follows the fillet's hold on the surface.
- Welding > Segments (TreeCraft's own rows):
  - Rings: extra rings in the weld zone; 0 = automatic, 8 + 6 x Offset.
  - Ring bias (1.35): how tightly they pack toward the parent.
  - Radial detail (2): how many times more steps the rings in the zone go round.
- Tested: cargo test weld welds a branch onto an upright trunk at 90, 60, 45 and 30 degrees, and no vertex comes within the trunk's radius.

## Weld texture fade off for now (picture 60)
- The dissolving texture fade (picture 58) left out pixels on the welded base, so a gap showed between branch and trunk. It's switched off (gen.rs TEXTURE_FADE = false) until texture blending is taken up again; the rule is kept in the code. The weld mesh, fillet, normals and shading are unchanged; the base now shows its bark all the way to the trunk.

## Specular one value, weld Push, Skin sub-tabs, main tab look (picture 61)
- Material Specular is one value (0..1) on the material card and in the Properties panel; the three colour channels are kept equal under it.
- Welding > Push (TreeCraft's own, -0.5..0.5, default 0): the surface the weld meets moves into (-) or out of (+) the parent by that share of its radius. It is used for the cut, the fillet and the never-inside check, so a weld can close a gap on a parent whose surface isn't round at the joint (a coarse, faceted trunk sits inside its round radius).
- Skin tab sub-tabs: All, General (the rows above the sections), then one per section (Radius, Welding, Subdivision, Intersection, Squash, Splits). Only the picked one's rows show; TreeCraft's extra Skin rows go with All and General. Other main tabs have none yet (to follow if the person likes them).
- Main tabs look distinct: solid rounded buttons on a dark strip, the picked one in the accent colour with a light line along its top. Sub-tabs are plain smaller words with an accent underline under the picked one, above a thin divider.

## Even Push, sub-tabs everywhere, Blender-like deselect (picture 62)
- Welding > Push now moves the finished weld edge along the parent's surface direction by Push x the parent radius, weighted by the fillet's hold (1 at the cut, easing to 0). The cut is found on the parent's own surface, so the upper and lower sides move alike. Tested: gen::weld_tests::push_moves_the_whole_edge (Push -0.2 brings both sides inside 0.9 of the radius).
- Sub-tabs on every generator tab but All, whenever a tab has more than one part (General plus its sections). Each tab keeps its own pick (branch:<tab> / leaf:<tab>). On the Skin tab the Intersection rows (Texture Blend) show under Welding, and Intersection no longer has its own sub-tab.
- The same tab looks across the app: the Generation / Materials / Meshes tabs use the main-tab style (accent fill, light top line); the material card's Settings / Meshes use the sub-tab style (accent underline).
- Selection like Blender:
  - A generator picked: highlighted, its settings shown.
  - One click on empty viewport space: the highlight and outline go, but its settings stay in the panel, still editable. The panel's subtitle says "not highlighted (click empty space again to deselect)".
  - A second click on empty space: fully deselected (the default panel).
  - Picking anything brings the highlight back. Materials, meshes and other panels still go straight back to the default view on one click.
