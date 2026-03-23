/**
 * Starlark VM Tests -- Comprehensive tests for the Starlark bytecode interpreter.
 *
 * These tests verify all 59 opcode handlers, all 23 built-in functions,
 * the factory pattern, and the helper utilities. We construct bytecode
 * directly (rather than compiling from source) to test the VM in isolation.
 */

import { describe, it, expect } from "vitest";
import type { CodeObject, Instruction } from "@coding-adventures/virtual-machine";
import {
  GenericVM,
  VMError,
  VMTypeError,
  DivisionByZeroError,
  InvalidOperandError,
  UndefinedNameError,
} from "@coding-adventures/virtual-machine";

import {
  Op,
  StarlarkFunction,
  StarlarkIterator,
  isTruthy,
  starlarkRepr,
  starlarkValueRepr,
  starlarkTypeName,
  createStarlarkVM,
  executeStarlark,
  getAllBuiltins,
  builtinType,
  builtinBool,
  builtinInt,
  builtinFloat,
  builtinStr,
  builtinLen,
  builtinList,
  builtinDict,
  builtinTuple,
  builtinRange,
  builtinSorted,
  builtinReversed,
  builtinEnumerate,
  builtinZip,
  builtinMin,
  builtinMax,
  builtinAbs,
  builtinAll,
  builtinAny,
  builtinRepr,
  builtinHasattr,
  builtinGetattr,
  builtinPrint,
} from "../src/index.js";

// =========================================================================
// Helper: build a CodeObject from instructions
// =========================================================================

function makeCode(
  instructions: Instruction[],
  constants: unknown[] = [],
  names: string[] = [],
): CodeObject {
  return { instructions, constants, names } as CodeObject;
}

// =========================================================================
// Opcode Enum Tests
// =========================================================================

describe("Op enum", () => {
  it("has correct hex values for stack operations", () => {
    expect(Op.LOAD_CONST).toBe(0x01);
    expect(Op.POP).toBe(0x02);
    expect(Op.DUP).toBe(0x03);
    expect(Op.LOAD_NONE).toBe(0x04);
    expect(Op.LOAD_TRUE).toBe(0x05);
    expect(Op.LOAD_FALSE).toBe(0x06);
  });

  it("has correct hex values for variable operations", () => {
    expect(Op.STORE_NAME).toBe(0x10);
    expect(Op.LOAD_NAME).toBe(0x11);
    expect(Op.STORE_LOCAL).toBe(0x12);
    expect(Op.LOAD_LOCAL).toBe(0x13);
  });

  it("has correct hex values for arithmetic operations", () => {
    expect(Op.ADD).toBe(0x20);
    expect(Op.SUB).toBe(0x21);
    expect(Op.MUL).toBe(0x22);
    expect(Op.DIV).toBe(0x23);
    expect(Op.FLOOR_DIV).toBe(0x24);
    expect(Op.MOD).toBe(0x25);
    expect(Op.POWER).toBe(0x26);
    expect(Op.NEGATE).toBe(0x27);
  });

  it("has HALT at 0xFF", () => {
    expect(Op.HALT).toBe(0xff);
  });

  it("has 61 opcodes total", () => {
    expect(Object.keys(Op).length).toBe(61);
  });
});

// =========================================================================
// StarlarkFunction Tests
// =========================================================================

describe("StarlarkFunction", () => {
  it("creates a function with defaults", () => {
    const code = makeCode([{ opcode: Op.HALT }]);
    const func = new StarlarkFunction(code, [], "add", 2, ["x", "y"]);
    expect(func.name).toBe("add");
    expect(func.paramCount).toBe(2);
    expect(func.paramNames).toEqual(["x", "y"]);
    expect(func.defaults).toEqual([]);
    expect(func.code).toBe(code);
  });

  it("toString returns function name", () => {
    const code = makeCode([{ opcode: Op.HALT }]);
    const func = new StarlarkFunction(code, [], "greet", 0, []);
    expect(func.toString()).toBe("<function greet>");
  });

  it("defaults to <lambda> name", () => {
    const code = makeCode([{ opcode: Op.HALT }]);
    const func = new StarlarkFunction(code);
    expect(func.name).toBe("<lambda>");
  });
});

// =========================================================================
// StarlarkIterator Tests
// =========================================================================

describe("StarlarkIterator", () => {
  it("iterates over an array", () => {
    const iter = new StarlarkIterator([1, 2, 3]);
    expect(iter.next()).toEqual({ value: 1, done: false });
    expect(iter.next()).toEqual({ value: 2, done: false });
    expect(iter.next()).toEqual({ value: 3, done: false });
    expect(iter.next().done).toBe(true);
  });

  it("iterates over a string", () => {
    const iter = new StarlarkIterator([..."abc"]);
    expect(iter.next().value).toBe("a");
    expect(iter.next().value).toBe("b");
    expect(iter.next().value).toBe("c");
    expect(iter.next().done).toBe(true);
  });

  it("returns done after exhaustion", () => {
    const iter = new StarlarkIterator([]);
    expect(iter.next().done).toBe(true);
    expect(iter.next().done).toBe(true); // stays done
  });

  it("has a string representation", () => {
    const iter = new StarlarkIterator([1]);
    expect(iter.toString()).toBe("<starlark_iterator>");
  });
});

// =========================================================================
// isTruthy Tests
// =========================================================================

describe("isTruthy", () => {
  it("null and undefined are falsy", () => {
    expect(isTruthy(null)).toBe(false);
    expect(isTruthy(undefined)).toBe(false);
  });

  it("false is falsy, true is truthy", () => {
    expect(isTruthy(false)).toBe(false);
    expect(isTruthy(true)).toBe(true);
  });

  it("0 and 0.0 are falsy, nonzero is truthy", () => {
    expect(isTruthy(0)).toBe(false);
    expect(isTruthy(0.0)).toBe(false);
    expect(isTruthy(1)).toBe(true);
    expect(isTruthy(-1)).toBe(true);
    expect(isTruthy(3.14)).toBe(true);
  });

  it("empty string is falsy, non-empty is truthy", () => {
    expect(isTruthy("")).toBe(false);
    expect(isTruthy("hello")).toBe(true);
  });

  it("empty array is falsy, non-empty is truthy", () => {
    expect(isTruthy([])).toBe(false);
    expect(isTruthy([1])).toBe(true);
  });

  it("empty object is falsy, non-empty is truthy", () => {
    expect(isTruthy({})).toBe(false);
    expect(isTruthy({ a: 1 })).toBe(true);
  });
});

// =========================================================================
// starlarkTypeName Tests
// =========================================================================

