/**
 * Starlark Interpreter Tests — Comprehensive test suite.
 *
 * These tests verify the interpreter's core functionality:
 *
 * 1. **Basic execution** — Running bytecode and inspecting results.
 * 2. **File resolvers** — Dict resolver, function resolver, error cases.
 * 3. **Load caching** — Each file evaluated at most once.
 * 4. **LOAD_MODULE override** — The interpreter's load() mechanism.
 * 5. **Convenience functions** — interpret(), interpretBytecode(), etc.
 * 6. **Mini VM handlers** — All basic opcode handlers work correctly.
 * 7. **Error handling** — Missing resolvers, missing files, etc.
 *
 * Since the starlark-ast-to-bytecode-compiler TypeScript package does not
 * exist yet, most tests use hand-crafted CodeObjects (bytecode). This
 * actually makes the tests more precise — we know exactly what bytecode
 * is being executed, without depending on the compiler's output.
 */

import { describe, it, expect, vi } from "vitest";
import { writeFileSync, mkdtempSync, rmSync } from "fs";
import { join } from "path";
import { tmpdir } from "os";
import {
  StarlarkInterpreter,
  interpret,
  interpretBytecode,
  interpretFile,
  dictResolver,
  resolveFile,
  createMiniStarlarkVM,
  registerMiniStarlarkHandlers,
  FileNotFoundError,
  Op,
} from "../src/index.js";
import type {
  FileResolver,
  StarlarkResult,
  CompileFn,
} from "../src/index.js";
import {
  GenericVM,
  type CodeObject,
} from "@coding-adventures/virtual-machine";

// =========================================================================
// Helper: Create simple CodeObjects for testing
// =========================================================================

/**
 * Build a CodeObject that assigns a constant to a variable.
 *
 *     x = <value>
 *
 * Bytecode: LOAD_CONST 0, STORE_NAME 0, HALT
 */
function makeAssign(name: string, value: number | string): CodeObject {
  return {
    instructions: [
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.STORE_NAME, operand: 0 },
      { opcode: Op.HALT },
    ],
    constants: [value],
    names: [name],
  };
}

/**
 * Build a CodeObject that adds two constants and stores the result.
 *
 *     x = a + b
 *
 * Bytecode: LOAD_CONST 0, LOAD_CONST 1, ADD, STORE_NAME 0, HALT
 */
function makeAdd(name: string, a: number, b: number): CodeObject {
  return {
    instructions: [
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.LOAD_CONST, operand: 1 },
      { opcode: Op.ADD },
      { opcode: Op.STORE_NAME, operand: 0 },
      { opcode: Op.HALT },
    ],
    constants: [a, b],
    names: [name],
  };
}

/**
 * Build a CodeObject that prints a constant value.
 *
 *     print(<value>)
 *
 * Bytecode: LOAD_CONST 0, PRINT, HALT
 */
function makePrint(value: number | string): CodeObject {
  return {
    instructions: [
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.PRINT },
      { opcode: Op.HALT },
    ],
    constants: [value],
    names: [],
  };
}

/**
 * Build a CodeObject that loads a module and imports a symbol.
 *
 *     load("module_label", "symbol_name")
 *
 * Bytecode: LOAD_MODULE 0, IMPORT_FROM 1, STORE_NAME 1, POP, HALT
 */
function makeLoad(
  moduleLabel: string,
  symbolName: string,
): CodeObject {
  return {
    instructions: [
      { opcode: Op.LOAD_MODULE, operand: 0 },
      { opcode: Op.IMPORT_FROM, operand: 1 },
      { opcode: Op.STORE_NAME, operand: 1 },
      { opcode: Op.POP },
      { opcode: Op.HALT },
    ],
    constants: [],
    names: [moduleLabel, symbolName],
  };
}

// =========================================================================
// Tests: Basic Bytecode Execution
// =========================================================================

describe("Basic bytecode execution", () => {
  it("should assign a constant to a variable", () => {
    const code = makeAssign("x", 42);
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(42);
  });

  it("should assign a string constant", () => {
    const code = makeAssign("greeting", "hello");
    const result = interpretBytecode(code);
    expect(result.variables["greeting"]).toBe("hello");
  });

  it("should add two numbers", () => {
    const code = makeAdd("sum", 10, 32);
    const result = interpretBytecode(code);
    expect(result.variables["sum"]).toBe(42);
  });

  it("should capture print output", () => {
    const code = makePrint(42);
    const result = interpretBytecode(code);
    expect(result.output).toEqual(["42"]);
  });

  it("should capture string print output", () => {
    const code = makePrint("hello world");
    const result = interpretBytecode(code);
    expect(result.output).toEqual(["hello world"]);
  });

  it("should return execution traces", () => {
    const code = makeAssign("x", 1);
    const result = interpretBytecode(code);
    expect(result.traces.length).toBeGreaterThan(0);
  });

  it("should handle subtraction", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.SUB },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [50, 8],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(42);
  });

  it("should handle multiplication", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [6, 7],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(42);
  });

  it("should handle division", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.DIV },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [84, 2],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(42);
  });

  it("should throw on division by zero", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.DIV },
        { opcode: Op.HALT },
      ],
      constants: [42, 0],
      names: [],
    };
    expect(() => interpretBytecode(code)).toThrow("Division by zero");
  });
});

