/**
 * Comprehensive tests for the Virtual Machine — Layer 5 of the computing stack.
 *
 * These tests verify every opcode, every error path, and several end-to-end
 * programs. They're organized by category to make it easy to find tests for
 * a specific feature.
 *
 * We follow the Arrange-Act-Assert pattern throughout:
 *     1. Arrange: Build a CodeObject with the instructions we want to test.
 *     2. Act: Execute it on a fresh VirtualMachine.
 *     3. Assert: Check the stack, variables, output, and traces.
 */

import { describe, it, expect } from "vitest";

import {
  OpCode,
  type Instruction,
  type CodeObject,
  type VMTrace,
  VirtualMachine,
  assembleCode,
  instructionToString,
  VMError,
  StackUnderflowError,
  UndefinedNameError,
  DivisionByZeroError,
  InvalidOpcodeError,
  InvalidOperandError,
} from "../src/index.js";

// =========================================================================
// Helpers
// =========================================================================

/**
 * Build a CodeObject, execute it, and return the VM + traces.
 *
 * This is a convenience wrapper so every test doesn't have to repeat
 * the boilerplate of creating a VM and calling execute().
 */
function run(
  instructions: Instruction[],
  constants?: (number | string)[] | null,
  names?: string[] | null,
): [VirtualMachine, VMTrace[]] {
  const code = assembleCode(instructions, constants, names);
  const vm = new VirtualMachine();
  const traces = vm.execute(code);
  return [vm, traces];
}

// =========================================================================
// Stack Operations
// =========================================================================

describe("TestLoadConst", () => {
  /** Tests for LOAD_CONST — pushing constants onto the stack. */

  it("should push an integer from the constants pool", () => {
    /** LOAD_CONST should push an integer from the constants pool. */
    const [vm, traces] = run(
      [{ opcode: OpCode.LOAD_CONST, operand: 0 }, { opcode: OpCode.HALT }],
      [42],
    );
    expect(vm.stack).toEqual([42]);
    expect(traces.length).toBe(2); // LOAD_CONST + HALT
  });

  it("should push a string from the constants pool", () => {
    /** LOAD_CONST should push a string from the constants pool. */
    const [vm] = run(
      [{ opcode: OpCode.LOAD_CONST, operand: 0 }, { opcode: OpCode.HALT }],
      ["hello"],
    );
    expect(vm.stack).toEqual(["hello"]);
  });

  it("should stack multiple values in order", () => {
    /** Multiple LOAD_CONST instructions stack values in order. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.LOAD_CONST, operand: 2 },
        { opcode: OpCode.HALT },
      ],
      [10, 20, 30],
    );
    expect(vm.stack).toEqual([10, 20, 30]);
  });

  it("should throw InvalidOperandError for out-of-range index", () => {
    /** LOAD_CONST with an out-of-range index should raise an error. */
    expect(() =>
      run(
        [{ opcode: OpCode.LOAD_CONST, operand: 5 }, { opcode: OpCode.HALT }],
        [42],
      ),
    ).toThrow(InvalidOperandError);
  });

  it("should throw InvalidOperandError when operand is missing", () => {
    /** LOAD_CONST without an operand should raise an error. */
    expect(() =>
      run([{ opcode: OpCode.LOAD_CONST }, { opcode: OpCode.HALT }]),
    ).toThrow(InvalidOperandError);
  });
});

describe("TestPop", () => {
  /** Tests for POP — discarding the top of the stack. */

  it("should remove exactly one value from the top", () => {
    /** POP should remove exactly one value from the top. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.POP },
        { opcode: OpCode.HALT },
      ],
      [10, 20],
    );
    expect(vm.stack).toEqual([10]);
  });

  it("should throw StackUnderflowError on empty stack", () => {
    /** POP on an empty stack should raise StackUnderflowError. */
    expect(() =>
      run([{ opcode: OpCode.POP }, { opcode: OpCode.HALT }]),
    ).toThrow(StackUnderflowError);
  });
});

describe("TestDup", () => {
  /** Tests for DUP — duplicating the top of the stack. */

  it("should push a copy of the top value", () => {
    /** DUP should push a copy of the top value. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.DUP },
        { opcode: OpCode.HALT },
      ],
      [42],
    );
    expect(vm.stack).toEqual([42, 42]);
  });

  it("should throw StackUnderflowError on empty stack", () => {
    /** DUP on an empty stack should raise StackUnderflowError. */
    expect(() =>
      run([{ opcode: OpCode.DUP }, { opcode: OpCode.HALT }]),
    ).toThrow(StackUnderflowError);
  });
});

// =========================================================================
// Arithmetic Operations
// =========================================================================

