/**
 * property-mappers.test.ts — every kernel property kind maps to its
 * documented CSS declaration per FM04 §9.2.
 *
 * Strategy: build a minimal `TokenSet` (so TokenRef resolution
 * succeeds) and exhaustively walk PROPERTY_KINDS, checking each
 * produces the right output.
 */

import { describe, it, expect } from "vitest";
import { propertyToCss } from "../src/index.js";
import {
  PROPERTY_KINDS, emptyTokenSet,
  type StyleProperty, type TokenSet,
} from "@coding-adventures/forme-style-ir";

const tokens: TokenSet = (() => {
  const t = emptyTokenSet();
  return {
    ...t,
    colors: { brand: { kind: "rgb", r: 10, g: 20, b: 30 } },
    typography: {
      ...t.typography,
      families: { body: ["Inter", "system-ui"] },
      scale:    { md: { unit: "rem", value: 1 } },
      weights:  { regular: 400 },
      leading:  { normal: 1.5 },
      tracking: { normal: { unit: "em", value: 0 } },
    },
    space: { md: { unit: "rem", value: 1 } },
    radii: { sm: { unit: "px", value: 4 } },
    shadows: {
      card: {
        offsetX: { unit: "px", value: 0 },
        offsetY: { unit: "px", value: 2 },
        blur:    { unit: "px", value: 4 },
        spread:  { unit: "px", value: 0 },
        color:   { kind: "rgb", r: 0, g: 0, b: 0, a: 0.1 },
      },
    },
  };
})();

function emit(p: StyleProperty): { decl?: string; warn?: string } {
  const r = propertyToCss(p, tokens);
  return r.ok ? { decl: r.declaration } : { warn: r.warning };
}

describe("color family", () => {
  it("color literal", () => {
    expect(emit({ kind: "color", value: { kind: "rgb", r: 1, g: 2, b: 3 } }).decl)
      .toBe("color: rgb(1 2 3)");
  });
  it("color TokenRef resolves", () => {
    expect(emit({ kind: "color", value: { kind: "token-ref", path: "colors.brand" } }).decl)
      .toBe("color: rgb(10 20 30)");
  });
  it("color unresolved warns", () => {
    expect(emit({ kind: "color", value: { kind: "token-ref", path: "colors.nope" } }).warn)
      .toMatch(/color: unresolved/);
  });
  it("background → background-color", () => {
    expect(emit({ kind: "background", value: { kind: "named", name: "white" } }).decl)
      .toBe("background-color: white");
  });
  it("border-color, outline-color emit corresponding properties", () => {
    expect(emit({ kind: "border-color", value: { kind: "named", name: "gray" } }).decl)
      .toBe("border-color: gray");
    expect(emit({ kind: "outline-color", value: { kind: "named", name: "red" } }).decl)
      .toBe("outline-color: red");
  });
});

describe("typography family", () => {
  it("font-family literal stack", () => {
    expect(emit({ kind: "font-family", value: ["Inter", "sans-serif"] }).decl)
      .toBe("font-family: Inter, sans-serif");
  });
  it("font-family from TokenRef", () => {
    expect(emit({ kind: "font-family", value: { kind: "token-ref", path: "typography.families.body" } }).decl)
      .toBe("font-family: Inter, system-ui");
  });
  it("font-family unresolved warns", () => {
    expect(emit({ kind: "font-family", value: { kind: "token-ref", path: "typography.families.nope" } }).warn)
      .toMatch(/unresolved/);
  });
  it("font-size from Length", () => {
    expect(emit({ kind: "font-size", value: { unit: "rem", value: 1.25 } }).decl)
      .toBe("font-size: 1.25rem");
  });
  it("font-weight literal number", () => {
    expect(emit({ kind: "font-weight", value: 700 }).decl).toBe("font-weight: 700");
  });
  it("font-weight TokenRef", () => {
    expect(emit({ kind: "font-weight", value: { kind: "token-ref", path: "typography.weights.regular" } }).decl)
      .toBe("font-weight: 400");
  });
  it("font-style → font-style", () => {
    expect(emit({ kind: "font-style", value: "italic" }).decl).toBe("font-style: italic");
  });
  it("text-transform → text-transform", () => {
    expect(emit({ kind: "text-transform", value: "uppercase" }).decl).toBe("text-transform: uppercase");
  });
  it("leading → line-height", () => {
    expect(emit({ kind: "leading", value: 1.6 }).decl).toBe("line-height: 1.6");
    expect(emit({ kind: "leading", value: { kind: "token-ref", path: "typography.leading.normal" } }).decl)
      .toBe("line-height: 1.5");
  });
  it("tracking → letter-spacing", () => {
    expect(emit({ kind: "tracking", value: { unit: "em", value: 0.05 } }).decl)
      .toBe("letter-spacing: 0.05em");
  });
  it("text-decoration: minimal", () => {
    expect(emit({ kind: "text-decoration", value: { line: "underline" } }).decl)
      .toBe("text-decoration: underline");
  });
  it("text-decoration: full", () => {
    expect(emit({ kind: "text-decoration", value: {
      line: "underline", style: "wavy",
      color: { kind: "named", name: "red" },
      thickness: { unit: "px", value: 2 },
    } }).decl).toBe("text-decoration: underline wavy red 2px");
  });
});

