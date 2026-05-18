/**
 * value-mappers.test.ts — Color / Length / FontStack → LaTeX.
 */

import { describe, it, expect } from "vitest";
import {
  colorToLatex, lengthToLatex, fontStackToLatex, fontStackFallbacksComment,
} from "../src/index.js";

describe("colorToLatex", () => {
  it("rgb opaque → xcolor RGB form", () => {
    expect(colorToLatex({ kind: "rgb", r: 31, g: 35, b: 40 })).toBe("{RGB}{31,35,40}");
  });

  it("rgb clamps out-of-range channels", () => {
    expect(colorToLatex({ kind: "rgb", r: -10, g: 999, b: 128 })).toBe("{RGB}{0,255,128}");
  });

  it("rgb rounds fractional channels", () => {
    expect(colorToLatex({ kind: "rgb", r: 127.4, g: 127.6, b: 128 })).toBe("{RGB}{127,128,128}");
  });

  it("hsl converts to RGB (xcolor doesn't speak HSL natively)", () => {
    // hsl(0, 100, 50) → pure red.
    expect(colorToLatex({ kind: "hsl", h: 0, s: 100, l: 50 })).toBe("{RGB}{255,0,0}");
  });

  it("hsl gray", () => {
    expect(colorToLatex({ kind: "hsl", h: 0, s: 0, l: 50 })).toBe("{RGB}{128,128,128}");
  });

  it("hsl black", () => {
    expect(colorToLatex({ kind: "hsl", h: 0, s: 0, l: 0 })).toBe("{RGB}{0,0,0}");
  });

  it("hsl white", () => {
    expect(colorToLatex({ kind: "hsl", h: 0, s: 0, l: 100 })).toBe("{RGB}{255,255,255}");
  });

  it("oklch returns null (lossy round-trip out of scope for v0)", () => {
    expect(colorToLatex({ kind: "oklch", l: 0.7, c: 0.15, h: 220 })).toBeNull();
  });

  it("named known color resolves via the safe map", () => {
    expect(colorToLatex({ kind: "named", name: "black" })).toBe("{RGB}{0,0,0}");
    expect(colorToLatex({ kind: "named", name: "tomato" })).toBe("{RGB}{255,99,71}");
  });

  it("named lookup is case-insensitive", () => {
    expect(colorToLatex({ kind: "named", name: "Tomato" })).toBe("{RGB}{255,99,71}");
  });

  it("named unknown returns null", () => {
    expect(colorToLatex({ kind: "named", name: "asparagus" })).toBeNull();
  });
});

describe("lengthToLatex", () => {
  it("pt / mm / in / ex / em pass through unchanged", () => {
    for (const u of ["pt", "mm", "in", "ex", "em"] as const) {
      expect(lengthToLatex({ unit: u, value: 12 })).toBe(`12${u}`);
    }
  });

  it("px converts to pt at 1px = 0.75pt", () => {
    expect(lengthToLatex({ unit: "px", value: 16 })).toBe("12pt");
  });

  it("rem maps to em (v0 approximation; documented)", () => {
    expect(lengthToLatex({ unit: "rem", value: 1.25 })).toBe("1.25em");
  });

  it("%, vh, vw, ch return null", () => {
    for (const u of ["%", "vh", "vw", "ch"] as const) {
      expect(lengthToLatex({ unit: u, value: 100 })).toBeNull();
    }
  });

  it("negative values supported", () => {
    expect(lengthToLatex({ unit: "pt", value: -2 })).toBe("-2pt");
  });

  it("fractional pt with bounded precision (4 dp)", () => {
    expect(lengthToLatex({ unit: "pt", value: 0.12345 })).toBe("0.1235pt");
  });
});

describe("fontStackToLatex", () => {
  it("returns the first family escaped", () => {
    expect(fontStackToLatex(["Inter", "system-ui", "sans-serif"])).toBe("Inter");
  });

  it("escapes LaTeX-specials in family names", () => {
    expect(fontStackToLatex(["My_Font"])).toBe("My\\_Font");
  });

  it("strips control characters from family names", () => {
    expect(fontStackToLatex(["bad\nname"])).toBe("badname");
  });

  it("returns null on empty stack (defensive)", () => {
    expect(fontStackToLatex([])).toBeNull();
  });
});

describe("fontStackFallbacksComment", () => {
  it("emits a comment listing fallbacks", () => {
    expect(fontStackFallbacksComment(["Inter", "system-ui", "sans-serif"]))
      .toBe("% font-fallbacks: system-ui, sans-serif");
  });

  it("returns empty string for single-element stack", () => {
    expect(fontStackFallbacksComment(["serif"])).toBe("");
  });

  it("returns empty string for empty stack", () => {
    expect(fontStackFallbacksComment([])).toBe("");
  });

  it("escapes specials in fallback names", () => {
    expect(fontStackFallbacksComment(["A", "B%C"])).toBe("% font-fallbacks: B\\%C");
  });
});
