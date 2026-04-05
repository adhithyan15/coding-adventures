/**
 * Tests for variable access instruction handlers.
 */

import { describe, it, expect } from "vitest";
import { GenericVM } from "@coding-adventures/virtual-machine";
import type { CodeObject } from "@coding-adventures/virtual-machine";
import { makeVm, makeContext, i32, i64, f64 } from "./helpers.js";
import { registerVariable } from "../src/instructions/variable.js";
import type { WasmExecutionContext } from "../src/types.js";

describe("variable", () => {
  const vm = makeVm(registerVariable);

  function run(instrs: { opcode: number; operand?: unknown }[], ctx: WasmExecutionContext) {
    const code: CodeObject = {
      instructions: instrs.map((i) => ({
        opcode: i.opcode,
        operand: i.operand as number | string | null | undefined,
      })),
      constants: [],
      names: [],
    };
    vm.reset();
    vm.executeWithContext(code, ctx);
  }

  describe("local.get (0x20)", () => {
    it("pushes a local variable value", () => {
      const ctx = makeContext({ typedLocals: [i32(42), i64(100n)] });
      run([{ opcode: 0x20, operand: 0 }], ctx);
      expect(vm.peekTyped().value).toBe(42);
    });

    it("reads the correct index", () => {
      const ctx = makeContext({ typedLocals: [i32(10), i32(20), i32(30)] });
      run([{ opcode: 0x20, operand: 2 }], ctx);
      expect(vm.peekTyped().value).toBe(30);
    });
  });

  describe("local.set (0x21)", () => {
    it("pops and stores to local", () => {
      const ctx = makeContext({ typedLocals: [i32(0)] });
      vm.reset();
      vm.pushTyped(i32(99));
      const code: CodeObject = {
        instructions: [{ opcode: 0x21, operand: 0 }],
        constants: [],
        names: [],
      };
      vm.executeWithContext(code, ctx);
      expect(ctx.typedLocals[0].value).toBe(99);
    });
  });

  describe("local.tee (0x22)", () => {
    it("stores to local WITHOUT popping from stack", () => {
      const ctx = makeContext({ typedLocals: [i32(0)] });
      vm.reset();
      vm.pushTyped(i32(77));
      const code: CodeObject = {
        instructions: [{ opcode: 0x22, operand: 0 }],
        constants: [],
        names: [],
      };
      vm.executeWithContext(code, ctx);
      /* Local should be updated */
      expect(ctx.typedLocals[0].value).toBe(77);
      /* Value should still be on the stack */
      expect(vm.peekTyped().value).toBe(77);
    });
  });

  describe("global.get (0x23)", () => {
    it("pushes a global variable value", () => {
      const ctx = makeContext({ globals: [f64(3.14), i32(7)] });
      run([{ opcode: 0x23, operand: 1 }], ctx);
      expect(vm.peekTyped().value).toBe(7);
    });
  });

  describe("global.set (0x24)", () => {
    it("pops and stores to global", () => {
      const ctx = makeContext({ globals: [i32(0)] });
      vm.reset();
      vm.pushTyped(i32(55));
      const code: CodeObject = {
        instructions: [{ opcode: 0x24, operand: 0 }],
        constants: [],
        names: [],
      };
      vm.executeWithContext(code, ctx);
      expect(ctx.globals[0].value).toBe(55);
    });
  });
});
