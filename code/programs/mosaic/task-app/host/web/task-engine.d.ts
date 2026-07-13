// Minimal typing for the dependency-free task-wasm JS accessor.
declare module "./task-engine.mjs" {
  export function createTaskEngine(wasmBytes: unknown, options?: unknown): any;
}