describe("layout / spacing", () => {
  it("space-before → margin-top", () => {
    expect(emit({ kind: "space-before", value: { unit: "rem", value: 1 } }).decl)
      .toBe("margin-top: 1rem");
  });
  it("space-after → margin-bottom", () => {
    expect(emit({ kind: "space-after", value: { unit: "rem", value: 0.5 } }).decl)
      .toBe("margin-bottom: 0.5rem");
  });
  it("indent → text-indent", () => {
    expect(emit({ kind: "indent", value: { unit: "em", value: 1 } }).decl)
      .toBe("text-indent: 1em");
  });
  it("padding emits all four sides", () => {
    expect(emit({ kind: "padding", value: {
      top:    { unit: "px", value: 1 },
      right:  { unit: "px", value: 2 },
      bottom: { unit: "px", value: 3 },
      left:   { unit: "px", value: 4 },
    } }).decl).toBe("padding: 1px 2px 3px 4px");
  });
  it("padding with one unresolved side warns", () => {
    expect(emit({ kind: "padding", value: {
      top:    { unit: "px", value: 1 },
      right:  { kind: "token-ref", path: "space.nope" },
      bottom: { unit: "px", value: 3 },
      left:   { unit: "px", value: 4 },
    } }).warn).toMatch(/padding: unresolved/);
  });
  it("max-width / min-height", () => {
    expect(emit({ kind: "max-width", value: { unit: "rem", value: 38 } }).decl)
      .toBe("max-width: 38rem");
    expect(emit({ kind: "min-height", value: { unit: "vh", value: 100 } }).decl)
      .toBe("min-height: 100vh");
  });
  it("align → text-align", () => {
    expect(emit({ kind: "align", value: "justify" }).decl).toBe("text-align: justify");
  });
  it("vertical-align → vertical-align", () => {
    expect(emit({ kind: "vertical-align", value: "middle" }).decl).toBe("vertical-align: middle");
  });
});

describe("decoration", () => {
  it("border all-sides → border: ...", () => {
    expect(emit({ kind: "border", value: {
      width: { unit: "px", value: 1 },
      style: "solid",
      color: { kind: "named", name: "gray" },
    } }).decl).toBe("border: 1px solid gray");
  });
  it("border per-side", () => {
    expect(emit({ kind: "border", value: {
      width: { unit: "px", value: 2 },
      style: "dashed",
      color: { kind: "named", name: "red" },
      sides: ["top", "bottom"],
    } }).decl).toBe("border-top: 2px dashed red; border-bottom: 2px dashed red");
  });
  it("border-radius → border-radius", () => {
    expect(emit({ kind: "border-radius", value: { unit: "px", value: 8 } }).decl)
      .toBe("border-radius: 8px");
  });
  it("shadow literal → box-shadow", () => {
    expect(emit({ kind: "shadow", value: {
      offsetX: { unit: "px", value: 0 },
      offsetY: { unit: "px", value: 1 },
      blur:    { unit: "px", value: 2 },
      spread:  { unit: "px", value: 0 },
      color:   { kind: "named", name: "black" },
    } }).decl).toBe("box-shadow: 0px 1px 2px 0px black");
  });
  it("shadow TokenRef resolves", () => {
    expect(emit({ kind: "shadow", value: { kind: "token-ref", path: "shadows.card" } }).decl)
      .toBe("box-shadow: 0px 2px 4px 0px rgb(0 0 0 / 0.1)");
  });
  it("opacity → opacity", () => {
    expect(emit({ kind: "opacity", value: 0.9 }).decl).toBe("opacity: 0.9");
  });
});