// =========================================================================
// Tests: Additional Arithmetic Operations
// =========================================================================

describe("Additional arithmetic operations", () => {
  it("should handle floor division", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.FLOOR_DIV },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [7, 2],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(3);
  });

  it("should handle modulo", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MOD },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [10, 3],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });

  it("should handle negation", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NEGATE },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(-42);
  });

  it("should handle string concatenation via ADD", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: ["hello ", "world"],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe("hello world");
  });

  it("should handle string repetition via MUL", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: ["ab", 3],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe("ababab");
  });
});

// =========================================================================
// Tests: Comparison Operations
// =========================================================================

describe("Comparison operations", () => {
  it("should handle CMP_EQ (equal)", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_EQ },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42, 42],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });

  it("should handle CMP_EQ (not equal)", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_EQ },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42, 43],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(0);
  });

  it("should handle CMP_NE", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_NE },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [1, 2],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });

  it("should handle CMP_LT", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_LT },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [1, 2],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });

  it("should handle CMP_GT", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_GT },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [5, 3],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });

  it("should handle CMP_LE", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_LE },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [5, 5],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });

  it("should handle CMP_GE", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_GE },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [5, 5],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });

  it("should handle NOT", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NOT },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [0],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(1);
  });
});

// =========================================================================
// Tests: Control Flow
// =========================================================================

describe("Control flow", () => {
  it("should handle unconditional JUMP", () => {
    // Jump over the first STORE_NAME to the second one.
    const code: CodeObject = {
      instructions: [
        { opcode: Op.JUMP, operand: 3 },        // 0: jump to 3
        { opcode: Op.LOAD_CONST, operand: 0 },  // 1: skipped
        { opcode: Op.STORE_NAME, operand: 0 },   // 2: skipped
        { opcode: Op.LOAD_CONST, operand: 1 },  // 3: load 99
        { opcode: Op.STORE_NAME, operand: 0 },   // 4: store in x
        { opcode: Op.HALT },
      ],
      constants: [42, 99],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(99);
  });

  it("should handle JUMP_IF_FALSE (takes jump)", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },   // 0: push 0 (falsy)
        { opcode: Op.JUMP_IF_FALSE, operand: 4 }, // 1: jump to 4
        { opcode: Op.LOAD_CONST, operand: 1 },    // 2: skipped
        { opcode: Op.STORE_NAME, operand: 0 },     // 3: skipped
        { opcode: Op.LOAD_CONST, operand: 2 },    // 4: load 99
        { opcode: Op.STORE_NAME, operand: 0 },     // 5: store in x
        { opcode: Op.HALT },
      ],
      constants: [0, 42, 99],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(99);
  });

  it("should handle JUMP_IF_FALSE (does not jump)", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },   // 0: push 1 (truthy)
        { opcode: Op.JUMP_IF_FALSE, operand: 4 }, // 1: no jump
        { opcode: Op.LOAD_CONST, operand: 1 },    // 2: load 42
        { opcode: Op.STORE_NAME, operand: 0 },     // 3: store in x
        { opcode: Op.HALT },
      ],
      constants: [1, 42],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(42);
  });

  it("should handle JUMP_IF_TRUE", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },   // 0: push 1 (truthy)
        { opcode: Op.JUMP_IF_TRUE, operand: 4 },  // 1: jump to 4
        { opcode: Op.LOAD_CONST, operand: 1 },    // 2: skipped
        { opcode: Op.STORE_NAME, operand: 0 },     // 3: skipped
        { opcode: Op.LOAD_CONST, operand: 2 },    // 4: load 99
        { opcode: Op.STORE_NAME, operand: 0 },     // 5: store in x
        { opcode: Op.HALT },
      ],
      constants: [1, 42, 99],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(99);
  });
});

// =========================================================================
// Tests: Stack Operations
// =========================================================================

describe("Stack operations", () => {
  it("should handle POP", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.POP },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42, 99],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(42);
  });

  it("should handle DUP", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.DUP },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 1 },
        { opcode: Op.HALT },
      ],
      constants: [42],
      names: ["x", "y"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(42);
    expect(result.variables["y"]).toBe(42);
  });

  it("should handle LOAD_NONE", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_NONE },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBeNull();
  });

  it("should handle LOAD_TRUE", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_TRUE },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(1);
  });

  it("should handle LOAD_FALSE", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_FALSE },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(0);
  });
});

// =========================================================================
// Tests: Collection Operations
// =========================================================================

describe("Collection operations", () => {
  it("should handle BUILD_LIST", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.BUILD_LIST, operand: 3 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [1, 2, 3],
      names: ["items"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["items"]).toEqual([1, 2, 3]);
  });

  it("should handle BUILD_DICT", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BUILD_DICT, operand: 1 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: ["key", 42],
      names: ["d"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["d"]).toEqual({ key: 42 });
  });

  it("should handle LOAD_SUBSCRIPT on list", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.BUILD_LIST, operand: 3 },
        { opcode: Op.LOAD_CONST, operand: 3 },
        { opcode: Op.LOAD_SUBSCRIPT },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [10, 20, 30, 1],
      names: ["item"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["item"]).toBe(20);
  });
});

