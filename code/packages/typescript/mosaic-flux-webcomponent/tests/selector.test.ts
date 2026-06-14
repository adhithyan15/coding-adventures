// selector.test.ts

import { describe, it, expect, vi } from "vitest";
import { createSelector } from "../src/selector.js";

interface S {
  a: number;
  b: number;
  label: string;
}

describe("createSelector", () => {
  it("throws when too few args", () => {
    expect(() => (createSelector as unknown as () => unknown)()).toThrow();
    expect(() =>
      (createSelector as unknown as (fn: () => unknown) => unknown)(() => 1),
    ).toThrow();
  });

  it("recomputes when input changes", () => {
    const combiner = vi.fn((a: number, b: number) => a + b);
    const sum = createSelector(
      (s: S) => s.a,
      (s: S) => s.b,
      combiner,
    );
    expect(sum({ a: 1, b: 2, label: "" })).toBe(3);
    expect(sum({ a: 5, b: 2, label: "" })).toBe(7);
    expect(combiner).toHaveBeenCalledTimes(2);
  });

  it("caches on stable inputs", () => {
    const combiner = vi.fn((a: number, b: number) => a + b);
    const sum = createSelector(
      (s: S) => s.a,
      (s: S) => s.b,
      combiner,
    );
    const state: S = { a: 1, b: 2, label: "" };
    sum(state);
    sum(state);
    sum(state);
    expect(combiner).toHaveBeenCalledTimes(1);
  });

  it("caches across different state refs with same projected inputs", () => {
    const combiner = vi.fn((a: number) => a * 2);
    const doubled = createSelector((s: S) => s.a, combiner);
    doubled({ a: 5, b: 0, label: "" });
    doubled({ a: 5, b: 99, label: "different" });
    expect(combiner).toHaveBeenCalledTimes(1);
  });

  it("re-runs when one of multiple inputs changes", () => {
    const combiner = vi.fn(
      (a: number, b: number, lbl: string) => `${lbl}:${a + b}`,
    );
    const fmt = createSelector(
      (s: S) => s.a,
      (s: S) => s.b,
      (s: S) => s.label,
      combiner,
    );
    fmt({ a: 1, b: 2, label: "x" });
    fmt({ a: 1, b: 2, label: "x" });
    fmt({ a: 1, b: 2, label: "y" });
    fmt({ a: 1, b: 2, label: "y" });
    expect(combiner).toHaveBeenCalledTimes(2);
  });
});
