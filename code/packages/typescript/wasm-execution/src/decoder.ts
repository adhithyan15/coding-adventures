/**
 * WASM Bytecode Decoder — Bridging Variable-Length to Fixed-Format.
 *
 * ==========================================================================
 * Chapter 1: The Decoding Problem
 * ==========================================================================
 *
 * WebAssembly bytecodes are **variable-length** — the opcode is 1 byte,
 * but immediates can be 1 to 10+ bytes depending on LEB128 encoding.
 * GenericVM expects **fixed-format** Instruction objects with a single
 * opcode and optional operand.
 *
 * The decoder bridges this gap as a **pre-instruction hook**. Before each
 * instruction is dispatched, the decoder:
 *
 * 1. Reads the raw opcode byte at the current byte offset.
 * 2. Decodes any immediate operands (LEB128 integers, memarg, etc.).
 * 3. Returns a fixed-format Instruction with the decoded operand.
 * 4. Advances the byte offset past the consumed bytes.
 *
 * This lets opcode handlers work with clean, decoded operands rather than
 * dealing with raw byte manipulation.
 *
 * ==========================================================================
 * Chapter 2: Control Flow Map Construction
 * ==========================================================================
 *
 * The decoder also builds the **control flow map** — a lookup table mapping
 * each block/loop/if instruction to its matching end (and else for if).
 * This is built once per function via a pre-scan pass, avoiding the need
 * to scan forward during execution.
 *
 * @module
 */

import { decodeUnsigned, decodeSigned } from "@coding-adventures/wasm-leb128";
import { getOpcode } from "@coding-adventures/wasm-opcodes";
import type { Instruction } from "@coding-adventures/virtual-machine";
import type { FunctionBody } from "@coding-adventures/wasm-types";
import type { ControlTarget } from "./types.js";

// =========================================================================
// Instruction Decoding
// =========================================================================

/**
 * A decoded WASM instruction with its byte offset and size.
 */
export interface DecodedInstruction {
  /** The opcode byte. */
  readonly opcode: number;

  /** The decoded operand(s), or undefined if no immediates. */
  readonly operand: unknown;

  /** The byte offset where this instruction starts. */
  readonly offset: number;

  /** The total number of bytes consumed by this instruction. */
  readonly size: number;
}

/**
 * Decode all instructions in a function body's bytecodes.
 *
 * Converts variable-length WASM bytecodes into an array of fixed-format
 * Instruction objects. This is the bridge between WASM's binary format
 * and GenericVM's instruction-index-based execution.
 *
 * @param body - The function body with raw bytecodes.
 * @returns    An array of decoded instructions.
 */
export function decodeFunctionBody(body: FunctionBody): DecodedInstruction[] {
  const code = body.code;
  const instructions: DecodedInstruction[] = [];
  let offset = 0;

  while (offset < code.length) {
    const startOffset = offset;
    const opcodeByte = code[offset];
    offset += 1;

    // Look up the opcode metadata to determine what immediates to expect.
    const info = getOpcode(opcodeByte);
    let operand: unknown = undefined;

    if (info) {
      operand = decodeImmediates(code, offset, info.immediates);
      offset += immediatesByteSize(code, offset, info.immediates);
    } else {
      // Unknown opcode — just record the byte with no operand.
      // The handler (if any) will deal with it.
    }

    instructions.push({
      opcode: opcodeByte,
      operand,
      offset: startOffset,
      size: offset - startOffset,
    });
  }

  return instructions;
}

/**
 * Decode immediate operands from the bytecodes.
 *
 * Each WASM opcode has a specific set of immediates defined by its metadata.
 * The immediates field is an array of strings like ["i32"], ["blocktype"],
 * ["memarg"], etc.
 */
function decodeImmediates(
  code: Uint8Array,
  offset: number,
  immediates: readonly string[],
): unknown {
  if (immediates.length === 0) return undefined;

  // Single immediate — return its value directly.
  if (immediates.length === 1) {
    return decodeSingleImmediate(code, offset, immediates[0]);
  }

  // Multiple immediates — return as an object.
  // This handles memarg (align + offset), call_indirect (typeidx + tableidx),
  // and br_table (vec_labelidx + default).
  const result: Record<string, unknown> = {};
  let pos = offset;
  for (const imm of immediates) {
    const [value, size] = decodeSingleImmediateWithSize(code, pos, imm);
    result[imm] = value;
    pos += size;
  }
  return result;
}

/**
 * Decode a single immediate value from the bytecodes.
 */
function decodeSingleImmediate(
  code: Uint8Array,
  offset: number,
  type: string,
): unknown {
  return decodeSingleImmediateWithSize(code, offset, type)[0];
}

/**
 * Decode a single immediate and return both value and byte size consumed.
 */
