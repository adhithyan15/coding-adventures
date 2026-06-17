import { describe, expect, it } from "vitest";

import { Range, includes, isRange, range, toList } from "../src/index.js";

describe("constructor", () => {
  it("builds a range with fields", () => {
    const r = range(1, 5, false);
    expect(r).toBeInstanceOf(Range);
    expect(r.start).toBe(1);
    expect(r.stop).toBe(5);
    expect(r.exclusive).toBe(false);
  });

  it("coerces the exclusive flag to a boolean", () => {
    expect(range(1, 5, null).exclusive).toBe(false);
    expect(range(1, 5, 1).exclusive).toBe(true);
  });
});

describe("iteration", () => {
  it("inclusive range iterates through stop", () => {
    expect([...range(1, 5, false)]).toEqual([1, 2, 3, 4, 5]);
  });

  it("exclusive range stops before stop", () => {
    expect([...range(1, 5, true)]).toEqual([1, 2, 3, 4]);
  });

  it("single-element inclusive range", () => {
    expect([...range(3, 3, false)]).toEqual([3]);
  });

  it("empty exclusive range", () => {
    expect([...range(3, 3, true)]).toEqual([]);
  });

  it("endless range yields forever, consumed lazily", () => {
    const out: number[] = [];
    for (const v of range(10, null, false)) {
      out.push(v);
      if (out.length === 4) break;
    }
    expect(out).toEqual([10, 11, 12, 13]);
  });

  it("beginless range cannot be iterated", () => {
    expect(() => [...range(null, 5, false)]).toThrow(/beginless/);
  });
});

describe("membership", () => {
  it("inclusive membership", () => {
    const r = range(1, 5, false);
    expect(r.includes(1)).toBe(true);
    expect(r.includes(5)).toBe(true);
    expect(r.includes(0)).toBe(false);
    expect(r.includes(6)).toBe(false);
  });

  it("exclusive membership excludes stop", () => {
    const r = range(1, 5, true);
    expect(r.includes(4)).toBe(true);
    expect(r.includes(5)).toBe(false);
  });

  it("endless membership", () => {
    const r = range(10, null, false);
    expect(r.includes(10)).toBe(true);
    expect(r.includes(1_000_000)).toBe(true);
    expect(r.includes(9)).toBe(false);
  });

  it("beginless membership", () => {
    const r = range(null, 5, false);
    expect(r.includes(-100)).toBe(true);
    expect(r.includes(5)).toBe(true);
    expect(r.includes(6)).toBe(false);
  });

  it("includes free function", () => {
    expect(includes(range(1, 5, false), 3)).toBe(true);
    expect(includes(range(1, 5, true), 5)).toBe(false);
  });
});

describe("toList", () => {
  it("materialises", () => {
    expect(toList(range(1, 4, false))).toEqual([1, 2, 3, 4]);
    expect(range(1, 4, true).toList()).toEqual([1, 2, 3]);
  });

  it("throws on an endless range", () => {
    expect(() => range(1, null, false).toList()).toThrow(/endless/);
  });

  it("throws on a beginless range", () => {
    expect(() => range(null, 5, false).toList()).toThrow(/beginless/);
  });
});

describe("isRange", () => {
  it("true for a range", () => {
    expect(isRange(range(1, 5, false))).toBe(true);
  });

  it("false for non-ranges", () => {
    expect(isRange(1)).toBe(false);
    expect(isRange(null)).toBe(false);
    expect(isRange([1, 2, 3])).toBe(false);
  });
});

describe("toString", () => {
  it("inclusive", () => {
    expect(String(range(1, 5, false))).toBe("1..5");
  });

  it("exclusive", () => {
    expect(String(range(1, 5, true))).toBe("1...5");
  });

  it("endless", () => {
    expect(String(range(1, null, false))).toBe("1..");
  });

  it("beginless", () => {
    expect(String(range(null, 5, false))).toBe("..5");
  });
});
