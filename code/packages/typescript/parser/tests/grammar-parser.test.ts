/**
 * Tests for the grammar-driven parser.
 *
 * =============================================================================
 * TESTING STRATEGY
 * =============================================================================
 *
 * These tests verify that the GrammarParser correctly interprets .grammar file
 * rules to build ASTs from token streams. Unlike the hand-written parser tests
 * (which construct tokens manually), these tests use the actual python.grammar
 * file to drive parsing — testing the full grammar-driven pipeline.
 *
 * The tests are organized from simple to complex:
 *
 *     1. Helper utilities (astToDict)
 *     2. Single atoms (numbers, strings, names)
 *     3. Binary operations and precedence
 *     4. Parenthesized expressions
 *     5. Assignment statements
 *     6. Multiple statements
 *     7. Edge cases and error handling
 *     8. Tree walking / value extraction
 *
 * Each test:
 *     1. Loads the actual python.grammar file
 *     2. Tokenizes source code using the tokenize() function
 *     3. Feeds tokens to GrammarParser
 *     4. Asserts the resulting generic AST has the expected structure
 * =============================================================================
 */

import { describe, it, expect, beforeAll } from "vitest";
import { readFileSync } from "fs";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

import { parseParserGrammar, parseTokenGrammar } from "@coding-adventures/grammar-tools";
import type { ParserGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize, tokenize } from "@coding-adventures/lexer";
import type { Token } from "@coding-adventures/lexer";

import {
  GrammarParser,
  GrammarParseError,
  isASTNode,
  isLeafNode,
  getLeafToken,
  walkAST,
  findNodes,
  collectTokens,
} from "../src/grammar-parser.js";
import type { ASTNode } from "../src/grammar-parser.js";

// =============================================================================
// FIXTURES — Load the grammar once and reuse across tests
// =============================================================================

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMARS_DIR = join(__dirname, '..', '..', '..', '..', 'grammars');

let grammar: ParserGrammar;

beforeAll(() => {
  /** Load the python.grammar file and parse it into a ParserGrammar. */
  const grammarPath = join(GRAMMARS_DIR, "python.grammar");
  const grammarText = readFileSync(grammarPath, "utf-8");
  grammar = parseParserGrammar(grammarText);
});

// =============================================================================
// HELPER — Convert AST tree to a readable dict for assertions
// =============================================================================

/**
 * Convert a generic ASTNode tree to a readable object for easy assertion.
 *
 * This helper transforms the tree into a JSON-like structure that's easy
 * to compare in assertions. Each ASTNode becomes an object with "rule" and
 * "children" keys. Tokens become strings of the form "TYPE:value".
 *
 * Examples:
 *
 *     astToDict({ ruleName: "factor", children: [{ type: "NUMBER", value: "42", line: 1, column: 1 }] })
 *     // Returns: { rule: "factor", children: ["NUMBER:42"] }
 *
 *     astToDict({ type: "PLUS", value: "+", line: 1, column: 1 })
 *     // Returns: "PLUS:+"
 *
 * @param node - An ASTNode or Token to convert.
 * @returns An object (for ASTNode) or string (for Token) representation.
 */
function astToDict(
  node: ASTNode | Token,
): Record<string, unknown> | string {
  if (!isASTNode(node)) {
    return `${node.type}:${node.value}`;
  }
  return {
    rule: node.ruleName,
    children: node.children.map((child) => astToDict(child)),
  };
}

/**
 * Tokenize source code and parse it with the grammar-driven parser.
 *
 * This is the main helper for tests — it handles the full pipeline from
 * source text to AST.
 *
 * @param source - The source code string to parse.
 * @returns The root ASTNode of the parse tree.
 */
function parseSource(source: string): ASTNode {
  const tokens = tokenize(source);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}

function makeRichSourceTokenGrammar() {
  return parseTokenGrammar([
    "NAME = /[a-zA-Z_][a-zA-Z0-9_]*/",
    "EQ = \"=\"",
    "skip:",
    "  WHITESPACE = /[ \\t\\r\\n]+/",
    "  LINE_COMMENT = /\\/\\/[^\\n]*/",
  ].join("\n"));
}

function makeRichSourceParserGrammar(): ParserGrammar {
  return parseParserGrammar([
    "program = assignment ;",
    "assignment = NAME EQ NAME ;",
  ].join("\n"));
}

// =============================================================================
// TREE TRAVERSAL HELPERS
// =============================================================================
//
// These utilities walk the generic AST tree to find specific nodes or tokens.
// They're used by the tests to verify tree structure without hardcoding
// exact positions.
// =============================================================================

/**
 * Recursively search the tree for a token with the given type and value.
 *
 * @param node - The root of the subtree to search.
 * @param tokenType - The token type to look for (e.g., "NUMBER").
 * @param value - The token value to look for (e.g., "42").
 * @returns True if a matching token was found anywhere in the tree.
 */
function findTokenInTree(
  node: ASTNode | Token,
  tokenType: string,
  value: string,
): boolean {
  if (!isASTNode(node)) {
    return node.type === tokenType && node.value === value;
  }
  for (const child of node.children) {
    if (findTokenInTree(child, tokenType, value)) {
      return true;
    }
  }
  return false;
}

/**
 * Find the first ASTNode with the given ruleName in the tree.
 *
 * Does a depth-first search. Returns the first match found, or null.
 *
 * @param node - The root of the subtree to search.
 * @param ruleName - The rule name to look for (e.g., "expression").
 * @returns The first matching ASTNode, or null if not found.
 */
