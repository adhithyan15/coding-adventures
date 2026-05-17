/**
 * value-mappers.test.ts — Color / Length / FontStack / Shadow → CSS.
 */

import { describe, it, expect } from "vitest";
import {
  colorToCss, lengthToCss, fontStackToCss, shadowToCss,
} from "../src/index.js";

describe("colorToCss", () => {
  it("rgb opaque", () => {
    expect(colorToCss({ kind: "rgb", r: 31, g: 35, b: 40 })).toBe("rgb(31 35 40)");
  });

  it("rgb with alpha < 1 emits 4-arg form", () => {
    expect(colorToCss({ kind: "rgb", r: 0, g: 0, b: 0, a: 0.5 }))
      .toBe("rgb(0 0 0 / 0.5)");
  });

  it("rgb with alpha === 1 keeps 3-arg form (cleaner output)", () => {
    expect(colorToCss({ kind: "rgb", r: 1, g: 1, b: 1, a: 1 })).toBe("rgb(1 1 1)");
  });

  it("hsl opaque", () => {
    expect(colorToCss({ kind: "hsl", h: 180, s: 50, l: 50 })).toBe("hsl(180 50% 50%)");
  });

  it("hsl alpha", () => {
    expect(colorToCss({ kind: "hsl", h: 0, s: 0, l: 0, a: 0.25 }))
      .toBe("hsl(0 0% 0% / 0.25)");
  });

  it("oklch opaque", () => {
    expect(colorToCss({ kind: "oklch", l: 0.7, c: 0.15, h: 220 }))
      .toBe("oklch(0.7 0.15 220)");
  });

  it("oklch alpha", () => {
    expect(colorToCss({ kind: "oklch", l: 0.5, c: 0.2, h: 90, a: 0.8 }))
      .toBe("oklch(0.5 0.2 90 / 0.8)");
  });

  it("named passes through verbatim", () => {
    expect(colorToCss({ kind: "named", name: "transparent" })).toBe("transparent");
    expect(colorToCss({ kind: "named", name: "tomato" })).toBe("tomato");
  });
});

describe("lengthToCss", () => {
  it("formats integer values cleanly", () => {
    expect(lengthToCss({ unit: "px", value: 16 })).toBe("16px");
  });

  it("formats fractional values", () => {
    expect(lengthToCss({ unit: "rem", value: 1.25 })).toBe("1.25rem");
  });

  it("every unit is preserved", () => {
    for (const u of ["px", "rem", "em", "%", "vh", "vw", "pt", "mm", "in", "ch", "ex"] as const) {
      expect(lengthToCss({ unit: u, value: 1 })).toBe(`1${u}`);
    }
  });

  it("negative values supported (text-indent: -1rem)", () => {
    expect(lengthToCss({ unit: "rem", value: -1 })).toBe("-1rem");
  });
});

describe("fontStackToCss", () => {
  it("safe identifiers go unquoted", () => {
    expect(fontStackToCss(["Inter", "system-ui", "sans-serif"]))
      .toBe("Inter, system-ui, sans-serif");
  });

  it("families with spaces get quoted", () => {
    expect(fontStackToCss(["SF Mono", "Menlo", "monospace"]))
      .toBe(`"SF Mono", Menlo, monospace`);
  });

  it("escapes embedded quotes", () => {
    expect(fontStackToCss([`Smith"s`, "sans-serif"]))
      .toBe(`"Smith\\"s", sans-serif`);
  });

  it("escapes embedded backslash before escaping quotes", () => {
    expect(fontStackToCss([`back\\slash`]))
      .toBe(`"back\\\\slash"`);
  });

  it("strips raw newline / control chars from family names", () => {
    expect(fontStackToCss(["bad\nname"])).toBe(`"badname"`);
  });

  it("single-element stack works", () => {
    expect(fontStackToCss(["serif"])).toBe("serif");
  });
});

describe("shadowToCss", () => {
  it("basic shadow formats correctly", () => {
    const s = {
      offsetX: { unit: "px", value: 0 } as const,
      offsetY: { unit: "px", value: 2 } as const,
      blur:    { unit: "px", value: 4 } as const,
      spread:  { unit: "px", value: 0 } as const,
      color:   { kind: "rgb", r: 0, g: 0, b: 0, a: 0.1 } as const,
    };
    expect(shadowToCss(s, "rgb(0 0 0 / 0.1)"))
      .toBe("0px 2px 4px 0px rgb(0 0 0 / 0.1)");
  });

  it("inset shadow includes the `inset` keyword first", () => {
    const s = {
      offsetX: { unit: "px", value: 0 } as const,
      offsetY: { unit: "px", value: 0 } as const,
      blur:    { unit: "px", value: 1 } as const,
      spread:  { unit: "px", value: 2 } as const,
      color:   { kind: "named", name: "red" } as const,
      inset: true,
    };
    expect(shadowToCss(s, "red")).toBe("inset 0px 0px 1px 2px red");
  });
});
