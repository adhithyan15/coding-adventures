/**
 * Comprehensive tests for the GenericVM — the pluggable bytecode interpreter.
 *
 * These tests verify every public method, every error path, and several
 * end-to-end scenarios. They demonstrate how a language would wire up its
 * opcodes and run programs through the GenericVM.
 *
 * We follow the Arrange-Act-Assert pattern throughout:
 *     1. Arrange: Create a GenericVM, register handlers, build a CodeObject.
 *     2. Act: Execute the code or call individual methods.
 *     3. Assert: Check the resulting state (stack, output, traces, errors).
 *
 * Since the GenericVM has no built-in opcodes, every test must register its
 * own handlers. We define a small set of "test opcodes" used across tests
 * to keep things consistent and readable.
 */

import { describe, it, expect } from "vitest";

import {
  GenericVM,
  MaxRecursionError,
  VMTypeError,
  type OpcodeHandler,
  type BuiltinFunction,
} from "../src/generic-vm.js";

import type { Instruction, CodeObject, VMTrace } from "../src/vm.js";
import {
  VMError,
  StackUnderflowError,
  InvalidOpcodeError,
} from "../src/vm.js";

// =========================================================================
// Test Opcodes
// =========================================================================

/**
 * A minimal set of opcodes for testing purposes.
 *
 * These mirror the opcodes from ``vm.ts`` but are defined here so the
 * tests are self-contained. The numeric values are arbitrary — they just
 * need to be unique within a test.
 *
 * Think of these as a tiny "test language" that we compile programs in.
 */
const TestOp = {
  /** Push a constant from the constants pool onto the stack. */
  LOAD_CONST: 0x01,

  /** Pop two values, push their sum. */
  ADD: 0x20,

  /** Pop the top value and add it to the output. */
  PRINT: 0x60,

  /** Stop execution. */
  HALT: 0xff,

  /** Pop two values, push their difference (second - first). */
  SUB: 0x21,

  /** Jump unconditionally to the operand address. */
  JUMP: 0x40,

  /** Store the top of stack into a named variable. */
  STORE_NAME: 0x10,

  /** Load a named variable onto the stack. */
  LOAD_NAME: 0x11,
} as const;

// =========================================================================
// Handler Factories
// =========================================================================

/**
 * Create a standard LOAD_CONST handler.
 *
 * Reads the operand as an index into the constants pool, pushes the
 * value, and advances the PC. This is the most basic handler — it
 * demonstrates the handler protocol clearly:
 *
 *   1. Read operand from instruction.
 *   2. Read data from code object.
 *   3. Modify VM state (push).
 *   4. Advance PC.
 *   5. Return a description.
 */
function makeLoadConstHandler(): OpcodeHandler {
  return (vm, instr, code) => {
    const index = instr.operand as number;
    const value = code.constants[index];
    vm.push(value);
    vm.advancePc();
    return `Loaded constant ${value}`;
  };
}

/**
 * Create a standard ADD handler.
 *
 * Pops two numbers, pushes their sum. Demonstrates a binary operation
 * handler — pop two operands, compute, push result.
 */
function makeAddHandler(): OpcodeHandler {
  return (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    vm.push(a + b);
    vm.advancePc();
    return `${a} + ${b} = ${a + b}`;
  };
}

/**
 * Create a standard SUB handler.
 *
 * Pops two numbers (top is subtrahend, second is minuend), pushes
 * the difference.
 */
function makeSubHandler(): OpcodeHandler {
  return (vm) => {
    const b = vm.pop() as number;
    const a = vm.pop() as number;
    vm.push(a - b);
    vm.advancePc();
    return `${a} - ${b} = ${a - b}`;
  };
}

/**
 * Create a standard PRINT handler.
 *
 * Pops the top value and appends its string representation to the
 * VM's output array.
 */
function makePrintHandler(): OpcodeHandler {
  return (vm) => {
    const value = vm.pop();
    const text = String(value);
    vm.output.push(text);
    vm.advancePc();
    return `Printed ${text}`;
  };
}

/**
 * Create a standard HALT handler.
 *
 * Sets the halted flag to true, causing the eval loop to stop.
 * Does NOT advance the PC — there's nowhere to go after halting.
 */
function makeHaltHandler(): OpcodeHandler {
  return (vm) => {
    vm.halted = true;
    return "Halted";
  };
}

/**
 * Create a standard JUMP handler.
 *
 * Reads the operand as a target PC and jumps to it. Demonstrates
 * how control flow handlers use ``jumpTo()`` instead of ``advancePc()``.
 */
function makeJumpHandler(): OpcodeHandler {
  return (vm, instr) => {
    const target = instr.operand as number;
    vm.jumpTo(target);
    return `Jumped to ${target}`;
  };
}

/**
 * Create a standard STORE_NAME handler.
 *
 * Pops the top of stack and stores it in a named variable.
 */
function makeStoreNameHandler(): OpcodeHandler {
  return (vm, instr, code) => {
    const nameIndex = instr.operand as number;
    const name = code.names[nameIndex];
    const value = vm.pop();
    vm.variables[name] = value;
    vm.advancePc();
    return `Stored ${value} into '${name}'`;
  };
}

/**
 * Create a standard LOAD_NAME handler.
 *
 * Looks up a named variable and pushes its value onto the stack.
 */
function makeLoadNameHandler(): OpcodeHandler {
  return (vm, instr, code) => {
    const nameIndex = instr.operand as number;
    const name = code.names[nameIndex];
    const value = vm.variables[name];
    vm.push(value);
    vm.advancePc();
    return `Loaded '${name}' = ${value}`;
  };
}

