import { describe, expect, it } from "vitest";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";

import { parseMacsyma } from "../src/index.js";

function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const out: ASTNode[] = [];
  if (node.ruleName === ruleName) out.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) out.push(...findNodes(child, ruleName));
  }
  return out;
}

describe("macsyma parser", () => {
  it("parses a single expression statement", () => {
    const ast = parseMacsyma("x;");
    expect(ast.ruleName).toBe("program");
    expect(findNodes(ast, "statement")).toHaveLength(1);
  });

  it("parses precedence-bearing arithmetic", () => {
    const ast = parseMacsyma("1 + 2 * 3;");
    expect(findNodes(ast, "additive").length).toBeGreaterThan(0);
    expect(findNodes(ast, "multiplicative").length).toBeGreaterThan(0);
  });

  it("parses function definitions and calls", () => {
    const ast = parseMacsyma("f(x) := x^2; diff(f(x), x);");
    expect(findNodes(ast, "statement")).toHaveLength(2);
    expect(findNodes(ast, "postfix").length).toBeGreaterThanOrEqual(2);
  });

  it("parses lists, comparisons, logic, and dollar terminators", () => {
    const ast = parseMacsyma("[1, 2, 3]$ a < b and not false;");
    expect(findNodes(ast, "list").length).toBeGreaterThan(0);
    expect(findNodes(ast, "comparison").length).toBeGreaterThan(0);
    expect(findNodes(ast, "logical_and").length).toBeGreaterThan(0);
  });

  it("parses control-flow grammar forms", () => {
    const ast = parseMacsyma("if x < 0 then -x else x; while x < 3 do x : x + 1;");
    expect(findNodes(ast, "if_expr")).toHaveLength(1);
    expect(findNodes(ast, "while_expr")).toHaveLength(1);
  });
});
