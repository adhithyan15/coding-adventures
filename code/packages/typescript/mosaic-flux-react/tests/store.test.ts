// store.test.ts — MosaicStore dispatch, state, subscribe, select.

import { describe, it, expect, vi } from "vitest";
import { MosaicStore } from "../src/store.js";
import type { MosaicAction } from "../src/action.js";

interface CounterState {
  count: number;
  label: string;
}

class Increment implements MosaicAction<CounterState> {
  apply(s: CounterState): CounterState {
    return { ...s, count: s.count + 1 };
  }
}

class SetLabel implements MosaicAction<CounterState> {
  constructor(public readonly label: string) {}
  apply(s: CounterState): CounterState {
    return { ...s, label: this.label };
  }
}

class NoOp implements MosaicAction<CounterState> {
  apply(s: CounterState): CounterState {
    return s; // returns the SAME reference; store should skip subscriber fire
  }
}

const initial: CounterState = { count: 0, label: "" };

describe("MosaicStore", () => {
  it("starts at the provided initial state", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    expect(store.state).toEqual(initial);
  });

  it("dispatch applies the action and updates state", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    store.dispatch(new Increment());
    expect(store.state).toEqual({ count: 1, label: "" });
  });

  it("supports actions with payload", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    store.dispatch(new SetLabel("hello"));
    expect(store.state.label).toBe("hello");
  });

  it("select returns the projected slice without subscribing", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    store.dispatch(new SetLabel("test"));
    expect(store.select((s) => s.label)).toBe("test");
  });
});

describe("MosaicStore.subscribe — fine-grained subscription", () => {
  it("fires subscriber when the selected slice changes", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    const cb = vi.fn();
    store.subscribe((s) => s.count, cb);
    store.dispatch(new Increment());
    expect(cb).toHaveBeenCalledTimes(1);
    expect(cb).toHaveBeenCalledWith(1);
  });

  it("does NOT fire when an unrelated slice changes", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    const countCb = vi.fn();
    store.subscribe((s) => s.count, countCb);
    store.dispatch(new SetLabel("anything"));
    // count didn't change, so countCb should not fire
    expect(countCb).not.toHaveBeenCalled();
  });

  it("fires multiple subscribers in registration order", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    const calls: string[] = [];
    store.subscribe(
      (s) => s.count,
      () => calls.push("first"),
    );
    store.subscribe(
      (s) => s.count,
      () => calls.push("second"),
    );
    store.dispatch(new Increment());
    expect(calls).toEqual(["first", "second"]);
  });

  it("unsubscribe stops further notifications", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    const cb = vi.fn();
    const unsub = store.subscribe((s) => s.count, cb);
    store.dispatch(new Increment());
    unsub();
    store.dispatch(new Increment());
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it("a subscriber unsubscribing during its own callback does not skip peers", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    const calls: string[] = [];
    const unsubFirst: () => void = (() => {
      let u: () => void = () => {};
      u = store.subscribe(
        (s) => s.count,
        () => {
          calls.push("first");
          unsubFirst();
        },
      );
      return u;
    })();
    void unsubFirst;
    store.subscribe(
      (s) => s.count,
      () => calls.push("second"),
    );
    store.dispatch(new Increment());
    // Both should fire on the dispatch where first unsubscribes
    expect(calls).toEqual(["first", "second"]);
  });

  it("supports custom equality functions", () => {
    interface ArrayState {
      list: number[];
    }
    const store = new MosaicStore<ArrayState>({
      initialState: { list: [1, 2, 3] },
    });
    const cb = vi.fn();
    // Use deep equality so semantically-same arrays don't trigger
    const deepEqual = (a: number[], b: number[]): boolean =>
      a.length === b.length && a.every((v, i) => v === b[i]);
    store.subscribe(
      (s) => s.list,
      cb,
      deepEqual as unknown as (a: number[], b: number[]) => boolean,
    );
    // Dispatch an action that returns a new array with same contents
    class ReplaceList implements MosaicAction<ArrayState> {
      apply(s: ArrayState): ArrayState {
        return { list: [...s.list] };
      }
    }
    store.dispatch(new ReplaceList());
    expect(cb).not.toHaveBeenCalled();
  });

  it("no-op action (apply returns same reference) skips subscriber notification", () => {
    const store = new MosaicStore<CounterState>({ initialState: initial });
    const cb = vi.fn();
    store.subscribe((s) => s.count, cb);
    store.dispatch(new NoOp());
    expect(cb).not.toHaveBeenCalled();
  });
});

describe("MosaicStore.middleware", () => {
  it("middleware sees (action, prevState, nextState) triples", () => {
    const seen: Array<{ action: unknown; prev: CounterState; next: CounterState }> =
      [];
    const store = new MosaicStore<CounterState>({
      initialState: initial,
      middleware: [
        (action, prev, next) => seen.push({ action, prev, next }),
      ],
    });
    const action = new Increment();
    store.dispatch(action);
    expect(seen).toHaveLength(1);
    expect(seen[0]?.action).toBe(action);
    expect(seen[0]?.prev).toEqual({ count: 0, label: "" });
    expect(seen[0]?.next).toEqual({ count: 1, label: "" });
  });

  it("middleware runs even for no-op dispatches (loggers still see them)", () => {
    const seen: number = 0;
    let counter = seen;
    const store = new MosaicStore<CounterState>({
      initialState: initial,
      middleware: [() => counter++],
    });
    store.dispatch(new NoOp());
    expect(counter).toBe(1);
  });
});
