// store.ts — the MosaicStore: state container + dispatcher.
//
// The MosaicStore is the runtime's center of gravity. It holds the
// current state, accepts action dispatches, runs middleware, and
// notifies subscribers when state changes.
//
// Key design choices vs. classical Redux:
//
//   1. No central reducer. The store's `dispatch(action)` calls
//      `action.apply(state)` directly — the action object is its
//      own reducer fragment. This is the Command Pattern (UI33-
//      rewrite §5). Adding a new action means adding a new class;
//      the store needs zero changes.
//
//   2. Fine-grained subscription. `subscribe` takes a selector and
//      only fires the callback when the selected slice changes
//      (by reference). This is what makes per-keystroke dispatch
//      cheap: components reading `edit-content` re-render; the
//      2,599 cells reading their own values don't.
//
//   3. No global singleton. Each MosaicStore is a fresh instance.
//      Multi-store apps just instantiate multiple stores. Tests
//      construct their own.
//
//   4. Synchronous dispatch. The action's `apply` runs immediately,
//      state swaps, subscribers fire — all on the dispatch thread.
//      Async work happens in middleware (which can schedule its own
//      dispatches later) or in host extension points called from
//      within `apply` (but the host MUST keep `apply` pure; see
//      action.ts).

import type { MosaicAction } from "./action.js";
import type { Middleware } from "./middleware.js";
import { composeMiddleware } from "./middleware.js";

/**
 * Subscriber callback: called when the selected slice of state
 * changes (by `Object.is` reference equality, unless a custom
 * comparator is provided).
 */
export type Subscriber<T> = (value: T) => void;

/**
 * Equality comparator for selectors. Defaults to `Object.is` which
 * matches React's default behaviour for hooks. Hosts can provide
 * a deep-equality comparator for slices that are recomputed each
 * render but logically unchanged.
 */
export type Equality<T> = (a: T, b: T) => boolean;

const defaultEquality: Equality<unknown> = (a, b) => Object.is(a, b);

/**
 * Options for store construction.
 */
export interface MosaicStoreOptions<State> {
  /**
   * Initial state. The store holds this until the first dispatch.
   */
  initialState: State;

  /**
   * Middleware to run after every successful dispatch. Optional —
   * the store works without any.
   */
  middleware?: ReadonlyArray<Middleware<State>>;
}

/**
 * The core store. Generic over the state type only — action types
 * are determined per-call (Action is a covariant union of all
 * action classes that target this state shape).
 */
export class MosaicStore<State> {
  #state: State;
  readonly #middleware: Middleware<State>;
  readonly #subscriptions: Set<InternalSubscription<State, unknown>> = new Set();

  constructor(options: MosaicStoreOptions<State>) {
    this.#state = options.initialState;
    this.#middleware = composeMiddleware(options.middleware ?? []);
  }

  /**
   * The current state. Read-only from the consumer's perspective;
   * mutations only happen via `dispatch`.
   */
  get state(): State {
    return this.#state;
  }

  /**
   * Dispatch an action. Runs the action's `apply`, swaps state,
   * notifies subscribers whose selected slice changed, and runs
   * middleware.
   *
   * Synchronous. By the time `dispatch` returns, all subscribers
   * whose slices changed have been notified.
   */
  dispatch<A extends MosaicAction<State>>(action: A): void {
    const prevState = this.#state;
    const nextState = action.apply(prevState);
    if (Object.is(prevState, nextState)) {
      // No-op transforms still run middleware (so loggers see them)
      // but skip subscriber notification.
      this.#middleware(action, prevState, nextState);
      return;
    }
    this.#state = nextState;
    // Notify subscribers whose selected slice actually changed.
    // We snapshot the set first so a subscriber that unsubscribes
    // during its own callback doesn't perturb iteration.
    const snapshot = Array.from(this.#subscriptions);
    for (const sub of snapshot) {
      sub.notifyIfChanged(prevState, nextState);
    }
    this.#middleware(action, prevState, nextState);
  }

  /**
   * Subscribe to a slice of state via a selector function. The
   * callback fires when the selected value changes (by `equality`,
   * default `Object.is`).
   *
   * Returns an unsubscribe function.
   *
   * Fine-grained subscription is what makes strict Flux cheap. A
   * component reading only `state.editContent` subscribes with
   * `(s) => s.editContent`; the store calls its callback only when
   * that field changes, not on every state update.
   */
  subscribe<T>(
    selector: (state: State) => T,
    callback: Subscriber<T>,
    equality: Equality<T> = defaultEquality as Equality<T>,
  ): () => void {
    const sub = new InternalSubscription<State, T>(
      selector,
      callback,
      equality,
      selector(this.#state),
    );
    // Erase the value-type parameter at the set boundary. Each
    // subscription stores its own selector/callback pair so type
    // identity inside the set is unimportant; only the notify-on-
    // change protocol matters.
    this.#subscriptions.add(sub as unknown as InternalSubscription<State, unknown>);
    return () => {
      this.#subscriptions.delete(sub as unknown as InternalSubscription<State, unknown>);
    };
  }

  /**
   * Select a slice of state without subscribing. Used by callers
   * that want a one-shot read (e.g., for an event handler that
   * needs current state at dispatch time but doesn't need updates).
   */
  select<T>(selector: (state: State) => T): T {
    return selector(this.#state);
  }
}

/**
 * Internal subscription bookkeeping. Keeps the last-seen value so
 * we can compare on each dispatch.
 */
class InternalSubscription<State, T> {
  #lastValue: T;
  constructor(
    private readonly selector: (state: State) => T,
    private readonly callback: Subscriber<T>,
    private readonly equality: Equality<T>,
    initial: T,
  ) {
    this.#lastValue = initial;
  }

  notifyIfChanged(_prevState: State, nextState: State): void {
    const nextValue = this.selector(nextState);
    if (!this.equality(this.#lastValue, nextValue)) {
      this.#lastValue = nextValue;
      this.callback(nextValue);
    }
  }
}