// =========================================================================
// Tests: Local Variable Operations
// =========================================================================

describe("Local variable operations", () => {
  it("should handle STORE_LOCAL and LOAD_LOCAL", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_LOCAL, operand: 0 },
        { opcode: Op.LOAD_LOCAL, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42],
      names: ["x"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(42);
  });
});

// =========================================================================
// Tests: File Resolvers
// =========================================================================

describe("File resolvers", () => {
  it("dictResolver should resolve known labels", () => {
    const resolver = dictResolver({
      "//rules/test.star": "x = 42",
    });
    expect(resolver("//rules/test.star")).toBe("x = 42");
  });

  it("dictResolver should throw FileNotFoundError for unknown labels", () => {
    const resolver = dictResolver({});
    expect(() => resolver("//unknown.star")).toThrow(FileNotFoundError);
  });

  it("resolveFile should work with dict resolver", () => {
    const files = { "//test.star": "content" };
    expect(resolveFile(files, "//test.star")).toBe("content");
  });

  it("resolveFile should throw for missing key in dict", () => {
    const files = { "//test.star": "content" };
    expect(() => resolveFile(files, "//missing.star")).toThrow(
      FileNotFoundError,
    );
  });

  it("resolveFile should work with function resolver", () => {
    const resolver = (label: string) => `content of ${label}`;
    expect(resolveFile(resolver, "//test.star")).toBe(
      "content of //test.star",
    );
  });

  it("resolveFile should throw when resolver is null", () => {
    expect(() => resolveFile(null, "//test.star")).toThrow(
      FileNotFoundError,
    );
    expect(() => resolveFile(null, "//test.star")).toThrow(
      "no fileResolver configured",
    );
  });

  it("resolveFile should propagate errors from function resolver", () => {
    const resolver = () => {
      throw new Error("disk error");
    };
    expect(() => resolveFile(resolver, "//test.star")).toThrow("disk error");
  });
});

// =========================================================================
// Tests: StarlarkInterpreter Class
// =========================================================================

describe("StarlarkInterpreter class", () => {
  it("should execute bytecode via interpretBytecode", () => {
    const interp = new StarlarkInterpreter();
    const code = makeAssign("x", 42);
    const result = interp.interpretBytecode(code);
    expect(result.variables["x"]).toBe(42);
  });

  it("should throw when interpret() is called without compileFn", () => {
    const interp = new StarlarkInterpreter();
    expect(() => interp.interpret("x = 42\n")).toThrow(
      "No compile function configured",
    );
  });

  it("should use custom compileFn", () => {
    const mockCompile: CompileFn = (_source: string) => makeAssign("x", 99);
    const interp = new StarlarkInterpreter({ compileFn: mockCompile });
    const result = interp.interpret("anything");
    expect(result.variables["x"]).toBe(99);
  });

  it("should use custom createVMFn", () => {
    let vmCreated = false;
    const customCreateVM = (opts?: { maxRecursionDepth?: number }) => {
      vmCreated = true;
      return createMiniStarlarkVM(opts);
    };
    const interp = new StarlarkInterpreter({ createVMFn: customCreateVM });
    interp.interpretBytecode(makeAssign("x", 1));
    expect(vmCreated).toBe(true);
  });

  it("should respect maxRecursionDepth", () => {
    const interp = new StarlarkInterpreter({ maxRecursionDepth: 10 });
    // We can't easily test recursion without a full compiler, but
    // we can verify the setting is passed through.
    expect(interp.maxRecursionDepth).toBe(10);
  });

  it("should clear the load cache", () => {
    const interp = new StarlarkInterpreter();
    // Manually populate cache for testing
    (interp as unknown as { _loadCache: Map<string, unknown> })._loadCache.set(
      "//test.star",
      { x: 1 },
    );
    expect(interp.getCache().size).toBe(1);
    interp.clearCache();
    expect(interp.getCache().size).toBe(0);
  });
});

// =========================================================================
// Tests: LOAD_MODULE Override (the core load() mechanism)
// =========================================================================

