/**
 * translate.test.ts — end-to-end `translateToCss` integration.
 */

import { describe, it, expect } from "vitest";
import { translateToCss } from "../src/index.js";
import {
  emptyStyleDocument, styleRuleId, sel,
  type StyleDocument, type StyleRule, type StyleRuleId,
} from "@coding-adventures/forme-style-ir";

function rule(
  id: string, selector: StyleRule["selector"],
  properties: StyleRule["properties"],
  context?: string,
): StyleRule {
  const base: StyleRule = { id: styleRuleId(id), selector, properties };
  return context === undefined ? base : { ...base, context };
}

function doc(rules: StyleRule[], contexts: string[] = []): StyleDocument {
  const d = emptyStyleDocument();
  return {
    ...d,
    tokens: {
      ...d.tokens,
      colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
      space: { md: { unit: "rem", value: 1 } },
    },
    rules,
    contexts,
  };
}

describe("translateToCss — basic emission", () => {
  it("empty doc → empty output, no warnings", () => {
    const r = translateToCss(emptyStyleDocument(), { activeContexts: [] });
    expect(r.output).toBe("");
    expect(r.warnings).toEqual([]);
    expect(r.emittedRules).toEqual([]);
  });

  it("single rule → one CSS block, listed in emittedRules", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color", value: { kind: "named", name: "tomato" } },
    ])]);
    const r = translateToCss(d, { activeContexts: [] });
    expect(r.output).toBe("p {\n  color: tomato;\n}");
    expect(r.emittedRules).toEqual(["r1"]);
    expect(r.warnings).toEqual([]);
  });

  it("multiple rules → each in its own block, joined by blank lines", () => {
    const d = doc([
      rule("r1", sel.type("p"), [{ kind: "color", value: { kind: "named", name: "red" } }]),
      rule("r2", sel.heading(1), [{ kind: "font-size", value: { unit: "rem", value: 2 } }]),
    ]);
    const out = translateToCss(d, { activeContexts: [] }).output;
    expect(out).toBe("p {\n  color: red;\n}\n\nh1 {\n  font-size: 2rem;\n}");
  });

  it("multiple declarations within one rule each terminated by ;", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color", value: { kind: "named", name: "red" } },
      { kind: "font-size", value: { unit: "rem", value: 1 } },
    ])]);
    const out = translateToCss(d, { activeContexts: [] }).output;
    expect(out).toBe("p {\n  color: red;\n  font-size: 1rem;\n}");
  });

  it("rule whose every property warns produces no block (no empty {})", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color", value: { kind: "token-ref", path: "colors.nope" } },
    ])]);
    const r = translateToCss(d, { activeContexts: [] });
    expect(r.output).toBe("");
    expect(r.emittedRules).toEqual([]);
    expect(r.warnings.length).toBe(1);
  });
});

describe("translateToCss — context filtering + @media wrapping", () => {
  it("rule with context not in activeContexts is skipped", () => {
    const d = doc([
      rule("r1", sel.type("p"), [{ kind: "color", value: { kind: "named", name: "red" } }], "dark"),
    ], ["dark"]);
    const r = translateToCss(d, { activeContexts: [] });
    expect(r.output).toBe("");
    expect(r.emittedRules).toEqual([]);
  });

  it("rule with active context wraps in @media", () => {
    const d = doc([
      rule("r1", sel.type("p"), [{ kind: "color", value: { kind: "named", name: "white" } }], "dark"),
    ], ["dark"]);
    const r = translateToCss(d, { activeContexts: ["dark"] });
    expect(r.output).toBe(
      "@media (prefers-color-scheme: dark) {\n" +
      "  p {\n" +
      "    color: white;\n" +
      "  }\n" +
      "}",
    );
    expect(r.emittedRules).toEqual(["r1"]);
  });

  it("unconditional rules emit before @media blocks", () => {
    const d = doc([
      rule("r-screen", sel.type("p"), [{ kind: "color", value: { kind: "named", name: "black" } }]),
      rule("r-dark",   sel.type("p"), [{ kind: "color", value: { kind: "named", name: "white" } }], "dark"),
    ], ["dark"]);
    const out = translateToCss(d, { activeContexts: ["dark"] }).output;
    expect(out).toMatch(/^p \{\n {2}color: black;\n\}\n\n@media \(prefers-color-scheme: dark\)/);
  });

  it("rules sharing a context are grouped in one @media block in order", () => {
    const d = doc([
      rule("r1", sel.type("p"),       [{ kind: "color",     value: { kind: "named", name: "white" } }], "dark"),
      rule("r2", sel.heading(1),      [{ kind: "font-size", value: { unit: "rem", value: 2 } }], "dark"),
    ], ["dark"]);
    const out = translateToCss(d, { activeContexts: ["dark"] }).output;
    expect(out).toContain("@media (prefers-color-scheme: dark) {");
    expect((out.match(/@media/g) ?? []).length).toBe(1);
  });

  it("ext: context warns + skips", () => {
    const d = doc([
      rule("r1", sel.type("p"), [{ kind: "color", value: { kind: "named", name: "x" } }], "ext:my:c"),
    ]);
    const r = translateToCss(d, { activeContexts: ["ext:my:c"] });
    expect(r.output).toBe("");
    expect(r.warnings[0]?.code).toBe("EXT_CONTEXT_NOT_TRANSLATED");
  });
});

