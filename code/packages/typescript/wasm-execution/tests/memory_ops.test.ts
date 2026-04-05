/**
 * Tests for WASM memory instruction handlers.
 *
 * These tests use the helpers pattern to test individual memory instructions
 * with a real LinearMemory instance. Because runInstructions() calls vm.reset()
 * internally, we include i32.const/i64.const/f32.const/f64.const instructions
 * in the instruction sequence to push values onto the stack.
 */

import { describe, it, expect } from "vitest";
import {
  makeVm,
  runInstructions,
  makeContext,
  i32,
  i64,
  f32,
  f64,
  LinearMemory,
} from "./helpers.js";
import { registerMemory } from "../src/instructions/memory.js";
import { registerNumericI32 } from "../src/instructions/numeric_i32.js";
import { registerNumericI64 } from "../src/instructions/numeric_i64.js";
import { registerNumericF32 } from "../src/instructions/numeric_f32.js";
import { registerNumericF64 } from "../src/instructions/numeric_f64.js";
import { TrapError } from "../src/host_interface.js";

/**
 * Create a VM with all numeric + memory handlers registered.
 */
function makeMemVm() {
  return makeVm((vm) => {
    registerNumericI32(vm);
    registerNumericI64(vm);
    registerNumericF32(vm);
    registerNumericF64(vm);
    registerMemory(vm);
  });
}

