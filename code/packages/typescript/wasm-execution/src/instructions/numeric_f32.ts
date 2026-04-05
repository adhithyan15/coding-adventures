/**
 * numeric_f32.ts --- 32-bit floating-point instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: f32 IN WEBASSEMBLY
 * ===========================================================================
 *
 * WASM f32 is IEEE 754 single-precision (32-bit) floating point. JavaScript
 * numbers are 64-bit doubles, so we must use ``Math.fround()`` to round
 * every result back to 32-bit precision. Without this, intermediate results
 * would carry extra precision that real WASM wouldn't have.
 *
 *   Math.fround(0.1 + 0.2)  !== 0.1 + 0.2   (different precision!)
 *
 * ===========================================================================
 * SPECIAL CASES: NaN AND SIGNED ZEROS
 * ===========================================================================
 *
 * IEEE 754 has two special concerns that WASM instructions must handle:
 *
 * 1. **NaN (Not a Number)**: If either operand to min/max is NaN, the
 *    result must be NaN. This differs from ``Math.min``/``Math.max``
 *    which return NaN correctly, but we need explicit checks for the
 *    signed-zero cases.
 *
 * 2. **Signed zeros**: IEEE 754 has both +0 and -0.
 *    - ``f32.min(+0, -0)`` must return -0 (the more negative).
 *    - ``f32.max(+0, -0)`` must return +0 (the more positive).
 *    JavaScript's ``Math.min(0, -0)`` returns -0 correctly, but
 *    ``Math.max(-0, 0)`` also returns 0 correctly. We handle them
 *    explicitly to be safe.
 *
 * ===========================================================================
 * BANKER'S ROUNDING (f32.nearest)
 * ===========================================================================
 *
 * WASM's ``nearest`` instruction uses "round half to even" (also called
 * banker's rounding). When a value is exactly halfway between two integers,
 * it rounds to the nearest EVEN integer:
 *
 *   nearest(0.5)  = 0   (rounds to even: 0)
 *   nearest(1.5)  = 2   (rounds to even: 2)
 *   nearest(2.5)  = 2   (rounds to even: 2)
 *   nearest(3.5)  = 4   (rounds to even: 4)
 *   nearest(-0.5) = -0  (rounds to even: 0, preserves sign)
 *
 * JavaScript's ``Math.round()`` rounds 0.5 UP (to 1), which is wrong for
 * WASM. We must implement banker's rounding manually.
 *
 * ===========================================================================
 * INSTRUCTION MAP (23 instructions)
 * ===========================================================================
 *
 *   Opcode  Mnemonic         Stack Effect
 *   ------  --------         ------------
 *   0x43    f32.const        [] -> [f32]
 *   0x5B    f32.eq           [f32 f32] -> [i32]
 *   0x5C    f32.ne           [f32 f32] -> [i32]
 *   0x5D    f32.lt           [f32 f32] -> [i32]
 *   0x5E    f32.gt           [f32 f32] -> [i32]
 *   0x5F    f32.le           [f32 f32] -> [i32]
 *   0x60    f32.ge           [f32 f32] -> [i32]
 *   0x8B    f32.abs          [f32] -> [f32]
 *   0x8C    f32.neg          [f32] -> [f32]
 *   0x8D    f32.ceil         [f32] -> [f32]
 *   0x8E    f32.floor        [f32] -> [f32]
 *   0x8F    f32.trunc        [f32] -> [f32]
 *   0x90    f32.nearest      [f32] -> [f32]
 *   0x91    f32.sqrt         [f32] -> [f32]
 *   0x92    f32.add          [f32 f32] -> [f32]
 *   0x93    f32.sub          [f32 f32] -> [f32]
 *   0x94    f32.mul          [f32 f32] -> [f32]
 *   0x95    f32.div          [f32 f32] -> [f32]
 *   0x96    f32.min          [f32 f32] -> [f32]
 *   0x97    f32.max          [f32 f32] -> [f32]
 *   0x98    f32.copysign     [f32 f32] -> [f32]
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { i32, f32, asF32 } from "../values.js";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Helper: Banker's Rounding for f32
// ===========================================================================

/**
 * Round a number to the nearest integer using "round half to even"
 * (banker's rounding), then apply Math.fround for f32 precision.
 *
 * When the fractional part is exactly 0.5, round to the nearest EVEN
 * integer. This eliminates the statistical bias that "round half up"
 * (JavaScript's Math.round) introduces.
 */