describe("starlarkTypeName", () => {
  it("returns correct type names", () => {
    expect(starlarkTypeName(null)).toBe("NoneType");
    expect(starlarkTypeName(undefined)).toBe("NoneType");
    expect(starlarkTypeName(true)).toBe("bool");
    expect(starlarkTypeName(false)).toBe("bool");
    expect(starlarkTypeName(42)).toBe("int");
    expect(starlarkTypeName(3.14)).toBe("float");
    expect(starlarkTypeName("hello")).toBe("string");
    expect(starlarkTypeName([1, 2])).toBe("list");
    expect(starlarkTypeName({ a: 1 })).toBe("dict");
  });

  it("identifies functions", () => {
    const code = makeCode([{ opcode: Op.HALT }]);
    const func = new StarlarkFunction(code);
    expect(starlarkTypeName(func as any)).toBe("function");
  });

  it("identifies iterators", () => {
    const iter = new StarlarkIterator([1]);
    expect(starlarkTypeName(iter as any)).toBe("iterator");
  });
});

// =========================================================================
// starlarkRepr / starlarkValueRepr Tests
// =========================================================================

describe("starlarkRepr", () => {
  it("formats None, bools, numbers", () => {
    expect(starlarkRepr(null)).toBe("None");
    expect(starlarkRepr(true)).toBe("True");
    expect(starlarkRepr(false)).toBe("False");
    expect(starlarkRepr(42)).toBe("42");
    expect(starlarkRepr(3.14)).toBe("3.14");
  });

  it("prints strings without quotes", () => {
    expect(starlarkRepr("hello")).toBe("hello");
  });

  it("formats lists", () => {
    expect(starlarkRepr([1, "a", true])).toBe('[1, "a", True]');
  });

  it("formats empty list", () => {
    expect(starlarkRepr([])).toBe("[]");
  });

  it("formats dicts", () => {
    expect(starlarkRepr({ a: 1 } as any)).toBe('{"a": 1}');
  });
});

describe("starlarkValueRepr", () => {
  it("puts quotes around strings", () => {
    expect(starlarkValueRepr("hello")).toBe('"hello"');
  });

  it("handles nested structures", () => {
    expect(starlarkValueRepr([1, "a"])).toBe('[1, "a"]');
  });
});

// =========================================================================
// Stack Handler Tests
// =========================================================================

describe("Stack handlers", () => {
  it("LOAD_CONST pushes a constant", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [{ opcode: Op.LOAD_CONST, operand: 0 }, { opcode: Op.HALT }],
      [42],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([42]);
  });

  it("LOAD_CONST with invalid operand throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [{ opcode: Op.LOAD_CONST, operand: 99 }],
      [42],
    );
    expect(() => vm.execute(code)).toThrow(InvalidOperandError);
  });

  it("POP discards top of stack", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.POP },
        { opcode: Op.HALT },
      ],
      [10, 20],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([10]);
  });

  it("DUP duplicates top of stack", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.DUP },
        { opcode: Op.HALT },
      ],
      [42],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([42, 42]);
  });

  it("LOAD_NONE pushes null", () => {
    const vm = createStarlarkVM();
    const code = makeCode([{ opcode: Op.LOAD_NONE }, { opcode: Op.HALT }]);
    vm.execute(code);
    expect(vm.stack).toEqual([null]);
  });

  it("LOAD_TRUE pushes true", () => {
    const vm = createStarlarkVM();
    const code = makeCode([{ opcode: Op.LOAD_TRUE }, { opcode: Op.HALT }]);
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("LOAD_FALSE pushes false", () => {
    const vm = createStarlarkVM();
    const code = makeCode([{ opcode: Op.LOAD_FALSE }, { opcode: Op.HALT }]);
    vm.execute(code);
    expect(vm.stack).toEqual([false]);
  });
});

// =========================================================================
// Variable Handler Tests
// =========================================================================

describe("Variable handlers", () => {
  it("STORE_NAME and LOAD_NAME work together", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.LOAD_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      [42],
      ["x"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([42]);
    expect(vm.variables["x"]).toBe(42);
  });

  it("LOAD_NAME with undefined variable throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [{ opcode: Op.LOAD_NAME, operand: 0 }],
      [],
      ["nonexistent"],
    );
    expect(() => vm.execute(code)).toThrow(UndefinedNameError);
  });

  it("LOAD_NAME resolves builtins", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [{ opcode: Op.LOAD_NAME, operand: 0 }, { opcode: Op.HALT }],
      [],
      ["len"],
    );
    vm.execute(code);
    expect(vm.stack.length).toBe(1);
    // Should have pushed the builtin
    expect(vm.stack[0]).toBeDefined();
  });

  it("STORE_LOCAL and LOAD_LOCAL work together", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_LOCAL, operand: 0 },
        { opcode: Op.LOAD_LOCAL, operand: 0 },
        { opcode: Op.HALT },
      ],
      [99],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([99]);
  });

  it("LOAD_LOCAL with unassigned slot throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode([{ opcode: Op.LOAD_LOCAL, operand: 5 }]);
    expect(() => vm.execute(code)).toThrow(UndefinedNameError);
  });

  it("STORE_CLOSURE and LOAD_CLOSURE delegate to locals", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_CLOSURE, operand: 0 },
        { opcode: Op.LOAD_CLOSURE, operand: 0 },
        { opcode: Op.HALT },
      ],
      [77],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([77]);
  });

  it("STORE_NAME with invalid operand throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 99 },
      ],
      [42],
      ["x"],
    );
    expect(() => vm.execute(code)).toThrow(InvalidOperandError);
  });

  it("LOAD_NAME with invalid operand throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [{ opcode: Op.LOAD_NAME, operand: 99 }],
      [],
      ["x"],
    );
    expect(() => vm.execute(code)).toThrow(InvalidOperandError);
  });
});

// =========================================================================
// Arithmetic Handler Tests
// =========================================================================

