/**
 * Tests for the Starlark Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses Starlark
 * source code when loaded with the `starlark.grammar` file.
 *
 * The Starlark grammar's top-level rule is `file` (not `program`), reflecting
 * that Starlark files are configuration files evaluated top-to-bottom.
 *
 * Test Strategy
 * -------------
 *
 * Each test parses a source string and then uses helper functions to walk the
 * resulting AST, looking for specific node types. This approach is more robust
 * than checking exact tree structure, because the grammar may wrap nodes in
 * multiple layers of rules (e.g., expression -> or_expr -> and_expr -> ... ->
 * primary -> atom -> NAME).
 */

import { describe, it, expect } from "vitest";
import { parseStarlark } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * This is the workhorse helper for these tests. Since the grammar wraps
 * expressions in many layers of precedence rules, we need to search the
 * entire tree to find the nodes we care about.
 *
 * @param node - The root node to search from.
 * @param ruleName - The grammar rule name to find (e.g., "assign_stmt").
 * @returns All nodes in the tree with the given ruleName.
 */
function findNodes(node: ASTNode, ruleName: string): ASTNode[] {
  const results: ASTNode[] = [];
  if (node.ruleName === ruleName) results.push(node);
  for (const child of node.children) {
    if (isASTNode(child)) results.push(...findNodes(child, ruleName));
  }
  return results;
}

/**
 * Collect all leaf tokens from an AST subtree.
 *
 * Flattens the tree into a list of tokens, which makes it easy to check
 * what tokens are present in a particular subtree without worrying about
 * the exact nesting structure.
 */
function findTokens(node: ASTNode): Token[] {
  const tokens: Token[] = [];
  for (const child of node.children) {
    if (isASTNode(child)) {
      tokens.push(...findTokens(child));
    } else {
      tokens.push(child as Token);
    }
  }
  return tokens;
}

describe("simple assignment", () => {
  it("parses x = 1", () => {
    /**
     * The simplest Starlark statement: assign a value to a name.
     * The top-level rule should be "file" and the tree should contain
     * an "assign_stmt" node.
     */
    const ast = parseStarlark("x = 1");
    expect(ast.ruleName).toBe("file");

    const assignments = findNodes(ast, "assign_stmt");
    expect(assignments).toHaveLength(1);
  });

  it("parses augmented assignment x += 1", () => {
    /**
     * Augmented assignment combines an operator with assignment.
     * The grammar rule assign_stmt handles both simple and augmented forms.
     */
    const ast = parseStarlark("x += 1");
    const assignments = findNodes(ast, "assign_stmt");
    expect(assignments).toHaveLength(1);

    const augOps = findNodes(ast, "augmented_assign_op");
    expect(augOps).toHaveLength(1);
  });
});

describe("expression parsing", () => {
  it("parses x = 1 + 2 with arithmetic expression", () => {
    /**
     * The expression 1 + 2 should be parsed using the operator precedence
     * chain: expression -> or_expr -> ... -> arith -> term -> factor -> ...
     * We verify by looking for a PLUS token somewhere in the AST, confirming
     * the arithmetic expression was parsed correctly.
     */
    const ast = parseStarlark("x = 1 + 2");

    const allTokens = findTokens(ast);
    const plusTokens = allTokens.filter((t) => t.type === "PLUS");
    expect(plusTokens).toHaveLength(1);
  });

  it("parses comparison expression a == b", () => {
    /**
     * Comparisons live at a specific level in the precedence chain.
     * The grammar rule "comparison" handles ==, !=, <, >, <=, >=, in, not in.
     */
    const ast = parseStarlark("a == b");

    const comparisons = findNodes(ast, "comparison");
    expect(comparisons.length).toBeGreaterThanOrEqual(1);
  });
});

describe("function definition with indentation", () => {
  it("parses a simple function definition", () => {
    /**
     * Function definitions in Starlark look like Python:
     *   def greet(name):
     *       return "Hello, " + name
     *
     * The parser should produce a "def_stmt" node. The indented body
     * is wrapped in a "suite" node (which contains INDENT ... DEDENT).
     */
    const source = 'def greet(name):\n    return "Hello, " + name\n';
    const ast = parseStarlark(source);

    const defNodes = findNodes(ast, "def_stmt");
    expect(defNodes).toHaveLength(1);

    // The function should contain a return statement
    const returnNodes = findNodes(ast, "return_stmt");
    expect(returnNodes).toHaveLength(1);
  });

  it("parses a function with default parameter", () => {
    /**
     * Starlark supports default parameter values:
     *   def greet(name, greeting="Hello"):
     *       return greeting + ", " + name
     *
     * The grammar rule "parameter" handles NAME EQUALS expression.
     */
    const source = 'def greet(name, greeting="Hello"):\n    return greeting\n';
    const ast = parseStarlark(source);

    const defNodes = findNodes(ast, "def_stmt");
    expect(defNodes).toHaveLength(1);

    const params = findNodes(ast, "parameters");
    expect(params).toHaveLength(1);
  });
});