describe("TestAdd", () => {
  /** Tests for ADD — popping two values and pushing their sum. */

  it("should pop two integers and push their sum", () => {
    /** ADD should pop two integers and push their sum. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.ADD },
        { opcode: OpCode.HALT },
      ],
      [3, 4],
    );
    expect(vm.stack).toEqual([7]);
  });

  it("should concatenate strings", () => {
    /** ADD should concatenate strings (like Python's + on strings). */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.ADD },
        { opcode: OpCode.HALT },
      ],
      ["hello ", "world"],
    );
    expect(vm.stack).toEqual(["hello world"]);
  });

  it("should throw StackUnderflowError with fewer than two values", () => {
    /** ADD with fewer than two values should raise StackUnderflowError. */
    expect(() =>
      run(
        [
          { opcode: OpCode.LOAD_CONST, operand: 0 },
          { opcode: OpCode.ADD },
          { opcode: OpCode.HALT },
        ],
        [42],
      ),
    ).toThrow(StackUnderflowError);
  });
});

describe("TestSub", () => {
  /** Tests for SUB — popping two values and pushing their difference. */

  it("should compute a - b where a is pushed first", () => {
    /** SUB should compute a - b where a is pushed first. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 }, // push 10
        { opcode: OpCode.LOAD_CONST, operand: 1 }, // push 3
        { opcode: OpCode.SUB },                     // 10 - 3 = 7
        { opcode: OpCode.HALT },
      ],
      [10, 3],
    );
    expect(vm.stack).toEqual([7]);
  });

  it("should handle negative results correctly", () => {
    /** SUB should handle negative results correctly. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.SUB },
        { opcode: OpCode.HALT },
      ],
      [3, 10],
    );
    expect(vm.stack).toEqual([-7]);
  });
});

describe("TestMul", () => {
  /** Tests for MUL — popping two values and pushing their product. */

  it("should pop two integers and push their product", () => {
    /** MUL should pop two integers and push their product. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.MUL },
        { opcode: OpCode.HALT },
      ],
      [6, 7],
    );
    expect(vm.stack).toEqual([42]);
  });

  it("should produce zero when multiplied by zero", () => {
    /** MUL by zero should produce zero. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.MUL },
        { opcode: OpCode.HALT },
      ],
      [999, 0],
    );
    expect(vm.stack).toEqual([0]);
  });
});

describe("TestDiv", () => {
  /** Tests for DIV — integer division. */

  it("should perform integer division (a / b truncated)", () => {
    /** DIV should perform integer division (Math.trunc(a / b)). */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.DIV },
        { opcode: OpCode.HALT },
      ],
      [10, 3],
    );
    expect(vm.stack).toEqual([3]); // Math.trunc(10 / 3) = 3
  });

  it("should throw DivisionByZeroError when dividing by zero", () => {
    /** DIV by zero should raise DivisionByZeroError. */
    expect(() =>
      run(
        [
          { opcode: OpCode.LOAD_CONST, operand: 0 },
          { opcode: OpCode.LOAD_CONST, operand: 1 },
          { opcode: OpCode.DIV },
          { opcode: OpCode.HALT },
        ],
        [10, 0],
      ),
    ).toThrow(DivisionByZeroError);
  });

  it("should produce correct quotient for exact division", () => {
    /** DIV with an exact result should produce the correct quotient. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.DIV },
        { opcode: OpCode.HALT },
      ],
      [42, 6],
    );
    expect(vm.stack).toEqual([7]);
  });
});

// =========================================================================
// Variable Operations
// =========================================================================

describe("TestNamedVariables", () => {
  /** Tests for STORE_NAME and LOAD_NAME — named variable operations. */

  it("should round-trip a value through store and load", () => {
    /** STORE_NAME followed by LOAD_NAME should round-trip a value. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.STORE_NAME, operand: 0 },
        { opcode: OpCode.LOAD_NAME, operand: 0 },
        { opcode: OpCode.HALT },
      ],
      [42],
      ["x"],
    );
    expect(vm.stack).toEqual([42]);
    expect(vm.variables).toEqual({ x: 42 });
  });

  it("should throw UndefinedNameError for undefined variable", () => {
    /** LOAD_NAME for an undefined variable should raise UndefinedNameError. */
    expect(() =>
      run(
        [{ opcode: OpCode.LOAD_NAME, operand: 0 }, { opcode: OpCode.HALT }],
        null,
        ["x"],
      ),
    ).toThrow(UndefinedNameError);
  });

  it("should overwrite previous value on re-store", () => {
    /** STORE_NAME should overwrite the previous value of a variable. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 }, // push 10
        { opcode: OpCode.STORE_NAME, operand: 0 },  // x = 10
        { opcode: OpCode.LOAD_CONST, operand: 1 },  // push 20
        { opcode: OpCode.STORE_NAME, operand: 0 },  // x = 20
        { opcode: OpCode.LOAD_NAME, operand: 0 },   // push x (should be 20)
        { opcode: OpCode.HALT },
      ],
      [10, 20],
      ["x"],
    );
    expect(vm.stack).toEqual([20]);
    expect(vm.variables["x"]).toBe(20);
  });

  it("should throw InvalidOperandError for out-of-range STORE_NAME index", () => {
    /** STORE_NAME with out-of-range index should raise InvalidOperandError. */
    expect(() =>
      run(
        [
          { opcode: OpCode.LOAD_CONST, operand: 0 },
          { opcode: OpCode.STORE_NAME, operand: 5 },
          { opcode: OpCode.HALT },
        ],
        [42],
        ["x"],
      ),
    ).toThrow(InvalidOperandError);
  });

  it("should throw InvalidOperandError for out-of-range LOAD_NAME index", () => {
    /** LOAD_NAME with out-of-range index should raise InvalidOperandError. */
    expect(() =>
      run(
        [{ opcode: OpCode.LOAD_NAME, operand: 5 }, { opcode: OpCode.HALT }],
        null,
        ["x"],
      ),
    ).toThrow(InvalidOperandError);
  });
});

describe("TestLocalVariables", () => {
  /** Tests for STORE_LOCAL and LOAD_LOCAL — indexed local variable slots. */

  it("should round-trip a value by slot index", () => {
    /** STORE_LOCAL + LOAD_LOCAL should round-trip a value by slot index. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.STORE_LOCAL, operand: 0 },
        { opcode: OpCode.LOAD_LOCAL, operand: 0 },
        { opcode: OpCode.HALT },
      ],
      [99],
    );
    expect(vm.stack).toEqual([99]);
  });

  it("should keep multiple local slots independent", () => {
    /** Multiple local slots should be independent. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },  // push 10
        { opcode: OpCode.STORE_LOCAL, operand: 0 },  // slot 0 = 10
        { opcode: OpCode.LOAD_CONST, operand: 1 },  // push 20
        { opcode: OpCode.STORE_LOCAL, operand: 1 },  // slot 1 = 20
        { opcode: OpCode.LOAD_LOCAL, operand: 0 },   // push slot 0 (10)
        { opcode: OpCode.LOAD_LOCAL, operand: 1 },   // push slot 1 (20)
        { opcode: OpCode.HALT },
      ],
      [10, 20],
    );
    expect(vm.stack).toEqual([10, 20]);
  });

  it("should throw InvalidOperandError for uninitialized slot", () => {
    /** LOAD_LOCAL from an uninitialized slot should raise an error. */
    expect(() =>
      run([{ opcode: OpCode.LOAD_LOCAL, operand: 0 }, { opcode: OpCode.HALT }]),
    ).toThrow(InvalidOperandError);
  });

  it("should auto-grow the locals array for high index", () => {
    /** STORE_LOCAL to a high index should auto-grow the locals array. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.STORE_LOCAL, operand: 5 },
        { opcode: OpCode.LOAD_LOCAL, operand: 5 },
        { opcode: OpCode.HALT },
      ],
      [77],
    );
    expect(vm.stack).toEqual([77]);
    expect(vm.locals.length).toBe(6); // slots 0-5
  });

  it("should throw InvalidOperandError for STORE_LOCAL with missing operand", () => {
    /** STORE_LOCAL without an operand should raise InvalidOperandError. */
    expect(() =>
      run(
        [
          { opcode: OpCode.LOAD_CONST, operand: 0 },
          { opcode: OpCode.STORE_LOCAL },
          { opcode: OpCode.HALT },
        ],
        [42],
      ),
    ).toThrow(InvalidOperandError);
  });

  it("should throw InvalidOperandError for LOAD_LOCAL with missing operand", () => {
    /** LOAD_LOCAL without an operand should raise InvalidOperandError. */
    expect(() =>
      run([{ opcode: OpCode.LOAD_LOCAL }, { opcode: OpCode.HALT }]),
    ).toThrow(InvalidOperandError);
  });
});

// =========================================================================
// Comparison Operations
// =========================================================================

describe("TestComparison", () => {
  /** Tests for CMP_EQ, CMP_LT, CMP_GT — comparison operations. */

  it("CMP_EQ should push 1 when values are equal", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.CMP_EQ },
        { opcode: OpCode.HALT },
      ],
      [42, 42],
    );
    expect(vm.stack).toEqual([1]);
  });

  it("CMP_EQ should push 0 when values are not equal", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.CMP_EQ },
        { opcode: OpCode.HALT },
      ],
      [42, 99],
    );
    expect(vm.stack).toEqual([0]);
  });

  it("CMP_LT should push 1 when a < b", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 }, // push 3 (a)
        { opcode: OpCode.LOAD_CONST, operand: 1 }, // push 7 (b)
        { opcode: OpCode.CMP_LT },                 // 3 < 7 -> 1
        { opcode: OpCode.HALT },
      ],
      [3, 7],
    );
    expect(vm.stack).toEqual([1]);
  });

  it("CMP_LT should push 0 when a >= b", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.CMP_LT },
        { opcode: OpCode.HALT },
      ],
      [7, 3],
    );
    expect(vm.stack).toEqual([0]);
  });

  it("CMP_GT should push 1 when a > b", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.CMP_GT },
        { opcode: OpCode.HALT },
      ],
      [7, 3],
    );
    expect(vm.stack).toEqual([1]);
  });

  it("CMP_GT should push 0 when a <= b", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.CMP_GT },
        { opcode: OpCode.HALT },
      ],
      [3, 7],
    );
    expect(vm.stack).toEqual([0]);
  });

  it("CMP_EQ should work with strings", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.CMP_EQ },
        { opcode: OpCode.HALT },
      ],
      ["hello", "hello"],
    );
    expect(vm.stack).toEqual([1]);
  });
});