describe("Arithmetic handlers", () => {
  it("ADD: int + int", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.HALT },
      ],
      [10, 20],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([30]);
  });

  it("ADD: str + str", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.HALT },
      ],
      ["hello", " world"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["hello world"]);
  });

  it("ADD: list + list", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.HALT },
      ],
      [[1, 2], [3, 4]],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[1, 2, 3, 4]]);
  });

  it("ADD: type error for mismatched types", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.ADD },
      ],
      [1, "hello"],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("SUB: int - int", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.SUB },
        { opcode: Op.HALT },
      ],
      [30, 12],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([18]);
  });

  it("SUB: type error for strings", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.SUB },
      ],
      ["a", "b"],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("MUL: str * int", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.HALT },
      ],
      ["ab", 3],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["ababab"]);
  });

  it("MUL: int * str", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.HALT },
      ],
      [3, "xy"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["xyxyxy"]);
  });

  it("MUL: list * int", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.HALT },
      ],
      [[1], 3],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[1, 1, 1]]);
  });

  it("MUL: int * list", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.HALT },
      ],
      [2, [5, 6]],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[5, 6, 5, 6]]);
  });

  it("MUL: numeric", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
        { opcode: Op.HALT },
      ],
      [6, 7],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([42]);
  });

  it("MUL: type error", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MUL },
      ],
      ["a", "b"],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("DIV: always returns float", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.DIV },
        { opcode: Op.HALT },
      ],
      [10, 4],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([2.5]);
  });

  it("DIV: division by zero throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.DIV },
      ],
      [10, 0],
    );
    expect(() => vm.execute(code)).toThrow(DivisionByZeroError);
  });

  it("DIV: type error", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.DIV },
      ],
      ["a", 2],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("FLOOR_DIV: integer division", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.FLOOR_DIV },
        { opcode: Op.HALT },
      ],
      [7, 2],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([3]);
  });

  it("FLOOR_DIV: division by zero throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.FLOOR_DIV },
      ],
      [10, 0],
    );
    expect(() => vm.execute(code)).toThrow(DivisionByZeroError);
  });

  it("FLOOR_DIV: type error", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.FLOOR_DIV },
      ],
      ["a", 2],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("MOD: numeric modulo", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MOD },
        { opcode: Op.HALT },
      ],
      [10, 3],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([1]);
  });

  it("MOD: string formatting", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MOD },
        { opcode: Op.HALT },
      ],
      ["hello %s", "world"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["hello world"]);
  });

  it("MOD: modulo by zero throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MOD },
      ],
      [10, 0],
    );
    expect(() => vm.execute(code)).toThrow(DivisionByZeroError);
  });

  it("MOD: type error", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.MOD },
      ],
      [[1], 2],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("POWER: exponentiation", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.POWER },
        { opcode: Op.HALT },
      ],
      [2, 10],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([1024]);
  });

  it("POWER: type error", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.POWER },
      ],
      ["a", 2],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("NEGATE: negation", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NEGATE },
        { opcode: Op.HALT },
      ],
      [42],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([-42]);
  });

  it("NEGATE: type error", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NEGATE },
      ],
      ["hello"],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });
});

// =========================================================================
// Bitwise Handler Tests
// =========================================================================

describe("Bitwise handlers", () => {
  it("BIT_AND: 0b1100 & 0b1010 = 0b1000", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BIT_AND },
        { opcode: Op.HALT },
      ],
      [0b1100, 0b1010],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([0b1000]);
  });

  it("BIT_AND: type error for floats", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BIT_AND },
      ],
      [3.5, 1],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("BIT_OR: 0b1100 | 0b1010 = 0b1110", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BIT_OR },
        { opcode: Op.HALT },
      ],
      [0b1100, 0b1010],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([0b1110]);
  });

  it("BIT_XOR: 0b1100 ^ 0b1010 = 0b0110", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BIT_XOR },
        { opcode: Op.HALT },
      ],
      [0b1100, 0b1010],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([0b0110]);
  });

  it("BIT_NOT: ~5 = -6", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.BIT_NOT },
        { opcode: Op.HALT },
      ],
      [5],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([-6]);
  });

  it("BIT_NOT: type error for floats", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.BIT_NOT },
      ],
      [3.5],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("LSHIFT: 1 << 3 = 8", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LSHIFT },
        { opcode: Op.HALT },
      ],
      [1, 3],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([8]);
  });

  it("RSHIFT: 8 >> 2 = 2", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.RSHIFT },
        { opcode: Op.HALT },
      ],
      [8, 2],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([2]);
  });

  it("LSHIFT: type error for strings", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LSHIFT },
      ],
      ["a", 1],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("RSHIFT: type error for strings", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.RSHIFT },
      ],
      ["a", 1],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("BIT_OR: type error for strings", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BIT_OR },
      ],
      ["a", 1],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("BIT_XOR: type error for strings", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BIT_XOR },
      ],
      ["a", 1],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });
});

// =========================================================================
// Comparison Handler Tests
// =========================================================================

describe("Comparison handlers", () => {
  it("CMP_EQ: equal values", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_EQ },
        { opcode: Op.HALT },
      ],
      [42, 42],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_EQ: unequal values", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_EQ },
        { opcode: Op.HALT },
      ],
      [42, 43],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([false]);
  });

  it("CMP_EQ: list equality (deep)", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_EQ },
        { opcode: Op.HALT },
      ],
      [[1, 2, 3], [1, 2, 3]],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_NE: not equal", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_NE },
        { opcode: Op.HALT },
      ],
      [1, 2],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_LT: less than", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_LT },
        { opcode: Op.HALT },
      ],
      [1, 2],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_GT: greater than", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_GT },
        { opcode: Op.HALT },
      ],
      [5, 3],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_LE: less than or equal", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_LE },
        { opcode: Op.HALT },
      ],
      [5, 5],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_GE: greater than or equal", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_GE },
        { opcode: Op.HALT },
      ],
      [3, 5],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([false]);
  });

  it("CMP_IN: element in list", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_IN },
        { opcode: Op.HALT },
      ],
      [2, [1, 2, 3]],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_IN: substring in string", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_IN },
        { opcode: Op.HALT },
      ],
      ["lo", "hello"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_IN: key in dict", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_IN },
        { opcode: Op.HALT },
      ],
      ["a", { a: 1, b: 2 }],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("CMP_NOT_IN: element not in list", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CMP_NOT_IN },
        { opcode: Op.HALT },
      ],
      [5, [1, 2, 3]],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });
});

// =========================================================================
// Boolean Handler Tests
// =========================================================================

describe("Boolean handler", () => {
  it("NOT: negates truthy value", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NOT },
        { opcode: Op.HALT },
      ],
      [42],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([false]);
  });

  it("NOT: negates falsy value", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NOT },
        { opcode: Op.HALT },
      ],
      [0],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("NOT: empty string is falsy", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.NOT },
        { opcode: Op.HALT },
      ],
      [""],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });
});

// =========================================================================
// Control Flow Handler Tests
// =========================================================================

