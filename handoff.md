
## Picture 48
meshdialog.rs MeshDialog.edit_all (set in start_rename). editmodes.rs viewport_pick empty click -> Sel::None for any non-None. props.rs draw_panel header: btn:prop.back (undo_selection or Sel::None). Self-test rename no longer presses Backspace (typing replaces the selected name).

## Picture 49
winprops.rs: tree_rows (Kind::Tree panel), window_rows (Sel::Window), App::wp / set_win_image / draw_window_background / draw_safe_frame. App.winp + win_image_path saved in treefile [window]. gen.rs place_tree (Size scalar, Shape:Sink, Shape:Roll). wintitle.rs sets the Win32 title (EnumWindows + SetWindowTextW) twice a second from main.rs frame(). Home panel: Global properties selects the Tree node, Window properties selects Sel::Window, Actions removed. prefs page 3 (Global) hidden from the list. TM_SEL=window (+ TM_SAFEFRAME=1) for pictures.
TRAP: never pass a one-pair @(@(a,b)) list to a replace loop in PowerShell (it flattens and replaces single characters). main.rs got every "m" turned into "o" this way and was rebuilt with a vocabulary fixer (tc_scan\fixm*.ps1); backup in %TEMP%\tc_scan\main_repaired_backup.rs. Prefer the Edit tool.

## Picture 50
cameras.rs: ViewCam (from_view, to_cam, take_view, sharp_range, basis), write/read_line for treefile [camera], App::add_camera / look_through / delete_camera / update_camera_view (main.rs before draw_viewport_bg) / draw_cameras_overlay (end of draw_viewport_overlay; icons, handles, frame; near-plane clipping of lines), camera_rows (props Sel::Camera(i)). render.rs Cam gained roll + clip. View menu indices count separators (views 0-6, sep 7, Orthographic 8, sep 9, Add camera 10, cameras 11+). TM_SEL=camera / camlook, TM_NOWELCOME=1 for pictures.

## Picture 51
cameras.rs: App.look_saved (saved in look_through(Some)), look_through(None) restores it, leave_camera_here() when navigation leaves the camera; ViewCam.show_guides (saved), guides drawn in draw_cameras_overlay (dimension lines, or a card when the icon spans < 160 px); frame label = button (vp.camlabel) selecting Sel::Camera. undo.rs Snapshot.cameras + signature + describe + restore. View menu: "Select active camera" at index n+4+cameras.len() while looking.

## Picture 52
cameras.rs: cam_sprite() (assets/camera_icon.png, cut from picture 52 by %TEMP%\tc_scan\camicon.ps1); icon loop rewritten (sprite + thin cone + full-size guides to c.dist); the hit test includes the sprite. Line splices go through %TEMP%\tc_scan\splice52.ps1, which checks the edge lines first.

## Picture 53
cameras.rs: CamMesh (built once), App::draw_cameras_3d (called in draw_viewport_3d after draw_scene_objects). The 2D overlay keeps the ring, name, handles and guides; cam_sprite was removed (assets/camera_icon.png stays, unused). matexport.rs: MatExport, ExportJob, draw_material_export (modal; main.rs dialog_input includes it), run_material_export (main.rs before draw_viewport_bg; waits for a built, loaded tree), render_mode_pixels (render_target + draw_gpu, now pub(crate)), compose/streak. F10 in keyboard(); File "Export material..." opens it; Assets "Export material asset..." = the old export_material. TM_SEL=matexport (+ TM_MATEXPORT_DIR) for pictures and test exports.
Craft Anything's "Open Folder" button is Craft Anything's own project record; it can't be changed from inside a build. The person opens F:\PROJECTS\ARTCRAFT\GitHub\TreeCraft as the project in Craft Anything.

## Picture 54
matexport.rs: ui.blocked=false at the window's start, then blocked + swallowed clicks at its end (the freeze). MatExport.win/win_drag/win_grab for move/resize; add_image_menu + add_image_source (tests); to_preset/from_preset, presets_dir, preset_names; Preset combo + Save/Load buttons in the bottom row. Tests: cargo test --release matexport (2 pass).