describe("translateToCss — usedRuleIds slicing", () => {
  it("with usedRuleIds → only listed rules emit", () => {
    const d = doc([
      rule("r1", sel.type("p"),  [{ kind: "color", value: { kind: "named", name: "red"  } }]),
      rule("r2", sel.heading(1), [{ kind: "color", value: { kind: "named", name: "blue" } }]),
      rule("r3", sel.type("li"), [{ kind: "color", value: { kind: "named", name: "green" } }]),
    ]);
    const r = translateToCss(d, {
      activeContexts: [],
      usedRuleIds: ["r2", "r3"] as readonly StyleRuleId[],
    });
    expect(r.output).toBe("h1 {\n  color: blue;\n}\n\nli {\n  color: green;\n}");
    expect(r.emittedRules).toEqual(["r2", "r3"]);
  });

  it("usedRuleIds=[] emits nothing", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color", value: { kind: "named", name: "red" } },
    ])]);
    const r = translateToCss(d, { activeContexts: [], usedRuleIds: [] });
    expect(r.output).toBe("");
  });

  it("ids in usedRuleIds not present in rules are silently ignored", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color", value: { kind: "named", name: "red" } },
    ])]);
    const r = translateToCss(d, {
      activeContexts: [],
      usedRuleIds: ["nope", "r1"] as readonly StyleRuleId[],
    });
    expect(r.emittedRules).toEqual(["r1"]);
  });
});

describe("translateToCss — scope", () => {
  it("scope prefixes every selector with the prefix + space", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color", value: { kind: "named", name: "red" } },
    ])]);
    const r = translateToCss(d, { activeContexts: [], scope: "#page-1" });
    expect(r.output).toBe("#page-1 p {\n  color: red;\n}");
  });

  it("scope applies per comma-path in or() selectors", () => {
    const d = doc([rule("r1",
      sel.or(sel.type("p"), sel.heading(1)),
      [{ kind: "color", value: { kind: "named", name: "red" } }],
    )]);
    const r = translateToCss(d, { activeContexts: [], scope: "#page-1" });
    expect(r.output).toBe("#page-1 p, #page-1 h1 {\n  color: red;\n}");
  });

  it("scope works inside @media blocks too", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color", value: { kind: "named", name: "white" } },
    ], "dark")], ["dark"]);
    const out = translateToCss(d, { activeContexts: ["dark"], scope: "#page-1" }).output;
    expect(out).toContain("#page-1 p {");
  });
});

describe("translateToCss — ext: property handling", () => {
  it("ext: property emits warning + skips", () => {
    const d = doc([{
      ...rule("r1", sel.type("p"), []),
      properties: [
        { kind: "ext:mask:image", value: "url(...)" },
        { kind: "color",          value: { kind: "named", name: "red" } },
      ],
    }]);
    const r = translateToCss(d, { activeContexts: [] });
    expect(r.output).toBe("p {\n  color: red;\n}");
    expect(r.warnings.some((w) => w.code === "EXT_PROPERTY_NOT_TRANSLATED")).toBe(true);
  });
});

describe("translateToCss — reproducibility", () => {
  it("same input produces byte-identical output", () => {
    const make = () => doc([
      rule("a", sel.type("p"),  [{ kind: "color",     value: { kind: "token-ref", path: "colors.text" } }]),
      rule("b", sel.heading(1), [{ kind: "font-size", value: { unit: "rem", value: 2 } }]),
    ]);
    const a = translateToCss(make(), { activeContexts: [] }).output;
    const b = translateToCss(make(), { activeContexts: [] }).output;
    expect(a).toBe(b);
  });
});

describe("translateToCss — never throws", () => {
  it("malformed token paths produce warnings, not exceptions", () => {
    const d = doc([rule("r1", sel.type("p"), [
      { kind: "color",   value: { kind: "token-ref", path: "colors.nope" } },
      { kind: "padding", value: {
        top:    { kind: "token-ref", path: "space.nope" },
        right:  { unit: "px", value: 0 },
        bottom: { unit: "px", value: 0 },
        left:   { unit: "px", value: 0 },
      } },
    ])]);
    expect(() => translateToCss(d, { activeContexts: [] })).not.toThrow();
  });
});