describe("Control flow handlers", () => {
  it("JUMP: unconditional jump", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.JUMP, operand: 2 },       // 0: jump to 2
      { opcode: Op.LOAD_CONST, operand: 0 },  // 1: skipped
      { opcode: Op.LOAD_CONST, operand: 1 },  // 2: executed
      { opcode: Op.HALT },                     // 3
    ], [10, 20]);
    vm.execute(code);
    expect(vm.stack).toEqual([20]);
  });

  it("JUMP_IF_FALSE: takes branch when falsy", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_FALSE },               // 0: push false
      { opcode: Op.JUMP_IF_FALSE, operand: 4 }, // 1: should jump
      { opcode: Op.LOAD_CONST, operand: 0 },    // 2: skipped
      { opcode: Op.HALT },                       // 3
      { opcode: Op.LOAD_CONST, operand: 1 },    // 4: landed here
      { opcode: Op.HALT },                       // 5
    ], [10, 20]);
    vm.execute(code);
    expect(vm.stack).toEqual([20]);
  });

  it("JUMP_IF_FALSE: falls through when truthy", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_TRUE },
      { opcode: Op.JUMP_IF_FALSE, operand: 4 },
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.HALT },
      { opcode: Op.LOAD_CONST, operand: 1 },
      { opcode: Op.HALT },
    ], [10, 20]);
    vm.execute(code);
    expect(vm.stack).toEqual([10]);
  });

  it("JUMP_IF_TRUE: takes branch when truthy", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_TRUE },
      { opcode: Op.JUMP_IF_TRUE, operand: 4 },
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.HALT },
      { opcode: Op.LOAD_CONST, operand: 1 },
      { opcode: Op.HALT },
    ], [10, 20]);
    vm.execute(code);
    expect(vm.stack).toEqual([20]);
  });

  it("JUMP_IF_FALSE_OR_POP: short-circuit AND (falsy)", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_FALSE },
      { opcode: Op.JUMP_IF_FALSE_OR_POP, operand: 3 },
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.HALT },
    ], [99]);
    vm.execute(code);
    expect(vm.stack).toEqual([false]); // falsy value kept
  });

  it("JUMP_IF_FALSE_OR_POP: falls through (truthy)", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_TRUE },
      { opcode: Op.JUMP_IF_FALSE_OR_POP, operand: 3 },
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.HALT },
    ], [99]);
    vm.execute(code);
    expect(vm.stack).toEqual([99]); // truthy popped, 99 pushed
  });

  it("JUMP_IF_TRUE_OR_POP: short-circuit OR (truthy)", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_TRUE },
      { opcode: Op.JUMP_IF_TRUE_OR_POP, operand: 3 },
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.HALT },
    ], [99]);
    vm.execute(code);
    expect(vm.stack).toEqual([true]); // truthy value kept
  });

  it("JUMP_IF_TRUE_OR_POP: falls through (falsy)", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_FALSE },
      { opcode: Op.JUMP_IF_TRUE_OR_POP, operand: 3 },
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.HALT },
    ], [99]);
    vm.execute(code);
    expect(vm.stack).toEqual([99]); // falsy popped, 99 pushed
  });
});

// =========================================================================
// Collection Handler Tests
// =========================================================================

describe("Collection handlers", () => {
  it("BUILD_LIST: creates list from stack items", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.BUILD_LIST, operand: 3 },
        { opcode: Op.HALT },
      ],
      [1, 2, 3],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[1, 2, 3]]);
  });

  it("BUILD_LIST: empty list", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.BUILD_LIST, operand: 0 },
      { opcode: Op.HALT },
    ]);
    vm.execute(code);
    expect(vm.stack).toEqual([[]]);
  });

  it("BUILD_DICT: creates dict from key-value pairs", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.LOAD_CONST, operand: 3 },
        { opcode: Op.BUILD_DICT, operand: 2 },
        { opcode: Op.HALT },
      ],
      ["a", 1, "b", 2],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([{ a: 1, b: 2 }]);
  });

  it("BUILD_TUPLE: creates tuple from stack items", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.BUILD_TUPLE, operand: 2 },
        { opcode: Op.HALT },
      ],
      [10, 20],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[10, 20]]);
  });

  it("LIST_APPEND: appends to existing list", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.BUILD_LIST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LIST_APPEND },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LIST_APPEND },
        { opcode: Op.HALT },
      ],
      [10, 20],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[10, 20]]);
  });

  it("LIST_APPEND: type error for non-list", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LIST_APPEND },
      ],
      [42, 10],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("DICT_SET: sets dict entry", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.BUILD_DICT, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.DICT_SET },
        { opcode: Op.HALT },
      ],
      ["key", "value"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([{ key: "value" }]);
  });

  it("DICT_SET: type error for non-dict", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.DICT_SET },
      ],
      [[1], "key", "value"],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });
});

// =========================================================================
// Subscript & Attribute Handler Tests
// =========================================================================

describe("Subscript handlers", () => {
  it("LOAD_SUBSCRIPT: list index", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_SUBSCRIPT },
        { opcode: Op.HALT },
      ],
      [[10, 20, 30], 1],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([20]);
  });

  it("LOAD_SUBSCRIPT: negative index", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_SUBSCRIPT },
        { opcode: Op.HALT },
      ],
      [[10, 20, 30], -1],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([30]);
  });

  it("LOAD_SUBSCRIPT: string index", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_SUBSCRIPT },
        { opcode: Op.HALT },
      ],
      ["hello", 1],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["e"]);
  });

  it("LOAD_SUBSCRIPT: dict key", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_SUBSCRIPT },
        { opcode: Op.HALT },
      ],
      [{ a: 42, b: 99 }, "a"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([42]);
  });

  it("LOAD_SUBSCRIPT: out of range throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_SUBSCRIPT },
      ],
      [[1, 2], 5],
    );
    expect(() => vm.execute(code)).toThrow(VMError);
  });

  it("LOAD_SUBSCRIPT: missing dict key throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_SUBSCRIPT },
      ],
      [{ a: 1 }, "z"],
    );
    expect(() => vm.execute(code)).toThrow(VMError);
  });

  it("STORE_SUBSCRIPT: list assignment", () => {
    const vm = createStarlarkVM();
    const list = [10, 20, 30];
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.STORE_SUBSCRIPT },
        { opcode: Op.HALT },
      ],
      [list, 1, 99],
    );
    vm.execute(code);
    expect(list[1]).toBe(99);
  });

  it("STORE_SUBSCRIPT: dict assignment", () => {
    const vm = createStarlarkVM();
    const dict = { a: 1 };
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.STORE_SUBSCRIPT },
        { opcode: Op.HALT },
      ],
      [dict, "b", 99],
    );
    vm.execute(code);
    expect((dict as any).b).toBe(99);
  });

  it("STORE_ATTR: throws (Starlark doesn't support attribute assignment)", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [{ opcode: Op.STORE_ATTR, operand: 0 }],
      [],
      ["x"],
    );
    expect(() => vm.execute(code)).toThrow(VMError);
  });
});

// =========================================================================
// Slice Tests
// =========================================================================

describe("LOAD_SLICE", () => {
  it("slices a list with start and stop", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },  // list
        { opcode: Op.LOAD_CONST, operand: 1 },  // start
        { opcode: Op.LOAD_CONST, operand: 2 },  // stop
        { opcode: Op.LOAD_SLICE, operand: 0x03 }, // has start + stop
        { opcode: Op.HALT },
      ],
      [[1, 2, 3, 4, 5], 1, 4],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[2, 3, 4]]);
  });

  it("slices a string", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.LOAD_SLICE, operand: 0x03 },
        { opcode: Op.HALT },
      ],
      ["hello", 1, 4],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["ell"]);
  });

  it("slices with no start or stop (full slice)", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_SLICE, operand: 0x00 },
        { opcode: Op.HALT },
      ],
      [[1, 2, 3]],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([[1, 2, 3]]);
  });
});

// =========================================================================
// Iteration Handler Tests
// =========================================================================