describe("LOAD_MODULE override", () => {
  /**
   * To test the load mechanism, we need a compile function that produces
   * appropriate bytecode. We create a mock compiler that recognizes
   * simple patterns and produces the right bytecode.
   */
  function makeMockCompiler(
    moduleFiles: Record<string, CodeObject>,
  ): CompileFn {
    return (source: string): CodeObject => {
      // If the source matches a known module's source, return its bytecode
      for (const [, code] of Object.entries(moduleFiles)) {
        // Check if this source was intended for a module
        if (source.trim() === "MODULE_SOURCE") {
          return code;
        }
      }
      // For the main program, return a load bytecode
      return makeAssign("x", 42);
    };
  }

  it("should override LOAD_MODULE to resolve files", () => {
    // Create a module that defines x = 99
    const moduleCode = makeAssign("x", 99);

    // The main code loads from the module and imports x
    const mainCode = makeLoad("//lib.star", "x");

    // Create a compiler that returns moduleCode for module source
    const compileFn: CompileFn = (source: string) => {
      if (source.includes("MODULE")) {
        return moduleCode;
      }
      return mainCode;
    };

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: { "//lib.star": "MODULE\n" },
    });

    const result = interp.interpret("MAIN\n");
    expect(result.variables["x"]).toBe(99);
  });

  it("should cache loaded modules", () => {
    let compileCount = 0;
    const moduleCode = makeAssign("val", 42);

    const compileFn: CompileFn = (source: string) => {
      compileCount++;
      if (source.includes("MODULE")) {
        return moduleCode;
      }
      // Main program loads from the same module twice
      return {
        instructions: [
          { opcode: Op.LOAD_MODULE, operand: 0 },
          { opcode: Op.IMPORT_FROM, operand: 1 },
          { opcode: Op.STORE_NAME, operand: 1 },
          { opcode: Op.POP },
          { opcode: Op.HALT },
        ],
        constants: [],
        names: ["//lib.star", "val"],
      };
    };

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: { "//lib.star": "MODULE\n" },
    });

    // First interpret
    interp.interpret("MAIN1\n");
    const firstCount = compileCount;

    // Second interpret reuses cache
    interp.interpret("MAIN2\n");
    // The module should NOT have been compiled again
    // (firstCount includes main + module = 2, second includes just main = 1 more)
    expect(compileCount).toBe(firstCount + 1);
  });

  it("should throw FileNotFoundError when module not found", () => {
    const compileFn: CompileFn = () => makeLoad("//missing.star", "x");

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: {},
    });

    expect(() => interp.interpret("anything\n")).toThrow(FileNotFoundError);
  });

  it("should throw when loading without fileResolver", () => {
    const compileFn: CompileFn = () => makeLoad("//lib.star", "x");

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: null,
    });

    expect(() => interp.interpret("anything\n")).toThrow(
      "no fileResolver configured",
    );
  });

  it("should throw IMPORT_FROM error for missing symbol", () => {
    const moduleCode: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42],
      names: ["other_name"],
    };

    const compileFn: CompileFn = (source: string) => {
      if (source.includes("MODULE")) return moduleCode;
      return makeLoad("//lib.star", "missing_symbol");
    };

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: { "//lib.star": "MODULE\n" },
    });

    expect(() => interp.interpret("MAIN\n")).toThrow(
      "symbol 'missing_symbol' not found",
    );
  });

  it("should inject globals into the main file and loaded files", () => {
    const moduleCode: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_NAME, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 1 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["ctx_os", "loaded_os"],
    };

    const mainCode: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_MODULE, operand: 0 },
        { opcode: Op.IMPORT_FROM, operand: 1 },
        { opcode: Op.STORE_NAME, operand: 1 },
        { opcode: Op.POP },
        { opcode: Op.LOAD_NAME, operand: 2 },
        { opcode: Op.STORE_NAME, operand: 3 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["//ctx.star", "loaded_os", "ctx_os", "main_os"],
    };

    const compileFn: CompileFn = (source: string) => {
      if (source.includes("MODULE")) {
        return moduleCode;
      }
      return mainCode;
    };

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: { "//ctx.star": "MODULE\n" },
      globals: { ctx_os: "darwin" },
    });

    const result = interp.interpret("MAIN\n");
    expect(result.variables["main_os"]).toBe("darwin");
    expect(result.variables["loaded_os"]).toBe("darwin");
  });
});

// =========================================================================
// Tests: Convenience Functions
// =========================================================================

describe("Convenience functions", () => {
  it("interpretBytecode should work as a standalone function", () => {
    const code = makeAssign("x", 42);
    const result = interpretBytecode(code);
    expect(result.variables["x"]).toBe(42);
  });

  it("interpretBytecode should accept options", () => {
    const code = makeAssign("x", 42);
    const result = interpretBytecode(code, { maxRecursionDepth: 50 });
    expect(result.variables["x"]).toBe(42);
  });

  it("interpretBytecode should accept fileResolver option", () => {
    const code = makeAssign("x", 42);
    const result = interpretBytecode(code, {
      fileResolver: { "//test.star": "x = 1" },
    });
    expect(result.variables["x"]).toBe(42);
  });
});

// =========================================================================
// Tests: Mini Starlark VM
// =========================================================================

