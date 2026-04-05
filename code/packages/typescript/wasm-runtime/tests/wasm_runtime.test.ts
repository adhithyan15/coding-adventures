/**
 * Tests for WasmRuntime — the user-facing runtime API.
 *
 * ==========================================================================
 * Test Strategy
 * ==========================================================================
 *
 * We test each method of WasmRuntime in isolation:
 *
 * 1. **load()** — already covered by square.test.ts for the binary path.
 *    Here we just verify it parses a minimal module.
 *
 * 2. **validate()** — verify it accepts a valid module.
 *
 * 3. **instantiate()** — the complex one. We build WasmModule objects by
 *    hand (bypassing parsing) to test each instantiation step:
 *    - Memory allocation from the memory section
 *    - Global initialization from const expressions
 *    - Data segment application (copying bytes into memory)
 *    - Element segment application (populating tables)
 *    - Import resolution (function, memory, table, global imports)
 *    - Start function invocation
 *
 * 4. **call()** — error paths: non-existent export, non-function export.
 *
 * 5. **loadAndRun()** — convenience method (already tested in square.test.ts
 *    but we add one more here for completeness).
 *
 * @module
 */

import { describe, it, expect } from "vitest";
import { WasmRuntime } from "../src/wasm_runtime.js";
import { WasiStub, ProcExitError } from "../src/wasi_stub.js";
import { WasmModule, ValueType, ExternalKind, makeFuncType } from "@coding-adventures/wasm-types";
import type { FuncType, GlobalType, Global, DataSegment, Export, Import, FunctionBody, Element as WasmElement, TableType, MemoryType } from "@coding-adventures/wasm-types";
import {
  TrapError,
  LinearMemory,
  Table,
  i32,
} from "@coding-adventures/wasm-execution";
import type { HostFunction, HostInterface, WasmValue } from "@coding-adventures/wasm-execution";
import { encodeSigned, encodeUnsigned } from "@coding-adventures/wasm-leb128";

// =========================================================================
// Helpers
// =========================================================================

/**
 * Build a WasmModule object directly (bypass parsing) for unit tests.
 * This lets us control exactly which sections are populated.
 */
function makeModule(overrides: Partial<WasmModule> = {}): WasmModule {
  const m = new WasmModule();
  Object.assign(m, overrides);
  return m;
}

/**
 * Build a constant expression for i32.const N.
 * Format: [0x41, ...signed_leb128(N), 0x0B]
 */
function i32ConstExpr(value: number): Uint8Array {
  return new Uint8Array([0x41, ...encodeSigned(value), 0x0B]);
}

/**
 * Build a minimal valid .wasm binary with just the header.
 * Optionally includes a type section.
 */
function buildMinimalWasm(): Uint8Array {
  const parts: number[] = [];
  // Magic: "\0asm"
  parts.push(0x00, 0x61, 0x73, 0x6D);
  // Version: 1
  parts.push(0x01, 0x00, 0x00, 0x00);

  // Type section with 1 func type: () -> ()
  const typePayload = [
    0x01,       // 1 type entry
    0x60,       // function type marker
    0x00,       // 0 params
    0x00,       // 0 results
  ];
  parts.push(0x01);                           // section ID = 1 (Type)
  parts.push(...encodeUnsigned(typePayload.length));
  parts.push(...typePayload);

  return new Uint8Array(parts);
}

// =========================================================================
// load() — Parse WASM Binary
// =========================================================================

describe("WasmRuntime.load", () => {
  it("parses a minimal WASM module", () => {
    const runtime = new WasmRuntime();
    const wasmBytes = buildMinimalWasm();
    const module = runtime.load(wasmBytes);
    expect(module.types.length).toBe(1);
    expect(module.types[0].params.length).toBe(0);
    expect(module.types[0].results.length).toBe(0);
  });
});

// =========================================================================
// validate() — Semantic Validation
// =========================================================================

describe("WasmRuntime.validate", () => {
  it("validates a parsed module", () => {
    const runtime = new WasmRuntime();
    const wasmBytes = buildMinimalWasm();
    const module = runtime.load(wasmBytes);
    const validated = runtime.validate(module);
    expect(validated.module).toBe(module);
  });
});

// =========================================================================
// instantiate() — Memory Allocation
// =========================================================================

describe("WasmRuntime.instantiate — memory", () => {
  it("allocates memory from the memory section", () => {
    const runtime = new WasmRuntime();
    const module = makeModule({
      memories: [{ limits: { min: 1, max: 4 } }],
    });

    const instance = runtime.instantiate(module);
    expect(instance.memory).not.toBeNull();
    expect(instance.memory!.size()).toBe(1);
  });

  it("creates instance with no memory when memory section is empty", () => {
    const runtime = new WasmRuntime();
    const module = makeModule();

    const instance = runtime.instantiate(module);
    expect(instance.memory).toBeNull();
  });
});