// =========================================================================
// Control Flow
// =========================================================================

describe("TestJump", () => {
  /** Tests for JUMP — unconditional jump. */

  it("should skip over instructions to the target", () => {
    /** JUMP should skip over instructions to the target. */
    // Program: push 1, jump past the "push 2", push 3, halt
    // Expected stack: [1, 3]  (2 is skipped)
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 }, // 0: push 1
        { opcode: OpCode.JUMP, operand: 3 },        // 1: jump to 3
        { opcode: OpCode.LOAD_CONST, operand: 1 },  // 2: push 2 (SKIPPED)
        { opcode: OpCode.LOAD_CONST, operand: 2 },  // 3: push 3
        { opcode: OpCode.HALT },                     // 4: halt
      ],
      [1, 2, 3],
    );
    expect(vm.stack).toEqual([1, 3]);
  });

  it("should throw InvalidOperandError when operand is missing", () => {
    /** JUMP without an operand should raise InvalidOperandError. */
    expect(() =>
      run([{ opcode: OpCode.JUMP }, { opcode: OpCode.HALT }]),
    ).toThrow(InvalidOperandError);
  });
});

describe("TestJumpIfFalse", () => {
  /** Tests for JUMP_IF_FALSE — conditional branch on falsy values. */

  it("should jump when top of stack is 0 (falsy)", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },       // 0: push 0 (false)
        { opcode: OpCode.JUMP_IF_FALSE, operand: 3 },     // 1: jump to 3
        { opcode: OpCode.LOAD_CONST, operand: 1 },        // 2: push 999 (SKIPPED)
        { opcode: OpCode.HALT },                           // 3: halt
      ],
      [0, 999],
    );
    expect(vm.stack).toEqual([]); // 0 was consumed by the jump, 999 was skipped
  });

  it("should NOT jump when top of stack is truthy", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },       // 0: push 1 (true)
        { opcode: OpCode.JUMP_IF_FALSE, operand: 3 },     // 1: doesn't jump
        { opcode: OpCode.LOAD_CONST, operand: 1 },        // 2: push 42
        { opcode: OpCode.HALT },                           // 3: halt
      ],
      [1, 42],
    );
    expect(vm.stack).toEqual([42]);
  });

  it("should consider empty string as falsy", () => {
    /** JUMP_IF_FALSE should consider empty string as falsy. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },       // push ""
        { opcode: OpCode.JUMP_IF_FALSE, operand: 3 },     // jump (empty string is falsy)
        { opcode: OpCode.LOAD_CONST, operand: 1 },        // SKIPPED
        { opcode: OpCode.HALT },
      ],
      ["", 999],
    );
    expect(vm.stack).toEqual([]);
  });
});

describe("TestJumpIfTrue", () => {
  /** Tests for JUMP_IF_TRUE — conditional branch on truthy values. */

  it("should jump when top of stack is truthy", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },       // 0: push 1 (true)
        { opcode: OpCode.JUMP_IF_TRUE, operand: 3 },      // 1: jump to 3
        { opcode: OpCode.LOAD_CONST, operand: 1 },        // 2: push 999 (SKIPPED)
        { opcode: OpCode.HALT },                           // 3: halt
      ],
      [1, 999],
    );
    expect(vm.stack).toEqual([]);
  });

  it("should NOT jump when top of stack is falsy", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },       // 0: push 0 (false)
        { opcode: OpCode.JUMP_IF_TRUE, operand: 3 },      // 1: doesn't jump
        { opcode: OpCode.LOAD_CONST, operand: 1 },        // 2: push 42
        { opcode: OpCode.HALT },                           // 3: halt
      ],
      [0, 42],
    );
    expect(vm.stack).toEqual([42]);
  });
});

