// store.ts — MosaicStore: state container + dispatcher.

import type { MosaicAction } from "./action.js";
import type { Middleware } from "./middleware.js";
import { composeMiddleware } from "./middleware.js";

export type Subscriber<T> = (value: T) => void;
export type Equality<T> = (a: T, b: T) => boolean;

const defaultEquality: Equality<unknown> = (a, b) => Object.is(a, b);

export interface MosaicStoreOptions<State> {
  initialState: State;
  middleware?: ReadonlyArray<Middleware<State>>;
}

/**
 * The core store. Synchronous dispatch; fine-grained subscriptions
 * via selectors. Middleware sees every (action, prevState, nextState)
 * triple even when state didn't change (so loggers see no-ops).
 */
export class MosaicStore<State> {
  #state: State;
  readonly #middleware: Middleware<State>;
  readonly #subscriptions: Set<InternalSubscription<State, unknown>> = new Set();

  constructor(options: MosaicStoreOptions<State>) {
    this.#state = options.initialState;
    this.#middleware = composeMiddleware(options.middleware ?? []);
  }

  get state(): State {
    return this.#state;
  }

  dispatch<A extends MosaicAction<State>>(action: A): void {
    const prevState = this.#state;
    const nextState = action.apply(prevState);
    if (Object.is(prevState, nextState)) {
      this.#middleware(action, prevState, nextState);
      return;
    }
    this.#state = nextState;
    const snapshot = Array.from(this.#subscriptions);
    for (const sub of snapshot) {
      sub.notifyIfChanged(prevState, nextState);
    }
    this.#middleware(action, prevState, nextState);
  }

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
    this.#subscriptions.add(sub as unknown as InternalSubscription<State, unknown>);
    return () => {
      this.#subscriptions.delete(sub as unknown as InternalSubscription<State, unknown>);
    };
  }

  select<T>(selector: (state: State) => T): T {
    return selector(this.#state);
  }
}

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
