/**
 * wasm_types.test.ts — Tests for the WASM 1.0 type system
 *
 * Covers:
 *   - ValueType enum values match spec bytes
 *   - BlockType.EMPTY = 0x40
 *   - ExternalKind enum values
 *   - FuncType construction and equality
 *   - Limits (with and without max)
 *   - MemoryType, TableType, GlobalType construction
 *   - Import for each ExternalKind
 *   - Export construction
 *   - Global with initExpr bytes
 *   - Element with functionIndices
 *   - DataSegment construction
 *   - FunctionBody with locals and code
 *   - CustomSection construction
 *   - WasmModule starts empty
 *   - WasmModule can be populated
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  VERSION,
  ValueType,
  BlockType,
  ExternalKind,
  FUNCREF,
  makeFuncType,
  WasmModule,
} from "../src/index.js";
import type {
  FuncType,
  Limits,
  MemoryType,
  TableType,
  GlobalType,
  Import,
  Export,
  Global,
  Element,
  DataSegment,
  FunctionBody,
  CustomSection,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Version
// ─────────────────────────────────────────────────────────────────────────────

describe("VERSION", () => {
  it("has a version", () => {
    assert.equal(VERSION, "0.1.0");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// ValueType
// ─────────────────────────────────────────────────────────────────────────────

describe("ValueType", () => {
  it("I32 = 0x7F", () => {
    assert.equal(ValueType.I32, 0x7f);
  });

  it("I64 = 0x7E", () => {
    assert.equal(ValueType.I64, 0x7e);
  });

  it("F32 = 0x7D", () => {
    assert.equal(ValueType.F32, 0x7d);
  });

  it("F64 = 0x7C", () => {
    assert.equal(ValueType.F64, 0x7c);
  });

  it("all four type codes are distinct", () => {
    const codes = [ValueType.I32, ValueType.I64, ValueType.F32, ValueType.F64];
    const unique = new Set(codes);
    assert.equal(unique.size, 4);
  });

  it("type codes are in descending order from 0x7F", () => {
    // WASM spec: I32 > I64 > F32 > F64 numerically
    assert.ok(ValueType.I32 > ValueType.I64);
    assert.ok(ValueType.I64 > ValueType.F32);
    assert.ok(ValueType.F32 > ValueType.F64);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BlockType
// ─────────────────────────────────────────────────────────────────────────────

describe("BlockType", () => {
  it("EMPTY = 0x40", () => {
    assert.equal(BlockType.EMPTY, 0x40);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// ExternalKind
// ─────────────────────────────────────────────────────────────────────────────

describe("ExternalKind", () => {
  it("FUNCTION = 0x00", () => {
    assert.equal(ExternalKind.FUNCTION, 0x00);
  });

  it("TABLE = 0x01", () => {
    assert.equal(ExternalKind.TABLE, 0x01);
  });

  it("MEMORY = 0x02", () => {
    assert.equal(ExternalKind.MEMORY, 0x02);
  });

  it("GLOBAL = 0x03", () => {
    assert.equal(ExternalKind.GLOBAL, 0x03);
  });

  it("all four external kinds are distinct", () => {
    const kinds = [
      ExternalKind.FUNCTION,
      ExternalKind.TABLE,
      ExternalKind.MEMORY,
      ExternalKind.GLOBAL,
    ];
    const unique = new Set(kinds);
    assert.equal(unique.size, 4);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// FUNCREF
// ─────────────────────────────────────────────────────────────────────────────

describe("FUNCREF", () => {
  it("FUNCREF = 0x70", () => {
    assert.equal(FUNCREF, 0x70);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// FuncType
// ─────────────────────────────────────────────────────────────────────────────

describe("FuncType", () => {
  it("construction with params and results", () => {
    const ft = makeFuncType([ValueType.I32, ValueType.I64], [ValueType.F64]);
    assert.deepEqual(ft.params, [ValueType.I32, ValueType.I64]);
    assert.deepEqual(ft.results, [ValueType.F64]);
  });

  it("with empty params and empty results", () => {
    const ft = makeFuncType([], []);
    assert.deepEqual(ft.params, []);
    assert.deepEqual(ft.results, []);
    assert.equal(ft.params.length, 0);
    assert.equal(ft.results.length, 0);
  });

  it("with multiple params (variadic style)", () => {
    const ft = makeFuncType(
      [ValueType.I32, ValueType.I32, ValueType.I32],
      [ValueType.I32]
    );
    assert.equal(ft.params.length, 3);
    assert.equal(ft.results.length, 1);
    assert.equal(ft.params[0], ValueType.I32);
    assert.equal(ft.params[1], ValueType.I32);
    assert.equal(ft.params[2], ValueType.I32);
    assert.equal(ft.results[0], ValueType.I32);
  });

  it("with only results (no params)", () => {
    const ft = makeFuncType([], [ValueType.I32]);
    assert.deepEqual(ft.params, []);
    assert.deepEqual(ft.results, [ValueType.I32]);
  });

  it("equality — two identical FuncTypes have equal fields", () => {
    const ft1 = makeFuncType([ValueType.I32], [ValueType.I64]);
    const ft2 = makeFuncType([ValueType.I32], [ValueType.I64]);
    assert.deepEqual(ft1.params, ft2.params);
    assert.deepEqual(ft1.results, ft2.results);
  });

  it("is structurally immutable — params array is frozen", () => {
    const ft = makeFuncType([ValueType.I32], [ValueType.I64]);
    assert.equal(Object.isFrozen(ft.params), true);
    assert.equal(Object.isFrozen(ft.results), true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Limits
// ─────────────────────────────────────────────────────────────────────────────

describe("Limits", () => {
  it("with only min (max = null)", () => {
    const limits: Limits = { min: 1, max: null };
    assert.equal(limits.min, 1);
    assert.equal(limits.max, null);
  });

  it("with min and max", () => {
    const limits: Limits = { min: 2, max: 16 };
    assert.equal(limits.min, 2);
    assert.equal(limits.max, 16);
  });

  it("min = 0 is valid (no initial pages)", () => {
    const limits: Limits = { min: 0, max: null };
    assert.equal(limits.min, 0);
    assert.equal(limits.max, null);
  });

  it("min = max is valid (fixed size)", () => {
    const limits: Limits = { min: 4, max: 4 };
    assert.equal(limits.min, limits.max);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// MemoryType
// ─────────────────────────────────────────────────────────────────────────────

describe("MemoryType", () => {
  it("construction with unbounded limits", () => {
    const mt: MemoryType = { limits: { min: 1, max: null } };
    assert.equal(mt.limits.min, 1);
    assert.equal(mt.limits.max, null);
  });

  it("construction with bounded limits", () => {
    const mt: MemoryType = { limits: { min: 1, max: 8 } };
    assert.equal(mt.limits.min, 1);
    assert.equal(mt.limits.max, 8);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// TableType
// ─────────────────────────────────────────────────────────────────────────────

describe("TableType", () => {
  it("construction with funcref and unbounded limits", () => {
    const tt: TableType = { elementType: FUNCREF, limits: { min: 10, max: null } };
    assert.equal(tt.elementType, 0x70);
    assert.equal(tt.limits.min, 10);
    assert.equal(tt.limits.max, null);
  });

  it("construction with funcref and bounded limits", () => {
    const tt: TableType = { elementType: FUNCREF, limits: { min: 0, max: 100 } };
    assert.equal(tt.elementType, FUNCREF);
    assert.equal(tt.limits.max, 100);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// GlobalType
// ─────────────────────────────────────────────────────────────────────────────

describe("GlobalType", () => {
  it("immutable i32 global", () => {
    const gt: GlobalType = { valueType: ValueType.I32, mutable: false };
    assert.equal(gt.valueType, ValueType.I32);
    assert.equal(gt.mutable, false);
  });

  it("mutable f64 global", () => {
    const gt: GlobalType = { valueType: ValueType.F64, mutable: true };
    assert.equal(gt.valueType, ValueType.F64);
    assert.equal(gt.mutable, true);
  });

  it("mutable and immutable are distinguishable", () => {
    const immut: GlobalType = { valueType: ValueType.I32, mutable: false };
    const mut: GlobalType = { valueType: ValueType.I32, mutable: true };
    assert.notEqual(immut.mutable, mut.mutable);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Import
// ─────────────────────────────────────────────────────────────────────────────

describe("Import", () => {
  it("function import", () => {
    const imp: Import = {
      moduleName: "env",
      name: "add",
      kind: ExternalKind.FUNCTION,
      typeInfo: 2,
    };
    assert.equal(imp.moduleName, "env");
    assert.equal(imp.name, "add");
    assert.equal(imp.kind, ExternalKind.FUNCTION);
    assert.equal(imp.typeInfo, 2);
  });

  it("table import", () => {
    const tt: TableType = { elementType: FUNCREF, limits: { min: 1, max: null } };
    const imp: Import = {
      moduleName: "env",
      name: "table",
      kind: ExternalKind.TABLE,
      typeInfo: tt,
    };
    assert.equal(imp.kind, ExternalKind.TABLE);
    assert.deepEqual(imp.typeInfo, tt);
  });

  it("memory import", () => {
    const mt: MemoryType = { limits: { min: 1, max: null } };
    const imp: Import = {
      moduleName: "env",
      name: "memory",
      kind: ExternalKind.MEMORY,
      typeInfo: mt,
    };
    assert.equal(imp.kind, ExternalKind.MEMORY);
    assert.deepEqual(imp.typeInfo, mt);
  });

  it("global import", () => {
    const gt: GlobalType = { valueType: ValueType.I32, mutable: false };
    const imp: Import = {
      moduleName: "env",
      name: "stackPointer",
      kind: ExternalKind.GLOBAL,
      typeInfo: gt,
    };
    assert.equal(imp.kind, ExternalKind.GLOBAL);
    assert.deepEqual(imp.typeInfo, gt);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Export
// ─────────────────────────────────────────────────────────────────────────────

describe("Export", () => {
  it("function export", () => {
    const exp: Export = {
      name: "main",
      kind: ExternalKind.FUNCTION,
      index: 0,
    };
    assert.equal(exp.name, "main");
    assert.equal(exp.kind, ExternalKind.FUNCTION);
    assert.equal(exp.index, 0);
  });

  it("memory export", () => {
    const exp: Export = {
      name: "memory",
      kind: ExternalKind.MEMORY,
      index: 0,
    };
    assert.equal(exp.kind, ExternalKind.MEMORY);
    assert.equal(exp.index, 0);
  });

  it("global export", () => {
    const exp: Export = {
      name: "stackPointer",
      kind: ExternalKind.GLOBAL,
      index: 1,
    };
    assert.equal(exp.kind, ExternalKind.GLOBAL);
    assert.equal(exp.index, 1);
  });

  it("table export", () => {
    const exp: Export = {
      name: "__indirect_function_table",
      kind: ExternalKind.TABLE,
      index: 0,
    };
    assert.equal(exp.kind, ExternalKind.TABLE);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Global
// ─────────────────────────────────────────────────────────────────────────────

describe("Global", () => {
  it("construction with initExpr bytes", () => {
    // i32.const 42; end  →  [0x41, 0x2A, 0x0B]
    const glob: Global = {
      globalType: { valueType: ValueType.I32, mutable: false },
      initExpr: new Uint8Array([0x41, 0x2a, 0x0b]),
    };
    assert.equal(glob.globalType.valueType, ValueType.I32);
    assert.equal(glob.globalType.mutable, false);
    assert.deepEqual(glob.initExpr, new Uint8Array([0x41, 0x2a, 0x0b]));
  });

  it("mutable global with f64 init", () => {
    // f64.const 0.0; end  →  [0x44, <8 zero bytes>, 0x0B]
    const initBytes = new Uint8Array([
      0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0b,
    ]);
    const glob: Global = {
      globalType: { valueType: ValueType.F64, mutable: true },
      initExpr: initBytes,
    };
    assert.equal(glob.globalType.mutable, true);
    assert.equal(glob.initExpr.length, 10);
    assert.equal(glob.initExpr[0], 0x44); // f64.const opcode
    assert.equal(glob.initExpr[glob.initExpr.length - 1], 0x0b); // end
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Element
// ─────────────────────────────────────────────────────────────────────────────

describe("Element", () => {
  it("construction with functionIndices", () => {
    // Fill table 0 at offset 0 with functions [0, 1, 2]
    const elem: Element = {
      tableIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]), // i32.const 0; end
      functionIndices: [0, 1, 2],
    };
    assert.equal(elem.tableIndex, 0);
    assert.deepEqual(elem.offsetExpr, new Uint8Array([0x41, 0x00, 0x0b]));
    assert.deepEqual(elem.functionIndices, [0, 1, 2]);
    assert.equal(elem.functionIndices.length, 3);
  });

  it("empty functionIndices is valid", () => {
    const elem: Element = {
      tableIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]),
      functionIndices: [],
    };
    assert.equal(elem.functionIndices.length, 0);
  });

  it("non-zero table offset", () => {
    // i32.const 5; end  →  [0x41, 0x05, 0x0B]
    const elem: Element = {
      tableIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x05, 0x0b]),
      functionIndices: [10, 11],
    };
    assert.equal(elem.offsetExpr[1], 5);
    assert.equal(elem.functionIndices[0], 10);
    assert.equal(elem.functionIndices[1], 11);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// DataSegment
// ─────────────────────────────────────────────────────────────────────────────

describe("DataSegment", () => {
  it("construction with raw data bytes", () => {
    // Write "Hi" at address 0
    const seg: DataSegment = {
      memoryIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]), // i32.const 0; end
      data: new Uint8Array([0x48, 0x69]), // "Hi"
    };
    assert.equal(seg.memoryIndex, 0);
    assert.deepEqual(seg.offsetExpr, new Uint8Array([0x41, 0x00, 0x0b]));
    assert.deepEqual(seg.data, new Uint8Array([0x48, 0x69]));
    assert.equal(seg.data.length, 2);
  });

  it("empty data segment is valid", () => {
    const seg: DataSegment = {
      memoryIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]),
      data: new Uint8Array(0),
    };
    assert.equal(seg.data.length, 0);
  });

  it("non-zero offset", () => {
    // i32.const 256; end  →  [0x41, 0x80, 0x02, 0x0B] (LEB128 256 = 0x80 0x02)
    const seg: DataSegment = {
      memoryIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x80, 0x02, 0x0b]),
      data: new Uint8Array([0xde, 0xad, 0xbe, 0xef]),
    };
    assert.equal(seg.offsetExpr.length, 4);
    assert.deepEqual(seg.data, new Uint8Array([0xde, 0xad, 0xbe, 0xef]));
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// FunctionBody
// ─────────────────────────────────────────────────────────────────────────────

describe("FunctionBody", () => {
  it("construction with locals and code", () => {
    // Two i32 locals, body: local.get 0; local.get 1; i32.add; end
    const body: FunctionBody = {
      locals: [ValueType.I32, ValueType.I32],
      code: new Uint8Array([0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b]),
    };
    assert.deepEqual(body.locals, [ValueType.I32, ValueType.I32]);
    assert.deepEqual(
      body.code,
      new Uint8Array([0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b]),
    );
    assert.equal(body.locals.length, 2);
    assert.equal(body.code.length, 6);
  });

  it("no locals — empty locals list", () => {
    const body: FunctionBody = {
      locals: [],
      code: new Uint8Array([0x0b]), // just end
    };
    assert.equal(body.locals.length, 0);
    assert.equal(body.code[0], 0x0b);
  });

  it("mixed local types", () => {
    const body: FunctionBody = {
      locals: [ValueType.I32, ValueType.F64, ValueType.I64],
      code: new Uint8Array([0x0b]),
    };
    assert.equal(body.locals[0], ValueType.I32);
    assert.equal(body.locals[1], ValueType.F64);
    assert.equal(body.locals[2], ValueType.I64);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// CustomSection
// ─────────────────────────────────────────────────────────────────────────────

describe("CustomSection", () => {
  it("construction with name and data", () => {
    const cs: CustomSection = {
      name: "name",
      data: new Uint8Array([0x01, 0x02, 0x03]),
    };
    assert.equal(cs.name, "name");
    assert.deepEqual(cs.data, new Uint8Array([0x01, 0x02, 0x03]));
  });

  it("empty data is valid", () => {
    const cs: CustomSection = { name: "producers", data: new Uint8Array(0) };
    assert.equal(cs.name, "producers");
    assert.equal(cs.data.length, 0);
  });

  it("any name is valid", () => {
    const cs: CustomSection = {
      name: "my.custom.tool",
      data: new Uint8Array([0xff]),
    };
    assert.equal(cs.name, "my.custom.tool");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// WasmModule
// ─────────────────────────────────────────────────────────────────────────────

describe("WasmModule", () => {
  it("starts with all empty arrays and null start", () => {
    const mod = new WasmModule();
    assert.deepEqual(mod.types, []);
    assert.deepEqual(mod.imports, []);
    assert.deepEqual(mod.functions, []);
    assert.deepEqual(mod.tables, []);
    assert.deepEqual(mod.memories, []);
    assert.deepEqual(mod.globals, []);
    assert.deepEqual(mod.exports, []);
    assert.equal(mod.start, null);
    assert.deepEqual(mod.elements, []);
    assert.deepEqual(mod.code, []);
    assert.deepEqual(mod.data, []);
    assert.deepEqual(mod.customs, []);
  });

  it("can be populated with types and functions", () => {
    const mod = new WasmModule();
    const ft = makeFuncType([ValueType.I32], [ValueType.I32]);
    mod.types.push(ft);
    mod.functions.push(0);

    assert.equal(mod.types.length, 1);
    assert.deepEqual(mod.types[0], ft);
    assert.equal(mod.functions[0], 0);
  });

  it("can be populated with memories", () => {
    const mod = new WasmModule();
    const mt: MemoryType = { limits: { min: 1, max: null } };
    mod.memories.push(mt);
    assert.equal(mod.memories.length, 1);
    assert.equal(mod.memories[0].limits.min, 1);
  });

  it("can set the start function", () => {
    const mod = new WasmModule();
    assert.equal(mod.start, null);
    mod.start = 3;
    assert.equal(mod.start, 3);
  });

  it("can be populated with imports and exports", () => {
    const mod = new WasmModule();
    const imp: Import = {
      moduleName: "env",
      name: "print",
      kind: ExternalKind.FUNCTION,
      typeInfo: 0,
    };
    const exp: Export = { name: "main", kind: ExternalKind.FUNCTION, index: 1 };
    mod.imports.push(imp);
    mod.exports.push(exp);

    assert.equal(mod.imports.length, 1);
    assert.equal(mod.exports.length, 1);
    assert.equal(mod.imports[0].moduleName, "env");
    assert.equal(mod.exports[0].name, "main");
  });

  it("can be populated with globals, elements, data, and customs", () => {
    const mod = new WasmModule();

    const glob: Global = {
      globalType: { valueType: ValueType.I32, mutable: true },
      initExpr: new Uint8Array([0x41, 0x00, 0x0b]),
    };
    const elem: Element = {
      tableIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]),
      functionIndices: [0],
    };
    const seg: DataSegment = {
      memoryIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]),
      data: new Uint8Array([42]),
    };
    const cs: CustomSection = { name: "name", data: new Uint8Array([]) };

    mod.globals.push(glob);
    mod.elements.push(elem);
    mod.data.push(seg);
    mod.customs.push(cs);

    assert.equal(mod.globals.length, 1);
    assert.equal(mod.elements.length, 1);
    assert.equal(mod.data.length, 1);
    assert.equal(mod.customs.length, 1);
  });

  it("multiple instances are independent", () => {
    const mod1 = new WasmModule();
    const mod2 = new WasmModule();
    mod1.types.push(makeFuncType([ValueType.I32], []));

    // mod2 should still be empty
    assert.equal(mod2.types.length, 0);
    assert.equal(mod1.types.length, 1);
  });
});