describe("createMiniStarlarkVM", () => {
  it("should create a VM with registered handlers", () => {
    const vm = createMiniStarlarkVM();
    expect(vm).toBeInstanceOf(GenericVM);
  });

  it("should set max recursion depth", () => {
    const vm = createMiniStarlarkVM({ maxRecursionDepth: 50 });
    expect(vm.getMaxRecursionDepth()).toBe(50);
  });

  it("should have print builtin registered", () => {
    const vm = createMiniStarlarkVM();
    expect(vm.getBuiltin("print")).toBeDefined();
  });

  it("should have len builtin registered", () => {
    const vm = createMiniStarlarkVM();
    expect(vm.getBuiltin("len")).toBeDefined();
  });

  it("should have str builtin registered", () => {
    const vm = createMiniStarlarkVM();
    expect(vm.getBuiltin("str")).toBeDefined();
  });

  it("should have int builtin registered", () => {
    const vm = createMiniStarlarkVM();
    expect(vm.getBuiltin("int")).toBeDefined();
  });

  it("should have type builtin registered", () => {
    const vm = createMiniStarlarkVM();
    expect(vm.getBuiltin("type")).toBeDefined();
  });

  it("should have bool builtin registered", () => {
    const vm = createMiniStarlarkVM();
    expect(vm.getBuiltin("bool")).toBeDefined();
  });

  it("should have range builtin registered", () => {
    const vm = createMiniStarlarkVM();
    expect(vm.getBuiltin("range")).toBeDefined();
  });
});

// =========================================================================
// Tests: registerMiniStarlarkHandlers
// =========================================================================

describe("registerMiniStarlarkHandlers", () => {
  it("should register handlers on a fresh GenericVM", () => {
    const vm = new GenericVM();
    registerMiniStarlarkHandlers(vm);

    // Test by executing a simple program
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42],
      names: ["x"],
    };

    vm.execute(code);
    expect(vm.variables["x"]).toBe(42);
  });
});

// =========================================================================
// Tests: Multiple variable assignments
// =========================================================================

describe("Multiple variables", () => {
  it("should handle multiple assignments", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.STORE_NAME, operand: 1 },
        { opcode: Op.LOAD_NAME, operand: 0 },
        { opcode: Op.LOAD_NAME, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.STORE_NAME, operand: 2 },
        { opcode: Op.HALT },
      ],
      constants: [10, 20],
      names: ["a", "b", "c"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["a"]).toBe(10);
    expect(result.variables["b"]).toBe(20);
    expect(result.variables["c"]).toBe(30);
  });
});

// =========================================================================
// Tests: Multiple print outputs
// =========================================================================

describe("Multiple print outputs", () => {
  it("should capture multiple print calls", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.PRINT },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.PRINT },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.PRINT },
        { opcode: Op.HALT },
      ],
      constants: ["hello", "world", 42],
      names: [],
    };
    const result = interpretBytecode(code);
    expect(result.output).toEqual(["hello", "world", "42"]);
  });
});

// =========================================================================
// Tests: Op constants
// =========================================================================

describe("Op constants", () => {
  it("should have LOAD_MODULE at 0x90", () => {
    expect(Op.LOAD_MODULE).toBe(0x90);
  });

  it("should have IMPORT_FROM at 0x91", () => {
    expect(Op.IMPORT_FROM).toBe(0x91);
  });

  it("should have HALT at 0xFF", () => {
    expect(Op.HALT).toBe(0xff);
  });

  it("should have PRINT at 0xA0", () => {
    expect(Op.PRINT).toBe(0xa0);
  });
});

// =========================================================================
// Tests: Error cases
// =========================================================================

describe("Error cases", () => {
  it("should throw for undefined variable in LOAD_NAME", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["undefined_var"],
    };
    expect(() => interpretBytecode(code)).toThrow("Undefined variable");
  });

  it("should throw for floor division by zero", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.FLOOR_DIV },
        { opcode: Op.HALT },
      ],
      constants: [42, 0],
      names: [],
    };
    expect(() => interpretBytecode(code)).toThrow("Division by zero");
  });

  it("should throw for modulo by zero", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MOD },
        { opcode: Op.HALT },
      ],
      constants: [42, 0],
      names: [],
    };
    expect(() => interpretBytecode(code)).toThrow("Division by zero");
  });
});

// =========================================================================
// Tests: FileNotFoundError
// =========================================================================

describe("FileNotFoundError", () => {
  it("should be an instance of Error", () => {
    const err = new FileNotFoundError("test");
    expect(err).toBeInstanceOf(Error);
  });

  it("should have name FileNotFoundError", () => {
    const err = new FileNotFoundError("test");
    expect(err.name).toBe("FileNotFoundError");
  });

  it("should have the correct message", () => {
    const err = new FileNotFoundError("file not found");
    expect(err.message).toBe("file not found");
  });
});

// =========================================================================
// Tests: Transitive load caching
// =========================================================================

