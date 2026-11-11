# What we found in C:\Program Files\SpeedTree\SpeedTree Modeler v10.2.0\win64\SpeedTree_Modeler.exe

SpeedTree_Modeler.exe is a program for Windows, built for x86_64 (64-bit). It looks like it was made with Qt. Its internal names were removed (stripped), so most functions will have generated names.

Evidence ev_e4b44d84ddaa740c5eca2a4c20202450bb331c20fbba89c7a7f6f43ba0c96594.

```json
{
  "architecture": "x86_64",
  "bits": 64,
  "dotnet_arch": null,
  "entry_point": "0x141876484",
  "format": "pe",
  "format_name": "PE (Windows)",
  "kind": "executable",
  "libraries": [
    "ADVAPI32.dll",
    "GLU32.dll",
    "IPHLPAPI.DLL",
    "KERNEL32.dll",
    "MSVCP140.dll",
    "OLEAUT32.dll",
    "OPENGL32.dll",
    "Qt6Core.dll",
    "Qt6Gui.dll",
    "Qt6Network.dll",
    "Qt6OpenGL.dll",
    "Qt6OpenGLWidgets.dll",
    "Qt6Svg.dll",
    "Qt6Widgets.dll",
    "QtAdvancedDocking.dll",
    "SHELL32.dll",
    "SHLWAPI.dll",
    "USER32.dll",
    "VCOMP140.DLL",
    "VCRUNTIME140.dll",
    "VCRUNTIME140_1.dll",
    "WINHTTP.dll",
    "WS2_32.dll",
    "api-ms-win-crt-convert-l1-1-0.dll",
    "api-ms-win-crt-environment-l1-1-0.dll",
    "api-ms-win-crt-filesystem-l1-1-0.dll",
    "api-ms-win-crt-heap-l1-1-0.dll",
    "api-ms-win-crt-locale-l1-1-0.dll",
    "api-ms-win-crt-math-l1-1-0.dll",
    "api-ms-win-crt-runtime-l1-1-0.dll",
    "api-ms-win-crt-stdio-l1-1-0.dll",
    "api-ms-win-crt-string-l1-1-0.dll"
  ],
  "next_steps": [
    {
      "tool": "core_overview_show",
      "why": "a one-page summary: sections, imports, counts"
    },
    {
      "tool": "native_functions_list",
      "why": "every function we found"
    },
    {
      "tool": "native_strings_find",
      "why": "find text the program shows or uses"
    },
    {
      "tool": "native_function_study",
      "why": "a full dossier on one function (start with main or the entry point)"
    },
    {
      "tool": "core_feature_find",
      "why": "follow a word you see in the app to the code behind it"
    },
    {
      "tool": "native_pe_resources_list",
      "why": "icons, version information and the manifest"
    }
  ],
  "operating_system": "Windows",
  "plain_english": "SpeedTree_Modeler.exe is a program for Windows, built for x86_64 (64-bit). It looks like it was made with Qt. Its internal names were removed (stripped), so most functions will have generated names.",
  "recommended_toolsets": [
    "core",
    "native",
    "formats"
  ],
  "stripped": true,
  "symbol_table_stripped": true,
  "toolchain_hints": [
    {
      "category": "framework",
      "evidence": "links Qt",
      "id": "qt",
      "label": "Qt",
      "strength": "strong"
    },
    {
      "category": "compiler",
      "evidence": "Rich header",
      "id": "msvc",
      "label": "Microsoft Visual C++ toolchain",
      "strength": "strong"
    }
  ]
}
```

## Ghidra

Deep mode: Ghidra has analysed C:\Program Files\SpeedTree\SpeedTree Modeler v10.2.0\win64\SpeedTree_Modeler.exe (0 min), so decompiling it is quick.

## The game's own files

Folder: C:\Program Files\SpeedTree\SpeedTree Modeler v10.2.0

- Maps and levels: 1 files, 0.0 MB. For example:
  - app_data\style.css.map
- Textures and pictures: 1110 files, 1085.5 MB. For example:
  - samples\VFX\Sample_Pine\Needles\Pine_Needles_1.png
  - samples\VFX\Sample_Pine\Bark\Cap.png
  - samples\VFX\Sample_Palm\Leaves\Palm_Front\Palm_Front_2.png
  - samples\VFX\Sample_Palm\Leaves\Dried_Palm\DriedPalm_1.png
  - samples\VFX\Sample_Palm\Bark\FlattenedDisplacement.png
  - samples\VFX\Sample_Mushroom\Bark\Mushroom_White.png
  - samples\Learning\Seasons\Leaves\EasternRedbud_front_1.png
  - samples\Learning\Seasons\Bark\Cherry_bark_1.png
- 3D models: 56 files, 87.5 MB. For example:
  - samples\Learning\Photogrammetry\Oak_Photogrammetry\Mesh\Sample_Photogrammetry_Trunk.obj
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Meshes\LongDetail_01\LongDetail_01_VFX_High.fbx
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Meshes\Knot_02\Knot_02_VFX_High.FBX
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Meshes\Knot_01\Knot_01_VFX_High.FBX
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Meshes\Bump_02\Bump_02_VFX_High.fbx
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Meshes\BrokenBare_01\BrokenBare_01.fbx
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Meshes\BarkPart\Bark_Part_01_LOD0.FBX
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Decals\BrokenBark_02\BrokenBark_02_VFX_High.fbx
- Settings and scripts: 57 files, 1.8 MB. For example:
  - vertex_packing\Standard.lua
  - texture_packing\Lumberyard.xml
  - samples\Learning\Photogrammetry\Decal_MeshDetail\Meshes\Bump_02\Bump_02_Anchor.xml
  - samples\Learning\Photogrammetry\All_Mesh_Tree_Example\Meshes\Photogrammetry_branch_1.xml
  - mesh_packing\Default.lua
  - licensing_client\Windows\Unity.Licensing.Client.deps.json
  - export_presets\VFX\Growth.ini
  - export_presets\Games\Generic FBX.ini
- Read-me and licence files: 6 files, 0.2 MB. For example:
  - licensing_client\Windows\LICENSE.md
  - app_data\fonts\LICENSE.txt
  - app_data\AppStyle\license-buy.png