function findRule(
  node: ASTNode | Token,
  ruleName: string,
): ASTNode | null {
  if (!isASTNode(node)) {
    return null;
  }
  if (node.ruleName === ruleName) {
    return node;
  }
  for (const child of node.children) {
    const result = findRule(child, ruleName);
    if (result !== null) {
      return result;
    }
  }
  return null;
}

/**
 * Collect all tokens of a given type from the tree.
 *
 * Does a depth-first traversal and returns all matching tokens in order.
 *
 * @param node - The root of the subtree to search.
 * @param tokenType - The token type to look for (e.g., "NUMBER").
 * @returns A list of matching Token objects in tree-traversal order.
 */
function collectTokens(
  node: ASTNode | Token,
  tokenType: string,
): Token[] {
  if (!isASTNode(node)) {
    if (node.type === tokenType) {
      return [node];
    }
    return [];
  }
  const tokens: Token[] = [];
  for (const child of node.children) {
    tokens.push(...collectTokens(child, tokenType));
  }
  return tokens;
}

/**
 * Collect all tokens from the tree in depth-first order.
 *
 * @param node - The root of the subtree to traverse.
 * @returns A list of all Token objects in tree-traversal order.
 */
function collectAllTokens(node: ASTNode | Token): Token[] {
  if (!isASTNode(node)) {
    return [node];
  }
  const tokens: Token[] = [];
  for (const child of node.children) {
    tokens.push(...collectAllTokens(child));
  }
  return tokens;
}

// =============================================================================
// TEST: astToDict HELPER
// =============================================================================

describe("TestAstToDict", () => {
  it("should convert a Token to 'TYPE:value'", () => {
    /** A Token should become 'TYPE:value'. */
    const token: Token = { type: "NUMBER", value: "42", line: 1, column: 1 };
    expect(astToDict(token)).toBe("NUMBER:42");
  });

  it("should convert a leaf ASTNode", () => {
    /** An ASTNode with a single token child. */
    const token: Token = { type: "NUMBER", value: "42", line: 1, column: 1 };
    const node: ASTNode = { ruleName: "factor", children: [token] };
    const result = astToDict(node);

    expect(result).toEqual({ rule: "factor", children: ["NUMBER:42"] });
  });

  it("should convert a nested ASTNode tree", () => {
    /** A nested ASTNode tree. */
    const inner: ASTNode = {
      ruleName: "factor",
      children: [{ type: "NUMBER", value: "1", line: 1, column: 1 }],
    };
    const outer: ASTNode = { ruleName: "term", children: [inner] };
    const result = astToDict(outer);

    expect(result).toEqual({
      rule: "term",
      children: [{ rule: "factor", children: ["NUMBER:1"] }],
    });
  });
});

// =============================================================================
// TEST: SINGLE ATOMS
// =============================================================================
//
// The simplest possible inputs: a single value that forms a complete
// expression. These test that the grammar-driven parser can match
// basic token types through the grammar rules.
// =============================================================================

describe("TestSingleAtoms", () => {
  it("should parse a number: 42", () => {
    /** Parsing `42` should produce a program with a single number. */
    const ast = parseSource("42");

    // The root should be a "program" node.
    expect(ast.ruleName).toBe("program");

    // Walk into the tree to find the NUMBER token.
    const tree = astToDict(ast);
    expect((tree as Record<string, unknown>)["rule"]).toBe("program");

    // The program contains a statement -> expression_stmt -> expression
    // -> term -> factor -> NUMBER:42. Let's verify the number is there.
    // Flatten and check the leaf value exists somewhere in the tree.
    expect(findTokenInTree(ast, "NUMBER", "42")).toBe(true);
  });

  it("should parse a string: 'hello'", () => {
    /** Parsing `"hello"` should produce a tree with a STRING token. */
    const ast = parseSource('"hello"');
    expect(ast.ruleName).toBe("program");
    expect(findTokenInTree(ast, "STRING", "hello")).toBe(true);
  });

  it("should parse a name: x", () => {
    /** Parsing `x` should produce a tree with a NAME token. */
    const ast = parseSource("x");
    expect(ast.ruleName).toBe("program");
    expect(findTokenInTree(ast, "NAME", "x")).toBe(true);
  });
});

// =============================================================================
// TEST: BINARY OPERATIONS
// =============================================================================

describe("TestBinaryOperations", () => {
  it("should parse addition: 1 + 2", () => {
    /** `1 + 2` should produce a tree with both numbers and the operator. */
    const ast = parseSource("1 + 2");
    expect(ast.ruleName).toBe("program");

    // The tree should contain NUMBER:1, PLUS:+, NUMBER:2
    expect(findTokenInTree(ast, "NUMBER", "1")).toBe(true);
    expect(findTokenInTree(ast, "PLUS", "+")).toBe(true);
    expect(findTokenInTree(ast, "NUMBER", "2")).toBe(true);
  });

  it("should parse subtraction: 5 - 3", () => {
    /** `5 - 3` should parse correctly. */
    const ast = parseSource("5 - 3");
    expect(findTokenInTree(ast, "NUMBER", "5")).toBe(true);
    expect(findTokenInTree(ast, "MINUS", "-")).toBe(true);
    expect(findTokenInTree(ast, "NUMBER", "3")).toBe(true);
  });

  it("should parse multiplication: 4 * 5", () => {
    /** `4 * 5` should parse correctly. */
    const ast = parseSource("4 * 5");
    expect(findTokenInTree(ast, "NUMBER", "4")).toBe(true);
    expect(findTokenInTree(ast, "STAR", "*")).toBe(true);
    expect(findTokenInTree(ast, "NUMBER", "5")).toBe(true);
  });

  it("should parse division: 10 / 2", () => {
    /** `10 / 2` should parse correctly. */
    const ast = parseSource("10 / 2");
    expect(findTokenInTree(ast, "NUMBER", "10")).toBe(true);
    expect(findTokenInTree(ast, "SLASH", "/")).toBe(true);
    expect(findTokenInTree(ast, "NUMBER", "2")).toBe(true);
  });
});

