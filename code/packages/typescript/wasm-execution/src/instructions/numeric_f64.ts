/**
 * numeric_f64.ts --- 64-bit floating-point instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: f64 IN WEBASSEMBLY
 * ===========================================================================
 *
 * WASM f64 is IEEE 754 double-precision (64-bit) floating point --- the
 * same as JavaScript's ``number`` type. This means no precision conversion
 * is needed (unlike f32 which requires ``Math.fround()``). However, we
 * still need to handle the same special cases: NaN propagation, signed
 * zeros in min/max, and banker's rounding for nearest.
 *
 * ===========================================================================
 * INSTRUCTION MAP (23 instructions)
 * ===========================================================================
 *
 *   Opcode  Mnemonic         Stack Effect
 *   ------  --------         ------------
 *   0x44    f64.const        [] -> [f64]
 *   0x61    f64.eq           [f64 f64] -> [i32]
 *   0x62    f64.ne           [f64 f64] -> [i32]
 *   0x63    f64.lt           [f64 f64] -> [i32]
 *   0x64    f64.gt           [f64 f64] -> [i32]
 *   0x65    f64.le           [f64 f64] -> [i32]
 *   0x66    f64.ge           [f64 f64] -> [i32]
 *   0x99    f64.abs          [f64] -> [f64]
 *   0x9A    f64.neg          [f64] -> [f64]
 *   0x9B    f64.ceil         [f64] -> [f64]
 *   0x9C    f64.floor        [f64] -> [f64]
 *   0x9D    f64.trunc        [f64] -> [f64]
 *   0x9E    f64.nearest      [f64] -> [f64]
 *   0x9F    f64.sqrt         [f64] -> [f64]
 *   0xA0    f64.add          [f64 f64] -> [f64]
 *   0xA1    f64.sub          [f64 f64] -> [f64]
 *   0xA2    f64.mul          [f64 f64] -> [f64]
 *   0xA3    f64.div          [f64 f64] -> [f64]
 *   0xA4    f64.min          [f64 f64] -> [f64]
 *   0xA5    f64.max          [f64 f64] -> [f64]
 *   0xA6    f64.copysign     [f64 f64] -> [f64]
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { i32, f64, asF64 } from "../values.js";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Helper: Banker's Rounding for f64
// ===========================================================================

/**
 * Round to nearest integer using "round half to even" (banker's rounding).
 *
 * Same logic as the f32 version, but without Math.fround since f64 is
 * native JS precision.
 */
