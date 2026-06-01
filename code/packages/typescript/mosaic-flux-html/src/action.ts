// action.ts — the MosaicAction Command Pattern interface.
//
// Per UI33-rewrite §5: each action is a class with a payload (its
// constructor arguments) and an `apply(state) → state` method that
// expresses the state transform. The dispatcher routes by calling
// `action.apply(state)` directly — there is no central switch
// statement or reducer registry.
//
// This interface is identical to the one in mosaic-flux-react.
// Eventually we may extract the shared core into mosaic-flux-core
// and depend on it across HTML / WebComponent / React; v0.1.0 keeps
// the packages standalone to avoid coupling release cadences while
// the design is still settling.

export interface MosaicAction<State> {
  /**
   * Pure state transform. Must not mutate input. Must be deterministic.
   * Side effects belong in middleware, not apply.
   */
  apply(state: State): State;
}

/**
 * Structural type guard: anything with an apply function is treated
 * as an action.
 */
export function isMosaicAction<State>(
  value: unknown,
): value is MosaicAction<State> {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as { apply?: unknown }).apply === "function"
  );
}
