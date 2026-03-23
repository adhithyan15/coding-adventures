/**
 * wasm_module_parser.test.ts — Comprehensive tests for WasmModuleParser
 *
 * Tests cover:
 *   1.  Minimal module (header only)
 *   2.  Type section: (i32, i32) → i32
 *   3.  Function section: type indices
 *   4.  Export section: function export
 *   5.  Code section: function with locals and instructions
 *   6.  Import section: function import
 *   7.  Memory section
 *   8.  Table section
 *   9.  Global section (immutable i32)
 *   10. Data section
 *   11. Element section
 *   12. Start section
 *   13. Custom section (name + data)
 *   14. Multi-section module (round-trip)
 *   15. Error: bad magic bytes
 *   16. Error: wrong version
 *   17. Error: truncated header
 *   18. Error: truncated section payload
 *   19. Round-trip: build binary manually, parse, verify all fields
 *
 * ─────────────────────────────────────────────────────────────────────────────
 * TEST HELPER UTILITIES
 * ─────────────────────────────────────────────────────────────────────────────
 *
 * We build test .wasm binaries by hand to avoid any dependency on a real
 * compiler. This also doubles as documentation of the binary format.
 */

import { describe, it, expect } from "vitest";
import { WasmModuleParser, WasmParseError } from "../src/index.js";
import { VERSION } from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Test Helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * encodeUleb128 — encode a non-negative integer as ULEB128.
 *
 * Used to build section headers and count fields in test binaries.
 *
 * Algorithm:
 *   do { byte = value & 0x7F; value >>= 7; if (value) byte |= 0x80; emit } while (value)
 */
function encodeUleb128(n: number): number[] {
  const bytes: number[] = [];
  let remaining = n >>> 0; // treat as u32
  do {
    let byte = remaining & 0x7f;
    remaining = remaining >>> 7;
    if (remaining !== 0) byte |= 0x80;
    bytes.push(byte);
  } while (remaining !== 0);
  return bytes;
}

/**
 * makeSection — build a WASM section with its header.
 *
 * Structure: id:u8 + size:u32leb + payload:bytes
 *
 * The size is the byte count of the payload. We compute it automatically.
 */
function makeSection(id: number, payload: number[]): number[] {
  const sizeBytes = encodeUleb128(payload.length);
  return [id, ...sizeBytes, ...payload];
}

/**
 * makeString — encode a WASM name string: length:u32leb + UTF-8 bytes.
 */
function makeString(s: string): number[] {
  const encoded = new TextEncoder().encode(s);
  return [...encodeUleb128(encoded.length), ...encoded];
}

/**
 * WASM_HEADER — the 8-byte header that starts every valid .wasm file.
 *
 *   Magic:   \0asm = [0x00, 0x61, 0x73, 0x6D]
 *   Version: 1     = [0x01, 0x00, 0x00, 0x00] (little-endian uint32)
 */
const WASM_HEADER = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

/**
 * makeWasm — combine header + sections into a complete .wasm binary.
 */
function makeWasm(...sections: number[][]): Uint8Array {
  const all = sections.reduce((acc, s) => [...acc, ...s], WASM_HEADER);
  return new Uint8Array(all);
}

// ─────────────────────────────────────────────────────────────────────────────
// Value type constants (matching WASM spec)
// ─────────────────────────────────────────────────────────────────────────────

const I32 = 0x7f;
const I64 = 0x7e;
const F32 = 0x7d;
const F64 = 0x7c;
const FUNCREF = 0x70;

// Section IDs
const SEC_CUSTOM = 0;
const SEC_TYPE = 1;
const SEC_IMPORT = 2;
const SEC_FUNCTION = 3;
const SEC_TABLE = 4;
const SEC_MEMORY = 5;
const SEC_GLOBAL = 6;
const SEC_EXPORT = 7;
const SEC_START = 8;
const SEC_ELEMENT = 9;
const SEC_CODE = 10;
const SEC_DATA = 11;

// External kind bytes
const KIND_FUNC = 0x00;
const KIND_TABLE = 0x01;
const KIND_MEMORY = 0x02;
const KIND_GLOBAL = 0x03;

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

const parser = new WasmModuleParser();

