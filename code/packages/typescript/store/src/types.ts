/**
 * types.ts — Type definitions for the event-driven store.
 *
 * This store implements the Flux architecture pattern, popularized by
 * Facebook in 2014 and later simplified into Redux (2015). The core idea:
 *
 *   1. STATE is a single immutable snapshot of the application.
 *   2. ACTIONS are plain objects describing "what happened" (e.g.,
 *      { type: "CHECK_ITEM", itemId: "abc" }).
 *   3. A REDUCER is a pure function: (state, action) → newState.
 *      It processes the action and returns the next state. No side effects.
 *   4. LISTENERS are callbacks notified after every state change. React
 *      components subscribe as listeners and re-render when state changes.
 *   5. MIDDLEWARE sits between dispatch and the reducer. It can intercept
 *      actions to perform side effects (logging, persistence, analytics)
 *      without polluting the reducer.
 *
 * The data flow is strictly unidirectional:
 *
 *   User action → dispatch(action) → middleware → reducer → new state → listeners
 *
 * This makes state changes predictable and debuggable: every mutation
 * goes through the same pipeline.
 */

/**
 * Action — a plain object describing what happened.
 *
 * The `type` field is the action's identity (e.g., "TEMPLATE_CREATE").
 * Additional fields carry the payload (e.g., templateId, name, items).
 * Actions are the ONLY way to change state — components never mutate
 * state directly.
 */
export interface Action {
  type: string;
  [key: string]: unknown;
}

/**
 * Reducer — a pure function that computes the next state.
 *
 * Given the current state and an action, return the new state.
 * The reducer MUST be pure: no side effects, no async, no randomness.
 * Same input always produces the same output.
 *
 * If the reducer doesn't recognize an action type, it returns the
 * current state unchanged.
 */
export type Reducer<S> = (state: S, action: Action) => S;

/**
 * Listener — a callback invoked after every state change.
 *
 * Listeners are registered via store.subscribe(). The store calls every
 * registered listener after the reducer runs and the state is updated.
 * The listener receives no arguments — it calls store.getState() to
 * read the new state.
 */
export type Listener = () => void;

/**
 * Middleware — a function that intercepts dispatch.
 *
 * Middleware wraps the dispatch pipeline. It receives the store (for
 * reading state), the action being dispatched, and a `next` function
 * that continues the pipeline.
 *
 * Calling `next()` passes the action to the next middleware (or to the
 * reducer if this is the last middleware). NOT calling `next()` swallows
 * the action — the reducer never sees it.
 *
 * Common uses: logging, persistence, analytics, async side effects.
 *
 * Example: a logging middleware:
 *   (store, action, next) => {
 *     console.log("Before:", store.getState());
 *     next();
 *     console.log("After:", store.getState());
 *   }
 */
export type Middleware<S> = (
  store: { getState: () => S; dispatch: (action: Action) => void },
  action: Action,
  next: () => void,
) => void;