// =============================================================================
// TEST: OPERATOR PRECEDENCE
// =============================================================================
//
// The grammar encodes precedence through rule nesting:
//   expression = term { (PLUS | MINUS) term }     — lowest precedence
//   term       = factor { (STAR | SLASH) factor }  — higher precedence
//   factor     = NUMBER | STRING | NAME | ...      — highest precedence
//
// This means multiplication/division are parsed INSIDE a term, which is
// then used as an operand of addition/subtraction. The tree structure
// naturally reflects this: * and / end up deeper than + and -.
// =============================================================================

describe("TestPrecedence", () => {
  it("should bind multiplication tighter than addition: 1 + 2 * 3", () => {
    /**
     * `1 + 2 * 3` should group multiplication tighter.
     *
     * Expected tree structure:
     *     expression
     *     +-- term (containing just "1")
     *     +-- PLUS
     *     +-- term
     *         +-- factor (containing "2")
     *         +-- STAR
     *         +-- factor (containing "3")
     *
     * The key insight: "2 * 3" is inside a single "term" node, while
     * "1" is in a separate "term" node. This means * binds tighter.
     */
    const ast = parseSource("1 + 2 * 3");

    // Navigate: program -> statement -> expression_stmt -> expression
    const expression = findRule(ast, "expression");
    expect(expression).not.toBeNull();

    // The expression should have children:
    // [term("1"), PLUS, term("2 * 3")]
    // The first term contains just "1", the second term contains "2 * 3".
    const terms = expression!.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "term",
    );
    expect(terms).toHaveLength(2);

    // First term should contain just the number 1
    expect(findTokenInTree(terms[0], "NUMBER", "1")).toBe(true);
    expect(findTokenInTree(terms[0], "STAR", "*")).toBe(false);

    // Second term should contain 2 * 3
    expect(findTokenInTree(terms[1], "NUMBER", "2")).toBe(true);
    expect(findTokenInTree(terms[1], "STAR", "*")).toBe(true);
    expect(findTokenInTree(terms[1], "NUMBER", "3")).toBe(true);
  });

  it("should bind division tighter than subtraction: 10 - 6 / 2", () => {
    /** `10 - 6 / 2` should group division tighter. */
    const ast = parseSource("10 - 6 / 2");
    const expression = findRule(ast, "expression");
    expect(expression).not.toBeNull();

    const terms = expression!.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "term",
    );
    expect(terms).toHaveLength(2);

    // First term: just 10
    expect(findTokenInTree(terms[0], "NUMBER", "10")).toBe(true);
    // Second term: 6 / 2
    expect(findTokenInTree(terms[1], "NUMBER", "6")).toBe(true);
    expect(findTokenInTree(terms[1], "SLASH", "/")).toBe(true);
    expect(findTokenInTree(terms[1], "NUMBER", "2")).toBe(true);
  });
});

// =============================================================================
// TEST: PARENTHESIZED EXPRESSIONS
// =============================================================================

describe("TestParentheses", () => {
  it("should override precedence with parens: (1 + 2) * 3", () => {
    /**
     * `(1 + 2) * 3` — parens should force addition first.
     *
     * Without parentheses, ``1 + 2 * 3`` groups as ``1 + (2 * 3)``.
     * With parentheses, ``(1 + 2)`` becomes a single factor that's
     * then multiplied by 3. In the tree, the addition should appear
     * INSIDE a factor node (deeper than the multiplication).
     */
    const ast = parseSource("(1 + 2) * 3");

    // The top-level expression should be a single term containing *.
    // That term's first factor contains the parenthesized (1 + 2).
    const expression = findRule(ast, "expression");
    expect(expression).not.toBeNull();

    // There should be a STAR somewhere in the expression's direct term
    const term = findRule(expression!, "term");
    expect(term).not.toBeNull();
    expect(findTokenInTree(term!, "STAR", "*")).toBe(true);

    // The LPAREN/RPAREN should appear in the factor
    expect(findTokenInTree(ast, "LPAREN", "(")).toBe(true);
    expect(findTokenInTree(ast, "RPAREN", ")")).toBe(true);
  });

  it("should handle nested parentheses: ((42))", () => {
    /** `((42))` should parse without error. */
    const ast = parseSource("((42))");
    expect(findTokenInTree(ast, "NUMBER", "42")).toBe(true);
  });
});

// =============================================================================
// TEST: ASSIGNMENT STATEMENTS
// =============================================================================

