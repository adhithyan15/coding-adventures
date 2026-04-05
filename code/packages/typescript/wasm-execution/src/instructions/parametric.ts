/**
 * parametric.ts --- Parametric instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: PARAMETRIC INSTRUCTIONS
 * ===========================================================================
 *
 * WASM has two "parametric" instructions that work with values of any type:
 *
 * 1. **drop** --- Discard the top value from the stack. This is like a
 *    "void" cast --- you computed something but don't need the result.
 *
 * 2. **select** --- A ternary operator. Pops three values: a condition (i32),
 *    and two values of the same type. Pushes val1 if the condition is
 *    non-zero, val2 if zero. This is WASM's equivalent of C's ternary
 *    operator ``condition ? val1 : val2``.
 *
 * These are called "parametric" because they are *type-polymorphic* ---
 * they work with i32, i64, f32, or f64 values without separate opcodes
 * for each type.
 *
 * ===========================================================================
 * INSTRUCTION MAP (2 instructions)
 * ===========================================================================
 *
 *   Opcode  Mnemonic   Stack Effect
 *   ------  --------   ------------
 *   0x1A    drop       [any] -> []
 *   0x1B    select     [any any i32] -> [any]
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register the 2 parametric instruction handlers.
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerParametric(vm: GenericVM): void {

  // =========================================================================
  // 0x1A: drop --- Discard the top stack value
  // =========================================================================
  //
  // Simply pops one value and throws it away. Used when a function returns
  // a value but the caller doesn't need it.
  //
  //   call $compute    ;; pushes a result
  //   drop             ;; discard it
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x1a, (vm, _instr, _code, _ctx) => {
    vm.popTyped();
    vm.advancePc();
    return "drop";
  });

  // =========================================================================
  // 0x1B: select --- Conditional pick
  // =========================================================================
  //
  // Pop order (top to bottom): condition (i32), val2, val1.
  //   - If condition is non-zero, push val1.
  //   - If condition is zero, push val2.
  //
  // This is a branchless conditional --- both values are always computed
  // before the select executes. Useful for simple conditional expressions
  // without the overhead of a branch.
  //
  //   i32.const 10      ;; val1
  //   i32.const 20      ;; val2
  //   local.get $flag   ;; condition
  //   select            ;; pushes 10 if flag != 0, else 20
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x1b, (vm, _instr, _code, _ctx) => {
    const condition = vm.popTyped();   /* i32 condition */
    const val2 = vm.popTyped();        /* second value (chosen when c == 0) */
    const val1 = vm.popTyped();        /* first value (chosen when c != 0) */
    vm.pushTyped((condition.value as number) !== 0 ? val1 : val2);
    vm.advancePc();
    return "select";
  });
}