describe("if/else blocks", () => {
  it("parses an if/else statement", () => {
    /**
     * Starlark supports if/elif/else with indented bodies:
     *   if x > 0:
     *       result = "positive"
     *   else:
     *       result = "non-positive"
     *
     * The grammar rule "if_stmt" handles the full if/elif/else chain.
     */
    const source = 'if x > 0:\n    result = "positive"\nelse:\n    result = "non-positive"\n';
    const ast = parseStarlark(source);

    const ifNodes = findNodes(ast, "if_stmt");
    expect(ifNodes).toHaveLength(1);

    // Should have two suites: one for if body, one for else body
    const suites = findNodes(ifNodes[0], "suite");
    expect(suites).toHaveLength(2);
  });
});

describe("for loops", () => {
  it("parses a simple for loop", () => {
    /**
     * Starlark has for loops but no while loops (termination guarantee).
     *   for item in items:
     *       process(item)
     *
     * The grammar rule "for_stmt" handles: for loop_vars in expression : suite
     */
    const source = "for x in items:\n    pass\n";
    const ast = parseStarlark(source);

    const forNodes = findNodes(ast, "for_stmt");
    expect(forNodes).toHaveLength(1);

    // Loop variables should be present
    const loopVars = findNodes(ast, "loop_vars");
    expect(loopVars).toHaveLength(1);
  });
});

describe("BUILD-file style function calls", () => {
  it("parses a function call with named arguments", () => {
    /**
     * The most common pattern in BUILD files is a function call with
     * named (keyword) arguments:
     *
     *   cc_library(
     *       name = "mylib",
     *       srcs = ["mylib.cc"],
     *   )
     *
     * In a single line: cc_library(name = "mylib")
     *
     * The grammar handles this through:
     *   assign_stmt -> expression_list -> expression -> ... -> primary -> atom suffix
     * where the suffix is a function call with arguments.
     */
    const source = 'cc_library(name = "mylib")';
    const ast = parseStarlark(source);
    expect(ast.ruleName).toBe("file");

    // The function call should produce an "arguments" node
    const argNodes = findNodes(ast, "arguments");
    expect(argNodes).toHaveLength(1);

    // Named argument: name = "mylib"
    const argumentNodes = findNodes(ast, "argument");
    expect(argumentNodes.length).toBeGreaterThanOrEqual(1);
  });

  it("parses a function call with multiple named arguments", () => {
    /**
     * BUILD files typically have multiple named arguments:
     *   py_library(name = "foo", srcs = ["foo.py"], deps = [":bar"])
     */
    const source = 'py_library(name = "foo", srcs = ["foo.py"])';
    const ast = parseStarlark(source);

    const argNodes = findNodes(ast, "argument");
    expect(argNodes.length).toBeGreaterThanOrEqual(2);
  });
});

describe("multiple statements", () => {
  it("parses two assignments separated by newline", () => {
    /**
     * Multiple statements are separated by newlines. The top-level
     * "file" rule collects them into a sequence.
     */
    const ast = parseStarlark("x = 1\ny = 2");

    const assignments = findNodes(ast, "assign_stmt");
    expect(assignments).toHaveLength(2);
  });

  it("parses a mix of assignment and expression statements", () => {
    /**
     * Expression statements (like bare function calls) are also valid
     * at the top level. They are parsed as assign_stmt without the
     * assignment suffix.
     */
    const source = 'x = 1\nprint("hello")';
    const ast = parseStarlark(source);

    const stmts = findNodes(ast, "simple_stmt");
    expect(stmts).toHaveLength(2);
  });
});

describe("list and dict literals", () => {
  it("parses a list literal", () => {
    /**
     * List literals are common in BUILD files for specifying sources, deps, etc:
     *   srcs = ["a.py", "b.py"]
     */
    const ast = parseStarlark('x = ["a", "b", "c"]');

    const listNodes = findNodes(ast, "list_expr");
    expect(listNodes).toHaveLength(1);
  });
});