// =========================================================================
// Helpers
// =========================================================================

/**
 * Create a GenericVM with a standard set of test handlers pre-registered.
 *
 * This is the "test language runtime" — a VM configured to understand
 * our test opcodes. Most tests use this instead of registering handlers
 * individually, to keep test code focused on what's being tested.
 */
function createTestVM(): GenericVM {
  const vm = new GenericVM();
  vm.registerOpcode(TestOp.LOAD_CONST, makeLoadConstHandler());
  vm.registerOpcode(TestOp.ADD, makeAddHandler());
  vm.registerOpcode(TestOp.SUB, makeSubHandler());
  vm.registerOpcode(TestOp.PRINT, makePrintHandler());
  vm.registerOpcode(TestOp.HALT, makeHaltHandler());
  vm.registerOpcode(TestOp.JUMP, makeJumpHandler());
  vm.registerOpcode(TestOp.STORE_NAME, makeStoreNameHandler());
  vm.registerOpcode(TestOp.LOAD_NAME, makeLoadNameHandler());
  return vm;
}

/**
 * Build a CodeObject from instructions, constants, and names.
 *
 * A thin wrapper that provides sensible defaults (empty arrays)
 * for constants and names when they're not needed.
 */
function makeCode(
  instructions: Instruction[],
  constants: (number | string)[] = [],
  names: string[] = [],
): CodeObject {
  return { instructions, constants, names };
}

/**
 * Create a test VM, execute instructions, and return both.
 *
 * The most common test pattern: create a configured VM, run some
 * bytecode, check the results.
 */
function run(
  instructions: Instruction[],
  constants: (number | string)[] = [],
  names: string[] = [],
): [GenericVM, VMTrace[]] {
  const vm = createTestVM();
  const code = makeCode(instructions, constants, names);
  const traces = vm.execute(code);
  return [vm, traces];
}

// =========================================================================
// Tests
// =========================================================================

