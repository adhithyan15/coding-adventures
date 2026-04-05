/**
 * variable.ts --- Variable access instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: VARIABLES IN WEBASSEMBLY
 * ===========================================================================
 *
 * WASM has two kinds of variables:
 *
 * 1. **Local variables** --- scoped to a single function invocation.
 *    Locals include both the function's parameters (which are passed by
 *    value) and any declared local variables. They are indexed starting
 *    from 0, with parameters first:
 *
 *      (func (param i32) (param i64) (local f32)
 *        ;; local 0 = first param (i32)
 *        ;; local 1 = second param (i64)
 *        ;; local 2 = declared local (f32), initialized to 0
 *      )
 *
 * 2. **Global variables** --- module-level variables shared across all
 *    functions. Globals can be mutable or immutable, and are accessed
 *    by index. Imported globals are numbered before module-defined globals.
 *
 * ===========================================================================
 * INSTRUCTION MAP (5 instructions)
 * ===========================================================================
 *
 *   Opcode  Mnemonic       Stack Effect
 *   ------  --------       ------------
 *   0x20    local.get      [] -> [value]
 *   0x21    local.set      [value] -> []
 *   0x22    local.tee      [value] -> [value]   (peek, don't pop!)
 *   0x23    global.get     [] -> [value]
 *   0x24    global.set     [value] -> []
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register all 5 variable access instruction handlers.
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerVariable(vm: GenericVM): void {

  // =========================================================================
  // 0x20: local.get --- Read a local variable
  // =========================================================================
  //
  // Push the value of the local variable at the given index onto the stack.
  // The operand is the local index (decoded from the bytecodes).
  //
  //   local.get 0   ;; push the value of local 0
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x20, (vm, instr, _code, ctx) => {
    const index = instr.operand as number;
    vm.pushTyped(ctx.typedLocals[index]);
    vm.advancePc();
    return "local.get";
  });

  // =========================================================================
  // 0x21: local.set --- Write a local variable
  // =========================================================================
  //
  // Pop a value from the stack and store it in the local variable at the
  // given index. The previous value is overwritten.
  //
  //   i32.const 42
  //   local.set 0   ;; local 0 = 42, stack is now empty
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x21, (vm, instr, _code, ctx) => {
    const index = instr.operand as number;
    ctx.typedLocals[index] = vm.popTyped();
    vm.advancePc();
    return "local.set";
  });

  // =========================================================================
  // 0x22: local.tee --- Write a local variable WITHOUT popping
  // =========================================================================
  //
  // Like local.set, but the value remains on the stack. This is equivalent
  // to: dup + local.set (but more efficient as a single instruction).
  //
  // "Tee" comes from the plumbing T-junction: the value flows to both the
  // local variable AND continues down the stack.
  //
  //   i32.const 42
  //   local.tee 0   ;; local 0 = 42, stack still has [42]
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x22, (vm, instr, _code, ctx) => {
    const index = instr.operand as number;
    /* peekTyped reads the top without popping. */
    ctx.typedLocals[index] = vm.peekTyped();
    vm.advancePc();
    return "local.tee";
  });

  // =========================================================================
  // 0x23: global.get --- Read a global variable
  // =========================================================================
  //
  // Push the current value of the global variable at the given index.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x23, (vm, instr, _code, ctx) => {
    const index = instr.operand as number;
    vm.pushTyped(ctx.globals[index]);
    vm.advancePc();
    return "global.get";
  });

  // =========================================================================
  // 0x24: global.set --- Write a global variable
  // =========================================================================
  //
  // Pop a value and store it in the global. Mutability is already verified
  // by the validator; the execution engine trusts that validation passed.
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x24, (vm, instr, _code, ctx) => {
    const index = instr.operand as number;
    ctx.globals[index] = vm.popTyped();
    vm.advancePc();
    return "global.set";
  });
}