function decodeSingleImmediateWithSize(
  code: Uint8Array,
  offset: number,
  type: string,
): [unknown, number] {
  switch (type) {
    case "i32":
    case "labelidx":
    case "funcidx":
    case "typeidx":
    case "localidx":
    case "globalidx":
    case "tableidx":
    case "memidx": {
      // Signed LEB128 for i32 const, unsigned LEB128 for indices.
      if (type === "i32") {
        const [value, consumed] = decodeSigned(code, offset);
        return [value, consumed];
      }
      const [value, consumed] = decodeUnsigned(code, offset);
      return [value, consumed];
    }

    case "i64": {
      // i64 immediate — signed LEB128, but can be up to 10 bytes.
      // For 64-bit, we need to decode as BigInt.
      const [value, consumed] = decodeSigned64(code, offset);
      return [value, consumed];
    }

    case "f32": {
      // 4 bytes, little-endian IEEE 754 float.
      const buf = new ArrayBuffer(4);
      const view = new DataView(buf);
      for (let i = 0; i < 4; i++) {
        view.setUint8(i, code[offset + i]);
      }
      return [view.getFloat32(0, true), 4];
    }

    case "f64": {
      // 8 bytes, little-endian IEEE 754 double.
      const buf = new ArrayBuffer(8);
      const view = new DataView(buf);
      for (let i = 0; i < 8; i++) {
        view.setUint8(i, code[offset + i]);
      }
      return [view.getFloat64(0, true), 8];
    }

    case "blocktype": {
      // Block type: 0x40 (empty) or a value type byte, or a signed LEB128 type index.
      const byte = code[offset];
      if (byte === 0x40) return [0x40, 1];
      if (byte === 0x7F || byte === 0x7E || byte === 0x7D || byte === 0x7C) {
        return [byte, 1];
      }
      // Type index (signed LEB128) — for multi-value blocks.
      const [value, consumed] = decodeSigned(code, offset);
      return [value, consumed];
    }

    case "memarg": {
      // Memory argument: align (unsigned LEB128) + offset (unsigned LEB128).
      const [align, alignSize] = decodeUnsigned(code, offset);
      const [memOffset, offsetSize] = decodeUnsigned(code, offset + alignSize);
      return [{ align, offset: memOffset }, alignSize + offsetSize];
    }

    case "vec_labelidx": {
      // Branch table: count (unsigned LEB128) + label indices + default label.
      const [count, countSize] = decodeUnsigned(code, offset);
      let pos = offset + countSize;
      const labels: number[] = [];
      for (let i = 0; i < count; i++) {
        const [label, labelSize] = decodeUnsigned(code, pos);
        labels.push(label);
        pos += labelSize;
      }
      const [defaultLabel, defaultSize] = decodeUnsigned(code, pos);
      pos += defaultSize;
      return [{ labels, defaultLabel }, pos - offset];
    }

    default:
      return [undefined, 0];
  }
}

/**
 * Calculate the byte size of immediate operands without decoding them.
 */
function immediatesByteSize(
  code: Uint8Array,
  offset: number,
  immediates: readonly string[],
): number {
  let totalSize = 0;
  let pos = offset;
  for (const imm of immediates) {
    const [, size] = decodeSingleImmediateWithSize(code, pos, imm);
    totalSize += size;
    pos += size;
  }
  return totalSize;
}

// =========================================================================
// 64-bit Signed LEB128 Decoding
// =========================================================================

/**
 * Decode a signed 64-bit LEB128 value as a BigInt.
 *
 * Standard LEB128 but extended to handle up to 10 bytes for 64-bit values.
 */
function decodeSigned64(data: Uint8Array, offset: number): [bigint, number] {
  let result = 0n;
  let shift = 0n;
  let bytesConsumed = 0;
  const size = 64n;

  // eslint-disable-next-line no-constant-condition
  while (true) {
    if (offset + bytesConsumed >= data.length) {
      throw new Error("unterminated LEB128 sequence");
    }
    const byte = data[offset + bytesConsumed];
    bytesConsumed++;

    result |= BigInt(byte & 0x7F) << shift;
    shift += 7n;

    if ((byte & 0x80) === 0) {
      // Sign extension: if the sign bit of the last byte is set,
      // fill the remaining bits with 1s.
      if (shift < size && (byte & 0x40) !== 0) {
        result |= -(1n << shift);
      }
      break;
    }

    if (bytesConsumed >= 10) {
      throw new Error("LEB128 sequence too long for i64");
    }
  }

  return [BigInt.asIntN(64, result), bytesConsumed];
}

// =========================================================================
// Control Flow Map Construction
// =========================================================================

/**
 * Build the control flow map for a function body.
 *
 * Scans through all decoded instructions and maps each block/loop/if
 * start to its matching end (and else for if instructions).
 *
 * This is a one-time O(n) scan that avoids forward-scanning during execution.
 * The algorithm uses a stack to track nesting:
 *
 * 1. When we see ``block``, ``loop``, or ``if``: push its instruction index.
 * 2. When we see ``else``: record it for the most recent ``if``.
 * 3. When we see ``end``: pop the stack and record the mapping.
 *
 * @param instructions - The decoded instructions for the function.
 * @returns            A map from block/loop/if start index to control target.
 */
export function buildControlFlowMap(
  instructions: DecodedInstruction[],
): Map<number, ControlTarget> {
  const map = new Map<number, ControlTarget>();

  // Stack of (instruction_index, opcode, else_index_or_null).
  const stack: { index: number; opcode: number; elsePc: number | null }[] = [];

  for (let i = 0; i < instructions.length; i++) {
    const instr = instructions[i];

    switch (instr.opcode) {
      case 0x02: // block
      case 0x03: // loop
      case 0x04: // if
        stack.push({ index: i, opcode: instr.opcode, elsePc: null });
        break;

      case 0x05: // else
        if (stack.length > 0) {
          stack[stack.length - 1].elsePc = i;
        }
        break;

      case 0x0B: // end
        if (stack.length > 0) {
          const opener = stack.pop()!;
          map.set(opener.index, {
            endPc: i,
            elsePc: opener.elsePc,
          });
        }
        // If stack is empty, this is the function's trailing ``end``.
        break;
    }
  }

  return map;
}

/**
 * Convert decoded instructions to GenericVM's Instruction[] format.
 *
 * This strips the byte-offset metadata, keeping only opcode + operand.
 */
export function toVmInstructions(decoded: DecodedInstruction[]): Instruction[] {
  return decoded.map(d => ({
    opcode: d.opcode as Instruction["opcode"],
    operand: d.operand as number | string | null | undefined,
  }));
}
