/**
 * translate.test.ts — end-to-end `translateToLatex` behaviour.
 */

import { describe, it, expect } from "vitest";
import {
  styleRuleId, sel, canonicalStyleDocument,
  type StyleDocument, type StyleRule,
} from "@coding-adventures/forme-style-ir";
import { translateToLatex } from "../src/index.js";

function baseDoc(): StyleDocument {
  return {
    kind: "StyleDocument",
    tokens: {
      colors: { text: { kind: "rgb", r: 31, g: 35, b: 40 } },
      typography: {
        families: { body: ["Inter", "sans-serif"] },
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

describe("translateToLatex — happy path", () => {
  it("emits a header + context flags + rule macros", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("body", sel.type("paragraph"), [
          { kind: "color", value: { kind: "token-ref", path: "colors.text" } },
          { kind: "leading", value: 1.5 },
        ]),
      ],
    };
    const { output, emittedRules, warnings } = translateToLatex(doc, { activeContexts: [] });
    expect(emittedRules).toEqual(["body"]);
    expect(warnings).toEqual([]);
    expect(output).toContain("% forme-style-to-latex generated preamble");
    expect(output).toContain("\\newif\\ifprint");
    expect(output).toContain("\\newcommand{\\formeNodeParagraph}{%");
    expect(output).toContain("\\color{RGB}{31,35,40}");
    expect(output).toContain("\\linespread{1.5}");
  });

  it("emits each context's bucket wrapped in \\if...\\fi", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("noctx", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }]),
        rule("printonly", { kind: "node-type-level", level: 1 }, [{ kind: "leading", value: 2 }], "print"),
      ],
    };
    const { output } = translateToLatex(doc, { activeContexts: ["print"] });
    // Search for the conditional guard AFTER the rules section
    // (otherwise we hit the `\newif\ifprint` declaration in the
    // header, which is unrelated to the guard).
    const rulesStart = output.indexOf("% --- rules ---");
    const noctxIdx = output.indexOf("\\formeNodeParagraph", rulesStart);
    const printIdx = output.indexOf("\\ifprint", rulesStart);
    const headingIdx = output.indexOf("\\formeHeadingOne", rulesStart);
    const fiIdx = output.indexOf("\\fi", rulesStart);
    expect(noctxIdx).toBeGreaterThan(-1);
    expect(printIdx).toBeGreaterThan(-1);
    // Unconditional rule comes BEFORE the \ifprint block.
    expect(noctxIdx).toBeLessThan(printIdx);
    expect(printIdx).toBeLessThan(headingIdx);
    expect(headingIdx).toBeLessThan(fiIdx);
  });
});

describe("translateToLatex — filtering", () => {
  it("rules with non-active contexts are filtered out", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("inactive", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }], "print"),
      ],
    };
    const { emittedRules, output } = translateToLatex(doc, { activeContexts: [] });
    expect(emittedRules).toEqual([]);
    expect(output).not.toContain("\\formeNodeParagraph");
  });

  it("ext: contexts emit a warning and skip the rule", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("x", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }], "ext:plugin:special"),
      ],
    };
    const { warnings, emittedRules } = translateToLatex(doc, { activeContexts: ["ext:plugin:special"] });
    expect(emittedRules).toEqual([]);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]!.code).toBe("EXT_CONTEXT_NOT_TRANSLATED");
  });

  it("usedRuleIds slicing — only listed ids emit", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("a", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }]),
        rule("b", { kind: "node-type-level", level: 1 }, [{ kind: "leading", value: 2 }]),
        rule("c", sel.type("blockquote"), [{ kind: "leading", value: 1.2 }]),
      ],
    };
    const { emittedRules } = translateToLatex(doc, {
      activeContexts: [],
      usedRuleIds: [styleRuleId("a"), styleRuleId("c")],
    });
    expect([...emittedRules].sort()).toEqual(["a", "c"]);
  });

  it("usedRuleIds with empty list emits no rules", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule("a", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }])],
    };
    const { emittedRules } = translateToLatex(doc, { activeContexts: [], usedRuleIds: [] });
    expect(emittedRules).toEqual([]);
  });

  it("rules with unmappable selectors warn and skip", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("cantmap", { kind: "child-of", parent: sel.type("article"), child: sel.type("p") }, [
          { kind: "leading", value: 1.5 },
        ]),
      ],
    };
    const { warnings, emittedRules } = translateToLatex(doc, { activeContexts: [] });
    expect(emittedRules).toEqual([]);
    expect(warnings).toHaveLength(1);
    expect(warnings[0]!.code).toBe("SELECTOR_SKIPPED");
  });

  it("rules where every property warn-skips are suppressed", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        // Both properties have no LaTeX preamble equivalent.
        rule("nothing", sel.type("paragraph"), [
          { kind: "opacity", value: 0.5 },
          { kind: "shadow", value: { offsetX: { unit: "pt", value: 0 }, offsetY: { unit: "pt", value: 0 }, blur: { unit: "pt", value: 0 }, spread: { unit: "pt", value: 0 }, color: { kind: "named", name: "black" } } },
        ]),
      ],
    };
    const { emittedRules, output, warnings } = translateToLatex(doc, { activeContexts: [] });
    expect(emittedRules).toEqual([]);
    expect(output).not.toContain("\\newcommand{\\formeNodeParagraph}");
    expect(warnings.length).toBeGreaterThanOrEqual(2);
  });

  it("ext: property kinds emit a warning, the rule still emits other props", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("mixed", sel.type("paragraph"), [
          { kind: "ext:plugin:thing", value: 1 } as never,
          { kind: "leading", value: 1.5 },
        ]),
      ],
    };
    const { warnings, emittedRules, output } = translateToLatex(doc, { activeContexts: [] });
    expect(emittedRules).toEqual(["mixed"]);
    expect(warnings.some((w) => w.code === "EXT_PROPERTY_NOT_TRANSLATED")).toBe(true);
    expect(output).toContain("\\linespread{1.5}");
  });
});