describe("TestLoop", () => {
  /** Test loops built from JUMP instructions. */

  it("should execute a countdown loop correctly", () => {
    /**
     * A countdown loop using JUMP_IF_FALSE.
     *
     * Equivalent to:
     *     x = 3
     *     while x > 0:
     *         print(x)
     *         x = x - 1
     */
    const [vm] = run(
      [
        // x = 3
        { opcode: OpCode.LOAD_CONST, operand: 0 },   // 0: push 3
        { opcode: OpCode.STORE_NAME, operand: 0 },    // 1: x = 3

        // while x > 0:
        { opcode: OpCode.LOAD_NAME, operand: 0 },     // 2: push x
        { opcode: OpCode.LOAD_CONST, operand: 1 },    // 3: push 0
        { opcode: OpCode.CMP_GT },                     // 4: x > 0?
        { opcode: OpCode.JUMP_IF_FALSE, operand: 13 }, // 5: if not, exit loop

        // print(x)
        { opcode: OpCode.LOAD_NAME, operand: 0 },     // 6: push x
        { opcode: OpCode.PRINT },                      // 7: print x

        // x = x - 1
        { opcode: OpCode.LOAD_NAME, operand: 0 },     // 8: push x
        { opcode: OpCode.LOAD_CONST, operand: 2 },    // 9: push 1
        { opcode: OpCode.SUB },                        // 10: x - 1
        { opcode: OpCode.STORE_NAME, operand: 0 },    // 11: x = x - 1

        { opcode: OpCode.JUMP, operand: 2 },          // 12: back to loop start

        { opcode: OpCode.HALT },                       // 13: done
      ],
      [3, 0, 1],
      ["x"],
    );
    expect(vm.output).toEqual(["3", "2", "1"]);
    expect(vm.variables["x"]).toBe(0);
  });
});

