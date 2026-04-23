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
 * **With WASI (I/O programs):**
 *
 * ```typescript
 * import { WasmRuntime, WasiHost } from "@coding-adventures/wasm-runtime";
 *
 * const output: string[] = [];
 * const wasi = new WasiHost({
 *   args: ["myapp", "--verbose"],
 *   env: { HOME: "/home/user" },
 *   stdout: (text) => output.push(text),
 * });
 * const runtime = new WasmRuntime(wasi);
 * runtime.loadAndRun(wasmBytes);
 * console.log(output.join(""));
 * ```
 *
 * @module
 */

export const VERSION = "0.1.0";

export { WasmRuntime } from "./wasm_runtime.js";
export {
  WasiHost,
  WasiStub,
  ProcExitError,
  SystemClock,
  SystemRandom,
} from "./wasi_stub.js";
export type {
  WasiClock,
  WasiRandom,
  WasiConfig,
} from "./wasi_stub.js";
export type { WasmInstance } from "./wasm_instance.js";
