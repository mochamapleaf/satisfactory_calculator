// WASM module integration layer
// This file will be populated when implementing task 5.2

export interface WasmModule {
  greet(name: string): void;
  // Additional WASM bindings will be added in later tasks
}

let wasmModule: WasmModule | null = null;

export async function initWasm(): Promise<WasmModule> {
  if (wasmModule) {
    return wasmModule;
  }

  try {
    const wasm = await import('../wasm-pkg/satisfactory-calculator');
    wasmModule = wasm as WasmModule;
    return wasmModule;
  } catch (error) {
    console.error('Failed to load WASM module:', error);
    throw error;
  }
}

export function getWasmModule(): WasmModule | null {
  return wasmModule;
}