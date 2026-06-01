// selector.ts — memoised derived-state combinator.

import type { Equality } from "./store.js";

const defaultEquality: Equality<unknown> = (a, b) => Object.is(a, b);

export function createSelector<State, R>(
  inputSelector: (state: State) => unknown,
  combiner: (...args: unknown[]) => R,
  equality?: Equality<unknown>,
): (state: State) => R;

export function createSelector<State, R>(
  ...args: ReadonlyArray<unknown>
): (state: State) => R {
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
