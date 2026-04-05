/**
 * Test helpers for WASM instruction handler tests.
 *
 * Provides a minimal test harness that creates a GenericVM, registers
 * instruction handlers, and runs individual instructions with a fake
 * WasmExecutionContext.
 */

import { GenericVM } from "@coding-adventures/virtual-machine";
import type { CodeObject, Instruction } from "@coding-adventures/virtual-machine";
import { i32, i64, f32, f64 } from "../src/values.js";
import type { WasmValue } from "../src/values.js";
import type { WasmExecutionContext } from "../src/types.js";
import { LinearMemory } from "../src/linear_memory.js";

/**
 * Create a GenericVM and register handlers using the given registration fn.
 */
export function makeVm(register: (vm: GenericVM) => void): GenericVM {
  const vm = new GenericVM();
  register(vm);
  return vm;
}

/**
 * Create a minimal WasmExecutionContext for testing.
 * Most fields are empty/null since individual instruction tests
 * don't need the full context.
 */
export function makeContext(overrides?: Partial<WasmExecutionContext>): WasmExecutionContext {
  return {
    memory: null,
    tables: [],
    globals: [],
    globalTypes: [],
    funcTypes: [],
    funcBodies: [],
    hostFunctions: [],
    typedLocals: [],
    labelStack: [],
    controlFlowMap: new Map(),
    savedFrames: [],
    returned: false,
    returnValues: [],
    ...overrides,
  };
}

/**
 * Execute a sequence of instructions on the given VM with a context.
 *
 * This builds a CodeObject from the given instructions and runs it
 * through ``executeWithContext``.
 *
 * @param vm    - The VM with handlers registered.
 * @param instrs - Array of {opcode, operand?} objects.
 * @param ctx   - The WasmExecutionContext (defaults to a minimal one).
 */
export function runInstructions(
  vm: GenericVM,
  instrs: { opcode: number; operand?: unknown }[],
  ctx?: WasmExecutionContext,
): void {
  /*
   * Each handler calls vm.advancePc() at the end, so after the last
   * instruction, the PC will be past the instruction array length and
   * the VM's execute loop naturally terminates (no HALT needed).
   */
  const code: CodeObject = {
    instructions: instrs.map((i) => ({
      opcode: i.opcode,
      operand: i.operand as number | string | null | undefined,
    })),
    constants: [],
    names: [],
  };

  const context = ctx ?? makeContext();
  vm.reset();
  vm.executeWithContext(code, context);
}

/**
 * Push a typed value onto the VM's typed stack, then run one instruction.
 * Returns the top of the typed stack after execution.
 *
 * We use i32.const / i64.const / f32.const / f64.const to push the values
 * as real WASM instructions so the reset inside runInstructions doesn't
 * clear them. Alternatively, we directly push after reset.
 */
export function runUnary(
  vm: GenericVM,
  opcode: number,
  input: WasmValue,
  ctx?: WasmExecutionContext,
): WasmValue {
  const context = ctx ?? makeContext();
  const code: CodeObject = {
    instructions: [{ opcode, operand: undefined }],
    constants: [],
    names: [],
  };
  vm.reset();
  vm.pushTyped(input);
  vm.executeWithContext(code, context);
  return vm.peekTyped();
}

/**
 * Push two typed values (a first, b second), run one instruction,
 * and return the top of the typed stack.
 */
export function runBinary(
  vm: GenericVM,
  opcode: number,
  a: WasmValue,
  b: WasmValue,
  ctx?: WasmExecutionContext,
): WasmValue {
  const context = ctx ?? makeContext();
  const code: CodeObject = {
    instructions: [{ opcode, operand: undefined }],
    constants: [],
    names: [],
  };
  vm.reset();
  vm.pushTyped(a);
  vm.pushTyped(b);
  vm.executeWithContext(code, context);
  return vm.peekTyped();
}

/* Re-export value constructors for convenience. */
export { i32, i64, f32, f64, LinearMemory };
export type { WasmValue };
