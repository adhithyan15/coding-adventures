/**
 * Tests for @coding-adventures/lattice-parser
 *
 * These tests verify that the Lattice parser correctly produces ASTs for all
 * Lattice constructs: variables, mixins, @include, control flow (@if/@for/@each),
 * functions, @return, @use, and all CSS constructs.
 *
 * The tests use structural checks on the AST (ruleName, children count)
 * rather than string comparisons, matching the pattern established in the
 * json-parser tests.
 */

import { describe, it, expect } from "vitest";
import { parseLattice, createLatticeParser } from "../src/index.js";
import type { ASTNode } from "../src/index.js";
import type { Token } from "@coding-adventures/lexer";

// Helper: check if a child is an ASTNode
function isASTNode(child: ASTNode | Token): child is ASTNode {
  return "ruleName" in child;
}

// Helper: find a descendant node with a given ruleName (depth-first)
function findNode(root: ASTNode, ruleName: string): ASTNode | undefined {
  if (root.ruleName === ruleName) return root;
  for (const child of root.children) {
    if (isASTNode(child)) {
      const found = findNode(child, ruleName);
      if (found) return found;
    }
  }
  return undefined;
}

// Helper: find ALL descendant nodes with a given ruleName
function findAllNodes(root: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (root.ruleName === ruleName) results.push(root);
  for (const child of root.children) {
    if (isASTNode(child)) {
      results.push(...findAllNodes(child, ruleName));
    }
  }
  return results;
}