## Picture 55
matexport.rs: MatExport.camera (name; "" = current view); combo "matexp.camera" in the Sequence box; render_mode_pixels uses cameras[name].to_cam(1.0). ExportResult + App.mat_export_result; draw_export_progress (progress or result, modal); material_export_takes_input also while a job or result exists. cameras.rs: draw_cameras_overlay clips to the viewport (inner fn draw_cameras_overlay_in). TM_MATEXPORT_CAM=1 renders from a camera added at the view (the view then turns, so identical files prove the camera was used: verified, tree.png 49168 bytes both times).

## Picture 56
gen.rs: shared_placement() after distribute (Position, Rotation, Extend parent, Clip, Favor splits); Knockout in process() with `kept` positions; make_spine: Align / Align style (mode prefix), Sink, t > 1 for Extend parent; the anchor pick falls back to Generation:Anchors. stpanel.rs used_extra has every Gen row. stprops Welding default false. editmodes.rs curve window: hover 18 px, over_graph includes handles just outside, the drag follows the absolute mouse. app.rs: light_vdrag (V + left drag), origin disc without ring; prefs.rs origin default + migration.

## Picture 57
gen.rs: weld info is now (parent radius, tangent, point); skin_tube hugs the parent's cylinder (weld_to), with normal/AO blend; skin() checks a failed weld (Force ray casts / Keep failed). make_spine: trunk Sink. render.rs Cam::zoom dollies the target forward under 0.5. props.rs Rows::label/faint_label right-aligned; prefs.rs row labels right-aligned (3 places). stprops: Keep failed, Force ray casts. TM_WELD=<node> for pictures (welding on, picture 57's spread, the view on the trunk).

## Picture 58
gen.rs: densify_base() at skin_tube's start when welded (zone computed first); ring_n doubled inside the zone; per-vertex fade into col[3]. render.rs FS: hash3/vnoise, weld fade discard + weldShade on base. TM_WELD=Branch (+ TM_WELDNEAR=1, + TM_RENDER=6 for wires) for pictures.

## Picture 59
gen.rs skin_tube: the zone uses the branch angle; densify_base(rings, bias); ring_n x weld_radial in the zone; the per-vertex solver (col_at, cross_cache, cut, fillet, the never-inside check). SkinLook.weld_rings / weld_bias / weld_radial; stprops + stpanel rows Skin:Welding:Segments:*. Test gen::weld_tests. Note: shot49.ps1 run twice back to back can catch the previous window; wait 9 s or take one shot per call.

## Picture 60
gen.rs skin_tube: const TEXTURE_FADE = false stops the per-vertex fade (the shader still reads alpha, now always full). Texture blending at the weld is deferred by the person's request.

## Picture 61
matpanel.rs slot 4 and props.rs material rows: Specular as one value. gen.rs SkinLook.weld_push (pr = pr0 x (1 + push) inside the solver); stprops + stpanel row Skin:Welding:Push. stpanel.rs: tab_bar restyled, sub_tab_bar(); draw() builds the Skin sub-tabs and filters rows (sub_filter); Rows.sub_tab <-> App.prop_sub_tab. TM_SUBTAB=<n> for pictures.

## Picture 62
gen.rs: Push applied after the never-inside step (o2 x push x pr x hug); tests in weld_tests. stpanel.rs draw(): sub-tabs for any tab != All with >= 3 entries; Rows.sub_tabs HashMap <-> App.prop_sub_tabs; Intersection merged into Welding. app.rs draw_dock tabs and matpanel SUBTABS restyled. App.sel_soft: editmodes viewport_pick (first empty click), GenCtx.highlight and the outline gate skip it, select() clears it; props subtitle hint.

## Picture 63
render.rs FS Scribed branch: plain white. app.rs Scribed edges: draw_thick_wires. editmodes.rs viewport_pick double-click: target = hit point, dist = |eye - point|, yaw/pitch from the eye direction (eye unchanged); orthographic: pan to the spot.
