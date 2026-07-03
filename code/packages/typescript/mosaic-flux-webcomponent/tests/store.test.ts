// store.test.ts

import { describe, it, expect, vi } from "vitest";
import { MosaicStore } from "../src/store.js";
import type { MosaicAction } from "../src/action.js";

interface S {
  count: number;
  label: string;
}

class Increment implements MosaicAction<S> {
  apply(s: S): S {
    return { ...s, count: s.count + 1 };
  }
}

class SetLabel implements MosaicAction<S> {
  constructor(public readonly label: string) {}
  apply(s: S): S {
    return { ...s, label: this.label };
  }
}

class NoOp implements MosaicAction<S> {
  apply(s: S): S {
    return s;
  }
}

const initial: S = { count: 0, label: "" };

describe("MosaicStore", () => {
  it("starts at initial state", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    expect(store.state).toEqual(initial);
  });

  it("dispatch applies action", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    store.dispatch(new Increment());
    expect(store.state).toEqual({ count: 1, label: "" });
  });

  it("payloaded actions work", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    store.dispatch(new SetLabel("hi"));
    expect(store.state.label).toBe("hi");
  });

  it("select returns projection without subscribing", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    store.dispatch(new SetLabel("t"));
    expect(store.select((s) => s.label)).toBe("t");
  });
});

describe("MosaicStore — fine-grained subscription", () => {
  it("fires on changed slice", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    const cb = vi.fn();
    store.subscribe((s) => s.count, cb);
    store.dispatch(new Increment());
    expect(cb).toHaveBeenCalledWith(1);
  });

  it("does NOT fire on unrelated slice change", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    const cb = vi.fn();
    store.subscribe((s) => s.count, cb);
    store.dispatch(new SetLabel("x"));
    expect(cb).not.toHaveBeenCalled();
  });

  it("multiple subscribers fire in order", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    const calls: string[] = [];
    store.subscribe(
      (s) => s.count,
      () => calls.push("a"),
    );
    store.subscribe(
      (s) => s.count,
      () => calls.push("b"),
    );
    store.dispatch(new Increment());
    expect(calls).toEqual(["a", "b"]);
  });

  it("unsubscribe stops notifications", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    const cb = vi.fn();
    const unsub = store.subscribe((s) => s.count, cb);
    store.dispatch(new Increment());
    unsub();
    store.dispatch(new Increment());
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it("re-entrant unsubscribe doesn't skip peers", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    const calls: string[] = [];
    let unsubFirst: () => void = () => {};
    unsubFirst = store.subscribe(
      (s) => s.count,
      () => {
        calls.push("first");
        unsubFirst();
      },
    );
    store.subscribe(
      (s) => s.count,
      () => calls.push("second"),
    );
    store.dispatch(new Increment());
    expect(calls).toEqual(["first", "second"]);
  });

  it("custom equality respected", () => {
    interface AS {
      list: number[];
    }
    const store = new MosaicStore<AS>({ initialState: { list: [1, 2] } });
    const cb = vi.fn();
    const deep = (a: number[], b: number[]) =>
      a.length === b.length && a.every((v, i) => v === b[i]);
    store.subscribe((s) => s.list, cb, deep as (a: number[], b: number[]) => boolean);
    class Replace implements MosaicAction<AS> {
      apply(s: AS): AS {
        return { list: [...s.list] };
      }
    }
    store.dispatch(new Replace());
    expect(cb).not.toHaveBeenCalled();
  });

  it("no-op dispatch skips subscriber notification", () => {
    const store = new MosaicStore<S>({ initialState: initial });
    const cb = vi.fn();
    store.subscribe((s) => s.count, cb);
    store.dispatch(new NoOp());
    expect(cb).not.toHaveBeenCalled();
  });
});

describe("MosaicStore — middleware", () => {
  it("sees (action, prev, next) triple", () => {
    const seen: Array<{ a: unknown; p: S; n: S }> = [];
    const store = new MosaicStore<S>({
      initialState: initial,
      middleware: [(a, p, n) => seen.push({ a, p, n })],
    });
    const action = new Increment();
    store.dispatch(action);
    expect(seen[0]?.a).toBe(action);
    expect(seen[0]?.p.count).toBe(0);
    expect(seen[0]?.n.count).toBe(1);
  });

  it("runs on no-op dispatches", () => {
    let count = 0;
    const store = new MosaicStore<S>({
      initialState: initial,
      middleware: [() => count++],
    });
    store.dispatch(new NoOp());
    expect(count).toBe(1);
  });
});