describe("lattice-parser", () => {
  // =========================================================================
  // Basic API
  // =========================================================================

  describe("API", () => {
    it("parseLattice returns ASTNode with ruleName 'stylesheet'", () => {
      const ast = parseLattice("$x: 1;");
      expect(ast.ruleName).toBe("stylesheet");
    });

    it("createLatticeParser returns GrammarParser with parse()", () => {
      const parser = createLatticeParser("$x: 1;");
      expect(typeof parser.parse).toBe("function");
      const ast = parser.parse();
      expect(ast.ruleName).toBe("stylesheet");
    });

    it("parseLattice and createLatticeParser produce equivalent results", () => {
      const src = "$color: red; h1 { color: $color; }";
      const ast1 = parseLattice(src);
      const ast2 = createLatticeParser(src).parse();
      expect(ast1.ruleName).toBe(ast2.ruleName);
      expect(ast1.children.length).toBe(ast2.children.length);
    });

    it("empty source produces stylesheet", () => {
      const ast = parseLattice("");
      expect(ast.ruleName).toBe("stylesheet");
    });
  });

  // =========================================================================
  // Variable Declarations
  // =========================================================================

  describe("variable declarations", () => {
    it("parses a simple variable declaration", () => {
      const ast = parseLattice("$color: red;");
      expect(ast.ruleName).toBe("stylesheet");
      const varDecl = findNode(ast, "variable_declaration");
      expect(varDecl).toBeDefined();
    });

    it("variable_declaration contains VARIABLE token", () => {
      const ast = parseLattice("$color: red;");
      const varDecl = findNode(ast, "variable_declaration")!;
      const hasVariable = varDecl.children.some(
        (c) => !isASTNode(c) && c.type === "VARIABLE"
      );
      expect(hasVariable).toBe(true);
    });

    it("variable_declaration contains value_list", () => {
      const ast = parseLattice("$size: 16px;");
      const varDecl = findNode(ast, "variable_declaration")!;
      const hasValueList = varDecl.children.some(
        (c) => isASTNode(c) && c.ruleName === "value_list"
      );
      expect(hasValueList).toBe(true);
    });

    it("parses multiple variable declarations", () => {
      const ast = parseLattice("$a: 1; $b: 2; $c: 3;");
      const decls = findAllNodes(ast, "variable_declaration");
      expect(decls.length).toBe(3);
    });

    it("parses dimension value", () => {
      const ast = parseLattice("$size: 16px;");
      const varDecl = findNode(ast, "variable_declaration");
      expect(varDecl).toBeDefined();
    });

    it("parses hash (color) value", () => {
      const ast = parseLattice("$primary: #4a90d9;");
      const varDecl = findNode(ast, "variable_declaration");
      expect(varDecl).toBeDefined();
    });
  });

  // =========================================================================
  // CSS Qualified Rules
  // =========================================================================

  describe("CSS qualified rules", () => {
    it("parses a simple CSS rule", () => {
      const ast = parseLattice("h1 { color: red; }");
      const rule = findNode(ast, "qualified_rule");
      expect(rule).toBeDefined();
    });

    it("qualified_rule has selector_list and block", () => {
      const ast = parseLattice("h1 { color: red; }");
      const rule = findNode(ast, "qualified_rule")!;
      const hasSelectorList = rule.children.some(
        (c) => isASTNode(c) && c.ruleName === "selector_list"
      );
      const hasBlock = rule.children.some(
        (c) => isASTNode(c) && c.ruleName === "block"
      );
      expect(hasSelectorList).toBe(true);
      expect(hasBlock).toBe(true);
    });

    it("parses class selector", () => {
      const ast = parseLattice(".btn { color: blue; }");
      const rule = findNode(ast, "qualified_rule");
      expect(rule).toBeDefined();
    });

    it("parses declaration inside block", () => {
      const ast = parseLattice("h1 { color: red; }");
      const decl = findNode(ast, "declaration");
      expect(decl).toBeDefined();
    });

    it("parses variable reference inside CSS rule", () => {
      const ast = parseLattice("$primary: red; h1 { color: $primary; }");
      const varDecl = findNode(ast, "variable_declaration");
      const decl = findNode(ast, "declaration");
      expect(varDecl).toBeDefined();
      expect(decl).toBeDefined();
    });
  });

  // =========================================================================
  // Mixin Definitions
  // =========================================================================

  describe("mixin definitions", () => {
    it("parses a mixin definition", () => {
      const ast = parseLattice("@mixin button($bg) { background: $bg; }");
      const mixinDef = findNode(ast, "mixin_definition");
      expect(mixinDef).toBeDefined();
    });

    it("mixin_definition has a block", () => {
      const ast = parseLattice("@mixin button($bg) { background: $bg; }");
      const mixinDef = findNode(ast, "mixin_definition")!;
      const hasBlock = mixinDef.children.some(
        (c) => isASTNode(c) && c.ruleName === "block"
      );
      expect(hasBlock).toBe(true);
    });

    it("parses mixin parameters", () => {
      const ast = parseLattice("@mixin button($bg, $fg) { color: $fg; }");
      const params = findNode(ast, "mixin_params");
      expect(params).toBeDefined();
    });

    it("parses mixin parameter with default", () => {
      const ast = parseLattice(
        "@mixin button($bg, $fg: white) { color: $fg; }"
      );
      const params = findNode(ast, "mixin_params");
      expect(params).toBeDefined();
    });

    it("parses mixin with no parameters", () => {
      const ast = parseLattice("@mixin clearfix() { overflow: hidden; }");
      const mixinDef = findNode(ast, "mixin_definition");
      expect(mixinDef).toBeDefined();
    });
  });

  // =========================================================================
  // @include Directives
  // =========================================================================

  describe("@include directives", () => {
    it("parses @include inside a rule", () => {
      const ast = parseLattice(
        "@mixin btn($c) { color: $c; } .b { @include btn(red); }"
      );
      const include = findNode(ast, "include_directive");
      expect(include).toBeDefined();
    });
  });

  // =========================================================================
  // @if / @else Control Flow
  // =========================================================================

  describe("@if control flow", () => {
    it("parses a simple @if directive", () => {
      const ast = parseLattice(
        "@mixin m($t) { @if $t == dark { color: white; } }"
      );
      const ifDir = findNode(ast, "if_directive");
      expect(ifDir).toBeDefined();
    });

    it("if_directive has lattice_expression and block", () => {
      const ast = parseLattice(
        "@mixin m($t) { @if $t == dark { color: white; } }"
      );
      const ifDir = findNode(ast, "if_directive")!;
      const hasExpr = ifDir.children.some(
        (c) => isASTNode(c) && c.ruleName === "lattice_expression"
      );
      const hasBlock = ifDir.children.some(
        (c) => isASTNode(c) && c.ruleName === "block"
      );
      expect(hasExpr).toBe(true);
      expect(hasBlock).toBe(true);
    });

    it("parses @if with @else", () => {
      const ast = parseLattice(
        "@mixin m($t) { @if $t == dark { color: white; } @else { color: black; } }"
      );
      const ifDir = findNode(ast, "if_directive");
      expect(ifDir).toBeDefined();
    });
  });

  // =========================================================================
  // @for Loop
  // =========================================================================

  describe("@for directive", () => {
    it("parses a @for loop", () => {
      const ast = parseLattice(
        "@for $i from 1 through 3 { .item { color: red; } }"
      );
      const forDir = findNode(ast, "for_directive");
      expect(forDir).toBeDefined();
    });

    it("for_directive contains VARIABLE token", () => {
      const ast = parseLattice(
        "@for $i from 1 through 3 { .item { color: red; } }"
      );
      const forDir = findNode(ast, "for_directive")!;
      const hasVar = forDir.children.some(
        (c) => !isASTNode(c) && c.type === "VARIABLE"
      );
      expect(hasVar).toBe(true);
    });
  });

  // =========================================================================
  // @each Loop
  // =========================================================================

  describe("@each directive", () => {
    it("parses a @each loop", () => {
      const ast = parseLattice(
        "@each $color in red, green, blue { .t { color: $color; } }"
      );
      const eachDir = findNode(ast, "each_directive");
      expect(eachDir).toBeDefined();
    });

    it("each_directive contains each_list", () => {
      const ast = parseLattice(
        "@each $color in red, green, blue { .t { color: $color; } }"
      );
      const eachDir = findNode(ast, "each_directive")!;
      const hasList = eachDir.children.some(
        (c) => isASTNode(c) && c.ruleName === "each_list"
      );
      expect(hasList).toBe(true);
    });
  });

  // =========================================================================
  // @function and @return
  // =========================================================================

  describe("@function and @return", () => {
    it("parses a @function definition", () => {
      const ast = parseLattice(
        "@function spacing($n) { @return $n * 8px; }"
      );
      const funcDef = findNode(ast, "function_definition");
      expect(funcDef).toBeDefined();
    });

    it("function_definition has function_body", () => {
      const ast = parseLattice(
        "@function spacing($n) { @return $n * 8px; }"
      );
      const funcDef = findNode(ast, "function_definition")!;
      const hasBody = funcDef.children.some(
        (c) => isASTNode(c) && c.ruleName === "function_body"
      );
      expect(hasBody).toBe(true);
    });

    it("function_body contains return_directive", () => {
      const ast = parseLattice(
        "@function spacing($n) { @return $n * 8px; }"
      );
      const returnDir = findNode(ast, "return_directive");
      expect(returnDir).toBeDefined();
    });

    it("return_directive has lattice_expression", () => {
      const ast = parseLattice(
        "@function spacing($n) { @return $n * 8px; }"
      );
      const returnDir = findNode(ast, "return_directive")!;
      const hasExpr = returnDir.children.some(
        (c) => isASTNode(c) && c.ruleName === "lattice_expression"
      );
      expect(hasExpr).toBe(true);
    });
  });

  // =========================================================================
  // @use Directive
  // =========================================================================

  describe("@use directive", () => {
    it("parses a @use directive", () => {
      const ast = parseLattice('@use "colors";');
      const useDir = findNode(ast, "use_directive");
      expect(useDir).toBeDefined();
    });

    it("@use with 'as' alias", () => {
      const ast = parseLattice('@use "colors" as c;');
      const useDir = findNode(ast, "use_directive");
      expect(useDir).toBeDefined();
    });
  });

  // =========================================================================
  // CSS At-Rules
  // =========================================================================

  describe("CSS at-rules", () => {
    it("parses @media query", () => {
      const ast = parseLattice("@media (max-width: 768px) { h1 { color: red; } }");
      const atRule = findNode(ast, "at_rule");
      expect(atRule).toBeDefined();
    });

    it("parses @import statement", () => {
      const ast = parseLattice('@import url("style.css");');
      const atRule = findNode(ast, "at_rule");
      expect(atRule).toBeDefined();
    });
  });

  // =========================================================================
  // Expressions
  // =========================================================================

  describe("expressions", () => {
    it("parses arithmetic in @return", () => {
      const ast = parseLattice(
        "@function double($n) { @return $n * 2; }"
      );
      const expr = findNode(ast, "lattice_expression");
      expect(expr).toBeDefined();
    });

    it("parses comparison in @if", () => {
      const ast = parseLattice(
        "@mixin m($x) { @if $x > 10 { color: red; } }"
      );
      const comparison = findNode(ast, "lattice_comparison");
      expect(comparison).toBeDefined();
    });

    it("parses logical AND operator", () => {
      const ast = parseLattice(
        "@mixin m($a, $b) { @if $a == 1 and $b == 2 { color: red; } }"
      );
      const andExpr = findNode(ast, "lattice_and_expr");
      expect(andExpr).toBeDefined();
    });
  });

  // =========================================================================
  // Complex Programs
  // =========================================================================

  describe("complex programs", () => {
    it("parses variable + CSS rule", () => {
      const ast = parseLattice("$color: red; h1 { color: $color; }");
      expect(ast.ruleName).toBe("stylesheet");
      expect(ast.children.length).toBeGreaterThan(0);
    });

    it("parses mixin definition and include", () => {
      const src = `
        @mixin flex($dir) {
          display: flex;
          flex-direction: $dir;
        }
        .container {
          @include flex(row);
        }
      `;
      const ast = parseLattice(src);
      const mixinDef = findNode(ast, "mixin_definition");
      const include = findNode(ast, "include_directive");
      expect(mixinDef).toBeDefined();
      expect(include).toBeDefined();
    });

    it("parses function definition and call in value", () => {
      const src = `
        @function spacing($n) {
          @return $n * 8px;
        }
        .card {
          padding: spacing(2);
        }
      `;
      const ast = parseLattice(src);
      const funcDef = findNode(ast, "function_definition");
      const funcCall = findNode(ast, "function_call");
      expect(funcDef).toBeDefined();
      expect(funcCall).toBeDefined();
    });

    it("parses @for loop", () => {
      const src = `
        @for $i from 1 through 3 {
          .col { color: red; }
        }
      `;
      const ast = parseLattice(src);
      const forDir = findNode(ast, "for_directive");
      expect(forDir).toBeDefined();
    });

    it("parses pseudo-class selectors", () => {
      const ast = parseLattice(".btn:hover { color: blue; }");
      const pseudoClass = findNode(ast, "pseudo_class");
      expect(pseudoClass).toBeDefined();
    });

    it("parses attribute selectors", () => {
      const ast = parseLattice("[type=text] { border: 1px; }");
      const attrSel = findNode(ast, "attribute_selector");
      expect(attrSel).toBeDefined();
    });

    it("parses !important declaration", () => {
      const ast = parseLattice("h1 { color: red !important; }");
      const priority = findNode(ast, "priority");
      expect(priority).toBeDefined();
    });

    it("parses function calls in values (CSS built-ins)", () => {
      const ast = parseLattice("h1 { color: rgb(255, 0, 0); }");
      const funcCall = findNode(ast, "function_call");
      expect(funcCall).toBeDefined();
    });
  });
});
