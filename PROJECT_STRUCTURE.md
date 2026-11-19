# Satisfactory Calculator - Project Structure

## Overview

This project combines a Rust WASM backend with a React TypeScript frontend for the Satisfactory Production Chain Optimizer.

## Directory Structure

```
├── src/                          # Rust WASM core engine
│   ├── lib.rs                   # WASM entry point and exports
│   ├── graph.rs                 # Graph engine (existing)
│   ├── utilities.rs             # Utilities (existing)
│   └── tests.rs                 # Tests (existing)
├── react-frontend/              # React TypeScript frontend
│   ├── src/
│   │   ├── components/          # React components
│   │   │   ├── App.tsx         # Main application component
│   │   │   └── index.ts        # Component exports
│   │   ├── wasm/               # WASM integration layer
│   │   │   └── index.ts        # WASM module loader
│   │   ├── types/              # TypeScript type definitions
│   │   │   └── index.ts        # Application types
│   │   ├── main.tsx            # React entry point
│   │   └── index.css           # Global styles
│   ├── public/                 # Static assets
│   ├── package.json            # Frontend dependencies and scripts
│   └── vite.config.ts          # Vite configuration with WASM support
├── build.sh                    # Production build script
├── dev.sh                      # Development script
├── Cargo.toml                  # Rust project configuration
└── PROJECT_STRUCTURE.md        # This file
```

## Build System

### Development
```bash
./dev.sh
```
This will:
1. Build the WASM module in development mode
2. Install frontend dependencies
3. Start the Vite development server

### Production
```bash
./build.sh
```
This will:
1. Build the optimized WASM module
2. Install frontend dependencies
3. Build the React application for production

### Manual WASM Build
```bash
# Development build
wasm-pack build --dev --target web --out-dir react-frontend/src/wasm-pkg --out-name satisfactory-calculator

# Production build
wasm-pack build --target web --out-dir react-frontend/src/wasm-pkg --out-name satisfactory-calculator
```

## Integration Points

- WASM module is built to `react-frontend/src/wasm-pkg/`
- React frontend loads WASM through `src/wasm/index.ts`
- TypeScript types are defined in `src/types/`
- Components are organized in `src/components/`

## Next Steps

This structure is ready for implementing the remaining tasks:
- Task 2: Migrate core Rust graph engine to WASM module
- Task 3: Implement enhanced data management system
- Task 4: Implement discrete optimization engine
- Task 5: Create React frontend foundation
- And subsequent tasks...