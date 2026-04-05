/**
 * Tests for WASM control flow instructions via WasmExecutionEngine.
 *
 * These tests verify block, loop, if/else/end, br, br_if, and return
 * by constructing small hand-built WASM programs and executing them.
 */

import { describe, it, expect } from "vitest";
import { WasmExecutionEngine } from "../src/wasm_execution.js";
import { i32 } from "../src/values.js";
import type { FunctionBody } from "@coding-adventures/wasm-types";
import { ValueType, makeFuncType } from "@coding-adventures/wasm-types";
import { encodeSigned, encodeUnsigned } from "@coding-adventures/wasm-leb128";
import { TrapError } from "../src/host_interface.js";

/**
 * Build a FunctionBody from raw bytecodes.
 * The trailing 0x0B (end) for the function is appended automatically.
 */
function makeBody(locals: number[], ...bytecodes: number[]): FunctionBody {
  return {
    locals: locals.map(t => t as ValueType),
    code: new Uint8Array([...bytecodes, 0x0b]),
  };
}

/**
 * Create a single-function engine and call it with given args.
 */
function run(
  params: ValueType[],
  results: ValueType[],
  locals: number[],
  bytecodes: number[],
  args: { type: number; value: number | bigint }[] = [],
) {
  const funcType = makeFuncType(params, results);
  const body = makeBody(locals, ...bytecodes);

  const engine = new WasmExecutionEngine({
    memory: null,
    tables: [],
    globals: [],
    globalTypes: [],
    funcTypes: [funcType],
    funcBodies: [body],
    hostFunctions: [null],
  });

  return engine.callFunction(0, args);
}

