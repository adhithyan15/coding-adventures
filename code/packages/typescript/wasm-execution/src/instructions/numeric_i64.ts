/**
 * numeric_i64.ts --- 64-bit integer instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: i64 ARITHMETIC IN WEBASSEMBLY
 * ===========================================================================
 *
 * WASM's i64 type represents 64-bit integers. In JavaScript, we use
 * ``BigInt`` to represent these values because the ``number`` type (a
 * 64-bit IEEE 754 double) can only represent integers exactly up to 2^53.
 *
 * BigInt arithmetic in JavaScript is arbitrary-precision, so we must
 * explicitly truncate results to 64 bits after each operation. The key
 * functions are:
 *
 *   - ``BigInt.asIntN(64, value)``  --- Interpret as signed 64-bit (wrapping).
 *   - ``BigInt.asUintN(64, value)`` --- Interpret as unsigned 64-bit.
 *
 * These are analogous to ``| 0`` and ``>>> 0`` for i32, but for BigInts.
 *
 * ===========================================================================
 * COMPARISON RESULTS ARE i32, NOT i64!
 * ===========================================================================
 *
 * A subtle but critical point: comparison instructions (eq, ne, lt, etc.)
 * always return an i32 value (0 or 1), even when comparing i64 operands.
 * This matches the WASM spec and is because WASM uses i32 as its boolean
 * type --- ``if``, ``br_if``, and ``select`` all consume i32 conditions.
 *
 * ===========================================================================
 * INSTRUCTION MAP
 * ===========================================================================
 *
 *   Opcode   Mnemonic        Stack Effect
 *   ------   --------        ------------
 *   0x42     i64.const       [] -> [i64]
 *   0x50     i64.eqz         [i64] -> [i32]       (note: result is i32!)
 *   0x51     i64.eq          [i64 i64] -> [i32]
 *   0x52     i64.ne          [i64 i64] -> [i32]
 *   0x53     i64.lt_s        [i64 i64] -> [i32]
 *   0x54     i64.lt_u        [i64 i64] -> [i32]
 *   0x55     i64.gt_s        [i64 i64] -> [i32]
 *   0x56     i64.gt_u        [i64 i64] -> [i32]
 *   0x57     i64.le_s        [i64 i64] -> [i32]
 *   0x58     i64.le_u        [i64 i64] -> [i32]
 *   0x59     i64.ge_s        [i64 i64] -> [i32]
 *   0x5A     i64.ge_u        [i64 i64] -> [i32]
 *   0x79     i64.clz         [i64] -> [i64]
 *   0x7A     i64.ctz         [i64] -> [i64]
 *   0x7B     i64.popcnt      [i64] -> [i64]
 *   0x7C     i64.add         [i64 i64] -> [i64]
 *   0x7D     i64.sub         [i64 i64] -> [i64]
 *   0x7E     i64.mul         [i64 i64] -> [i64]
 *   0x7F     i64.div_s       [i64 i64] -> [i64]   (traps!)
 *   0x80     i64.div_u       [i64 i64] -> [i64]   (traps!)
 *   0x81     i64.rem_s       [i64 i64] -> [i64]   (traps!)
 *   0x82     i64.rem_u       [i64 i64] -> [i64]   (traps!)
 *   0x83     i64.and         [i64 i64] -> [i64]
 *   0x84     i64.or          [i64 i64] -> [i64]
 *   0x85     i64.xor         [i64 i64] -> [i64]
 *   0x86     i64.shl         [i64 i64] -> [i64]
 *   0x87     i64.shr_s       [i64 i64] -> [i64]
 *   0x88     i64.shr_u       [i64 i64] -> [i64]
 *   0x89     i64.rotl        [i64 i64] -> [i64]
 *   0x8A     i64.rotr        [i64 i64] -> [i64]
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { TrapError } from "../host_interface.js";
import { i32, i64, asI64 } from "../values.js";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Constants
// ===========================================================================

/** Minimum signed 64-bit integer. */
const MIN_I64 = -9223372036854775808n;

// ===========================================================================
// Helpers for 64-bit Bit Operations
// ===========================================================================

/**
 * Count leading zeros in a 64-bit BigInt.
 *
 * Unlike i32, there's no ``Math.clz64`` built-in. We check the upper
 * 32 bits first; if they're all zero, the answer is 32 + clz of lower 32.
 */
function clz64(value: bigint): bigint {
  const v = BigInt.asUintN(64, value);
  if (v === 0n) return 64n;
  let count = 0n;
  let remaining = v;
  /* Shift right and count until we find the highest set bit. */
  for (let bit = 63n; bit >= 0n; bit--) {
    if ((remaining >> bit) & 1n) {
      return count;
    }
    count++;
  }
  return 64n; /* Should not reach here, but safety fallback. */
}

/**
 * Count trailing zeros in a 64-bit BigInt.
 */
function ctz64(value: bigint): bigint {
  const v = BigInt.asUintN(64, value);
  if (v === 0n) return 64n;
  let count = 0n;
  let remaining = v;
  while ((remaining & 1n) === 0n) {
    count++;
    remaining >>= 1n;
  }
  return count;
}

