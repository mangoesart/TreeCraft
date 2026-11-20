# Credits – original files used

All from the person's own installed copy, folder `C:\Program Files\SpeedTree\SpeedTree Modeler v10.2.0`.

| File in this build | Source | Notes |
|---|---|---|
| assets/icons/generator_tree.png | app_data\generator_tree.tga | node icon, converted TGA → PNG |
| assets/icons/generator_branch.png | app_data\generator_branch.tga | node icon (trunk and branch) |
| assets/icons/generator_leaf.png | app_data\generator_leaf.tga | node icon |
| assets/icons/generator_border.png | app_data\generator_border.tga | selection ring |
| assets/icons/generator_frond.png, generator_leaf_mesh.png, generator_leaf_card.png, generator_branch_node.png, generator_highlight.png, generator_blank.png | app_data\*.tga of the same names | converted, not used yet |
| assets/fonts/InterVariable.ttf, assets/fonts/LICENSE.txt | app_data\fonts\ | Inter font (SIL Open Font License) |

Values read (not copied as files): node defaults from `templates\combined\*.stt`, material channels
from `tree_templates\Games\Broadleaf.spm`, colours from `app_data\style.css`, generation mode
names from the program (function 0x1403175a0). Textures (bark, leaf) are drawn by code.

- `render_modes/**/*.fx` (install folder): the viewport render modes' formulas were read from these files and rewritten in TreeCraft's own shader (no file is copied or shipped).

- assets/camera_icon.png: the camera picture the person supplied with their request (picture 52), background removed.