describe("TestAssignment (grammar-driven)", () => {
  it("should parse simple assignment: x = 42", () => {
    /** `x = 42` should produce an assignment node. */
    const ast = parseSource("x = 42\n");
    expect(ast.ruleName).toBe("program");

    // Should contain an assignment rule node
    const assignment = findRule(ast, "assignment");
    expect(assignment).not.toBeNull();
    expect(findTokenInTree(assignment!, "NAME", "x")).toBe(true);
    expect(findTokenInTree(assignment!, "EQUALS", "=")).toBe(true);
    expect(findTokenInTree(assignment!, "NUMBER", "42")).toBe(true);
  });

  it("should parse assignment with expression: x = 1 + 2", () => {
    /** `x = 1 + 2` should have a binary expression on the right side. */
    const ast = parseSource("x = 1 + 2\n");
    const assignment = findRule(ast, "assignment");
    expect(assignment).not.toBeNull();
    expect(findTokenInTree(assignment!, "NAME", "x")).toBe(true);
    expect(findTokenInTree(assignment!, "PLUS", "+")).toBe(true);
    expect(findTokenInTree(assignment!, "NUMBER", "1")).toBe(true);
    expect(findTokenInTree(assignment!, "NUMBER", "2")).toBe(true);
  });

  it("should preserve precedence in assignment: result = 1 + 2 * 3", () => {
    /** `result = 1 + 2 * 3` — precedence in the value expression. */
    const ast = parseSource("result = 1 + 2 * 3\n");
    const assignment = findRule(ast, "assignment");
    expect(assignment).not.toBeNull();

    // Find the expression inside the assignment
    const expression = findRule(assignment!, "expression");
    expect(expression).not.toBeNull();
    const terms = expression!.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "term",
    );
    expect(terms).toHaveLength(2);
  });
});

// =============================================================================
// TEST: MULTIPLE STATEMENTS
// =============================================================================

describe("TestMultipleStatements (grammar-driven)", () => {
  it("should parse two assignments", () => {
    /** `x = 1\ny = 2\n` should produce two statement nodes. */
    const ast = parseSource("x = 1\ny = 2\n");
    expect(ast.ruleName).toBe("program");

    // Count the statement children
    const statements = ast.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "statement",
    );
    expect(statements).toHaveLength(2);
  });

  it("should parse assignment then expression", () => {
    /** `x = 1\nx + 2\n` — assignment then expression statement. */
    const ast = parseSource("x = 1\nx + 2\n");
    const statements = ast.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "statement",
    );
    expect(statements).toHaveLength(2);

    // First statement should be an assignment
    expect(findRule(statements[0], "assignment")).not.toBeNull();
    // Second should be an expression_stmt
    expect(findRule(statements[1], "expression_stmt")).not.toBeNull();
  });

  it("should parse three statements", () => {
    /** Three-statement program. */
    const source = "a = 10\nb = 20\na + b\n";
    const ast = parseSource(source);
    const statements = ast.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "statement",
    );
    expect(statements).toHaveLength(3);
  });
});

// =============================================================================
// TEST: EMPTY PROGRAM
// =============================================================================

describe("TestEmptyProgram (grammar-driven)", () => {
  it("should parse an empty program", () => {
    /** An empty program should produce a program node with no statements. */
    const ast = parseSource("");
    expect(ast.ruleName).toBe("program");
    // The program's children should be empty (no statements).
    const statements = ast.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "statement",
    );
    expect(statements).toHaveLength(0);
  });

  it("should parse a program with only newlines", () => {
    /** A program with only newlines should be empty. */
    const ast = parseSource("\n\n\n");
    expect(ast.ruleName).toBe("program");
    const statements = ast.children.filter(
      (c): c is ASTNode => isASTNode(c) && c.ruleName === "statement",
    );
    expect(statements).toHaveLength(0);
  });
});

// =============================================================================
// TEST: ERROR HANDLING
// =============================================================================

