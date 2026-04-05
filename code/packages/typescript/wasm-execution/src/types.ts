/**
 * Shared type definitions for the WASM execution engine.
 *
 * ==========================================================================
 * Chapter 1: Why Separate Types?
 * ==========================================================================
 *
 * The WASM execution engine uses several data structures that are shared
 * across multiple modules: the execution context, labels for structured
 * control flow, and control flow map entries. Rather than defining these
 * in each module and creating circular imports, we define them here as
 * the single source of truth.
 *
 * @module
 */

import type { FuncType, FunctionBody, GlobalType } from "@coding-adventures/wasm-types";
import type { LinearMemory } from "./linear_memory.js";
import type { Table } from "./table.js";
import type { HostFunction } from "./host_interface.js";
import type { WasmValue } from "./values.js";

// =========================================================================
// Control Flow Structures
// =========================================================================

/**
 * A label on the label stack, tracking one level of structured control flow.
 *
 * WASM uses **structured control flow** — there are no arbitrary gotos.
 * Instead, ``block``, ``loop``, and ``if`` create labeled scopes, and
 * ``br`` (branch) can jump to these labels by index.
 *
 * When a ``br N`` instruction executes, it unwinds to the Nth label from
 * the top of the label stack. The behavior depends on whether the label
 * is from a ``block`` or a ``loop``:
 *
 * - **block/if labels**: branch jumps to the END (forward). The label's
 *   ``arity`` values are kept on the stack as the block's result.
 *
 * - **loop labels**: branch jumps to the LOOP START (backward). This is
 *   how WASM implements looping — a ``br 0`` inside a ``loop`` jumps
 *   back to the top.
 *
 * ```
 *   block (result i32)      ;; label L0 — br 0 jumps to END
 *     i32.const 42
 *     br 0                  ;; jump forward to end, carrying 42
 *   end                     ;; ← br 0 lands here; stack has [42]
 *
 *   loop                    ;; label L0 — br 0 jumps to START
 *     br 0                  ;; jump backward to loop start
 *   end
 * ```
 */
export interface Label {
  /**
   * How many values this block/loop/if produces.
   *
   * When branching to this label, exactly ``arity`` values are kept on
   * the stack (popped from above the label's stack height). For WASM 1.0,
   * this is 0 or 1 (multi-value returns are a post-MVP feature).
   */
  readonly arity: number;

  /**
   * Where to jump when branching to this label.
   *
   * - For blocks/if: the instruction index of the matching ``end``.
   * - For loops: the instruction index of the ``loop`` instruction itself
   *   (so execution restarts from the top of the loop).
   */
  readonly targetPc: number;

  /**
   * The typed stack height when this block started.
   *
   * When branching, the stack is unwound to this height (plus ``arity``
   * result values). This ensures blocks can't "leak" extra values.
   */
  readonly stackHeight: number;

  /**
   * Whether this label is from a ``loop`` instruction.
   *
   * Loops branch backward (to the start); blocks and if/else branch
   * forward (to the end). This flag determines the branching direction.
   */
  readonly isLoop: boolean;
}

/**
 * A control flow map entry: records where a block/loop/if ends.
 *
 * Built by a one-time pre-scan of the function's bytecodes when the
 * function is first called. This avoids scanning forward during execution
 * to find matching end/else instructions.
 */
export interface ControlTarget {
  /** Instruction index of the matching ``end``. */
  readonly endPc: number;

  /** Instruction index of the ``else``, or null if there's no else branch. */
  readonly elsePc: number | null;
}

// =========================================================================
// Execution Context
// =========================================================================

/**
 * The per-execution context passed to all WASM instruction handlers.
 *
 * This carries all the runtime state that WASM instructions need:
 * linear memory for loads/stores, tables for indirect calls, globals,
 * locals, and the control flow structures.
 *
 * Rather than storing this on the GenericVM (which is generic), we pass
 * it through via ``executeWithContext()``. Each handler receives this as
 * its fourth argument.
 *
 * ```
 * ┌──────────────────────────────────────────────────────┐
 * │              WasmExecutionContext                     │
 * │                                                       │
 * │  memory: LinearMemory  ── byte-addressable heap       │
 * │  tables: Table[]       ── function reference arrays   │
 * │  globals: WasmValue[]  ── module-level variables      │
 * │  typedLocals: WasmValue[] ── current frame's locals   │
 * │  labelStack: Label[]   ── control flow nesting        │
 * │  funcTypes: FuncType[] ── for call_indirect type check│
 * │  controlFlowMap: Map   ── block → end mappings        │
 * │  ...                                                   │
 * └──────────────────────────────────────────────────────┘
 * ```
 */
export interface WasmExecutionContext {
  /** Linear memory (null if the module has no memory section). */
  memory: LinearMemory | null;

  /** Function reference tables for indirect calls. */
  tables: Table[];

  /** Global variable values (mutable — changes persist across calls). */
  globals: WasmValue[];

  /** Global variable type descriptors (for mutability checking). */
  globalTypes: GlobalType[];

  /** All function type signatures (imports + module functions). */
  funcTypes: FuncType[];

  /** Function bodies (null for imported functions). */
  funcBodies: (FunctionBody | null)[];

  /** Host function implementations (null for module-defined functions). */
  hostFunctions: (HostFunction | null)[];

  /** The current frame's local variables (params + declared locals). */
  typedLocals: WasmValue[];

  /** Control flow label stack for the current frame. */
  labelStack: Label[];

  /**
   * Pre-computed control flow map: block/loop/if start → end/else locations.
   *
   * Built once per function by a pre-scan pass. Keyed by the instruction
   * index of the block/loop/if start; value is the matching end (and else
   * for if instructions).
   */
  controlFlowMap: Map<number, ControlTarget>;

  /**
   * Saved frames for function calls. Each entry is a snapshot of the
   * caller's state (PC, locals, label stack, typed stack height).
   */
  savedFrames: SavedFrame[];

  /** Whether the current function has returned. */
  returned: boolean;

  /** Return values from the current function. */
  returnValues: WasmValue[];
}

/**
 * A saved call frame — the caller's state before a function call.
 *
 * When ``call`` or ``call_indirect`` executes, we snapshot the caller's
 * locals, label stack, and stack height, then set up fresh state for the
 * callee. When the callee returns, we restore from this snapshot.
 */
export interface SavedFrame {
  /** The caller's local variables. */
  readonly locals: WasmValue[];

  /** The caller's label stack. */
  readonly labelStack: Label[];

  /** The typed stack height when the call was made. */
  readonly stackHeight: number;

  /** The caller's control flow map. */
  readonly controlFlowMap: Map<number, ControlTarget>;

  /** The caller's return PC (instruction after the call). */
  readonly returnPc: number;

  /** The caller's function return arity. */
  readonly returnArity: number;
}