// =========================================================================
// I/O Operations
// =========================================================================

describe("TestPrint", () => {
  /** Tests for PRINT — capturing output. */

  it("should convert an integer to string and capture it", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.PRINT },
        { opcode: OpCode.HALT },
      ],
      [42],
    );
    expect(vm.output).toEqual(["42"]);
    expect(vm.stack).toEqual([]); // PRINT consumes the value
  });

  it("should capture a string as-is", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.PRINT },
        { opcode: OpCode.HALT },
      ],
      ["hello world"],
    );
    expect(vm.output).toEqual(["hello world"]);
  });

  it("should append multiple prints to the output list", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.PRINT },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.PRINT },
        { opcode: OpCode.HALT },
      ],
      ["hello", "world"],
    );
    expect(vm.output).toEqual(["hello", "world"]);
  });

  it("should throw StackUnderflowError on empty stack", () => {
    expect(() =>
      run([{ opcode: OpCode.PRINT }, { opcode: OpCode.HALT }]),
    ).toThrow(StackUnderflowError);
  });
});

// =========================================================================
// HALT and VM Control
// =========================================================================

describe("TestHalt", () => {
  /** Tests for HALT — stopping execution. */

  it("should stop the VM immediately", () => {
    const [vm, traces] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.HALT },
        { opcode: OpCode.LOAD_CONST, operand: 1 }, // Should NOT execute
      ],
      [1, 2],
    );
    expect(vm.stack).toEqual([1]); // Only first LOAD_CONST ran
    expect(vm.halted).toBe(true);
    expect(traces.length).toBe(2); // LOAD_CONST + HALT
  });

  it("should terminate when PC runs off the end (no HALT)", () => {
    /** A program without HALT should still terminate (PC runs off the end). */
    const [vm, traces] = run(
      [{ opcode: OpCode.LOAD_CONST, operand: 0 }],
      [42],
    );
    expect(vm.stack).toEqual([42]);
    expect(traces.length).toBe(1);
  });
});

// =========================================================================
// Function Operations
// =========================================================================

describe("TestFunctions", () => {
  /** Tests for CALL and RETURN — function calling. */

  it("should execute a function stored as a CodeObject variable", () => {
    /** CALL should execute a function stored as a CodeObject variable. */
    // Define a function that pushes 99 and prints it.
    const funcCode = assembleCode(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.PRINT },
        { opcode: OpCode.RETURN },
      ],
      [99],
    );

    // Main program: store the function, call it.
    const vm = new VirtualMachine();
    vm.variables["my_func"] = funcCode;

    const mainCode = assembleCode(
      [
        { opcode: OpCode.CALL, operand: 0 },
        { opcode: OpCode.HALT },
      ],
      null,
      ["my_func"],
    );
    vm.execute(mainCode);
    expect(vm.output).toEqual(["99"]);
  });

  it("should throw UndefinedNameError for undefined function", () => {
    /** CALL on an undefined name should raise UndefinedNameError. */
    expect(() =>
      run(
        [{ opcode: OpCode.CALL, operand: 0 }, { opcode: OpCode.HALT }],
        null,
        ["no_such_func"],
      ),
    ).toThrow(UndefinedNameError);
  });

  it("should throw VMError for non-callable", () => {
    /** CALL on a non-CodeObject should raise VMError. */
    const vm = new VirtualMachine();
    vm.variables["not_func"] = 42;

    const code = assembleCode(
      [{ opcode: OpCode.CALL, operand: 0 }, { opcode: OpCode.HALT }],
      null,
      ["not_func"],
    );
    expect(() => vm.execute(code)).toThrow(VMError);
  });

  it("should treat RETURN at top level as HALT", () => {
    /** RETURN at the top level (no call frame) should act like HALT. */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.RETURN },
        { opcode: OpCode.LOAD_CONST, operand: 1 }, // Should NOT execute
      ],
      [42, 99],
    );
    expect(vm.stack).toEqual([42]);
    expect(vm.halted).toBe(true);
  });
});

