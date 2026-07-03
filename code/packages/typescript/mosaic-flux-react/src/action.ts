// action.ts — the MosaicAction Command Pattern interface.
//
// Per UI33-rewrite §5: each action is a class with a payload (its
// constructor arguments) and an `apply(state) → state` method that
// expresses the state transform. The dispatcher routes by calling
// `action.apply(state)` directly — there is no central switch
// statement or reducer registry, because each action knows how to
// transform state.
//
// Why a class instead of a tagged-union object: each action's
// transform logic is co-located with its payload shape. A new
// developer reading `EditCommit.ts` sees both "what data does this
// carry" and "what does it do to state" in one place. Bug fixes and
// code review happen at the file level. Testing reduces to
// `new EditCommit().apply(testState)`.
//
// The interface is intentionally minimal. No middleware-style hooks;
// those live in the store (see middleware.ts). No async escape hatch;
// effects flow through middleware, not through actions themselves.
// This keeps `apply` pure and the strict-Flux invariant intact.

export interface MosaicAction<State> {
  /**
   * Pure state transform. Given the current state, return the next
   * state. Must not mutate the input. Must be deterministic — given
   * the same state, the same action instance produces the same next
   * state.
   *
   * Side effects (logging, persistence, async) belong in middleware,
   * not in apply.
   */
  apply(state: State): State;
}

/**
 * Type guard: is this thing a MosaicAction? Useful for runtime
 * middleware that wants to introspect dispatch arguments.
 *
 * The check is structural: anything with an `apply` function is
 * treated as an action. We don't require a brand symbol because
 * action classes are author-controlled — if you can dispatch it,
 * you implemented this contract.
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
