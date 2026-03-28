/**
 * state.ts — Store creation and singleton export.
 *
 * The store is the single source of truth for the entire application.
 * Components read state via useStore(store) and trigger changes via
 * store.dispatch(action).
 *
 * The store is created with an empty initial state. On startup, main.tsx
 * loads tasks, views, and calendars from IndexedDB and dispatches STATE_LOAD
 * to hydrate. Until then, the app renders with empty collections (showing
 * appropriate empty states).
 */

import { Store } from "@coding-adventures/store";
import { reducer } from "./reducer.js";
import type { AppState } from "./reducer.js";

export type { AppState };

/** The singleton store used by the running app. */
export const store = new Store<AppState>(
  { tasks: [], views: [], calendars: [], activeViewId: "" },
  reducer,
);
