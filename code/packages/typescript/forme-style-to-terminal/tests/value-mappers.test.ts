/**
 * value-mappers.test.ts — Color → SGR triple / fragment.
 */

import { describe, it, expect } from "vitest";
import {
  colorToRgbTriple, colorToSgrFg, colorToSgrBg,
} from "../src/index.js";

describe("colorToRgbTriple", () => {
  it("rgb passes through", () => {
    expect(colorToRgbTriple({ kind: "rgb", r: 31, g: 35, b: 40 })).toEqual([31, 35, 40]);
  });

  it("rgb clamps below 0 and above 255", () => {
    expect(colorToRgbTriple({ kind: "rgb", r: -10, g: 999, b: 128 })).toEqual([0, 255, 128]);
  });

  it("rgb rounds fractional channels", () => {
    expect(colorToRgbTriple({ kind: "rgb", r: 127.4, g: 127.6, b: 128 })).toEqual([127, 128, 128]);
  });

  it("rgb treats NaN as 0 (defensive)", () => {
    expect(colorToRgbTriple({ kind: "rgb", r: NaN, g: 100, b: 200 })).toEqual([0, 100, 200]);
  });

  it("hsl pure red", () => {
    expect(colorToRgbTriple({ kind: "hsl", h: 0, s: 100, l: 50 })).toEqual([255, 0, 0]);
  });

  it("hsl pure black / white", () => {
    expect(colorToRgbTriple({ kind: "hsl", h: 0, s: 0, l: 0 })).toEqual([0, 0, 0]);
    expect(colorToRgbTriple({ kind: "hsl", h: 0, s: 0, l: 100 })).toEqual([255, 255, 255]);
  });

  it("oklch returns null (v0 out-of-scope)", () => {
    expect(colorToRgbTriple({ kind: "oklch", l: 0.7, c: 0.15, h: 220 })).toBeNull();
  });

  it("named known colors resolve", () => {
    expect(colorToRgbTriple({ kind: "named", name: "black" })).toEqual([0, 0, 0]);
    expect(colorToRgbTriple({ kind: "named", name: "tomato" })).toEqual([255, 99, 71]);
  });

  it("named lookup is case-insensitive", () => {
    expect(colorToRgbTriple({ kind: "named", name: "ToMaTo" })).toEqual([255, 99, 71]);
  });

  it("named unknown returns null", () => {
    expect(colorToRgbTriple({ kind: "named", name: "asparagus" })).toBeNull();
  });
});

describe("colorToSgrFg / colorToSgrBg", () => {
  it("fg sequence has the 38;2; prefix", () => {
    expect(colorToSgrFg({ kind: "rgb", r: 31, g: 35, b: 40 })).toBe("38;2;31;35;40");
  });

  it("bg sequence has the 48;2; prefix", () => {
    expect(colorToSgrBg({ kind: "rgb", r: 31, g: 35, b: 40 })).toBe("48;2;31;35;40");
  });

  it("returns null when color isn't representable", () => {
    expect(colorToSgrFg({ kind: "oklch", l: 0.5, c: 0.1, h: 0 })).toBeNull();
    expect(colorToSgrBg({ kind: "named", name: "asparagus" })).toBeNull();
  });
});