// =========================================================================
// Trace Output
// =========================================================================

describe("TestTrace", () => {
  /** Tests for VMTrace — the execution trace system. */

  it("should capture stack states before and after each instruction", () => {
    const [, traces] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.ADD },
        { opcode: OpCode.HALT },
      ],
      [3, 4],
    );
    // LOAD_CONST 3
    expect(traces[0].stackBefore).toEqual([]);
    expect(traces[0].stackAfter).toEqual([3]);
    expect(traces[0].pc).toBe(0);

    // LOAD_CONST 4
    expect(traces[1].stackBefore).toEqual([3]);
    expect(traces[1].stackAfter).toEqual([3, 4]);
    expect(traces[1].pc).toBe(1);

    // ADD
    expect(traces[2].stackBefore).toEqual([3, 4]);
    expect(traces[2].stackAfter).toEqual([7]);
    expect(traces[2].pc).toBe(2);
  });

  it("should snapshot the variables dict after each step", () => {
    const [, traces] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.STORE_NAME, operand: 0 },
        { opcode: OpCode.HALT },
      ],
      [42],
      ["x"],
    );
    // After STORE_NAME, variables should contain x=42
    expect(traces[1].variables).toEqual({ x: 42 });
  });

  it("should capture PRINT output in the trace", () => {
    const [, traces] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.PRINT },
        { opcode: OpCode.HALT },
      ],
      [42],
    );
    // The PRINT trace (index 1) should have output
    expect(traces[1].output).toBe("42");

    // Non-PRINT traces should have null
    expect(traces[0].output).toBeNull();
  });

  it("should have non-empty descriptions", () => {
    const [, traces] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.HALT },
      ],
      [42],
    );
    for (const trace of traces) {
      expect(trace.description).not.toBe("");
    }
  });

  it("should have meaningful description content", () => {
    const [, traces] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.ADD },
        { opcode: OpCode.STORE_NAME, operand: 0 },
        { opcode: OpCode.HALT },
      ],
      [3, 4],
      ["x"],
    );
    expect(traces[0].description).not.toContain("42"); // Shouldn't mention 42
    expect(traces[0].description).toContain("3");      // Should mention the constant
    expect(
      traces[2].description.toLowerCase().includes("sum") ||
      traces[2].description.includes("+"),
    ).toBe(true);
    expect(traces[3].description).toContain("x");
  });
});

// =========================================================================
// Error Cases
// =========================================================================

describe("TestErrors", () => {
  /** Tests for runtime error handling. */

  it("should throw StackUnderflowError on arithmetic with empty stack", () => {
    expect(() =>
      run([{ opcode: OpCode.SUB }, { opcode: OpCode.HALT }]),
    ).toThrow(StackUnderflowError);
  });

  it("should throw StackUnderflowError on comparison with empty stack", () => {
    expect(() =>
      run([{ opcode: OpCode.CMP_EQ }, { opcode: OpCode.HALT }]),
    ).toThrow(StackUnderflowError);
  });

  it("should throw InvalidOpcodeError for unknown opcode", () => {
    /** An unrecognized opcode should raise InvalidOpcodeError. */
    // Create an instruction with a fake opcode value.
    const fakeInstruction: Instruction = {
      opcode: 0xaa as any,
    };

    const code: CodeObject = {
      instructions: [fakeInstruction],
      constants: [],
      names: [],
    };
    const vm = new VirtualMachine();
    expect(() => vm.execute(code)).toThrow(InvalidOpcodeError);
  });
});

// =========================================================================
// End-to-End Programs
// =========================================================================