describe("TestErrors (grammar-driven)", () => {
  it("should error on unexpected token", () => {
    /** A stray operator should raise GrammarParseError. */
    expect(() => parseSource(")")).toThrow(GrammarParseError);
    expect(() => parseSource(")")).toThrow(/Unexpected token/);
  });

  it("should include position in error", () => {
    /** GrammarParseError should include the problematic token. */
    try {
      parseSource(")");
      expect.unreachable("Should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(GrammarParseError);
      const error = e as GrammarParseError;
      expect(error.token).not.toBeNull();
    }
  });

  it("should error on empty grammar", () => {
    /** A grammar with no rules should raise an error. */
    const emptyGrammar: ParserGrammar = { rules: [] };
    const tokens: Token[] = [{ type: "EOF", value: "", line: 1, column: 1 }];
    const parser = new GrammarParser(tokens, emptyGrammar);

    expect(() => parser.parse()).toThrow(GrammarParseError);
    expect(() => parser.parse()).toThrow(/no rules/);
  });

  it("should error on undefined rule", () => {
    /** Referencing an undefined rule should raise an error. */
    const badGrammar: ParserGrammar = {
      rules: [
        {
          name: "start",
          body: { type: "rule_reference", name: "nonexistent" },
          lineNumber: 1,
        },
      ],
    };
    const tokens: Token[] = [
      { type: "NUMBER", value: "42", line: 1, column: 1 },
    ];
    const parser = new GrammarParser(tokens, badGrammar);

    expect(() => parser.parse()).toThrow(GrammarParseError);
  });
});

// =============================================================================
// TEST: TREE WALKING / VALUE EXTRACTION
// =============================================================================
//
// The grammar-driven parser produces generic ASTNode trees. These tests
// verify that the trees can be walked to extract meaningful values — which
// is what a bytecode compiler or interpreter would need to do.
// =============================================================================

describe("TestTreeWalking", () => {
  it("should extract number value from tree: 42", () => {
    /** Walk the tree to extract the numeric value from `42`. */
    const ast = parseSource("42");
    const numbers = collectTokens(ast, "NUMBER");
    expect(numbers).toHaveLength(1);
    expect(numbers[0].value).toBe("42");
  });

  it("should extract all tokens from expression: 1 + 2 * 3", () => {
    /** Walk `1 + 2 * 3` to find all tokens in order. */
    const ast = parseSource("1 + 2 * 3");
    const allTokens = collectAllTokens(ast);

    // Filter out NEWLINE and EOF
    const significant = allTokens.filter(
      (t) => t.type !== "NEWLINE" && t.type !== "EOF",
    );
    const values = significant.map((t) => t.value);
    expect(values).toEqual(["1", "+", "2", "*", "3"]);
  });

  it("should extract assignment parts from tree: x = 1 + 2", () => {
    /** Walk `x = 1 + 2` to extract the name and value tokens. */
    const ast = parseSource("x = 1 + 2\n");

    const assignment = findRule(ast, "assignment");
    expect(assignment).not.toBeNull();

    const names = collectTokens(assignment!, "NAME");
    expect(names).toHaveLength(1);
    expect(names[0].value).toBe("x");

    const numbers = collectTokens(assignment!, "NUMBER");
    expect(numbers).toHaveLength(2);
    expect(numbers.map((n) => n.value)).toEqual(["1", "2"]);
  });

  it("should correctly identify leaf nodes", () => {
    /** isLeafNode should correctly identify leaf nodes. */
    const token: Token = { type: "NUMBER", value: "42", line: 1, column: 1 };
    const leaf: ASTNode = { ruleName: "factor", children: [token] };
    expect(isLeafNode(leaf)).toBe(true);
    expect(getLeafToken(leaf)).toEqual(token);

    const nonLeaf: ASTNode = { ruleName: "term", children: [leaf] };
    expect(isLeafNode(nonLeaf)).toBe(false);
    expect(getLeafToken(nonLeaf)).toBeNull();
  });

  it("should return null for non-leaf token property", () => {
    /** getLeafToken should return null for non-leaf nodes. */
    const inner: ASTNode = {
      ruleName: "factor",
      children: [{ type: "NUMBER", value: "1", line: 1, column: 1 }],
    };
    const outer: ASTNode = { ruleName: "term", children: [inner, inner] };
    expect(getLeafToken(outer)).toBeNull();
  });

  it("should produce a complete dict representation with astToDict", () => {
    /** astToDict should produce a complete dict representation. */
    const ast = parseSource("42");
    const d = astToDict(ast) as Record<string, unknown>;

    // Should be an object with "rule" and "children" keys
    expect(typeof d).toBe("object");
    expect(d["rule"]).toBe("program");
    expect(Array.isArray(d["children"])).toBe(true);
  });
});

// =============================================================================
// TEST: GrammarParseError CLASS
// =============================================================================

describe("TestGrammarParseError", () => {
  it("should include position info when given a token", () => {
    /** GrammarParseError with a token should include position info. */
    const token: Token = { type: "PLUS", value: "+", line: 3, column: 7 };
    const error = new GrammarParseError("bad syntax", token);

    expect(error.token).toEqual(token);
    expect(String(error)).toContain("3:7");
    expect(String(error)).toContain("bad syntax");
  });

  it("should work without a token", () => {
    /** GrammarParseError without a token should still work. */
    const error = new GrammarParseError("no rules");

    expect(error.token).toBeNull();
    expect(String(error)).toContain("no rules");
    expect(String(error)).toContain("Parse error");
  });
});

// =============================================================================
// TEST: PACKRAT MEMOIZATION
// =============================================================================

describe("TestPackratMemoization", () => {
  it("should produce identical results on repeated parsing", () => {
    const grammarSource = `
program = { statement } ;
statement = assignment | expression_stmt ;
assignment = NAME EQUALS expression ;
expression_stmt = expression ;
expression = NUMBER ;
`;
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "NUMBER", value: "42", line: 1, column: 1 },
      { type: "EOF", value: "", line: 1, column: 3 },
    ];
    const parser1 = new GrammarParser(tokens, pg);
    const ast1 = parser1.parse();
    const parser2 = new GrammarParser(tokens, pg);
    const ast2 = parser2.parse();
    expect(ast1.ruleName).toBe(ast2.ruleName);
  });
});

// =============================================================================
// TEST: STRING-BASED TOKEN TYPES
// =============================================================================

describe("TestStringTokenTypes", () => {
  it("should match string-based token types", () => {
    const grammarSource = "expr = INT ;";
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "INT", value: "42", line: 1, column: 1 },
      { type: "EOF", value: "", line: 1, column: 3 },
    ];
    const parser = new GrammarParser(tokens, pg);
    const ast = parser.parse();
    expect(ast.ruleName).toBe("expr");
  });
});

// =============================================================================
// TEST: SIGNIFICANT NEWLINES
// =============================================================================

describe("TestSignificantNewlines", () => {
  it("should detect newlines as significant when grammar uses NEWLINE", () => {
    const grammarSource = "file = { NAME NEWLINE } ;";
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "NAME", value: "x", line: 1, column: 1 },
      { type: "NEWLINE", value: "\\n", line: 1, column: 2 },
      { type: "EOF", value: "", line: 2, column: 1 },
    ];
    const parser = new GrammarParser(tokens, pg);
    expect(parser.isNewlinesSignificant()).toBe(true);
    const ast = parser.parse();
    expect(ast.ruleName).toBe("file");
  });

  it("should treat newlines as insignificant when not referenced", () => {
    const grammarSource = "expr = NUMBER ;";
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "NEWLINE", value: "\\n", line: 1, column: 1 },
      { type: "NUMBER", value: "42", line: 2, column: 1 },
      { type: "EOF", value: "", line: 2, column: 3 },
    ];
    const parser = new GrammarParser(tokens, pg);
    expect(parser.isNewlinesSignificant()).toBe(false);
    const ast = parser.parse();
    expect(ast.ruleName).toBe("expr");
  });
});

