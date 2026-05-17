/**
 * validate.test.ts — `validateStyleDocument` coverage.
 *
 * Strategy: exercise every documented rejection reason once, plus
 * the happy path, plus the "warnings, not errors" path for soft
 * cases like unrecognised contexts.
 *
 * We construct minimal invalid inputs (a doc with one bad field at a
 * time) so each test pins exactly one validator path.  The "deeply
 * malformed" tests verify that the validator collects multiple
 * errors in one pass rather than bailing.
 */

import { describe, it, expect } from "vitest";
import {
  StyleError, validateStyleDocument,
  emptyStyleDocument, styleRuleId, sel,
  type StyleDocument, type StyleErrorCode,
} from "../src/index.js";

/**
 * Helper: returns the list of error codes thrown by validating
 * `value`.  If validation succeeds, returns `[]` so tests can assert
 * "should not throw" via `[]`.
 */
function codesOf(value: unknown): StyleErrorCode[] {
  try {
    validateStyleDocument(value);
    return [];
  } catch (e) {
    if (e instanceof StyleError) return e.errors.map((x) => x.code);
    throw e;
  }
}

describe("happy path", () => {
  it("validates an empty StyleDocument with no warnings", () => {
    const result = validateStyleDocument(emptyStyleDocument());
    expect(result.warnings).toEqual([]);
  });

  it("validates a fully-fleshed minimal doc", () => {
    const doc: StyleDocument = {
      kind: "StyleDocument",
      tokens: {
        colors: {
          text: { kind: "rgb", r: 31, g: 35, b: 40 },
          link: { kind: "token-ref", path: "colors.text" },
        },
        typography: {
          families: { body: ["Inter", "system-ui", "sans-serif"] },
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
      },
      rules: [
        {
          id: styleRuleId("body-text"),
          selector: sel.type("paragraph"),
          properties: [
            { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
            { kind: "font-family", value: { kind: "token-ref", path: "typography.families.body" } },
            { kind: "leading", value: 1.6 },
            { kind: "opacity", value: 0.9 },
          ],
        },
      ],
      contexts: ["screen", "print"],
      theme: null,
    };
    const r = validateStyleDocument(doc);
    expect(r.warnings).toEqual([]);
    expect(r.document).toBe(doc);
  });
});

describe("top-level shape", () => {
  it("non-object root throws MALFORMED", () => {
    expect(codesOf(null)).toEqual(["MALFORMED"]);
    expect(codesOf(42)).toEqual(["MALFORMED"]);
    expect(codesOf("doc")).toEqual(["MALFORMED"]);
    expect(codesOf([])).toEqual(["MALFORMED"]);
  });

  it("wrong kind value throws MALFORMED", () => {
    const codes = codesOf({ ...emptyStyleDocument(), kind: "WhatEven" });
    expect(codes).toContain("MALFORMED");
  });

  it("tokens missing/non-object throws MALFORMED (bails early)", () => {
    const codes = codesOf({ kind: "StyleDocument", contexts: [], theme: null, rules: [] });
    expect(codes).toContain("MALFORMED");
  });

  it("contexts non-array throws MALFORMED", () => {
    const bad = { ...emptyStyleDocument(), contexts: "screen" } as unknown;
    const codes = codesOf(bad);
    expect(codes).toContain("MALFORMED");
  });

  it("contexts containing non-strings throws MALFORMED for each bad entry", () => {
    const bad = { ...emptyStyleDocument(), contexts: ["ok", 42, true] } as unknown;
    const codes = codesOf(bad);
    expect(codes.filter((c) => c === "MALFORMED").length).toBeGreaterThanOrEqual(2);
  });

  it("theme not string|null throws MALFORMED", () => {
    const bad = { ...emptyStyleDocument(), theme: 42 } as unknown;
    expect(codesOf(bad)).toContain("MALFORMED");
  });

  it("rules non-array throws MALFORMED", () => {
    const bad = { ...emptyStyleDocument(), rules: "no" } as unknown;
    expect(codesOf(bad)).toContain("MALFORMED");
  });
});

describe("rule-level errors", () => {
  const base = emptyStyleDocument();

  it("non-object rule entry throws MALFORMED", () => {
    expect(codesOf({ ...base, rules: ["nope"] })).toContain("MALFORMED");
  });

  it("missing id throws MALFORMED", () => {
    expect(codesOf({ ...base, rules: [{ selector: sel.type("p"), properties: [] }] }))
      .toContain("MALFORMED");
  });

  it("empty id throws EMPTY_RULE_ID", () => {
    expect(codesOf({ ...base, rules: [{ id: "", selector: sel.type("p"), properties: [] }] }))
      .toContain("EMPTY_RULE_ID");
  });

  it("duplicate ids across two rules throws DUPLICATE_RULE_ID", () => {
    const doc = {
      ...base,
      rules: [
        { id: "x", selector: sel.type("p"), properties: [] },
        { id: "x", selector: sel.type("h1"), properties: [] },
      ],
    };
    expect(codesOf(doc)).toContain("DUPLICATE_RULE_ID");
  });

  it("properties non-array throws MALFORMED", () => {
    expect(codesOf({ ...base, rules: [{ id: "r", selector: sel.type("p"), properties: "x" }] }))
      .toContain("MALFORMED");
  });

  it("non-boolean important throws MALFORMED", () => {
    const codes = codesOf({ ...base, rules: [{
      id: "r", selector: sel.type("p"),
      properties: [{ kind: "color", value: { kind: "rgb", r: 1, g: 1, b: 1 }, important: "yes" }],
    }] });
    expect(codes).toContain("MALFORMED");
  });
});

describe("selector errors", () => {
  const base = emptyStyleDocument();
  function withSelector(s: unknown) {
    return { ...base, rules: [{ id: "r", selector: s, properties: [] }] };
  }

  it("unknown kind throws UNKNOWN_SELECTOR_KIND", () => {
    expect(codesOf(withSelector({ kind: "wat" }))).toContain("UNKNOWN_SELECTOR_KIND");
  });

  it("node-type with empty type throws MALFORMED", () => {
    expect(codesOf(withSelector({ kind: "node-type", type: "" }))).toContain("MALFORMED");
  });

  it("node-type-level wrong type throws MALFORMED", () => {
    expect(codesOf(withSelector({ kind: "node-type-level", type: "paragraph", level: 1 })))
      .toContain("MALFORMED");
  });

  it("node-type-level bad level throws INVALID_HEADING_LEVEL", () => {
    expect(codesOf(withSelector({ kind: "node-type-level", type: "heading", level: 7 })))
      .toContain("INVALID_HEADING_LEVEL");
  });

  it("nth with negative literal throws MALFORMED", () => {
    expect(codesOf(withSelector(sel.nth(sel.type("p"), -1)))).toContain("MALFORMED");
  });

  it("nth with bad formula throws MALFORMED", () => {
    expect(codesOf(withSelector({
      kind: "nth", of: sel.type("p"), n: { a: NaN, b: 0 },
    }))).toContain("MALFORMED");
  });

  it("and with empty array throws EMPTY_COMPOSITION", () => {
    expect(codesOf(withSelector({ kind: "and", all: [] }))).toContain("EMPTY_COMPOSITION");
  });

  it("or with empty array throws EMPTY_COMPOSITION", () => {
    expect(codesOf(withSelector({ kind: "or", any: [] }))).toContain("EMPTY_COMPOSITION");
  });

  it("not wraps a selector validated recursively", () => {
    expect(codesOf(withSelector({ kind: "not", inner: { kind: "wat" } })))
      .toContain("UNKNOWN_SELECTOR_KIND");
  });

  it("child-of recursively validates both halves", () => {
    const codes = codesOf(withSelector({
      kind: "child-of",
      parent: { kind: "wat" },
      child:  { kind: "node-type", type: "" },
    }));
    expect(codes).toContain("UNKNOWN_SELECTOR_KIND");
    expect(codes).toContain("MALFORMED");
  });

  it("custom-kind / tag / id / role: empty payload → MALFORMED", () => {
    expect(codesOf(withSelector({ kind: "custom-kind", customKind: "" }))).toContain("MALFORMED");
    expect(codesOf(withSelector({ kind: "tag", tag: "" }))).toContain("MALFORMED");
    expect(codesOf(withSelector({ kind: "id", id: "" }))).toContain("MALFORMED");
    expect(codesOf(withSelector({ kind: "role", role: "" }))).toContain("MALFORMED");
  });
});

describe("property errors", () => {
  const base = emptyStyleDocument();
  function withProperty(p: unknown) {
    return { ...base, rules: [{ id: "r", selector: sel.type("p"), properties: [p] }] };
  }

  it("unknown kernel kind throws UNKNOWN_PROPERTY_KIND", () => {
    expect(codesOf(withProperty({ kind: "transmogrify", value: 1 })))
      .toContain("UNKNOWN_PROPERTY_KIND");
  });

  it("ext: with missing value throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "ext:foo:bar" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("ext: with any defined value validates", () => {
    expect(codesOf(withProperty({ kind: "ext:foo:bar", value: null }))).toEqual([]);
    expect(codesOf(withProperty({ kind: "ext:foo:bar", value: 0 }))).toEqual([]);
  });

  it("color with bad channel throws INVALID_COLOR_CHANNEL", () => {
    expect(codesOf(withProperty({ kind: "color", value: { kind: "rgb", r: 999, g: 0, b: 0 } })))
      .toContain("INVALID_COLOR_CHANNEL");
  });

  it("color with bad kind throws INVALID_COLOR", () => {
    expect(codesOf(withProperty({ kind: "color", value: { kind: "cmyk", c: 0, m: 0, y: 0, k: 0 } })))
      .toContain("INVALID_COLOR");
  });

  it("color with named requiring non-empty name", () => {
    expect(codesOf(withProperty({ kind: "color", value: { kind: "named", name: "" } })))
      .toContain("INVALID_COLOR");
  });

  it("Color alpha out of [0,1] throws INVALID_COLOR_CHANNEL", () => {
    expect(codesOf(withProperty({ kind: "color", value: { kind: "rgb", r: 0, g: 0, b: 0, a: 2 } })))
      .toContain("INVALID_COLOR_CHANNEL");
  });

  it("font-size with bad length unit throws INVALID_LENGTH_UNIT", () => {
    expect(codesOf(withProperty({ kind: "font-size", value: { unit: "leagues", value: 3 } })))
      .toContain("INVALID_LENGTH_UNIT");
  });

  it("font-size with bad TokenRef path throws INVALID_TOKEN_REF_PATH", () => {
    expect(codesOf(withProperty({ kind: "font-size", value: { kind: "token-ref", path: "1bad" } })))
      .toContain("INVALID_TOKEN_REF_PATH");
  });

  it("font-weight literal must be a finite number", () => {
    expect(codesOf(withProperty({ kind: "font-weight", value: "bold" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("leading TokenRef path is validated", () => {
    expect(codesOf(withProperty({ kind: "leading", value: { kind: "token-ref", path: "" } })))
      .toContain("INVALID_TOKEN_REF_PATH");
  });

  it("opacity out of [0,1] throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "opacity", value: 1.5 })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("widow-orphan non-integer throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "widow-orphan", value: 1.5 })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("visible non-boolean throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "visible", value: "yes" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("font-style enum violation throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "font-style", value: "slanted" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("text-transform enum violation throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "text-transform", value: "title-case" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("align enum violation throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "align", value: "middle" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("vertical-align enum violation throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "vertical-align", value: "south" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("column-break/page-break enum violation throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "column-break", value: "now" })))
      .toContain("INVALID_PROPERTY_VALUE");
    expect(codesOf(withProperty({ kind: "page-break", value: "now" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("display enum violation throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "display", value: "flex" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("font-family non-array, non-TokenRef value throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "font-family", value: "Inter" })))
      .toContain("INVALID_PROPERTY_VALUE");
  });

  it("padding missing side throws INVALID_PROPERTY_VALUE", () => {
    expect(codesOf(withProperty({ kind: "padding", value: {
      top: { unit: "px", value: 0 }, right: { unit: "px", value: 0 }, bottom: { unit: "px", value: 0 },
    } }))).toContain("INVALID_PROPERTY_VALUE");
  });

  it("text-decoration enum + sub-shape", () => {
    const codes = codesOf(withProperty({ kind: "text-decoration", value: {
      line: "swoosh", color: { kind: "rgb", r: 999, g: 0, b: 0 },
    } }));
    expect(codes).toContain("INVALID_PROPERTY_VALUE");
    expect(codes).toContain("INVALID_COLOR_CHANNEL");
  });

  it("border style enum violation", () => {
    expect(codesOf(withProperty({ kind: "border", value: {
      width: { unit: "px", value: 1 }, style: "wavy",
      color: { kind: "rgb", r: 0, g: 0, b: 0 },
    } }))).toContain("INVALID_PROPERTY_VALUE");
  });

  it("border sides outside allowed strings", () => {
    expect(codesOf(withProperty({ kind: "border", value: {
      width: { unit: "px", value: 1 }, style: "solid",
      color: { kind: "rgb", r: 0, g: 0, b: 0 },
      sides: ["start"],
    } }))).toContain("INVALID_PROPERTY_VALUE");
  });

  it("shadow value validates Shadow shape", () => {
    expect(codesOf(withProperty({ kind: "shadow", value: {
      offsetX: { unit: "leagues", value: 0 }, offsetY: { unit: "px", value: 0 },
      blur: { unit: "px", value: 0 }, spread: { unit: "px", value: 0 },
      color: { kind: "rgb", r: 0, g: 0, b: 0 },
    } }))).toContain("INVALID_LENGTH_UNIT");
  });

  it("shadow can be a TokenRef", () => {
    expect(codesOf(withProperty({ kind: "shadow", value: { kind: "token-ref", path: "shadows.card" } })))
      .toEqual([]);
  });
});

describe("TokenSet errors", () => {
  it("rejects malformed color value in tokens.colors", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, colors: { primary: { kind: "rgb", r: "nope" as unknown as number, g: 0, b: 0 } } } };
    expect(codesOf(doc)).toContain("INVALID_COLOR_CHANNEL");
  });

  it("rejects bad TokenRef in tokens.colors", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, colors: { primary: { kind: "token-ref", path: "" } } } };
    expect(codesOf(doc)).toContain("INVALID_TOKEN_REF_PATH");
  });

  it("rejects malformed length in tokens.space", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, space: { md: { unit: "px", value: NaN } } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("rejects malformed shadow", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, shadows: { card: "nope" as unknown as object } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("rejects shadow with non-boolean inset", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, shadows: { card: {
      offsetX: { unit: "px", value: 0 }, offsetY: { unit: "px", value: 0 },
      blur: { unit: "px", value: 0 }, spread: { unit: "px", value: 0 },
      color: { kind: "rgb", r: 0, g: 0, b: 0 }, inset: "yes",
    } } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("rejects typography that isn't an object", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, typography: "no" as unknown as object } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("rejects typography.families with non-string entries", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, typography: {
      ...ts.tokens.typography, families: { body: ["Inter", 42 as unknown as string] },
    } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("rejects typography.weights with non-number values", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, typography: {
      ...ts.tokens.typography, weights: { regular: "bold" as unknown as number },
    } } };
    expect(codesOf(doc)).toContain("MALFORMED");
  });

  it("rejects extensions key not matching ext:<package>[:<group>]", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, extensions: { "not-an-ext-key": {} } } };
    expect(codesOf(doc)).toContain("INVALID_EXTENSION_KEY");
  });

  it("accepts well-formed extensions", () => {
    const ts = emptyStyleDocument();
    const doc = { ...ts, tokens: { ...ts.tokens, extensions: { "ext:my-plugin:palette": { primary: "#fff" } } } };
    expect(codesOf(doc)).toEqual([]);
  });
});