describe("TestEndToEnd", () => {
  /** End-to-end tests that represent real programs compiled to bytecode. */

  it("should compile and run: x = 1 + 2", () => {
    /**
     * Compile and run: x = 1 + 2
     *
     * This is the canonical introductory example. A compiler would produce:
     *     LOAD_CONST 0  (1)
     *     LOAD_CONST 1  (2)
     *     ADD
     *     STORE_NAME 0  (x)
     *     HALT
     */
    const [vm, traces] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.ADD },
        { opcode: OpCode.STORE_NAME, operand: 0 },
        { opcode: OpCode.HALT },
      ],
      [1, 2],
      ["x"],
    );
    expect(vm.variables["x"]).toBe(3);
    expect(vm.stack).toEqual([]); // Stack should be clean after store
    expect(traces.length).toBe(5);
  });

  it("should compile and run an if/else branch", () => {
    /**
     * Compile and run an if/else:
     *
     *     x = 10
     *     if x > 5:
     *         print("big")
     *     else:
     *         print("small")
     *
     * Expected: prints "big"
     */
    const [vm] = run(
      [
        // x = 10
        { opcode: OpCode.LOAD_CONST, operand: 0 },   // 0: push 10
        { opcode: OpCode.STORE_NAME, operand: 0 },    // 1: x = 10

        // if x > 5:
        { opcode: OpCode.LOAD_NAME, operand: 0 },     // 2: push x
        { opcode: OpCode.LOAD_CONST, operand: 1 },    // 3: push 5
        { opcode: OpCode.CMP_GT },                     // 4: x > 5?
        { opcode: OpCode.JUMP_IF_FALSE, operand: 9 },  // 5: if false, goto else

        // then: print("big")
        { opcode: OpCode.LOAD_CONST, operand: 2 },    // 6: push "big"
        { opcode: OpCode.PRINT },                      // 7: print
        { opcode: OpCode.JUMP, operand: 11 },          // 8: skip else

        // else: print("small")
        { opcode: OpCode.LOAD_CONST, operand: 3 },    // 9: push "small"
        { opcode: OpCode.PRINT },                      // 10: print

        { opcode: OpCode.HALT },                       // 11: done
      ],
      [10, 5, "big", "small"],
      ["x"],
    );
    expect(vm.output).toEqual(["big"]);
  });

  it("should compile and run: sum of 1 to 5", () => {
    /**
     * Compile and run: sum of 1 to 5 (corrected jump targets).
     *
     *     total = 0
     *     i = 1
     *     while i <= 5:
     *         total = total + i
     *         i = i + 1
     *     print(total)
     *
     * Expected: prints "15"
     */
    const [vm] = run(
      [
        // total = 0
        { opcode: OpCode.LOAD_CONST, operand: 0 },   // 0: push 0
        { opcode: OpCode.STORE_NAME, operand: 0 },    // 1: total = 0

        // i = 1
        { opcode: OpCode.LOAD_CONST, operand: 1 },   // 2: push 1
        { opcode: OpCode.STORE_NAME, operand: 1 },    // 3: i = 1

        // while i <= 5:  ->  if i > 5: break
        { opcode: OpCode.LOAD_NAME, operand: 1 },     // 4: push i
        { opcode: OpCode.LOAD_CONST, operand: 2 },    // 5: push 5
        { opcode: OpCode.CMP_GT },                     // 6: i > 5?
        { opcode: OpCode.JUMP_IF_TRUE, operand: 17 },  // 7: if i > 5, exit

        // total = total + i
        { opcode: OpCode.LOAD_NAME, operand: 0 },     // 8: push total
        { opcode: OpCode.LOAD_NAME, operand: 1 },     // 9: push i
        { opcode: OpCode.ADD },                        // 10: total + i
        { opcode: OpCode.STORE_NAME, operand: 0 },    // 11: total = ...

        // i = i + 1
        { opcode: OpCode.LOAD_NAME, operand: 1 },     // 12: push i
        { opcode: OpCode.LOAD_CONST, operand: 1 },    // 13: push 1
        { opcode: OpCode.ADD },                        // 14: i + 1
        { opcode: OpCode.STORE_NAME, operand: 1 },    // 15: i = ...

        { opcode: OpCode.JUMP, operand: 4 },          // 16: back to loop

        // print(total)
        { opcode: OpCode.LOAD_NAME, operand: 0 },     // 17: push total
        { opcode: OpCode.PRINT },                      // 18: print
        { opcode: OpCode.HALT },                       // 19: done
      ],
      [0, 1, 5],
      ["total", "i"],
    );
    expect(vm.output).toEqual(["15"]);
    expect(vm.variables["total"]).toBe(15);
    expect(vm.variables["i"]).toBe(6); // i incremented past 5
  });

  it("should compile and run: string concatenation", () => {
    /**
     * Compile and run: greeting = "hello" + " " + "world"
     *
     * Demonstrates that the VM handles strings with the same arithmetic
     * opcodes as integers — dynamic typing in action.
     */
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },  // "hello"
        { opcode: OpCode.LOAD_CONST, operand: 1 },  // " "
        { opcode: OpCode.ADD },                      // "hello "
        { opcode: OpCode.LOAD_CONST, operand: 2 },  // "world"
        { opcode: OpCode.ADD },                      // "hello world"
        { opcode: OpCode.STORE_NAME, operand: 0 },  // greeting = "hello world"
        { opcode: OpCode.HALT },
      ],
      ["hello", " ", "world"],
      ["greeting"],
    );
    expect(vm.variables["greeting"]).toBe("hello world");
  });
});

