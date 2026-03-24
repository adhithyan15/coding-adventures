/**
 * state.ts — Store creation and re-exports.
 *
 * V0.3 replaces the mutable singleton with a @coding-adventures/store
 * instance. The store holds immutable state snapshots; all mutations
 * go through dispatch(action) → reducer → new state → listeners.
 *
 * Components import `store` and call:
 *   - useStore(store) to read state reactively
 *   - store.dispatch(action) to trigger state transitions
 *
 * Utility functions (flattenVisibleItems, computeStats, countBranchItems)
 * are re-exported from reducer.ts for convenience — they are pure helpers,
 * not actions.
 */

import { Store } from "@coding-adventures/store";
import { reducer } from "./reducer.js";
import type { AppState } from "./reducer.js";

export type { AppState };
export { flattenVisibleItems, computeStats, countBranchItems } from "./reducer.js";

/** The singleton store used by the running app. */
export const store = new Store<AppState>({ templates: [], instances: [], todos: [] }, reducer);
