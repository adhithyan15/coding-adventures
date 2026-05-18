/**
 * compose.test.ts — `composeWithTheme` semantics.
 *
 * The big-ticket invariants:
 *
 * - Base tokens NOT mentioned by the theme stay at their base value.
 * - Theme tokens override per-named-entry.
 * - Bucket-level merge: a theme adding one color doesn't wipe other
 *   base colors.
 * - Typography is bucket-nested; same rules apply to families /
 *   scale / weights / leading / tracking individually.
 * - Theme rules are APPENDED to base rules in order (FM04 §4.9
 *   specificity = source order).
 * - Inputs are not mutated.
 * - Empty theme (no overrides, no rules) is a no-op shape-wise.
 * - Reproducibility: same inputs → byte-identical canonical output.
 * - Prototype-pollution defence (deny-list keys silently dropped).
 */

import { describe, it, expect } from "vitest";
import {
  canonicalStyleDocument,
  emptyStyleDocument, styleRuleId, sel,
  type StyleDocument, type Theme, type StyleRule,
} from "@coding-adventures/forme-style-ir";
import { composeWithTheme } from "../src/index.js";

// ─── Fixtures ────────────────────────────────────────────────────────────

function baseDoc(): StyleDocument {
  return {
    kind: "StyleDocument",
    tokens: {
      colors: {
        text: { kind: "rgb", r: 31, g: 35, b: 40 },
        link: { kind: "rgb", r: 9,  g: 105, b: 218 },
      },
      typography: {
        families: { body: ["Inter", "sans-serif"] },
        scale:    { md: { unit: "rem", value: 1 } },
        weights:  { regular: 400 },
        leading:  { normal: 1.5 },
        tracking: { normal: { unit: "em", value: 0 } },
      },
      space:   { md: { unit: "rem", value: 1 } },
      radii:   { sm: { unit: "px", value: 4 } },
      shadows: {},
    },
    rules: [
      {
        id: styleRuleId("body"),
        selector: sel.type("paragraph"),
        properties: [
          { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
        ],
      },
    ],
    contexts: [],
    theme: null,
  };
}

// ─── Tests ───────────────────────────────────────────────────────────────

describe("composeWithTheme — token override", () => {
  it("preserves base tokens not overridden by the theme", () => {
    const base = baseDoc();
    const theme: Theme = {
      name: "dark",
      tokens: {
        colors: {
          // Override `text` only.  `link` should survive.
          text: { kind: "rgb", r: 240, g: 240, b: 240 },
        },
      },
    };
    const out = composeWithTheme(base, theme);
    expect(out.tokens.colors.text).toEqual({ kind: "rgb", r: 240, g: 240, b: 240 });
    expect(out.tokens.colors.link).toEqual({ kind: "rgb", r: 9, g: 105, b: 218 });
  });

  it("overrides per-named-entry within a typography sub-bucket", () => {
    const base = baseDoc();
    // Theme.tokens is Partial<TokenSet> at the bucket level — but the
    // typography sub-buckets themselves should also be partial-overridable
    // at runtime per FM04 §7.2.  We cast the typography sub-record
    // because the static type is conservatively "full TypographyTokens"
    // (TypeScript's Partial<> doesn't recurse into nested types).
    const theme = {
      name: "compact",
      tokens: {
        typography: {
          // Override scale.md only.  Other typography sub-buckets and
          // other scale entries should be untouched.
          scale: { md: { unit: "rem", value: 0.875 } },
        },
      },
    } as unknown as Theme;
    const out = composeWithTheme(base, theme);
    expect(out.tokens.typography.scale.md).toEqual({ unit: "rem", value: 0.875 });
    expect(out.tokens.typography.families.body).toEqual(["Inter", "sans-serif"]);
    expect(out.tokens.typography.weights.regular).toBe(400);
  });

  it("a theme adding one color doesn't drop other base colors", () => {
    const base = baseDoc();
    const theme: Theme = {
      name: "extra",
      tokens: {
        colors: {
          accent: { kind: "named", name: "tomato" },
        },
      },
    };
    const out = composeWithTheme(base, theme);
    expect(Object.keys(out.tokens.colors).sort()).toEqual(["accent", "link", "text"]);
  });

  it("a missing theme.tokens leaves base.tokens untouched (reference-equal bucket OK)", () => {
    const base = baseDoc();
    const theme: Theme = { name: "rules-only" };
    const out = composeWithTheme(base, theme);
    expect(out.tokens).toBe(base.tokens);
  });
});

describe("composeWithTheme — rules append", () => {
  it("appends theme.rules after base.rules in order", () => {
    const base = baseDoc();
    const extra: StyleRule = {
      id: styleRuleId("heading"),
      selector: sel.type("heading"),
      properties: [
        { kind: "font-weight", value: { kind: "token-ref", path: "typography.weights.bold" } },
      ],
    };
    const theme: Theme = { name: "headings", rules: [extra] };
    const out = composeWithTheme(base, theme);
    expect(out.rules.length).toBe(2);
    expect(out.rules[0]!.id).toBe("body");
    expect(out.rules[1]!.id).toBe("heading");
  });

  it("no theme.rules ⇒ rules array is preserved by reference", () => {
    const base = baseDoc();
    const theme: Theme = { name: "tokens-only", tokens: { colors: {} } };
    const out = composeWithTheme(base, theme);
    expect(out.rules).toBe(base.rules);
  });

  it("multiple theme rules retain their relative order", () => {
    const base = baseDoc();
    const r1: StyleRule = {
      id: styleRuleId("r1"), selector: sel.type("h1"), properties: [],
    };
    const r2: StyleRule = {
      id: styleRuleId("r2"), selector: sel.type("h2"), properties: [],
    };
    const r3: StyleRule = {
      id: styleRuleId("r3"), selector: sel.type("h3"), properties: [],
    };
    const out = composeWithTheme(base, { name: "h", rules: [r1, r2, r3] });
    expect(out.rules.map((r) => r.id)).toEqual(["body", "r1", "r2", "r3"]);
  });
});

describe("composeWithTheme — immutability", () => {
  it("does not mutate the base document", () => {
    const base = baseDoc();
    const snapshot = canonicalStyleDocument(base);
    composeWithTheme(base, {
      name: "x",
      tokens: { colors: { text: { kind: "named", name: "white" } } },
      rules: [{
        id: styleRuleId("r"), selector: sel.type("paragraph"), properties: [],
      }],
    });
    expect(canonicalStyleDocument(base)).toBe(snapshot);
  });

  it("does not mutate the theme", () => {
    const base = baseDoc();
    const theme: Theme = {
      name: "x",
      tokens: { colors: { text: { kind: "named", name: "white" } } },
    };
    const before = JSON.stringify(theme);
    composeWithTheme(base, theme);
    expect(JSON.stringify(theme)).toBe(before);
  });
});

describe("composeWithTheme — passthrough fields", () => {
  it("preserves contexts, theme, and extensions from base", () => {
    const base = baseDoc();
    const richBase: StyleDocument = {
      ...base,
      contexts: ["context:print"],
      theme: "brand",
      extensions: { "ext:plugin:meta": { version: 1 } },
    };
    const out = composeWithTheme(richBase, { name: "x" });
    expect(out.contexts).toEqual(["context:print"]);
    expect(out.theme).toBe("brand");
    expect(out.extensions).toEqual({ "ext:plugin:meta": { version: 1 } });
  });

  it("preserves base extensions even when overlay has its own", () => {
    const base = baseDoc();
    const richBase: StyleDocument = {
      ...base,
      extensions: { "ext:a:meta": { v: 1 } },
    };
    const out = composeWithTheme(richBase, { name: "x" });
    expect(out.extensions).toEqual({ "ext:a:meta": { v: 1 } });
  });

  it("kind discriminant is preserved", () => {
    const out = composeWithTheme(emptyStyleDocument(), { name: "noop" });
    expect(out.kind).toBe("StyleDocument");
  });
});

describe("composeWithTheme — token extensions bucket", () => {
  it("uses overlay-only extensions when base has none", () => {
    // Covers the `base.extensions ?? {}` branch in mergeTokens.
    const base = baseDoc();
    const theme: Theme = {
      name: "x",
      tokens: { extensions: { "ext:b:meta": { v: 2 } } },
    };
    const out = composeWithTheme(base, theme);
    expect(out.tokens.extensions).toEqual({ "ext:b:meta": { v: 2 } });
  });

  it("merges extensions per-named-entry when present in either side", () => {
    const base = baseDoc();
    const richBase: StyleDocument = {
      ...base,
      tokens: {
        ...base.tokens,
        extensions: { "ext:a:meta": { v: 1 } },
      },
    };
    const theme: Theme = {
      name: "x",
      tokens: {
        extensions: { "ext:b:meta": { v: 2 } },
      },
    };
    const out = composeWithTheme(richBase, theme);
    expect(out.tokens.extensions).toEqual({
      "ext:a:meta": { v: 1 },
      "ext:b:meta": { v: 2 },
    });
  });
});

describe("composeWithTheme — reproducibility (FM04 §12)", () => {
  it("same inputs produce byte-identical canonical output", () => {
    const base = baseDoc();
    const theme: Theme = {
      name: "dark",
      tokens: { colors: { text: { kind: "named", name: "white" } } },
      rules: [{
        id: styleRuleId("dark-only"),
        selector: sel.type("paragraph"),
        properties: [
          { kind: "background", value: { kind: "named", name: "black" } },
        ],
      }],
    };
    const a = canonicalStyleDocument(composeWithTheme(base, theme));
    const b = canonicalStyleDocument(composeWithTheme(base, theme));
    expect(a).toBe(b);
  });
});

describe("composeWithTheme — security (prototype-pollution defence)", () => {
  it("silently drops a forbidden __proto__ key in theme overrides", () => {
    const base = baseDoc();
    // Build a theme that bypasses the type system (a malicious or
    // buggy stage that didn't go through the validator).
    const pollutedOverlay: Record<string, unknown> = {};
    pollutedOverlay["__proto__"] = { polluted: true };
    pollutedOverlay["accent"] = { kind: "named", name: "tomato" };
    const theme = {
      name: "polluted",
      tokens: { colors: pollutedOverlay as unknown as Record<string, never> },
    } as unknown as Theme;

    const out = composeWithTheme(base, theme);
    // The legitimate key landed.
    expect(out.tokens.colors).toHaveProperty("accent");
    // The pollution did NOT land in the merged record.
    expect(Object.prototype.hasOwnProperty.call(out.tokens.colors, "__proto__")).toBe(false);
    // And critically, `Object.prototype` was not poisoned.
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("silently drops a forbidden prototype key", () => {
    const base = baseDoc();
    const overlay: Record<string, unknown> = {
      prototype: { evil: true },
      good: { unit: "px", value: 12 },
    };
    const theme = {
      name: "x",
      tokens: { radii: overlay as unknown as Record<string, never> },
    } as unknown as Theme;
    const out = composeWithTheme(base, theme);
    expect(out.tokens.radii).toHaveProperty("good");
    expect(out.tokens.radii).toHaveProperty("sm");           // base preserved
    expect(Object.prototype.hasOwnProperty.call(out.tokens.radii, "prototype")).toBe(false);
  });

  it("silently drops a forbidden constructor key", () => {
    const base = baseDoc();
    const overlay: Record<string, unknown> = { constructor: { evil: true }, ok: { unit: "px", value: 8 } };
    const theme = {
      name: "x",
      tokens: { space: overlay as unknown as Record<string, never> },
    } as unknown as Theme;
    const out = composeWithTheme(base, theme);
    expect(out.tokens.space).toHaveProperty("ok");
    // We back the merged record with `Object.create(null)` AND refuse
    // the "constructor" key.  So the overlay's `constructor: {evil:true}`
    // does NOT land, and there's no inherited `constructor` either —
    // the property is genuinely absent.
    expect((out.tokens.space as Record<string, unknown>).constructor).toBeUndefined();
  });
});
