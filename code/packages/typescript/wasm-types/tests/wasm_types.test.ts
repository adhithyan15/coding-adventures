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

import { describe, it, expect } from "vitest";
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
    expect(VERSION).toBe("0.1.0");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// ValueType
// ─────────────────────────────────────────────────────────────────────────────

describe("ValueType", () => {
  it("I32 = 0x7F", () => {
    expect(ValueType.I32).toBe(0x7f);
  });

  it("I64 = 0x7E", () => {
    expect(ValueType.I64).toBe(0x7e);
  });

  it("F32 = 0x7D", () => {
    expect(ValueType.F32).toBe(0x7d);
  });

  it("F64 = 0x7C", () => {
    expect(ValueType.F64).toBe(0x7c);
  });

  it("all four type codes are distinct", () => {
    const codes = [ValueType.I32, ValueType.I64, ValueType.F32, ValueType.F64];
    const unique = new Set(codes);
    expect(unique.size).toBe(4);
  });

  it("type codes are in descending order from 0x7F", () => {
    // WASM spec: I32 > I64 > F32 > F64 numerically
    expect(ValueType.I32).toBeGreaterThan(ValueType.I64);
    expect(ValueType.I64).toBeGreaterThan(ValueType.F32);
    expect(ValueType.F32).toBeGreaterThan(ValueType.F64);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// BlockType
// ─────────────────────────────────────────────────────────────────────────────

describe("BlockType", () => {
  it("EMPTY = 0x40", () => {
    expect(BlockType.EMPTY).toBe(0x40);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// ExternalKind
// ─────────────────────────────────────────────────────────────────────────────

describe("ExternalKind", () => {
  it("FUNCTION = 0x00", () => {
    expect(ExternalKind.FUNCTION).toBe(0x00);
  });

  it("TABLE = 0x01", () => {
    expect(ExternalKind.TABLE).toBe(0x01);
  });

  it("MEMORY = 0x02", () => {
    expect(ExternalKind.MEMORY).toBe(0x02);
  });

  it("GLOBAL = 0x03", () => {
    expect(ExternalKind.GLOBAL).toBe(0x03);
  });

  it("all four external kinds are distinct", () => {
    const kinds = [
      ExternalKind.FUNCTION,
      ExternalKind.TABLE,
      ExternalKind.MEMORY,
      ExternalKind.GLOBAL,
    ];
    const unique = new Set(kinds);
    expect(unique.size).toBe(4);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// FUNCREF
// ─────────────────────────────────────────────────────────────────────────────

describe("FUNCREF", () => {
  it("FUNCREF = 0x70", () => {
    expect(FUNCREF).toBe(0x70);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// FuncType
// ─────────────────────────────────────────────────────────────────────────────

describe("FuncType", () => {
  it("construction with params and results", () => {
    const ft = makeFuncType([ValueType.I32, ValueType.I64], [ValueType.F64]);
    expect(ft.params).toEqual([ValueType.I32, ValueType.I64]);
    expect(ft.results).toEqual([ValueType.F64]);
  });

  it("with empty params and empty results", () => {
    const ft = makeFuncType([], []);
    expect(ft.params).toEqual([]);
    expect(ft.results).toEqual([]);
    expect(ft.params.length).toBe(0);
    expect(ft.results.length).toBe(0);
  });

  it("with multiple params (variadic style)", () => {
    const ft = makeFuncType(
      [ValueType.I32, ValueType.I32, ValueType.I32],
      [ValueType.I32]
    );
    expect(ft.params.length).toBe(3);
    expect(ft.results.length).toBe(1);
    expect(ft.params[0]).toBe(ValueType.I32);
    expect(ft.params[1]).toBe(ValueType.I32);
    expect(ft.params[2]).toBe(ValueType.I32);
    expect(ft.results[0]).toBe(ValueType.I32);
  });

  it("with only results (no params)", () => {
    const ft = makeFuncType([], [ValueType.I32]);
    expect(ft.params).toEqual([]);
    expect(ft.results).toEqual([ValueType.I32]);
  });

  it("equality — two identical FuncTypes have equal fields", () => {
    const ft1 = makeFuncType([ValueType.I32], [ValueType.I64]);
    const ft2 = makeFuncType([ValueType.I32], [ValueType.I64]);
    expect(ft1.params).toEqual(ft2.params);
    expect(ft1.results).toEqual(ft2.results);
  });

  it("is structurally immutable — params array is frozen", () => {
    const ft = makeFuncType([ValueType.I32], [ValueType.I64]);
    expect(Object.isFrozen(ft.params)).toBe(true);
    expect(Object.isFrozen(ft.results)).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Limits
// ─────────────────────────────────────────────────────────────────────────────

describe("Limits", () => {
  it("with only min (max = null)", () => {
    const limits: Limits = { min: 1, max: null };
    expect(limits.min).toBe(1);
    expect(limits.max).toBeNull();
  });

  it("with min and max", () => {
    const limits: Limits = { min: 2, max: 16 };
    expect(limits.min).toBe(2);
    expect(limits.max).toBe(16);
  });

  it("min = 0 is valid (no initial pages)", () => {
    const limits: Limits = { min: 0, max: null };
    expect(limits.min).toBe(0);
    expect(limits.max).toBeNull();
  });

  it("min = max is valid (fixed size)", () => {
    const limits: Limits = { min: 4, max: 4 };
    expect(limits.min).toBe(limits.max);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// MemoryType
// ─────────────────────────────────────────────────────────────────────────────

describe("MemoryType", () => {
  it("construction with unbounded limits", () => {
    const mt: MemoryType = { limits: { min: 1, max: null } };
    expect(mt.limits.min).toBe(1);
    expect(mt.limits.max).toBeNull();
  });

  it("construction with bounded limits", () => {
    const mt: MemoryType = { limits: { min: 1, max: 8 } };
    expect(mt.limits.min).toBe(1);
    expect(mt.limits.max).toBe(8);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// TableType
// ─────────────────────────────────────────────────────────────────────────────

describe("TableType", () => {
  it("construction with funcref and unbounded limits", () => {
    const tt: TableType = { elementType: FUNCREF, limits: { min: 10, max: null } };
    expect(tt.elementType).toBe(0x70);
    expect(tt.limits.min).toBe(10);
    expect(tt.limits.max).toBeNull();
  });

  it("construction with funcref and bounded limits", () => {
    const tt: TableType = { elementType: FUNCREF, limits: { min: 0, max: 100 } };
    expect(tt.elementType).toBe(FUNCREF);
    expect(tt.limits.max).toBe(100);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// GlobalType
// ─────────────────────────────────────────────────────────────────────────────

describe("GlobalType", () => {
  it("immutable i32 global", () => {
    const gt: GlobalType = { valueType: ValueType.I32, mutable: false };
    expect(gt.valueType).toBe(ValueType.I32);
    expect(gt.mutable).toBe(false);
  });

  it("mutable f64 global", () => {
    const gt: GlobalType = { valueType: ValueType.F64, mutable: true };
    expect(gt.valueType).toBe(ValueType.F64);
    expect(gt.mutable).toBe(true);
  });

  it("mutable and immutable are distinguishable", () => {
    const immut: GlobalType = { valueType: ValueType.I32, mutable: false };
    const mut: GlobalType = { valueType: ValueType.I32, mutable: true };
    expect(immut.mutable).not.toBe(mut.mutable);
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
    expect(imp.moduleName).toBe("env");
    expect(imp.name).toBe("add");
    expect(imp.kind).toBe(ExternalKind.FUNCTION);
    expect(imp.typeInfo).toBe(2);
  });

  it("table import", () => {
    const tt: TableType = { elementType: FUNCREF, limits: { min: 1, max: null } };
    const imp: Import = {
      moduleName: "env",
      name: "table",
      kind: ExternalKind.TABLE,
      typeInfo: tt,
    };
    expect(imp.kind).toBe(ExternalKind.TABLE);
    expect(imp.typeInfo).toEqual(tt);
  });

  it("memory import", () => {
    const mt: MemoryType = { limits: { min: 1, max: null } };
    const imp: Import = {
      moduleName: "env",
      name: "memory",
      kind: ExternalKind.MEMORY,
      typeInfo: mt,
    };
    expect(imp.kind).toBe(ExternalKind.MEMORY);
    expect(imp.typeInfo).toEqual(mt);
  });

  it("global import", () => {
    const gt: GlobalType = { valueType: ValueType.I32, mutable: false };
    const imp: Import = {
      moduleName: "env",
      name: "stackPointer",
      kind: ExternalKind.GLOBAL,
      typeInfo: gt,
    };
    expect(imp.kind).toBe(ExternalKind.GLOBAL);
    expect(imp.typeInfo).toEqual(gt);
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
    expect(exp.name).toBe("main");
    expect(exp.kind).toBe(ExternalKind.FUNCTION);
    expect(exp.index).toBe(0);
  });

  it("memory export", () => {
    const exp: Export = {
      name: "memory",
      kind: ExternalKind.MEMORY,
      index: 0,
    };
    expect(exp.kind).toBe(ExternalKind.MEMORY);
    expect(exp.index).toBe(0);
  });

  it("global export", () => {
    const exp: Export = {
      name: "stackPointer",
      kind: ExternalKind.GLOBAL,
      index: 1,
    };
    expect(exp.kind).toBe(ExternalKind.GLOBAL);
    expect(exp.index).toBe(1);
  });

  it("table export", () => {
    const exp: Export = {
      name: "__indirect_function_table",
      kind: ExternalKind.TABLE,
      index: 0,
    };
    expect(exp.kind).toBe(ExternalKind.TABLE);
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
    expect(glob.globalType.valueType).toBe(ValueType.I32);
    expect(glob.globalType.mutable).toBe(false);
    expect(glob.initExpr).toEqual(new Uint8Array([0x41, 0x2a, 0x0b]));
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
    expect(glob.globalType.mutable).toBe(true);
    expect(glob.initExpr.length).toBe(10);
    expect(glob.initExpr[0]).toBe(0x44); // f64.const opcode
    expect(glob.initExpr[glob.initExpr.length - 1]).toBe(0x0b); // end
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
    expect(elem.tableIndex).toBe(0);
    expect(elem.offsetExpr).toEqual(new Uint8Array([0x41, 0x00, 0x0b]));
    expect(elem.functionIndices).toEqual([0, 1, 2]);
    expect(elem.functionIndices.length).toBe(3);
  });

  it("empty functionIndices is valid", () => {
    const elem: Element = {
      tableIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]),
      functionIndices: [],
    };
    expect(elem.functionIndices.length).toBe(0);
  });

  it("non-zero table offset", () => {
    // i32.const 5; end  →  [0x41, 0x05, 0x0B]
    const elem: Element = {
      tableIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x05, 0x0b]),
      functionIndices: [10, 11],
    };
    expect(elem.offsetExpr[1]).toBe(5);
    expect(elem.functionIndices[0]).toBe(10);
    expect(elem.functionIndices[1]).toBe(11);
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
    expect(seg.memoryIndex).toBe(0);
    expect(seg.offsetExpr).toEqual(new Uint8Array([0x41, 0x00, 0x0b]));
    expect(seg.data).toEqual(new Uint8Array([0x48, 0x69]));
    expect(seg.data.length).toBe(2);
  });

  it("empty data segment is valid", () => {
    const seg: DataSegment = {
      memoryIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x00, 0x0b]),
      data: new Uint8Array(0),
    };
    expect(seg.data.length).toBe(0);
  });

  it("non-zero offset", () => {
    // i32.const 256; end  →  [0x41, 0x80, 0x02, 0x0B] (LEB128 256 = 0x80 0x02)
    const seg: DataSegment = {
      memoryIndex: 0,
      offsetExpr: new Uint8Array([0x41, 0x80, 0x02, 0x0b]),
      data: new Uint8Array([0xde, 0xad, 0xbe, 0xef]),
    };
    expect(seg.offsetExpr.length).toBe(4);
    expect(seg.data).toEqual(new Uint8Array([0xde, 0xad, 0xbe, 0xef]));
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
    expect(body.locals).toEqual([ValueType.I32, ValueType.I32]);
    expect(body.code).toEqual(
      new Uint8Array([0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b])
    );
    expect(body.locals.length).toBe(2);
    expect(body.code.length).toBe(6);
  });

  it("no locals — empty locals list", () => {
    const body: FunctionBody = {
      locals: [],
      code: new Uint8Array([0x0b]), // just end
    };
    expect(body.locals.length).toBe(0);
    expect(body.code[0]).toBe(0x0b);
  });

  it("mixed local types", () => {
    const body: FunctionBody = {
      locals: [ValueType.I32, ValueType.F64, ValueType.I64],
      code: new Uint8Array([0x0b]),
    };
    expect(body.locals[0]).toBe(ValueType.I32);
    expect(body.locals[1]).toBe(ValueType.F64);
    expect(body.locals[2]).toBe(ValueType.I64);
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
    expect(cs.name).toBe("name");
    expect(cs.data).toEqual(new Uint8Array([0x01, 0x02, 0x03]));
  });

  it("empty data is valid", () => {
    const cs: CustomSection = { name: "producers", data: new Uint8Array(0) };
    expect(cs.name).toBe("producers");
    expect(cs.data.length).toBe(0);
  });

  it("any name is valid", () => {
    const cs: CustomSection = {
      name: "my.custom.tool",
      data: new Uint8Array([0xff]),
    };
    expect(cs.name).toBe("my.custom.tool");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// WasmModule
// ─────────────────────────────────────────────────────────────────────────────

describe("WasmModule", () => {
  it("starts with all empty arrays and null start", () => {
    const mod = new WasmModule();
    expect(mod.types).toEqual([]);
    expect(mod.imports).toEqual([]);
    expect(mod.functions).toEqual([]);
    expect(mod.tables).toEqual([]);
    expect(mod.memories).toEqual([]);
    expect(mod.globals).toEqual([]);
    expect(mod.exports).toEqual([]);
    expect(mod.start).toBeNull();
    expect(mod.elements).toEqual([]);
    expect(mod.code).toEqual([]);
    expect(mod.data).toEqual([]);
    expect(mod.customs).toEqual([]);
  });

  it("can be populated with types and functions", () => {
    const mod = new WasmModule();
    const ft = makeFuncType([ValueType.I32], [ValueType.I32]);
    mod.types.push(ft);
    mod.functions.push(0);

    expect(mod.types.length).toBe(1);
    expect(mod.types[0]).toEqual(ft);
    expect(mod.functions[0]).toBe(0);
  });

  it("can be populated with memories", () => {
    const mod = new WasmModule();
    const mt: MemoryType = { limits: { min: 1, max: null } };
    mod.memories.push(mt);
    expect(mod.memories.length).toBe(1);
    expect(mod.memories[0].limits.min).toBe(1);
  });

  it("can set the start function", () => {
    const mod = new WasmModule();
    expect(mod.start).toBeNull();
    mod.start = 3;
    expect(mod.start).toBe(3);
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

    expect(mod.imports.length).toBe(1);
    expect(mod.exports.length).toBe(1);
    expect(mod.imports[0].moduleName).toBe("env");
    expect(mod.exports[0].name).toBe("main");
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

    expect(mod.globals.length).toBe(1);
    expect(mod.elements.length).toBe(1);
    expect(mod.data.length).toBe(1);
    expect(mod.customs.length).toBe(1);
  });

  it("multiple instances are independent", () => {
    const mod1 = new WasmModule();
    const mod2 = new WasmModule();
    mod1.types.push(makeFuncType([ValueType.I32], []));

    // mod2 should still be empty
    expect(mod2.types.length).toBe(0);
    expect(mod1.types.length).toBe(1);
  });
});
