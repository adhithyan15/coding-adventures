/**
 * translate.test.ts — end-to-end translateToTerminal.
 */

import { describe, it, expect } from "vitest";
import {
  styleRuleId, sel,
  type StyleDocument, type StyleRule,
} from "@coding-adventures/forme-style-ir";
import { translateToTerminal } from "../src/index.js";

function baseDoc(): StyleDocument {
  return {
    kind: "StyleDocument",
    tokens: {
      colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
      typography: {
        families: { body: ["Inter"] },
        scale:    { md: { unit: "pt", value: 12 } },
        weights:  { regular: 400, bold: 700 },
        leading:  { normal: 1.5 },
        tracking: { normal: { unit: "em", value: 0 } },
      },
      space:   {},
      radii:   {},
      shadows: {},
    },
    rules: [],
    contexts: [],
    theme: null,
  };
}

function rule(id: string, sel0: StyleRule["selector"], props: StyleRule["properties"], context?: string): StyleRule {
  return context !== undefined
    ? { id: styleRuleId(id), selector: sel0, properties: props, context }
    : { id: styleRuleId(id), selector: sel0, properties: props };
}

describe("translateToTerminal — happy path", () => {
  it("emits a TS module fragment with a single rule's SGR wrap", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("body", sel.type("paragraph"), [
          { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
          { kind: "font-weight", value: 700 },
        ]),
      ],
    };
    const { output, emittedRules, warnings } = translateToTerminal(doc, { activeContexts: [] });
    expect(emittedRules).toEqual(["body"]);
    expect(warnings).toEqual([]);
    expect(output).toContain("export const formeStyles");
    expect(output).toContain("// rule \"body\" — node-type:paragraph");
    // The two SGR fragments combined: 38;2;31;35;40 (color), 1 (bold).
    // Property iteration order is color-first, then font-weight.
    expect(output).toContain('["body", { prefix: "\\x1b[38;2;31;35;40;1m", suffix: "\\x1b[0m" }],');
  });

  it("emits an interface declaration in the header", () => {
    const { output } = translateToTerminal(baseDoc(), { activeContexts: [] });
    expect(output).toContain("export interface AnsiStyle {");
    expect(output).toContain("readonly prefix: string;");
    expect(output).toContain("readonly suffix: string;");
  });

  it("emits an empty Map when there are no rules", () => {
    const { output, emittedRules } = translateToTerminal(baseDoc(), { activeContexts: [] });
    expect(emittedRules).toEqual([]);
    expect(output).toContain("new Map([");
    expect(output).toContain("]);");
  });
});

describe("translateToTerminal — filtering", () => {
  it("rules with non-active contexts are filtered out", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("inactive", sel.type("paragraph"), [{ kind: "font-weight", value: 700 }], "print"),
      ],
    };
    const { emittedRules, output } = translateToTerminal(doc, { activeContexts: [] });
    expect(emittedRules).toEqual([]);
    expect(output).not.toContain('"inactive"');
  });

  it("rule emits when its context is in activeContexts", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("dark-mode", sel.type("paragraph"), [{ kind: "color", value: { kind: "named", name: "white" } }], "dark"),
      ],
    };
    const { emittedRules, output } = translateToTerminal(doc, { activeContexts: ["dark"] });
    expect(emittedRules).toEqual(["dark-mode"]);
    expect(output).toContain('"dark-mode"');
  });

  it("ext: contexts emit a warning and skip the rule", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("x", sel.type("paragraph"), [{ kind: "font-weight", value: 700 }], "ext:plugin:custom"),
      ],
    };
    const { warnings, emittedRules } = translateToTerminal(doc, { activeContexts: ["ext:plugin:custom"] });
    expect(emittedRules).toEqual([]);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]!.code).toBe("EXT_CONTEXT_NOT_TRANSLATED");
  });

  it("usedRuleIds slices the output", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("a", sel.type("p"), [{ kind: "font-weight", value: 700 }]),
        rule("b", sel.type("h"), [{ kind: "font-weight", value: 700 }]),
        rule("c", sel.type("x"), [{ kind: "font-weight", value: 700 }]),
      ],
    };
    const { emittedRules } = translateToTerminal(doc, {
      activeContexts: [],
      usedRuleIds: [styleRuleId("a"), styleRuleId("c")],
    });
    expect([...emittedRules].sort()).toEqual(["a", "c"]);
  });

  it("usedRuleIds with empty list emits no rules", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule("a", sel.type("p"), [{ kind: "font-weight", value: 700 }])],
    };
    const { emittedRules } = translateToTerminal(doc, { activeContexts: [], usedRuleIds: [] });
    expect(emittedRules).toEqual([]);
  });

  it("ext: property kinds emit a warning; rule still emits other props", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("mixed", sel.type("paragraph"), [
          { kind: "ext:plugin:thing", value: 1 } as never,
          { kind: "font-weight", value: 700 },
        ]),
      ],
    };
    const { warnings, emittedRules } = translateToTerminal(doc, { activeContexts: [] });
    expect(emittedRules).toEqual(["mixed"]);
    expect(warnings.some((w) => w.code === "EXT_PROPERTY_NOT_TRANSLATED")).toBe(true);
  });

  it("rules where everything warn-skips still emit (with empty prefix), but NOT in emittedRules", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("noop", sel.type("paragraph"), [
          { kind: "padding", value: { top: { unit: "pt", value: 0 }, right: { unit: "pt", value: 0 }, bottom: { unit: "pt", value: 0 }, left: { unit: "pt", value: 0 } } },
        ]),
      ],
    };
    const { emittedRules, output, warnings } = translateToTerminal(doc, { activeContexts: [] });
    // The id IS present in the output (consumers shouldn't get
    // surprised by Map lookup misses) but with empty prefix/suffix.
    expect(output).toContain('["noop", { prefix: "", suffix: "" }],');
    // emittedRules excludes no-op rules — the AOT compiler doesn't
    // need to wrap content that produces no styling change.
    expect(emittedRules).toEqual([]);
    expect(warnings.length).toBeGreaterThanOrEqual(1);
  });
});