describe("GenericVM", () => {
  // -----------------------------------------------------------------------
  // Basic Execution
  // -----------------------------------------------------------------------

  describe("basic execution", () => {
    it("should execute LOAD_CONST + HALT and leave value on stack", () => {
      /**
       * The simplest possible program: push a constant and halt.
       *
       * Program: LOAD_CONST 0; HALT
       * Constants: [42]
       * Expected: stack = [42], halted = true
       */
      const [vm, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [42],
      );

      expect(vm.stack).toEqual([42]);
      expect(vm.halted).toBe(true);
      expect(traces).toHaveLength(2);
    });

    it("should execute LOAD_CONST + LOAD_CONST + ADD + HALT", () => {
      /**
       * Basic arithmetic: 10 + 32 = 42.
       *
       * This tests the fundamental stack-based computation pattern:
       * push operands, then apply an operator that pops them and
       * pushes the result.
       *
       *   Stack transitions:
       *     []        → LOAD_CONST 0 → [10]
       *     [10]      → LOAD_CONST 1 → [10, 32]
       *     [10, 32]  → ADD          → [42]
       *     [42]      → HALT         → [42]
       */
      const [vm, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          { opcode: TestOp.HALT },
        ],
        [10, 32],
      );

      expect(vm.stack).toEqual([42]);
      expect(traces).toHaveLength(4);
    });

    it("should capture print output", () => {
      /**
       * Test the output mechanism: PRINT pops a value and appends it
       * to the output array.
       *
       * Program: LOAD_CONST 0; PRINT; HALT
       * Expected output: ["Hello, World!"]
       */
      const [vm] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.PRINT },
          { opcode: TestOp.HALT },
        ],
        ["Hello, World!"],
      );

      expect(vm.output).toEqual(["Hello, World!"]);
      expect(vm.stack).toEqual([]);
    });

    it("should handle multiple print statements", () => {
      /**
       * Multiple PRINT calls accumulate in the output array.
       */
      const [vm] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.PRINT },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.PRINT },
          { opcode: TestOp.HALT },
        ],
        ["Hello", "World"],
      );

      expect(vm.output).toEqual(["Hello", "World"]);
    });
  });

  // -----------------------------------------------------------------------
  // Trace Recording
  // -----------------------------------------------------------------------

  describe("trace recording", () => {
    it("should record one trace per instruction", () => {
      /**
       * Every executed instruction produces exactly one VMTrace.
       * A 3-instruction program should yield 3 traces.
       */
      const [, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.HALT },
        ],
        [10, 20],
      );

      expect(traces).toHaveLength(3);
    });

    it("should record the correct PC for each trace", () => {
      /**
       * Each trace's ``pc`` field should be the PC *before* that
       * instruction executed — i.e., the index of the instruction
       * in the instructions array.
       */
      const [, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          { opcode: TestOp.HALT },
        ],
        [3, 7],
      );

      expect(traces[0].pc).toBe(0);
      expect(traces[1].pc).toBe(1);
      expect(traces[2].pc).toBe(2);
      expect(traces[3].pc).toBe(3);
    });

    it("should snapshot the stack before and after each instruction", () => {
      /**
       * The trace captures stack state *before* and *after* each step.
       * This lets debuggers show the transformation at each instruction.
       *
       *   Step 0 (LOAD_CONST 10): before=[], after=[10]
       *   Step 1 (LOAD_CONST 20): before=[10], after=[10, 20]
       *   Step 2 (ADD):           before=[10, 20], after=[30]
       */
      const [, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          { opcode: TestOp.HALT },
        ],
        [10, 20],
      );

      // Step 0: LOAD_CONST 10
      expect(traces[0].stackBefore).toEqual([]);
      expect(traces[0].stackAfter).toEqual([10]);

      // Step 1: LOAD_CONST 20
      expect(traces[1].stackBefore).toEqual([10]);
      expect(traces[1].stackAfter).toEqual([10, 20]);

      // Step 2: ADD
      expect(traces[2].stackBefore).toEqual([10, 20]);
      expect(traces[2].stackAfter).toEqual([30]);
    });

    it("should include the instruction in each trace", () => {
      /**
       * Each trace captures which instruction was executed, so you
       * can correlate the trace with the source bytecode.
       */
      const [, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [42],
      );

      expect(traces[0].instruction).toEqual({
        opcode: TestOp.LOAD_CONST,
        operand: 0,
      });
      expect(traces[1].instruction).toEqual({ opcode: TestOp.HALT });
    });

    it("should include a description from the handler", () => {
      /**
       * The description field comes from the handler's return value.
       * Our LOAD_CONST handler returns "Loaded constant <value>".
       */
      const [, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [42],
      );

      expect(traces[0].description).toBe("Loaded constant 42");
      expect(traces[1].description).toBe("Halted");
    });

    it("should capture variables in traces", () => {
      /**
       * After a STORE_NAME, the trace's variables snapshot should
       * contain the stored variable.
       */
      const [, traces] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.STORE_NAME, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [42],
        ["x"],
      );

      // After STORE_NAME, variables should contain x = 42
      expect(traces[1].variables).toEqual({ x: 42 });
    });
  });

  // -----------------------------------------------------------------------
  // Stack Operations
  // -----------------------------------------------------------------------

  describe("stack operations", () => {
    it("should push and pop values correctly", () => {
      /**
       * Direct test of push() and pop() — not through opcodes.
       * Push 10, 20, 30, then pop in LIFO order.
       */
      const vm = new GenericVM();

      vm.push(10);
      vm.push(20);
      vm.push(30);

      expect(vm.pop()).toBe(30);
      expect(vm.pop()).toBe(20);
      expect(vm.pop()).toBe(10);
    });

    it("should peek at the top without removing it", () => {
      /**
       * peek() returns the top value but leaves the stack unchanged.
       */
      const vm = new GenericVM();

      vm.push(42);
      expect(vm.peek()).toBe(42);
      expect(vm.stack).toEqual([42]); // still there
    });

    it("should throw StackUnderflowError when popping empty stack", () => {
      /**
       * Popping from an empty stack is a bug in the compiler (it
       * generated bytecode that pops more than it pushes). The VM
       * reports this clearly with StackUnderflowError.
       */
      const vm = new GenericVM();

      expect(() => vm.pop()).toThrow(StackUnderflowError);
      expect(() => vm.pop()).toThrow("Cannot pop from an empty stack");
    });

    it("should throw StackUnderflowError when peeking empty stack", () => {
      /**
       * Same as pop — you can't peek at nothing.
       */
      const vm = new GenericVM();

      expect(() => vm.peek()).toThrow(StackUnderflowError);
      expect(() => vm.peek()).toThrow("Cannot peek at an empty stack");
    });

    it("should handle null values on the stack", () => {
      /**
       * null is a valid VMValue — it represents "no value" (like
       * Python's None or Ruby's nil).
       */
      const vm = new GenericVM();

      vm.push(null);
      expect(vm.peek()).toBeNull();
      expect(vm.pop()).toBeNull();
    });

    it("should handle string values on the stack", () => {
      /**
       * Strings are valid VMValues — used for string constants and
       * the string results of operations.
       */
      const vm = new GenericVM();

      vm.push("hello");
      expect(vm.peek()).toBe("hello");
      expect(vm.pop()).toBe("hello");
    });
  });

  // -----------------------------------------------------------------------
  // Call Stack
  // -----------------------------------------------------------------------

  describe("call stack", () => {
    it("should push and pop frames correctly", () => {
      /**
       * pushFrame/popFrame manage the call stack in LIFO order,
       * just like function call/return.
       */
      const vm = new GenericVM();

      const frame1 = { returnAddress: 5, caller: "main" };
      const frame2 = { returnAddress: 10, caller: "foo" };

      vm.pushFrame(frame1);
      vm.pushFrame(frame2);

      expect(vm.popFrame()).toEqual(frame2);
      expect(vm.popFrame()).toEqual(frame1);
    });

    it("should throw VMError when popping from empty call stack", () => {
      /**
       * A RETURN without a matching CALL means the call stack is empty.
       * This is always a bug — either in the compiler or the bytecode.
       */
      const vm = new GenericVM();

      expect(() => vm.popFrame()).toThrow(VMError);
      expect(() => vm.popFrame()).toThrow(
        "Cannot pop from an empty call stack",
      );
    });

    it("should enforce max recursion depth", () => {
      /**
       * When maxRecursionDepth is set, pushFrame() throws
       * MaxRecursionError once the limit is reached.
       *
       * This prevents infinite recursion from consuming all memory.
       */
      const vm = new GenericVM();
      vm.setMaxRecursionDepth(3);

      vm.pushFrame({ depth: 1 });
      vm.pushFrame({ depth: 2 });
      vm.pushFrame({ depth: 3 });

      // The 4th push should fail — we're already at depth 3
      expect(() => vm.pushFrame({ depth: 4 })).toThrow(MaxRecursionError);
      expect(() => vm.pushFrame({ depth: 4 })).toThrow(
        "Maximum recursion depth of 3 exceeded",
      );
    });

    it("should allow zero recursion depth (no calls at all)", () => {
      /**
       * A max recursion depth of 0 means no function calls are allowed.
       * Even the first pushFrame() should fail.
       *
       * This is an edge case but it's useful for sandboxing — you can
       * create a VM that only runs flat (non-recursive) code.
       */
      const vm = new GenericVM();
      vm.setMaxRecursionDepth(0);

      expect(() => vm.pushFrame({ depth: 1 })).toThrow(MaxRecursionError);
    });

    it("should allow unlimited recursion when depth is null", () => {
      /**
       * The default maxRecursionDepth is null (unlimited). We can
       * push many frames without hitting a limit.
       */
      const vm = new GenericVM();

      // null is the default — verify it
      expect(vm.getMaxRecursionDepth()).toBeNull();

      // Push 100 frames — should all succeed
      for (let i = 0; i < 100; i++) {
        vm.pushFrame({ depth: i });
      }

      expect(vm.callStack).toHaveLength(100);
    });

    it("should allow changing recursion depth after construction", () => {
      /**
       * setMaxRecursionDepth can be called at any time to adjust
       * the limit — useful for languages that let users configure it
       * (like Python's sys.setrecursionlimit()).
       */
      const vm = new GenericVM();

      vm.setMaxRecursionDepth(2);
      vm.pushFrame({ depth: 1 });
      vm.pushFrame({ depth: 2 });
      expect(() => vm.pushFrame({ depth: 3 })).toThrow(MaxRecursionError);

      // Now increase the limit
      vm.setMaxRecursionDepth(5);
      vm.pushFrame({ depth: 3 }); // should succeed now
      expect(vm.callStack).toHaveLength(3);
    });
  });

  // -----------------------------------------------------------------------
  // Program Counter
  // -----------------------------------------------------------------------

  describe("program counter", () => {
    it("should advance PC by 1 with advancePc()", () => {
      /**
       * advancePc() is the normal flow — move to the next instruction.
       */
      const vm = new GenericVM();

      expect(vm.pc).toBe(0);
      vm.advancePc();
      expect(vm.pc).toBe(1);
      vm.advancePc();
      expect(vm.pc).toBe(2);
    });

    it("should jump to target with jumpTo()", () => {
      /**
       * jumpTo() sets the PC to an arbitrary target — used by
       * JUMP, JUMP_IF_TRUE, and other control flow opcodes.
       */
      const vm = new GenericVM();

      vm.jumpTo(10);
      expect(vm.pc).toBe(10);

      vm.jumpTo(0);
      expect(vm.pc).toBe(0);
    });

    it("should support forward jumps in a program", () => {
      /**
       * A JUMP instruction that skips over other instructions.
       *
       * Program:
       *   0: LOAD_CONST 0   (push 10)
       *   1: JUMP 3          (skip instruction 2)
       *   2: LOAD_CONST 1   (push 99 — SHOULD BE SKIPPED)
       *   3: HALT
       *
       * Expected: stack = [10], NOT [10, 99]
       */
      const [vm] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.JUMP, operand: 3 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.HALT },
        ],
        [10, 99],
      );

      expect(vm.stack).toEqual([10]);
    });
  });

  // -----------------------------------------------------------------------
  // Built-in Functions
  // -----------------------------------------------------------------------

  describe("built-in functions", () => {
    it("should register and retrieve built-in functions", () => {
      /**
       * registerBuiltin() stores a function; getBuiltin() retrieves it.
       */
      const vm = new GenericVM();

      const impl = (...args: (number | string | CodeObject | null)[]) =>
        (args[0] as string).length;
      vm.registerBuiltin("len", impl);

      const builtin = vm.getBuiltin("len");
      expect(builtin).toBeDefined();
      expect(builtin!.name).toBe("len");
      expect(builtin!.implementation("hello")).toBe(5);
    });

    it("should return undefined for unregistered builtins", () => {
      /**
       * getBuiltin() returns undefined if the name hasn't been registered.
       */
      const vm = new GenericVM();

      expect(vm.getBuiltin("nonexistent")).toBeUndefined();
    });

    it("should allow overwriting a builtin", () => {
      /**
       * Registering a builtin with the same name replaces the previous one.
       */
      const vm = new GenericVM();

      vm.registerBuiltin("greet", () => "hello");
      expect(vm.getBuiltin("greet")!.implementation()).toBe("hello");

      vm.registerBuiltin("greet", () => "bonjour");
      expect(vm.getBuiltin("greet")!.implementation()).toBe("bonjour");
    });

    it("should work with opcode handlers that call builtins", () => {
      /**
       * End-to-end test: register a builtin, register an opcode that
       * calls it, and execute a program that triggers the call.
       *
       * We register a CALL_BUILTIN opcode (0xB0) that looks up a
       * builtin by name and invokes it with the top-of-stack argument.
       */
      const vm = new GenericVM();

      // Register handlers
      vm.registerOpcode(TestOp.LOAD_CONST, makeLoadConstHandler());
      vm.registerOpcode(TestOp.HALT, makeHaltHandler());

      // Custom CALL_BUILTIN opcode
      const CALL_BUILTIN = 0xb0;
      vm.registerOpcode(CALL_BUILTIN, (v, instr, code) => {
        const nameIndex = instr.operand as number;
        const name = code.names[nameIndex];
        const builtin = v.getBuiltin(name);
        if (!builtin) {
          throw new VMError(`Unknown builtin: ${name}`);
        }
        const arg = v.pop();
        const result = builtin.implementation(arg);
        v.push(result);
        v.advancePc();
        return `Called ${name}(${arg}) = ${result}`;
      });

      // Register a "double" builtin
      vm.registerBuiltin("double", (x) => (x as number) * 2);

      const code = makeCode(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: CALL_BUILTIN, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [21],
        ["double"],
      );

      const traces = vm.execute(code);
      expect(vm.stack).toEqual([42]);
      expect(traces).toHaveLength(3);
    });
  });

  // -----------------------------------------------------------------------
  // Configuration
  // -----------------------------------------------------------------------

  describe("configuration", () => {
    it("should start unfrozen by default", () => {
      const vm = new GenericVM();
      expect(vm.isFrozen()).toBe(false);
    });

    it("should freeze and unfreeze", () => {
      /**
       * setFrozen() controls whether the VM will execute.
       */
      const vm = new GenericVM();

      vm.setFrozen(true);
      expect(vm.isFrozen()).toBe(true);

      vm.setFrozen(false);
      expect(vm.isFrozen()).toBe(false);
    });

    it("should return empty traces when frozen", () => {
      /**
       * A frozen VM's execute() returns immediately with no traces.
       * No instructions are executed, and the VM state is unchanged.
       */
      const vm = createTestVM();
      vm.setFrozen(true);

      const code = makeCode(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [42],
      );

      const traces = vm.execute(code);
      expect(traces).toEqual([]);
      expect(vm.stack).toEqual([]); // nothing was executed
      expect(vm.pc).toBe(0); // PC didn't move
    });

    it("should start with null max recursion depth", () => {
      const vm = new GenericVM();
      expect(vm.getMaxRecursionDepth()).toBeNull();
    });

    it("should get and set max recursion depth", () => {
      const vm = new GenericVM();

      vm.setMaxRecursionDepth(500);
      expect(vm.getMaxRecursionDepth()).toBe(500);

      vm.setMaxRecursionDepth(null);
      expect(vm.getMaxRecursionDepth()).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // Reset
  // -----------------------------------------------------------------------

  describe("reset", () => {
    it("should clear all execution state", () => {
      /**
       * After running a program, reset() should return the VM to
       * a clean state — empty stack, no variables, PC at 0, etc.
       */
      const [vm] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.STORE_NAME, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.PRINT },
          { opcode: TestOp.HALT },
        ],
        [42, "hello"],
        ["x"],
      );

      // Verify state is dirty
      expect(vm.variables).toEqual({ x: 42 });
      expect(vm.output).toEqual(["hello"]);
      expect(vm.halted).toBe(true);

      // Reset
      vm.reset();

      // Verify state is clean
      expect(vm.stack).toEqual([]);
      expect(vm.variables).toEqual({});
      expect(vm.locals).toEqual([]);
      expect(vm.pc).toBe(0);
      expect(vm.halted).toBe(false);
      expect(vm.output).toEqual([]);
      expect(vm.callStack).toEqual([]);
    });

    it("should preserve registered handlers after reset", () => {
      /**
       * reset() clears execution state but keeps the opcode handlers.
       * This lets you reuse the same VM configuration for multiple runs.
       *
       * Analogy: Rebooting a computer clears RAM but keeps the OS
       * installed on disk.
       */
      const vm = createTestVM();

      // Run a program
      const code1 = makeCode(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [42],
      );
      vm.execute(code1);
      expect(vm.stack).toEqual([42]);

      // Reset
      vm.reset();

      // Run another program — handlers should still work
      const code2 = makeCode(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          { opcode: TestOp.HALT },
        ],
        [100, 200],
      );
      const traces = vm.execute(code2);
      expect(vm.stack).toEqual([300]);
      expect(traces).toHaveLength(4);
    });

    it("should preserve builtins after reset", () => {
      /**
       * Built-in functions survive a reset, just like handlers.
       */
      const vm = new GenericVM();
      vm.registerBuiltin("double", (x) => (x as number) * 2);

      vm.reset();

      const builtin = vm.getBuiltin("double");
      expect(builtin).toBeDefined();
      expect(builtin!.implementation(21)).toBe(42);
    });
  });

  // -----------------------------------------------------------------------
  // Error Handling
  // -----------------------------------------------------------------------

  describe("error handling", () => {
    it("should throw InvalidOpcodeError for unknown opcodes", () => {
      /**
       * If the VM encounters an opcode with no registered handler,
       * it throws InvalidOpcodeError. This catches corrupted bytecode
       * or compiler bugs that emit invalid opcodes.
       */
      const vm = createTestVM();

      const code = makeCode([
        { opcode: 0xde }, // not registered
      ]);

      expect(() => vm.execute(code)).toThrow(InvalidOpcodeError);
      expect(() => {
        vm.reset();
        vm.execute(code);
      }).toThrow("No handler registered for opcode 0xde");
    });

    it("should throw InvalidOpcodeError when no handlers are registered", () => {
      /**
       * A brand-new GenericVM has no handlers at all. Any instruction
       * will fail with InvalidOpcodeError.
       */
      const vm = new GenericVM();

      const code = makeCode([{ opcode: 0x01 }]);

      expect(() => vm.execute(code)).toThrow(InvalidOpcodeError);
    });

    it("should propagate errors from handlers", () => {
      /**
       * If a handler throws an error, it should propagate out of
       * execute() unchanged. The VM doesn't catch handler errors.
       */
      const vm = new GenericVM();
      vm.registerOpcode(0x01, () => {
        throw new VMTypeError("cannot add string and number");
      });

      const code = makeCode([{ opcode: 0x01 }]);

      expect(() => vm.execute(code)).toThrow(VMTypeError);
      expect(() => {
        vm.reset();
        vm.execute(code);
      }).toThrow("cannot add string and number");
    });

    it("should throw StackUnderflowError from ADD on empty stack", () => {
      /**
       * ADD pops two values. If the stack is empty (or has only one
       * value), the pop() inside the ADD handler throws
       * StackUnderflowError.
       */
      const vm = createTestVM();

      const code = makeCode([{ opcode: TestOp.ADD }]);

      expect(() => vm.execute(code)).toThrow(StackUnderflowError);
    });
  });

  // -----------------------------------------------------------------------
  // Step-by-Step Execution
  // -----------------------------------------------------------------------

  describe("step-by-step execution", () => {
    it("should execute one instruction per step() call", () => {
      /**
       * step() executes exactly one instruction and returns its trace.
       * This is the foundation for debuggers — they call step()
       * repeatedly, inspecting state between each call.
       */
      const vm = createTestVM();

      const code = makeCode(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          { opcode: TestOp.HALT },
        ],
        [10, 20],
      );

      // Step 1: LOAD_CONST 10
      const trace1 = vm.step(code);
      expect(trace1.pc).toBe(0);
      expect(vm.stack).toEqual([10]);
      expect(vm.halted).toBe(false);

      // Step 2: LOAD_CONST 20
      const trace2 = vm.step(code);
      expect(trace2.pc).toBe(1);
      expect(vm.stack).toEqual([10, 20]);

      // Step 3: ADD
      const trace3 = vm.step(code);
      expect(trace3.pc).toBe(2);
      expect(vm.stack).toEqual([30]);

      // Step 4: HALT
      const trace4 = vm.step(code);
      expect(trace4.pc).toBe(3);
      expect(vm.halted).toBe(true);
    });

    it("should return a trace with before/after stack snapshots", () => {
      /**
       * Each trace from step() captures the stack state transformation.
       */
      const vm = createTestVM();

      const code = makeCode(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
        ],
        [5, 7],
      );

      // Step through LOAD_CONST 5
      vm.step(code);

      // Step through LOAD_CONST 7
      vm.step(code);

      // Step through ADD — this is the interesting one
      const trace = vm.step(code);
      expect(trace.stackBefore).toEqual([5, 7]);
      expect(trace.stackAfter).toEqual([12]);
    });
  });

  // -----------------------------------------------------------------------
  // Program Ends Without Halt
  // -----------------------------------------------------------------------

  describe("program ends without halt", () => {
    it("should stop when PC goes past the last instruction", () => {
      /**
       * Not every program needs an explicit HALT. If the PC advances
       * past the last instruction, the eval loop naturally stops.
       *
       * This is like reaching the end of main() in C — the program
       * is done even without an explicit ``exit(0)``.
       */
      const vm = createTestVM();

      const code = makeCode(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          // No HALT — PC will advance past the end
        ],
        [3, 4],
      );

      const traces = vm.execute(code);

      // All 3 instructions executed
      expect(traces).toHaveLength(3);

      // Result is on the stack
      expect(vm.stack).toEqual([7]);

      // VM is NOT halted — it just ran out of instructions
      expect(vm.halted).toBe(false);
    });

    it("should return empty traces for an empty program", () => {
      /**
       * A program with zero instructions produces zero traces.
       * The eval loop condition (pc < instructions.length) is false
       * immediately.
       */
      const vm = createTestVM();
      const code = makeCode([]);

      const traces = vm.execute(code);

      expect(traces).toEqual([]);
      expect(vm.pc).toBe(0);
    });
  });

  // -----------------------------------------------------------------------
  // Opcode Registration
  // -----------------------------------------------------------------------

  describe("opcode registration", () => {
    it("should allow overwriting an opcode handler", () => {
      /**
       * Registering a handler for an opcode that already has one
       * replaces the old handler. This is useful for languages that
       * want to specialize or override default behavior.
       */
      const vm = new GenericVM();

      // First handler: pushes 1
      vm.registerOpcode(0x01, (v) => {
        v.push(1);
        v.advancePc();
        return null;
      });

      // Overwrite: pushes 999
      vm.registerOpcode(0x01, (v) => {
        v.push(999);
        v.advancePc();
        return null;
      });

      const code = makeCode([{ opcode: 0x01 }]);
      vm.execute(code);

      expect(vm.stack).toEqual([999]);
    });

    it("should support handlers that return null description", () => {
      /**
       * When a handler returns null, the trace should get a default
       * description instead of null.
       */
      const vm = new GenericVM();

      vm.registerOpcode(0x01, (v) => {
        v.push(42);
        v.advancePc();
        return null; // no custom description
      });

      const code = makeCode([{ opcode: 0x01 }]);
      const traces = vm.execute(code);

      // Should have a default description, not null
      expect(traces[0].description).toBe("Executed opcode 0x01");
    });
  });

  // -----------------------------------------------------------------------
  // Error Classes
  // -----------------------------------------------------------------------

  describe("error classes", () => {
    it("MaxRecursionError should be an instance of VMError", () => {
      /**
       * MaxRecursionError inherits from VMError, so it can be caught
       * by generic VMError handlers.
       */
      const error = new MaxRecursionError();
      expect(error).toBeInstanceOf(VMError);
      expect(error).toBeInstanceOf(MaxRecursionError);
      expect(error.name).toBe("MaxRecursionError");
      expect(error.message).toBe("Maximum recursion depth exceeded");
    });

    it("MaxRecursionError should accept a custom message", () => {
      const error = new MaxRecursionError("depth 42 exceeded");
      expect(error.message).toBe("depth 42 exceeded");
    });

    it("VMTypeError should be an instance of VMError", () => {
      const error = new VMTypeError();
      expect(error).toBeInstanceOf(VMError);
      expect(error).toBeInstanceOf(VMTypeError);
      expect(error.name).toBe("VMTypeError");
      expect(error.message).toBe("Type error");
    });

    it("VMTypeError should accept a custom message", () => {
      const error = new VMTypeError("cannot add string and number");
      expect(error.message).toBe("cannot add string and number");
    });
  });

  // -----------------------------------------------------------------------
  // Variables and Locals
  // -----------------------------------------------------------------------

  describe("variables and locals", () => {
    it("should store and load named variables", () => {
      /**
       * STORE_NAME + LOAD_NAME roundtrip: store a value, load it back.
       */
      const [vm] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.STORE_NAME, operand: 0 },
          { opcode: TestOp.LOAD_NAME, operand: 0 },
          { opcode: TestOp.HALT },
        ],
        [42],
        ["x"],
      );

      expect(vm.stack).toEqual([42]);
      expect(vm.variables).toEqual({ x: 42 });
    });

    it("should allow direct manipulation of locals", () => {
      /**
       * The locals array is public, so handlers can read/write it
       * directly using index-based access — faster than named variables.
       */
      const vm = new GenericVM();

      vm.locals = [10, 20, 30];
      expect(vm.locals[1]).toBe(20);

      vm.locals[1] = 99;
      expect(vm.locals).toEqual([10, 99, 30]);
    });
  });

  // -----------------------------------------------------------------------
  // Complex Programs
  // -----------------------------------------------------------------------

  describe("complex programs", () => {
    it("should compute (10 + 20) - 5 = 25", () => {
      /**
       * A multi-step arithmetic program that uses both ADD and SUB.
       *
       * Stack transitions:
       *   []           → LOAD 10  → [10]
       *   [10]         → LOAD 20  → [10, 20]
       *   [10, 20]     → ADD      → [30]
       *   [30]         → LOAD 5   → [30, 5]
       *   [30, 5]      → SUB      → [25]
       *   [25]         → HALT     → [25]
       */
      const [vm] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          { opcode: TestOp.LOAD_CONST, operand: 2 },
          { opcode: TestOp.SUB },
          { opcode: TestOp.HALT },
        ],
        [10, 20, 5],
      );

      expect(vm.stack).toEqual([25]);
    });

    it("should store a result and print it", () => {
      /**
       * Compute 3 + 4, store in x, load x, print it.
       *
       * This exercises the full pipeline: arithmetic → variable
       * storage → variable retrieval → output.
       */
      const [vm] = run(
        [
          { opcode: TestOp.LOAD_CONST, operand: 0 },
          { opcode: TestOp.LOAD_CONST, operand: 1 },
          { opcode: TestOp.ADD },
          { opcode: TestOp.STORE_NAME, operand: 0 },
          { opcode: TestOp.LOAD_NAME, operand: 0 },
          { opcode: TestOp.PRINT },
          { opcode: TestOp.HALT },
        ],
        [3, 4],
        ["result"],
      );

      expect(vm.variables).toEqual({ result: 7 });
      expect(vm.output).toEqual(["7"]);
      expect(vm.stack).toEqual([]);
    });
  });

  describe("global injection", () => {
    it("should merge injected globals into existing variables", () => {
      const vm = new GenericVM();
      vm.variables = { existing: 1, ctx_os: "linux" };

      vm.injectGlobals({ ctx_os: "darwin", answer: 42 });

      expect(vm.variables).toEqual({
        existing: 1,
        ctx_os: "darwin",
        answer: 42,
      });
    });
  });

  // =====================================================================
  // Typed Stack Operations
  // =====================================================================

  describe("typed stack operations", () => {
    it("should push and pop typed values", () => {
      const vm = new GenericVM();

      vm.pushTyped({ type: 0x7F, value: 42 });
      vm.pushTyped({ type: 0x7E, value: 7n });

      const top = vm.popTyped();
      expect(top).toEqual({ type: 0x7E, value: 7n });

      const second = vm.popTyped();
      expect(second).toEqual({ type: 0x7F, value: 42 });
    });

    it("should peek typed values without consuming them", () => {
      const vm = new GenericVM();
      vm.pushTyped({ type: 0x7D, value: 3.14 });

      const peeked = vm.peekTyped();
      expect(peeked).toEqual({ type: 0x7D, value: 3.14 });
      expect(vm.typedStack.length).toBe(1);
    });

    it("should throw on pop from empty typed stack", () => {
      const vm = new GenericVM();
      expect(() => vm.popTyped()).toThrow(StackUnderflowError);
    });

    it("should throw on peek of empty typed stack", () => {
      const vm = new GenericVM();
      expect(() => vm.peekTyped()).toThrow(StackUnderflowError);
    });

    it("should support BigInt values on typed stack", () => {
      const vm = new GenericVM();
      const big = BigInt("9223372036854775807"); // i64 max
      vm.pushTyped({ type: 0x7E, value: big });
      const result = vm.popTyped();
      expect(result.value).toBe(big);
      expect(typeof result.value).toBe("bigint");
    });

    it("should be independent from untyped stack", () => {
      const vm = new GenericVM();
      vm.push(100);
      vm.pushTyped({ type: 0x7F, value: 200 });

      expect(vm.stack.length).toBe(1);
      expect(vm.typedStack.length).toBe(1);

      expect(vm.pop()).toBe(100);
      expect(vm.popTyped()).toEqual({ type: 0x7F, value: 200 });
    });

    it("should be cleared by reset", () => {
      const vm = new GenericVM();
      vm.pushTyped({ type: 0x7F, value: 42 });
      vm.reset();
      expect(vm.typedStack.length).toBe(0);
    });
  });

  // =====================================================================
  // BigInt Support in VMValue
  // =====================================================================

  describe("bigint support", () => {
    it("should support bigint values on the untyped stack", () => {
      const vm = new GenericVM();
      vm.push(42n);
      expect(vm.pop()).toBe(42n);
    });

    it("should support bigint in variables", () => {
      const vm = new GenericVM();
      vm.variables["big"] = 9999999999999999n;
      expect(vm.variables["big"]).toBe(9999999999999999n);
    });
  });

  // =====================================================================
  // Pre/Post Instruction Hooks
  // =====================================================================

  describe("instruction hooks", () => {
    it("pre-hook should transform instructions before dispatch", () => {
      const vm = new GenericVM();
      const transformed: number[] = [];

      // Register a handler for opcode 0x01
      vm.registerOpcode(0x01, (vm, instr) => {
        // The handler should see the TRANSFORMED instruction
        transformed.push(instr.operand as number);
        vm.advancePc();
        return null;
      });

      // Pre-hook doubles the operand
      vm.setPreInstructionHook((_vm, instruction, _code) => {
        return {
          opcode: instruction.opcode,
          operand: ((instruction.operand as number) ?? 0) * 2,
        };
      });

      const code: CodeObject = {
        instructions: [
          { opcode: 0x01, operand: 5 },
          { opcode: 0x01, operand: 10 },
        ],
        constants: [],
        names: [],
      };

      // Register a halt to stop after 2 instructions
      vm.execute(code);

      expect(transformed).toEqual([10, 20]); // 5*2, 10*2
    });

    it("post-hook should run after each instruction", () => {
      const vm = new GenericVM();
      const postLog: number[] = [];

      vm.registerOpcode(0x01, (vm) => {
        vm.push(1);
        vm.advancePc();
        return null;
      });

      vm.setPostInstructionHook((vm, _instruction, _code) => {
        postLog.push(vm.pc);
      });

      const code: CodeObject = {
        instructions: [{ opcode: 0x01 }, { opcode: 0x01 }],
        constants: [],
        names: [],
      };

      vm.execute(code);
      // After each handler calls advancePc, post-hook sees the new PC
      expect(postLog).toEqual([1, 2]);
    });

    it("should allow removing hooks with null", () => {
      const vm = new GenericVM();
      let hookCalled = false;

      vm.setPreInstructionHook(() => {
        hookCalled = true;
        return { opcode: 0x01 };
      });
      vm.setPreInstructionHook(null);

      vm.registerOpcode(0x01, (vm) => {
        vm.advancePc();
        return null;
      });

      vm.execute({
        instructions: [{ opcode: 0x01 }],
        constants: [],
        names: [],
      });

      expect(hookCalled).toBe(false);
    });
  });

  // =====================================================================
  // Context-Aware Execution
  // =====================================================================

  describe("context-aware execution", () => {
    it("should pass context to context-aware handlers", () => {
      const vm = new GenericVM();
      let receivedContext: unknown = null;

      vm.registerContextOpcode(0x01, (_vm, _instr, _code, ctx) => {
        receivedContext = ctx;
        _vm.advancePc();
        return null;
      });

      const code: CodeObject = {
        instructions: [{ opcode: 0x01 }],
        constants: [],
        names: [],
      };

      const myContext = { memory: [1, 2, 3], label: "test" };
      vm.executeWithContext(code, myContext);

      expect(receivedContext).toBe(myContext);
    });

    it("should prefer context handlers over regular handlers during context execution", () => {
      const vm = new GenericVM();
      let which = "";

      vm.registerOpcode(0x01, (vm) => {
        which = "regular";
        vm.advancePc();
        return null;
      });

      vm.registerContextOpcode(0x01, (vm) => {
        which = "context";
        vm.advancePc();
        return null;
      });

      const code: CodeObject = {
        instructions: [{ opcode: 0x01 }],
        constants: [],
        names: [],
      };

      // With context: should use context handler
      vm.executeWithContext(code, { data: true });
      expect(which).toBe("context");

      // Without context: should use regular handler
      vm.reset();
      vm.registerOpcode(0x01, (vm) => {
        which = "regular-again";
        vm.advancePc();
        return null;
      });
      vm.execute(code);
      expect(which).toBe("regular-again");
    });

    it("should restore previous context after execution", () => {
      const vm = new GenericVM();

      vm.registerContextOpcode(0x01, (vm) => {
        vm.advancePc();
        return null;
      });

      const code: CodeObject = {
        instructions: [{ opcode: 0x01 }],
        constants: [],
        names: [],
      };

      vm.executeWithContext(code, { outer: true });
      expect(vm.executionContext).toBeNull();
    });

    it("should clear execution context on reset", () => {
      const vm = new GenericVM();
      vm.executionContext = { test: true };
      vm.reset();
      expect(vm.executionContext).toBeNull();
    });
  });
});
