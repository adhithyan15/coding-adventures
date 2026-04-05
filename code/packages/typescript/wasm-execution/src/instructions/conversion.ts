/**
 * conversion.ts --- Type conversion instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: TYPE CONVERSIONS IN WEBASSEMBLY
 * ===========================================================================
 *
 * WASM is strongly typed: every value has an explicit type (i32, i64, f32, f64).
 * You cannot, for example, pass an i64 to a function expecting an i32 without
 * an explicit conversion instruction. This is unlike C, where implicit
 * conversions happen silently (and often cause subtle bugs).
 *
 * The conversion instructions fall into several categories:
 *
 * 1. **Wrapping** (i32.wrap_i64): Truncate a wider type to a narrower type
 *    by discarding the high bits.
 *
 * 2. **Extension** (i64.extend_i32_s/u): Widen a narrower type to a wider
 *    type, either sign-extending or zero-extending.
 *
 * 3. **Truncation** (i32.trunc_fNN / i64.trunc_fNN): Convert a float to
 *    an integer by truncating toward zero. TRAPS if the value is NaN or
 *    out of the integer range!
 *
 * 4. **Conversion** (f32.convert_iNN / f64.convert_iNN): Convert an integer
 *    to a float. May lose precision for large integers.
 *
 * 5. **Promotion/Demotion** (f64.promote_f32, f32.demote_f64): Convert
 *    between float precisions.
 *
 * 6. **Reinterpretation** (*.reinterpret_*): Reinterpret the raw bits of
 *    one type as another type, without changing any bits. This uses a
 *    shared ArrayBuffer trick in JavaScript.
 *
 * ===========================================================================
 * THE REINTERPRET TRICK
 * ===========================================================================
 *
 * JavaScript has no "type punning" or "union types" like C. To reinterpret
 * the bits of a float as an integer (or vice versa), we use a shared
 * ArrayBuffer with multiple typed array views:
 *
 *   const buf = new ArrayBuffer(8);
 *   const f64view = new Float64Array(buf);
 *   const i64view = new BigInt64Array(buf);
 *
 *   f64view[0] = 3.14;       // Write as float64
 *   const bits = i64view[0]; // Read same bytes as BigInt64
 *
 * Both views point to the same underlying bytes, so writing through one
 * and reading through the other gives us bit-for-bit reinterpretation.
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { TrapError } from "../host_interface.js";
import { i32, i64, f32, f64, asI32, asI64, asF32, asF64 } from "../values.js";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Shared Reinterpret Buffer
// ===========================================================================
//
// A single 8-byte buffer with views for all four types. This is more
// efficient than creating new buffers for each reinterpret operation.
//

const REINTERPRET_BUF = new ArrayBuffer(8);
const REINTERPRET_I32 = new Int32Array(REINTERPRET_BUF);
const REINTERPRET_F32 = new Float32Array(REINTERPRET_BUF);
const REINTERPRET_I64 = new BigInt64Array(REINTERPRET_BUF);
const REINTERPRET_F64 = new Float64Array(REINTERPRET_BUF);

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register all 27 type conversion instruction handlers.
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerConversion(vm: GenericVM): void {

  // =========================================================================
  // 0xA7: i32.wrap_i64 --- Truncate i64 to i32
  // =========================================================================
  //
  // Discards the high 32 bits of a 64-bit integer. This is how you convert
  // an i64 to an i32 when you only care about the low bits (like casting
  // ``long`` to ``int`` in Java).
  //
  //   wrap(0x00000001_FFFFFFFF) = 0xFFFFFFFF = -1 (as signed i32)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0xa7, (vm, _instr, _code, _ctx) => {
    const value = asI64(vm.popTyped());
    vm.pushTyped(i32(Number(BigInt.asIntN(32, value))));
    vm.advancePc();
    return "i32.wrap_i64";
  });

  // =========================================================================
  // 0xA8 - 0xAB: i32.trunc_f* --- Float to i32 (trapping!)
  // =========================================================================
  //
  // Truncate a float toward zero to produce an i32. Traps if:
  //   - The value is NaN.
  //   - The truncated value is outside the i32 range.
  //
  // This is WASM's strict version of C's ``(int)floatValue`` cast.
  //

  // 0xA8: i32.trunc_f32_s
  vm.registerContextOpcode<WasmExecutionContext>(0xa8, (vm, _instr, _code, _ctx) => {
    const value = asF32(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    const truncated = Math.trunc(value);
    if (truncated < -2147483648 || truncated > 2147483647) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i32(truncated | 0));
    vm.advancePc();
    return "i32.trunc_f32_s";
  });

  // 0xA9: i32.trunc_f32_u
  vm.registerContextOpcode<WasmExecutionContext>(0xa9, (vm, _instr, _code, _ctx) => {
    const value = asF32(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    const truncated = Math.trunc(value);
    if (truncated < 0 || truncated > 4294967295) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i32(truncated | 0));
    vm.advancePc();
    return "i32.trunc_f32_u";
  });

  // 0xAA: i32.trunc_f64_s
  vm.registerContextOpcode<WasmExecutionContext>(0xaa, (vm, _instr, _code, _ctx) => {
    const value = asF64(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    const truncated = Math.trunc(value);
    if (truncated < -2147483648 || truncated > 2147483647) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i32(truncated | 0));
    vm.advancePc();
    return "i32.trunc_f64_s";
  });

  // 0xAB: i32.trunc_f64_u
  vm.registerContextOpcode<WasmExecutionContext>(0xab, (vm, _instr, _code, _ctx) => {
    const value = asF64(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    const truncated = Math.trunc(value);
    if (truncated < 0 || truncated > 4294967295) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i32(truncated | 0));
    vm.advancePc();
    return "i32.trunc_f64_u";
  });

  // =========================================================================
  // 0xAC - 0xAD: i64.extend_i32 --- Widen i32 to i64
  // =========================================================================

  // 0xAC: i64.extend_i32_s --- Sign-extend
  //
  // ``BigInt(value | 0)`` works because JS ``| 0`` produces a signed 32-bit
  // integer, and ``BigInt()`` preserves the sign.
  //
  //   extend_s(-1) = -1n  (all 64 bits are 1)
  //   extend_s(1)  = 1n   (only LSB set)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0xac, (vm, _instr, _code, _ctx) => {
    const value = asI32(vm.popTyped());
    vm.pushTyped(i64(BigInt(value | 0)));
    vm.advancePc();
    return "i64.extend_i32_s";
  });

  // 0xAD: i64.extend_i32_u --- Zero-extend
  //
  // ``BigInt(value >>> 0)`` works because ``>>> 0`` produces an unsigned
  // 32-bit integer, and ``BigInt()`` converts it without sign extension.
  //
  //   extend_u(0xFFFFFFFF) = 4294967295n  (not -1n!)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0xad, (vm, _instr, _code, _ctx) => {
    const value = asI32(vm.popTyped());
    vm.pushTyped(i64(BigInt(value >>> 0)));
    vm.advancePc();
    return "i64.extend_i32_u";
  });

  // =========================================================================
  // 0xAE - 0xB1: i64.trunc_f* --- Float to i64 (trapping!)
  // =========================================================================

  // 0xAE: i64.trunc_f32_s
  vm.registerContextOpcode<WasmExecutionContext>(0xae, (vm, _instr, _code, _ctx) => {
    const value = asF32(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    if (!isFinite(value)) throw new TrapError("integer overflow");
    const truncated = Math.trunc(value);
    const big = BigInt(truncated);
    if (big < -9223372036854775808n || big > 9223372036854775807n) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i64(big));
    vm.advancePc();
    return "i64.trunc_f32_s";
  });

  // 0xAF: i64.trunc_f32_u
  vm.registerContextOpcode<WasmExecutionContext>(0xaf, (vm, _instr, _code, _ctx) => {
    const value = asF32(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    if (!isFinite(value)) throw new TrapError("integer overflow");
    const truncated = Math.trunc(value);
    if (truncated < 0) throw new TrapError("integer overflow");
    const big = BigInt(truncated);
    if (big > 18446744073709551615n) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i64(BigInt.asIntN(64, big)));
    vm.advancePc();
    return "i64.trunc_f32_u";
  });

  // 0xB0: i64.trunc_f64_s
  vm.registerContextOpcode<WasmExecutionContext>(0xb0, (vm, _instr, _code, _ctx) => {
    const value = asF64(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    if (!isFinite(value)) throw new TrapError("integer overflow");
    const truncated = Math.trunc(value);
    const big = BigInt(truncated);
    if (big < -9223372036854775808n || big > 9223372036854775807n) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i64(big));
    vm.advancePc();
    return "i64.trunc_f64_s";
  });

  // 0xB1: i64.trunc_f64_u
  vm.registerContextOpcode<WasmExecutionContext>(0xb1, (vm, _instr, _code, _ctx) => {
    const value = asF64(vm.popTyped());
    if (isNaN(value)) throw new TrapError("invalid conversion to integer");
    if (!isFinite(value)) throw new TrapError("integer overflow");
    const truncated = Math.trunc(value);
    if (truncated < 0) throw new TrapError("integer overflow");
    const big = BigInt(truncated);
    if (big > 18446744073709551615n) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i64(BigInt.asIntN(64, big)));
    vm.advancePc();
    return "i64.trunc_f64_u";
  });

  // =========================================================================
  // 0xB2 - 0xB5: f32.convert_i* --- Integer to f32
  // =========================================================================

  // 0xB2: f32.convert_i32_s
  vm.registerContextOpcode<WasmExecutionContext>(0xb2, (vm, _instr, _code, _ctx) => {
    const value = asI32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(value | 0)));
    vm.advancePc();
    return "f32.convert_i32_s";
  });

  // 0xB3: f32.convert_i32_u
  vm.registerContextOpcode<WasmExecutionContext>(0xb3, (vm, _instr, _code, _ctx) => {
    const value = asI32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(value >>> 0)));
    vm.advancePc();
    return "f32.convert_i32_u";
  });

  // 0xB4: f32.convert_i64_s
  vm.registerContextOpcode<WasmExecutionContext>(0xb4, (vm, _instr, _code, _ctx) => {
    const value = asI64(vm.popTyped());
    vm.pushTyped(f32(Math.fround(Number(BigInt.asIntN(64, value)))));
    vm.advancePc();
    return "f32.convert_i64_s";
  });

  // 0xB5: f32.convert_i64_u
  vm.registerContextOpcode<WasmExecutionContext>(0xb5, (vm, _instr, _code, _ctx) => {
    const value = asI64(vm.popTyped());
    vm.pushTyped(f32(Math.fround(Number(BigInt.asUintN(64, value)))));
    vm.advancePc();
    return "f32.convert_i64_u";
  });

  // =========================================================================
  // 0xB6: f32.demote_f64 --- Narrow f64 to f32
  // =========================================================================
  //
  // Math.fround converts from f64 precision to f32 precision. Values
  // outside the f32 range become +Infinity or -Infinity.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0xb6, (vm, _instr, _code, _ctx) => {
    const value = asF64(vm.popTyped());
    vm.pushTyped(f32(Math.fround(value)));
    vm.advancePc();
    return "f32.demote_f64";
  });

  // =========================================================================
  // 0xB7 - 0xBA: f64.convert_i* --- Integer to f64
  // =========================================================================

  // 0xB7: f64.convert_i32_s
  vm.registerContextOpcode<WasmExecutionContext>(0xb7, (vm, _instr, _code, _ctx) => {
    const value = asI32(vm.popTyped());
    vm.pushTyped(f64(value | 0));
    vm.advancePc();
    return "f64.convert_i32_s";
  });

  // 0xB8: f64.convert_i32_u
  vm.registerContextOpcode<WasmExecutionContext>(0xb8, (vm, _instr, _code, _ctx) => {
    const value = asI32(vm.popTyped());
    vm.pushTyped(f64(value >>> 0));
    vm.advancePc();
    return "f64.convert_i32_u";
  });

  // 0xB9: f64.convert_i64_s
  vm.registerContextOpcode<WasmExecutionContext>(0xb9, (vm, _instr, _code, _ctx) => {
    const value = asI64(vm.popTyped());
    vm.pushTyped(f64(Number(BigInt.asIntN(64, value))));
    vm.advancePc();
    return "f64.convert_i64_s";
  });

  // 0xBA: f64.convert_i64_u
  vm.registerContextOpcode<WasmExecutionContext>(0xba, (vm, _instr, _code, _ctx) => {
    const value = asI64(vm.popTyped());
    vm.pushTyped(f64(Number(BigInt.asUintN(64, value))));
    vm.advancePc();
    return "f64.convert_i64_u";
  });

  // =========================================================================
  // 0xBB: f64.promote_f32 --- Widen f32 to f64
  // =========================================================================
  //
  // In JavaScript, f32 values are already stored as f64 (JS number), so
  // promotion is essentially a type-tag change with no value modification.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0xbb, (vm, _instr, _code, _ctx) => {
    const value = asF32(vm.popTyped());
    vm.pushTyped(f64(value));
    vm.advancePc();
    return "f64.promote_f32";
  });

  // =========================================================================
  // 0xBC - 0xBF: Reinterpret Instructions
  // =========================================================================
  //
  // These don't change any bits --- they just reinterpret the same bit
  // pattern as a different type. Think of it as a C ``union`` read.
  //

  // 0xBC: i32.reinterpret_f32
  //
  // Write the f32 value's bytes, read them back as i32.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0xbc, (vm, _instr, _code, _ctx) => {
    const value = asF32(vm.popTyped());
    REINTERPRET_F32[0] = value;
    vm.pushTyped(i32(REINTERPRET_I32[0]));
    vm.advancePc();
    return "i32.reinterpret_f32";
  });

  // 0xBD: i64.reinterpret_f64
  vm.registerContextOpcode<WasmExecutionContext>(0xbd, (vm, _instr, _code, _ctx) => {
    const value = asF64(vm.popTyped());
    REINTERPRET_F64[0] = value;
    vm.pushTyped(i64(REINTERPRET_I64[0]));
    vm.advancePc();
    return "i64.reinterpret_f64";
  });

  // 0xBE: f32.reinterpret_i32
  vm.registerContextOpcode<WasmExecutionContext>(0xbe, (vm, _instr, _code, _ctx) => {
    const value = asI32(vm.popTyped());
    REINTERPRET_I32[0] = value;
    vm.pushTyped(f32(REINTERPRET_F32[0]));
    vm.advancePc();
    return "f32.reinterpret_i32";
  });

  // 0xBF: f64.reinterpret_i64
  vm.registerContextOpcode<WasmExecutionContext>(0xbf, (vm, _instr, _code, _ctx) => {
    const value = asI64(vm.popTyped());
    REINTERPRET_I64[0] = value;
    vm.pushTyped(f64(REINTERPRET_F64[0]));
    vm.advancePc();
    return "f64.reinterpret_i64";
  });
}
