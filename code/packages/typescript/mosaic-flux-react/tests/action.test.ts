// action.test.ts — MosaicAction interface and type guard.

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
  it("apply returns the new state without mutating input", () => {
    const initial: CounterState = { count: 5 };
    const next = new Increment().apply(initial);
    expect(next).toEqual({ count: 6 });
    expect(initial).toEqual({ count: 5 }); // input unchanged
  });

  it("payload is accessible from the action instance", () => {
    const action = new Add(7);
    expect(action.amount).toBe(7);
    expect(action.apply({ count: 3 })).toEqual({ count: 10 });
  });

  it("is deterministic — same state + action produces same next state", () => {
    const state: CounterState = { count: 0 };
    const action = new Add(5);
    const a = action.apply(state);
    const b = action.apply(state);
    expect(a).toEqual(b);
  });
});

describe("isMosaicAction", () => {
  it("returns true for action instances", () => {
    expect(isMosaicAction(new Increment())).toBe(true);
    expect(isMosaicAction(new Add(1))).toBe(true);
  });

  it("returns true for plain objects with an apply method", () => {
    // The contract is structural; anything with apply is treated as
    // an action. This is intentional — author-controlled classes
    // don't need to extend a base.
    expect(isMosaicAction({ apply: () => ({}) })).toBe(true);
  });

  it("returns false for non-action values", () => {
    expect(isMosaicAction(null)).toBe(false);
    expect(isMosaicAction(undefined)).toBe(false);
    expect(isMosaicAction(42)).toBe(false);
    expect(isMosaicAction("string")).toBe(false);
    expect(isMosaicAction({})).toBe(false);
    expect(isMosaicAction({ apply: "not a function" })).toBe(false);
  });
});
