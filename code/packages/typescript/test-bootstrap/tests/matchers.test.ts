import { describe, it } from "node:test";
import { expect, AssertionError } from "../dist/matchers.js";
import { vi } from "../dist/runner.js";

describe("Level 1 matchers", () => {
  it("toBe handles primitive and reference identity", () => {
    expect(42).toBe(42);
    expect("hello").toBe("hello");
    expect(true).toBe(true);
    expect(null).toBe(null);
    expect(undefined).toBe(undefined);

    const obj = { x: 1 };
    expect(obj).toBe(obj);

    expect(42).not.toBe(43);
    expect("hello").not.toBe("world");
    expect({ x: 1 }).not.toBe({ x: 1 });
  });

  it("toEqual performs pure deep equality", () => {
    expect({ a: 1, b: [2, 3] }).toEqual({ a: 1, b: [2, 3] });
    expect([1, 2, 3]).toEqual([1, 2, 3]);
    expect(new Uint8Array([1, 2, 3])).toEqual(new Uint8Array([1, 2, 3]));

    expect({ a: 1 }).not.toEqual({ a: 2 });
    expect([1, 2]).not.toEqual([1, 2, 3]);
  });

  it("truthy and falsy checks", () => {
    expect(true).toBeTruthy();
    expect(1).toBeTruthy();
    expect("non-empty").toBeTruthy();
    expect({}).toBeTruthy();

    expect(false).toBeFalsy();
    expect(0).toBeFalsy();
    expect("").toBeFalsy();
    expect(null).toBeFalsy();
    expect(undefined).toBeFalsy();

    expect(false).not.toBeTruthy();
    expect(true).not.toBeFalsy();
  });

  it("null, undefined, defined, NaN checks", () => {
    expect(null).toBeNull();
    expect(42).not.toBeNull();

    expect(undefined).toBeUndefined();
    expect(0).not.toBeUndefined();

    expect(0).toBeDefined();
    expect("").toBeDefined();
    expect(undefined).not.toBeDefined();

    expect(NaN).toBeNaN();
    expect(42).not.toBeNaN();
  });

  it("numeric comparisons", () => {
    expect(10).toBeGreaterThan(5);
    expect(10).toBeGreaterThanOrEqual(10);
    expect(5).toBeLessThan(10);
    expect(5).toBeLessThanOrEqual(5);

    expect(5).not.toBeGreaterThan(10);
    expect(10).not.toBeLessThan(5);
  });

  it("toBeCloseTo floating-point comparisons", () => {
    expect(0.1 + 0.2).toBeCloseTo(0.3, 2);
    expect(Math.PI).toBeCloseTo(3.14, 2);
    expect(10).not.toBeCloseTo(20, 1);
  });

  it("toContain checks arrays, strings, sets, maps", () => {
    expect("hello world").toContain("world");
    expect("hello world").not.toContain("earth");

    expect([1, 2, 3]).toContain(2);
    expect([1, 2, 3]).not.toContain(4);

    const s = new Set(["a", "b"]);
    expect(s).toContain("a");
    expect(s).not.toContain("c");

    const m = new Map([["key", "val"]]);
    expect(m).toContain("key");
    expect(m).not.toContain("other");
  });

  it("toHaveLength checks length property", () => {
    expect([1, 2, 3]).toHaveLength(3);
    expect("hello").toHaveLength(5);
    expect([]).toHaveLength(0);
    expect([1]).not.toHaveLength(2);
  });

  it("toThrow catches exceptions", () => {
    const throwsCustom = () => {
      throw new Error("boundary exceeded");
    };
    const returnsOk = () => 42;

    expect(throwsCustom).toThrow();
    expect(throwsCustom).toThrow("boundary");
    expect(throwsCustom).toThrow(/exceeded/);
    expect(throwsCustom).toThrow(Error);

    expect(returnsOk).not.toThrow();

    expect(() => {
      expect(returnsOk).toThrow();
    }).toThrow(AssertionError);
  });

  it("vi.fn tracks invocations and returns", () => {
    const mock = vi.fn((x: number) => x * 2);
    expect(mock(5)).toBe(10);
    expect(mock(7)).toBe(14);
    expect(mock.mock.calls).toEqual([[5], [7]]);
    expect(mock.mock.results.length).toBe(2);

    mock.mock.clear();
    expect(mock.mock.calls.length).toBe(0);

    const throwingMock = vi.fn(() => {
      throw new Error("mock error");
    });
    expect(() => throwingMock()).toThrow("mock error");
    throwingMock.mock.reset();
  });
});

