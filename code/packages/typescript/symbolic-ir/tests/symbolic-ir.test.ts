import { describe, expect, it } from "vitest";
import {
  ADD,
  MUL,
  POW,
  app,
  equals,
  int,
  rational,
  stringNode,
  structuralKey,
  sym,
  toDisplayString,
} from "../src/index.js";

describe("symbolic-ir", () => {
  it("normalizes rational signs and gcd", () => {
    expect(rational(2, 4)).toEqual({ kind: "rational", numer: 1n, denom: 2n });
    expect(rational(1, -2)).toEqual({ kind: "rational", numer: -1n, denom: 2n });
    expect(rational(-6, -9)).toEqual({ kind: "rational", numer: 2n, denom: 3n });
  });

  it("rejects zero rational denominator", () => {
    expect(() => rational(1, 0)).toThrow(RangeError);
  });

  it("supports arbitrary precision integer inputs", () => {
    const huge = int("123456789012345678901234567890");
    expect(huge.value).toBe(123456789012345678901234567890n);
  });

  it("compares trees structurally", () => {
    const x = sym("x");
    const left = app(ADD, [app(POW, [x, int(2)]), int(1)]);
    const right = app(ADD, [app(POW, [sym("x"), int(2)]), int(1)]);
    const different = app(MUL, [sym("x"), int(2)]);

    expect(equals(left, right)).toBe(true);
    expect(equals(left, different)).toBe(false);
  });

  it("builds stable structural keys", () => {
    const expr = app(ADD, [stringNode("a,b"), rational(2, 4)]);
    expect(structuralKey(expr)).toBe('A:S:"Add"(T:"a,b",Q:1/2)');
  });

  it("formats display strings", () => {
    const x = sym("x");
    const expr = app(ADD, [app(POW, [x, int(2)]), int(1)]);
    expect(toDisplayString(expr)).toBe("Add(Pow(x, 2), 1)");
  });
});