// =============================================================================
// TEST: FURTHEST FAILURE TRACKING
// =============================================================================

describe("TestFurthestFailure", () => {
  it("should include expected tokens in error message", () => {
    const grammarSource = "expr = NUMBER PLUS NUMBER ;";
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "NUMBER", value: "1", line: 1, column: 1 },
      { type: "MINUS", value: "-", line: 1, column: 3 },
      { type: "NUMBER", value: "2", line: 1, column: 5 },
      { type: "EOF", value: "", line: 1, column: 6 },
    ];
    const parser = new GrammarParser(tokens, pg);
    try {
      parser.parse();
      expect.unreachable("Should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(GrammarParseError);
      const error = e as GrammarParseError;
      expect(error.message).toContain("Expected");
    }
  });
});

// =============================================================================
// TEST: STARLARK PIPELINE
// =============================================================================

describe("TestStarlarkPipeline", () => {
  it("should parse a simple assignment through full pipeline", () => {
    const grammarSource = `
program = { statement } ;
statement = assignment NEWLINE | expression_stmt NEWLINE ;
assignment = NAME EQUALS expression ;
expression_stmt = expression ;
expression = atom { PLUS atom } ;
atom = NUMBER | NAME ;
`;
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "NAME", value: "x", line: 1, column: 1 },
      { type: "EQUALS", value: "=", line: 1, column: 3 },
      { type: "NUMBER", value: "42", line: 1, column: 5 },
      { type: "NEWLINE", value: "\\n", line: 1, column: 7 },
      { type: "EOF", value: "", line: 2, column: 1 },
    ];
    const parser = new GrammarParser(tokens, pg);
    const ast = parser.parse();
    expect(ast.ruleName).toBe("program");
    expect(findTokenInTree(ast, "NAME", "x")).toBe(true);
    expect(findTokenInTree(ast, "NUMBER", "42")).toBe(true);
  });

  it("should parse expression with addition", () => {
    const grammarSource = `
program = { statement } ;
statement = expression_stmt NEWLINE ;
expression_stmt = expression ;
expression = atom { PLUS atom } ;
atom = NUMBER | NAME ;
`;
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "NUMBER", value: "1", line: 1, column: 1 },
      { type: "PLUS", value: "+", line: 1, column: 3 },
      { type: "NUMBER", value: "2", line: 1, column: 5 },
      { type: "NEWLINE", value: "\\n", line: 1, column: 6 },
      { type: "EOF", value: "", line: 2, column: 1 },
    ];
    const parser = new GrammarParser(tokens, pg);
    const ast = parser.parse();
    expect(ast.ruleName).toBe("program");
    expect(findTokenInTree(ast, "NUMBER", "1")).toBe(true);
    expect(findTokenInTree(ast, "PLUS", "+")).toBe(true);
    expect(findTokenInTree(ast, "NUMBER", "2")).toBe(true);
  });

  it("should parse function-like structure with indentation tokens", () => {
    const grammarSource = `
file = { definition NEWLINE } ;
definition = NAME COLON NEWLINE INDENT { NAME NEWLINE } DEDENT ;
`;
    const pg = parseParserGrammar(grammarSource);
    const tokens: Token[] = [
      { type: "NAME", value: "block", line: 1, column: 1 },
      { type: "COLON", value: ":", line: 1, column: 6 },
      { type: "NEWLINE", value: "\\n", line: 1, column: 7 },
      { type: "INDENT", value: "", line: 2, column: 1 },
      { type: "NAME", value: "x", line: 2, column: 5 },
      { type: "NEWLINE", value: "\\n", line: 2, column: 6 },
      { type: "DEDENT", value: "", line: 3, column: 1 },
      { type: "NEWLINE", value: "\\n", line: 3, column: 1 },
      { type: "EOF", value: "", line: 4, column: 1 },
    ];
    const parser = new GrammarParser(tokens, pg);
    expect(parser.isNewlinesSignificant()).toBe(true);
    const ast = parser.parse();
    expect(ast.ruleName).toBe("file");
    expect(findTokenInTree(ast, "NAME", "block")).toBe(true);
    expect(findTokenInTree(ast, "NAME", "x")).toBe(true);
  });
});

// =============================================================================
// TRACE MODE — options.trace writes [TRACE] lines to process.stderr
// =============================================================================
//
// When GrammarParser is constructed with ``{ trace: true }``, it emits a
// ``[TRACE]`` line to ``process.stderr`` for each rule attempt, reporting:
//   - the rule name
//   - the current token index
//   - the current token type and value
//   - whether the attempt matched or failed
//
// We capture stderr by replacing ``process.stderr.write`` with a spy so that
// the trace output is collected in memory rather than printed to the terminal.
// =============================================================================

/**
 * Helper that captures all output written to ``process.stderr``.
 *
 * Returns a function that, when called, restores the original ``write``
 * method and returns the accumulated output as a single string.
 */