describe("wasm-module-parser", () => {
  // ── Version ──────────────────────────────────────────────────────────────

  it("exports VERSION 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });

  // ── Test 1: Minimal module ────────────────────────────────────────────────

  it("1. parses a minimal module (header only)", () => {
    const wasm = new Uint8Array(WASM_HEADER);
    const module = parser.parse(wasm);
    // All section arrays should be empty, start should be null
    expect(module.types).toHaveLength(0);
    expect(module.imports).toHaveLength(0);
    expect(module.functions).toHaveLength(0);
    expect(module.tables).toHaveLength(0);
    expect(module.memories).toHaveLength(0);
    expect(module.globals).toHaveLength(0);
    expect(module.exports).toHaveLength(0);
    expect(module.start).toBeNull();
    expect(module.elements).toHaveLength(0);
    expect(module.code).toHaveLength(0);
    expect(module.data).toHaveLength(0);
    expect(module.customs).toHaveLength(0);
  });

  // ── Test 2: Type section ──────────────────────────────────────────────────

  it("2. parses type section: function type (i32, i32) → i32", () => {
    // Type section payload:
    //   count=1
    //   type[0]: 0x60 + params=[i32,i32] + results=[i32]
    const typePayload = [
      1, // count = 1
      0x60, // function type prefix
      2, I32, I32, // param_count=2, i32, i32
      1, I32, // result_count=1, i32
    ];
    const wasm = makeWasm(makeSection(SEC_TYPE, typePayload));
    const module = parser.parse(wasm);

    expect(module.types).toHaveLength(1);
    expect(module.types[0].params).toEqual([I32, I32]);
    expect(module.types[0].results).toEqual([I32]);
  });

  it("2b. parses type section: multiple types", () => {
    const typePayload = [
      2, // count = 2
      0x60, 0, 0, // () → ()
      0x60, 1, I64, 1, F64, // (i64) → f64
    ];
    const wasm = makeWasm(makeSection(SEC_TYPE, typePayload));
    const module = parser.parse(wasm);

    expect(module.types).toHaveLength(2);
    expect(module.types[0].params).toEqual([]);
    expect(module.types[0].results).toEqual([]);
    expect(module.types[1].params).toEqual([I64]);
    expect(module.types[1].results).toEqual([F64]);
  });

  // ── Test 3: Function section ──────────────────────────────────────────────

  it("3. parses function section: type indices", () => {
    // Type section: one type () → ()
    const typePayload = [1, 0x60, 0, 0];
    // Function section: two functions both referencing type 0
    const funcPayload = [
      2, // count=2
      0, // function 0 → type index 0
      0, // function 1 → type index 0
    ];
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_FUNCTION, funcPayload)
    );
    const module = parser.parse(wasm);

    expect(module.functions).toHaveLength(2);
    expect(module.functions[0]).toBe(0);
    expect(module.functions[1]).toBe(0);
  });

  // ── Test 4: Export section ────────────────────────────────────────────────

  it("4. parses export section: function export named 'main'", () => {
    // Type: () → ()
    const typePayload = [1, 0x60, 0, 0];
    // Function section: 1 function, type 0
    const funcPayload = [1, 0];
    // Export section: export function at index 0 as "main"
    const exportPayload = [
      1, // count=1
      ...makeString("main"),
      KIND_FUNC, // kind=function
      0, // index=0
    ];
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_FUNCTION, funcPayload),
      makeSection(SEC_EXPORT, exportPayload)
    );
    const module = parser.parse(wasm);

    expect(module.exports).toHaveLength(1);
    expect(module.exports[0].name).toBe("main");
    expect(module.exports[0].kind).toBe(0); // ExternalKind.FUNCTION
    expect(module.exports[0].index).toBe(0);
  });

  // ── Test 5: Code section ──────────────────────────────────────────────────

  it("5. parses code section: function with locals and instructions", () => {
    // Type: (i32, i32) → i32
    const typePayload = [1, 0x60, 2, I32, I32, 1, I32];
    // Function: 1 function using type 0
    const funcPayload = [1, 0];
    // Code body:
    //   1 local group: 1 × i32
    //   code: local.get 0; local.get 1; i32.add; end
    const bodyBytes = [
      1, // local_decl_count = 1
      1, I32, // 1 × i32 local
      0x20, 0x00, // local.get 0
      0x20, 0x01, // local.get 1
      0x6a, // i32.add
      0x0b, // end
    ];
    const codePayload = [1, ...encodeUleb128(bodyBytes.length), ...bodyBytes];
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_FUNCTION, funcPayload),
      makeSection(SEC_CODE, codePayload)
    );
    const module = parser.parse(wasm);

    expect(module.code).toHaveLength(1);
    expect(module.code[0].locals).toEqual([I32]);
    // code should be: [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]
    expect(Array.from(module.code[0].code)).toEqual([0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b]);
  });

  it("5b. code section: multiple locals of different types", () => {
    const typePayload = [1, 0x60, 0, 0];
    const funcPayload = [1, 0];
    // 2 local groups: 2×i32, 1×f64
    const bodyBytes = [
      2, // 2 local groups
      2, I32, // 2 × i32
      1, F64, // 1 × f64
      0x0b, // end (empty body)
    ];
    const codePayload = [1, ...encodeUleb128(bodyBytes.length), ...bodyBytes];
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_FUNCTION, funcPayload),
      makeSection(SEC_CODE, codePayload)
    );
    const module = parser.parse(wasm);

    expect(module.code[0].locals).toEqual([I32, I32, F64]);
  });

  // ── Test 6: Import section ────────────────────────────────────────────────

  it("6. parses import section: function import", () => {
    // Type: () → ()
    const typePayload = [1, 0x60, 0, 0];
    // Import: "env"."print" as function type 0
    const importPayload = [
      1, // count=1
      ...makeString("env"),
      ...makeString("print"),
      KIND_FUNC, // kind=function
      0, // type_index=0
    ];
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_IMPORT, importPayload)
    );
    const module = parser.parse(wasm);

    expect(module.imports).toHaveLength(1);
    expect(module.imports[0].moduleName).toBe("env");
    expect(module.imports[0].name).toBe("print");
    expect(module.imports[0].kind).toBe(0); // FUNCTION
    expect(module.imports[0].typeInfo).toBe(0); // type index 0
  });

  it("6b. parses import section: memory import", () => {
    const importPayload = [
      1, // count=1
      ...makeString("env"),
      ...makeString("memory"),
      KIND_MEMORY, // kind=memory
      0x00, 1, // limits: flags=0 (no max), min=1
    ];
    const wasm = makeWasm(makeSection(SEC_IMPORT, importPayload));
    const module = parser.parse(wasm);

    expect(module.imports[0].kind).toBe(2); // MEMORY
    const typeInfo = module.imports[0].typeInfo as { limits: { min: number; max: number | null } };
    expect(typeInfo.limits.min).toBe(1);
    expect(typeInfo.limits.max).toBeNull();
  });

  it("6c. parses import section: table import", () => {
    const importPayload = [
      1,
      ...makeString("env"),
      ...makeString("table"),
      KIND_TABLE,
      FUNCREF, // element type
      0x00, 10, // limits: flags=0, min=10
    ];
    const wasm = makeWasm(makeSection(SEC_IMPORT, importPayload));
    const module = parser.parse(wasm);

    expect(module.imports[0].kind).toBe(1); // TABLE
    const typeInfo = module.imports[0].typeInfo as { elementType: number; limits: { min: number; max: number | null } };
    expect(typeInfo.elementType).toBe(FUNCREF);
    expect(typeInfo.limits.min).toBe(10);
  });

  it("6d. parses import section: global import", () => {
    const importPayload = [
      1,
      ...makeString("env"),
      ...makeString("stackPointer"),
      KIND_GLOBAL,
      I32, // value type
      0x01, // mutable = true
    ];
    const wasm = makeWasm(makeSection(SEC_IMPORT, importPayload));
    const module = parser.parse(wasm);

    expect(module.imports[0].kind).toBe(3); // GLOBAL
    const typeInfo = module.imports[0].typeInfo as { valueType: number; mutable: boolean };
    expect(typeInfo.valueType).toBe(I32);
    expect(typeInfo.mutable).toBe(true);
  });

  // ── Test 7: Memory section ────────────────────────────────────────────────

  it("7. parses memory section: 1 memory, min=1, no max", () => {
    const memPayload = [
      1, // count=1
      0x00, // flags: no max
      1, // min=1 page (64 KiB)
    ];
    const wasm = makeWasm(makeSection(SEC_MEMORY, memPayload));
    const module = parser.parse(wasm);

    expect(module.memories).toHaveLength(1);
    expect(module.memories[0].limits.min).toBe(1);
    expect(module.memories[0].limits.max).toBeNull();
  });

  it("7b. parses memory section: min=2, max=4", () => {
    const memPayload = [1, 0x01, 2, 4]; // flags=1 (has max), min=2, max=4
    const wasm = makeWasm(makeSection(SEC_MEMORY, memPayload));
    const module = parser.parse(wasm);

    expect(module.memories[0].limits.min).toBe(2);
    expect(module.memories[0].limits.max).toBe(4);
  });

  // ── Test 8: Table section ─────────────────────────────────────────────────

  it("8. parses table section: 1 table, funcref, min=10", () => {
    const tablePayload = [
      1, // count=1
      FUNCREF, // element_type=funcref
      0x00, // limits flags: no max
      10, // min=10
    ];
    const wasm = makeWasm(makeSection(SEC_TABLE, tablePayload));
    const module = parser.parse(wasm);

    expect(module.tables).toHaveLength(1);
    expect(module.tables[0].elementType).toBe(FUNCREF);
    expect(module.tables[0].limits.min).toBe(10);
    expect(module.tables[0].limits.max).toBeNull();
  });

  // ── Test 9: Global section ────────────────────────────────────────────────

  it("9. parses global section: immutable i32 global with value 42", () => {
    // i32.const 42; end = [0x41, 0x2A, 0x0B]
    const globalPayload = [
      1, // count=1
      I32, // value_type = i32
      0x00, // mutable = false
      0x41, 42, 0x0b, // init_expr: i32.const 42; end
    ];
    const wasm = makeWasm(makeSection(SEC_GLOBAL, globalPayload));
    const module = parser.parse(wasm);

    expect(module.globals).toHaveLength(1);
    expect(module.globals[0].globalType.valueType).toBe(I32);
    expect(module.globals[0].globalType.mutable).toBe(false);
    expect(Array.from(module.globals[0].initExpr)).toEqual([0x41, 42, 0x0b]);
  });

  it("9b. parses global section: mutable i64 global", () => {
    const globalPayload = [
      1,
      I64, // i64
      0x01, // mutable
      0x42, 0x00, 0x0b, // i64.const 0; end
    ];
    const wasm = makeWasm(makeSection(SEC_GLOBAL, globalPayload));
    const module = parser.parse(wasm);

    expect(module.globals[0].globalType.valueType).toBe(I64);
    expect(module.globals[0].globalType.mutable).toBe(true);
  });

  // ── Test 10: Data section ─────────────────────────────────────────────────

  it("10. parses data section: write 'Hi' into memory 0 at address 0", () => {
    const dataPayload = [
      1, // count=1
      0, // memory_index=0
      0x41, 0x00, 0x0b, // offset_expr: i32.const 0; end
      2, // byte_count=2
      0x48, 0x69, // "Hi"
    ];
    const wasm = makeWasm(makeSection(SEC_DATA, dataPayload));
    const module = parser.parse(wasm);

    expect(module.data).toHaveLength(1);
    expect(module.data[0].memoryIndex).toBe(0);
    expect(Array.from(module.data[0].offsetExpr)).toEqual([0x41, 0x00, 0x0b]);
    expect(Array.from(module.data[0].data)).toEqual([0x48, 0x69]);
  });

  // ── Test 11: Element section ──────────────────────────────────────────────

  it("11. parses element section: fill table 0 at slot 0 with functions [0, 1]", () => {
    const elemPayload = [
      1, // count=1
      0, // table_index=0
      0x41, 0x00, 0x0b, // offset_expr: i32.const 0; end
      2, // func_count=2
      0, // func_index[0]=0
      1, // func_index[1]=1
    ];
    const wasm = makeWasm(makeSection(SEC_ELEMENT, elemPayload));
    const module = parser.parse(wasm);

    expect(module.elements).toHaveLength(1);
    expect(module.elements[0].tableIndex).toBe(0);
    expect(Array.from(module.elements[0].offsetExpr)).toEqual([0x41, 0x00, 0x0b]);
    expect(Array.from(module.elements[0].functionIndices)).toEqual([0, 1]);
  });

  // ── Test 12: Start section ────────────────────────────────────────────────

  it("12. parses start section: function index 2", () => {
    const startPayload = [2]; // function_index = 2
    const wasm = makeWasm(makeSection(SEC_START, startPayload));
    const module = parser.parse(wasm);

    expect(module.start).toBe(2);
  });

  it("12b. start is null when start section is absent", () => {
    const wasm = new Uint8Array(WASM_HEADER);
    const module = parser.parse(wasm);
    expect(module.start).toBeNull();
  });

  // ── Test 13: Custom section ───────────────────────────────────────────────

  it("13. parses custom section: name + arbitrary data", () => {
    // Custom section: name="name", data=[0x01, 0x02, 0x03]
    const customPayload = [
      ...makeString("name"),
      0x01, 0x02, 0x03, // arbitrary payload
    ];
    const wasm = makeWasm(makeSection(SEC_CUSTOM, customPayload));
    const module = parser.parse(wasm);

    expect(module.customs).toHaveLength(1);
    expect(module.customs[0].name).toBe("name");
    expect(Array.from(module.customs[0].data)).toEqual([0x01, 0x02, 0x03]);
  });

  it("13b. multiple custom sections are all collected", () => {
    const custom1 = makeSection(SEC_CUSTOM, [...makeString("debug"), 0xAA]);
    const custom2 = makeSection(SEC_CUSTOM, [...makeString("producers"), 0xBB, 0xCC]);
    const wasm = makeWasm(custom1, custom2);
    const module = parser.parse(wasm);

    expect(module.customs).toHaveLength(2);
    expect(module.customs[0].name).toBe("debug");
    expect(module.customs[1].name).toBe("producers");
  });

  // ── Test 14: Multi-section module ─────────────────────────────────────────

  it("14. parses multi-section module: type + import + function + export + code", () => {
    // Type: (i32) → i32
    const typePayload = [1, 0x60, 1, I32, 1, I32];
    // Import: "env"."add" as function type 0
    const importPayload = [1, ...makeString("env"), ...makeString("add"), KIND_FUNC, 0];
    // Function: 1 local function at type 0
    const funcPayload = [1, 0];
    // Export: export local function (index = 1, since 1 imported func has index 0) as "double"
    const exportPayload = [1, ...makeString("double"), KIND_FUNC, 1];
    // Code: body with no locals, just: local.get 0; call 0; end
    const bodyBytes = [
      0, // 0 local groups
      0x20, 0x00, // local.get 0
      0x10, 0x00, // call 0 (the imported "add")
      0x0b, // end
    ];
    const codePayload = [1, ...encodeUleb128(bodyBytes.length), ...bodyBytes];

    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_IMPORT, importPayload),
      makeSection(SEC_FUNCTION, funcPayload),
      makeSection(SEC_EXPORT, exportPayload),
      makeSection(SEC_CODE, codePayload)
    );
    const module = parser.parse(wasm);

    expect(module.types).toHaveLength(1);
    expect(module.imports).toHaveLength(1);
    expect(module.functions).toHaveLength(1);
    expect(module.exports).toHaveLength(1);
    expect(module.code).toHaveLength(1);
    expect(module.exports[0].name).toBe("double");
  });

  // ── Test 15: Error: bad magic ─────────────────────────────────────────────

  it("15. throws WasmParseError on bad magic bytes", () => {
    const badMagic = new Uint8Array([0x00, 0x61, 0x73, 0x6e, 0x01, 0x00, 0x00, 0x00]); // 'n' not 'm'
    expect(() => parser.parse(badMagic)).toThrow(WasmParseError);
    try {
      parser.parse(badMagic);
    } catch (e) {
      expect(e).toBeInstanceOf(WasmParseError);
      expect((e as WasmParseError).offset).toBe(3);
    }
  });

  it("15b. throws WasmParseError with correct offset for first wrong byte", () => {
    const badMagic = new Uint8Array([0xFF, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
    let caught: WasmParseError | null = null;
    try {
      parser.parse(badMagic);
    } catch (e) {
      caught = e as WasmParseError;
    }
    expect(caught).not.toBeNull();
    expect(caught!.offset).toBe(0);
  });

  // ── Test 16: Error: wrong version ─────────────────────────────────────────

  it("16. throws WasmParseError on wrong version bytes", () => {
    const wrongVersion = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x02, 0x00, 0x00, 0x00]); // version=2
    expect(() => parser.parse(wrongVersion)).toThrow(WasmParseError);
    try {
      parser.parse(wrongVersion);
    } catch (e) {
      expect((e as WasmParseError).offset).toBe(4);
    }
  });

  // ── Test 17: Error: truncated header ─────────────────────────────────────

  it("17. throws WasmParseError on truncated header (< 8 bytes)", () => {
    const truncated = new Uint8Array([0x00, 0x61, 0x73, 0x6d]); // only 4 bytes
    expect(() => parser.parse(truncated)).toThrow(WasmParseError);
    try {
      parser.parse(truncated);
    } catch (e) {
      expect(e).toBeInstanceOf(WasmParseError);
      expect((e as WasmParseError).offset).toBe(0);
    }
  });

  it("17b. throws on completely empty input", () => {
    expect(() => parser.parse(new Uint8Array([]))).toThrow(WasmParseError);
  });

  // ── Test 18: Error: truncated section ────────────────────────────────────

  it("18. throws WasmParseError on section whose payload extends beyond end of data", () => {
    // Build a module with a type section that claims to be 100 bytes but isn't
    const truncated = new Uint8Array([
      ...WASM_HEADER,
      SEC_TYPE, // section id
      100, // payload size = 100 (lies)
      1, // only 1 byte of actual payload
    ]);
    expect(() => parser.parse(truncated)).toThrow(WasmParseError);
  });

  // ── Test 19: Round-trip test ──────────────────────────────────────────────

  it("19. round-trip: build full binary manually, parse, verify all fields", () => {
    // We'll build a module with:
    //   - 1 type: (i32) → i32
    //   - 1 import: "env"."abort" : func type 0
    //   - 1 function: type 0 (local function index = 1 in overall space)
    //   - 1 table: funcref, min=1, max=null
    //   - 1 memory: min=1
    //   - 1 global: immutable i32 = 0
    //   - 1 export: "identity" = function 1 (the local one)
    //   - start: function 1
    //   - 1 element: table 0, offset=0, funcs=[1]
    //   - 1 code body: no locals, local.get 0; end
    //   - 1 data segment: mem 0, offset=0, bytes=[0x42]
    //   - 1 custom section: "debug", data=[0xDE, 0xAD]

    const typePayload = [1, 0x60, 1, I32, 1, I32];
    const importPayload = [1, ...makeString("env"), ...makeString("abort"), KIND_FUNC, 0];
    const funcPayload = [1, 0]; // 1 local function using type 0
    const tablePayload = [1, FUNCREF, 0x00, 1]; // funcref, flags=0, min=1
    const memPayload = [1, 0x00, 1]; // flags=0, min=1
    const globalPayload = [1, I32, 0x00, 0x41, 0x00, 0x0b]; // immutable i32 = 0
    const exportPayload = [1, ...makeString("identity"), KIND_FUNC, 1];
    const startPayload = [1]; // start = function index 1
    const elemPayload = [1, 0, 0x41, 0x00, 0x0b, 1, 1]; // table 0, offset=0, [1]
    const bodyBytes = [0, 0x20, 0x00, 0x0b]; // 0 locals; local.get 0; end
    const codePayload = [1, ...encodeUleb128(bodyBytes.length), ...bodyBytes];
    const dataPayload = [1, 0, 0x41, 0x00, 0x0b, 1, 0x42]; // mem 0, offset 0, [0x42]
    const customPayload = [...makeString("debug"), 0xde, 0xad];

    const wasm = makeWasm(
      makeSection(SEC_CUSTOM, customPayload), // custom before type section (allowed)
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_IMPORT, importPayload),
      makeSection(SEC_FUNCTION, funcPayload),
      makeSection(SEC_TABLE, tablePayload),
      makeSection(SEC_MEMORY, memPayload),
      makeSection(SEC_GLOBAL, globalPayload),
      makeSection(SEC_EXPORT, exportPayload),
      makeSection(SEC_START, startPayload),
      makeSection(SEC_ELEMENT, elemPayload),
      makeSection(SEC_CODE, codePayload),
      makeSection(SEC_DATA, dataPayload)
    );

    const module = parser.parse(wasm);

    // Types
    expect(module.types).toHaveLength(1);
    expect(module.types[0].params).toEqual([I32]);
    expect(module.types[0].results).toEqual([I32]);

    // Imports
    expect(module.imports).toHaveLength(1);
    expect(module.imports[0].moduleName).toBe("env");
    expect(module.imports[0].name).toBe("abort");
    expect(module.imports[0].kind).toBe(0);

    // Functions
    expect(module.functions).toHaveLength(1);
    expect(module.functions[0]).toBe(0);

    // Tables
    expect(module.tables).toHaveLength(1);
    expect(module.tables[0].elementType).toBe(FUNCREF);
    expect(module.tables[0].limits.min).toBe(1);
    expect(module.tables[0].limits.max).toBeNull();

    // Memories
    expect(module.memories).toHaveLength(1);
    expect(module.memories[0].limits.min).toBe(1);
    expect(module.memories[0].limits.max).toBeNull();

    // Globals
    expect(module.globals).toHaveLength(1);
    expect(module.globals[0].globalType.valueType).toBe(I32);
    expect(module.globals[0].globalType.mutable).toBe(false);
    expect(Array.from(module.globals[0].initExpr)).toEqual([0x41, 0x00, 0x0b]);

    // Exports
    expect(module.exports).toHaveLength(1);
    expect(module.exports[0].name).toBe("identity");
    expect(module.exports[0].index).toBe(1);

    // Start
    expect(module.start).toBe(1);

    // Elements
    expect(module.elements).toHaveLength(1);
    expect(module.elements[0].tableIndex).toBe(0);
    expect(Array.from(module.elements[0].functionIndices)).toEqual([1]);

    // Code
    expect(module.code).toHaveLength(1);
    expect(module.code[0].locals).toHaveLength(0);
    expect(Array.from(module.code[0].code)).toEqual([0x20, 0x00, 0x0b]);

    // Data
    expect(module.data).toHaveLength(1);
    expect(module.data[0].memoryIndex).toBe(0);
    expect(Array.from(module.data[0].data)).toEqual([0x42]);

    // Customs
    expect(module.customs).toHaveLength(1);
    expect(module.customs[0].name).toBe("debug");
    expect(Array.from(module.customs[0].data)).toEqual([0xde, 0xad]);
  });

  // ── Additional coverage ───────────────────────────────────────────────────

  it("WasmParseError has correct name property", () => {
    const err = new WasmParseError("test", 42);
    expect(err.name).toBe("WasmParseError");
    expect(err.message).toBe("test");
    expect(err.offset).toBe(42);
    expect(err).toBeInstanceOf(Error);
  });

  it("handles empty sections gracefully (zero-count vectors)", () => {
    // Type section with 0 types
    const wasm = makeWasm(makeSection(SEC_TYPE, [0]));
    const module = parser.parse(wasm);
    expect(module.types).toHaveLength(0);
  });

  it("handles LEB128-encoded count > 127 (multi-byte)", () => {
    // Build 200 trivial () → () types
    const types: number[] = [];
    for (let i = 0; i < 200; i++) {
      types.push(0x60, 0, 0);
    }
    // 200 in ULEB128 = [0xC8, 0x01]
    const typePayload = [0xc8, 0x01, ...types];
    const wasm = makeWasm(makeSection(SEC_TYPE, typePayload));
    const module = parser.parse(wasm);
    expect(module.types).toHaveLength(200);
  });

  it("parses all four value types in function signatures", () => {
    const typePayload = [
      1,
      0x60,
      4, I32, I64, F32, F64, // params: i32, i64, f32, f64
      4, F64, F32, I64, I32, // results: f64, f32, i64, i32
    ];
    const wasm = makeWasm(makeSection(SEC_TYPE, typePayload));
    const module = parser.parse(wasm);
    expect(module.types[0].params).toEqual([I32, I64, F32, F64]);
    expect(module.types[0].results).toEqual([F64, F32, I64, I32]);
  });

  it("exports all four export kinds", () => {
    const tablePayload = [1, FUNCREF, 0x00, 1];
    const memPayload = [1, 0x00, 1];
    const globalPayload = [1, I32, 0x00, 0x41, 0x00, 0x0b];
    const typePayload = [1, 0x60, 0, 0];
    const funcPayload = [1, 0];
    const bodyBytes = [0, 0x0b];
    const codePayload = [1, ...encodeUleb128(bodyBytes.length), ...bodyBytes];
    const exportPayload = [
      4,
      ...makeString("fn"), KIND_FUNC, 0,
      ...makeString("tbl"), KIND_TABLE, 0,
      ...makeString("mem"), KIND_MEMORY, 0,
      ...makeString("glb"), KIND_GLOBAL, 0,
    ];
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_FUNCTION, funcPayload),
      makeSection(SEC_TABLE, tablePayload),
      makeSection(SEC_MEMORY, memPayload),
      makeSection(SEC_GLOBAL, globalPayload),
      makeSection(SEC_EXPORT, exportPayload),
      makeSection(SEC_CODE, codePayload)
    );
    const module = parser.parse(wasm);
    expect(module.exports).toHaveLength(4);
    expect(module.exports[0].kind).toBe(0); // FUNCTION
    expect(module.exports[1].kind).toBe(1); // TABLE
    expect(module.exports[2].kind).toBe(2); // MEMORY
    expect(module.exports[3].kind).toBe(3); // GLOBAL
  });

  it("custom section with empty data", () => {
    const customPayload = [...makeString("empty")];
    const wasm = makeWasm(makeSection(SEC_CUSTOM, customPayload));
    const module = parser.parse(wasm);
    expect(module.customs[0].name).toBe("empty");
    expect(module.customs[0].data).toHaveLength(0);
  });

  it("memory with both min and max (limits flags=1)", () => {
    const memPayload = [1, 0x01, 2, 8]; // flags=1, min=2, max=8
    const wasm = makeWasm(makeSection(SEC_MEMORY, memPayload));
    const module = parser.parse(wasm);
    expect(module.memories[0].limits.min).toBe(2);
    expect(module.memories[0].limits.max).toBe(8);
  });

  it("throws WasmParseError when sections appear out of order", () => {
    // Export section (7) followed by Function section (3) — out of order
    const typePayload = [1, 0x60, 0, 0];
    const funcPayload = [1, 0];
    const exportPayload = [1, ...makeString("fn"), KIND_FUNC, 0];
    // Manually put export before function (wrong order: 7 then 3)
    const bytes = new Uint8Array([
      ...WASM_HEADER,
      ...makeSection(SEC_TYPE, typePayload),
      ...makeSection(SEC_EXPORT, exportPayload), // id=7
      ...makeSection(SEC_FUNCTION, funcPayload), // id=3 after id=7 → error
    ]);
    expect(() => parser.parse(bytes)).toThrow(WasmParseError);
  });

  it("silently skips unknown section IDs (forward-compat)", () => {
    // Section ID 99 is not a valid WASM 1.0 section; parser should skip it
    const unknownSection = makeSection(99, [0xAA, 0xBB]);
    const wasm = makeWasm(unknownSection);
    const module = parser.parse(wasm); // should NOT throw
    // No standard sections populated
    expect(module.types).toHaveLength(0);
    expect(module.customs).toHaveLength(0);
  });

  it("throws WasmParseError on invalid type prefix (not 0x60)", () => {
    const typePayload = [1, 0x61, 0, 0]; // 0x61 instead of 0x60
    const wasm = makeWasm(makeSection(SEC_TYPE, typePayload));
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });

  it("throws WasmParseError on invalid value type byte in params", () => {
    const typePayload = [1, 0x60, 1, 0x00, 0]; // 0x00 is not a valid value type
    const wasm = makeWasm(makeSection(SEC_TYPE, typePayload));
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });

  it("throws WasmParseError on invalid export kind", () => {
    const typePayload = [1, 0x60, 0, 0];
    const funcPayload = [1, 0];
    const exportPayload = [1, ...makeString("fn"), 0x99, 0]; // 0x99 invalid kind
    const bodyBytes = [0, 0x0b];
    const codePayload = [1, ...encodeUleb128(bodyBytes.length), ...bodyBytes];
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_FUNCTION, funcPayload),
      makeSection(SEC_EXPORT, exportPayload),
      makeSection(SEC_CODE, codePayload)
    );
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });

  it("throws WasmParseError on invalid import kind", () => {
    const typePayload = [1, 0x60, 0, 0];
    const importPayload = [1, ...makeString("env"), ...makeString("x"), 0x99, 0]; // 0x99 invalid
    const wasm = makeWasm(
      makeSection(SEC_TYPE, typePayload),
      makeSection(SEC_IMPORT, importPayload)
    );
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });

  it("throws WasmParseError on init expr with no end opcode (truncated)", () => {
    // Global with init expr that never has 0x0B
    const globalPayload = [
      1,
      I32, 0x00, // global type
      0x41, 42,  // i32.const 42 — but NO 0x0B terminator
    ];
    const wasm = makeWasm(makeSection(SEC_GLOBAL, globalPayload));
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });

  it("throws WasmParseError on invalid table element type", () => {
    const tablePayload = [1, 0x6F, 0x00, 1]; // 0x6F = externref, not valid in WASM 1.0 here
    const wasm = makeWasm(makeSection(SEC_TABLE, tablePayload));
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });

  it("throws WasmParseError on code body exceeding data bounds", () => {
    // Claim body size=50 but only provide 2 bytes
    const codePayload = [1, 50, 0, 0x0b]; // 1 body, size=50, but short
    const wasm = makeWasm(makeSection(SEC_CODE, codePayload));
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });

  it("throws WasmParseError on truncated read (readByte at end)", () => {
    // A section whose payload is 0 bytes, but we declared count=1 (expects at least 1 byte)
    const startPayload: number[] = []; // start section with no bytes — but expected u32leb
    const wasm = makeWasm(makeSection(SEC_START, startPayload));
    expect(() => parser.parse(wasm)).toThrow(WasmParseError);
  });
});
