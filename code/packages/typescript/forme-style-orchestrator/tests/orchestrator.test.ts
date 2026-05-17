/**
 * compile.test.ts — end-to-end orchestration over the FM04 family.
 */

import { describe, it, expect } from "vitest";
import {
  emptyStyleDocument, styleRuleId, sel,
  type StyleDocument, type Theme,
} from "@coding-adventures/forme-style-ir";
import { createThemeRegistry } from "@coding-adventures/forme-style-theme";
import {
  compile, isCompileError, isCompileSuccess, fingerprintDocument,
} from "../src/index.js";

// ─── Fixture ─────────────────────────────────────────────────────────────

function fixture(): StyleDocument {
  return {
    ...emptyStyleDocument(),
    tokens: {
      ...emptyStyleDocument().tokens,
      colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
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
  };
}

// ─── Tests ───────────────────────────────────────────────────────────────

describe("compile — happy path dispatch", () => {
  it("dispatches to the CSS translator", () => {
    const r = compile(fixture(), "css", { activeContexts: [] });
    expect(isCompileSuccess(r)).toBe(true);
    expect(r.target).toBe("css");
    expect(r.output).toContain("color: rgb(31 35 40)");
    expect(r.emittedRules).toEqual(["body"]);
    expect(r.warnings).toEqual([]);
    expect(r.errors).toEqual([]);
  });

  it("dispatches to the LaTeX translator", () => {
    const r = compile(fixture(), "latex", { activeContexts: [] });
    expect(isCompileSuccess(r)).toBe(true);
    expect(r.target).toBe("latex");
    expect(r.output).toContain("\\color{RGB}{31,35,40}");
    expect(r.emittedRules).toEqual(["body"]);
  });

  it("dispatches to the terminal translator", () => {
    const r = compile(fixture(), "terminal", { activeContexts: [] });
    expect(isCompileSuccess(r)).toBe(true);
    expect(r.target).toBe("terminal");
    expect(r.output).toContain("38;2;31;35;40");
    expect(r.emittedRules).toEqual(["body"]);
  });
});

describe("compile — validator failure capture", () => {
  it("captures errors and never throws when validator rejects", () => {
    // `null` is the canonical "this is not a StyleDocument" input.
    const r = compile(null, "css", { activeContexts: [] });
    expect(isCompileError(r)).toBe(true);
    expect(r.output).toBe("");
    expect(r.emittedRules).toEqual([]);
    expect(r.errors.length).toBeGreaterThan(0);
  });

  it("captures errors on a structurally-broken document", () => {
    const broken = { kind: "StyleDocument" /* missing tokens, rules, contexts, theme */ };
    const r = compile(broken, "css", { activeContexts: [] });
    expect(isCompileError(r)).toBe(true);
    expect(r.errors.length).toBeGreaterThan(0);
  });
});

describe("compile — theme composition", () => {
  it("applies a theme passed by value (override colors.text)", () => {
    const theme: Theme = {
      name: "dark",
      tokens: { colors: { text: { kind: "named", name: "white" } } },
    };
    const r = compile(fixture(), "css", { activeContexts: [], theme });
    expect(isCompileSuccess(r)).toBe(true);
    // Override won — `white` lands instead of rgb(31,35,40).
    expect(r.output).toContain("color: white");
  });

  it("applies a theme by name via registry", () => {
    const registry = createThemeRegistry();
    registry.register({
      name: "dark",
      tokens: { colors: { text: { kind: "named", name: "white" } } },
    });
    const r = compile(fixture(), "css", {
      activeContexts: [],
      theme: "dark",
      themeRegistry: registry,
    });
    expect(isCompileSuccess(r)).toBe(true);
    expect(r.output).toContain("color: white");
  });

  it("warns (not errors) when theme name is unknown; proceeds with base", () => {
    const registry = createThemeRegistry();
    const r = compile(fixture(), "css", {
      activeContexts: [],
      theme: "nonexistent",
      themeRegistry: registry,
    });
    expect(isCompileSuccess(r)).toBe(true);
    expect(r.warnings.some((w) => w.code === "THEME_NOT_FOUND")).toBe(true);
    // Base tokens still applied.
    expect(r.output).toContain("color: rgb(31 35 40)");
  });

  it("throws TypeError when theme is a string but no registry supplied", () => {
    expect(() =>
      compile(fixture(), "css", { activeContexts: [], theme: "dark" }),
    ).toThrow(TypeError);
  });

  it("no theme requested ⇒ base tokens emit", () => {
    const r = compile(fixture(), "css", { activeContexts: [] });
    expect(r.output).toContain("color: rgb(31 35 40)");
  });
});

describe("compile — options pass-through", () => {
  it("activeContexts filters context-tagged rules", () => {
    const doc: StyleDocument = {
      ...fixture(),
      rules: [
        ...fixture().rules,
        {
          id: styleRuleId("print-only"),
          selector: sel.type("paragraph"),
          properties: [{ kind: "color", value: { kind: "named", name: "black" } }],
          context: "print",
        },
      ],
    };
    const screen = compile(doc, "css", { activeContexts: ["screen"] });
    expect(screen.emittedRules).toEqual(["body"]);
    const print = compile(doc, "css", { activeContexts: ["print"] });
    expect([...print.emittedRules].sort()).toEqual(["body", "print-only"]);
  });

  it("usedRuleIds slicing trims output", () => {
    const doc: StyleDocument = {
      ...fixture(),
      rules: [
        ...fixture().rules,
        {
          id: styleRuleId("extra"),
          selector: sel.type("heading"),
          properties: [{ kind: "color", value: { kind: "named", name: "black" } }],
        },
      ],
    };
    const r = compile(doc, "css", {
      activeContexts: [],
      usedRuleIds: [styleRuleId("extra")],
    });
    expect(r.emittedRules).toEqual(["extra"]);
  });

  it("scope is forwarded to the CSS translator", () => {
    const r = compile(fixture(), "css", { activeContexts: [], scope: ".page" });
    expect(r.output).toContain(".page paragraph");
  });

  it("scope is forwarded to the terminal translator (Map key prefix)", () => {
    const r = compile(fixture(), "terminal", { activeContexts: [], scope: "abc." });
    expect(r.output).toContain('["abc.body"');
  });
});

describe("compile — unknown target", () => {
  it("throws TypeError when target is not a known backend", () => {
    expect(() =>
      compile(fixture(), "wat" as never, { activeContexts: [] }),
    ).toThrow(TypeError);
  });
});

describe("compile — reproducibility (FM03)", () => {
  it("same inputs → byte-identical output (CSS)", () => {
    const opts = { activeContexts: [] };
    expect(compile(fixture(), "css", opts).output)
      .toBe(compile(fixture(), "css", opts).output);
  });

  it("same inputs + theme → byte-identical output (LaTeX)", () => {
    const theme: Theme = {
      name: "dark",
      tokens: { colors: { text: { kind: "named", name: "white" } } },
    };
    const opts = { activeContexts: [], theme };
    expect(compile(fixture(), "latex", opts).output)
      .toBe(compile(fixture(), "latex", opts).output);
  });
});

describe("isCompileError / isCompileSuccess type guards", () => {
  it("complementary", () => {
    const ok = compile(fixture(), "css", { activeContexts: [] });
    expect(isCompileError(ok)).toBe(false);
    expect(isCompileSuccess(ok)).toBe(true);

    const bad = compile(null, "css", { activeContexts: [] });
    expect(isCompileError(bad)).toBe(true);
    expect(isCompileSuccess(bad)).toBe(false);
  });
});

describe("fingerprintDocument", () => {
  it("returns the canonical-JSON string for a valid document", () => {
    const fp = fingerprintDocument(fixture());
    expect(fp).not.toBeNull();
    expect(fp).toContain("StyleDocument");
    // Stable across calls.
    expect(fingerprintDocument(fixture())).toBe(fp);
  });

  it("returns null for an invalid document", () => {
    expect(fingerprintDocument(null)).toBeNull();
    expect(fingerprintDocument({ kind: "Wrong" })).toBeNull();
  });
});

describe("compile — non-StyleError throwing path is re-raised", () => {
  it("does not swallow unexpected exceptions", () => {
    // We can't easily inject a non-StyleError exception into the
    // validator path without mocking — but the public surface DOES
    // re-raise.  The test pin: nothing in the orchestrator swallows
    // a TypeError thrown by `compile`'s own theme-registry check.
    expect(() =>
      compile(fixture(), "css", {
        activeContexts: [],
        theme: "x",          // string theme name
        themeRegistry: undefined as never,
      }),
    ).toThrow(TypeError);
  });
});
