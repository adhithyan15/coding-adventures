/**
 * @coding-adventures/wasm-runtime
 *
 * WebAssembly 1.0 runtime: parse, validate, instantiate, and execute WASM modules.
 *
 * This is the user-facing entry point for the WASM stack. It composes the parser,
 * validator, and execution engine into a single API.
 *
 * **Quick start:**
 *
 * ```typescript
 * import { WasmRuntime } from "@coding-adventures/wasm-runtime";
 *
 * const runtime = new WasmRuntime();
 * const result = runtime.loadAndRun(wasmBytes, "square", [5]);
 * console.log(result); // [25]
 * ```
 *
 * @module
 */

export const VERSION = "0.1.0";

export { WasmRuntime } from "./wasm_runtime.js";
export { WasiStub, ProcExitError } from "./wasi_stub.js";
export type { WasmInstance } from "./wasm_instance.js";