function captureStderr(): () => string {
  const chunks: string[] = [];
  const original = process.stderr.write.bind(process.stderr);
  process.stderr.write = (chunk: string | Uint8Array): boolean => {
    chunks.push(chunk.toString());
    return true;
  };
  return () => {
    process.stderr.write = original;
    return chunks.join("");
  };
}

describe("GrammarParserTraceMode", () => {
  it("should produce a correct parse result when trace is enabled", () => {
    /**
     * Trace mode must not affect the correctness of the parse — it only adds
     * side-effect output. The AST structure should be identical to parsing
     * without trace.
     */
    const restore = captureStderr();
    const ast = new GrammarParser(tokenize("42"), grammar, { trace: true }).parse();
    restore();

    expect(ast.ruleName).toBe("program");
    expect(findTokenInTree(ast, "NUMBER", "42")).toBe(true);
  });

  it("should write [TRACE] lines to stderr", () => {
    /**
     * When trace is enabled, at least one ``[TRACE]`` line must appear on
     * stderr. We don't assert the exact count because the grammar may evolve.
     */
    const restore = captureStderr();
    new GrammarParser(tokenize("1 + 2"), grammar, { trace: true }).parse();
    const output = restore();

    expect(output).toContain("[TRACE]");
  });

  it("should format trace lines with rule name, token index, type, and value", () => {
    /**
     * Every [TRACE] line must follow the format:
     *   [TRACE] rule '<name>' at token <N> (<TYPE> "<value>") → match|fail
     *
     * We check that at least one line matches the expected pattern.
     */
    const restore = captureStderr();
    new GrammarParser(tokenize("42"), grammar, { trace: true }).parse();
    const output = restore();

    // Match the canonical format, e.g.:
    // [TRACE] rule 'program' at token 0 (NUMBER "42") → match
    const traceLinePattern = /\[TRACE\] rule '[a-z_]+' at token \d+ \([A-Z_]+ ".*?"\) → (match|fail)/;
    const lines = output.split("\n").filter((l) => l.startsWith("[TRACE]"));
    expect(lines.length).toBeGreaterThan(0);
    for (const line of lines) {
      expect(line).toMatch(traceLinePattern);
    }
  });

  it("should report both match and fail outcomes", () => {
    /**
     * A non-trivial input will cause some rule attempts to fail (backtracking)
     * and others to succeed. Both outcomes must appear in the trace.
     */
    const restore = captureStderr();
    // "1 + 2" exercises alternation — some rules will fail before the right
    // one is tried.
    new GrammarParser(tokenize("1 + 2"), grammar, { trace: true }).parse();
    const output = restore();

    expect(output).toContain("→ match");
    expect(output).toContain("→ fail");
  });

  it("should not write anything to stderr when trace is disabled", () => {
    /**
     * The default (no options) must produce no trace output. This ensures that
     * production parsing does not accidentally emit debug output.
     */
    const restore = captureStderr();
    new GrammarParser(tokenize("42"), grammar).parse();
    const output = restore();

    expect(output).not.toContain("[TRACE]");
  });

  it("should not write anything to stderr when trace is explicitly false", () => {
    const restore = captureStderr();
    new GrammarParser(tokenize("x = 1"), grammar, { trace: false }).parse();
    const output = restore();

    expect(output).not.toContain("[TRACE]");
  });

  it("should include the current token in each trace line", () => {
    /**
     * The token at the current position when the rule is attempted must
     * appear in the trace line. For input "42", the first rule attempt
     * should show the NUMBER token.
     */
    const restore = captureStderr();
    new GrammarParser(tokenize("42"), grammar, { trace: true }).parse();
    const output = restore();

    // The NUMBER token with value "42" must appear somewhere in the trace.
    expect(output).toContain('NUMBER "42"');
  });

  it("should produce correct result for assignment with trace enabled", () => {
    /**
     * Trace mode should work for a multi-token input that exercises
     * assignment parsing (two tokens: NAME = ... ; multi-rule grammar).
     */
    const restore = captureStderr();
    const ast = new GrammarParser(tokenize("x = 42"), grammar, { trace: true }).parse();
    restore();

    expect(ast.ruleName).toBe("program");
    expect(findTokenInTree(ast, "NAME", "x")).toBe(true);
    expect(findTokenInTree(ast, "NUMBER", "42")).toBe(true);
  });
});

// =============================================================================
// AST UTILITIES — walkAST, findNodes, collectTokens
// =============================================================================

describe("walkAST", () => {
  it("should visit all nodes with enter callback", () => {
    const ast = new GrammarParser(tokenize("x = 1 + 2"), grammar).parse();
    const visited: string[] = [];
    walkAST(ast, {
      enter(node) {
        visited.push(node.ruleName);
      },
    });
    expect(visited.length).toBeGreaterThan(0);
    expect(visited[0]).toBe("program");
  });

  it("should visit all nodes with leave callback", () => {
    const ast = new GrammarParser(tokenize("42"), grammar).parse();
    const visited: string[] = [];
    walkAST(ast, {
      leave(node) {
        visited.push(node.ruleName);
      },
    });
    expect(visited.length).toBeGreaterThan(0);
    // Leave visits in post-order, so program is last
    expect(visited[visited.length - 1]).toBe("program");
  });

  it("should allow replacing nodes in enter", () => {
    const token: Token = { type: "NUMBER", value: "42", line: 1, column: 1 };
    const inner: ASTNode = { ruleName: "factor", children: [token] };
    const ast: ASTNode = { ruleName: "program", children: [inner] };
    const modified = walkAST(ast, {
      enter(node) {
        if (node.ruleName === "factor") {
          return { ...node, ruleName: "replaced_factor" };
        }
      },
    });
    const hasReplaced = JSON.stringify(modified).includes("replaced_factor");
    expect(hasReplaced).toBe(true);
  });

  it("should allow replacing nodes in leave", () => {
    const token: Token = { type: "NUMBER", value: "42", line: 1, column: 1 };
    const inner: ASTNode = { ruleName: "factor", children: [token] };
    const ast: ASTNode = { ruleName: "program", children: [inner] };
    const modified = walkAST(ast, {
      leave(node) {
        if (node.ruleName === "factor") {
          return { ...node, ruleName: "replaced_factor" };
        }
      },
    });
    const hasReplaced = JSON.stringify(modified).includes("replaced_factor");
    expect(hasReplaced).toBe(true);
  });
});

