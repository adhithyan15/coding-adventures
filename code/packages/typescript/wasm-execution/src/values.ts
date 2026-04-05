/**
 * values.ts --- Typed WASM values and constructor/assertion helpers.
 *
 * ===========================================================================
 * WHAT ARE WASM VALUES?
 * ===========================================================================
 *
 * Every value in WebAssembly is *typed* --- it carries both a raw payload
 * and a type tag that identifies which of the four numeric types it belongs
 * to. This is unlike JavaScript, where ``42`` and ``42.0`` are the same
 * thing (both are IEEE 754 doubles). In WASM:
 *
 *   - ``i32(42)`` is a 32-bit integer with value 42.
 *   - ``f64(42.0)`` is a 64-bit float with value 42.0.
 *   - They are DIFFERENT values, and using one where the other is expected
 *     would be a type error.
 *
 * The four WASM 1.0 value types are:
 *
 *   +------+------------------------------------------------------+
 *   | Type | Description                                          |
 *   +------+------------------------------------------------------+
 *   | i32  | 32-bit integer (stored as JavaScript ``number``)     |
 *   | i64  | 64-bit integer (stored as JavaScript ``bigint``)     |
 *   | f32  | 32-bit IEEE 754 float (stored as JS ``number``)      |
 *   | f64  | 64-bit IEEE 754 float (stored as JS ``number``)      |
 *   +------+------------------------------------------------------+
 *
 * Note: i32, f32, and f64 all use JavaScript ``number`` as the payload
 * type, but i64 uses ``bigint`` because JavaScript ``number`` (64-bit
 * double) cannot represent all 64-bit integers exactly.
 *
 * ===========================================================================
 * WRAPPING SEMANTICS
 * ===========================================================================
 *
 * WASM integers have fixed bit widths. When an arithmetic operation produces
 * a result that exceeds the representable range, the value *wraps around*
 * (modular arithmetic). Our constructor functions enforce this:
 *
 *   - i32: uses ``(v | 0)`` which coerces to a signed 32-bit integer.
 *     This means values wrap to the range [-2^31, 2^31 - 1].
 *     Example: i32(0xFFFFFFFF).value === -1  (two's complement)
 *
 *   - i64: uses ``BigInt.asIntN(64, v)`` which clamps to signed 64-bit.
 *     Example: i64(2n ** 64n).value === 0n  (wraps around)
 *
 *   - f32: uses ``Math.fround(v)`` which rounds to the nearest IEEE 754
 *     single-precision (32-bit) float.
 *     Example: f32(1.1).value !== 1.1  (limited precision)
 *
 *   - f64: uses the native JS number (already IEEE 754 double-precision).
 *     No rounding needed.
 *
 * ===========================================================================
 * TYPE-SAFE EXTRACTION
 * ===========================================================================
 *
 * The ``as*`` functions (asI32, asI64, asF32, asF64) extract the raw JS
 * value with a type assertion --- if the type tag does not match, a
 * TrapError is thrown. This catches type confusion bugs early, which is
 * important because WASM is a typed language.
 *
 * @module
 */

import { ValueType } from "@coding-adventures/wasm-types";
import { TrapError } from "./host_interface.js";

// ===========================================================================
// WasmValue Type
// ===========================================================================

/**
 * A typed WASM value: a numeric payload tagged with its ValueType.
 *
 * This is the fundamental unit of data in the WASM execution engine.
 * The ``type`` field is one of the ValueType constants (0x7F for i32,
 * 0x7E for i64, 0x7D for f32, 0x7C for f64), and ``value`` is the
 * raw payload in the appropriate JavaScript type.
 */
/**
 * WasmValue is a TypedVMValue — we use the GenericVM's typed value interface
 * directly. The ``value`` field uses VMValue (which includes string/null/CodeObject)
 * for type compatibility with the GenericVM's typed stack. In practice, WASM
 * values are always numeric (number or bigint), but the type must be compatible
 * with pushTyped/popTyped.
 */
export type WasmValue = import("@coding-adventures/virtual-machine").TypedVMValue;

/**
 * Helper to pop a WasmValue from the typed stack.
 *
 * GenericVM.popTyped() returns TypedVMValue (where value is VMValue). Since
 * we know all WASM values have numeric payloads, this helper casts safely.
 */
export function popWasm(vm: { popTyped(): { type: number; value: unknown } }): WasmValue {
  const v = vm.popTyped();
  return v as unknown as WasmValue;
}

/**
 * Helper to peek a WasmValue from the typed stack.
 */
export function peekWasm(vm: { peekTyped(): { type: number; value: unknown } }): WasmValue {
  const v = vm.peekTyped();
  return v as unknown as WasmValue;
}

// ===========================================================================
// Constructor Functions
// ===========================================================================