describe("Transitive load caching", () => {
  it("should share cache across multiple interpret calls", () => {
    const moduleCode = makeAssign("shared", 100);
    let moduleCompileCount = 0;

    const compileFn: CompileFn = (source: string) => {
      if (source.includes("MODULE")) {
        moduleCompileCount++;
        return moduleCode;
      }
      return makeLoad("//shared.star", "shared");
    };

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: { "//shared.star": "MODULE\n" },
    });

    // First call compiles the module
    const result1 = interp.interpret("MAIN1\n");
    expect(result1.variables["shared"]).toBe(100);
    expect(moduleCompileCount).toBe(1);

    // Second call uses the cache — module is NOT compiled again
    const result2 = interp.interpret("MAIN2\n");
    expect(result2.variables["shared"]).toBe(100);
    expect(moduleCompileCount).toBe(1);
  });

  it("should recompile after clearCache", () => {
    const moduleCode = makeAssign("val", 42);
    let moduleCompileCount = 0;

    const compileFn: CompileFn = (source: string) => {
      if (source.includes("MODULE")) {
        moduleCompileCount++;
        return moduleCode;
      }
      return makeLoad("//lib.star", "val");
    };

    const interp = new StarlarkInterpreter({
      compileFn,
      fileResolver: { "//lib.star": "MODULE\n" },
    });

    interp.interpret("MAIN1\n");
    expect(moduleCompileCount).toBe(1);

    interp.clearCache();

    interp.interpret("MAIN2\n");
    expect(moduleCompileCount).toBe(2);
  });
});

// =========================================================================
// Tests: Module-level interpret() convenience function
// =========================================================================

describe("Module-level interpret() function", () => {
  it("should execute source code with a compile function", () => {
    const mockCompile: CompileFn = () => makeAssign("x", 77);
    const result = interpret("anything\n", { compileFn: mockCompile });
    expect(result.variables["x"]).toBe(77);
  });

  it("should pass fileResolver through", () => {
    const moduleCode = makeAssign("y", 55);
    const mainCode = makeLoad("//lib.star", "y");
    const compileFn: CompileFn = (source: string) => {
      if (source.includes("MODULE")) return moduleCode;
      return mainCode;
    };
    const result = interpret("MAIN\n", {
      compileFn,
      fileResolver: { "//lib.star": "MODULE\n" },
    });
    expect(result.variables["y"]).toBe(55);
  });

  it("should pass maxRecursionDepth through", () => {
    const mockCompile: CompileFn = () => makeAssign("x", 1);
    const result = interpret("anything\n", {
      compileFn: mockCompile,
      maxRecursionDepth: 50,
    });
    expect(result.variables["x"]).toBe(1);
  });
});

// =========================================================================
// Tests: interpretFile() — both class method and module function
// =========================================================================

describe("interpretFile", () => {
  let tmpDir: string;

  it("class method should read and interpret a file", () => {
    tmpDir = mkdtempSync(join(tmpdir(), "starlark-test-"));
    const filePath = join(tmpDir, "test.star");
    writeFileSync(filePath, "x = 42\n");

    const mockCompile: CompileFn = () => makeAssign("x", 42);
    const interp = new StarlarkInterpreter({ compileFn: mockCompile });
    const result = interp.interpretFile(filePath);
    expect(result.variables["x"]).toBe(42);
    rmSync(tmpDir, { recursive: true });
  });

  it("class method should append newline if missing", () => {
    tmpDir = mkdtempSync(join(tmpdir(), "starlark-test-"));
    const filePath = join(tmpDir, "test.star");
    writeFileSync(filePath, "x = 42");  // No trailing newline

    const mockCompile: CompileFn = () => makeAssign("x", 42);
    const interp = new StarlarkInterpreter({ compileFn: mockCompile });
    const result = interp.interpretFile(filePath);
    expect(result.variables["x"]).toBe(42);
    rmSync(tmpDir, { recursive: true });
  });

  it("module-level function should read and interpret a file", () => {
    tmpDir = mkdtempSync(join(tmpdir(), "starlark-test-"));
    const filePath = join(tmpDir, "test.star");
    writeFileSync(filePath, "y = 99\n");

    const mockCompile: CompileFn = () => makeAssign("y", 99);
    const result = interpretFile(filePath, { compileFn: mockCompile });
    expect(result.variables["y"]).toBe(99);
    rmSync(tmpDir, { recursive: true });
  });
});

// =========================================================================
// Tests: MAKE_FUNCTION / CALL_FUNCTION / RETURN flow
// =========================================================================

