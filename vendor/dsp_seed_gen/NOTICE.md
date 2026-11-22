# dsp_seed_gen — provenance

`src/data/` and `src/worldgen/` are copied from **DoubleUTH/DSP-Seed-Finder**
(https://github.com/DoubleUTH/DSP-Seed-Finder), commit `c050ff02ac59923ae318e04c613c98ecf92ee9f8`
(2026-10-08), licensed under the Apache License 2.0 (`LICENSE.txt`, copied unchanged). The upstream
repository has no NOTICE file. License checked on the GitHub repository (license field Apache-2.0) and
in its `LICENSE.txt`.

Changes made here (Apache-2.0 §4(b)); every changed upstream file starts with a `// planner fork:` line:
- `src/lib.rs` is new: plain-Rust entry point (`generate`) returning an owned summary, and `fingerprint`.
  The upstream wasm-bindgen / websocket entry points, search rules (`rules/`, `transform_rules.rs`) and
  tests are not copied.
- `data/vein.rs`, `data/planet.rs`: `ActualVein` gains `nodes`, the number of vein nodes kept
  (above water) per type; for crude oil one node is one seep.
- `src/detmath.rs` is new, and 14 upstream files call it (2026-11-22, review 8): `sin`, `cos`, `asin`,
  `acos`, `atan2`, `ln`, `log10`, `log`, `powf` now go through the `libm` crate (MIT) in double
  precision on every target, instead of `std` (the platform C library natively, Rust's libm on wasm32).
  Files: `data/planet.rs`, `quaternion.rs`, `random_table.rs`, `star.rs`, `vector3.rs`, `vector_f3.rs`,
  `planet_algorithms/algo{3,8,9,10,11,12,13,14}.rs`. Upstream's native and WASM builds differ in the
  last bit of these functions, which moved veins on one planet of seed 12345678; this fork gives the
  same numbers on both (`fingerprint`), equal to upstream's native Windows build for that seed.

Redistribution: the web build contains this code compiled to WebAssembly; the licence and this notice
are shipped with it as `licenses/DSP-Seed-Finder-LICENSE.txt` and `licenses/DSP-Seed-Finder-NOTICE.txt`
(`react-frontend/public/licenses/`, linked from the Galaxy tab), as §4(a) asks.

The theme / vein tables inside `data/theme_proto.rs` were exported from the game by the upstream
author; the game data belongs to its publisher (report §2.4).
