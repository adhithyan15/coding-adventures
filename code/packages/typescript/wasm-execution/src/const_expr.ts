/**
 * const_expr.ts --- Evaluate WASM constant expressions.
 *
 * ===========================================================================
 * WHAT ARE CONSTANT EXPRESSIONS?
 * ===========================================================================
 *
 * In WebAssembly, some values must be known at module *instantiation* time
 * (before any code runs). These values are specified as "constant
 * expressions" --- tiny programs consisting of a handful of allowed opcodes
 * that produce a single value.
 *
 * Constant expressions appear in three places:
 *
 *   1. **Global initializers** --- the initial value of a global variable.
 *      Example: ``(global (mut i32) (i32.const 42))``
 *
 *   2. **Data segment offsets** --- where in linear memory to place data.
 *      Example: ``(data (i32.const 1024) "hello")``
 *
 *   3. **Element segment offsets** --- where in a table to place function
 *      references.
 *      Example: ``(elem (i32.const 0) $func0 $func1)``
 *
 * ===========================================================================
 * ALLOWED OPCODES
 * ===========================================================================
 *
 * The WASM 1.0 spec restricts constant expressions to a very small set of
 * opcodes. This is a security/simplicity measure --- it ensures that
 * initialization cannot trigger arbitrary computation:
 *
 *   +----------+----------+-------------------------------------------+
 *   | Opcode   | Hex      | Description                               |
 *   +----------+----------+-------------------------------------------+
 *   | i32.const| 0x41     | Push a 32-bit integer (LEB128 immediate)  |
 *   | i64.const| 0x42     | Push a 64-bit integer (LEB128 immediate)  |
 *   | f32.const| 0x43     | Push a 32-bit float (4 bytes LE)          |
 *   | f64.const| 0x44     | Push a 64-bit float (8 bytes LE)          |
 *   | global.get| 0x23    | Push value of an imported global          |
 *   | end      | 0x0B     | End of expression                         |
 *   +----------+----------+-------------------------------------------+
 *
 * A valid constant expression is a sequence of these opcodes ending with
 * ``end`` (0x0B). The result is the single value left on the stack.
 *
 * ===========================================================================
 * ENCODING DETAILS
 * ===========================================================================
 *
 * Each opcode has a different immediate encoding:
 *
 *   i32.const: followed by a signed LEB128 32-bit integer
 *   i64.const: followed by a signed LEB128 64-bit integer
 *   f32.const: followed by 4 raw bytes (IEEE 754 LE)
 *   f64.const: followed by 8 raw bytes (IEEE 754 LE)
 *   global.get: followed by an unsigned LEB128 global index
 *   end: no immediate (terminates the expression)
 *
 * ===========================================================================
 * WORKED EXAMPLE
 * ===========================================================================
 *
 * Consider the constant expression for ``(i32.const 42)``:
 *
 *   Bytecodes: [0x41, 0x2A, 0x0B]
 *               ^^^^  ^^^^  ^^^^
 *               |     |     |
 *               |     |     end instruction
 *               |     42 as signed LEB128 (0x2A = 42)
 *               i32.const opcode
 *
 *   Evaluation:
 *     1. Read 0x41 (i32.const), decode LEB128 → 42, push i32(42)
 *     2. Read 0x0B (end), return the value on stack → i32(42)
 *
 * @module
 */

import { decodeSigned, decodeUnsigned } from "@coding-adventures/wasm-leb128";
import { ValueType } from "@coding-adventures/wasm-types";
import type { WasmValue } from "./values.js";
import { i32, i64, f32, f64 } from "./values.js";
import { TrapError } from "./host_interface.js";

// ===========================================================================
// Opcode Constants for Constant Expressions
// ===========================================================================

/**
 * The six opcodes allowed in WASM constant expressions.
 * We define them as local constants for clarity and to avoid importing
 * the full opcode table (which includes 200+ opcodes we don't need here).
 */
const OPCODE_I32_CONST = 0x41;
const OPCODE_I64_CONST = 0x42;
const OPCODE_F32_CONST = 0x43;
const OPCODE_F64_CONST = 0x44;
const OPCODE_GLOBAL_GET = 0x23;
const OPCODE_END = 0x0b;

// ===========================================================================
// 64-bit Signed LEB128 Decoder
// ===========================================================================

/**
 * Decode a signed LEB128-encoded 64-bit integer as a BigInt.
 *
 * The ``@coding-adventures/wasm-leb128`` package only handles 32-bit
 * values (returning a JS number). For i64.const immediates, we need a
 * 64-bit decoder that returns a BigInt.
 *
 * The algorithm is the same as 32-bit signed LEB128 but uses BigInt
 * arithmetic and allows up to 10 bytes (ceil(64/7) = 10).
 *
 * @param data   - The byte array.
 * @param offset - The position to start decoding.
 * @returns A tuple of [decoded bigint value, number of bytes consumed].
 */
function decodeSigned64(
  data: Uint8Array,
  offset: number
): [bigint, number] {
  let result = 0n;
  let shift = 0n;
  let bytesConsumed = 0;

  const maxBytes = 10; // ceil(64 / 7) = 10

  for (let i = offset; i < data.length; i++) {
    if (bytesConsumed >= maxBytes) {
      throw new TrapError(
        `LEB128 sequence exceeds maximum ${maxBytes} bytes for a 64-bit value`
      );
    }

    const byte = data[i];
    const payload = BigInt(byte & 0x7f); // lower 7 bits

    result |= payload << shift;
    shift += 7n;
    bytesConsumed++;

    // If continuation bit is not set, this is the last byte.
    if ((byte & 0x80) === 0) {
      // Sign extension: if the MSB of the last 7-bit group is set and
      // we haven't filled all 64 bits yet, extend the sign.
      if (shift < 64n && (byte & 0x40) !== 0) {
        result |= -(1n << shift);
      }
      return [BigInt.asIntN(64, result), bytesConsumed];
    }
  }

  throw new TrapError(
    `LEB128 sequence is unterminated at offset ${offset + bytesConsumed}`
  );
}