describe("Iteration handlers", () => {
  it("GET_ITER + FOR_ITER: iterates over list", () => {
    // Simulate: for x in [10, 20, 30]: push x
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },  // 0: push [10,20,30]
        { opcode: Op.GET_ITER },                  // 1: convert to iterator
        { opcode: Op.FOR_ITER, operand: 5 },      // 2: get next or jump to 5
        { opcode: Op.STORE_NAME, operand: 0 },    // 3: store as "x"
        { opcode: Op.JUMP, operand: 2 },           // 4: loop back
        { opcode: Op.HALT },                       // 5: done
      ],
      [[10, 20, 30]],
      ["x"],
    );
    vm.execute(code);
    // After loop, x should be 30 (last value)
    expect(vm.variables["x"]).toBe(30);
  });

  it("GET_ITER: iterates over string", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.GET_ITER },
        { opcode: Op.FOR_ITER, operand: 5 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.JUMP, operand: 2 },
        { opcode: Op.HALT },
      ],
      ["abc"],
      ["ch"],
    );
    vm.execute(code);
    expect(vm.variables["ch"]).toBe("c");
  });

  it("GET_ITER: iterates over dict keys", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.GET_ITER },
        { opcode: Op.FOR_ITER, operand: 5 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.JUMP, operand: 2 },
        { opcode: Op.HALT },
      ],
      [{ x: 1, y: 2 }],
      ["k"],
    );
    vm.execute(code);
    // Last key iterated
    expect(["x", "y"]).toContain(vm.variables["k"]);
  });

  it("GET_ITER: type error for non-iterable", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.GET_ITER },
    ], [42]);
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("UNPACK_SEQUENCE: unpacks list", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.UNPACK_SEQUENCE, operand: 3 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 1 },
        { opcode: Op.STORE_NAME, operand: 2 },
        { opcode: Op.HALT },
      ],
      [[1, 2, 3]],
      ["a", "b", "c"],
    );
    vm.execute(code);
    expect(vm.variables["a"]).toBe(1);
    expect(vm.variables["b"]).toBe(2);
    expect(vm.variables["c"]).toBe(3);
  });

  it("UNPACK_SEQUENCE: wrong count throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.UNPACK_SEQUENCE, operand: 2 },
      ],
      [[1, 2, 3]],
    );
    expect(() => vm.execute(code)).toThrow(VMError);
  });
});

// =========================================================================
// Module Handler Tests
// =========================================================================

describe("Module handlers", () => {
  it("LOAD_MODULE: pushes placeholder", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_MODULE, operand: 0 },
        { opcode: Op.HALT },
      ],
      [],
      ["my_module.star"],
    );
    vm.execute(code);
    expect(vm.stack.length).toBe(1);
    expect((vm.stack[0] as any).__name__).toBe("my_module.star");
  });

  it("IMPORT_FROM: extracts symbol from module", () => {
    const vm = createStarlarkVM();
    // Push a fake module with a symbol
    vm.stack.push({ __name__: "mod", my_func: 42 } as any);
    const code = makeCode(
      [
        { opcode: Op.IMPORT_FROM, operand: 0 },
        { opcode: Op.HALT },
      ],
      [],
      ["my_func"],
    );
    vm.execute(code);
    // Should have the module and the extracted value
    expect(vm.stack[vm.stack.length - 1]).toBe(42);
  });

  it("IMPORT_FROM: missing symbol throws", () => {
    const vm = createStarlarkVM();
    vm.stack.push({ __name__: "mod" } as any);
    const code = makeCode(
      [{ opcode: Op.IMPORT_FROM, operand: 0 }],
      [],
      ["nonexistent"],
    );
    expect(() => vm.execute(code)).toThrow(VMError);
  });
});

// =========================================================================
// I/O Handler Tests
// =========================================================================

describe("PRINT handler", () => {
  it("captures output", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.PRINT },
        { opcode: Op.HALT },
      ],
      [42],
    );
    vm.execute(code);
    expect(vm.output).toEqual(["42"]);
  });

  it("prints strings without quotes", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.PRINT },
        { opcode: Op.HALT },
      ],
      ["hello world"],
    );
    vm.execute(code);
    expect(vm.output).toEqual(["hello world"]);
  });

  it("prints None", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_NONE },
      { opcode: Op.PRINT },
      { opcode: Op.HALT },
    ]);
    vm.execute(code);
    expect(vm.output).toEqual(["None"]);
  });

  it("prints booleans", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_TRUE },
      { opcode: Op.PRINT },
      { opcode: Op.LOAD_FALSE },
      { opcode: Op.PRINT },
      { opcode: Op.HALT },
    ]);
    vm.execute(code);
    expect(vm.output).toEqual(["True", "False"]);
  });
});

// =========================================================================
// HALT and RETURN Tests
// =========================================================================

describe("HALT and RETURN", () => {
  it("HALT stops execution", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.HALT },
      { opcode: Op.LOAD_CONST, operand: 1 }, // should not execute
    ], [1, 2]);
    vm.execute(code);
    expect(vm.stack).toEqual([1]);
  });

  it("RETURN at top level halts", () => {
    const vm = createStarlarkVM();
    const code = makeCode([
      { opcode: Op.LOAD_CONST, operand: 0 },
      { opcode: Op.RETURN },
      { opcode: Op.LOAD_CONST, operand: 1 },
    ], [1, 2]);
    vm.execute(code);
    expect(vm.halted).toBe(true);
  });
});

// =========================================================================
// Builtin Function Tests
// =========================================================================

