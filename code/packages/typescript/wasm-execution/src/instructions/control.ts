/**
 * Control Flow Instruction Handlers — The Heart of WASM's Execution Model.
 *
 * ==========================================================================
 * Chapter 1: Structured Control Flow
 * ==========================================================================
 *
 * Unlike assembly language where you can ``jmp`` to any address, WASM uses
 * **structured control flow**. Every branch target is a labeled scope
 * created by ``block``, ``loop``, or ``if``. This makes WASM programs
 * easier to validate, optimize, and reason about.
 *
 * The three control flow constructs:
 *
 * 1. **block**: Creates a forward-branching scope. ``br 0`` inside a block
 *    jumps to the block's ``end``.
 *
 * 2. **loop**: Creates a backward-branching scope. ``br 0`` inside a loop
 *    jumps back to the loop's start (this is how WASM does looping).
 *
 * 3. **if/else**: Pops a condition from the stack. If nonzero, executes
 *    the "then" branch; if zero, jumps to ``else`` (or ``end`` if no else).
 *
 * ==========================================================================
 * Chapter 2: How Branching Works
 * ==========================================================================
 *
 * When ``br N`` executes:
 *
 * 1. Look up the Nth label from the top of the label stack (0 = innermost).
 * 2. Pop the label's ``arity`` values from the value stack (the block's results).
 * 3. Unwind the value stack to the label's ``stackHeight``.
 * 4. Push the result values back.
 * 5. Pop labels down to (and including) the target label.
 * 6. Set PC to the label's ``targetPc``.
 *    - For blocks/if: targetPc points to ``end`` (forward jump).
 *    - For loops: targetPc points to ``loop`` start (backward jump).
 *
 * ==========================================================================
 * Chapter 3: Function Calls
 * ==========================================================================
 *
 * ``call`` and ``call_indirect`` create new activation records:
 *
 * 1. Save the caller's state (locals, labels, stack height, return PC).
 * 2. Pop arguments from the caller's stack.
 * 3. Initialize the callee's locals (arguments + zero-initialized declared locals).
 * 4. Set PC to 0 (start of callee's code).
 * 5. Build the callee's control flow map.
 * 6. Execute the callee until it returns (via ``return`` or reaching ``end``).
 * 7. Restore the caller's state.
 * 8. Push the callee's return values onto the caller's stack.
 *
 * @module
 */

import type { GenericVM } from "@coding-adventures/virtual-machine";
import { ValueType } from "@coding-adventures/wasm-types";
import type { FuncType } from "@coding-adventures/wasm-types";
import { TrapError } from "../host_interface.js";
import { i32, defaultValue } from "../values.js";
import type { WasmValue } from "../values.js";
import type { WasmExecutionContext, Label, ControlTarget, SavedFrame } from "../types.js";

// =========================================================================
// Helper: Block Type Resolution
// =========================================================================

/**
 * Resolve a block type to its result arity.
 *
 * WASM 1.0 block types:
 * - 0x40: empty — the block produces 0 values.
 * - 0x7F/0x7E/0x7D/0x7C: single value type — the block produces 1 value.
 * - Negative number (signed LEB128): type index — references a FuncType
 *   for multi-value (post-MVP, but we handle it for correctness).
 */
function blockArity(blockType: number | null, funcTypes: FuncType[]): number {
  if (blockType === null || blockType === 0x40) return 0;
  if (blockType === ValueType.I32 || blockType === ValueType.I64 ||
      blockType === ValueType.F32 || blockType === ValueType.F64) {
    return 1;
  }
  // Type index (for multi-value blocks) — get result count from FuncType
  if (blockType >= 0 && blockType < funcTypes.length) {
    return funcTypes[blockType].results.length;
  }
  return 0;
}

/**
 * Resolve a block type to its parameter arity (for loop labels).
 *
 * When branching to a loop, the branch carries the loop's *parameter*
 * types, not its result types. For WASM 1.0 with single-value blocks,
 * loop parameters are always 0.
 */
function blockParamArity(blockType: number | null, funcTypes: FuncType[]): number {
  if (blockType === null || blockType === 0x40) return 0;
  if (blockType === ValueType.I32 || blockType === ValueType.I64 ||
      blockType === ValueType.F32 || blockType === ValueType.F64) {
    return 0; // Single-value blocks have 0 params
  }
  if (blockType >= 0 && blockType < funcTypes.length) {
    return funcTypes[blockType].params.length;
  }
  return 0;
}

