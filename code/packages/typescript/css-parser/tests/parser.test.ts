import { describe, expect, it } from "vitest";

import { createCssParser, parseCSS, parseCss } from "../src/index.js";
import { GrammarParseError, GrammarParser, isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

function findTokens(node: ASTNode, tokenType?: string): Token[] {
  const tokens: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      tokens.push(...findTokens(child, tokenType));
    } else if (tokenType === undefined || child.type === tokenType) {
      tokens.push(child as Token);
    }
  }
  return tokens;
}

describe("CSS parser", () => {
  it("creates a configured grammar parser", () => {
    const parser = createCssParser("h1 { color: red; }");
    expect(parser).toBeInstanceOf(GrammarParser);
    expect(parser.parse().ruleName).toBe("stylesheet");
  });

  it("parses empty and whitespace-only stylesheets", () => {
    expect(parseCss("").children).toHaveLength(0);
    expect(parseCss("  \n\t  ").children).toHaveLength(0);
  });

  it("parses a qualified rule with declarations", () => {
    const ast = parseCss("h1 { color: red; margin: 0; }");

    expect(ast.ruleName).toBe("stylesheet");
    expect(findNodes(ast, "qualified_rule")).toHaveLength(1);
    expect(findNodes(ast, "declaration")).toHaveLength(2);
    expect(findTokens(ast, "IDENT").map((token) => token.value)).toContain("color");
  });

  it("parses selector lists and subclass selectors", () => {
    const ast = parseCss("h1, .active, #main { display: block; }");

    expect(findNodes(ast, "complex_selector")).toHaveLength(3);
    expect(findNodes(ast, "class_selector")).toHaveLength(1);
    expect(findNodes(ast, "id_selector")).toHaveLength(1);
  });

  it("parses combinators, attributes, pseudo classes, and pseudo elements", () => {
    const ast = parseCss('nav > a[href^="https"]:hover::before { content: "go"; }');

    expect(findNodes(ast, "combinator")).toHaveLength(1);
    expect(findNodes(ast, "attribute_selector")).toHaveLength(1);
    expect(findNodes(ast, "pseudo_class")).toHaveLength(1);
    expect(findNodes(ast, "pseudo_element")).toHaveLength(1);
  });

  it("parses at-rules and nested rules", () => {
    const ast = parseCss("@media screen { .parent { color: red; & .child { color: blue; } } }");

    expect(findNodes(ast, "at_rule")).toHaveLength(1);
    expect(findNodes(ast, "qualified_rule").length).toBeGreaterThanOrEqual(2);
  });

  it("parses functions and !important priorities", () => {
    const ast = parseCss(":root { --gap: 12px; width: calc(100% - var(--gap)); color: red !important; }");

    expect(findNodes(ast, "function_call")).toHaveLength(1);
    expect(findNodes(ast, "priority")).toHaveLength(1);
    expect(findTokens(ast, "FUNCTION")).toHaveLength(2);
    expect(findTokens(ast, "CUSTOM_PROPERTY").map((token) => token.value)).toContain("--gap");
  });

  it("exposes a parseCSS alias", () => {
    expect(parseCSS("* { margin: 0; }").ruleName).toBe("stylesheet");
  });

  it("raises grammar parse errors for invalid CSS", () => {
    expect(() => parseCss("h1 { color: red;")).toThrow(GrammarParseError);
  });
});
