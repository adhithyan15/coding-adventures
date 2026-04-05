/**
 * Tests for the WASM bytecode decoder.
 *
 * The decoder bridges variable-length WASM bytecodes to the fixed-format
 * Instruction objects that GenericVM expects. These tests verify that
 * decodeFunctionBody correctly parses opcodes and immediates, that
 * buildControlFlowMap correctly maps block starts to ends, and that
 * toVmInstructions strips byte-offset metadata.
 */

import { describe, it, expect } from "vitest";
import {
  decodeFunctionBody,
  buildControlFlowMap,
  toVmInstructions,
} from "../src/decoder.js";
import type { DecodedInstruction } from "../src/decoder.js";
import type { FunctionBody } from "@coding-adventures/wasm-types";
import { encodeSigned, encodeUnsigned } from "@coding-adventures/wasm-leb128";

/**
 * Helper: build a FunctionBody from raw bytes (no locals).
 * The caller should include the trailing 0x0B (end) byte.
 */
function body(...bytes: number[]): FunctionBody {
  return { locals: [], code: new Uint8Array(bytes) };
}

describe("decoder", () => {

  // =========================================================================
  // decodeFunctionBody
  // =========================================================================

  describe("decodeFunctionBody", () => {

    it("decodes a simple function: local.get 0, local.get 0, i32.mul, end", () => {
      /*
       * local.get = 0x20, with a localidx immediate (unsigned LEB128).
       * i32.mul   = 0x6C, no immediates.
       * end       = 0x0B, no immediates.
       */
      const fn = body(0x20, 0x00, 0x20, 0x00, 0x6c, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded).toHaveLength(4);

      // local.get 0
      expect(decoded[0].opcode).toBe(0x20);
      expect(decoded[0].operand).toBe(0);

      // local.get 0
      expect(decoded[1].opcode).toBe(0x20);
      expect(decoded[1].operand).toBe(0);

      // i32.mul (no operand)
      expect(decoded[2].opcode).toBe(0x6c);
      expect(decoded[2].operand).toBeUndefined();

      // end
      expect(decoded[3].opcode).toBe(0x0b);
    });

    it("decodes i32.const with LEB128 immediate", () => {
      /*
       * i32.const = 0x41, with a signed LEB128 i32 immediate.
       * The value 42 encodes as a single byte: 0x2A.
       * The value 300 encodes as two bytes: 0xAC 0x02.
       */
      const leb300 = encodeSigned(300);
      const fn = body(0x41, ...leb300, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded).toHaveLength(2); // i32.const + end
      expect(decoded[0].opcode).toBe(0x41);
      expect(decoded[0].operand).toBe(300);
    });

    it("decodes i32.const with negative LEB128 immediate", () => {
      const lebNeg1 = encodeSigned(-1);
      const fn = body(0x41, ...lebNeg1, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded[0].opcode).toBe(0x41);
      expect(decoded[0].operand).toBe(-1);
    });

    it("decodes block with blocktype immediate", () => {
      /*
       * block = 0x02, with a blocktype immediate.
       * 0x40 means empty block type (no results).
       */
      const fn = body(0x02, 0x40, 0x0b, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded).toHaveLength(3); // block, end (block), end (function)
      expect(decoded[0].opcode).toBe(0x02);
      expect(decoded[0].operand).toBe(0x40); // empty blocktype
    });

    it("decodes block with i32 result blocktype", () => {
      /*
       * block(result i32): blocktype = 0x7F
       */
      const fn = body(0x02, 0x7f, 0x41, 0x2a, 0x0b, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded[0].opcode).toBe(0x02);
      expect(decoded[0].operand).toBe(0x7f); // i32 result type
    });

    it("decodes f32.const with 4-byte IEEE 754 immediate", () => {
      /*
       * f32.const = 0x43, with a 4-byte little-endian f32 immediate.
       * The value 1.0 in f32 is 0x3F800000 = bytes [0x00, 0x00, 0x80, 0x3F].
       */
      const fn = body(0x43, 0x00, 0x00, 0x80, 0x3f, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded).toHaveLength(2);
      expect(decoded[0].opcode).toBe(0x43);
      expect(decoded[0].operand).toBeCloseTo(1.0, 5);
    });

    it("decodes memory load with memarg immediate (align + offset)", () => {
      /*
       * i32.load = 0x28, with memarg: align (unsigned LEB128) + offset (unsigned LEB128).
       * align=2 (means 2^2=4-byte aligned), offset=0.
       */
      const fn = body(0x28, 0x02, 0x00, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded).toHaveLength(2);
      expect(decoded[0].opcode).toBe(0x28);
      const memarg = decoded[0].operand as { memarg: { align: number; offset: number } };
      // The decoder returns a memarg object
      expect(memarg).toEqual({ align: 2, offset: 0 });
    });

    it("decodes memory load with nonzero offset", () => {
      const offsetLeb = encodeUnsigned(16);
      const fn = body(0x28, 0x02, ...offsetLeb, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded[0].opcode).toBe(0x28);
      const memarg = decoded[0].operand as { align: number; offset: number };
      expect(memarg).toEqual({ align: 2, offset: 16 });
    });

    it("records byte offsets and sizes correctly", () => {
      // local.get 0 (2 bytes), i32.mul (1 byte), end (1 byte)
      const fn = body(0x20, 0x00, 0x6c, 0x0b);
      const decoded = decodeFunctionBody(fn);

      expect(decoded[0].offset).toBe(0);
      expect(decoded[0].size).toBe(2); // opcode + 1-byte LEB128

      expect(decoded[1].offset).toBe(2);
      expect(decoded[1].size).toBe(1); // opcode only

      expect(decoded[2].offset).toBe(3);
      expect(decoded[2].size).toBe(1);
    });
  });

  // =========================================================================
  // buildControlFlowMap
  // =========================================================================

  describe("buildControlFlowMap", () => {

    it("maps block start to its end", () => {
      /*
       * Instructions: [block, nop, end, end]
       * Index:         0      1    2    3
       * The block at index 0 should map to end at index 2.
       */
      const decoded: DecodedInstruction[] = [
        { opcode: 0x02, operand: 0x40, offset: 0, size: 2 },  // block
        { opcode: 0x01, operand: undefined, offset: 2, size: 1 },  // nop
        { opcode: 0x0b, operand: undefined, offset: 3, size: 1 },  // end (block)
        { opcode: 0x0b, operand: undefined, offset: 4, size: 1 },  // end (function)
      ];

      const map = buildControlFlowMap(decoded);

      expect(map.get(0)).toEqual({ endPc: 2, elsePc: null });
    });

    it("maps if to else and end", () => {
      /*
       * Instructions: [if, nop, else, nop, end, end]
       * Index:         0   1    2     3    4    5
       */
      const decoded: DecodedInstruction[] = [
        { opcode: 0x04, operand: 0x40, offset: 0, size: 2 },  // if
        { opcode: 0x01, operand: undefined, offset: 2, size: 1 },  // nop (then)
        { opcode: 0x05, operand: undefined, offset: 3, size: 1 },  // else
        { opcode: 0x01, operand: undefined, offset: 4, size: 1 },  // nop (else)
        { opcode: 0x0b, operand: undefined, offset: 5, size: 1 },  // end
        { opcode: 0x0b, operand: undefined, offset: 6, size: 1 },  // end (function)
      ];

      const map = buildControlFlowMap(decoded);

      expect(map.get(0)).toEqual({ endPc: 4, elsePc: 2 });
    });

    it("maps loop start to its end", () => {
      const decoded: DecodedInstruction[] = [
        { opcode: 0x03, operand: 0x40, offset: 0, size: 2 },  // loop
        { opcode: 0x01, operand: undefined, offset: 2, size: 1 },  // nop
        { opcode: 0x0b, operand: undefined, offset: 3, size: 1 },  // end (loop)
        { opcode: 0x0b, operand: undefined, offset: 4, size: 1 },  // end (function)
      ];

      const map = buildControlFlowMap(decoded);
      expect(map.get(0)).toEqual({ endPc: 2, elsePc: null });
    });

    it("handles nested blocks", () => {
      /*
       * Instructions: [block, block, nop, end, end, end]
       * Index:         0      1      2    3    4    5
       */
      const decoded: DecodedInstruction[] = [
        { opcode: 0x02, operand: 0x40, offset: 0, size: 2 },  // outer block
        { opcode: 0x02, operand: 0x40, offset: 2, size: 2 },  // inner block
        { opcode: 0x01, operand: undefined, offset: 4, size: 1 },  // nop
        { opcode: 0x0b, operand: undefined, offset: 5, size: 1 },  // end (inner)
        { opcode: 0x0b, operand: undefined, offset: 6, size: 1 },  // end (outer)
        { opcode: 0x0b, operand: undefined, offset: 7, size: 1 },  // end (function)
      ];

      const map = buildControlFlowMap(decoded);

      expect(map.get(0)).toEqual({ endPc: 4, elsePc: null }); // outer
      expect(map.get(1)).toEqual({ endPc: 3, elsePc: null }); // inner
    });

    it("maps if without else (no else clause)", () => {
      const decoded: DecodedInstruction[] = [
        { opcode: 0x04, operand: 0x40, offset: 0, size: 2 },  // if
        { opcode: 0x01, operand: undefined, offset: 2, size: 1 },  // nop
        { opcode: 0x0b, operand: undefined, offset: 3, size: 1 },  // end
        { opcode: 0x0b, operand: undefined, offset: 4, size: 1 },  // end (function)
      ];

      const map = buildControlFlowMap(decoded);
      expect(map.get(0)).toEqual({ endPc: 2, elsePc: null });
    });
  });

  // =========================================================================
  // toVmInstructions
  // =========================================================================

  describe("toVmInstructions", () => {

    it("strips byte offset metadata", () => {
      const decoded: DecodedInstruction[] = [
        { opcode: 0x41, operand: 42, offset: 0, size: 2 },
        { opcode: 0x0b, operand: undefined, offset: 2, size: 1 },
      ];

      const instrs = toVmInstructions(decoded);

      expect(instrs).toHaveLength(2);
      expect(instrs[0]).toEqual({ opcode: 0x41, operand: 42 });
      expect(instrs[1]).toEqual({ opcode: 0x0b, operand: undefined });

      // Verify metadata is gone
      expect((instrs[0] as unknown as Record<string, unknown>).offset).toBeUndefined();
      expect((instrs[0] as unknown as Record<string, unknown>).size).toBeUndefined();
    });
  });
});
