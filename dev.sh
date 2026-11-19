#!/bin/bash

# Development script for Satisfactory Calculator React + WASM

set -e

echo "Building WASM module in development mode..."
wasm-pack build --dev --target web --out-dir react-frontend/src/wasm-pkg --out-name satisfactory-calculator

echo "Starting development server..."
cd react-frontend
npm install
npm run dev