function nearestF64(v: number): number {
  if (!isFinite(v)) return v;
  if (v === 0) return v;

  const floor = Math.floor(v);
  const frac = v - floor;

  if (frac === 0.5) {
    return floor % 2 === 0 ? floor : floor + 1;
  }

  return Math.round(v);
}

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register all 23 f64 instruction handlers on the given GenericVM.
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerNumericF64(vm: GenericVM): void {

  // 0x44: f64.const
  vm.registerContextOpcode<WasmExecutionContext>(0x44, (vm, instr, _code, _ctx) => {
    vm.pushTyped(f64(instr.operand as number));
    vm.advancePc();
    return "f64.const";
  });

  // =========================================================================
  // 0x61 - 0x66: Comparison Operations (return i32)
  // =========================================================================

  // 0x61: f64.eq
  vm.registerContextOpcode<WasmExecutionContext>(0x61, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(i32(a === b ? 1 : 0));
    vm.advancePc();
    return "f64.eq";
  });

  // 0x62: f64.ne
  vm.registerContextOpcode<WasmExecutionContext>(0x62, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(i32(a !== b ? 1 : 0));
    vm.advancePc();
    return "f64.ne";
  });

  // 0x63: f64.lt
  vm.registerContextOpcode<WasmExecutionContext>(0x63, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(i32(a < b ? 1 : 0));
    vm.advancePc();
    return "f64.lt";
  });

  // 0x64: f64.gt
  vm.registerContextOpcode<WasmExecutionContext>(0x64, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(i32(a > b ? 1 : 0));
    vm.advancePc();
    return "f64.gt";
  });

  // 0x65: f64.le
  vm.registerContextOpcode<WasmExecutionContext>(0x65, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(i32(a <= b ? 1 : 0));
    vm.advancePc();
    return "f64.le";
  });

  // 0x66: f64.ge
  vm.registerContextOpcode<WasmExecutionContext>(0x66, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(i32(a >= b ? 1 : 0));
    vm.advancePc();
    return "f64.ge";
  });

  // =========================================================================
  // 0x99 - 0x9F: Unary Operations
  // =========================================================================

  // 0x99: f64.abs
  vm.registerContextOpcode<WasmExecutionContext>(0x99, (vm, _instr, _code, _ctx) => {
    vm.pushTyped(f64(Math.abs(asF64(vm.popTyped()))));
    vm.advancePc();
    return "f64.abs";
  });

  // 0x9A: f64.neg
  vm.registerContextOpcode<WasmExecutionContext>(0x9a, (vm, _instr, _code, _ctx) => {
    vm.pushTyped(f64(-asF64(vm.popTyped())));
    vm.advancePc();
    return "f64.neg";
  });

  // 0x9B: f64.ceil
  vm.registerContextOpcode<WasmExecutionContext>(0x9b, (vm, _instr, _code, _ctx) => {
    vm.pushTyped(f64(Math.ceil(asF64(vm.popTyped()))));
    vm.advancePc();
    return "f64.ceil";
  });

  // 0x9C: f64.floor
  vm.registerContextOpcode<WasmExecutionContext>(0x9c, (vm, _instr, _code, _ctx) => {
    vm.pushTyped(f64(Math.floor(asF64(vm.popTyped()))));
    vm.advancePc();
    return "f64.floor";
  });

  // 0x9D: f64.trunc
  vm.registerContextOpcode<WasmExecutionContext>(0x9d, (vm, _instr, _code, _ctx) => {
    vm.pushTyped(f64(Math.trunc(asF64(vm.popTyped()))));
    vm.advancePc();
    return "f64.trunc";
  });

  // 0x9E: f64.nearest --- Banker's rounding
  vm.registerContextOpcode<WasmExecutionContext>(0x9e, (vm, _instr, _code, _ctx) => {
    vm.pushTyped(f64(nearestF64(asF64(vm.popTyped()))));
    vm.advancePc();
    return "f64.nearest";
  });

  // 0x9F: f64.sqrt
  vm.registerContextOpcode<WasmExecutionContext>(0x9f, (vm, _instr, _code, _ctx) => {
    vm.pushTyped(f64(Math.sqrt(asF64(vm.popTyped()))));
    vm.advancePc();
    return "f64.sqrt";
  });

  // =========================================================================
  // 0xA0 - 0xA6: Binary Operations
  // =========================================================================

  // 0xA0: f64.add
  vm.registerContextOpcode<WasmExecutionContext>(0xa0, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(f64(a + b));
    vm.advancePc();
    return "f64.add";
  });

  // 0xA1: f64.sub
  vm.registerContextOpcode<WasmExecutionContext>(0xa1, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(f64(a - b));
    vm.advancePc();
    return "f64.sub";
  });

  // 0xA2: f64.mul
  vm.registerContextOpcode<WasmExecutionContext>(0xa2, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(f64(a * b));
    vm.advancePc();
    return "f64.mul";
  });

  // 0xA3: f64.div
  vm.registerContextOpcode<WasmExecutionContext>(0xa3, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());
    vm.pushTyped(f64(a / b));
    vm.advancePc();
    return "f64.div";
  });

  // 0xA4: f64.min
  vm.registerContextOpcode<WasmExecutionContext>(0xa4, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());

    let result: number;
    if (isNaN(a) || isNaN(b)) {
      result = NaN;
    } else if (a === 0 && b === 0) {
      result = (1 / a === -Infinity || 1 / b === -Infinity) ? -0 : 0;
    } else {
      result = Math.min(a, b);
    }

    vm.pushTyped(f64(result));
    vm.advancePc();
    return "f64.min";
  });

  // 0xA5: f64.max
  vm.registerContextOpcode<WasmExecutionContext>(0xa5, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());

    let result: number;
    if (isNaN(a) || isNaN(b)) {
      result = NaN;
    } else if (a === 0 && b === 0) {
      result = (1 / a === Infinity || 1 / b === Infinity) ? 0 : -0;
    } else {
      result = Math.max(a, b);
    }

    vm.pushTyped(f64(result));
    vm.advancePc();
    return "f64.max";
  });

  // 0xA6: f64.copysign
  vm.registerContextOpcode<WasmExecutionContext>(0xa6, (vm, _instr, _code, _ctx) => {
    const b = asF64(vm.popTyped());
    const a = asF64(vm.popTyped());

    const signB = Object.is(b, -0) || b < 0 ? -1 : 1;
    const magnitude = Math.abs(a);
    const result = signB < 0 ? -magnitude : magnitude;

    vm.pushTyped(f64(result));
    vm.advancePc();
    return "f64.copysign";
  });
}