describe("Built-in functions", () => {
  describe("type()", () => {
    it("returns correct type strings", () => {
      expect(builtinType(42)).toBe("int");
      expect(builtinType(3.14)).toBe("float");
      expect(builtinType("hello")).toBe("string");
      expect(builtinType(true)).toBe("bool");
      expect(builtinType(null)).toBe("NoneType");
      expect(builtinType([1, 2])).toBe("list");
      expect(builtinType({ a: 1 } as any)).toBe("dict");
    });

    it("throws for wrong number of args", () => {
      expect(() => builtinType()).toThrow(VMTypeError);
      expect(() => builtinType(1, 2)).toThrow(VMTypeError);
    });
  });

  describe("bool()", () => {
    it("converts to boolean", () => {
      expect(builtinBool(0)).toBe(false);
      expect(builtinBool(1)).toBe(true);
      expect(builtinBool("")).toBe(false);
      expect(builtinBool("x")).toBe(true);
      expect(builtinBool(null)).toBe(false);
      expect(builtinBool([])).toBe(false);
      expect(builtinBool([1])).toBe(true);
    });

    it("throws for wrong number of args", () => {
      expect(() => builtinBool()).toThrow(VMTypeError);
    });
  });

  describe("int()", () => {
    it("converts various types", () => {
      expect(builtinInt(42)).toBe(42);
      expect(builtinInt(3.7)).toBe(3);
      expect(builtinInt("42")).toBe(42);
      expect(builtinInt(true)).toBe(1);
      expect(builtinInt(false)).toBe(0);
    });

    it("converts with base", () => {
      expect(builtinInt("ff", 16)).toBe(255);
      expect(builtinInt("111", 2)).toBe(7);
    });

    it("throws for non-string with base", () => {
      expect(() => builtinInt(42, 16)).toThrow(VMTypeError);
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinInt()).toThrow(VMTypeError);
    });

    it("throws for invalid type", () => {
      expect(() => builtinInt([1] as any)).toThrow(VMTypeError);
    });
  });

  describe("float()", () => {
    it("converts to float", () => {
      expect(builtinFloat(42)).toBe(42);
      expect(builtinFloat("3.14")).toBe(3.14);
    });

    it("throws for invalid input", () => {
      expect(() => builtinFloat("abc")).toThrow(VMTypeError);
      expect(() => builtinFloat([] as any)).toThrow(VMTypeError);
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinFloat()).toThrow(VMTypeError);
    });
  });

  describe("str()", () => {
    it("converts to string", () => {
      expect(builtinStr(42)).toBe("42");
      expect(builtinStr(null)).toBe("None");
      expect(builtinStr(true)).toBe("True");
      expect(builtinStr("hello")).toBe("hello");
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinStr()).toThrow(VMTypeError);
    });
  });

  describe("len()", () => {
    it("returns length of string", () => {
      expect(builtinLen("hello")).toBe(5);
    });

    it("returns length of list", () => {
      expect(builtinLen([1, 2, 3])).toBe(3);
    });

    it("returns length of dict", () => {
      expect(builtinLen({ a: 1, b: 2 } as any)).toBe(2);
    });

    it("throws for non-iterable", () => {
      expect(() => builtinLen(42)).toThrow(VMTypeError);
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinLen()).toThrow(VMTypeError);
    });
  });

  describe("list()", () => {
    it("creates empty list", () => {
      expect(builtinList()).toEqual([]);
    });

    it("copies a list", () => {
      expect(builtinList([1, 2, 3])).toEqual([1, 2, 3]);
    });

    it("converts string to list", () => {
      expect(builtinList("abc")).toEqual(["a", "b", "c"]);
    });
  });

  describe("dict()", () => {
    it("creates empty dict", () => {
      expect(builtinDict()).toEqual({});
    });

    it("creates dict from pairs", () => {
      expect(builtinDict([["a", 1], ["b", 2]])).toEqual({ a: 1, b: 2 });
    });
  });

  describe("tuple()", () => {
    it("creates empty tuple", () => {
      expect(builtinTuple()).toEqual([]);
    });

    it("copies a list to tuple", () => {
      expect(builtinTuple([1, 2])).toEqual([1, 2]);
    });
  });

  describe("range()", () => {
    it("range(stop)", () => {
      expect(builtinRange(5)).toEqual([0, 1, 2, 3, 4]);
    });

    it("range(start, stop)", () => {
      expect(builtinRange(2, 5)).toEqual([2, 3, 4]);
    });

    it("range(start, stop, step)", () => {
      expect(builtinRange(0, 10, 3)).toEqual([0, 3, 6, 9]);
    });

    it("range with negative step", () => {
      expect(builtinRange(5, 0, -1)).toEqual([5, 4, 3, 2, 1]);
    });

    it("range(0) returns empty", () => {
      expect(builtinRange(0)).toEqual([]);
    });

    it("throws for zero step", () => {
      expect(() => builtinRange(0, 10, 0)).toThrow(VMTypeError);
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinRange()).toThrow(VMTypeError);
    });
  });

  describe("sorted()", () => {
    it("sorts a list", () => {
      expect(builtinSorted([3, 1, 2])).toEqual([1, 2, 3]);
    });

    it("sorts in reverse", () => {
      expect(builtinSorted([3, 1, 2], true)).toEqual([3, 2, 1]);
    });

    it("sorts strings", () => {
      expect(builtinSorted(["c", "a", "b"])).toEqual(["a", "b", "c"]);
    });
  });

  describe("reversed()", () => {
    it("reverses a list", () => {
      expect(builtinReversed([1, 2, 3])).toEqual([3, 2, 1]);
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinReversed()).toThrow(VMTypeError);
    });
  });

  describe("enumerate()", () => {
    it("enumerates a list", () => {
      expect(builtinEnumerate(["a", "b"])).toEqual([[0, "a"], [1, "b"]]);
    });

    it("enumerates with custom start", () => {
      expect(builtinEnumerate(["a", "b"], 5)).toEqual([[5, "a"], [6, "b"]]);
    });
  });

  describe("zip()", () => {
    it("zips two lists", () => {
      expect(builtinZip([1, 2], ["a", "b"])).toEqual([[1, "a"], [2, "b"]]);
    });

    it("truncates to shortest", () => {
      expect(builtinZip([1, 2, 3], ["a"])).toEqual([[1, "a"]]);
    });

    it("returns empty for no args", () => {
      expect(builtinZip()).toEqual([]);
    });
  });

  describe("min() and max()", () => {
    it("min of iterable", () => {
      expect(builtinMin([3, 1, 2])).toBe(1);
    });

    it("min of args", () => {
      expect(builtinMin(5, 3, 8)).toBe(3);
    });

    it("max of iterable", () => {
      expect(builtinMax([3, 1, 2])).toBe(3);
    });

    it("max of args", () => {
      expect(builtinMax(5, 3, 8)).toBe(8);
    });

    it("min of empty list throws", () => {
      expect(() => builtinMin([])).toThrow(VMTypeError);
    });

    it("max of empty list throws", () => {
      expect(() => builtinMax([])).toThrow(VMTypeError);
    });
  });

  describe("abs()", () => {
    it("absolute value", () => {
      expect(builtinAbs(-5)).toBe(5);
      expect(builtinAbs(5)).toBe(5);
      expect(builtinAbs(0)).toBe(0);
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinAbs()).toThrow(VMTypeError);
    });
  });

  describe("all() and any()", () => {
    it("all: all truthy", () => {
      expect(builtinAll([1, 2, 3])).toBe(true);
    });

    it("all: has falsy", () => {
      expect(builtinAll([1, 0, 3])).toBe(false);
    });

    it("all: empty", () => {
      expect(builtinAll([])).toBe(true);
    });

    it("any: has truthy", () => {
      expect(builtinAny([0, 0, 1])).toBe(true);
    });

    it("any: all falsy", () => {
      expect(builtinAny([0, "", null])).toBe(false);
    });

    it("any: empty", () => {
      expect(builtinAny([])).toBe(false);
    });
  });

  describe("repr()", () => {
    it("returns repr with quotes for strings", () => {
      expect(builtinRepr("hello")).toBe('"hello"');
    });

    it("returns repr for numbers", () => {
      expect(builtinRepr(42)).toBe("42");
    });

    it("throws for wrong arg count", () => {
      expect(() => builtinRepr()).toThrow(VMTypeError);
    });
  });

  describe("hasattr()", () => {
    it("checks attribute presence", () => {
      expect(builtinHasattr({ x: 1 } as any, "x")).toBe(true);
      expect(builtinHasattr({ x: 1 } as any, "y")).toBe(false);
    });

    it("returns false for non-objects", () => {
      expect(builtinHasattr(42, "x")).toBe(false);
    });
  });

  describe("getattr()", () => {
    it("gets attribute", () => {
      expect(builtinGetattr({ x: 42 } as any, "x")).toBe(42);
    });

    it("returns default for missing attribute", () => {
      expect(builtinGetattr({ x: 42 } as any, "y", 99)).toBe(99);
    });

    it("throws for missing attribute without default", () => {
      expect(() => builtinGetattr({ x: 42 } as any, "y")).toThrow(VMTypeError);
    });
  });

  describe("print()", () => {
    it("returns null", () => {
      expect(builtinPrint("hello")).toBe(null);
    });
  });
});

