# dsp_seed_gen — provenance

`src/data/` and `src/worldgen/` are copied from **DoubleUTH/DSP-Seed-Finder**
(https://github.com/DoubleUTH/DSP-Seed-Finder), commit `c050ff02ac59923ae318e04c613c98ecf92ee9f8`
(2026-10-08), licensed under the Apache License 2.0 (`LICENSE.txt`, copied unchanged). The upstream
repository has no NOTICE file. License checked on the GitHub repository (license field Apache-2.0) and
in its `LICENSE.txt`.

Changes made here (Apache-2.0 §4(b)):
- `src/lib.rs` is new: plain-Rust entry point (`generate`) returning an owned summary. The upstream
  wasm-bindgen / websocket entry points, search rules (`rules/`, `transform_rules.rs`) and tests are not copied.
- `data/vein.rs`, `data/planet.rs`: `ActualVein` gains `nodes`, the number of vein nodes kept
  (above water) per type; for crude oil one node is one seep.

The theme / vein tables inside `data/theme_proto.rs` were exported from the game by the upstream
author; the game data belongs to its publisher (report §2.4).