/**
 * Population count (number of 1-bits) in a 64-bit BigInt.
 */
function popcnt64(value: bigint): bigint {
  let v = BigInt.asUintN(64, value);
  let count = 0n;
  while (v !== 0n) {
    count += v & 1n;
    v >>= 1n;
  }
  return count;
}

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register all 32 i64 numeric instruction handlers on the given GenericVM.
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerNumericI64(vm: GenericVM): void {

  // =========================================================================
  // 0x42: i64.const --- Push an i64 constant
  // =========================================================================
  vm.registerContextOpcode<WasmExecutionContext>(0x42, (vm, instr, _code, _ctx) => {
    vm.pushTyped(i64(BigInt(instr.operand as number | bigint)));
    vm.advancePc();
    return "i64.const";
  });

  // =========================================================================
  // 0x50: i64.eqz --- Test if zero (result is i32!)
  // =========================================================================
  vm.registerContextOpcode<WasmExecutionContext>(0x50, (vm, _instr, _code, _ctx) => {
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(a === 0n ? 1 : 0));
    vm.advancePc();
    return "i64.eqz";
  });

  // =========================================================================
  // 0x51 - 0x5A: Comparison Operations (all return i32!)
  // =========================================================================

  // 0x51: i64.eq
  vm.registerContextOpcode<WasmExecutionContext>(0x51, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(a === b ? 1 : 0));
    vm.advancePc();
    return "i64.eq";
  });

  // 0x52: i64.ne
  vm.registerContextOpcode<WasmExecutionContext>(0x52, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(a !== b ? 1 : 0));
    vm.advancePc();
    return "i64.ne";
  });

  // 0x53: i64.lt_s --- Less than (signed)
  //
  // BigInt.asIntN(64, v) gives the signed interpretation. BigInt comparison
  // operators already work correctly for signed comparison, but we use
  // asIntN to ensure the value is in the signed range first.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x53, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asIntN(64, a) < BigInt.asIntN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.lt_s";
  });

  // 0x54: i64.lt_u --- Less than (unsigned)
  vm.registerContextOpcode<WasmExecutionContext>(0x54, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asUintN(64, a) < BigInt.asUintN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.lt_u";
  });

  // 0x55: i64.gt_s
  vm.registerContextOpcode<WasmExecutionContext>(0x55, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asIntN(64, a) > BigInt.asIntN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.gt_s";
  });

  // 0x56: i64.gt_u
  vm.registerContextOpcode<WasmExecutionContext>(0x56, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asUintN(64, a) > BigInt.asUintN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.gt_u";
  });

  // 0x57: i64.le_s
  vm.registerContextOpcode<WasmExecutionContext>(0x57, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asIntN(64, a) <= BigInt.asIntN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.le_s";
  });

  // 0x58: i64.le_u
  vm.registerContextOpcode<WasmExecutionContext>(0x58, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asUintN(64, a) <= BigInt.asUintN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.le_u";
  });

  // 0x59: i64.ge_s
  vm.registerContextOpcode<WasmExecutionContext>(0x59, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asIntN(64, a) >= BigInt.asIntN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.ge_s";
  });

  // 0x5A: i64.ge_u
  vm.registerContextOpcode<WasmExecutionContext>(0x5a, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i32(BigInt.asUintN(64, a) >= BigInt.asUintN(64, b) ? 1 : 0));
    vm.advancePc();
    return "i64.ge_u";
  });

  // =========================================================================
  // 0x79 - 0x7B: Unary Bit Operations
  // =========================================================================

  // 0x79: i64.clz
  vm.registerContextOpcode<WasmExecutionContext>(0x79, (vm, _instr, _code, _ctx) => {
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(clz64(a)));
    vm.advancePc();
    return "i64.clz";
  });

  // 0x7A: i64.ctz
  vm.registerContextOpcode<WasmExecutionContext>(0x7a, (vm, _instr, _code, _ctx) => {
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(ctz64(a)));
    vm.advancePc();
    return "i64.ctz";
  });

  // 0x7B: i64.popcnt
  vm.registerContextOpcode<WasmExecutionContext>(0x7b, (vm, _instr, _code, _ctx) => {
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(popcnt64(a)));
    vm.advancePc();
    return "i64.popcnt";
  });

  // =========================================================================
  // 0x7C - 0x82: Arithmetic Operations
  // =========================================================================

  // 0x7C: i64.add
  vm.registerContextOpcode<WasmExecutionContext>(0x7c, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(BigInt.asIntN(64, a + b)));
    vm.advancePc();
    return "i64.add";
  });

  // 0x7D: i64.sub
  vm.registerContextOpcode<WasmExecutionContext>(0x7d, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(BigInt.asIntN(64, a - b)));
    vm.advancePc();
    return "i64.sub";
  });

  // 0x7E: i64.mul
  vm.registerContextOpcode<WasmExecutionContext>(0x7e, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(BigInt.asIntN(64, a * b)));
    vm.advancePc();
    return "i64.mul";
  });

  // 0x7F: i64.div_s --- Signed division (trapping)
  vm.registerContextOpcode<WasmExecutionContext>(0x7f, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    if (b === 0n) {
      throw new TrapError("integer divide by zero");
    }
    const sa = BigInt.asIntN(64, a);
    const sb = BigInt.asIntN(64, b);
    if (sa === MIN_I64 && sb === -1n) {
      throw new TrapError("integer overflow");
    }
    vm.pushTyped(i64(BigInt.asIntN(64, sa / sb)));
    vm.advancePc();
    return "i64.div_s";
  });

  // 0x80: i64.div_u --- Unsigned division (trapping)
  vm.registerContextOpcode<WasmExecutionContext>(0x80, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    const ub = BigInt.asUintN(64, b);
    if (ub === 0n) {
      throw new TrapError("integer divide by zero");
    }
    const ua = BigInt.asUintN(64, a);
    vm.pushTyped(i64(BigInt.asIntN(64, ua / ub)));
    vm.advancePc();
    return "i64.div_u";
  });

  // 0x81: i64.rem_s --- Signed remainder (trapping on zero)
  vm.registerContextOpcode<WasmExecutionContext>(0x81, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    if (b === 0n) {
      throw new TrapError("integer divide by zero");
    }
    const sa = BigInt.asIntN(64, a);
    const sb = BigInt.asIntN(64, b);
    /* MIN_I64 % -1 = 0 (no trap, unlike div_s). */
    if (sa === MIN_I64 && sb === -1n) {
      vm.pushTyped(i64(0n));
    } else {
      vm.pushTyped(i64(BigInt.asIntN(64, sa % sb)));
    }
    vm.advancePc();
    return "i64.rem_s";
  });

  // 0x82: i64.rem_u --- Unsigned remainder (trapping on zero)
  vm.registerContextOpcode<WasmExecutionContext>(0x82, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    const ub = BigInt.asUintN(64, b);
    if (ub === 0n) {
      throw new TrapError("integer divide by zero");
    }
    const ua = BigInt.asUintN(64, a);
    vm.pushTyped(i64(BigInt.asIntN(64, ua % ub)));
    vm.advancePc();
    return "i64.rem_u";
  });

  // =========================================================================
  // 0x83 - 0x85: Bitwise Logic
  // =========================================================================

  // 0x83: i64.and
  vm.registerContextOpcode<WasmExecutionContext>(0x83, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(BigInt.asIntN(64, a & b)));
    vm.advancePc();
    return "i64.and";
  });

  // 0x84: i64.or
  vm.registerContextOpcode<WasmExecutionContext>(0x84, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(BigInt.asIntN(64, a | b)));
    vm.advancePc();
    return "i64.or";
  });

  // 0x85: i64.xor
  vm.registerContextOpcode<WasmExecutionContext>(0x85, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    vm.pushTyped(i64(BigInt.asIntN(64, a ^ b)));
    vm.advancePc();
    return "i64.xor";
  });

  // =========================================================================
  // 0x86 - 0x8A: Shift and Rotate
  // =========================================================================
  //
  // Shift amounts are masked to 6 bits (mod 64), matching i32's mod 32.
  //

  // 0x86: i64.shl
  vm.registerContextOpcode<WasmExecutionContext>(0x86, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    const n = BigInt.asUintN(64, b) & 63n;
    vm.pushTyped(i64(BigInt.asIntN(64, a << n)));
    vm.advancePc();
    return "i64.shl";
  });

  // 0x87: i64.shr_s --- Arithmetic shift right
  vm.registerContextOpcode<WasmExecutionContext>(0x87, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    const n = BigInt.asUintN(64, b) & 63n;
    vm.pushTyped(i64(BigInt.asIntN(64, BigInt.asIntN(64, a) >> n)));
    vm.advancePc();
    return "i64.shr_s";
  });

  // 0x88: i64.shr_u --- Logical shift right
  vm.registerContextOpcode<WasmExecutionContext>(0x88, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    const n = BigInt.asUintN(64, b) & 63n;
    vm.pushTyped(i64(BigInt.asIntN(64, BigInt.asUintN(64, a) >> n)));
    vm.advancePc();
    return "i64.shr_u";
  });

  // 0x89: i64.rotl --- Rotate left
  vm.registerContextOpcode<WasmExecutionContext>(0x89, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    const n = BigInt.asUintN(64, b) & 63n;
    const ua = BigInt.asUintN(64, a);
    const result = (ua << n) | (ua >> (64n - n));
    vm.pushTyped(i64(BigInt.asIntN(64, result)));
    vm.advancePc();
    return "i64.rotl";
  });

  // 0x8A: i64.rotr --- Rotate right
  vm.registerContextOpcode<WasmExecutionContext>(0x8a, (vm, _instr, _code, _ctx) => {
    const b = asI64(vm.popTyped());
    const a = asI64(vm.popTyped());
    const n = BigInt.asUintN(64, b) & 63n;
    const ua = BigInt.asUintN(64, a);
    const result = (ua >> n) | (ua << (64n - n));
    vm.pushTyped(i64(BigInt.asIntN(64, result)));
    vm.advancePc();
    return "i64.rotr";
  });
}