// =========================================================================
// getAllBuiltins Tests
// =========================================================================

describe("getAllBuiltins", () => {
  it("returns all 23 builtins", () => {
    const builtins = getAllBuiltins();
    expect(Object.keys(builtins).length).toBe(23);
  });

  it("contains expected function names", () => {
    const builtins = getAllBuiltins();
    const expected = [
      "type", "bool", "int", "float", "str",
      "len", "list", "dict", "tuple", "range",
      "sorted", "reversed", "enumerate", "zip",
      "min", "max", "abs", "all", "any",
      "repr", "hasattr", "getattr", "print",
    ];
    for (const name of expected) {
      expect(builtins[name]).toBeDefined();
    }
  });
});

// =========================================================================
// Factory and Integration Tests
// =========================================================================

describe("createStarlarkVM", () => {
  it("creates a VM with all opcodes registered", () => {
    const vm = createStarlarkVM();
    // Verify by executing a simple program
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      [42],
      ["x"],
    );
    vm.execute(code);
    expect(vm.variables["x"]).toBe(42);
  });

  it("configures max recursion depth", () => {
    const vm = createStarlarkVM(100);
    expect(vm.getMaxRecursionDepth()).toBe(100);
  });

  it("configures frozen state", () => {
    const vm = createStarlarkVM(200, true);
    expect(vm.isFrozen()).toBe(true);
  });

  it("registers print builtin with capture", () => {
    const vm = createStarlarkVM();
    // Test that the print builtin captures to vm.output
    const printBuiltin = vm.getBuiltin("print");
    expect(printBuiltin).toBeDefined();
    printBuiltin!.implementation("hello", "world");
    expect(vm.output).toContain("hello world");
  });
});

describe("executeStarlark", () => {
  it("executes a simple program", () => {
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      [1, 2],
      ["x"],
    );
    const result = executeStarlark(code);
    expect(result.variables["x"]).toBe(3);
  });

  it("captures print output", () => {
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.PRINT },
        { opcode: Op.HALT },
      ],
      [42],
    );
    const result = executeStarlark(code);
    expect(result.output).toEqual(["42"]);
  });

  it("returns traces", () => {
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.HALT },
      ],
      [42],
    );
    const result = executeStarlark(code);
    expect(result.traces.length).toBeGreaterThan(0);
  });
});

// =========================================================================
// LOAD_ATTR Tests (String, List, Dict methods)
// =========================================================================

describe("LOAD_ATTR (string methods)", () => {
  it("string.upper()", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },   // "hello"
        { opcode: Op.LOAD_ATTR, operand: 0 },      // .upper
        { opcode: Op.CALL_FUNCTION, operand: 0 },  // call upper()
        { opcode: Op.HALT },
      ],
      ["hello"],
      ["upper"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["HELLO"]);
  });

  it("string.lower()", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_ATTR, operand: 0 },
        { opcode: Op.CALL_FUNCTION, operand: 0 },
        { opcode: Op.HALT },
      ],
      ["HELLO"],
      ["lower"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual(["hello"]);
  });

  it("string.split()", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_ATTR, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CALL_FUNCTION, operand: 1 },
        { opcode: Op.HALT },
      ],
      ["a,b,c", ","],
      ["split"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([["a", "b", "c"]]);
  });

  it("string.startswith()", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_ATTR, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CALL_FUNCTION, operand: 1 },
        { opcode: Op.HALT },
      ],
      ["hello world", "hello"],
      ["startswith"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([true]);
  });

  it("LOAD_ATTR: unknown attribute throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_ATTR, operand: 0 },
      ],
      [42],
      ["nonexistent"],
    );
    expect(() => vm.execute(code)).toThrow(VMError);
  });
});

describe("LOAD_ATTR (list methods)", () => {
  it("list.append()", () => {
    const vm = createStarlarkVM();
    const list = [1, 2];
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_ATTR, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.CALL_FUNCTION, operand: 1 },
        { opcode: Op.HALT },
      ],
      [list, 3],
      ["append"],
    );
    vm.execute(code);
    expect(list).toEqual([1, 2, 3]);
  });
});

describe("LOAD_ATTR (dict methods)", () => {
  it("dict.keys()", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_ATTR, operand: 0 },
        { opcode: Op.CALL_FUNCTION, operand: 0 },
        { opcode: Op.HALT },
      ],
      [{ a: 1, b: 2 }],
      ["keys"],
    );
    vm.execute(code);
    expect((vm.stack[0] as string[]).sort()).toEqual(["a", "b"]);
  });

  it("dict.get() with default", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_ATTR, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.LOAD_CONST, operand: 2 },
        { opcode: Op.CALL_FUNCTION, operand: 2 },
        { opcode: Op.HALT },
      ],
      [{ a: 1 }, "b", 42],
      ["get"],
    );
    vm.execute(code);
    expect(vm.stack).toEqual([42]);
  });
});

// =========================================================================
// Function Handler Tests (MAKE_FUNCTION, CALL_FUNCTION)
// =========================================================================