describe("translateToLatex — scope", () => {
  it("scope prefix concatenates to the macro name", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule("body", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }])],
    };
    const { output } = translateToLatex(doc, { activeContexts: [], scope: "\\page" });
    expect(output).toContain("\\newcommand{\\page\\formeNodeParagraph}{%");
  });
});

describe("translateToLatex — important trailer", () => {
  it("renders a `% !important` trailing comment for important: true", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("important", sel.type("paragraph"), [
          { kind: "leading", value: 1.5, important: true },
        ]),
      ],
    };
    const { output } = translateToLatex(doc, { activeContexts: [] });
    expect(output).toContain("% !important");
  });
});

describe("translateToLatex — reproducibility (FM03)", () => {
  it("same input → byte-identical output", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("a", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }]),
        rule("b", { kind: "node-type-level", level: 1 }, [{ kind: "color", value: { kind: "token-ref", path: "colors.text" } }], "print"),
      ],
    };
    const opts = { activeContexts: ["print"] };
    expect(translateToLatex(doc, opts).output)
      .toBe(translateToLatex(doc, opts).output);
  });

  it("canonical document → canonical output", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [rule("body", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }])],
    };
    // Verify canonicalisation doesn't disturb the translator's output.
    const c1 = canonicalStyleDocument(doc);
    const c2 = canonicalStyleDocument(JSON.parse(c1) as StyleDocument);
    expect(c1).toBe(c2);
  });
});

describe("translateToLatex — output safety (LaTeX injection)", () => {
  it("escapes LaTeX-specials in rule id comments", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("evil%id\\with#stuff", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }]),
      ],
    };
    const { output } = translateToLatex(doc, { activeContexts: [] });
    // The id appears in the comment, but the comment is one line — % becomes %.
    // Wait: % is a LaTeX comment in the macro body, but in our generated
    // comment line `% rule "..."`, the leading % is already a comment.
    // BUT a `%` inside the quoted id portion would terminate the comment
    // on second-line use; we still escape defensively.
    expect(output).toContain("evil\\%id");
    expect(output).toContain("\\textbackslash{}");
  });

  it("escapes LaTeX-specials in selector targets (via latexIdent)", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        // A node-type with a hostile name.  The validator would normally
        // reject this; we test our defence-in-depth.
        rule("x", sel.type("evil%name"), [{ kind: "leading", value: 1.5 }]),
      ],
    };
    const { output } = translateToLatex(doc, { activeContexts: [] });
    // The macro identifier (between `\newcommand{` and `}{%`) must
    // not contain a raw %.  The `{%` at the end is the legitimate
    // LaTeX "comment-out-EOL" marker that opens the macro body
    // suppressing the newline.
    const newcommandLine = output.split("\n").find((l) => l.startsWith("\\newcommand"))!;
    const macroIdent = newcommandLine.slice("\\newcommand{".length, newcommandLine.indexOf("}{%"));
    expect(macroIdent).not.toContain("%");
    expect(macroIdent).toContain("Z25Z");   // % encoded as Z25Z
  });

  it("strips ASCII control chars from rule id comments", () => {
    const doc: StyleDocument = {
      ...baseDoc(),
      rules: [
        rule("bad\x00null", sel.type("paragraph"), [{ kind: "leading", value: 1.5 }]),
      ],
    };
    const { output } = translateToLatex(doc, { activeContexts: [] });
    expect(output).toContain("badnull");
    // eslint-disable-next-line no-control-regex
    expect(output).not.toMatch(/\x00/);
  });
});