// =========================================================================
// instantiate() — Globals Initialization
// =========================================================================

describe("WasmRuntime.instantiate — globals", () => {
  it("initializes globals from const expressions", () => {
    const runtime = new WasmRuntime();
    const module = makeModule({
      globals: [
        {
          globalType: { valueType: ValueType.I32, mutable: false },
          initExpr: i32ConstExpr(42),
        },
        {
          globalType: { valueType: ValueType.I32, mutable: true },
          initExpr: i32ConstExpr(100),
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.globals.length).toBe(2);
    expect(instance.globals[0].value).toBe(42);
    expect(instance.globals[1].value).toBe(100);
    expect(instance.globalTypes[0]).toEqual({ valueType: ValueType.I32, mutable: false });
    expect(instance.globalTypes[1]).toEqual({ valueType: ValueType.I32, mutable: true });
  });
});

// =========================================================================
// instantiate() — Data Segments
// =========================================================================

describe("WasmRuntime.instantiate — data segments", () => {
  it("copies data segment bytes into memory", () => {
    const runtime = new WasmRuntime();
    const helloBytes = new TextEncoder().encode("Hello");

    const module = makeModule({
      memories: [{ limits: { min: 1, max: null } }],
      data: [
        {
          memoryIndex: 0,
          offsetExpr: i32ConstExpr(256), // Write at byte 256
          data: helloBytes,
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.memory).not.toBeNull();

    // Verify bytes were copied.
    for (let i = 0; i < helloBytes.length; i++) {
      const byte = instance.memory!.loadI32_8u(256 + i);
      expect(byte).toBe(helloBytes[i]);
    }
  });

  it("applies multiple data segments", () => {
    const runtime = new WasmRuntime();
    const module = makeModule({
      memories: [{ limits: { min: 1, max: null } }],
      data: [
        {
          memoryIndex: 0,
          offsetExpr: i32ConstExpr(0),
          data: new Uint8Array([0xAA, 0xBB]),
        },
        {
          memoryIndex: 0,
          offsetExpr: i32ConstExpr(100),
          data: new Uint8Array([0xCC, 0xDD]),
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.memory!.loadI32_8u(0)).toBe(0xAA);
    expect(instance.memory!.loadI32_8u(1)).toBe(0xBB);
    expect(instance.memory!.loadI32_8u(100)).toBe(0xCC);
    expect(instance.memory!.loadI32_8u(101)).toBe(0xDD);
  });
});

// =========================================================================
// instantiate() — Element Segments (Table Init)
// =========================================================================

describe("WasmRuntime.instantiate — element segments", () => {
  it("populates tables from element segments", () => {
    const runtime = new WasmRuntime();
    const module = makeModule({
      tables: [{ elementType: 0x70, limits: { min: 10, max: null } }],
      elements: [
        {
          tableIndex: 0,
          offsetExpr: i32ConstExpr(2),
          functionIndices: [5, 10, 15],
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.tables.length).toBe(1);
    // Verify the function indices were placed at offsets 2, 3, 4.
    expect(instance.tables[0].get(2)).toBe(5);
    expect(instance.tables[0].get(3)).toBe(10);
    expect(instance.tables[0].get(4)).toBe(15);
  });
});

// =========================================================================
// instantiate() — Import Resolution
// =========================================================================

describe("WasmRuntime.instantiate — imports", () => {
  it("resolves function imports from a host interface", () => {
    const mockHost: HostInterface = {
      resolveFunction(moduleName: string, name: string): HostFunction | undefined {
        if (moduleName === "env" && name === "add") {
          return {
            type: makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]),
            call(args: WasmValue[]): WasmValue[] {
              return [i32((args[0].value as number) + (args[1].value as number))];
            },
          };
        }
        return undefined;
      },
      resolveGlobal() { return undefined; },
      resolveMemory() { return undefined; },
      resolveTable() { return undefined; },
    };

    const runtime = new WasmRuntime(mockHost);
    const module = makeModule({
      types: [makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32])],
      imports: [
        {
          moduleName: "env",
          name: "add",
          kind: ExternalKind.FUNCTION,
          typeInfo: 0,
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.hostFunctions.length).toBe(1);
    expect(instance.hostFunctions[0]).toBeDefined();
    expect(instance.funcTypes.length).toBe(1);
  });

  it("resolves memory imports from host", () => {
    const importedMemory = new LinearMemory(2);
    const mockHost: HostInterface = {
      resolveFunction() { return undefined; },
      resolveGlobal() { return undefined; },
      resolveMemory(moduleName: string, name: string) {
        if (moduleName === "env" && name === "memory") return importedMemory;
        return undefined;
      },
      resolveTable() { return undefined; },
    };

    const runtime = new WasmRuntime(mockHost);
    const module = makeModule({
      imports: [
        {
          moduleName: "env",
          name: "memory",
          kind: ExternalKind.MEMORY,
          typeInfo: { limits: { min: 1, max: null } },
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.memory).toBe(importedMemory);
  });

  it("resolves table imports from host", () => {
    const importedTable = new Table(5);
    const mockHost: HostInterface = {
      resolveFunction() { return undefined; },
      resolveGlobal() { return undefined; },
      resolveMemory() { return undefined; },
      resolveTable(moduleName: string, name: string) {
        if (moduleName === "env" && name === "__indirect_function_table") return importedTable;
        return undefined;
      },
    };

    const runtime = new WasmRuntime(mockHost);
    const module = makeModule({
      imports: [
        {
          moduleName: "env",
          name: "__indirect_function_table",
          kind: ExternalKind.TABLE,
          typeInfo: { elementType: 0x70, limits: { min: 5, max: null } },
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.tables.length).toBe(1);
    expect(instance.tables[0]).toBe(importedTable);
  });

  it("resolves global imports from host", () => {
    const mockHost: HostInterface = {
      resolveFunction() { return undefined; },
      resolveGlobal(moduleName: string, name: string) {
        if (moduleName === "env" && name === "stack_pointer") {
          return {
            type: { valueType: ValueType.I32, mutable: true },
            value: i32(65536),
          };
        }
        return undefined;
      },
      resolveMemory() { return undefined; },
      resolveTable() { return undefined; },
    };

    const runtime = new WasmRuntime(mockHost);
    const module = makeModule({
      imports: [
        {
          moduleName: "env",
          name: "stack_pointer",
          kind: ExternalKind.GLOBAL,
          typeInfo: { valueType: ValueType.I32, mutable: true },
        },
      ],
    });

    const instance = runtime.instantiate(module);
    expect(instance.globals.length).toBe(1);
    expect(instance.globals[0].value).toBe(65536);
    expect(instance.globalTypes[0]).toEqual({ valueType: ValueType.I32, mutable: true });
  });

  it("handles unresolved imports gracefully (null host)", () => {
    const runtime = new WasmRuntime(); // No host
    const module = makeModule({
      types: [makeFuncType([], [])],
      imports: [
        {
          moduleName: "env",
          name: "missing",
          kind: ExternalKind.FUNCTION,
          typeInfo: 0,
        },
      ],
    });

    const instance = runtime.instantiate(module);
    // Should have a null host function for the unresolved import.
    expect(instance.hostFunctions[0]).toBeNull();
  });
});

// =========================================================================
// instantiate() — Module Functions
// =========================================================================

describe("WasmRuntime.instantiate — module functions", () => {
  it("registers module-defined function types and bodies", () => {
    const runtime = new WasmRuntime();
    const funcType = makeFuncType([ValueType.I32], [ValueType.I32]);
    const body: FunctionBody = {
      locals: [],
      code: new Uint8Array([0x20, 0x00, 0x0B]), // local.get 0, end
    };

    const module = makeModule({
      types: [funcType],
      functions: [0],
      code: [body],
      exports: [{ name: "identity", kind: ExternalKind.FUNCTION, index: 0 }],
    });

    const instance = runtime.instantiate(module);
    expect(instance.funcTypes.length).toBe(1);
    expect(instance.funcBodies.length).toBe(1);
    expect(instance.funcBodies[0]).toBe(body);
    expect(instance.exports.has("identity")).toBe(true);
  });
});

// =========================================================================
// call() — Error Paths
// =========================================================================

describe("WasmRuntime.call — error paths", () => {
  it("throws TrapError for non-existent export", () => {
    const runtime = new WasmRuntime();
    const module = makeModule();
    const instance = runtime.instantiate(module);

    expect(() => runtime.call(instance, "nonexistent", [])).toThrow(TrapError);
    expect(() => runtime.call(instance, "nonexistent", [])).toThrow(
      'export "nonexistent" not found'
    );
  });

  it("throws TrapError for non-function export", () => {
    const runtime = new WasmRuntime();
    const module = makeModule({
      memories: [{ limits: { min: 1, max: null } }],
      exports: [
        { name: "memory", kind: ExternalKind.MEMORY, index: 0 },
      ],
    });

    const instance = runtime.instantiate(module);

    expect(() => runtime.call(instance, "memory", [])).toThrow(TrapError);
    expect(() => runtime.call(instance, "memory", [])).toThrow(
      'export "memory" is not a function'
    );
  });
});

// =========================================================================
// call() — Argument Type Conversion
// =========================================================================

describe("WasmRuntime.call — type conversion", () => {
  it("calls a function and converts arguments by parameter types", () => {
    const runtime = new WasmRuntime();

    // Build a module with a function that returns its first arg (identity).
    const funcType = makeFuncType([ValueType.I32], [ValueType.I32]);
    const body: FunctionBody = {
      locals: [],
      code: new Uint8Array([0x20, 0x00, 0x0B]), // local.get 0, end
    };

    const module = makeModule({
      types: [funcType],
      functions: [0],
      code: [body],
      exports: [{ name: "id", kind: ExternalKind.FUNCTION, index: 0 }],
    });

    const instance = runtime.instantiate(module);
    const result = runtime.call(instance, "id", [42]);
    expect(result).toEqual([42]);
  });
});

// =========================================================================
// loadAndRun() — Convenience Method
// =========================================================================

describe("WasmRuntime.loadAndRun", () => {
  it("parses, validates, instantiates, and calls in one step", () => {
    // Build a minimal wasm that exports a function returning a constant.
    const parts: number[] = [];
    // Header
    parts.push(0x00, 0x61, 0x73, 0x6D);
    parts.push(0x01, 0x00, 0x00, 0x00);

    // Type section: () -> (i32)
    const typePayload = [0x01, 0x60, 0x00, 0x01, 0x7F];
    parts.push(0x01, ...encodeUnsigned(typePayload.length), ...typePayload);

    // Function section: 1 function, type 0
    const funcPayload = [0x01, 0x00];
    parts.push(0x03, ...encodeUnsigned(funcPayload.length), ...funcPayload);

    // Export section: "answer" -> function 0
    const nameBytes = new TextEncoder().encode("answer");
    const exportPayload = [
      0x01,
      ...encodeUnsigned(nameBytes.length),
      ...nameBytes,
      0x00, // function
      0x00, // index 0
    ];
    parts.push(0x07, ...encodeUnsigned(exportPayload.length), ...exportPayload);

    // Code section: i32.const 42, end
    const bodyCode = [0x41, 0x2A, 0x0B]; // i32.const 42, end
    const bodyPayload = [0x00, ...bodyCode]; // 0 locals
    const funcBody = [...encodeUnsigned(bodyPayload.length), ...bodyPayload];
    const codePayload = [0x01, ...funcBody];
    parts.push(0x0A, ...encodeUnsigned(codePayload.length), ...codePayload);

    const wasmBytes = new Uint8Array(parts);
    const runtime = new WasmRuntime();
    const result = runtime.loadAndRun(wasmBytes, "answer", []);
    expect(result).toEqual([42]);
  });
});

// =========================================================================
// instantiate() — Start Function
// =========================================================================

describe("WasmRuntime.instantiate — start function", () => {
  it("calls the start function during instantiation", () => {
    const runtime = new WasmRuntime();

    // The start function writes a value to a global.
    // We use a function that sets global 0 to 99.
    // global.set requires the global to be mutable.
    const funcType = makeFuncType([], []);
    const body: FunctionBody = {
      locals: [],
      // i32.const 99, global.set 0, end
      code: new Uint8Array([0x41, ...encodeSigned(99), 0x24, 0x00, 0x0B]),
    };

    const module = makeModule({
      types: [funcType],
      functions: [0],
      code: [body],
      globals: [
        {
          globalType: { valueType: ValueType.I32, mutable: true },
          initExpr: i32ConstExpr(0),
        },
      ],
      start: 0,
    });

    const instance = runtime.instantiate(module);
    // After start function runs, global 0 should be 99.
    expect(instance.globals[0].value).toBe(99);
  });
});

// =========================================================================
// Constructor — With and Without Host
// =========================================================================

describe("WasmRuntime constructor", () => {
  it("works without a host interface", () => {
    const runtime = new WasmRuntime();
    const module = makeModule();
    const instance = runtime.instantiate(module);
    expect(instance.host).toBeNull();
  });

  it("stores the host interface on the instance", () => {
    const wasi = new WasiStub();
    const runtime = new WasmRuntime(wasi);
    const module = makeModule();
    const instance = runtime.instantiate(module);
    expect(instance.host).toBe(wasi);
  });
});