describe("memory instructions", () => {
  const vm = makeMemVm();

  /** Create a context with a 1-page linear memory. */
  function memCtx() {
    return makeContext({ memory: new LinearMemory(1) });
  }

  describe("i32.store + i32.load roundtrip", () => {
    it("stores and loads an i32 value", () => {
      const ctx = memCtx();

      // Store: i32.const 0 (addr), i32.const 42 (value), i32.store
      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },    // i32.const 0 (address)
        { opcode: 0x41, operand: 42 },   // i32.const 42 (value)
        { opcode: 0x36, operand: { align: 2, offset: 0 } },  // i32.store
      ], ctx);

      // Load: i32.const 0 (addr), i32.load
      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },    // i32.const 0 (address)
        { opcode: 0x28, operand: { align: 2, offset: 0 } },  // i32.load
      ], ctx);

      expect(vm.peekTyped().value).toBe(42);
      expect(vm.peekTyped().type).toBe(0x7f); // i32
    });

    it("works with nonzero offset in memarg", () => {
      const ctx = memCtx();

      // Store at effective address = base(0) + offset(8) = 8
      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 123 },
        { opcode: 0x36, operand: { align: 2, offset: 8 } },
      ], ctx);

      // Load from effective address 8
      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x28, operand: { align: 2, offset: 8 } },
      ], ctx);

      expect(vm.peekTyped().value).toBe(123);
    });
  });

  describe("i64.store + i64.load roundtrip", () => {
    it("stores and loads an i64 value", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },          // i32.const 0 (address)
        { opcode: 0x42, operand: 9223372036854775807n }, // i64.const MAX_I64
        { opcode: 0x37, operand: { align: 3, offset: 0 } },  // i64.store
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x29, operand: { align: 3, offset: 0 } },  // i64.load
      ], ctx);

      expect(vm.peekTyped().value).toBe(9223372036854775807n);
      expect(vm.peekTyped().type).toBe(0x7e); // i64
    });
  });

  describe("f32.store + f32.load roundtrip", () => {
    it("stores and loads an f32 value", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },         // i32.const 0 (address)
        { opcode: 0x43, operand: 3.14 },       // f32.const 3.14
        { opcode: 0x38, operand: { align: 2, offset: 0 } },  // f32.store
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2a, operand: { align: 2, offset: 0 } },  // f32.load
      ], ctx);

      expect(vm.peekTyped().value).toBeCloseTo(3.14, 2);
      expect(vm.peekTyped().type).toBe(0x7d); // f32
    });
  });

  describe("f64.store + f64.load roundtrip", () => {
    it("stores and loads an f64 value", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },          // i32.const 0 (address)
        { opcode: 0x44, operand: Math.PI },     // f64.const pi
        { opcode: 0x39, operand: { align: 3, offset: 0 } },  // f64.store
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2b, operand: { align: 3, offset: 0 } },  // f64.load
      ], ctx);

      expect(vm.peekTyped().value).toBe(Math.PI);
      expect(vm.peekTyped().type).toBe(0x7c); // f64
    });
  });

  describe("partial-width: i32.store8 + i32.load8_s (sign extension)", () => {
    it("stores low byte and sign-extends on load", () => {
      const ctx = memCtx();

      // Store 0xFF (255 unsigned, -1 signed) as a single byte
      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xff },
        { opcode: 0x3a, operand: { align: 0, offset: 0 } },  // i32.store8
      ], ctx);

      // Load it back with sign extension
      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2c, operand: { align: 0, offset: 0 } },  // i32.load8_s
      ], ctx);

      // 0xFF sign-extended to i32 = -1
      expect(vm.peekTyped().value).toBe(-1);
    });

    it("stores low byte and zero-extends on load", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xff },
        { opcode: 0x3a, operand: { align: 0, offset: 0 } },  // i32.store8
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2d, operand: { align: 0, offset: 0 } },  // i32.load8_u
      ], ctx);

      // 0xFF zero-extended = 255
      expect(vm.peekTyped().value).toBe(255);
    });
  });

  describe("memory.size (0x3F)", () => {
    it("returns current page count", () => {
      const ctx = memCtx(); // 1 page

      runInstructions(vm, [
        { opcode: 0x3f, operand: 0 },  // memory.size (memidx 0)
      ], ctx);

      expect(vm.peekTyped().value).toBe(1);
      expect(vm.peekTyped().type).toBe(0x7f); // i32
    });
  });

  describe("memory.grow (0x40)", () => {
    it("grows memory and returns previous size", () => {
      const ctx = makeContext({ memory: new LinearMemory(1, 10) });

      // Grow by 2 pages
      runInstructions(vm, [
        { opcode: 0x41, operand: 2 },   // i32.const 2
        { opcode: 0x40, operand: 0 },   // memory.grow
      ], ctx);

      // Should return previous page count (1)
      expect(vm.peekTyped().value).toBe(1);

      // Verify size is now 3
      runInstructions(vm, [
        { opcode: 0x3f, operand: 0 },
      ], ctx);
      expect(vm.peekTyped().value).toBe(3);
    });

    it("returns -1 when growth would exceed maximum", () => {
      const ctx = makeContext({ memory: new LinearMemory(1, 2) });

      // Try to grow by 5 pages (exceeds max of 2)
      runInstructions(vm, [
        { opcode: 0x41, operand: 5 },
        { opcode: 0x40, operand: 0 },
      ], ctx);

      expect(vm.peekTyped().value).toBe(-1);
    });
  });

  describe("i32.store16 + i32.load16_s roundtrip", () => {
    it("stores low 16 bits and sign-extends on load", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xFFFF },  // 65535 = -1 as signed i16
        { opcode: 0x3b, operand: { align: 1, offset: 0 } },  // i32.store16
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2e, operand: { align: 1, offset: 0 } },  // i32.load16_s
      ], ctx);

      expect(vm.peekTyped().value).toBe(-1);
    });
  });

  describe("i32.load16_u", () => {
    it("zero-extends 16-bit value", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xFFFF },
        { opcode: 0x3b, operand: { align: 1, offset: 0 } },  // i32.store16
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2f, operand: { align: 1, offset: 0 } },  // i32.load16_u
      ], ctx);

      expect(vm.peekTyped().value).toBe(65535);
    });
  });

  describe("i64 narrow loads", () => {
    it("i64.load8_s sign-extends byte to i64", () => {
      const ctx = memCtx();

      // Store 0xFF as a byte
      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xFF },
        { opcode: 0x3a, operand: { align: 0, offset: 0 } },  // i32.store8
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x30, operand: { align: 0, offset: 0 } },  // i64.load8_s
      ], ctx);

      expect(vm.peekTyped().value).toBe(-1n);
      expect(vm.peekTyped().type).toBe(0x7e);
    });

    it("i64.load8_u zero-extends byte to i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xFF },
        { opcode: 0x3a, operand: { align: 0, offset: 0 } },
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x31, operand: { align: 0, offset: 0 } },  // i64.load8_u
      ], ctx);

      expect(vm.peekTyped().value).toBe(255n);
    });

    it("i64.load16_s sign-extends 16-bit to i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xFFFF },
        { opcode: 0x3b, operand: { align: 1, offset: 0 } },
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x32, operand: { align: 1, offset: 0 } },  // i64.load16_s
      ], ctx);

      expect(vm.peekTyped().value).toBe(-1n);
    });

    it("i64.load16_u zero-extends 16-bit to i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: 0xFFFF },
        { opcode: 0x3b, operand: { align: 1, offset: 0 } },
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x33, operand: { align: 1, offset: 0 } },  // i64.load16_u
      ], ctx);

      expect(vm.peekTyped().value).toBe(65535n);
    });

    it("i64.load32_s sign-extends 32-bit to i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: -1 },  // 0xFFFFFFFF
        { opcode: 0x36, operand: { align: 2, offset: 0 } },
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x34, operand: { align: 2, offset: 0 } },  // i64.load32_s
      ], ctx);

      expect(vm.peekTyped().value).toBe(-1n);
    });

    it("i64.load32_u zero-extends 32-bit to i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x41, operand: -1 },
        { opcode: 0x36, operand: { align: 2, offset: 0 } },
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x35, operand: { align: 2, offset: 0 } },  // i64.load32_u
      ], ctx);

      expect(vm.peekTyped().value).toBe(4294967295n);
    });
  });

  describe("i64 narrow stores", () => {
    it("i64.store8 stores low byte of i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x42, operand: 0x1FFn },  // i64.const 511 (low byte = 0xFF)
        { opcode: 0x3c, operand: { align: 0, offset: 0 } },  // i64.store8
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2d, operand: { align: 0, offset: 0 } },  // i32.load8_u
      ], ctx);

      expect(vm.peekTyped().value).toBe(0xFF);
    });

    it("i64.store16 stores low 16 bits of i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x42, operand: 0x1FFFFn },  // low 16 = 0xFFFF
        { opcode: 0x3d, operand: { align: 1, offset: 0 } },  // i64.store16
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x2f, operand: { align: 1, offset: 0 } },  // i32.load16_u
      ], ctx);

      expect(vm.peekTyped().value).toBe(0xFFFF);
    });

    it("i64.store32 stores low 32 bits of i64", () => {
      const ctx = memCtx();

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x42, operand: 0x1FFFFFFFFn },  // low 32 bits = 0xFFFFFFFF
        { opcode: 0x3e, operand: { align: 2, offset: 0 } },  // i64.store32
      ], ctx);

      runInstructions(vm, [
        { opcode: 0x41, operand: 0 },
        { opcode: 0x28, operand: { align: 2, offset: 0 } },  // i32.load
      ], ctx);

      expect(vm.peekTyped().value).toBe(-1);  // 0xFFFFFFFF as signed i32
    });
  });

  describe("no memory trap", () => {
    it("traps when accessing memory on a module with no memory", () => {
      const ctx = makeContext({ memory: null });

      expect(() => {
        runInstructions(vm, [
          { opcode: 0x41, operand: 0 },
          { opcode: 0x28, operand: { align: 2, offset: 0 } },
        ], ctx);
      }).toThrow(TrapError);
    });
  });
});
