import { describe, expect, it } from "vitest";
import {
  makeFuncType,
  ValueType,
  ExternalKind,
  FUNCREF,
  WasmModule,
} from "@coding-adventures/wasm-types";
import { getOpcodeByName } from "@coding-adventures/wasm-opcodes";
import { encodeSigned } from "@coding-adventures/wasm-leb128";
import { WasmModuleParser } from "@coding-adventures/wasm-module-parser";
import { validate } from "@coding-adventures/wasm-validator";

import { encodeModule, WASM_MAGIC, WASM_VERSION, WasmEncodeError } from "../src/index.js";

function constExpr(value: number): Uint8Array {
  const i32Const = getOpcodeByName("i32.const")!.opcode;
  const end = getOpcodeByName("end")!.opcode;
  return new Uint8Array([i32Const, ...encodeSigned(value), end]);
}

describe("encodeModule", () => {
  it("encodes a simple exported function into a valid wasm binary", () => {
    const i32Const = getOpcodeByName("i32.const")!.opcode;
    const end = getOpcodeByName("end")!.opcode;

    const module = new WasmModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.exports.push({
      name: "answer",
      kind: ExternalKind.FUNCTION,
      index: 0,
    });
    module.code.push({
      locals: [],
      code: new Uint8Array([i32Const, ...encodeSigned(42), end]),
    });

    const bytes = encodeModule(module);
    expect(bytes.slice(0, 4)).toEqual(WASM_MAGIC);
    expect(bytes.slice(4, 8)).toEqual(WASM_VERSION);

    const parsed = new WasmModuleParser().parse(bytes);
    expect(parsed.exports[0]?.name).toBe("answer");
    expect(() => validate(parsed)).not.toThrow();
  });

  it("encodes every standard section and preserves parsed structure", () => {
    const module = new WasmModule();
    module.customs.push({
      name: "meta",
      data: Uint8Array.from([0xaa, 0xbb]),
    });
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.types.push(makeFuncType([ValueType.I32], []));
    module.imports.push({
      moduleName: "env",
      name: "log",
      kind: ExternalKind.FUNCTION,
      typeInfo: 1,
    });
    module.imports.push({
      moduleName: "env",
      name: "table",
      kind: ExternalKind.TABLE,
      typeInfo: {
        elementType: FUNCREF,
        limits: { min: 1, max: 2 },
      },
    });
    module.imports.push({
      moduleName: "env",
      name: "memory",
      kind: ExternalKind.MEMORY,
      typeInfo: {
        limits: { min: 1, max: 3 },
      },
    });
    module.imports.push({
      moduleName: "env",
      name: "flag",
      kind: ExternalKind.GLOBAL,
      typeInfo: {
        valueType: ValueType.I32,
        mutable: true,
      },
    });
    module.functions.push(0);
    module.tables.push({
      elementType: FUNCREF,
      limits: { min: 1, max: null },
    });
    module.memories.push({
      limits: { min: 1, max: null },
    });
    module.globals.push({
      globalType: { valueType: ValueType.I32, mutable: false },
      initExpr: constExpr(7),
    });
    module.exports.push({
      name: "answer",
      kind: ExternalKind.FUNCTION,
      index: 1,
    });
    module.exports.push({
      name: "table_local",
      kind: ExternalKind.TABLE,
      index: 1,
    });
    module.exports.push({
      name: "memory_local",
      kind: ExternalKind.MEMORY,
      index: 1,
    });
    module.exports.push({
      name: "global_local",
      kind: ExternalKind.GLOBAL,
      index: 1,
    });
    module.start = 1;
    module.elements.push({
      tableIndex: 1,
      offsetExpr: constExpr(0),
      functionIndices: [1],
    });
    module.code.push({
      locals: [
        ValueType.I32,
        ValueType.I32,
        ValueType.F32,
        ValueType.F32,
        ValueType.I32,
      ],
      code: new Uint8Array([
        getOpcodeByName("i32.const")!.opcode,
        ...encodeSigned(42),
        getOpcodeByName("end")!.opcode,
      ]),
    });
    module.data.push({
      memoryIndex: 1,
      offsetExpr: constExpr(4),
      data: Uint8Array.from([65, 66]),
    });

    const parsed = new WasmModuleParser().parse(encodeModule(module));

    expect(parsed.customs[0]).toEqual({
      name: "meta",
      data: Uint8Array.from([0xaa, 0xbb]),
    });
    expect(parsed.imports.map((entry) => entry.name)).toEqual([
      "log",
      "table",
      "memory",
      "flag",
    ]);
    expect(parsed.imports.map((entry) => entry.kind)).toEqual([
      ExternalKind.FUNCTION,
      ExternalKind.TABLE,
      ExternalKind.MEMORY,
      ExternalKind.GLOBAL,
    ]);
    expect(parsed.functions).toEqual([0]);
    expect(parsed.tables).toEqual([
      {
        elementType: FUNCREF,
        limits: { min: 1, max: null },
      },
    ]);
    expect(parsed.memories).toEqual([{ limits: { min: 1, max: null } }]);
    expect(parsed.globals[0]?.globalType).toEqual({
      valueType: ValueType.I32,
      mutable: false,
    });
    expect(parsed.exports.map((entry) => entry.name)).toEqual([
      "answer",
      "table_local",
      "memory_local",
      "global_local",
    ]);
    expect(parsed.start).toBe(1);
    expect(parsed.elements).toEqual([
      {
        tableIndex: 1,
        offsetExpr: constExpr(0),
        functionIndices: [1],
      },
    ]);
    expect(parsed.code[0]?.locals).toEqual([
      ValueType.I32,
      ValueType.I32,
      ValueType.F32,
      ValueType.F32,
      ValueType.I32,
    ]);
    expect(parsed.data).toEqual([
      {
        memoryIndex: 1,
        offsetExpr: constExpr(4),
        data: Uint8Array.from([65, 66]),
      },
    ]);
  });

  it("rejects function imports without a numeric type index", () => {
    const module = new WasmModule();
    module.imports.push({
      moduleName: "env",
      name: "bad",
      kind: ExternalKind.FUNCTION,
      typeInfo: {
        limits: { min: 1, max: null },
      },
    });

    expect(() => encodeModule(module)).toThrowError(
      new WasmEncodeError("function imports require a numeric type index"),
    );
  });

  it("rejects unsupported import kinds", () => {
    const module = new WasmModule();
    module.imports.push({
      moduleName: "env",
      name: "mystery",
      kind: 99 as ExternalKind,
      typeInfo: 0,
    });

    expect(() => encodeModule(module)).toThrowError(
      new WasmEncodeError("unsupported import kind: 99"),
    );
  });
});
