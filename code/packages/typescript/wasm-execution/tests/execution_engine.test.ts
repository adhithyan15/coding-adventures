/**
 * Tests for WasmExecutionEngine end-to-end execution.
 *
 * These tests build small WASM programs by hand (raw bytecodes) and
 * execute them through the full engine pipeline: decode -> control flow
 * map -> initialize locals -> execute -> collect results.
 */

import { describe, it, expect } from "vitest";
import { WasmExecutionEngine } from "../src/wasm_execution.js";
import { i32, i64, f32, f64, defaultValue } from "../src/values.js";
import type { WasmValue } from "../src/values.js";
import type { FuncType, FunctionBody } from "@coding-adventures/wasm-types";
import { ValueType, makeFuncType } from "@coding-adventures/wasm-types";
import { encodeSigned, encodeUnsigned } from "@coding-adventures/wasm-leb128";
import { TrapError } from "../src/host_interface.js";
import type { HostFunction } from "../src/host_interface.js";

/**
 * Build a FunctionBody from raw bytecodes.
 *
 * @param locals - Array of ValueType codes for declared locals (NOT params).
 * @param bytecodes - Raw opcode bytes. The trailing 0x0B (end) is appended
 *                    automatically.
 */
function makeBody(locals: number[], ...bytecodes: number[]): FunctionBody {
  return {
    locals: locals.map(t => t as ValueType),
    code: new Uint8Array([...bytecodes, 0x0b]),
  };
}

describe("WasmExecutionEngine", () => {

  describe("simple constant return", () => {
    it("returns i32.const 42", () => {
      /*
       * Function type: () -> i32
       * Body: i32.const 42, end
       */
      const funcType = makeFuncType([], [ValueType.I32]);
      const leb42 = encodeSigned(42);
      const body = makeBody([], 0x41, ...leb42);

      const engine = new WasmExecutionEngine({
        memory: null,
        tables: [],
        globals: [],
        globalTypes: [],
        funcTypes: [funcType],
        funcBodies: [body],
        hostFunctions: [null],
      });

      const result = engine.callFunction(0, []);
      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
      expect(result[0].type).toBe(ValueType.I32);
    });
  });

  describe("add function with arguments", () => {
    it("adds two i32 arguments: (3 + 4) = 7", () => {
      /*
       * Function type: (i32, i32) -> i32
       * Body: local.get 0, local.get 1, i32.add, end
       *
       * local.get = 0x20, localidx is unsigned LEB128.
       * i32.add   = 0x6A.
       */
      const funcType = makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]);
      const body = makeBody([], 0x20, 0x00, 0x20, 0x01, 0x6a);

      const engine = new WasmExecutionEngine({
        memory: null,
        tables: [],
        globals: [],
        globalTypes: [],
        funcTypes: [funcType],
        funcBodies: [body],
        hostFunctions: [null],
      });

      const result = engine.callFunction(0, [i32(3), i32(4)]);
      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(7);
    });
  });

  describe("function with declared locals", () => {
    it("uses local.set and local.get for a declared local", () => {
      /*
       * Function type: (i32) -> i32
       * Declared locals: [i32]  (so local 0 = param, local 1 = declared)
       * Body: local.get 0, local.set 1, local.get 1, end
       *
       * local.set = 0x21
       */
      const funcType = makeFuncType([ValueType.I32], [ValueType.I32]);
      const body = makeBody(
        [ValueType.I32],  // one declared i32 local
        0x20, 0x00,       // local.get 0
        0x21, 0x01,       // local.set 1
        0x20, 0x01,       // local.get 1
      );

      const engine = new WasmExecutionEngine({
        memory: null,
        tables: [],
        globals: [],
        globalTypes: [],
        funcTypes: [funcType],
        funcBodies: [body],
        hostFunctions: [null],
      });

      const result = engine.callFunction(0, [i32(99)]);
      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(99);
    });
  });

  describe("host function call", () => {
    it("calls a host function and returns its result", () => {
      /*
       * Set up func index 0 as a host function that doubles its argument.
       * Function type: (i32) -> i32
       */
      const funcType = makeFuncType([ValueType.I32], [ValueType.I32]);
      const hostFunc: HostFunction = {
        type: funcType,
        call(args: WasmValue[]): WasmValue[] {
          const val = args[0].value as number;
          return [i32(val * 2)];
        },
      };

      const engine = new WasmExecutionEngine({
        memory: null,
        tables: [],
        globals: [],
        globalTypes: [],
        funcTypes: [funcType],
        funcBodies: [null],       // no body — it's a host function
        hostFunctions: [hostFunc],
      });

      const result = engine.callFunction(0, [i32(21)]);
      expect(result).toHaveLength(1);
      expect(result[0].value).toBe(42);
    });
  });

  describe("error handling", () => {
    it("traps on undefined function index", () => {
      const engine = new WasmExecutionEngine({
        memory: null,
        tables: [],
        globals: [],
        globalTypes: [],
        funcTypes: [],
        funcBodies: [],
        hostFunctions: [],
      });

      expect(() => engine.callFunction(99, [])).toThrow(TrapError);
    });

    it("traps on argument count mismatch", () => {
      const funcType = makeFuncType([ValueType.I32, ValueType.I32], [ValueType.I32]);
      const body = makeBody([], 0x20, 0x00); // doesn't matter

      const engine = new WasmExecutionEngine({
        memory: null,
        tables: [],
        globals: [],
        globalTypes: [],
        funcTypes: [funcType],
        funcBodies: [body],
        hostFunctions: [null],
      });

      // Pass 1 arg when 2 are expected
      expect(() => engine.callFunction(0, [i32(1)])).toThrow(TrapError);
    });
  });
});
