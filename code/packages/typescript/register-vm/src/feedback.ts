/**
 * feedback.ts — Feedback vector utilities.
 *
 * ## What are feedback vectors?
 *
 * When V8 first executes a function, it runs in "unoptimised" mode (Ignition
 * interpreter). As it runs, every "interesting" instruction (binary ops,
 * property loads, function calls) records type information into a slot in the
 * function's *feedback vector*.
 *
 * After the function has been called enough times, the optimising JIT compiler
 * (Maglev / Turbofan) reads the feedback vector and emits specialised machine
 * code. For example, if ADD has only ever seen (number, number) pairs, the
 * compiler emits a single `addq` x86 instruction instead of a general-purpose
 * addition routine.
 *
 * ## The state machine
 *
 * Each slot tracks *how many distinct type pairs* it has seen:
 *
 * ```
 *  ┌──────────────┐
 *  │ uninitialized│  (slot has never been executed)
 *  └──────┬───────┘
 *         │ first type pair P
 *         ▼
 *  ┌──────────────┐
 *  │ monomorphic  │  (only ever seen P)
 *  └──────┬───────┘
 *         │ new type pair Q ≠ P
 *         ▼
 *  ┌──────────────┐
 *  │ polymorphic  │  (seen 2–4 distinct pairs)
 *  └──────┬───────┘
 *         │ 5th distinct pair
 *         ▼
 *  ┌──────────────┐
 *  │ megamorphic  │  (terminal — too many types to track)
 *  └──────────────┘
 * ```
 *
 * The JIT treats each state differently:
 *  - monomorphic  → emit a single specialised fast path
 *  - polymorphic  → emit an if-else chain of fast paths
 *  - megamorphic  → emit generic slow path only
 *
 * ## Usage
 *
 * ```typescript
 * const vec = newVector(3);
 * recordBinaryOp(vec, 0, 42, 7);    // slot 0: uninitialized → monomorphic
 * recordBinaryOp(vec, 0, 42, 7);    // slot 0: monomorphic (no change)
 * recordBinaryOp(vec, 0, 'a', 'b'); // slot 0: monomorphic → polymorphic
 * ```
 */

import type { FeedbackSlot, TypePair, VMValue } from './types.js';

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Allocate a new feedback vector of the given size.
 * All slots start in the 'uninitialized' state.
 *
 * @param size Number of slots (must match CodeObject.feedbackSlotCount).
 */
export function newVector(size: number): FeedbackSlot[] {
  return Array.from({ length: size }, (): FeedbackSlot => ({ kind: 'uninitialized' }));
}

/**
 * Classify a VMValue into a type string for feedback recording.
 *
 * Unlike JavaScript's `typeof`, this function returns:
 *   - 'null'      for null  (JS typeof returns 'object', which is confusing)
 *   - 'undefined' for undefined
 *   - 'array'     for arrays (JS typeof returns 'object')
 *   - 'function'  for VMFunction
 *
 * This gives the optimiser finer-grained information about what's in the
 * accumulator, making it more likely to find a truly monomorphic site.
 *
 * @param v Any VM value.
 * @returns A short lower-case type tag.
 */
export function valueType(v: VMValue): string {
  if (v === null) return 'null';
  if (v === undefined) return 'undefined';
  if (typeof v === 'number') return 'number';
  if (typeof v === 'string') return 'string';
  if (typeof v === 'boolean') return 'boolean';
  if (Array.isArray(v)) return 'array';
  if (typeof v === 'object' && 'kind' in v && (v as { kind: string }).kind === 'function') return 'function';
  return 'object';
}

/**
 * Record the types seen at a binary-operation feedback slot.
 *
 * Follows the state machine documented at the top of this file.
 *
 * Mutates `vector[slot]` in place.
 *
 * @param vector  The frame's feedback vector.
 * @param slot    Index of the slot to update (must be a valid index).
 * @param left    Left-hand operand (usually the accumulator).
 * @param right   Right-hand operand (usually a named register value).
 */
export function recordBinaryOp(
  vector: FeedbackSlot[],
  slot: number,
  left: VMValue,
  right: VMValue,
): void {
  const pair: TypePair = [valueType(left), valueType(right)];
  _recordPair(vector, slot, pair);
}

/**
 * Record the hidden-class ID seen at a property-load feedback slot.
 *
 * For property loads the relevant "type" is the object's shape (hidden class),
 * not a JS type name.  We encode the ID as the string `"hc:<id>"` so it fits
 * in the same TypePair slot format.
 *
 * @param vector       The frame's feedback vector.
 * @param slot         Slot index to update.
 * @param hiddenClassId The hiddenClassId of the object being accessed.
 */
export function recordPropertyLoad(
  vector: FeedbackSlot[],
  slot: number,
  hiddenClassId: number,
): void {
  const pair: TypePair = [`hc:${hiddenClassId}`, `hc:${hiddenClassId}`];
  _recordPair(vector, slot, pair);
}

/**
 * Record the callee type seen at a call-site feedback slot.
 *
 * @param vector     The frame's feedback vector.
 * @param slot       Slot index to update.
 * @param calleeType Type string of the callee (e.g. 'function', 'object').
 */
export function recordCallSite(
  vector: FeedbackSlot[],
  slot: number,
  calleeType: string,
): void {
  const pair: TypePair = [calleeType, calleeType];
  _recordPair(vector, slot, pair);
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Core of the feedback state machine.
 * Applies the transition rules for a newly observed TypePair.
 */
function _recordPair(vector: FeedbackSlot[], slot: number, pair: TypePair): void {
  if (slot < 0 || slot >= vector.length) return;

  const current = vector[slot];

  switch (current.kind) {
    case 'uninitialized':
      // First observation: transition to monomorphic.
      vector[slot] = { kind: 'monomorphic', types: [pair] };
      break;

    case 'monomorphic': {
      // Already have one type pair.  If it's the same, do nothing (stay monomorphic).
      if (_pairEquals(current.types[0], pair)) return;
      // Different pair seen — transition to polymorphic.
      vector[slot] = { kind: 'polymorphic', types: [pair, ...current.types] };
      break;
    }

    case 'polymorphic': {
      // Check if we've already seen this pair.
      if (current.types.some(p => _pairEquals(p, pair))) return;
      // New pair seen.  If we'd exceed 4, transition to megamorphic.
      if (current.types.length >= 4) {
        vector[slot] = { kind: 'megamorphic' };
      } else {
        // Still within polymorphic budget — add the new pair.
        vector[slot] = { kind: 'polymorphic', types: [pair, ...current.types] };
      }
      break;
    }

    case 'megamorphic':
      // Terminal state — nothing more to record.
      break;
  }
}

/**
 * Deep equality for TypePair tuples.
 */
function _pairEquals(a: TypePair, b: TypePair): boolean {
  return a[0] === b[0] && a[1] === b[1];
}