// =========================================================================
// Helper: Branch Execution
// =========================================================================

/**
 * Execute a branch to the Nth label from the top of the label stack.
 *
 * This is the core branching primitive used by ``br``, ``br_if``, and
 * ``br_table``. It:
 *
 * 1. Finds the target label.
 * 2. Determines how many result values to carry.
 * 3. Saves those values, unwinds the stack, pushes them back.
 * 4. Pops labels down to the target.
 * 5. Sets the PC to the label's target.
 */
function executeBranch(
  vm: GenericVM,
  ctx: WasmExecutionContext,
  labelIndex: number,
): void {
  // The label stack is indexed from the top: 0 = innermost, 1 = next out, etc.
  const labelStackIndex = ctx.labelStack.length - 1 - labelIndex;
  if (labelStackIndex < 0) {
    throw new TrapError(`branch target ${labelIndex} out of range`);
  }

  const label = ctx.labelStack[labelStackIndex];

  // For loops, branching carries the loop's parameter values (0 in MVP).
  // For blocks, branching carries the block's result values.
  const arity = label.isLoop ? blockParamArity(null, ctx.funcTypes) : label.arity;

  // Save the result values from the top of the stack.
  const results: WasmValue[] = [];
  for (let j = 0; j < arity; j++) {
    results.unshift(vm.popTyped() as WasmValue);
  }

  // Unwind the value stack to the label's recorded height.
  while (vm.typedStack.length > label.stackHeight) {
    vm.popTyped();
  }

  // Push the result values back.
  for (const v of results) {
    vm.pushTyped(v);
  }

  // Branching to a block/if lands on that label's `end`, so the target label
  // must stay on the stack until the `end` instruction executes and pops it.
  // Branching to a loop jumps back to the loop header, which will push a fresh
  // loop label when it re-executes, so the current loop label can be removed.
  ctx.labelStack.length = label.isLoop ? labelStackIndex : labelStackIndex + 1;

  // Jump to the label's target PC.
  vm.jumpTo(label.targetPc);
}

// =========================================================================
// Control Flow Instruction Registration
// =========================================================================

/**
 * Register all 13 control flow instruction handlers.
 *
 * | Opcode | Name           | Description                          |
 * |--------|----------------|--------------------------------------|
 * | 0x00   | unreachable    | Always traps                         |
 * | 0x01   | nop            | No operation                         |
 * | 0x02   | block          | Start a forward-branching block      |
 * | 0x03   | loop           | Start a backward-branching loop      |
 * | 0x04   | if             | Conditional branch                   |
 * | 0x05   | else           | Start else branch of if              |
 * | 0x0B   | end            | End block/loop/if/function           |
 * | 0x0C   | br             | Unconditional branch                 |
 * | 0x0D   | br_if          | Conditional branch                   |
 * | 0x0E   | br_table       | Indexed branch table                 |
 * | 0x0F   | return         | Return from function                 |
 * | 0x10   | call           | Call function by index               |
 * | 0x11   | call_indirect  | Call function through table          |
 */
