/**
 * Tests for parametric instruction handlers (drop, select).
 */

import { describe, it, expect } from "vitest";
import { GenericVM } from "@coding-adventures/virtual-machine";
import type { CodeObject } from "@coding-adventures/virtual-machine";
import { makeVm, makeContext, i32, f64 } from "./helpers.js";
import { registerParametric } from "../src/instructions/parametric.js";

describe("parametric", () => {
  const vm = makeVm(registerParametric);

  describe("drop (0x1A)", () => {
    it("removes the top value from the stack", () => {
      vm.reset();
      vm.pushTyped(i32(10));
      vm.pushTyped(i32(20));

      const code: CodeObject = {
        instructions: [{ opcode: 0x1a }],
        constants: [],
        names: [],
      };
      vm.executeWithContext(code, makeContext());

      /* 20 was dropped, 10 should be on top */
      expect(vm.peekTyped().value).toBe(10);
    });
  });

  describe("select (0x1B)", () => {
    it("selects val1 when condition is non-zero", () => {
      vm.reset();
      vm.pushTyped(i32(10));   /* val1 */
      vm.pushTyped(i32(20));   /* val2 */
      vm.pushTyped(i32(1));    /* condition (non-zero) */

      const code: CodeObject = {
        instructions: [{ opcode: 0x1b }],
        constants: [],
        names: [],
      };
      vm.executeWithContext(code, makeContext());

      expect(vm.peekTyped().value).toBe(10);
    });

    it("selects val2 when condition is zero", () => {
      vm.reset();
      vm.pushTyped(i32(10));   /* val1 */
      vm.pushTyped(i32(20));   /* val2 */
      vm.pushTyped(i32(0));    /* condition (zero) */

      const code: CodeObject = {
        instructions: [{ opcode: 0x1b }],
        constants: [],
        names: [],
      };
      vm.executeWithContext(code, makeContext());

      expect(vm.peekTyped().value).toBe(20);
    });

    it("works with f64 values", () => {
      vm.reset();
      vm.pushTyped(f64(3.14)); /* val1 */
      vm.pushTyped(f64(2.72)); /* val2 */
      vm.pushTyped(i32(42));   /* condition (non-zero) */

      const code: CodeObject = {
        instructions: [{ opcode: 0x1b }],
        constants: [],
        names: [],
      };
      vm.executeWithContext(code, makeContext());

      expect(vm.peekTyped().value).toBe(3.14);
    });
  });
});
