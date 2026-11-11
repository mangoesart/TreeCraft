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

## Controls
- Viewport: left-drag orbit, middle-drag or Shift+left-drag pan, right-drag or wheel zoom, F frame.
- Keys: 0 perspective, 1 front (Ctrl: back), 3 right (Ctrl: left), 7 top (Ctrl: bottom), 5 toggle orthographic,
  Delete removes the selected node, Ctrl+D duplicates, Esc deselects.
- Graph: click selects, drag moves, Shift+drag onto another node re-links, right-click for a menu,
  wheel zooms, drag empty space pans. All estimated (the original's exact bindings weren't readable).