describe("page breaks", () => {
  it("column-break before/after", () => {
    expect(emit({ kind: "column-break", value: "before" }).decl).toBe("break-before: column");
    expect(emit({ kind: "column-break", value: "after" }).decl).toBe("break-after: column");
  });
  it("column-break avoid → break-inside: avoid-column", () => {
    expect(emit({ kind: "column-break", value: "avoid" }).decl).toBe("break-inside: avoid-column");
  });
  it("page-break before/after/avoid", () => {
    expect(emit({ kind: "page-break", value: "before" }).decl).toBe("break-before: page");
    expect(emit({ kind: "page-break", value: "after" }).decl).toBe("break-after: page");
    expect(emit({ kind: "page-break", value: "avoid" }).decl).toBe("break-inside: avoid-page");
  });
  it("widow-orphan emits both widows and orphans", () => {
    expect(emit({ kind: "widow-orphan", value: 3 }).decl)
      .toBe("widows: 3; orphans: 3");
  });
});

describe("visibility", () => {
  it("display values pass through", () => {
    for (const v of ["block", "inline", "inline-block", "none"] as const) {
      expect(emit({ kind: "display", value: v }).decl).toBe(`display: ${v}`);
    }
  });
  it("visible: false → visibility: hidden", () => {
    expect(emit({ kind: "visible", value: false }).decl).toBe("visibility: hidden");
  });
  it("visible: true → visibility: visible", () => {
    expect(emit({ kind: "visible", value: true }).decl).toBe("visibility: visible");
  });
});

describe("important flag", () => {
  it("appends !important to a declaration", () => {
    expect(emit({ kind: "color", value: { kind: "named", name: "red" }, important: true }).decl)
      .toBe("color: red !important");
  });
});

describe("exhaustive coverage check", () => {
  it("every PROPERTY_KINDS entry has an emit path (no kernel kind warns 'unhandled')", () => {
    // We use minimal/dummy values just to drive the switch; the goal
    // is to ensure no kernel kind falls through to the default branch
    // and emits 'unhandled property kind'.
    const dummyByKind: Partial<Record<typeof PROPERTY_KINDS[number], StyleProperty>> = {
      "color":           { kind: "color", value: { kind: "named", name: "x" } },
      "background":      { kind: "background", value: { kind: "named", name: "x" } },
      "border-color":    { kind: "border-color", value: { kind: "named", name: "x" } },
      "outline-color":   { kind: "outline-color", value: { kind: "named", name: "x" } },
      "font-family":     { kind: "font-family", value: ["x"] },
      "font-size":       { kind: "font-size", value: { unit: "px", value: 1 } },
      "font-weight":     { kind: "font-weight", value: 400 },
      "font-style":      { kind: "font-style", value: "italic" },
      "text-transform":  { kind: "text-transform", value: "none" },
      "leading":         { kind: "leading", value: 1.5 },
      "tracking":        { kind: "tracking", value: { unit: "em", value: 0 } },
      "text-decoration": { kind: "text-decoration", value: { line: "none" } },
      "space-before":    { kind: "space-before", value: { unit: "px", value: 0 } },
      "space-after":     { kind: "space-after", value: { unit: "px", value: 0 } },
      "indent":          { kind: "indent", value: { unit: "px", value: 0 } },
      "padding":         { kind: "padding", value: {
        top: { unit: "px", value: 0 }, right: { unit: "px", value: 0 },
        bottom: { unit: "px", value: 0 }, left: { unit: "px", value: 0 },
      } },
      "max-width":       { kind: "max-width", value: { unit: "px", value: 0 } },
      "min-height":      { kind: "min-height", value: { unit: "px", value: 0 } },
      "align":           { kind: "align", value: "start" },
      "vertical-align":  { kind: "vertical-align", value: "top" },
      "border":          { kind: "border", value: {
        width: { unit: "px", value: 1 }, style: "solid",
        color: { kind: "named", name: "x" },
      } },
      "border-radius":   { kind: "border-radius", value: { unit: "px", value: 0 } },
      "shadow":          { kind: "shadow", value: {
        offsetX: { unit: "px", value: 0 }, offsetY: { unit: "px", value: 0 },
        blur: { unit: "px", value: 0 }, spread: { unit: "px", value: 0 },
        color: { kind: "named", name: "x" },
      } },
      "opacity":         { kind: "opacity", value: 1 },
      "column-break":    { kind: "column-break", value: "before" },
      "page-break":      { kind: "page-break", value: "before" },
      "widow-orphan":    { kind: "widow-orphan", value: 3 },
      "display":         { kind: "display", value: "block" },
      "visible":         { kind: "visible", value: true },
    };
    for (const k of PROPERTY_KINDS) {
      const prop = dummyByKind[k];
      expect(prop, `missing dummy for ${k}`).toBeDefined();
      const result = propertyToCss(prop!, tokens);
      expect(result.ok, `${k} fell through default branch`).toBe(true);
    }
  });
});
