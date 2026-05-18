/**
 * property-mappers.test.ts — every kernel property kind.
 */

import { describe, it, expect } from "vitest";
import { PROPERTY_KINDS, type StyleProperty, type TokenSet } from "@coding-adventures/forme-style-ir";
import { propertyToLatex } from "../src/index.js";

const tokens: TokenSet = {
  colors:  { text: { kind: "rgb", r: 0, g: 0, b: 0 } },
  typography: {
    families: { body: ["Inter", "sans-serif"] },
    scale:    { md: { unit: "pt", value: 12 } },
    weights:  { regular: 400, bold: 700, black: 900 },
    leading:  { normal: 1.5 },
    tracking: { normal: { unit: "em", value: 0 } },
  },
  space:   { md: { unit: "pt", value: 6 } },
  radii:   {},
  shadows: {},
};

describe("propertyToLatex — color", () => {
  it("color emits \\color{RGB}{...}", () => {
    const r = propertyToLatex({ kind: "color", value: { kind: "rgb", r: 31, g: 35, b: 40 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\color{RGB}{31,35,40}");
  });

  it("color via token-ref resolves", () => {
    const r = propertyToLatex({ kind: "color", value: { kind: "token-ref", path: "colors.text" } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\color{RGB}{0,0,0}");
  });

  it("color with oklch warns (model not expressible)", () => {
    const r = propertyToLatex({ kind: "color", value: { kind: "oklch", l: 0.7, c: 0.15, h: 220 } }, tokens);
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.warning).toMatch(/xcolor/);
  });

  it("color with unresolved ref warns", () => {
    const r = propertyToLatex({ kind: "color", value: { kind: "token-ref", path: "colors.gone" } }, tokens);
    expect(r.ok).toBe(false);
  });

  it("background / border-color / outline-color warn-skip (no preamble form)", () => {
    for (const k of ["background", "border-color", "outline-color"] as const) {
      const r = propertyToLatex(
        { kind: k, value: { kind: "named", name: "red" } } as StyleProperty,
        tokens,
      );
      expect(r.ok).toBe(false);
    }
  });
});

describe("propertyToLatex — typography", () => {
  it("font-family emits \\setmainfont{...}", () => {
    const r = propertyToLatex({ kind: "font-family", value: ["Inter", "sans-serif"] }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\setmainfont{Inter}");
  });

  it("font-family with unresolved ref warns", () => {
    const r = propertyToLatex({ kind: "font-family", value: { kind: "token-ref", path: "typography.families.nope" } }, tokens);
    expect(r.ok).toBe(false);
  });

  it("font-size emits \\fontsize with 1.2× leading default", () => {
    const r = propertyToLatex({ kind: "font-size", value: { unit: "pt", value: 12 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\fontsize{12pt}{14.4pt}\\selectfont");
  });

  it("font-size in px converts to pt", () => {
    const r = propertyToLatex({ kind: "font-size", value: { unit: "px", value: 16 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\fontsize{12pt}{14.4pt}\\selectfont");
  });

  it("font-size in rem maps to em", () => {
    const r = propertyToLatex({ kind: "font-size", value: { unit: "rem", value: 1 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\fontsize{1em}{1.2em}\\selectfont");
  });

  it("font-size in vh warns", () => {
    const r = propertyToLatex({ kind: "font-size", value: { unit: "vh", value: 10 } }, tokens);
    expect(r.ok).toBe(false);
  });

  it("font-weight 400 → \\fontseries{m}", () => {
    const r = propertyToLatex({ kind: "font-weight", value: 400 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\fontseries{m}\\selectfont");
  });

  it("font-weight 700 → \\fontseries{b}", () => {
    const r = propertyToLatex({ kind: "font-weight", value: 700 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\fontseries{b}\\selectfont");
  });

  it("font-weight 900 → \\fontseries{bx}", () => {
    const r = propertyToLatex({ kind: "font-weight", value: 900 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\fontseries{bx}\\selectfont");
  });

  it("font-weight via token-ref", () => {
    const r = propertyToLatex({ kind: "font-weight", value: { kind: "token-ref", path: "typography.weights.bold" } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\fontseries{b}\\selectfont");
  });

  it("font-weight unresolved warns", () => {
    const r = propertyToLatex({ kind: "font-weight", value: { kind: "token-ref", path: "typography.weights.nope" } }, tokens);
    expect(r.ok).toBe(false);
  });

  it("font-style italic / oblique / normal", () => {
    expect((propertyToLatex({ kind: "font-style", value: "italic" }, tokens) as { commands: string }).commands).toBe("\\fontshape{it}\\selectfont");
    expect((propertyToLatex({ kind: "font-style", value: "oblique" }, tokens) as { commands: string }).commands).toBe("\\fontshape{sl}\\selectfont");
    expect((propertyToLatex({ kind: "font-style", value: "normal" }, tokens) as { commands: string }).commands).toBe("\\fontshape{n}\\selectfont");
  });

  it("text-transform uppercase / lowercase / none / capitalize", () => {
    expect((propertyToLatex({ kind: "text-transform", value: "uppercase" }, tokens) as { commands: string }).commands).toContain("MakeUppercase");
    expect((propertyToLatex({ kind: "text-transform", value: "lowercase" }, tokens) as { commands: string }).commands).toContain("MakeLowercase");
    expect((propertyToLatex({ kind: "text-transform", value: "none" }, tokens) as { commands: string }).commands).toContain("relax");
    expect(propertyToLatex({ kind: "text-transform", value: "capitalize" }, tokens).ok).toBe(false);
  });

  it("leading → \\linespread{n}", () => {
    const r = propertyToLatex({ kind: "leading", value: 1.5 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\linespread{1.5}\\selectfont");
  });

  it("leading unresolved warns", () => {
    const r = propertyToLatex({ kind: "leading", value: { kind: "token-ref", path: "typography.leading.nope" } }, tokens);
    expect(r.ok).toBe(false);
  });

  it("tracking warns (needs microtype)", () => {
    expect(propertyToLatex({ kind: "tracking", value: { unit: "em", value: 0.05 } }, tokens).ok).toBe(false);
  });

  it("text-decoration underline / none / line-through / overline", () => {
    expect((propertyToLatex({ kind: "text-decoration", value: { line: "underline" } }, tokens) as { commands: string }).commands).toContain("underline");
    expect((propertyToLatex({ kind: "text-decoration", value: { line: "none" } }, tokens) as { commands: string }).commands).toContain("relax");
    expect(propertyToLatex({ kind: "text-decoration", value: { line: "line-through" } }, tokens).ok).toBe(false);
    expect(propertyToLatex({ kind: "text-decoration", value: { line: "overline" } }, tokens).ok).toBe(false);
  });
});

describe("propertyToLatex — layout / spacing", () => {
  it("space-before → \\setlength{\\parskip}{...}", () => {
    const r = propertyToLatex({ kind: "space-before", value: { unit: "pt", value: 8 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\setlength{\\parskip}{8pt}");
  });

  it("space-after", () => {
    const r = propertyToLatex({ kind: "space-after", value: { unit: "pt", value: 8 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\setlength{\\parskip}{8pt}");
  });

  it("indent → \\setlength{\\parindent}{...}", () => {
    const r = propertyToLatex({ kind: "indent", value: { unit: "em", value: 1 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\setlength{\\parindent}{1em}");
  });

  it("indent with non-LaTeX unit warns", () => {
    expect(propertyToLatex({ kind: "indent", value: { unit: "%", value: 10 } }, tokens).ok).toBe(false);
  });

  it("indent unresolved warns", () => {
    expect(propertyToLatex({ kind: "indent", value: { kind: "token-ref", path: "space.nope" } }, tokens).ok).toBe(false);
  });

  it("padding warns (no preamble equivalent)", () => {
    expect(propertyToLatex(
      { kind: "padding", value: { top: { unit: "pt", value: 4 }, right: { unit: "pt", value: 4 }, bottom: { unit: "pt", value: 4 }, left: { unit: "pt", value: 4 } } },
      tokens,
    ).ok).toBe(false);
  });

  it("max-width → \\setlength{\\linewidth}{...}", () => {
    const r = propertyToLatex({ kind: "max-width", value: { unit: "pt", value: 400 } }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\setlength{\\linewidth}{400pt}");
  });

  it("min-height warns", () => {
    expect(propertyToLatex({ kind: "min-height", value: { unit: "pt", value: 100 } }, tokens).ok).toBe(false);
  });

  it("align start / end / center / justify", () => {
    expect((propertyToLatex({ kind: "align", value: "start" }, tokens) as { commands: string }).commands).toBe("\\raggedright");
    expect((propertyToLatex({ kind: "align", value: "end" }, tokens) as { commands: string }).commands).toBe("\\raggedleft");
    expect((propertyToLatex({ kind: "align", value: "center" }, tokens) as { commands: string }).commands).toBe("\\centering");
    expect((propertyToLatex({ kind: "align", value: "justify" }, tokens) as { commands: string }).commands).toContain("leftskip");
  });

  it("vertical-align warns", () => {
    expect(propertyToLatex({ kind: "vertical-align", value: "middle" }, tokens).ok).toBe(false);
  });
});

describe("propertyToLatex — decoration warn-skips", () => {
  it.each(["border", "border-radius", "shadow", "opacity"] as const)("%s warns", (k) => {
    let p: StyleProperty;
    switch (k) {
      case "border":         p = { kind: "border", value: { width: { unit: "pt", value: 1 }, style: "solid", color: { kind: "named", name: "black" } } }; break;
      case "border-radius":  p = { kind: "border-radius", value: { unit: "pt", value: 4 } }; break;
      case "shadow":         p = { kind: "shadow", value: { offsetX: { unit: "pt", value: 0 }, offsetY: { unit: "pt", value: 2 }, blur: { unit: "pt", value: 4 }, spread: { unit: "pt", value: 0 }, color: { kind: "named", name: "black" } } }; break;
      case "opacity":        p = { kind: "opacity", value: 0.5 }; break;
    }
    expect(propertyToLatex(p, tokens).ok).toBe(false);
  });
});

describe("propertyToLatex — page break", () => {
  it("column-break before/after/avoid", () => {
    expect((propertyToLatex({ kind: "column-break", value: "before" }, tokens) as { commands: string }).commands).toBe("\\columnbreak");
    expect((propertyToLatex({ kind: "column-break", value: "after" }, tokens) as { commands: string }).commands).toBe("\\columnbreak");
    expect((propertyToLatex({ kind: "column-break", value: "avoid" }, tokens) as { commands: string }).commands).toBe("\\nobreak");
  });

  it("page-break before/after/avoid", () => {
    expect((propertyToLatex({ kind: "page-break", value: "before" }, tokens) as { commands: string }).commands).toBe("\\pagebreak");
    expect((propertyToLatex({ kind: "page-break", value: "after" }, tokens) as { commands: string }).commands).toBe("\\pagebreak");
    expect((propertyToLatex({ kind: "page-break", value: "avoid" }, tokens) as { commands: string }).commands).toBe("\\nopagebreak");
  });

  it("widow-orphan scales to 0-10000 penalty", () => {
    const r = propertyToLatex({ kind: "widow-orphan", value: 4 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\widowpenalty=10000\\clubpenalty=10000");
  });

  it("widow-orphan 0 = 0 penalty", () => {
    const r = propertyToLatex({ kind: "widow-orphan", value: 0 }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\widowpenalty=0\\clubpenalty=0");
  });
});

describe("propertyToLatex — visibility", () => {
  it("display warns", () => {
    expect(propertyToLatex({ kind: "display", value: "block" }, tokens).ok).toBe(false);
  });

  it("visible true / false", () => {
    expect((propertyToLatex({ kind: "visible", value: true }, tokens) as { commands: string }).commands).toContain("relax");
    expect((propertyToLatex({ kind: "visible", value: false }, tokens) as { commands: string }).commands).toContain("hphantom");
  });
});

describe("propertyToLatex — extension kinds", () => {
  it("unknown ext: kind emits warning", () => {
    const r = propertyToLatex({ kind: "ext:plugin:foo" as never, value: "bar" } as unknown as StyleProperty, tokens);
    expect(r.ok).toBe(false);
  });
});

describe("propertyToLatex — exhaustiveness over PROPERTY_KINDS", () => {
  it("every kernel-known kind yields some result (ok or warning, not crash)", () => {
    for (const k of PROPERTY_KINDS) {
      // Build a minimal valid value per kind.  We don't care about
      // ok/warning — we just want propertyToLatex never to throw.
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
      expect(() => propertyToLatex(p, tokens)).not.toThrow();
    }
  });
});

describe("propertyToLatex — defensive fallthroughs (typed-as-never branches)", () => {
  // These pin the "unknown value" defensive returns inside each
  // exhaustive switch.  The static types make these unreachable, but
  // a hand-rolled IR that bypasses the validator could land here, and
  // we want to confirm the code returns a warning rather than crashes.
  it("font-style unknown value → warning", () => {
    expect(propertyToLatex({ kind: "font-style", value: "wonky" as never }, tokens).ok).toBe(false);
  });
  it("text-transform unknown value → warning", () => {
    expect(propertyToLatex({ kind: "text-transform", value: "wonky" as never }, tokens).ok).toBe(false);
  });
  it("text-decoration unknown line → warning", () => {
    expect(propertyToLatex({ kind: "text-decoration", value: { line: "wonky" as never } }, tokens).ok).toBe(false);
  });
  it("align unknown value → warning", () => {
    expect(propertyToLatex({ kind: "align", value: "wonky" as never }, tokens).ok).toBe(false);
  });
  it("column-break unknown value → warning", () => {
    expect(propertyToLatex({ kind: "column-break", value: "wonky" as never }, tokens).ok).toBe(false);
  });
  it("page-break unknown value → warning", () => {
    expect(propertyToLatex({ kind: "page-break", value: "wonky" as never }, tokens).ok).toBe(false);
  });
  it("widow-orphan non-numeric → warning", () => {
    expect(propertyToLatex({ kind: "widow-orphan", value: "x" as never }, tokens).ok).toBe(false);
  });
});

describe("propertyToLatex — important trailer is the translator's concern", () => {
  it("propertyToLatex itself doesn't append !important (translator does)", () => {
    const r = propertyToLatex({ kind: "leading", value: 1.5, important: true }, tokens);
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.commands).toBe("\\linespread{1.5}\\selectfont");
  });
});