export function registerControl(vm: GenericVM): void {

  // ── unreachable (0x00) ─────────────────────────────────────────────
  // Always traps. Used by compilers to mark code that should never execute.
  // If execution reaches this instruction, something has gone very wrong.
  vm.registerContextOpcode(0x00, (_vm, _instr, _code, _ctx: WasmExecutionContext) => {
    throw new TrapError("unreachable instruction executed");
  });

  // ── nop (0x01) ─────────────────────────────────────────────────────
  // Does absolutely nothing. Used for alignment or as a placeholder.
  vm.registerContextOpcode(0x01, (vm, _instr, _code, _ctx: WasmExecutionContext) => {
    vm.advancePc();
    return "nop";
  });

  // ── block (0x02) ────────────────────────────────────────────────────
  // Starts a new block scope. The operand is the block type.
  // Pushes a label pointing to the matching ``end``.
  vm.registerContextOpcode(0x02, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const blockType = instr.operand as number | null;
    const arity = blockArity(blockType, ctx.funcTypes);
    const target = ctx.controlFlowMap.get(vm.pc);
    const endPc = target ? target.endPc : vm.pc + 1;

    ctx.labelStack.push({
      arity,
      targetPc: endPc,      // br jumps to end (forward)
      stackHeight: vm.typedStack.length,
      isLoop: false,
    });

    vm.advancePc();
    return "block";
  });

  // ── loop (0x03) ─────────────────────────────────────────────────────
  // Starts a loop. Unlike blocks, branching to a loop's label jumps
  // BACK to the loop start, not forward to the end.
  vm.registerContextOpcode(0x03, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const blockType = instr.operand as number | null;
    const arity = blockArity(blockType, ctx.funcTypes);

    ctx.labelStack.push({
      arity,
      targetPc: vm.pc,       // br jumps back to loop start!
      stackHeight: vm.typedStack.length,
      isLoop: true,
    });

    vm.advancePc();
    return "loop";
  });

  // ── if (0x04) ───────────────────────────────────────────────────────
  // Pops an i32 condition. If nonzero, enters the "then" branch.
  // If zero, jumps to ``else`` (if present) or ``end``.
  vm.registerContextOpcode(0x04, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const blockType = instr.operand as number | null;
    const arity = blockArity(blockType, ctx.funcTypes);
    const condition = (vm.popTyped() as WasmValue).value as number;

    const target = ctx.controlFlowMap.get(vm.pc);
    const endPc = target ? target.endPc : vm.pc + 1;
    const elsePc = target ? target.elsePc : null;

    ctx.labelStack.push({
      arity,
      targetPc: endPc,
      stackHeight: vm.typedStack.length,
      isLoop: false,
    });

    if (condition !== 0) {
      // Condition is true — enter the "then" branch (next instruction).
      vm.advancePc();
    } else {
      // Condition is false — jump to else or end.
      vm.jumpTo(elsePc !== null ? elsePc + 1 : endPc);
    }

    return `if (${condition !== 0 ? "then" : "else"})`;
  });

  // ── else (0x05) ─────────────────────────────────────────────────────
  // Marks the start of the else branch. When reached sequentially (after
  // completing the "then" branch), we jump to ``end`` to skip the else.
  vm.registerContextOpcode(0x05, (vm, _instr, _code, ctx: WasmExecutionContext) => {
    // Find the enclosing if's label (top of label stack).
    const label = ctx.labelStack[ctx.labelStack.length - 1];
    // Jump to end — skip the else branch (we just finished "then").
    vm.jumpTo(label.targetPc);
    return "else (skip to end)";
  });

  // ── end (0x0B) ──────────────────────────────────────────────────────
  // Ends a block/loop/if, or ends the entire function.
  vm.registerContextOpcode(0x0B, (vm, _instr, _code, ctx: WasmExecutionContext) => {
    if (ctx.labelStack.length > 0) {
      // End of a block/loop/if — pop the label.
      ctx.labelStack.pop();
      vm.advancePc();
      return "end (block)";
    } else {
      // End of function — signal return.
      ctx.returned = true;
      // Collect return values from the stack.
      // The number of return values is determined by the function type.
      // For now, we'll let the execution engine handle this.
      vm.halted = true;
      return "end (function)";
    }
  });

  // ── br (0x0C) ───────────────────────────────────────────────────────
  // Unconditional branch to the Nth enclosing label.
  vm.registerContextOpcode(0x0C, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const labelIndex = instr.operand as number;
    executeBranch(vm, ctx, labelIndex);
    return `br ${labelIndex}`;
  });

  // ── br_if (0x0D) ────────────────────────────────────────────────────
  // Conditional branch: pops an i32 condition. Branches if nonzero.
  vm.registerContextOpcode(0x0D, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const labelIndex = instr.operand as number;
    const condition = (vm.popTyped() as WasmValue).value as number;

    if (condition !== 0) {
      executeBranch(vm, ctx, labelIndex);
      return `br_if ${labelIndex} (taken)`;
    } else {
      vm.advancePc();
      return `br_if ${labelIndex} (not taken)`;
    }
  });

  // ── br_table (0x0E) ─────────────────────────────────────────────────
  // Indexed branch: pops an i32 index, branches to labels[index] or
  // the default label if index is out of bounds.
  vm.registerContextOpcode(0x0E, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const { labels, defaultLabel } = instr.operand as unknown as { labels: number[]; defaultLabel: number };
    const index = (vm.popTyped() as WasmValue).value as number;

    const targetLabel = (index >= 0 && index < labels.length)
      ? labels[index]
      : defaultLabel;

    executeBranch(vm, ctx, targetLabel);
    return `br_table → ${targetLabel}`;
  });

  // ── return (0x0F) ───────────────────────────────────────────────────
  // Return from the current function. Equivalent to ``br`` to the
  // outermost label (the function's implicit block).
  vm.registerContextOpcode(0x0F, (vm, _instr, _code, ctx: WasmExecutionContext) => {
    ctx.returned = true;
    vm.halted = true;
    return "return";
  });

  // ── call (0x10) ─────────────────────────────────────────────────────
  // Call a function by index. The operand is the function index.
  vm.registerContextOpcode(0x10, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const funcIndex = instr.operand as number;
    callFunction(vm, ctx, funcIndex);
    return `call ${funcIndex}`;
  });

  // ── call_indirect (0x11) ────────────────────────────────────────────
  // Indirect call via a table. The operand is { typeIdx, tableIdx }.
  // Pops an i32 table index from the stack, looks up the function in the
  // table, verifies its type matches, then calls it.
  vm.registerContextOpcode(0x11, (vm, instr, _code, ctx: WasmExecutionContext) => {
    const { typeIdx, tableIdx } = instr.operand as unknown as { typeIdx: number; tableIdx: number };
    const elemIndex = (vm.popTyped() as WasmValue).value as number;

    // Look up the function in the table.
    const table = ctx.tables[tableIdx ?? 0];
    if (!table) {
      throw new TrapError("undefined table");
    }

    const funcIndex = table.get(elemIndex);
    if (funcIndex === null || funcIndex === undefined) {
      throw new TrapError("uninitialized table element");
    }

    // Type check: the function's type must match the expected type.
    const expectedType = ctx.funcTypes[typeIdx];
    const actualType = ctx.funcTypes[funcIndex];
    if (!expectedType || !actualType) {
      throw new TrapError("undefined type");
    }
    if (expectedType.params.length !== actualType.params.length ||
        expectedType.results.length !== actualType.results.length ||
        !expectedType.params.every((p, i) => p === actualType.params[i]) ||
        !expectedType.results.every((r, i) => r === actualType.results[i])) {
      throw new TrapError("indirect call type mismatch");
    }

    callFunction(vm, ctx, funcIndex);
    return `call_indirect [${elemIndex}] → func ${funcIndex}`;
  });
}

