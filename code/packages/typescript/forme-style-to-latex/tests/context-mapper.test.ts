/**
 * context-mapper.test.ts — context → LaTeX conditional.
 */

import { describe, it, expect } from "vitest";
import { contextToLatex, CONTEXT_FLAG_DECLARATIONS } from "../src/index.js";

describe("contextToLatex — kernel-blessed contexts", () => {
  it("maps the seven standard contexts to \\if<flag>", () => {
    expect(contextToLatex("print")).toBe("\\ifprint");
    expect(contextToLatex("screen")).toBe("\\ifscreen");
    expect(contextToLatex("dark")).toBe("\\ifdark");
    expect(contextToLatex("narrow")).toBe("\\ifnarrow");
    expect(contextToLatex("wide")).toBe("\\ifwide");
    expect(contextToLatex("reduced-motion")).toBe("\\ifreducedmotion");
    expect(contextToLatex("high-contrast")).toBe("\\ifhighcontrast");
  });

  it("returns null for ext: contexts", () => {
    expect(contextToLatex("ext:my-plugin:dark-blue")).toBeNull();
  });

  it("returns null for unrecognised contexts", () => {
    expect(contextToLatex("typo")).toBeNull();
  });
});

describe("CONTEXT_FLAG_DECLARATIONS", () => {
  it("declares a \\newif\\if<flag> for every kernel context", () => {
    expect(CONTEXT_FLAG_DECLARATIONS).toEqual([
      "\\newif\\ifprint",
      "\\newif\\ifscreen",
      "\\newif\\ifdark",
      "\\newif\\ifnarrow",
      "\\newif\\ifwide",
      "\\newif\\ifreducedmotion",
      "\\newif\\ifhighcontrast",
    ]);
  });

  it("is frozen (caller mutations don't leak into future calls)", () => {
    expect(Object.isFrozen(CONTEXT_FLAG_DECLARATIONS)).toBe(true);
  });
});
