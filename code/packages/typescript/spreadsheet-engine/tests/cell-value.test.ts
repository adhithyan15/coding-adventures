import { describe, it, expect } from "vitest";
import {
  EMPTY,
  num,
  text,
  bool,
  err,
  isError,
  toNumber,
  toText,
  toBoolean,
  formatValue,
} from "../src/cell-value.js";

describe("constructors and isError", () => {
  it("builds each value kind", () => {
    expect(num(3).kind).toBe("number");
    expect(text("x").kind).toBe("text");
    expect(bool(true).kind).toBe("boolean");
    expect(err("#NA").kind).toBe("error");
    expect(EMPTY.kind).toBe("empty");
  });

  it("isError narrows correctly", () => {
    expect(isError(err("#DIV/0!"))).toBe(true);
    expect(isError(num(1))).toBe(false);
  });
});

describe("toNumber coercions (spec §2)", () => {
  it("empty cell coerces to 0", () => {
    expect(toNumber(EMPTY)).toBe(0);
  });
  it("number passes through", () => {
    expect(toNumber(num(42))).toBe(42);
  });
  it("boolean becomes 1/0", () => {
    expect(toNumber(bool(true))).toBe(1);
    expect(toNumber(bool(false))).toBe(0);
  });
  it("numeric text parses; non-numeric text is #VALUE!", () => {
    expect(toNumber(text("  12 "))).toBe(12);
    expect(toNumber(text("12abc"))).toMatchObject({ code: "#VALUE!" });
    expect(toNumber(text(""))).toMatchObject({ code: "#VALUE!" });
  });
  it("error propagates unchanged", () => {
    expect(toNumber(err("#REF!"))).toMatchObject({ code: "#REF!" });
  });
});

describe("toText coercions", () => {
  it("empty → '', boolean → TRUE/FALSE, error → code", () => {
    expect(toText(EMPTY)).toBe("");
    expect(toText(num(3))).toBe("3");
    expect(toText(text("hi"))).toBe("hi");
    expect(toText(bool(true))).toBe("TRUE");
    expect(toText(bool(false))).toBe("FALSE");
    expect(toText(err("#NA"))).toBe("#NA");
  });
});

describe("toBoolean coercions", () => {
  it("empty → false, number → truthiness", () => {
    expect(toBoolean(EMPTY)).toBe(false);
    expect(toBoolean(num(0))).toBe(false);
    expect(toBoolean(num(5))).toBe(true);
    expect(toBoolean(bool(true))).toBe(true);
  });
  it("text TRUE/FALSE parse; other text is #VALUE!", () => {
    expect(toBoolean(text("true"))).toBe(true);
    expect(toBoolean(text("FALSE"))).toBe(false);
    expect(toBoolean(text("nope"))).toMatchObject({ code: "#VALUE!" });
  });
  it("error propagates", () => {
    expect(toBoolean(err("#NA"))).toMatchObject({ code: "#NA" });
  });
});

describe("formatValue", () => {
  it("renders each kind", () => {
    expect(formatValue(EMPTY)).toBe("<empty>");
    expect(formatValue(num(3))).toBe("3");
    expect(formatValue(text("a"))).toBe('"a"');
    expect(formatValue(bool(false))).toBe("FALSE");
    expect(formatValue(err("#CIRC!"))).toBe("#CIRC!");
  });
});
