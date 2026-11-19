#!/bin/bash
# Dev: build WASM once, then run the Vite dev server (re-run build:wasm after Rust changes)
set -e
wasm-pack build --dev --target web --out-dir react-frontend/src/wasm-pkg --out-name satisfactory_calculator
cd react-frontend
npm install
npm run dev
