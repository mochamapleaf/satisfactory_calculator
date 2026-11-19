#!/bin/bash

# Build script for Satisfactory Calculator React + WASM

set -e

echo "Building WASM module..."
wasm-pack build --target web --out-dir react-frontend/src/wasm-pkg --out-name satisfactory-calculator

echo "Installing frontend dependencies..."
cd react-frontend
npm install

echo "Building React frontend..."
npm run build

echo "Build complete! Output in react-frontend/dist/"