// =========================================================================
// Function Call Implementation
// =========================================================================

/**
 * Call a function by index.
 *
 * This handles both module-defined functions and imported host functions.
 * For module functions, it saves the caller's state, sets up the callee's
 * frame, and lets the eval loop execute the callee's bytecodes.
 *
 * For host functions, it calls the host implementation directly and pushes
 * the results.
 */
function callFunction(
  vm: GenericVM,
  ctx: WasmExecutionContext,
  funcIndex: number,
): void {
  const funcType = ctx.funcTypes[funcIndex];
  if (!funcType) {
    throw new TrapError(`undefined function ${funcIndex}`);
  }

  // Pop arguments from the stack (in reverse order).
  const args: WasmValue[] = [];
  for (let j = 0; j < funcType.params.length; j++) {
    args.unshift(vm.popTyped() as WasmValue);
  }

  // Check if this is a host function (imported).
  const hostFunc = ctx.hostFunctions[funcIndex];
  if (hostFunc) {
    // Call the host function directly.
    const results = hostFunc.call(args);
    for (const r of results) {
      vm.pushTyped(r);
    }
    vm.advancePc();
    return;
  }

  // Module-defined function — set up a new frame.
  const body = ctx.funcBodies[funcIndex];
  if (!body) {
    throw new TrapError(`no body for function ${funcIndex}`);
  }

  // Save the caller's state.
  ctx.savedFrames.push({
    locals: [...ctx.typedLocals],
    labelStack: [...ctx.labelStack],
    stackHeight: vm.typedStack.length,
    controlFlowMap: ctx.controlFlowMap,
    returnPc: vm.pc + 1, // Return to the instruction after the call.
    returnArity: funcType.results.length,
  });

  // Initialize the callee's locals: arguments + zero-initialized declared locals.
  ctx.typedLocals = [
    ...args,
    ...body.locals.map(t => defaultValue(t)),
  ];

  // Clear the label stack for the new frame.
  ctx.labelStack = [];

  // The control flow map will be built by the execution engine
  // when it detects a new function entry (PC = 0 with a new frame).

  // Reset execution state for the callee.
  ctx.returned = false;
  vm.halted = false;
  vm.jumpTo(0);
}