describe("findNodes", () => {
  it("should find nodes by rule name", () => {
    const ast = new GrammarParser(tokenize("1 + 2"), grammar).parse();
    const factors = findNodes(ast, "factor");
    expect(factors.length).toBe(2);
  });

  it("should return empty array when no nodes match", () => {
    const ast = new GrammarParser(tokenize("42"), grammar).parse();
    const nodes = findNodes(ast, "nonexistent_rule");
    expect(nodes).toEqual([]);
  });
});

describe("collectTokens", () => {
  it("should collect all tokens when given a type filter", () => {
    const token1: Token = { type: "NUMBER", value: "1", line: 1, column: 1 };
    const token2: Token = { type: "PLUS", value: "+", line: 1, column: 3 };
    const token3: Token = { type: "NUMBER", value: "2", line: 1, column: 5 };
    const inner1: ASTNode = { ruleName: "factor", children: [token1] };
    const inner2: ASTNode = { ruleName: "factor", children: [token3] };
    const ast: ASTNode = { ruleName: "expr", children: [inner1, token2, inner2] };

    // Filter by NUMBER type
    const numbers = collectTokens(ast, "NUMBER");
    expect(numbers.length).toBe(2);
    expect(numbers[0].value).toBe("1");
    expect(numbers[1].value).toBe("2");

    // Filter by PLUS type
    const plusTokens = collectTokens(ast, "PLUS");
    expect(plusTokens.length).toBe(1);
    expect(plusTokens[0].value).toBe("+");
  });

  it("should return empty array when no tokens of given type", () => {
    const token: Token = { type: "NUMBER", value: "42", line: 1, column: 1 };
    const ast: ASTNode = { ruleName: "factor", children: [token] };
    const strings = collectTokens(ast, "STRING");
    expect(strings).toEqual([]);
  });
});

// =============================================================================
// PRE-PARSE AND POST-PARSE HOOKS
// =============================================================================

describe("pre-parse and post-parse hooks", () => {
  it("should apply pre-parse hooks before parsing", () => {
    const parser = new GrammarParser(tokenize("42"), grammar);
    let hookCalled = false;
    parser.addPreParse((tokens) => {
      hookCalled = true;
      return tokens;
    });
    parser.parse();
    expect(hookCalled).toBe(true);
  });

  it("should apply post-parse hooks after parsing", () => {
    const parser = new GrammarParser(tokenize("42"), grammar);
    let hookCalled = false;
    parser.addPostParse((ast) => {
      hookCalled = true;
      return ast;
    });
    parser.parse();
    expect(hookCalled).toBe(true);
  });

  it("post-parse hook can transform the AST", () => {
    const parser = new GrammarParser(tokenize("42"), grammar);
    parser.addPostParse((ast) => {
      return { ...ast, ruleName: "transformed_program" };
    });
    const result = parser.parse();
    expect(result.ruleName).toBe("transformed_program");
  });
});

describe("rich source preservation", () => {
  it("propagates token-derived offsets, indices, and trivia onto AST nodes", () => {
    const tokenGrammar = makeRichSourceTokenGrammar();
    const parserGrammar = makeRichSourceParserGrammar();
    const tokens = grammarTokenize(
      "  // lead\nfoo=bar",
      tokenGrammar,
      { preserveSourceInfo: true },
    );

    const parser = new GrammarParser(tokens, parserGrammar, {
      preserveSourceInfo: true,
    });
    const ast = parser.parse();
    const assignment = findNodes(ast, "assignment")[0];

    expect(ast.startOffset).toBe(10);
    expect(ast.endOffset).toBe(17);
    expect(ast.firstTokenIndex).toBe(0);
    expect(ast.lastTokenIndex).toBe(2);
    expect(ast.leadingTrivia?.map((item) => item.type)).toEqual([
      "WHITESPACE",
      "LINE_COMMENT",
      "WHITESPACE",
    ]);

    expect(assignment.startOffset).toBe(10);
    expect(assignment.endOffset).toBe(17);
    expect(assignment.firstTokenIndex).toBe(0);
    expect(assignment.lastTokenIndex).toBe(2);
  });

  it("keeps AST nodes lean when preserveSourceInfo is disabled", () => {
    const tokenGrammar = makeRichSourceTokenGrammar();
    const parserGrammar = makeRichSourceParserGrammar();
    const tokens = grammarTokenize(
      "foo=bar",
      tokenGrammar,
      { preserveSourceInfo: true },
    );

    const parser = new GrammarParser(tokens, parserGrammar);
    const ast = parser.parse();

    expect(ast.startOffset).toBeUndefined();
    expect(ast.firstTokenIndex).toBeUndefined();
    expect(ast.leadingTrivia).toBeUndefined();
  });
});
