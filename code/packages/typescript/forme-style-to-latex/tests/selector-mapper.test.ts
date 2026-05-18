/**
 * selector-mapper.test.ts — Selector → LaTeX macro name.
 */

import { describe, it, expect } from "vitest";
import { sel } from "@coding-adventures/forme-style-ir";
import { selectorToLatex } from "../src/index.js";

describe("selectorToLatex — simple selectors", () => {
  it("node-type → \\formeNode<Type>", () => {
    const emit = selectorToLatex(sel.type("paragraph"));
    expect(emit.ok).toBe(true);
    if (emit.ok) {
      expect(emit.macroName).toBe("\\formeNodeParagraph");
      expect(emit.description).toBe("node type: paragraph");
    }
  });

  it("node-type-level → \\formeHeading<Word>", () => {
    const emit = selectorToLatex({ kind: "node-type-level", level: 1 });
    expect(emit.ok).toBe(true);
    if (emit.ok) expect(emit.macroName).toBe("\\formeHeadingOne");
  });

  it("node-type-level handles 2-6", () => {
    const names = ["Two", "Three", "Four", "Five", "Six"];
    for (let i = 0; i < names.length; i++) {
      const emit = selectorToLatex({ kind: "node-type-level", level: i + 2 });
      expect(emit.ok).toBe(true);
      if (emit.ok) expect(emit.macroName).toBe(`\\formeHeading${names[i]}`);
    }
  });

  it("node-type-level out-of-range encodes via latexIdent (Z<hex>Z)", () => {
    const emit = selectorToLatex({ kind: "node-type-level", level: 12 });
    expect(emit.ok).toBe(true);
    if (emit.ok) {
      // "12" → "Z31ZZ32Z" (digits 1, 2 encoded)
      expect(emit.macroName).toBe("\\formeHeadingZ31ZZ32Z");
    }
  });

  it("node-type-level negative numerics encode safely (no raw '-')", () => {
    const emit = selectorToLatex({ kind: "node-type-level", level: -1 });
    expect(emit.ok).toBe(true);
    if (emit.ok) {
      // No raw `-` (invalid in LaTeX command names).
      expect(emit.macroName).not.toContain("-");
    }
  });

  it("node-type-level fractional numerics encode safely (no raw '.')", () => {
    const emit = selectorToLatex({ kind: "node-type-level", level: 1.5 });
    expect(emit.ok).toBe(true);
    if (emit.ok) {
      expect(emit.macroName).not.toContain(".");
    }
  });

  it("custom-kind → \\formeKind<Slug>", () => {
    const emit = selectorToLatex({ kind: "custom-kind", customKind: "Callout" });
    expect(emit.ok).toBe(true);
    if (emit.ok) expect(emit.macroName).toBe("\\formeKindCallout");
  });

  it("tag → \\formeTag<Slug>", () => {
    const emit = selectorToLatex({ kind: "tag", tag: "warning" });
    expect(emit.ok).toBe(true);
    if (emit.ok) expect(emit.macroName).toBe("\\formeTagWarning");
  });

  it("id → \\formeId<Slug>", () => {
    const emit = selectorToLatex({ kind: "id", id: "main" });
    expect(emit.ok).toBe(true);
    if (emit.ok) expect(emit.macroName).toBe("\\formeIdMain");
  });

  it("role → \\formeRole<Slug>", () => {
    const emit = selectorToLatex({ kind: "role", role: "note" });
    expect(emit.ok).toBe(true);
    if (emit.ok) expect(emit.macroName).toBe("\\formeRoleNote");
  });
});

describe("selectorToLatex — identifier sanitisation", () => {
  it("hyphens in names are encoded so macro names stay valid", () => {
    const emit = selectorToLatex(sel.type("block-quote"));
    expect(emit.ok).toBe(true);
    if (emit.ok) {
      // Must NOT contain a literal hyphen in the macro name (LaTeX
      // command-name grammar forbids it).
      expect(emit.macroName).not.toContain("-");
      // Must still contain the encoded form.
      expect(emit.macroName).toContain("Z2dZ");
    }
  });

  it("LaTeX-special chars in selector targets are encoded", () => {
    const emit = selectorToLatex({ kind: "tag", tag: "a%b" });
    expect(emit.ok).toBe(true);
    if (emit.ok) {
      // No raw % survives.
      expect(emit.macroName).not.toContain("%");
      // Encoded with Z25Z (25 = hex for %).
      expect(emit.macroName).toContain("Z25Z");
    }
  });

  it("ASCII control characters are stripped before encoding", () => {
    const emit = selectorToLatex({ kind: "id", id: "main\x00x" });
    expect(emit.ok).toBe(true);
    if (emit.ok) expect(emit.macroName).toBe("\\formeIdMainx");
  });
});

describe("selectorToLatex — composition selectors warn-skip", () => {
  it.each(["nth", "child-of", "descendant-of", "adjacent", "and", "or", "not"] as const)(
    "%s returns warning",
    (kind) => {
      // Construct a minimal selector for each composition kind.
      const inner = sel.type("p");
      let s;
      switch (kind) {
        case "nth":           s = { kind: "nth" as const, n: 0, of: inner }; break;
        case "child-of":      s = { kind: "child-of" as const, parent: inner, child: inner }; break;
        case "descendant-of": s = { kind: "descendant-of" as const, ancestor: inner, descendant: inner }; break;
        case "adjacent":      s = { kind: "adjacent" as const, previous: inner, following: inner }; break;
        case "and":           s = { kind: "and" as const, all: [inner, inner] }; break;
        case "or":            s = { kind: "or" as const, any: [inner, inner] }; break;
        case "not":           s = { kind: "not" as const, inner }; break;
      }
      const emit = selectorToLatex(s);
      expect(emit.ok).toBe(false);
      if (!emit.ok) expect(emit.warning).toMatch(/composition|structural/);
    },
  );
});
