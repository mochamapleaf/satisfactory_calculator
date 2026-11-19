# Project structure (2026-11)

```
data/game-data.json        Recipe/item/building data (Satisfactory 1.0, valid for 1.1) — see data/SOURCE.md
tools/convert_data.py      Docs.json-derived dump -> data/game-data.json
src/planner/               Rust planner (compiled to WASM)
  data.rs                  game data model
  matrix.rs                readme incidence-matrix method: (Aᵀ)⁻¹ b with raw input rows
  lp.rs                    LP generalisation (minilp): multi-recipe, byproducts, limits, objectives,
                           clock speed, Somersloops
  flows.rs                 producer -> consumer flows for the graph view
  tests.rs                 known-ratio tests (cargo test)
src/wasm_api.rs            solve_json / game_data_json for the front end
src/graph.rs, utilities.rs original 2023 resource graph + topological sort
examples/plan.rs           CLI: cargo run --example plan -- '{"targets":[{"item":"Plastic","rate":60}]}'
react-frontend/            React + Vite UI (planner sidebar, plan tables, React Flow graph)
legacy/                    Yew front end and 2025-08 Kiro WASM modules (not compiled)
docs/PROGRESS.md           work log
```

## Build
- Toolchain: `rust-toolchain.toml` pins Rust 1.91.0 + wasm32 target; install wasm-pack (`cargo install wasm-pack`).
- `./dev.sh` — WASM (dev) + Vite dev server.
- `./build.sh` — tests, release WASM, `react-frontend/dist` (static site).
- CI: `.github/workflows/build-site.yml` builds and deploys `react-frontend/dist` to GitHub Pages on push to master.
