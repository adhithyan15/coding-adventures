import { describe, it, expect, vi } from "vitest";
import { Store } from "../store.js";
import type { Action, Middleware } from "../types.js";

// ---------------------------------------------------------------------------
// Test reducer — a simple counter.
//
// Handles three action types:
//   INCREMENT → count + 1
//   DECREMENT → count - 1
//   SET       → count = action.value
//   (default) → return state unchanged
// ---------------------------------------------------------------------------

interface CounterState {
  count: number;
}

const counterReducer = (state: CounterState, action: Action): CounterState => {
  switch (action.type) {
    case "INCREMENT":
      return { count: state.count + 1 };
    case "DECREMENT":
      return { count: state.count - 1 };
    case "SET":
      return { count: action.value as number };
    default:
      return state;
  }
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("Store", () => {
  // === getState ===

  it("getState returns the initial state", () => {
    const store = new Store({ count: 0 }, counterReducer);
    expect(store.getState()).toEqual({ count: 0 });
  });

  // === dispatch ===

  it("dispatch runs the reducer and updates state", () => {
    const store = new Store({ count: 0 }, counterReducer);
    store.dispatch({ type: "INCREMENT" });
    expect(store.getState()).toEqual({ count: 1 });
  });

  it("dispatch with an unknown action type returns the same state", () => {
    const store = new Store({ count: 5 }, counterReducer);
    const stateBefore = store.getState();
    store.dispatch({ type: "UNKNOWN_ACTION" });
    expect(store.getState()).toBe(stateBefore);
  });

  it("dispatch processes multiple actions sequentially", () => {
    const store = new Store({ count: 0 }, counterReducer);
    store.dispatch({ type: "INCREMENT" });
    store.dispatch({ type: "INCREMENT" });
    store.dispatch({ type: "DECREMENT" });
    expect(store.getState()).toEqual({ count: 1 });
  });

  it("dispatch with SET action uses the payload value", () => {
    const store = new Store({ count: 0 }, counterReducer);
    store.dispatch({ type: "SET", value: 42 });
    expect(store.getState()).toEqual({ count: 42 });
  });

  // === subscribe ===

  it("subscribe listener is called after dispatch", () => {
    const store = new Store({ count: 0 }, counterReducer);
    const listener = vi.fn();
    store.subscribe(listener);
    store.dispatch({ type: "INCREMENT" });
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("unsubscribe stops the listener from being called", () => {
    const store = new Store({ count: 0 }, counterReducer);
    const listener = vi.fn();
    const unsubscribe = store.subscribe(listener);
    store.dispatch({ type: "INCREMENT" });
    expect(listener).toHaveBeenCalledTimes(1);

    unsubscribe();
    store.dispatch({ type: "INCREMENT" });
    expect(listener).toHaveBeenCalledTimes(1); // still 1, not 2
  });

  it("multiple listeners are all called after dispatch", () => {
    const store = new Store({ count: 0 }, counterReducer);
    const listener1 = vi.fn();
    const listener2 = vi.fn();
    const listener3 = vi.fn();
    store.subscribe(listener1);
    store.subscribe(listener2);
    store.subscribe(listener3);

    store.dispatch({ type: "INCREMENT" });

    expect(listener1).toHaveBeenCalledTimes(1);
    expect(listener2).toHaveBeenCalledTimes(1);
    expect(listener3).toHaveBeenCalledTimes(1);
  });

  it("listeners can read the updated state via getState()", () => {
    const store = new Store({ count: 0 }, counterReducer);
    let stateInListener: CounterState | undefined;
    store.subscribe(() => {
      stateInListener = store.getState();
    });
    store.dispatch({ type: "SET", value: 99 });
    expect(stateInListener).toEqual({ count: 99 });
  });

  // === middleware ===

  it("middleware intercepts dispatch and can read state", () => {
    const store = new Store({ count: 0 }, counterReducer);
    const states: CounterState[] = [];

    const loggingMiddleware: Middleware<CounterState> = (api, _action, next) => {
      states.push(api.getState());
      next();
      states.push(api.getState());
    };

    store.use(loggingMiddleware);
    store.dispatch({ type: "INCREMENT" });

    // Before reducer: count = 0, after reducer: count = 1
    expect(states).toEqual([{ count: 0 }, { count: 1 }]);
  });

  it("middleware calling next() passes the action to the reducer", () => {
    const store = new Store({ count: 0 }, counterReducer);
    const passThrough: Middleware<CounterState> = (_api, _action, next) => {
      next();
    };
    store.use(passThrough);
    store.dispatch({ type: "INCREMENT" });
    expect(store.getState()).toEqual({ count: 1 });
  });

  it("middleware NOT calling next() swallows the action", () => {
    const store = new Store({ count: 0 }, counterReducer);
    const swallower: Middleware<CounterState> = (_api, _action, _next) => {
      // intentionally not calling next()
    };
    store.use(swallower);
    store.dispatch({ type: "INCREMENT" });
    expect(store.getState()).toEqual({ count: 0 }); // unchanged
  });

  it("multiple middlewares run in order (first added = outermost)", () => {
    const store = new Store({ count: 0 }, counterReducer);
    const order: string[] = [];

    const mw1: Middleware<CounterState> = (_api, _action, next) => {
      order.push("mw1-before");
      next();
      order.push("mw1-after");
    };

    const mw2: Middleware<CounterState> = (_api, _action, next) => {
      order.push("mw2-before");
      next();
      order.push("mw2-after");
    };

    store.use(mw1);
    store.use(mw2);
    store.dispatch({ type: "INCREMENT" });

    // mw1 is outermost, mw2 is inner, reducer is innermost
    expect(order).toEqual(["mw1-before", "mw2-before", "mw2-after", "mw1-after"]);
  });

  it("middleware can read state after reducer via getState()", () => {
    const store = new Store({ count: 0 }, counterReducer);
    let afterState: CounterState | undefined;

    const mw: Middleware<CounterState> = (api, _action, next) => {
      next();
      afterState = api.getState();
    };

    store.use(mw);
    store.dispatch({ type: "SET", value: 77 });
    expect(afterState).toEqual({ count: 77 });
  });

  it("dispatch inside middleware works (re-entrant dispatch)", () => {
    const store = new Store({ count: 0 }, counterReducer);

    // This middleware dispatches a second action when it sees "DOUBLE_INCREMENT"
    const doubler: Middleware<CounterState> = (api, action, next) => {
      next();
      if (action.type === "DOUBLE_INCREMENT") {
        api.dispatch({ type: "INCREMENT" });
      }
    };

    store.use(doubler);
    store.dispatch({ type: "DOUBLE_INCREMENT" });

    // The unknown "DOUBLE_INCREMENT" passes through reducer unchanged (count stays 0),
    // then middleware dispatches INCREMENT (count becomes 1).
    // But wait — "DOUBLE_INCREMENT" is unknown, so reducer returns state unchanged.
    // Then the re-entrant INCREMENT makes count = 1.
    expect(store.getState()).toEqual({ count: 1 });
  });

  it("middleware receives the correct action object", () => {
    const store = new Store({ count: 0 }, counterReducer);
    let capturedAction: Action | undefined;

    const spy: Middleware<CounterState> = (_api, action, next) => {
      capturedAction = action;
      next();
    };

    store.use(spy);
    store.dispatch({ type: "SET", value: 10 });
    expect(capturedAction).toEqual({ type: "SET", value: 10 });
  });

  it("listeners are notified even when middleware swallows the action", () => {
    // This verifies that listeners fire after the middleware chain completes,
    // regardless of whether the reducer ran. The state won't change, but
    // listeners are still called.
    const store = new Store({ count: 0 }, counterReducer);
    const listener = vi.fn();
    store.subscribe(listener);

    const swallower: Middleware<CounterState> = (_api, _action, _next) => {
      // swallow
    };
    store.use(swallower);

    store.dispatch({ type: "INCREMENT" });
    expect(listener).toHaveBeenCalledTimes(1);
    expect(store.getState()).toEqual({ count: 0 }); // state unchanged
  });
});