describe("control flow", () => {

  describe("block/end (0x02/0x0B)", () => {
    it("block with i32.const inside returns the value", () => {
      /*
       * block (result i32)    ;; 0x02 0x7F
       *   i32.const 42        ;; 0x41 0x2A
       * end                   ;; 0x0B
       * ;; function end        ;; 0x0B (appended by makeBody)
       */
      const leb42 = encodeSigned(42);
      const result = run([], [ValueType.I32], [], [
        0x02, 0x7f,           // block (result i32)
        0x41, ...leb42,       // i32.const 42
        0x0b,                 // end (block)
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
    });
  });

  describe("loop with br 0 (counting loop)", () => {
    it("loops 3 times using a counter and returns final value", () => {
      /*
       * Count from 3 down to 0 using a loop inside a result-bearing block.
       * When counter reaches 0, push the counter value and br out.
       *
       * block (result i32)
       *   loop
       *     local.get 0       ;; counter
       *     i32.eqz
       *     if                ;; counter == 0?
       *       local.get 0
       *       br 2            ;; break out to result-bearing block with counter on stack
       *     end
       *     local.get 0
       *     i32.const 1
       *     i32.sub
       *     local.set 0       ;; counter--
       *     br 0              ;; continue loop
       *   end
       *   i32.const -1        ;; unreachable fallback
       * end
       */
      const leb3 = encodeSigned(3);
      const leb1 = encodeSigned(1);
      const leb0 = encodeUnsigned(0);
      const result = run([], [ValueType.I32], [ValueType.I32], [
        0x41, ...leb3,          // i32.const 3
        0x21, ...leb0,          // local.set 0

        0x02, 0x7f,             // block (result i32)
          0x03, 0x40,           // loop (no result)

            0x20, ...leb0,      // local.get 0
            0x45,               // i32.eqz
            0x04, 0x40,         // if (no result)
              0x20, ...leb0,    // local.get 0 (= 0)
              0x0c, 0x02,       // br 2 (outermost block: block label)
            0x0b,               // end (if)

            0x20, ...leb0,      // local.get 0
            0x41, ...leb1,      // i32.const 1
            0x6b,               // i32.sub
            0x21, ...leb0,      // local.set 0

            0x0c, 0x00,         // br 0 (loop continue)
          0x0b,                 // end (loop)

          0x41, ...encodeSigned(-1), // unreachable fallback
        0x0b,                   // end (block)
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(0);
    });
  });

  describe("if/else/end (0x04/0x05/0x0B)", () => {
    it("takes the then branch when condition is nonzero", () => {
      /*
       * i32.const 1           ;; push nonzero (true)
       * if (result i32)       ;; 0x04 0x7F
       *   i32.const 10        ;; then branch
       * else                  ;; 0x05
       *   i32.const 20        ;; else branch
       * end                   ;; 0x0B
       */
      const result = run([], [ValueType.I32], [], [
        0x41, ...encodeSigned(1),  // i32.const 1 (true)
        0x04, 0x7f,                // if (result i32)
        0x41, ...encodeSigned(10), // i32.const 10
        0x05,                      // else
        0x41, ...encodeSigned(20), // i32.const 20
        0x0b,                      // end
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(10);
    });

    it("takes the else branch when condition is zero", () => {
      const result = run([], [ValueType.I32], [], [
        0x41, ...encodeSigned(0),  // i32.const 0 (false)
        0x04, 0x7f,                // if (result i32)
        0x41, ...encodeSigned(10), // i32.const 10
        0x05,                      // else
        0x41, ...encodeSigned(20), // i32.const 20
        0x0b,                      // end
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(20);
    });
  });

  describe("br (0x0C)", () => {
    it("branches out of a block", () => {
      /*
       * block (result i32)
       *   i32.const 42
       *   br 0                ;; jump to end of block, carrying 42
       *   i32.const 99        ;; should be skipped
       * end
       */
      const result = run([], [ValueType.I32], [], [
        0x02, 0x7f,                // block (result i32)
        0x41, ...encodeSigned(42), // i32.const 42
        0x0c, 0x00,                // br 0
        0x41, ...encodeSigned(99), // i32.const 99 (dead code)
        0x0b,                      // end (block)
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
    });
  });

  describe("br_if (0x0D)", () => {
    it("takes the branch when condition is nonzero", () => {
      const result = run([], [ValueType.I32], [], [
        0x02, 0x7f,                // block (result i32)
        0x41, ...encodeSigned(42), // i32.const 42
        0x41, ...encodeSigned(1),  // i32.const 1 (condition: true)
        0x0d, 0x00,                // br_if 0 (taken)
        0x1a,                      // drop (would drop 42 if reached)
        0x41, ...encodeSigned(99), // i32.const 99
        0x0b,                      // end
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
    });

    it("does not branch when condition is zero", () => {
      const result = run([], [ValueType.I32], [], [
        0x02, 0x7f,                // block (result i32)
        0x41, ...encodeSigned(42), // i32.const 42
        0x41, ...encodeSigned(0),  // i32.const 0 (condition: false)
        0x0d, 0x00,                // br_if 0 (not taken)
        0x1a,                      // drop (drops 42)
        0x41, ...encodeSigned(99), // i32.const 99
        0x0b,                      // end
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(99);
    });
  });

  describe("return (0x0F)", () => {
    it("returns early from a function", () => {
      /*
       * i32.const 42
       * return             ;; early return
       * i32.const 99      ;; should be skipped
       */
      const result = run([], [ValueType.I32], [], [
        0x41, ...encodeSigned(42),  // i32.const 42
        0x0f,                       // return
        0x41, ...encodeSigned(99),  // dead code
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
    });
  });

  describe("nested blocks", () => {
    it("branch to outer block from inner block", () => {
      /*
       * block (result i32)          ;; outer (label 1 from inner)
       *   block                     ;; inner (label 0 from inner)
       *     i32.const 42
       *     br 1                    ;; branch to outer block
       *     i32.const 99            ;; dead code
       *   end
       *   i32.const 77              ;; dead code (skipped by br 1)
       * end
       */
      const result = run([], [ValueType.I32], [], [
        0x02, 0x7f,                // outer block (result i32)
        0x02, 0x40,                // inner block (no result)
        0x41, ...encodeSigned(42), // i32.const 42
        0x0c, 0x01,                // br 1 (jump to outer end)
        0x41, ...encodeSigned(99), // dead code
        0x0b,                      // end (inner)
        0x41, ...encodeSigned(77), // dead code
        0x0b,                      // end (outer)
      ]);

      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
    });
  });

  describe("unreachable (0x00)", () => {
    it("traps when executed", () => {
      expect(() => run([], [], [], [0x00])).toThrow(TrapError);
    });
  });

  describe("nop (0x01)", () => {
    it("does nothing and continues", () => {
      const result = run([], [ValueType.I32], [], [
        0x01,                       // nop
        0x41, ...encodeSigned(42),  // i32.const 42
      ]);
      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
    });
  });
});
