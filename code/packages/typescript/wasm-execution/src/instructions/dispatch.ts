/**
 * dispatch.ts --- Central registration of all WASM instruction handlers.
 *
 * ===========================================================================
 * OVERVIEW
 * ===========================================================================
 *
 * This module is the single entry point for registering all WASM instruction
 * handlers on a GenericVM. Rather than having the runtime call each
 * registration function individually, it calls ``registerAllInstructions``
 * which delegates to the per-category modules.
 *
 * The handler categories are:
 *
 *   1. **numeric_i32** --- 33 handlers for 32-bit integer operations
 *   2. **numeric_i64** --- 32 handlers for 64-bit integer operations
 *   3. **numeric_f32** --- 23 handlers for 32-bit float operations
 *   4. **numeric_f64** --- 23 handlers for 64-bit float operations
 *   5. **conversion**  --- 27 handlers for type conversions
 *   6. **variable**    --- 5 handlers for local/global variable access
 *   7. **parametric**  --- 2 handlers for drop and select
 *   8. **memory**      --- 27 handlers for linear memory access
 *
 * Control flow instructions (block, loop, if, br, call, etc.) are NOT
 * included here --- they will be added in a separate ``control.ts`` module
 * because they require more complex interaction with the VM's execution
 * loop (e.g., manipulating the label stack and saved frames).
 *
 * ===========================================================================
 * TOTAL: 172 handlers (out of ~182 in WASM 1.0)
 * ===========================================================================
 *
 * The remaining ~10 instructions are control flow (block, loop, if, else,
 * end, br, br_if, br_table, return, call, call_indirect, unreachable, nop).
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { registerNumericI32 } from "./numeric_i32.js";
import { registerNumericI64 } from "./numeric_i64.js";
import { registerNumericF32 } from "./numeric_f32.js";
import { registerNumericF64 } from "./numeric_f64.js";
import { registerConversion } from "./conversion.js";
import { registerVariable } from "./variable.js";
import { registerParametric } from "./parametric.js";
import { registerMemory } from "./memory.js";

/**
 * Register all non-control-flow WASM instruction handlers on the VM.
 *
 * Call this once during module instantiation, before executing any code.
 * After this returns, the VM will have handlers for all numeric, conversion,
 * variable, parametric, and memory instructions.
 *
 * @param vm - The GenericVM to register all handlers on.
 */
export function registerAllInstructions(vm: GenericVM): void {
  registerNumericI32(vm);
  registerNumericI64(vm);
  registerNumericF32(vm);
  registerNumericF64(vm);
  registerConversion(vm);
  registerVariable(vm);
  registerParametric(vm);
  registerMemory(vm);
}
