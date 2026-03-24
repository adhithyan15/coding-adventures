/**
 * @coding-adventures/store
 *
 * Flux-like event-driven state store with middleware and a React hook.
 *
 * Usage:
 * ```typescript
 * import { Store, useStore } from "@coding-adventures/store";
 * import type { Action, Reducer, Middleware } from "@coding-adventures/store";
 *
 * const store = new Store(initialState, reducer);
 * store.use(loggingMiddleware);
 * store.dispatch({ type: "INCREMENT" });
 *
 * // In a React component:
 * function Counter() {
 *   const state = useStore(store);
 *   return <p>{state.count}</p>;
 * }
 * ```
 */

export type { Action, Reducer, Listener, Middleware } from "./types.js";
export { Store } from "./store.js";
export { useStore } from "./use-store.js";
