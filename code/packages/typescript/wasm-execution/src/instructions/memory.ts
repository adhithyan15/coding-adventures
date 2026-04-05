/**
 * memory.ts --- Linear memory instruction handlers for WASM.
 *
 * ===========================================================================
 * OVERVIEW: MEMORY ACCESS IN WEBASSEMBLY
 * ===========================================================================
 *
 * WASM provides 27 memory-related instructions that load from and store to
 * linear memory. Each load/store instruction has:
 *
 * 1. An **alignment hint** (powers of 2) --- currently advisory only;
 *    WASM 1.0 ignores misaligned accesses (they're just slower).
 *
 * 2. A **static offset** --- added to the dynamic base address on the stack.
 *    The effective address is: ``(base >>> 0) + offset``.
 *
 * The load/store pattern is always:
 *   - Pop base address (i32) from stack.
 *   - Compute effective address = (base unsigned) + offset.
 *   - Access memory at that address.
 *   - For loads: push the loaded value.
 *   - For stores: also pop the value to store.
 *
 * Additionally, two instructions manage memory size:
 *
 *   - ``memory.size`` (0x3F): push current memory size in pages.
 *   - ``memory.grow`` (0x40): try to grow memory; push previous size or -1.
 *
 * ===========================================================================
 * EFFECTIVE ADDRESS CALCULATION
 * ===========================================================================
 *
 * The effective address uses UNSIGNED interpretation of the base address:
 *
 *   effective = (base >>> 0) + offset
 *
 * This is important because an i32 value of -1 (0xFFFFFFFF) should be
 * interpreted as address 4294967295, not -1.
 *
 * ===========================================================================
 * INSTRUCTION MAP (27 instructions)
 * ===========================================================================
 *
 *   Opcode  Mnemonic             Stack Effect
 *   ------  --------             ------------
 *   0x28    i32.load             [i32] -> [i32]
 *   0x29    i64.load             [i32] -> [i64]
 *   0x2A    f32.load             [i32] -> [f32]
 *   0x2B    f64.load             [i32] -> [f64]
 *   0x2C    i32.load8_s          [i32] -> [i32]
 *   0x2D    i32.load8_u          [i32] -> [i32]
 *   0x2E    i32.load16_s         [i32] -> [i32]
 *   0x2F    i32.load16_u         [i32] -> [i32]
 *   0x30    i64.load8_s          [i32] -> [i64]
 *   0x31    i64.load8_u          [i32] -> [i64]
 *   0x32    i64.load16_s         [i32] -> [i64]
 *   0x33    i64.load16_u         [i32] -> [i64]
 *   0x34    i64.load32_s         [i32] -> [i64]
 *   0x35    i64.load32_u         [i32] -> [i64]
 *   0x36    i32.store            [i32 i32] -> []
 *   0x37    i64.store            [i32 i64] -> []
 *   0x38    f32.store            [i32 f32] -> []
 *   0x39    f64.store            [i32 f64] -> []
 *   0x3A    i32.store8           [i32 i32] -> []
 *   0x3B    i32.store16          [i32 i32] -> []
 *   0x3C    i64.store8           [i32 i64] -> []
 *   0x3D    i64.store16          [i32 i64] -> []
 *   0x3E    i64.store32          [i32 i64] -> []
 *   0x3F    memory.size          [] -> [i32]
 *   0x40    memory.grow          [i32] -> [i32]
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { TrapError } from "../host_interface.js";
import { i32, i64, f32, f64, asI32, asI64, asF32, asF64 } from "../values.js";
import type { WasmExecutionContext } from "../types.js";

// ===========================================================================
// Helpers
// ===========================================================================

/**
 * Get the effective memory address from the instruction operand and
 * the base address on top of the stack.
 *
 * The operand is expected to be ``{ align: number, offset: number }``,
 * decoded by the pre-instruction hook from the raw bytecodes.
 */
