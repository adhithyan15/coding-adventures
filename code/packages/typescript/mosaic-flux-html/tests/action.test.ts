// action.test.ts

import { describe, it, expect } from "vitest";
import { isMosaicAction, type MosaicAction } from "../src/action.js";

interface CounterState {
  count: number;
}

class Increment implements MosaicAction<CounterState> {
  apply(state: CounterState): CounterState {
    return { count: state.count + 1 };
  }
}

class Add implements MosaicAction<CounterState> {
  constructor(public readonly amount: number) {}
  apply(state: CounterState): CounterState {
    return { count: state.count + this.amount };
  }
}

describe("MosaicAction", () => {
  it("apply returns next state without mutating input", () => {
    const initial: CounterState = { count: 5 };
    expect(new Increment().apply(initial)).toEqual({ count: 6 });
    expect(initial).toEqual({ count: 5 });
  });

  it("payload accessible from instance", () => {
    const action = new Add(7);
    expect(action.amount).toBe(7);
    expect(action.apply({ count: 3 })).toEqual({ count: 10 });
  });

  it("deterministic", () => {
    const state: CounterState = { count: 0 };
    const action = new Add(5);
    expect(action.apply(state)).toEqual(action.apply(state));
  });
});

describe("isMosaicAction", () => {
  it("true for action instances", () => {
    expect(isMosaicAction(new Increment())).toBe(true);
    expect(isMosaicAction(new Add(1))).toBe(true);
  });

  it("true for plain objects with apply function (structural)", () => {
    expect(isMosaicAction({ apply: () => ({}) })).toBe(true);
  });

  it("false for non-actions", () => {
    expect(isMosaicAction(null)).toBe(false);
    expect(isMosaicAction(undefined)).toBe(false);
    expect(isMosaicAction(42)).toBe(false);
    expect(isMosaicAction("s")).toBe(false);
    expect(isMosaicAction({})).toBe(false);
    expect(isMosaicAction({ apply: "not a fn" })).toBe(false);
  });
});