// ===========================================================================
// Constant Expression Evaluator
// ===========================================================================

/**
 * Evaluate a WASM constant expression and return its result.
 *
 * A constant expression is a sequence of bytes (opcodes + immediates)
 * ending with the ``end`` byte (0x0B). The expression pushes exactly
 * one value onto a conceptual stack, and that value is the result.
 *
 * @param expr    - The raw bytes of the constant expression.
 * @param globals - The global variable values available for ``global.get``.
 *                  Only imported globals (already initialized) should be
 *                  accessible here.
 * @returns The single WasmValue produced by the expression.
 * @throws TrapError if the expression is malformed, uses an illegal
 *         opcode, or doesn't produce exactly one value.
 *
 * @example
 *   // Evaluate (i32.const 42):
 *   evaluateConstExpr(new Uint8Array([0x41, 0x2A, 0x0B]), [])
 *   // → { type: 0x7F, value: 42 }
 *
 * @example
 *   // Evaluate (global.get 0) where global 0 is i32(100):
 *   evaluateConstExpr(
 *     new Uint8Array([0x23, 0x00, 0x0B]),
 *     [{ type: 0x7F, value: 100 }]
 *   )
 *   // → { type: 0x7F, value: 100 }
 */
export function evaluateConstExpr(
  expr: Uint8Array,
  globals: WasmValue[]
): WasmValue {
  /**
   * We use a simple stack-based evaluator. Valid constant expressions
   * should push exactly one value and then end. We track the stack as
   * a single variable (since there should be at most one value).
   */
  let result: WasmValue | null = null;
  let pos = 0;

  while (pos < expr.length) {
    const opcode = expr[pos];
    pos++;

    switch (opcode) {
      // -----------------------------------------------------------------
      // i32.const: push a 32-bit integer
      // -----------------------------------------------------------------
      case OPCODE_I32_CONST: {
        // The immediate is a signed LEB128-encoded 32-bit integer.
        const [value, bytesRead] = decodeSigned(expr, pos);
        pos += bytesRead;
        result = i32(value);
        break;
      }

      // -----------------------------------------------------------------
      // i64.const: push a 64-bit integer
      // -----------------------------------------------------------------
      case OPCODE_I64_CONST: {
        // The immediate is a signed LEB128-encoded 64-bit integer.
        // We use our custom 64-bit decoder since the standard one is
        // limited to 32 bits.
        const [value, bytesRead] = decodeSigned64(expr, pos);
        pos += bytesRead;
        result = i64(value);
        break;
      }

      // -----------------------------------------------------------------
      // f32.const: push a 32-bit float
      // -----------------------------------------------------------------
      case OPCODE_F32_CONST: {
        // The immediate is 4 raw bytes in little-endian IEEE 754 format.
        // We use a DataView to decode the float from the byte array.
        if (pos + 4 > expr.length) {
          throw new TrapError(
            `f32.const at offset ${pos - 1}: not enough bytes for float32`
          );
        }
        const f32View = new DataView(
          expr.buffer,
          expr.byteOffset + pos,
          4
        );
        const f32Value = f32View.getFloat32(0, true);
        pos += 4;
        result = f32(f32Value);
        break;
      }

      // -----------------------------------------------------------------
      // f64.const: push a 64-bit float
      // -----------------------------------------------------------------
      case OPCODE_F64_CONST: {
        // The immediate is 8 raw bytes in little-endian IEEE 754 format.
        if (pos + 8 > expr.length) {
          throw new TrapError(
            `f64.const at offset ${pos - 1}: not enough bytes for float64`
          );
        }
        const f64View = new DataView(
          expr.buffer,
          expr.byteOffset + pos,
          8
        );
        const f64Value = f64View.getFloat64(0, true);
        pos += 8;
        result = f64(f64Value);
        break;
      }

      // -----------------------------------------------------------------
      // global.get: push the value of a global variable
      // -----------------------------------------------------------------
      case OPCODE_GLOBAL_GET: {
        // The immediate is an unsigned LEB128 global index.
        const [globalIndex, bytesRead] = decodeUnsigned(expr, pos);
        pos += bytesRead;

        if (globalIndex >= globals.length) {
          throw new TrapError(
            `global.get: index ${globalIndex} out of bounds ` +
              `(${globals.length} globals available)`
          );
        }
        result = globals[globalIndex];
        break;
      }

      // -----------------------------------------------------------------
      // end: terminate the expression and return the result
      // -----------------------------------------------------------------
      case OPCODE_END: {
        if (result === null) {
          throw new TrapError(
            "Constant expression produced no value (empty expression)"
          );
        }
        return result;
      }

      // -----------------------------------------------------------------
      // Anything else is illegal in a constant expression
      // -----------------------------------------------------------------
      default:
        throw new TrapError(
          `Illegal opcode 0x${opcode.toString(16).padStart(2, "0")} ` +
            `in constant expression at offset ${pos - 1}`
        );
    }
  }

  // If we reached the end of the byte array without encountering ``end``,
  // the expression is malformed.
  throw new TrapError("Constant expression missing end opcode (0x0B)");
}