describe("translateToTerminal — scope", () => {
  it("scope prefix is prepended to the Map key", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule("body", sel.type("paragraph"), [{ kind: "font-weight", value: 700 }])],
    };
    const { output } = translateToTerminal(doc, { activeContexts: [], scope: "page-abc123." });
    expect(output).toContain('["page-abc123.body"');
  });
});

describe("translateToTerminal — reproducibility (FM03)", () => {
  it("same input → byte-identical output", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("a", sel.type("p"), [{ kind: "color", value: { kind: "named", name: "red" } }]),
        rule("b", { kind: "node-type-level", level: 1 }, [{ kind: "font-weight", value: 700 }]),
      ],
    };
    const opts = { activeContexts: [] };
    expect(translateToTerminal(doc, opts).output).toBe(translateToTerminal(doc, opts).output);
  });
});

describe("translateToTerminal — output safety (ANSI / TS-string injection)", () => {
  it("strips ESC from rule ids before they reach the Map key or comment", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule("evil\x1b[31m", sel.type("paragraph"), [{ kind: "font-weight", value: 700 }])],
    };
    const { output } = translateToTerminal(doc, { activeContexts: [] });
    // No raw 0x1B should appear in the output.
    // eslint-disable-next-line no-control-regex
    expect(output).not.toMatch(/\x1b(?!\[)/);   // (apart from the literal \x1b[ we emit as text, which is the 4 bytes \, x, 1, b)
    // The id appears with the ESC stripped.
    expect(output).toContain("evil[31m");
  });

  it("escapes backslash and quote in rule ids (TS string-literal safety)", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule(`he"llo\\world`, sel.type("paragraph"), [{ kind: "font-weight", value: 700 }])],
    };
    const { output } = translateToTerminal(doc, { activeContexts: [] });
    expect(output).toContain(`he\\"llo\\\\world`);
    // None of these would break the JS string literal — verify by
    // parsing the line as JS.
    const line = output.split("\n").find((l) => l.includes("he\\\"llo\\\\world"))!;
    // Strip leading whitespace + trailing comma.
    const trimmed = line.replace(/^\s+/, "").replace(/,$/, "");
    expect(() => Function(`"use strict"; return ${trimmed};`)()).not.toThrow();
  });

  it("strips ESC from selector descriptions before they reach the comment", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule("x", sel.type("evil\x1bname"), [{ kind: "font-weight", value: 700 }])],
    };
    const { output } = translateToTerminal(doc, { activeContexts: [] });
    expect(output).toContain("// rule \"x\" — node-type:evilname");
  });

  it("color components are always integer SGR parameters (no injection from numeric coercion)", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      // r is `NaN` (somehow); colorToRgbTriple defensively coerces to 0.
      rules: [rule("nanrule", sel.type("p"), [{ kind: "color", value: { kind: "rgb", r: NaN, g: 100, b: 200 } }])],
    };
    const { output } = translateToTerminal(doc, { activeContexts: [] });
    // SGR fragment is well-formed — no `NaN` substring.
    expect(output).toContain("38;2;0;100;200");
    expect(output).not.toContain("NaN");
  });
});