/**
 * Create an i32 (32-bit integer) WASM value.
 *
 * The bitwise OR with zero (``v | 0``) coerces any JS number to a signed
 * 32-bit integer. This wrapping is required by the WASM specification:
 *
 *   +-----------------------+-----------+-----------------------+
 *   | Input                 | v | 0     | Why?                  |
 *   +-----------------------+-----------+-----------------------+
 *   | 42                    | 42        | Fits in i32           |
 *   | -1                    | -1        | Already valid         |
 *   | 0xFFFFFFFF (2^32 - 1) | -1        | Wraps to signed       |
 *   | 0x100000000 (2^32)    | 0         | Truncates to 32 bits  |
 *   | 3.7                   | 3         | Truncates fraction    |
 *   | NaN                   | 0         | NaN becomes 0         |
 *   +-----------------------+-----------+-----------------------+
 */
export function i32(value: number): WasmValue {
  return { type: ValueType.I32, value: value | 0 };
}

/**
 * Create an i64 (64-bit integer) WASM value (uses BigInt).
 *
 * ``BigInt.asIntN(64, v)`` clamps to the signed 64-bit range
 * [-2^63, 2^63 - 1], wrapping on overflow.
 */
export function i64(value: bigint): WasmValue {
  return { type: ValueType.I64, value: BigInt.asIntN(64, value) };
}

/**
 * Create an f32 (32-bit float) WASM value.
 *
 * ``Math.fround`` rounds to the nearest IEEE 754 single-precision value.
 * This matters because JS numbers are 64-bit doubles with ~15 digits of
 * precision, while f32 only has ~7 digits.
 */
export function f32(value: number): WasmValue {
  return { type: ValueType.F32, value: Math.fround(value) };
}

/**
 * Create an f64 (64-bit float) WASM value.
 *
 * JavaScript numbers are already 64-bit doubles, so no conversion needed.
 */
export function f64(value: number): WasmValue {
  return { type: ValueType.F64, value };
}

// ===========================================================================
// Default Value
// ===========================================================================

/**
 * Create a zero-initialized WasmValue for a given type code.
 *
 * When a WASM function is called, all local variables are initialized to
 * the "default value" for their type. The WASM spec (section 4.2.1) says:
 * "The default value of a value type is the respective zero."
 *
 *   +------+-------------------+
 *   | Type | Default           |
 *   +------+-------------------+
 *   | i32  | 0                 |
 *   | i64  | 0n                |
 *   | f32  | 0.0               |
 *   | f64  | 0.0               |
 *   +------+-------------------+
 */
export function defaultValue(type: number): WasmValue {
  switch (type) {
    case ValueType.I32:
      return i32(0);
    case ValueType.I64:
      return i64(0n);
    case ValueType.F32:
      return f32(0);
    case ValueType.F64:
      return f64(0);
    default:
      throw new TrapError(`Unknown value type: 0x${type.toString(16)}`);
  }
}

// ===========================================================================
// Type Extraction Helpers
// ===========================================================================

/**
 * Human-readable names for WASM value types, used in error messages.
 */
const typeNames: Record<number, string> = {
  [ValueType.I32]: "i32",
  [ValueType.I64]: "i64",
  [ValueType.F32]: "f32",
  [ValueType.F64]: "f64",
};

/**
 * Extract the raw ``number`` from an i32 WasmValue.
 *
 * Traps if the value is not actually an i32. This type-safety check
 * catches bugs in the execution engine early --- a type mismatch means
 * either the compiler emitted wrong code or the engine has a bug.
 */
export function asI32(v: WasmValue): number {
  if (v.type !== ValueType.I32) {
    throw new TrapError(
      `Type mismatch: expected i32, got ${typeNames[v.type] ?? `0x${v.type.toString(16)}`}`
    );
  }
  return v.value as number;
}

/**
 * Extract the raw ``bigint`` from an i64 WasmValue.
 *
 * Traps on type mismatch.
 */
export function asI64(v: WasmValue): bigint {
  if (v.type !== ValueType.I64) {
    throw new TrapError(
      `Type mismatch: expected i64, got ${typeNames[v.type] ?? `0x${v.type.toString(16)}`}`
    );
  }
  return v.value as bigint;
}

/**
 * Extract the raw ``number`` from an f32 WasmValue.
 *
 * Traps on type mismatch.
 */
export function asF32(v: WasmValue): number {
  if (v.type !== ValueType.F32) {
    throw new TrapError(
      `Type mismatch: expected f32, got ${typeNames[v.type] ?? `0x${v.type.toString(16)}`}`
    );
  }
  return v.value as number;
}

/**
 * Extract the raw ``number`` from an f64 WasmValue.
 *
 * Traps on type mismatch.
 */
export function asF64(v: WasmValue): number {
  if (v.type !== ValueType.F64) {
    throw new TrapError(
      `Type mismatch: expected f64, got ${typeNames[v.type] ?? `0x${v.type.toString(16)}`}`
    );
  }
  return v.value as number;
}