describe("warnings (soft, not thrown)", () => {
  it("warns when a rule's context is not a kernel-recognised name", () => {
    const doc = {
      ...emptyStyleDocument(),
      rules: [{ id: "r", selector: sel.type("p"), properties: [], context: "darkk" }],
    };
    const result = validateStyleDocument(doc);
    expect(result.warnings.length).toBe(1);
    expect(result.warnings[0]!.code).toBe("UNKNOWN_CONTEXT");
  });

  it("warns when a rule references a recognised context not declared in document.contexts", () => {
    const doc = {
      ...emptyStyleDocument(),
      contexts: [],   // doc declares none
      rules: [{ id: "r", selector: sel.type("p"), properties: [], context: "dark" }],
    };
    const result = validateStyleDocument(doc);
    expect(result.warnings.length).toBe(1);
    expect(result.warnings[0]!.code).toBe("CONTEXT_NOT_DECLARED");
  });

  it("does NOT warn for ext:* context not declared (extensions are open-ended)", () => {
    const doc = {
      ...emptyStyleDocument(),
      contexts: [],
      rules: [{ id: "r", selector: sel.type("p"), properties: [], context: "ext:my-plugin:state-x" }],
    };
    expect(validateStyleDocument(doc).warnings).toEqual([]);
  });

  it("rejects non-string context with MALFORMED, not a warning", () => {
    const doc = {
      ...emptyStyleDocument(),
      rules: [{ id: "r", selector: sel.type("p"), properties: [], context: 42 }],
    };
    expect(codesOf(doc)).toContain("MALFORMED");
  });
});