describe("Harness options, args, and utilities", () => {
  it("parseArgs handles valid arguments and error conditions", async () => {
    const { parseArgs } = await import("../dist/harness.js");
    const opt1 = parseArgs([]);
    expect(opt1.seed).toBe(0n);
    expect(opt1.sabotage).toBe(false);

    const opt2 = parseArgs(["--seed", "123", "--sabotage", "always-pass", "--nocapture"]);
    expect(opt2.seed).toBe(123n);
    expect(opt2.sabotage).toBe(true);

    expect(() => parseArgs(["--seed"])).toThrow("--seed needs a value");
    expect(() => parseArgs(["--seed", "abc"])).toThrow("needs a decimal u64");
    expect(() => parseArgs(["--sabotage"])).toThrow("--sabotage needs the mode always-pass");
    expect(() => parseArgs(["--sabotage", "other"])).toThrow("--sabotage needs the mode always-pass");
    expect(() => parseArgs(["--unknown"])).toThrow("unknown argument");
  });

  it("Suite validates IDs and catches duplicates and panics", async () => {
    const { Suite } = await import("../dist/harness.js");
    const dupSuite = new Suite();
    dupSuite.add("valid-id-1", () => ({ status: "pass" }));
    dupSuite.add("valid-id-1", () => ({ status: "pass" }));
    const dupSummary = await dupSuite.run({ silent: true });
    expect(dupSummary.bailed).toBe(true);
    expect(dupSummary.exitCode).toBe(1);
    expect(dupSummary.tapOutput.some((line) => line.includes("Bail out! duplicate test id"))).toBe(true);

    const badIdSuite = new Suite();
    badIdSuite.add("invalid id with spaces", () => ({ status: "pass" }));
    const badSummary = await badIdSuite.run({ silent: true });
    expect(badSummary.exitCode).toBe(1);
    expect(badSummary.tapOutput.some((line) => line.includes("Bail out! invalid test id"))).toBe(true);

    const panicSuite = new Suite();
    panicSuite.add("panic-test", () => {
      throw new Error("simulated panic");
    });
    const panicSummary = await panicSuite.run({ silent: true });
    expect(panicSummary.failed).toBe(1);
    expect(panicSummary.exitCode).toBe(1);
    expect(panicSummary.tapOutput.some((line) => line.includes('failure_id: "panic"'))).toBe(true);
  });

  it("f64 bit conversions and toValue edge cases", async () => {
    const { numberToF64Bits, f64BitsToNumber, toValue } = await import("../dist/deep_equal.js");
    const bits = numberToF64Bits(3.14159);
    const num = f64BitsToNumber(bits);
    expect(Math.abs(num - 3.14159)).toBeLessThan(1e-10);

    expect(toValue(null).kind).toBe("null");
    expect(toValue(undefined).kind).toBe("null");
    expect(toValue(true).kind).toBe("bool");
    expect(toValue(123n).kind).toBe("int");
    expect(toValue(new Uint8Array([1, 2])).kind).toBe("bytes");
    expect(toValue([1, 2]).kind).toBe("list");
    expect(toValue(new Set([1, 2])).kind).toBe("list");
    expect(toValue(new Map([["a", 1]])).kind).toBe("map");
    expect(toValue({ a: 1, b: 2 }).kind).toBe("map");
  });
});