describe("Function call mechanism", () => {
  it("should call a builtin function via CALL_FUNCTION", () => {
    // Call print("hello") — LOAD_NAME "print", LOAD_CONST "hello", CALL_FUNCTION 1
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_NAME, operand: 0 },       // push "print"
        { opcode: Op.LOAD_CONST, operand: 0 },      // push "hello"
        { opcode: Op.CALL_FUNCTION, operand: 1 },   // call print(1 arg)
        { opcode: Op.POP },                          // discard return val
        { opcode: Op.HALT },
      ],
      constants: ["hello"],
      names: ["print"],
    };
    const result = interpretBytecode(code);
    expect(result.output).toEqual(["hello"]);
  });

  it("should throw when calling unknown function", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },      // push "nonexistent" string
        { opcode: Op.CALL_FUNCTION, operand: 0 },   // call it
        { opcode: Op.HALT },
      ],
      constants: ["nonexistent_func"],
      names: [],
    };
    // Push the string directly as a "function name" that doesn't exist
    const vm = createMiniStarlarkVM();
    vm.push("unknown_func" as never);
    const callCode: CodeObject = {
      instructions: [
        { opcode: Op.CALL_FUNCTION, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: [],
    };
    expect(() => vm.execute(callCode)).toThrow("Unknown function");
  });

  it("should throw when calling a non-function value", () => {
    const vm = createMiniStarlarkVM();
    vm.push(42);  // push a number, not a function
    const code: CodeObject = {
      instructions: [
        { opcode: Op.CALL_FUNCTION, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: [],
    };
    expect(() => vm.execute(code)).toThrow("Cannot call non-function");
  });

  it("should handle MAKE_FUNCTION without defaults", () => {
    // Create a function code that returns 42
    const funcCode: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.RETURN },
      ],
      constants: [42],
      names: [],
    };

    const vm = createMiniStarlarkVM();
    // Push the function code object, then MAKE_FUNCTION
    vm.push(funcCode as never);
    const code: CodeObject = {
      instructions: [
        { opcode: Op.MAKE_FUNCTION, operand: 0 },   // flags=0 (no defaults)
        { opcode: Op.STORE_NAME, operand: 0 },       // store as "f"
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["f"],
    };
    vm.execute(code);
    const f = vm.variables["f"] as { __type__: string };
    expect(f.__type__).toBe("function");
  });

  it("should handle RETURN at top level", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.RETURN },
      ],
      constants: [42],
      names: [],
    };
    const vm = createMiniStarlarkVM();
    vm.execute(code);
    // Top-level RETURN halts and pushes return value
    expect(vm.halted).toBe(true);
  });
});

// =========================================================================
// Tests: Builtin functions via CALL_FUNCTION
// =========================================================================

describe("Builtin functions via CALL_FUNCTION", () => {
  function callBuiltin(name: string, ...args: (number | string)[]): CodeObject {
    const instructions: { opcode: number; operand?: number }[] = [];
    const constants: (number | string)[] = [];
    const names: string[] = [name];

    // Load the builtin name
    instructions.push({ opcode: Op.LOAD_NAME, operand: 0 });

    // Load args
    for (const arg of args) {
      instructions.push({ opcode: Op.LOAD_CONST, operand: constants.length });
      constants.push(arg);
    }

    // Call
    instructions.push({ opcode: Op.CALL_FUNCTION, operand: args.length });
    instructions.push({ opcode: Op.STORE_NAME, operand: names.length });
    names.push("result");
    instructions.push({ opcode: Op.HALT });

    return { instructions, constants, names };
  }

  it("should call len() on a string", () => {
    const result = interpretBytecode(callBuiltin("len", "hello"));
    expect(result.variables["result"]).toBe(5);
  });

  it("should call str() on a number", () => {
    const result = interpretBytecode(callBuiltin("str", 42));
    expect(result.variables["result"]).toBe("42");
  });

  it("should call int() on a string", () => {
    const result = interpretBytecode(callBuiltin("int", "42"));
    expect(result.variables["result"]).toBe(42);
  });

  it("should call type() on a number", () => {
    const result = interpretBytecode(callBuiltin("type", 42));
    expect(result.variables["result"]).toBe("int");
  });

  it("should call type() on a string", () => {
    const result = interpretBytecode(callBuiltin("type", "hello"));
    expect(result.variables["result"]).toBe("string");
  });

  it("should call bool() on 0", () => {
    const result = interpretBytecode(callBuiltin("bool", 0));
    expect(result.variables["result"]).toBe(0);
  });

  it("should call bool() on 1", () => {
    const result = interpretBytecode(callBuiltin("bool", 1));
    expect(result.variables["result"]).toBe(1);
  });
});

// =========================================================================
// Tests: range builtin
// =========================================================================

describe("range builtin", () => {
  it("should produce a range with just stop", () => {
    const vm = createMiniStarlarkVM();
    const rangeFn = vm.getBuiltin("range")!;
    const result = rangeFn.implementation(5);
    expect(result).toEqual([0, 1, 2, 3, 4]);
  });

  it("should produce a range with start and stop", () => {
    const vm = createMiniStarlarkVM();
    const rangeFn = vm.getBuiltin("range")!;
    const result = rangeFn.implementation(2, 5);
    expect(result).toEqual([2, 3, 4]);
  });

  it("should produce a range with start, stop, and step", () => {
    const vm = createMiniStarlarkVM();
    const rangeFn = vm.getBuiltin("range")!;
    const result = rangeFn.implementation(0, 10, 3);
    expect(result).toEqual([0, 3, 6, 9]);
  });

  it("should produce a range with negative step", () => {
    const vm = createMiniStarlarkVM();
    const rangeFn = vm.getBuiltin("range")!;
    const result = rangeFn.implementation(5, 0, -1);
    expect(result).toEqual([5, 4, 3, 2, 1]);
  });
});

// =========================================================================
// Tests: type builtin edge cases
// =========================================================================

describe("type builtin edge cases", () => {
  it("should return NoneType for null", () => {
    const vm = createMiniStarlarkVM();
    const typeFn = vm.getBuiltin("type")!;
    expect(typeFn.implementation(null)).toBe("NoneType");
  });

  it("should return list for arrays", () => {
    const vm = createMiniStarlarkVM();
    const typeFn = vm.getBuiltin("type")!;
    expect(typeFn.implementation([1, 2, 3])).toBe("list");
  });

  it("should return dict for objects", () => {
    const vm = createMiniStarlarkVM();
    const typeFn = vm.getBuiltin("type")!;
    expect(typeFn.implementation({ a: 1 })).toBe("dict");
  });
});