function effectiveAddr(
  base: number,
  operand: unknown,
): number {
  const { offset } = operand as { align: number; offset: number };
  return (base >>> 0) + offset;
}

/**
 * Ensure the execution context has a non-null linear memory.
 * Throws a TrapError if memory is null (the module has no memory).
 */
function requireMemory(ctx: WasmExecutionContext) {
  if (ctx.memory === null) {
    throw new TrapError("no linear memory");
  }
  return ctx.memory;
}

// ===========================================================================
// Registration Function
// ===========================================================================

/**
 * Register all 27 memory instruction handlers.
 *
 * @param vm - The GenericVM to register handlers on.
 */
export function registerMemory(vm: GenericVM): void {

  // =========================================================================
  // Load Instructions
  // =========================================================================

  // 0x28: i32.load --- Load 4 bytes as i32
  vm.registerContextOpcode<WasmExecutionContext>(0x28, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i32(mem.loadI32(addr)));
    vm.advancePc();
    return "i32.load";
  });

  // 0x29: i64.load --- Load 8 bytes as i64
  vm.registerContextOpcode<WasmExecutionContext>(0x29, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i64(mem.loadI64(addr)));
    vm.advancePc();
    return "i64.load";
  });

  // 0x2A: f32.load --- Load 4 bytes as f32
  vm.registerContextOpcode<WasmExecutionContext>(0x2a, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(f32(mem.loadF32(addr)));
    vm.advancePc();
    return "f32.load";
  });

  // 0x2B: f64.load --- Load 8 bytes as f64
  vm.registerContextOpcode<WasmExecutionContext>(0x2b, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(f64(mem.loadF64(addr)));
    vm.advancePc();
    return "f64.load";
  });

  // 0x2C: i32.load8_s --- Load 1 byte, sign-extend to i32
  vm.registerContextOpcode<WasmExecutionContext>(0x2c, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i32(mem.loadI32_8s(addr)));
    vm.advancePc();
    return "i32.load8_s";
  });

  // 0x2D: i32.load8_u --- Load 1 byte, zero-extend to i32
  vm.registerContextOpcode<WasmExecutionContext>(0x2d, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i32(mem.loadI32_8u(addr)));
    vm.advancePc();
    return "i32.load8_u";
  });

  // 0x2E: i32.load16_s --- Load 2 bytes, sign-extend to i32
  vm.registerContextOpcode<WasmExecutionContext>(0x2e, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i32(mem.loadI32_16s(addr)));
    vm.advancePc();
    return "i32.load16_s";
  });

  // 0x2F: i32.load16_u --- Load 2 bytes, zero-extend to i32
  vm.registerContextOpcode<WasmExecutionContext>(0x2f, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i32(mem.loadI32_16u(addr)));
    vm.advancePc();
    return "i32.load16_u";
  });

  // 0x30: i64.load8_s --- Load 1 byte, sign-extend to i64
  vm.registerContextOpcode<WasmExecutionContext>(0x30, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i64(mem.loadI64_8s(addr)));
    vm.advancePc();
    return "i64.load8_s";
  });

  // 0x31: i64.load8_u --- Load 1 byte, zero-extend to i64
  vm.registerContextOpcode<WasmExecutionContext>(0x31, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i64(mem.loadI64_8u(addr)));
    vm.advancePc();
    return "i64.load8_u";
  });

  // 0x32: i64.load16_s --- Load 2 bytes, sign-extend to i64
  vm.registerContextOpcode<WasmExecutionContext>(0x32, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i64(mem.loadI64_16s(addr)));
    vm.advancePc();
    return "i64.load16_s";
  });

  // 0x33: i64.load16_u --- Load 2 bytes, zero-extend to i64
  vm.registerContextOpcode<WasmExecutionContext>(0x33, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i64(mem.loadI64_16u(addr)));
    vm.advancePc();
    return "i64.load16_u";
  });

  // 0x34: i64.load32_s --- Load 4 bytes, sign-extend to i64
  vm.registerContextOpcode<WasmExecutionContext>(0x34, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i64(mem.loadI64_32s(addr)));
    vm.advancePc();
    return "i64.load32_s";
  });

  // 0x35: i64.load32_u --- Load 4 bytes, zero-extend to i64
  vm.registerContextOpcode<WasmExecutionContext>(0x35, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    vm.pushTyped(i64(mem.loadI64_32u(addr)));
    vm.advancePc();
    return "i64.load32_u";
  });

  // =========================================================================
  // Store Instructions
  // =========================================================================
  //
  // Store instructions pop: [base_address (i32), value_to_store].
  // Note: the value is on TOP, the address is below.
  //

  // 0x36: i32.store
  vm.registerContextOpcode<WasmExecutionContext>(0x36, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asI32(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeI32(addr, value);
    vm.advancePc();
    return "i32.store";
  });

  // 0x37: i64.store
  vm.registerContextOpcode<WasmExecutionContext>(0x37, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asI64(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeI64(addr, value);
    vm.advancePc();
    return "i64.store";
  });

  // 0x38: f32.store
  vm.registerContextOpcode<WasmExecutionContext>(0x38, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asF32(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeF32(addr, value);
    vm.advancePc();
    return "f32.store";
  });

  // 0x39: f64.store
  vm.registerContextOpcode<WasmExecutionContext>(0x39, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asF64(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeF64(addr, value);
    vm.advancePc();
    return "f64.store";
  });

  // 0x3A: i32.store8 --- Store low byte of i32
  vm.registerContextOpcode<WasmExecutionContext>(0x3a, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asI32(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeI32_8(addr, value);
    vm.advancePc();
    return "i32.store8";
  });

  // 0x3B: i32.store16 --- Store low 2 bytes of i32
  vm.registerContextOpcode<WasmExecutionContext>(0x3b, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asI32(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeI32_16(addr, value);
    vm.advancePc();
    return "i32.store16";
  });

  // 0x3C: i64.store8 --- Store low byte of i64
  vm.registerContextOpcode<WasmExecutionContext>(0x3c, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asI64(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeI64_8(addr, value);
    vm.advancePc();
    return "i64.store8";
  });

  // 0x3D: i64.store16 --- Store low 2 bytes of i64
  vm.registerContextOpcode<WasmExecutionContext>(0x3d, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asI64(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeI64_16(addr, value);
    vm.advancePc();
    return "i64.store16";
  });

  // 0x3E: i64.store32 --- Store low 4 bytes of i64
  vm.registerContextOpcode<WasmExecutionContext>(0x3e, (vm, instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const value = asI64(vm.popTyped());
    const base = asI32(vm.popTyped());
    const addr = effectiveAddr(base, instr.operand);
    mem.storeI64_32(addr, value);
    vm.advancePc();
    return "i64.store32";
  });

  // =========================================================================
  // Memory Management
  // =========================================================================

  // 0x3F: memory.size --- Get current memory size in pages
  //
  // The operand is a memory index (always 0 in WASM 1.0, since only one
  // linear memory is allowed per module).
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x3f, (vm, _instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    vm.pushTyped(i32(mem.size()));
    vm.advancePc();
    return "memory.size";
  });

  // 0x40: memory.grow --- Try to grow memory by N pages
  //
  // Pops the number of pages to grow by. Pushes the previous size in pages,
  // or -1 if the grow failed (e.g., would exceed maximum).
  //
  vm.registerContextOpcode<WasmExecutionContext>(0x40, (vm, _instr, _code, ctx) => {
    const mem = requireMemory(ctx);
    const delta = asI32(vm.popTyped());
    const result = mem.grow(delta);
    vm.pushTyped(i32(result));
    vm.advancePc();
    return "memory.grow";
  });
}
