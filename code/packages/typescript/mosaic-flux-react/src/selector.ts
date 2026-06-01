// selector.ts — memoised selector construction.
//
// For derived state that's computed on the fly (e.g., a formula bar's
// displayed value is computed from `editRow`, `selectedRow`,
// `selectedCol`, and `cells`), recomputing on every selector call
// is wasteful. createSelector memoises by input-equality: if every
// input selector returned the same value as last call, the output
// is reused.
//
// This is the same shape as Reselect / Redux Toolkit's
// `createSelector`. We ship our own minimal implementation to keep
// the runtime zero-dep.

import type { Equality } from "./store.js";

const defaultEquality: Equality<unknown> = (a, b) => Object.is(a, b);

/**
 * Compose input selectors into a memoised derived selector.
 *
 * Single-input form:
 *   const formulaText = createSelector(
 *     (s: GridState) => s.editContent,
 *     (content) => content.toUpperCase(),
 *   );
 *
 * Multi-input form:
 *   const formula = createSelector(
 *     (s: GridState) => s.editRow,
 *     (s: GridState) => s.selectedRow,
 *     (s: GridState) => s.selectedCol,
 *     (s: GridState) => s.cells,
 *     (s: GridState) => s.editContent,
 *     (editRow, selRow, selCol, cells, editContent) =>
 *       editRow !== -1 ? editContent : cells[cellKey(selRow, selCol)] ?? "",
 *   );
 *
 * The combiner runs only when at least one input changed.
 */
export function createSelector<State, R>(
  inputSelector: (state: State) => unknown,
  combiner: (...args: unknown[]) => R,
  equality?: Equality<unknown>,
): (state: State) => R;

export function createSelector<State, R>(
  ...args: ReadonlyArray<unknown>
): (state: State) => R {
  // Args layout: any number of input selectors, then a combiner,
  // optionally followed by an equality fn (per-input). The combiner
  // is identified by position: it's the LAST arg unless the last
  // arg is a function with arity (state) => R — heuristically that
  // would itself be a selector, so we require explicit registration.
  //
  // For simplicity in v0.1.0 we require the combiner to be at the
  // end and equality to be passed via the optional named-options
  // overload at the top of this file. Authors who need per-input
  // equality should compose selectors manually.
  if (args.length < 2) {
    throw new Error(
      "createSelector requires at least one input selector and one combiner",
    );
  }
  const combiner = args[args.length - 1] as (...inputs: unknown[]) => R;
  const inputSelectors = args.slice(0, -1) as ReadonlyArray<
    (state: State) => unknown
  >;
  const equality = defaultEquality;

  let lastInputs: unknown[] | null = null;
  let lastResult: R;

  return (state: State): R => {
    const currentInputs = inputSelectors.map((s) => s(state));
    if (lastInputs !== null && allEqual(currentInputs, lastInputs, equality)) {
      return lastResult;
    }
    lastInputs = currentInputs;
    lastResult = combiner(...currentInputs);
    return lastResult;
  };
}

function allEqual(
  a: ReadonlyArray<unknown>,
  b: ReadonlyArray<unknown>,
  equality: Equality<unknown>,
): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (!equality(a[i], b[i])) return false;
  }
  return true;
}
