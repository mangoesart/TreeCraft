This build contains the original game's own files, for the private use of the person who owns the game, on their own computer. It must not be shared or published.

# TreeCraft

A procedural tree modeler for games: build trees from Trunk, Branch and Leaf generator nodes,
edit materials and leaf cutout meshes, and look at the result in a perspective or orthographic viewport.

- Build: `cargo build` (Rust + macroquad).
- Run: `target/debug/treecraft`.
- `--screenshot <file.png>`: draws about 120 frames, saves a picture and exits.
- `--self-test <file.json>`: drives the app with scripted mouse input (orbit, select a node, drag a
  property, add a node, switch tabs) and writes the results.

See MECHANICS.md for the rules and where every default value comes from, and CREDITS.md for the
original files used.
