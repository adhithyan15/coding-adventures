/**
 * End-to-End Test: square(n) = n * n
 *
 * ==========================================================================
 * The Ultimate Integration Test
 * ==========================================================================
 *
 * This test proves the entire WASM stack works end-to-end:
 *
 *   Raw .wasm bytes → Parse → Validate → Instantiate → Execute → Result
 *
 * The test hand-assembles a minimal WASM module that exports a ``square``
 * function: ``(i32) -> (i32)`` which computes ``n * n``.
 *
 * In WAT (WebAssembly Text Format), this module is:
 *
 * ```wat
 * (module
 *   (type (func (param i32) (result i32)))
 *   (func (export "square") (type 0)
 *     local.get 0
 *     local.get 0
 *     i32.mul)
 * )
 * ```
 *
 * The binary encoding is hand-assembled byte by byte below, following the
 * WASM 1.0 binary format specification.
 *
 * @module
 */

import { describe, it, expect } from "vitest";
import { WasmRuntime } from "../src/wasm_runtime.js";
import { encodeSigned, encodeUnsigned } from "@coding-adventures/wasm-leb128";

// =========================================================================
// Hand-Assembled WASM Module: square(n) = n * n
// =========================================================================

/**
 * Build the square.wasm binary by hand.
 *
 * This is a minimal valid WASM module with:
 * - 1 type:     (i32) -> (i32)
 * - 1 function: uses type 0
 * - 1 export:   "square" -> function 0
 * - 1 code body: local.get 0, local.get 0, i32.mul, end
 */
function buildSquareWasm(): Uint8Array {
  const parts: number[] = [];

  // ── Header ──────────────────────────────────────────────────────────
  // Magic: "\0asm"
  parts.push(0x00, 0x61, 0x73, 0x6D);
  // Version: 1
  parts.push(0x01, 0x00, 0x00, 0x00);

  // ── Type Section (ID 1) ─────────────────────────────────────────────
  // Contains 1 function type: (i32) -> (i32)
  const typePayload = [
    0x01,       // 1 type entry
    0x60,       // function type marker
    0x01, 0x7F, // 1 param: i32
    0x01, 0x7F, // 1 result: i32
  ];
  parts.push(0x01);                           // section ID = 1 (Type)
  parts.push(...encodeUnsigned(typePayload.length)); // section size
  parts.push(...typePayload);

  // ── Function Section (ID 3) ─────────────────────────────────────────
  // Contains 1 function pointing to type index 0
  const funcPayload = [
    0x01,       // 1 function
    0x00,       // type index 0
  ];
  parts.push(0x03);                           // section ID = 3 (Function)
  parts.push(...encodeUnsigned(funcPayload.length));
  parts.push(...funcPayload);

  // ── Export Section (ID 7) ───────────────────────────────────────────
  // Exports "square" as function 0
  const nameBytes = new TextEncoder().encode("square");
  const exportPayload = [
    0x01,                                     // 1 export
    ...encodeUnsigned(nameBytes.length),       // name length
    ...nameBytes,                              // name bytes
    0x00,                                     // export kind: function
    0x00,                                     // function index 0
  ];
  parts.push(0x07);                           // section ID = 7 (Export)
  parts.push(...encodeUnsigned(exportPayload.length));
  parts.push(...exportPayload);

  // ── Code Section (ID 10) ────────────────────────────────────────────
  // Contains 1 function body:
  //   local.get 0    (0x20 0x00)
  //   local.get 0    (0x20 0x00)
  //   i32.mul        (0x6C)
  //   end            (0x0B)
  const bodyCode = [
    0x20, 0x00,   // local.get 0
    0x20, 0x00,   // local.get 0
    0x6C,         // i32.mul
    0x0B,         // end
  ];
  const bodyPayload = [
    0x00,         // 0 local declarations
    ...bodyCode,
  ];
  const funcBody = [
    ...encodeUnsigned(bodyPayload.length),    // body size
    ...bodyPayload,
  ];
  const codePayload = [
    0x01,         // 1 function body
    ...funcBody,
  ];
  parts.push(0x0A);                           // section ID = 10 (Code)
  parts.push(...encodeUnsigned(codePayload.length));
  parts.push(...codePayload);

  return new Uint8Array(parts);
}

// =========================================================================
// Tests
// =========================================================================

describe("square.wasm end-to-end", () => {
  it("should compute square(5) = 25", () => {
    const runtime = new WasmRuntime();
    const wasmBytes = buildSquareWasm();
    const result = runtime.loadAndRun(wasmBytes, "square", [5]);
    expect(result).toEqual([25]);
  });

  it("should compute square(0) = 0", () => {
    const runtime = new WasmRuntime();
    const wasmBytes = buildSquareWasm();
    const result = runtime.loadAndRun(wasmBytes, "square", [0]);
    expect(result).toEqual([0]);
  });

  it("should compute square(-3) = 9", () => {
    const runtime = new WasmRuntime();
    const wasmBytes = buildSquareWasm();
    const result = runtime.loadAndRun(wasmBytes, "square", [-3]);
    expect(result).toEqual([9]);
  });

  it("should handle i32 overflow: square(2147483647) wraps", () => {
    const runtime = new WasmRuntime();
    const wasmBytes = buildSquareWasm();
    const result = runtime.loadAndRun(wasmBytes, "square", [2147483647]);
    // 2147483647 * 2147483647 = 4611686014132420609, which wraps to 1 in i32
    // Math.imul(2147483647, 2147483647) = 1
    expect(result).toEqual([1]);
  });

  it("should support step-by-step: load, validate, instantiate, call", () => {
    const runtime = new WasmRuntime();
    const wasmBytes = buildSquareWasm();

    // Step 1: Parse
    const module = runtime.load(wasmBytes);
    expect(module.types.length).toBe(1);
    expect(module.functions.length).toBe(1);
    expect(module.exports.length).toBe(1);

    // Step 2: Validate
    const validated = runtime.validate(module);
    expect(validated.module).toBe(module);

    // Step 3: Instantiate
    const instance = runtime.instantiate(module);
    expect(instance.exports.has("square")).toBe(true);

    // Step 4: Call
    const result = runtime.call(instance, "square", [7]);
    expect(result).toEqual([49]);
  });
});
