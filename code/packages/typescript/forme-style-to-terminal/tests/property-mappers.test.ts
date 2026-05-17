/**
 * property-mappers.test.ts — every kernel property kind.
 */

import { describe, it, expect } from "vitest";
import { PROPERTY_KINDS, type StyleProperty, type TokenSet } from "@coding-adventures/forme-style-ir";
import { propertyToTerminal } from "../src/index.js";

const tokens: TokenSet = {
  colors:  { text: { kind: "rgb", r: 0, g: 0, b: 0 } },
  typography: {
    families: { body: ["Inter"] },
    scale:    { md: { unit: "pt", value: 12 } },
    weights:  { regular: 400, bold: 700, black: 900 },
    leading:  { normal: 1.5 },
    tracking: { normal: { unit: "em", value: 0 } },
  },
  space:   {},
  radii:   {},
  shadows: {},
};

describe("propertyToTerminal — color", () => {
  it("color emits fg SGR", () => {
    const r = propertyToTerminal({ kind: "color", value: { kind: "rgb", r: 31, g: 35, b: 40 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.sgr).toEqual(["38;2;31;35;40"]);
  });

  it("color via token-ref resolves", () => {
    const r = propertyToTerminal({ kind: "color", value: { kind: "token-ref", path: "colors.text" } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.sgr).toEqual(["38;2;0;0;0"]);
  });

  it("color OKLCH warns", () => {
    expect(propertyToTerminal({ kind: "color", value: { kind: "oklch", l: 0.5, c: 0.1, h: 0 } }, tokens).ok).toBe(false);
  });

  it("color unresolved warns", () => {
    expect(propertyToTerminal({ kind: "color", value: { kind: "token-ref", path: "colors.gone" } }, tokens).ok).toBe(false);
  });

  it("background emits bg SGR", () => {
    const r = propertyToTerminal({ kind: "background", value: { kind: "rgb", r: 255, g: 255, b: 255 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.sgr).toEqual(["48;2;255;255;255"]);
  });

  it("background unresolved warns", () => {
    expect(propertyToTerminal({ kind: "background", value: { kind: "token-ref", path: "colors.nope" } }, tokens).ok).toBe(false);
  });

  it("background OKLCH warns", () => {
    expect(propertyToTerminal({ kind: "background", value: { kind: "oklch", l: 0.5, c: 0.1, h: 0 } }, tokens).ok).toBe(false);
  });

  it("border-color / outline-color warn-skip", () => {
    for (const k of ["border-color", "outline-color"] as const) {
      expect(propertyToTerminal({ kind: k, value: { kind: "named", name: "red" } } as StyleProperty, tokens).ok).toBe(false);
    }
  });
});

describe("propertyToTerminal — typography", () => {
  it("font-family warns", () => {
    expect(propertyToTerminal({ kind: "font-family", value: ["Inter"] }, tokens).ok).toBe(false);
  });

  it("font-size warns", () => {
    expect(propertyToTerminal({ kind: "font-size", value: { unit: "pt", value: 12 } }, tokens).ok).toBe(false);
  });

  it("font-weight >= 600 emits bold SGR 1", () => {
    const r = propertyToTerminal({ kind: "font-weight", value: 700 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.sgr).toEqual(["1"]);
  });

  it("font-weight < 600 emits no SGR (success, empty)", () => {
    const r = propertyToTerminal({ kind: "font-weight", value: 400 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.sgr).toEqual([]);
  });

  it("font-weight via token-ref", () => {
    const r = propertyToTerminal({ kind: "font-weight", value: { kind: "token-ref", path: "typography.weights.bold" } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.sgr).toEqual(["1"]);
  });

  it("font-weight unresolved warns", () => {
    expect(propertyToTerminal({ kind: "font-weight", value: { kind: "token-ref", path: "typography.weights.nope" } }, tokens).ok).toBe(false);
  });

  it("font-style italic / oblique → SGR 3", () => {
    expect((propertyToTerminal({ kind: "font-style", value: "italic" }, tokens) as { sgr: string[] }).sgr).toEqual(["3"]);
    expect((propertyToTerminal({ kind: "font-style", value: "oblique" }, tokens) as { sgr: string[] }).sgr).toEqual(["3"]);
  });

  it("font-style normal emits empty SGR (success)", () => {
    expect((propertyToTerminal({ kind: "font-style", value: "normal" }, tokens) as { sgr: string[] }).sgr).toEqual([]);
  });

  it("font-style unknown value warns", () => {
    expect(propertyToTerminal({ kind: "font-style", value: "wonky" as never }, tokens).ok).toBe(false);
  });

  it("text-transform warns (no terminal equivalent)", () => {
    expect(propertyToTerminal({ kind: "text-transform", value: "uppercase" }, tokens).ok).toBe(false);
  });

  it("leading / tracking warn", () => {
    expect(propertyToTerminal({ kind: "leading", value: 1.5 }, tokens).ok).toBe(false);
    expect(propertyToTerminal({ kind: "tracking", value: { unit: "em", value: 0.05 } }, tokens).ok).toBe(false);
  });

  it("text-decoration underline → SGR 4", () => {
    expect((propertyToTerminal({ kind: "text-decoration", value: { line: "underline" } }, tokens) as { sgr: string[] }).sgr).toEqual(["4"]);
  });

  it("text-decoration line-through → SGR 9", () => {
    expect((propertyToTerminal({ kind: "text-decoration", value: { line: "line-through" } }, tokens) as { sgr: string[] }).sgr).toEqual(["9"]);
  });

  it("text-decoration overline → SGR 53", () => {
    expect((propertyToTerminal({ kind: "text-decoration", value: { line: "overline" } }, tokens) as { sgr: string[] }).sgr).toEqual(["53"]);
  });

  it("text-decoration none emits empty SGR", () => {
    expect((propertyToTerminal({ kind: "text-decoration", value: { line: "none" } }, tokens) as { sgr: string[] }).sgr).toEqual([]);
  });

  it("text-decoration unknown line warns", () => {
    expect(propertyToTerminal({ kind: "text-decoration", value: { line: "wonky" as never } }, tokens).ok).toBe(false);
  });
});

describe("propertyToTerminal — layout / decoration / page-break (all warn)", () => {
  it.each(["space-before", "space-after", "indent", "padding", "max-width", "min-height",
           "align", "vertical-align", "border", "border-radius", "shadow", "opacity",
           "column-break", "page-break", "widow-orphan"] as const)("%s warns", (k) => {
    let p: StyleProperty;
    switch (k) {
      case "space-before": case "space-after": case "indent": case "max-width": case "min-height":
      case "border-radius":
        p = { kind: k, value: { unit: "pt", value: 1 } } as StyleProperty; break;
      case "padding":
        p = { kind: k, value: { top: { unit: "pt", value: 0 }, right: { unit: "pt", value: 0 }, bottom: { unit: "pt", value: 0 }, left: { unit: "pt", value: 0 } } } as StyleProperty; break;
      case "align":
        p = { kind: k, value: "start" } as StyleProperty; break;
      case "vertical-align":
        p = { kind: k, value: "baseline" } as StyleProperty; break;
      case "border":
        p = { kind: k, value: { width: { unit: "pt", value: 1 }, style: "solid", color: { kind: "named", name: "black" } } } as StyleProperty; break;
      case "shadow":
        p = { kind: k, value: { offsetX: { unit: "pt", value: 0 }, offsetY: { unit: "pt", value: 0 }, blur: { unit: "pt", value: 0 }, spread: { unit: "pt", value: 0 }, color: { kind: "named", name: "black" } } } as StyleProperty; break;
      case "opacity": case "widow-orphan":
        p = { kind: k, value: 1 } as StyleProperty; break;
      case "column-break": case "page-break":
        p = { kind: k, value: "before" } as StyleProperty; break;
    }
    expect(propertyToTerminal(p, tokens).ok).toBe(false);
  });
});

describe("propertyToTerminal — visibility", () => {
  it("display warns (semantic mismatch with visible)", () => {
    expect(propertyToTerminal({ kind: "display", value: "none" }, tokens).ok).toBe(false);
  });

  it("visible: true → no SGR (default state)", () => {
    expect((propertyToTerminal({ kind: "visible", value: true }, tokens) as { sgr: string[] }).sgr).toEqual([]);
  });

  it("visible: false → SGR 8 (conceal)", () => {
    expect((propertyToTerminal({ kind: "visible", value: false }, tokens) as { sgr: string[] }).sgr).toEqual(["8"]);
  });
});

describe("propertyToTerminal — extension kinds", () => {
  it("ext: kind warns", () => {
    expect(propertyToTerminal({ kind: "ext:plugin:foo", value: 1 } as unknown as StyleProperty, tokens).ok).toBe(false);
  });
});

describe("propertyToTerminal — exhaustiveness over PROPERTY_KINDS", () => {
  it("every kernel kind yields a result (ok or warn, never throws)", () => {
    for (const k of PROPERTY_KINDS) {
      let p: StyleProperty;
      switch (k) {
        case "color": case "background": case "border-color": case "outline-color":
          p = { kind: k, value: { kind: "named", name: "black" } } as StyleProperty; break;
        case "font-family":
          p = { kind: k, value: ["Inter"] } as StyleProperty; break;
        case "font-size": case "tracking": case "space-before": case "space-after":
        case "indent": case "max-width": case "min-height": case "border-radius":
          p = { kind: k, value: { unit: "pt", value: 1 } } as StyleProperty; break;
        case "font-weight": case "leading": case "opacity": case "widow-orphan":
          p = { kind: k, value: 1 } as StyleProperty; break;
        case "font-style":
          p = { kind: k, value: "normal" } as StyleProperty; break;
        case "text-transform":
          p = { kind: k, value: "none" } as StyleProperty; break;
        case "text-decoration":
          p = { kind: k, value: { line: "none" } } as StyleProperty; break;
        case "padding":
          p = { kind: k, value: { top: { unit: "pt", value: 0 }, right: { unit: "pt", value: 0 }, bottom: { unit: "pt", value: 0 }, left: { unit: "pt", value: 0 } } } as StyleProperty; break;
        case "align":
          p = { kind: k, value: "start" } as StyleProperty; break;
        case "vertical-align":
          p = { kind: k, value: "baseline" } as StyleProperty; break;
        case "border":
          p = { kind: k, value: { width: { unit: "pt", value: 1 }, style: "solid", color: { kind: "named", name: "black" } } } as StyleProperty; break;
        case "shadow":
          p = { kind: k, value: { offsetX: { unit: "pt", value: 0 }, offsetY: { unit: "pt", value: 0 }, blur: { unit: "pt", value: 0 }, spread: { unit: "pt", value: 0 }, color: { kind: "named", name: "black" } } } as StyleProperty; break;
        case "column-break": case "page-break":
          p = { kind: k, value: "before" } as StyleProperty; break;
        case "display":
          p = { kind: k, value: "block" } as StyleProperty; break;
        case "visible":
          p = { kind: k, value: true } as StyleProperty; break;
      }
      expect(() => propertyToTerminal(p, tokens)).not.toThrow();
    }
  });
});
