import { describe, expect, it } from "vitest";
import {
  ExternalKind,
  ValueType,
  WasmModule,
  makeFuncType,
} from "@coding-adventures/wasm-types";
import type {
  Export,
  FunctionBody,
  Global,
  Import,
} from "@coding-adventures/wasm-types";
import {
  ValidationError,
  ValidationErrorKind,
  validate,
  validateConstExpr,
  validateStructure,
} from "../src/index.js";

function bytes(...values: number[]): Uint8Array {
  return Uint8Array.from(values);
}

function makeBody(code: number[], locals: ValueType[] = []): FunctionBody {
  return {
    locals,
    code: bytes(...code),
  };
}

function makeModule(): WasmModule {
  return new WasmModule();
}

function memory(min: number, max: number | null = null) {
  return { limits: { min, max } };
}

function table(min: number, max: number | null = null) {
  return { elementType: 0x70, limits: { min, max } };
}

describe("wasm-validator", () => {
  it("validates an empty module", () => {
    expect(() => validate(makeModule())).not.toThrow();
  });

  it("validates a simple i32.add function and caches locals", () => {
    const module = makeModule();
    module.types.push(makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]));
    module.functions.push(0);
    module.code.push(makeBody([0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b], [ValueType.I32]));

    const validated = validate(module);

    expect(validated.funcTypes).toHaveLength(1);
    expect(validated.funcLocals[0]).toEqual([
      ValueType.I32,
      ValueType.I32,
      ValueType.I32,
    ]);
  });

  it("accepts unreachable dead code after br", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x02, 0x7f, // block (result i32)
        0x41, 0x07, // i32.const 7
        0x0c, 0x00, // br 0
        0x43, 0x00, 0x00, 0x00, 0x00, // f32.const 0
        0x7c, // i64.add
        0x0b, // end block
        0x0b, // end function
      ])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("accepts unreachable dead code after return", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x41, 0x01, // i32.const 1
        0x0f, // return
        0x43, 0x00, 0x00, 0x00, 0x00, // f32.const 0
        0x7c, // i64.add
        0x0b, // end
      ])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("accepts br_if when the not-taken path preserves the stack", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x02, 0x7f, // block (result i32)
        0x41, 0x2a, // i32.const 42
        0x41, 0x00, // i32.const 0
        0x0d, 0x00, // br_if 0
        0x0b, // end block
        0x0b, // end function
      ])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("allows global.get in a constant expression when it references an imported global", () => {
    const module = makeModule();
    module.imports.push({
      moduleName: "env",
      name: "seed",
      kind: ExternalKind.GLOBAL,
      typeInfo: { valueType: ValueType.I32, mutable: false },
    } as Import);
    module.globals.push({
      globalType: { valueType: ValueType.I32, mutable: false },
      initExpr: bytes(0x23, 0x00, 0x0b),
    } as Global);

    const indexSpaces = validateStructure(module);

    expect(indexSpaces.globalTypes).toHaveLength(2);
  });

  it("rejects multiple memories", () => {
    const module = makeModule();
    module.memories.push(memory(1), memory(1));

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.MULTIPLE_MEMORIES
    );
  });

  it("rejects multiple tables", () => {
    const module = makeModule();
    module.imports.push({
      moduleName: "env",
      name: "table",
      kind: ExternalKind.TABLE,
      typeInfo: table(1),
    } as Import);
    module.tables.push(table(1));

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.MULTIPLE_TABLES
    );
  });

  it("rejects memory limits above the WASM 1.0 maximum", () => {
    const module = makeModule();
    module.memories.push(memory(1, 70000));

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.MEMORY_LIMIT_EXCEEDED
    );
  });

  it("rejects memory limits when min exceeds max", () => {
    const module = makeModule();
    module.memories.push(memory(5, 3));

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.MEMORY_LIMIT_ORDER
    );
  });

  it("rejects table limits when min exceeds max", () => {
    const module = makeModule();
    module.tables.push(table(5, 3));

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.TABLE_LIMIT_ORDER
    );
  });

  it("validates element and data segments against existing table, memory, and function spaces", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x0b]));
    module.tables.push(table(1, 1));
    module.memories.push(memory(1, 1));
    module.elements.push({
      tableIndex: 0,
      offsetExpr: bytes(0x41, 0x00, 0x0b),
      functionIndices: [0],
    });
    module.data.push({
      memoryIndex: 0,
      offsetExpr: bytes(0x41, 0x00, 0x0b),
      data: bytes(0xde, 0xad, 0xbe, 0xef),
    });

    expect(() => validateStructure(module)).not.toThrow();
  });

  it("rejects element segments with invalid function indices", () => {
    const module = makeModule();
    module.tables.push(table(1, 1));
    module.elements.push({
      tableIndex: 0,
      offsetExpr: bytes(0x41, 0x00, 0x0b),
      functionIndices: [3],
    });

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.INVALID_FUNC_INDEX
    );
  });

  it("rejects data segments with invalid memory indices", () => {
    const module = makeModule();
    module.data.push({
      memoryIndex: 0,
      offsetExpr: bytes(0x41, 0x00, 0x0b),
      data: bytes(0x01),
    });

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.INVALID_MEMORY_INDEX
    );
  });

  it("rejects duplicate export names", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x0b]));
    module.exports.push(
      { name: "main", kind: ExternalKind.FUNCTION, index: 0 } as Export,
      { name: "main", kind: ExternalKind.FUNCTION, index: 0 } as Export
    );

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.DUPLICATE_EXPORT_NAME
    );
  });

  it("rejects export indices that point past the function space", () => {
    const module = makeModule();
    module.exports.push({
      name: "main",
      kind: ExternalKind.FUNCTION,
      index: 0,
    } as Export);

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.EXPORT_INDEX_OUT_OF_RANGE
    );
  });

  it("rejects a local function with an invalid type index", () => {
    const module = makeModule();
    module.functions.push(99);
    module.code.push(makeBody([0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_TYPE_INDEX
    );
  });

  it("rejects mismatched function/code section counts", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_FUNC_INDEX
    );
  });

  it("rejects a start function that is not () -> ()", () => {
    const module = makeModule();
    module.types.push(makeFuncType([ValueType.I32], []));
    module.functions.push(0);
    module.code.push(makeBody([0x0b]));
    module.start = 0;

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.START_FUNCTION_BAD_TYPE
    );
  });

  it("rejects a start function index that is out of range", () => {
    const module = makeModule();
    module.start = 0;

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.INVALID_FUNC_INDEX
    );
  });

  it("rejects invalid constant-expression opcodes", () => {
    const module = makeModule();
    module.globals.push({
      globalType: { valueType: ValueType.I32, mutable: false },
      initExpr: bytes(0x41, 0x01, 0x41, 0x02, 0x6a, 0x0b),
    } as Global);

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INIT_EXPR_INVALID
    );
  });

  it("rejects constant expressions that terminate early or leave the wrong type", () => {
    expectValidationError(
      () =>
        validateConstExpr(
          bytes(0x41, 0x01, 0x0b, 0x41, 0x02),
          ValueType.I32,
          {
            funcTypes: [],
            numImportedFuncs: 0,
            tableTypes: [],
            numImportedTables: 0,
            memoryTypes: [],
            numImportedMemories: 0,
            globalTypes: [],
            numImportedGlobals: 0,
            numTypes: 0,
          }
        ),
      ValidationErrorKind.INIT_EXPR_INVALID
    );
  });

  it("wraps malformed constant-expression encodings as INIT_EXPR_INVALID", () => {
    expectValidationError(
      () =>
        validateConstExpr(
          bytes(0x41, 0x80),
          ValueType.I32,
          {
            funcTypes: [],
            numImportedFuncs: 0,
            tableTypes: [],
            numImportedTables: 0,
            memoryTypes: [],
            numImportedMemories: 0,
            globalTypes: [],
            numImportedGlobals: 0,
            numTypes: 0,
          }
        ),
      ValidationErrorKind.INIT_EXPR_INVALID
    );
  });

  it("rejects local.get when the index is out of range", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x20, 0x63, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_LOCAL_INDEX
    );
  });

  it("validates local.set and local.tee", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.code.push(
      makeBody(
        [
          0x41, 0x05, // i32.const 5
          0x21, 0x00, // local.set 0
          0x41, 0x07, // i32.const 7
          0x22, 0x00, // local.tee 0
          0x1a, // drop
          0x20, 0x00, // local.get 0
          0x0b,
        ],
        [ValueType.I32]
      )
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("rejects type mismatches in numeric instructions", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x43, 0x00, 0x00, 0x00, 0x00, // f32.const 0
        0x41, 0x01, // i32.const 1
        0x6a, // i32.add
        0x0b,
      ])
    );

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects stack underflow in numeric instructions", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x41, 0x01, 0x6a, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.STACK_UNDERFLOW
    );
  });

  it("validates f64 numeric ops and conversion ops", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // f64.const 0
        0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // f64.const 0
        0x61, // f64.eq
        0x1a, // drop
        0x42, 0x01, // i64.const 1
        0xa7, // i32.wrap_i64
        0x1a, // drop
        0x0b,
      ])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("rejects blocks that end with the wrong stack height", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x02, 0x7f, 0x0b, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.STACK_HEIGHT_MISMATCH
    );
  });

  it("validates a reachable if/else with a result", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x41, 0x01, // i32.const 1
        0x04, 0x7f, // if (result i32)
        0x41, 0x02, // i32.const 2
        0x05, // else
        0x41, 0x03, // i32.const 3
        0x0b, // end if
        0x0b, // end function
      ])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("rejects else without a matching if", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x05, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects branch labels that are out of range", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x0c, 0x01, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_LABEL_INDEX
    );
  });

  it("rejects br_table targets with incompatible label types", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x02, 0x7f, // block (result i32)
        0x03, 0x40, // loop
        0x41, 0x00, // i32.const 0
        0x0e, 0x01, 0x00, 0x01, // br_table [0] default=1
        0x0b, // end loop
        0x0b, // end block
        0x0b, // end function
      ])
    );

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects memory operations when no memory exists", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x41, 0x00, // i32.const 0
        0x28, 0x02, 0x00, // i32.load align=2 offset=0
        0x1a, // drop
        0x0b,
      ])
    );

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_MEMORY_INDEX
    );
  });

  it("validates memory store, load, size, and grow when a memory exists", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.memories.push(memory(1, 2));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x41, 0x00, // i32.const 0
        0x41, 0x2a, // i32.const 42
        0x36, 0x02, 0x00, // i32.store align=2 offset=0
        0x41, 0x00, // i32.const 0
        0x28, 0x02, 0x00, // i32.load align=2 offset=0
        0x1a, // drop
        0x3f, 0x00, // memory.size 0
        0x1a, // drop
        0x41, 0x00, // i32.const 0
        0x40, 0x00, // memory.grow 0
        0x1a, // drop
        0x0b,
      ])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("rejects memory alignments larger than the natural width", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.memories.push(memory(1, 1));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x41, 0x00,
        0x28, 0x03, 0x00, // i32.load align=3 (too large)
        0x1a,
        0x0b,
      ])
    );

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects global.set on immutable globals", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.imports.push({
      moduleName: "env",
      name: "flag",
      kind: ExternalKind.GLOBAL,
      typeInfo: { valueType: ValueType.I32, mutable: false },
    } as Import);
    module.functions.push(0);
    module.code.push(makeBody([0x41, 0x01, 0x24, 0x00, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.IMMUTABLE_GLOBAL_WRITE
    );
  });

  it("validates global.get on an imported global", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.imports.push({
      moduleName: "env",
      name: "answer",
      kind: ExternalKind.GLOBAL,
      typeInfo: { valueType: ValueType.I32, mutable: false },
    } as Import);
    module.functions.push(0);
    module.code.push(makeBody([0x23, 0x00, 0x0b]));

    expect(() => validate(module)).not.toThrow();
  });

  it("rejects global.get when the global index is out of range", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0);
    module.code.push(makeBody([0x23, 0x00, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_GLOBAL_INDEX
    );
  });

  it("rejects select when the candidate values have different types", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(
      makeBody([
        0x41, 0x01, // i32.const 1
        0x43, 0x00, 0x00, 0x00, 0x00, // f32.const 0
        0x41, 0x00, // i32.const 0
        0x1b, // select
        0x1a, // drop (would only run if select validated)
        0x0b,
      ])
    );

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("validates a direct call through the function index space", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.types.push(makeFuncType([], [ValueType.I32]));
    module.functions.push(0, 1);
    module.code.push(
      makeBody([0x41, 0x07, 0x0b]),
      makeBody([0x10, 0x00, 0x0b])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("rejects direct calls to invalid function indices", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x10, 0x01, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_FUNC_INDEX
    );
  });

  it("rejects call_indirect when no table exists", () => {
    const module = makeModule();
    module.types.push(
      makeFuncType([ValueType.I32], [ValueType.I32]),
      makeFuncType([], [ValueType.I32])
    );
    module.functions.push(1);
    module.code.push(
      makeBody([
        0x41, 0x29, // i32.const 41
        0x41, 0x00, // i32.const 0 (table slot)
        0x11, 0x00, 0x00, // call_indirect type 0 table 0
        0x0b,
      ])
    );

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.INVALID_TABLE_INDEX
    );
  });

  it("validates call_indirect when the table and type exist", () => {
    const module = makeModule();
    module.types.push(
      makeFuncType([ValueType.I32], [ValueType.I32]),
      makeFuncType([], [ValueType.I32])
    );
    module.tables.push(table(1, 1));
    module.functions.push(1);
    module.code.push(
      makeBody([
        0x41, 0x29, // i32.const 41
        0x41, 0x00, // i32.const 0 (table slot)
        0x11, 0x00, 0x00, // call_indirect type 0 table 0
        0x0b,
      ])
    );

    expect(() => validate(module)).not.toThrow();
  });

  it("rejects function imports without numeric type indices", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.imports.push({
      moduleName: "env",
      name: "bad",
      kind: ExternalKind.FUNCTION,
      typeInfo: table(1, 1),
    } as unknown as Import);

    expectValidationError(
      () => validateStructure(module),
      ValidationErrorKind.INVALID_TYPE_INDEX
    );
  });

  it("rejects function bodies that omit the final end opcode", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x01]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects trailing bytes after the final function end", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x0b, 0x01]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects unknown opcodes", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0xff, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects unsupported blocktype bytes", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x02, 0x01, 0x0b, 0x0b]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects truncated fixed-width immediates", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x43, 0x00, 0x00]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });

  it("rejects malformed unsigned LEB128 immediates", () => {
    const module = makeModule();
    module.types.push(makeFuncType([], []));
    module.functions.push(0);
    module.code.push(makeBody([0x20, 0x80]));

    expectValidationError(
      () => validate(module),
      ValidationErrorKind.TYPE_MISMATCH
    );
  });
});

function expectValidationError(
  action: () => unknown,
  kind: ValidationErrorKind
): void {
  expect(action).toThrowError(ValidationError);
  try {
    action();
  } catch (error) {
    expect(error).toBeInstanceOf(ValidationError);
    expect((error as ValidationError).kind).toBe(kind);
  }
}
