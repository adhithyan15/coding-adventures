/**
 * @coding-adventures/wasm-module-parser
 *
 * WASM binary module parser: decodes .wasm files into structured WasmModule data
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 *
 * Public API:
 *   WasmModuleParser  — the main parser class (call .parse(Uint8Array))
 *   WasmParseError    — error class for malformed input (has .offset field)
 *
 * Example:
 *   import { WasmModuleParser } from "@coding-adventures/wasm-module-parser";
 *   const parser = new WasmModuleParser();
 *   const module = parser.parse(wasmBytes);
 *   console.log(module.exports);  // see all exports
 */

export const VERSION = "0.1.0";

export { WasmModuleParser, WasmParseError } from "./wasm_module_parser.js";
