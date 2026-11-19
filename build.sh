#!/bin/bash
# Production build: Rust planner -> WASM, then the React front end (output: react-frontend/dist)
set -e
cargo test
wasm-pack build --release --target web --out-dir react-frontend/src/wasm-pkg --out-name satisfactory_calculator
cd react-frontend
npm ci
npm run build
