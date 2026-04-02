/**
 * state.ts — Global Flux store for the Journal app.
 *
 * Creates the single store instance that holds all application state.
 * Components read state via useStore(store) and dispatch actions via
 * store.dispatch(someAction(...)).
 *
 * The store is a singleton — one instance for the lifetime of the app.
 */

import { Store } from "@coding-adventures/store";
import { reducer, initialState } from "./reducer.js";
import type { AppState } from "./reducer.js";

export type { AppState };

export const store = new Store<AppState>(initialState, reducer);
