// Minimal typing for the dependency-free task-wasm JS accessor. The real module
// (task-engine.mjs) is copied in from task-wasm by scripts/build-web.
declare module "./task-engine.mjs" {
  export function createTaskEngine(wasmBytes: unknown, options?: unknown): any;
}