describe("Function handlers", () => {
  it("MAKE_FUNCTION + CALL_FUNCTION: simple function", () => {
    // Define function that returns local 0 + local 1
    const funcCode = makeCode(
      [
        { opcode: Op.LOAD_LOCAL, operand: 0 },
        { opcode: Op.LOAD_LOCAL, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.RETURN },
      ],
      [],
      ["x", "y"],
    );

    const vm = createStarlarkVM();
    const mainCode = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },    // push funcCode
        { opcode: Op.LOAD_CONST, operand: 1 },    // push param names
        { opcode: Op.MAKE_FUNCTION, operand: 0x08 }, // has param names
        { opcode: Op.STORE_NAME, operand: 0 },     // store as "add"
        { opcode: Op.LOAD_NAME, operand: 0 },      // load "add"
        { opcode: Op.LOAD_CONST, operand: 2 },    // push 10
        { opcode: Op.LOAD_CONST, operand: 3 },    // push 20
        { opcode: Op.CALL_FUNCTION, operand: 2 },  // call add(10, 20)
        { opcode: Op.HALT },
      ],
      [funcCode, ["x", "y"], 10, 20],
      ["add"],
    );
    vm.execute(mainCode);
    expect(vm.stack).toEqual([30]);
  });

  it("CALL_FUNCTION: not callable throws", () => {
    const vm = createStarlarkVM();
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.CALL_FUNCTION, operand: 0 },
      ],
      [42],
    );
    expect(() => vm.execute(code)).toThrow(VMTypeError);
  });

  it("CALL_FUNCTION_KW: keyword arguments", () => {
    const funcCode = makeCode(
      [
        { opcode: Op.LOAD_LOCAL, operand: 0 },
        { opcode: Op.LOAD_LOCAL, operand: 1 },
        { opcode: Op.SUB },
        { opcode: Op.RETURN },
      ],
      [],
      ["a", "b"],
    );

    const vm = createStarlarkVM();
    const mainCode = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },    // funcCode
        { opcode: Op.LOAD_CONST, operand: 1 },    // param names
        { opcode: Op.MAKE_FUNCTION, operand: 0x08 },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.LOAD_NAME, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 2 },    // 30
        { opcode: Op.LOAD_CONST, operand: 3 },    // 12
        { opcode: Op.LOAD_CONST, operand: 4 },    // kw names
        { opcode: Op.CALL_FUNCTION_KW, operand: 2 },
        { opcode: Op.HALT },
      ],
      [funcCode, ["a", "b"], 30, 12, ["a", "b"]],
      ["sub"],
    );
    vm.execute(mainCode);
    expect(vm.stack).toEqual([18]);
  });
});

// =========================================================================
// Integration: Complex Programs
// =========================================================================

describe("Integration tests", () => {
  it("computes x = 1 + 2 and stores result", () => {
    const code = makeCode(
      [
        { opcode: Op.LOAD_CONST, operand: 0 },
        { opcode: Op.LOAD_CONST, operand: 1 },
        { opcode: Op.ADD },
        { opcode: Op.STORE_NAME, operand: 0 },
        { opcode: Op.HALT },
      ],
      [1, 2],
      ["x"],
    );
    const result = executeStarlark(code);
    expect(result.variables["x"]).toBe(3);
  });

  it("if/else: takes true branch", () => {
    const code = makeCode(
      [
        { opcode: Op.LOAD_TRUE },                 // 0
        { opcode: Op.JUMP_IF_FALSE, operand: 5 }, // 1: skip to else
        { opcode: Op.LOAD_CONST, operand: 0 },    // 2: true branch
        { opcode: Op.STORE_NAME, operand: 0 },    // 3
        { opcode: Op.JUMP, operand: 7 },           // 4: skip else
        { opcode: Op.LOAD_CONST, operand: 1 },    // 5: else branch
        { opcode: Op.STORE_NAME, operand: 0 },    // 6
        { opcode: Op.HALT },                       // 7
      ],
      ["yes", "no"],
      ["result"],
    );
    const result = executeStarlark(code);
    expect(result.variables["result"]).toBe("yes");
  });

  it("while loop: sum 1..5", () => {
    // sum = 0; i = 1; while i <= 5: sum += i; i += 1
    const code = makeCode(
      [
        // sum = 0
        { opcode: Op.LOAD_CONST, operand: 0 },    // 0: push 0
        { opcode: Op.STORE_NAME, operand: 0 },    // 1: store sum
        // i = 1
        { opcode: Op.LOAD_CONST, operand: 1 },    // 2: push 1
        { opcode: Op.STORE_NAME, operand: 1 },    // 3: store i
        // while i <= 5:
        { opcode: Op.LOAD_NAME, operand: 1 },     // 4: load i
        { opcode: Op.LOAD_CONST, operand: 2 },    // 5: push 5
        { opcode: Op.CMP_LE },                     // 6: i <= 5
        { opcode: Op.JUMP_IF_FALSE, operand: 16 }, // 7: exit loop
        // sum += i
        { opcode: Op.LOAD_NAME, operand: 0 },     // 8: load sum
        { opcode: Op.LOAD_NAME, operand: 1 },     // 9: load i
        { opcode: Op.ADD },                         // 10: sum + i
        { opcode: Op.STORE_NAME, operand: 0 },    // 11: store sum
        // i += 1
        { opcode: Op.LOAD_NAME, operand: 1 },     // 12: load i
        { opcode: Op.LOAD_CONST, operand: 1 },    // 13: push 1
        { opcode: Op.ADD },                         // 14: i + 1
        { opcode: Op.STORE_NAME, operand: 1 },    // 15: store i
        { opcode: Op.JUMP, operand: 4 },            // 16: back to condition
        // (actually this needs to jump after the condition check exits)
        { opcode: Op.HALT },                        // 17
      ],
      [0, 1, 5],
      ["sum", "i"],
    );
    // Fix: the JUMP should go to 4 (condition), not after HALT
    // And JUMP_IF_FALSE should go to 17 (HALT)
    code.instructions[7] = { opcode: Op.JUMP_IF_FALSE, operand: 17 };
    code.instructions[16] = { opcode: Op.JUMP, operand: 4 };
    const result = executeStarlark(code);
    expect(result.variables["sum"]).toBe(15);
  });

  it("for loop: sum elements of list", () => {
    const code = makeCode(
      [
        // sum = 0
        { opcode: Op.LOAD_CONST, operand: 0 },    // 0: push 0
        { opcode: Op.STORE_NAME, operand: 0 },    // 1: store sum
        // for x in [1,2,3,4,5]:
        { opcode: Op.LOAD_CONST, operand: 1 },    // 2: push list
        { opcode: Op.GET_ITER },                    // 3: get iterator
        { opcode: Op.FOR_ITER, operand: 10 },      // 4: next or jump to 10
        { opcode: Op.STORE_NAME, operand: 1 },    // 5: store x
        // sum += x
        { opcode: Op.LOAD_NAME, operand: 0 },     // 6: load sum
        { opcode: Op.LOAD_NAME, operand: 1 },     // 7: load x
        { opcode: Op.ADD },                         // 8: sum + x
        { opcode: Op.STORE_NAME, operand: 0 },    // 9: store sum
        { opcode: Op.JUMP, operand: 4 },            // 10: back to for_iter
        // Actually the JUMP at 10 should loop back
        { opcode: Op.HALT },                        // 11
      ],
      [0, [1, 2, 3, 4, 5]],
      ["sum", "x"],
    );
    // Fix jump targets
    code.instructions[4] = { opcode: Op.FOR_ITER, operand: 11 };
    code.instructions[10] = { opcode: Op.JUMP, operand: 4 };
    const result = executeStarlark(code);
    expect(result.variables["sum"]).toBe(15);
  });
});
