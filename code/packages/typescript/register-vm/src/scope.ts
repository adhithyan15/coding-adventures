/**
 * scope.ts — Lexical scope chain (Context) operations.
 *
 * ## How closures work in V8
 *
 * Consider this JavaScript:
 *
 * ```js
 * function outer() {
 *   let x = 42;
 *   return function inner() { return x; };
 * }
 * const f = outer();
 * f(); // → 42
 * ```
 *
 * When `outer` is called, `x` lives in its register file.  But `inner`
 * is a closure that outlives `outer`'s stack frame.  V8 solves this by
 * *context-allocating* `x`: instead of keeping `x` in the ephemeral
 * register file, it moves `x` into a heap-allocated Context object.
 *
 * The Context object is a simple array of slots (one per captured
 * variable).  Each function that references captured variables receives
 * a pointer to the relevant context chain.
 *
 * Context chains form a linked list:
 *
 * ```
 * innerCtx  →  outerCtx  →  null
 *   slots       slots
 *  [inner-y]   [outer-x]
 * ```
 *
 * To access `x` from inside `inner`, the bytecode emits:
 *
 *   LDA_CONTEXT_SLOT [depth=1, slotIdx=0]
 *
 * which walks up 1 parent link and reads slot 0.
 *
 * ## Invariant
 *
 * Depth 0 is always the *current* context — i.e. the innermost active
 * scope.  Each parent link goes one scope outward.
 */

import type { Context, VMValue } from './types.js';

/**
 * Allocate a fresh Context with `slotCount` undefined slots.
 *
 * @param parent    The enclosing context, or null for the outermost scope.
 * @param slotCount Number of captured variables in this scope.
 */
export function newContext(parent: Context | null, slotCount: number): Context {
  return {
    slots: new Array<VMValue>(slotCount).fill(undefined),
    parent,
  };
}

/**
 * Read a context slot at the given chain depth and index.
 *
 * @param ctx   The innermost (current) context.
 * @param depth How many parent links to follow (0 = current context).
 * @param idx   Slot index within the target context.
 * @returns The stored value, or undefined if the path is invalid.
 *
 * @example
 *   const outer = newContext(null, 1);
 *   outer.slots[0] = 42;
 *   const inner = newContext(outer, 0);
 *   getSlot(inner, 1, 0); // → 42
 */
export function getSlot(ctx: Context, depth: number, idx: number): VMValue {
  let current: Context | null = ctx;
  for (let d = 0; d < depth; d++) {
    current = current?.parent ?? null;
  }
  if (current === null || idx < 0 || idx >= current.slots.length) return undefined;
  return current.slots[idx];
}

/**
 * Write a value to a context slot at the given chain depth and index.
 *
 * @param ctx   The innermost (current) context.
 * @param depth How many parent links to follow.
 * @param idx   Slot index within the target context.
 * @param value The value to store.
 */
export function setSlot(ctx: Context, depth: number, idx: number, value: VMValue): void {
  let current: Context | null = ctx;
  for (let d = 0; d < depth; d++) {
    current = current?.parent ?? null;
  }
  if (current === null || idx < 0 || idx >= current.slots.length) return;
  current.slots[idx] = value;
}