function nearestF32(v: number): number {
  if (!isFinite(v)) return v; /* NaN, +Inf, -Inf pass through. */
  if (v === 0) return v;      /* Preserve -0 and +0. */

  const floor = Math.floor(v);
  const frac = v - floor;

  /* If the fractional part is exactly 0.5, round to even. */
  if (frac === 0.5) {
    /* floor is the integer below. If it's even, use it; otherwise use floor+1. */
    return Math.fround(floor % 2 === 0 ? floor : floor + 1);
  }
  if (frac === -0.5) {
    /* For negative half-integers (e.g., -1.5 -> floor=-2, frac=0.5).
       Actually for negative numbers, floor is more negative, so frac is
       always positive. This branch won't trigger for normal cases. */
    return Math.fround(floor % 2 === 0 ? floor : floor + 1);
  }

  /* Standard rounding for non-halfway cases. Math.round rounds 0.5 up
     which is fine here because we've already handled the 0.5 case. */
  return Math.fround(Math.round(v));
}

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register all 23 f32 instruction handlers on the given GenericVM.
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerNumericF32(vm: GenericVM): void {

  // =========================================================================
  // 0x43: f32.const --- Push an f32 constant
  // =========================================================================
  vm.registerContextOpcode<WasmExecutionContext>(0x43, (vm, instr, _code, _ctx) => {
    vm.pushTyped(f32(instr.operand as number));
    vm.advancePc();
    return "f32.const";
  });

  // =========================================================================
  // 0x5B - 0x60: Comparison Operations (return i32)
  // =========================================================================
  //
  // IEEE 754 comparisons: NaN is not equal to anything (including itself).
  //   NaN == NaN  -> false
  //   NaN < x     -> false
  //   NaN > x     -> false
  // JavaScript's comparison operators handle this correctly.
  //

  // 0x5B: f32.eq
  vm.registerContextOpcode<WasmExecutionContext>(0x5b, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(i32(a === b ? 1 : 0));
    vm.advancePc();
    return "f32.eq";
  });

  // 0x5C: f32.ne
  vm.registerContextOpcode<WasmExecutionContext>(0x5c, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(i32(a !== b ? 1 : 0));
    vm.advancePc();
    return "f32.ne";
  });

  // 0x5D: f32.lt
  vm.registerContextOpcode<WasmExecutionContext>(0x5d, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(i32(a < b ? 1 : 0));
    vm.advancePc();
    return "f32.lt";
  });

  // 0x5E: f32.gt
  vm.registerContextOpcode<WasmExecutionContext>(0x5e, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(i32(a > b ? 1 : 0));
    vm.advancePc();
    return "f32.gt";
  });

  // 0x5F: f32.le
  vm.registerContextOpcode<WasmExecutionContext>(0x5f, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(i32(a <= b ? 1 : 0));
    vm.advancePc();
    return "f32.le";
  });

  // 0x60: f32.ge
  vm.registerContextOpcode<WasmExecutionContext>(0x60, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(i32(a >= b ? 1 : 0));
    vm.advancePc();
    return "f32.ge";
  });

  // =========================================================================
  // 0x8B - 0x91: Unary Operations
  // =========================================================================

  // 0x8B: f32.abs
  vm.registerContextOpcode<WasmExecutionContext>(0x8b, (vm, _instr, _code, _ctx) => {
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(Math.abs(a))));
    vm.advancePc();
    return "f32.abs";
  });

  // 0x8C: f32.neg
  vm.registerContextOpcode<WasmExecutionContext>(0x8c, (vm, _instr, _code, _ctx) => {
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(-a)));
    vm.advancePc();
    return "f32.neg";
  });

  // 0x8D: f32.ceil
  vm.registerContextOpcode<WasmExecutionContext>(0x8d, (vm, _instr, _code, _ctx) => {
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(Math.ceil(a))));
    vm.advancePc();
    return "f32.ceil";
  });

  // 0x8E: f32.floor
  vm.registerContextOpcode<WasmExecutionContext>(0x8e, (vm, _instr, _code, _ctx) => {
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(Math.floor(a))));
    vm.advancePc();
    return "f32.floor";
  });

  // 0x8F: f32.trunc --- Round toward zero
  vm.registerContextOpcode<WasmExecutionContext>(0x8f, (vm, _instr, _code, _ctx) => {
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(Math.trunc(a))));
    vm.advancePc();
    return "f32.trunc";
  });

  // 0x90: f32.nearest --- Banker's rounding
  vm.registerContextOpcode<WasmExecutionContext>(0x90, (vm, _instr, _code, _ctx) => {
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(nearestF32(a)));
    vm.advancePc();
    return "f32.nearest";
  });

  // 0x91: f32.sqrt
  vm.registerContextOpcode<WasmExecutionContext>(0x91, (vm, _instr, _code, _ctx) => {
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(Math.sqrt(a))));
    vm.advancePc();
    return "f32.sqrt";
  });

  // =========================================================================
  // 0x92 - 0x98: Binary Operations
  // =========================================================================

  // 0x92: f32.add
  vm.registerContextOpcode<WasmExecutionContext>(0x92, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(a + b)));
    vm.advancePc();
    return "f32.add";
  });

  // 0x93: f32.sub
  vm.registerContextOpcode<WasmExecutionContext>(0x93, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(a - b)));
    vm.advancePc();
    return "f32.sub";
  });

  // 0x94: f32.mul
  vm.registerContextOpcode<WasmExecutionContext>(0x94, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(a * b)));
    vm.advancePc();
    return "f32.mul";
  });

  // 0x95: f32.div
  //
  // Unlike integer division, float division by zero does NOT trap.
  // Instead, it returns +Infinity, -Infinity, or NaN per IEEE 754.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x95, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());
    vm.pushTyped(f32(Math.fround(a / b)));
    vm.advancePc();
    return "f32.div";
  });

  // 0x96: f32.min --- Minimum with NaN and signed-zero handling
  //
  // WASM spec requires:
  //   - If either operand is NaN, return NaN.
  //   - min(+0, -0) = min(-0, +0) = -0
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x96, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());

    let result: number;
    if (isNaN(a) || isNaN(b)) {
      result = NaN;
    } else if (a === 0 && b === 0) {
      /* Both are zero --- return -0 if either is negative zero. */
      result = (1 / a === -Infinity || 1 / b === -Infinity) ? -0 : 0;
    } else {
      result = Math.min(a, b);
    }

    vm.pushTyped(f32(Math.fround(result)));
    vm.advancePc();
    return "f32.min";
  });

  // 0x97: f32.max --- Maximum with NaN and signed-zero handling
  //
  // WASM spec requires:
  //   - If either operand is NaN, return NaN.
  //   - max(+0, -0) = max(-0, +0) = +0
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x97, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());

    let result: number;
    if (isNaN(a) || isNaN(b)) {
      result = NaN;
    } else if (a === 0 && b === 0) {
      /* Both are zero --- return +0 if either is positive zero. */
      result = (1 / a === Infinity || 1 / b === Infinity) ? 0 : -0;
    } else {
      result = Math.max(a, b);
    }

    vm.pushTyped(f32(Math.fround(result)));
    vm.advancePc();
    return "f32.max";
  });

  // 0x98: f32.copysign --- Copy sign of b to magnitude of a
  //
  // Returns a value with the magnitude of ``a`` and the sign of ``b``.
  // This is useful for implementing absolute value, sign transfer, etc.
  //
  //   copysign(3.0, -1.0)  = -3.0
  //   copysign(-3.0, 1.0)  = 3.0
  //   copysign(NaN, -1.0)  = -NaN  (yes, NaN can carry a sign bit!)
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x98, (vm, _instr, _code, _ctx) => {
    const b = asF32(vm.popTyped());
    const a = asF32(vm.popTyped());

    /* Extract sign of b using the sign bit. 1/x trick:
       1/+0 = +Inf, 1/-0 = -Inf. But for NaN, use Object.is. */
    const signB = Object.is(b, -0) || b < 0 ? -1 : 1;
    const magnitude = Math.abs(a);
    const result = signB < 0 ? -magnitude : magnitude;

    vm.pushTyped(f32(Math.fround(result)));
    vm.advancePc();
    return "f32.copysign";
  });
}
