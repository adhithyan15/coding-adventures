/**
 * use-store.ts — React hook for subscribing to a Store.
 *
 * useStore(store) returns the current state and re-renders the component
 * whenever the store dispatches an action that changes state.
 *
 * === How it works ===
 *
 * React 19 provides useSyncExternalStore — a hook designed exactly for
 * subscribing to external (non-React) state sources. We use it here
 * because our Store is external to React's state management.
 *
 * useSyncExternalStore takes two functions:
 *   1. subscribe(callback) — called once on mount. We register the callback
 *      as a store listener. Return an unsubscribe function for cleanup.
 *   2. getSnapshot() — called on every render. Returns the current state.
 *      React compares snapshots by reference (Object.is) to decide if
 *      re-render is needed.
 *
 * This is the modern, correct way to bridge external stores to React.
 * The older pattern (useState + useEffect + subscribe) has subtle bugs
 * with concurrent mode that useSyncExternalStore fixes.
 */

import { useSyncExternalStore } from "react";
import type { Store } from "./store.js";

export function useStore<S>(store: Store<S>): S {
  return useSyncExternalStore(
    // subscribe: register a listener, return unsubscribe
    (callback) => store.subscribe(callback),
    // getSnapshot: return current state for React to compare
    () => store.getState(),
  );
}