describe("multi-error collection in one pass", () => {
  it("collects errors from multiple rules and properties without bailing", () => {
    const doc = {
      ...emptyStyleDocument(),
      rules: [
        { id: "", selector: { kind: "wat" }, properties: [] },                                // 2 errors here
        { id: "r2", selector: sel.type("p"), properties: [{ kind: "huh", value: 1 }] },       // 1 error
        { id: "r2", selector: sel.type("p"), properties: [] },                                // duplicate id
      ],
    };
    const codes = codesOf(doc);
    expect(codes).toContain("EMPTY_RULE_ID");
    expect(codes).toContain("UNKNOWN_SELECTOR_KIND");
    expect(codes).toContain("UNKNOWN_PROPERTY_KIND");
    expect(codes).toContain("DUPLICATE_RULE_ID");
    expect(codes.length).toBeGreaterThanOrEqual(4);
  });
});

describe("StyleError formatting", () => {
  it("single-entry message is concise", () => {
    // Null root bails on the first check, so we get exactly one entry.
    try {
      validateStyleDocument(null);
      throw new Error("expected throw");
    } catch (e) {
      expect(e).toBeInstanceOf(StyleError);
      expect((e as StyleError).errors.length).toBe(1);
      expect((e as StyleError).message).toMatch(/^StyleError: MALFORMED at /);
    }
  });

  it("multi-entry message bullet-lists each error", () => {
    try {
      validateStyleDocument({
        kind: "Bad", tokens: {}, contexts: 5 as unknown as string[],
        theme: 5 as unknown as null, rules: [],
      });
    } catch (e) {
      expect((e as StyleError).message).toMatch(/violations:/);
      expect((e as StyleError).errors.length).toBeGreaterThan(1);
    }
  });

  it("entries are frozen", () => {
    try {
      validateStyleDocument(null);
    } catch (e) {
      const entry = (e as StyleError).errors[0]!;
      expect(() => ((entry as { code: string }).code = "X")).toThrow();
    }
  });
});