// =========================================================================
// Tests: len builtin edge cases
// =========================================================================

describe("len builtin edge cases", () => {
  it("should return length of array", () => {
    const vm = createMiniStarlarkVM();
    const lenFn = vm.getBuiltin("len")!;
    expect(lenFn.implementation([1, 2, 3])).toBe(3);
  });

  it("should return length of dict", () => {
    const vm = createMiniStarlarkVM();
    const lenFn = vm.getBuiltin("len")!;
    expect(lenFn.implementation({ a: 1, b: 2 })).toBe(2);
  });

  it("should throw for non-string/list/dict", () => {
    const vm = createMiniStarlarkVM();
    const lenFn = vm.getBuiltin("len")!;
    expect(() => lenFn.implementation(42)).toThrow();
  });
});

// =========================================================================
// Tests: bool builtin edge cases
// =========================================================================

describe("bool builtin edge cases", () => {
  it("should return 0 for empty string", () => {
    const vm = createMiniStarlarkVM();
    const boolFn = vm.getBuiltin("bool")!;
    expect(boolFn.implementation("")).toBe(0);
  });

  it("should return 0 for null", () => {
    const vm = createMiniStarlarkVM();
    const boolFn = vm.getBuiltin("bool")!;
    expect(boolFn.implementation(null)).toBe(0);
  });

  it("should return 0 for empty array", () => {
    const vm = createMiniStarlarkVM();
    const boolFn = vm.getBuiltin("bool")!;
    expect(boolFn.implementation([])).toBe(0);
  });

  it("should return 1 for non-empty string", () => {
    const vm = createMiniStarlarkVM();
    const boolFn = vm.getBuiltin("bool")!;
    expect(boolFn.implementation("hello")).toBe(1);
  });
});

// =========================================================================
// Tests: MUL string repetition (number * string branch)
// =========================================================================

describe("MUL number * string", () => {
  it("should handle number * string", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [3, "xy"],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe("xyxyxy");
  });
});

// =========================================================================
// Tests: STORE_SUBSCRIPT on dict
// =========================================================================

describe("STORE_SUBSCRIPT on dict", () => {
  it("should store into a dict by key", () => {
    const vm = createMiniStarlarkVM();
    // Build a dict, then store a new key into it
    const code: CodeObject = {
      instructions: [
        { opcode: Op.BUILD_DICT, operand: 0 },      // empty dict
        { opcode: Op.DUP },                           // keep ref for subscript
        { opcode: Op.LOAD_CONST, operand: 0 },       // key "a"
        { opcode: Op.LOAD_CONST, operand: 1 },       // value 42
        { opcode: Op.STORE_SUBSCRIPT },               // dict["a"] = 42
        { opcode: Op.STORE_NAME, operand: 0 },        // store dict as "d"
        { opcode: Op.HALT },
      ],
      constants: ["a", 42],
      names: ["d"],
    };
    vm.execute(code);
    const d = vm.variables["d"] as Record<string, unknown>;
    expect(d["a"]).toBe(42);
  });
});

// =========================================================================
// Tests: LOAD_SUBSCRIPT on dict
// =========================================================================

describe("LOAD_SUBSCRIPT on dict", () => {
  it("should load from a dict by key", () => {
    const vm = createMiniStarlarkVM();
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },       // key "x"
        { opcode: Op.LOAD_CONST, operand: 1 },       // value 42
        { opcode: Op.BUILD_DICT, operand: 1 },       // {"x": 42}
        { opcode: Op.LOAD_CONST, operand: 0 },       // key "x"
        { opcode: Op.LOAD_SUBSCRIPT },                // dict["x"]
        { opcode: Op.STORE_NAME, operand: 0 },       // store as "val"
        { opcode: Op.HALT },
      ],
      constants: ["x", 42],
      names: ["val"],
    };
    vm.execute(code);
    expect(vm.variables["val"]).toBe(42);
  });
});

// =========================================================================
// Tests: NOT operation on truthy values
// =========================================================================

describe("NOT operation on truthy value", () => {
  it("should return 0 for truthy value", () => {
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NOT },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [42],
      names: ["result"],
    };
    const result = interpretBytecode(code);
    expect(result.variables["result"]).toBe(0);
  });
});

// =========================================================================
// Tests: LOAD_MODULE without compile function
// =========================================================================

describe("LOAD_MODULE without compile function", () => {
  it("should use default stub handler when no load handler override", () => {
    // Use a bare VM without interpreter overlay — the default stub should push {}
    const vm = createMiniStarlarkVM();
    const code: CodeObject = {
      instructions: [
        { opcode: Op.LOAD_MODULE, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      constants: [],
      names: ["//lib.star"],
    };
    vm.execute(code);
    // The stub handler pushes an empty object
    expect(vm.variables["//lib.star"]).toEqual({});
  });
});