// =========================================================================
// assembleCode helper
// =========================================================================

describe("TestAssembleCode", () => {
  /** Tests for the assembleCode convenience function. */

  it("should produce a valid CodeObject", () => {
    const code = assembleCode(
      [{ opcode: OpCode.HALT }],
      [1, 2, 3],
      ["x"],
    );
    expect(code.instructions.length).toBe(1);
    expect(code.constants).toEqual([1, 2, 3]);
    expect(code.names).toEqual(["x"]);
  });

  it("should default to empty pools", () => {
    /** assembleCode with no constants/names should use empty lists. */
    const code = assembleCode([{ opcode: OpCode.HALT }]);
    expect(code.constants).toEqual([]);
    expect(code.names).toEqual([]);
  });
});

// =========================================================================
// VM Reset
// =========================================================================

describe("TestReset", () => {
  /** Tests for the VM's reset method. */

  it("should restore the VM to its initial state", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.STORE_NAME, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.PRINT },
        { opcode: OpCode.HALT },
      ],
      [42, 99],
      ["x"],
    );

    // VM has state now
    expect(Object.keys(vm.variables).length).toBeGreaterThan(0);
    expect(vm.output.length).toBeGreaterThan(0);
    expect(vm.halted).toBe(true);

    // Reset should clear everything
    vm.reset();
    expect(vm.stack).toEqual([]);
    expect(vm.variables).toEqual({});
    expect(vm.locals).toEqual([]);
    expect(vm.pc).toBe(0);
    expect(vm.halted).toBe(false);
    expect(vm.output).toEqual([]);
    expect(vm.callStack).toEqual([]);
  });
});

// =========================================================================
// Instruction repr
// =========================================================================

describe("TestInstructionToString", () => {
  /** Tests for the instructionToString function. */

  it("should show opcode name and operand", () => {
    const instr: Instruction = { opcode: OpCode.LOAD_CONST, operand: 0 };
    const s = instructionToString(instr);
    expect(s).toContain("LOAD_CONST");
    expect(s).toContain("0");
  });

  it("should show just the opcode name without operand", () => {
    const instr: Instruction = { opcode: OpCode.ADD };
    const s = instructionToString(instr);
    expect(s).toContain("ADD");
  });

  it("should show string operands with quotes", () => {
    const instr: Instruction = { opcode: OpCode.LOAD_CONST, operand: "hello" };
    const s = instructionToString(instr);
    expect(s).toContain("LOAD_CONST");
    expect(s).toContain("hello");
  });
});

// =========================================================================
// Edge Cases and Additional Coverage
// =========================================================================

describe("TestEdgeCases", () => {
  /** Additional edge case tests for thorough coverage. */

  it("DUP should preserve the original value below", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.DUP },
        { opcode: OpCode.HALT },
      ],
      [10, 20],
    );
    expect(vm.stack).toEqual([10, 20, 20]);
  });

  it("CMP_LT with equal values should push 0", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.CMP_LT },
        { opcode: OpCode.HALT },
      ],
      [5],
    );
    expect(vm.stack).toEqual([0]);
  });

  it("CMP_GT with equal values should push 0", () => {
    const [vm] = run(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.CMP_GT },
        { opcode: OpCode.HALT },
      ],
      [5],
    );
    expect(vm.stack).toEqual([0]);
  });

  it("MUL on empty stack should throw StackUnderflowError", () => {
    expect(() =>
      run([{ opcode: OpCode.MUL }, { opcode: OpCode.HALT }]),
    ).toThrow(StackUnderflowError);
  });

  it("DIV on empty stack should throw StackUnderflowError", () => {
    expect(() =>
      run([{ opcode: OpCode.DIV }, { opcode: OpCode.HALT }]),
    ).toThrow(StackUnderflowError);
  });

  it("should produce DIVISION BY ZERO description for div-by-zero case", () => {
    /** The describe method should handle div-by-zero case in description. */
    const code = assembleCode(
      [
        { opcode: OpCode.LOAD_CONST, operand: 0 },
        { opcode: OpCode.LOAD_CONST, operand: 1 },
        { opcode: OpCode.DIV },
        { opcode: OpCode.HALT },
      ],
      [10, 0],
    );
    const vm = new VirtualMachine();
    // Execute LOAD_CONST twice
    vm.step(code);
    vm.step(code);
    // Check the describe path directly before the error
    const desc = vm._describe(
      { opcode: OpCode.DIV },
      code,
      [...vm.stack],
    );
    expect(desc).toContain("DIVISION BY ZERO");
  });
});
