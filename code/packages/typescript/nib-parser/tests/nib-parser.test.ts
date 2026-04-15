import { describe, expect, it } from "vitest";

import { parseNib } from "../src/index.js";

interface AstLike {
  ruleName: string;
  children?: unknown[];
}

function collectRuleNames(node: unknown, result: string[] = []): string[] {
  if (!node || typeof node !== "object") {
    return result;
  }

  const candidate = node as AstLike;
  if (typeof candidate.ruleName === "string") {
    result.push(candidate.ruleName);
  }

  for (const child of candidate.children ?? []) {
    collectRuleNames(child, result);
  }

  return result;
}

describe("nib-parser", () => {
  it("parses an empty program", () => {
    const ast = parseNib("");
    expect(ast.ruleName).toBe("program");
  });

  it("parses a function with a typed let statement", () => {
    const ast = parseNib("fn main() { let x: u4 = 5; }");
    const ruleNames = collectRuleNames(ast);

    expect(ruleNames).toContain("program");
    expect(ruleNames).toContain("fn_decl");
    expect(ruleNames).toContain("block");
    expect(ruleNames).toContain("let_stmt");
  });

  it("parses a for-loop over a Nib range", () => {
    const ast = parseNib("fn main() { for i: u8 in 0..10 { } }");
    expect(collectRuleNames(ast)).toContain("for_stmt");
  });

  it("rejects a missing statement semicolon", () => {
    expect(() => parseNib("fn main() { let x: u4 = 5 }")).toThrow();
  });
});
