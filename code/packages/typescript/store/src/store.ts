/**
 * store.ts — The event-driven state store.
 *
 * This is a minimal reimplementation of the Redux store pattern, built
 * from scratch to understand every piece. The store holds state, processes
 * actions through a reducer, notifies listeners, and supports middleware.
 *
 * === How dispatch works ===
 *
 * When you call store.dispatch(action):
 *
 *   1. The action enters the middleware chain.
 *   2. Each middleware can inspect/modify the action, then call next().
 *   3. The last `next()` calls the reducer: newState = reducer(state, action).
 *   4. The state is replaced with newState.
 *   5. All subscribed listeners are called (they re-render the UI).
 *
 * If no middleware is registered, dispatch goes straight to the reducer.
 *
 * === Why immutability matters ===
 *
 * The reducer must return a NEW state object (or the same reference if
 * nothing changed). Listeners compare old vs new state by reference to
 * decide whether to re-render. If the reducer mutates the existing state
 * object instead of returning a new one, listeners won't detect the change.
 */

import type { Action, Reducer, Listener, Middleware } from "./types.js";

export class Store<S> {
  private state: S;
  private reducer: Reducer<S>;
  private listeners: Set<Listener> = new Set();
  private middlewares: Middleware<S>[] = [];

  constructor(initialState: S, reducer: Reducer<S>) {
    this.state = initialState;
    this.reducer = reducer;
  }

  /**
   * getState — returns the current state snapshot.
   *
   * This is a direct reference, not a copy. Components should treat it
   * as read-only. Mutations go through dispatch().
   */
  getState(): S {
    return this.state;
  }

  /**
   * dispatch — sends an action through the middleware chain and reducer.
   *
   * The middleware chain is built inside-out: the last middleware added
   * runs first (wrapping all previous ones). The innermost function is
   * the actual reducer call.
   */
  dispatch(action: Action): void {
    // Build the middleware chain. Start with the core: run the reducer.
    let chain = () => {
      this.state = this.reducer(this.state, action);
    };

    // Wrap each middleware around the chain, from last to first.
    // This means the first middleware added runs outermost (first to see
    // the action, last to see the new state).
    const storeAPI = {
      getState: () => this.state,
      dispatch: (a: Action) => this.dispatch(a),
    };

    for (let i = this.middlewares.length - 1; i >= 0; i--) {
      const mw = this.middlewares[i]!;
      const nextInChain = chain;
      chain = () => mw(storeAPI, action, nextInChain);
    }

    // Execute the chain
    chain();

    // Notify all listeners after state has been updated
    for (const listener of this.listeners) {
      listener();
    }
  }

  /**
   * subscribe — registers a listener called after every dispatch.
   *
   * Returns an unsubscribe function. Call it to stop receiving notifications.
   * This pattern (returning a cleanup function) matches React's useEffect
   * cleanup convention.
   */
  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  /**
   * use — adds a middleware to the dispatch pipeline.
   *
   * Middleware is called in the order it was added (first added = outermost).
   * Add middleware BEFORE dispatching any actions.
   */
  use(middleware: Middleware<S>): void {
    this.middlewares.push(middleware);
  }
}
