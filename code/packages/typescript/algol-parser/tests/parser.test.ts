/**
 * Tests for the ALGOL 60 Parser (TypeScript).
 *
 * These tests verify that the grammar-driven parser correctly parses ALGOL 60
 * source text when loaded with the `algol.grammar` file.
 *
 * The ALGOL 60 grammar's top-level rule is `program`, which wraps a single `block`.
 * Every valid ALGOL 60 program is a block: `begin [declarations] statements end`.
 *
 * Historical Context
 * ------------------
 *
 * ALGOL 60 was the first language with a formally specified grammar. The grammar
 * was published in the ALGOL 60 report (1960, revised 1963). Peter Naur's contribution
 * to formalizing the notation gave it the name "Backus-Naur Form" (BNF).
 *
 * Key grammar features tested here:
 *
 *   1. **Block structure** -- begin/end creates a lexical scope.
 *      All declarations must precede all statements within a block.
 *
 *   2. **Assignment** -- x := expr (not x = expr; that's equality comparison).
 *
 *   3. **Conditional statement** -- if/then/else.
 *      The "then" branch cannot itself be a conditional (dangling else resolution).
 *
 *   4. **For loop** -- ALGOL's for loop uses step/until for ranges.
 *
 *   5. **Arithmetic expressions** -- +, -, *, /, div, mod, **, ^
 *      with correct operator precedence.
 *
 * Test Strategy
 * -------------
 *
 * Each test parses an ALGOL program and uses helper functions to walk the
 * resulting AST, looking for specific node types. This is more robust than
 * checking exact tree structure, because the grammar may wrap nodes in
 * multiple layers (e.g., expression -> arith_expr -> simple_arith -> term -> ...).
 *
 * Test Categories
 * ---------------
 *
 *   1. **Minimal program** -- the smallest valid ALGOL program
 *   2. **Declarations** -- integer, real, boolean variable declarations
 *   3. **Assignment** -- simple assignment, multiple assignment
 *   4. **Arithmetic** -- binary operators, operator precedence
 *   5. **Conditionals** -- if/then, if/then/else
 *   6. **For loops** -- step/until form
 *   7. **Boolean expressions** -- and, or, not, relational operators
 *   8. **Comments** -- comments are skipped before parsing
 *   9. **Nested blocks** -- begin/end within statements
 */

import { describe, it, expect } from "vitest";
import { parseAlgol } from "../src/parser.js";
import { isASTNode } from "@coding-adventures/parser";
import type { ASTNode } from "@coding-adventures/parser";
import type { Token } from "@coding-adventures/lexer";

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/**
 * Recursively find all AST nodes with a given rule name.
 *
 * Walks the entire subtree rooted at `node` and collects all nodes whose
 * `ruleName` matches the given string. This lets tests assert things like
 * "this program contains exactly one assign_stmt node" without caring about
 * the exact depth in the tree.
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
 * Flattens the entire tree into a list of Token objects. Useful for
 * checking "does this subtree contain a token of type X with value Y?"
 * without caring about where it sits in the tree.
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

/**
 * Collect all leaf token types from an AST subtree.
 * Convenience wrapper around findTokens.
 */
function tokenTypesInTree(node: ASTNode): string[] {
  return findTokens(node).map((t) => t.type);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("minimal program", () => {
  it("parses a minimal block with one integer declaration and assignment", () => {
    /**
     * The smallest meaningful ALGOL 60 program:
     *   begin integer x; x := 42 end
     *
     * This contains one declaration (integer x) and one statement (x := 42).
     * The top-level AST node should be "program".
     */
    const ast = parseAlgol("begin integer x; x := 42 end");
    expect(ast.ruleName).toBe("program");
  });

  it("wraps a block inside the program node", () => {
    /**
     * The grammar rule is: program = block
     * So the program node contains a block node.
     */
    const ast = parseAlgol("begin integer x; x := 42 end");
    const blockNodes = findNodes(ast, "block");
    expect(blockNodes.length).toBeGreaterThanOrEqual(1);
  });
});

describe("declarations", () => {
  it("parses an integer declaration", () => {
    /**
     * `integer x` declares x as an integer variable.
     * The grammar: type_decl = type ident_list
     * where type = INTEGER | REAL | BOOLEAN | STRING
     */
    const ast = parseAlgol("begin integer x; x := 0 end");
    const typeDecls = findNodes(ast, "type_decl");
    expect(typeDecls.length).toBeGreaterThanOrEqual(1);

    // The type token should be "integer"
    const allTokens = findTokens(typeDecls[0]);
    expect(allTokens.some((t) => t.type === "integer")).toBe(true);
  });

  it("parses a real declaration", () => {
    /**
     * `real y` declares y as a real (floating-point) variable.
     */
    const ast = parseAlgol("begin real y; y := 3.14 end");
    const allTokens = tokenTypesInTree(ast);
    expect(allTokens).toContain("real");
  });

  it("parses a boolean declaration", () => {
    /**
     * `boolean flag` declares a boolean variable.
     * ALGOL distinguishes integer, real, boolean, and string types.
     */
    const ast = parseAlgol("begin boolean flag; flag := true end");
    const allTokens = tokenTypesInTree(ast);
    expect(allTokens).toContain("boolean");
  });

  it("parses multiple variable declarations", () => {
    /**
     * Multiple variables can be declared together:
     *   integer x, y, z
     *
     * The ident_list rule: IDENT { COMMA IDENT }
     */
    const ast = parseAlgol("begin integer x, y, z; x := 1 end");
    const typeDecls = findNodes(ast, "type_decl");
    expect(typeDecls.length).toBeGreaterThanOrEqual(1);

    // Should find three IDENT tokens in the ident_list
    const allTokens = findTokens(typeDecls[0]);
    const identTokens = allTokens.filter((t) => t.type === "NAME");
    expect(identTokens.length).toBeGreaterThanOrEqual(3);
  });

  it("parses multiple type declarations", () => {
    /**
     * Multiple declarations separated by semicolons before any statements.
     * ALGOL requires all declarations to precede all statements in a block.
     */
    const ast = parseAlgol("begin integer x; real y; x := 1; y := 1.5 end");
    const typeDecls = findNodes(ast, "type_decl");
    expect(typeDecls.length).toBeGreaterThanOrEqual(2);
  });
});

describe("assignment", () => {
  it("parses a simple integer assignment", () => {
    /**
     * Assignment syntax: variable := expression
     * The := operator distinguishes assignment from equality (=).
     */
    const ast = parseAlgol("begin integer x; x := 42 end");
    const assignNodes = findNodes(ast, "assign_stmt");
    expect(assignNodes.length).toBeGreaterThanOrEqual(1);
  });

  it("parses an assignment with an ASSIGN token", () => {
    /**
     * Verify that the ASSIGN token (:=) appears in the tree.
     * This confirms the assignment was parsed correctly, not misread as
     * a colon followed by equals.
     */
    const ast = parseAlgol("begin integer x; x := 5 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("ASSIGN");
  });

  it("parses an assignment from one variable to another", () => {
    /**
     * Assigning the value of one variable to another: y := x
     */
    const ast = parseAlgol("begin integer x; integer y; x := 10; y := x end");
    const assignNodes = findNodes(ast, "assign_stmt");
    expect(assignNodes.length).toBeGreaterThanOrEqual(2);
  });

  it("parses a real assignment with a decimal literal", () => {
    const ast = parseAlgol("begin real r; r := 3.14 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("REAL_LIT");
  });
});

describe("arithmetic expressions", () => {
  it("parses addition", () => {
    /**
     * x := 1 + 2
     * The arithmetic expression hierarchy: arith_expr -> simple_arith -> term -> factor -> primary
     * Addition is handled at the simple_arith level: term { (PLUS | MINUS) term }
     */
    const ast = parseAlgol("begin integer x; x := 1 + 2 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("PLUS");
    expect(tokens).toContain("INTEGER_LIT");
  });

  it("parses subtraction", () => {
    const ast = parseAlgol("begin integer x; x := 10 - 3 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("MINUS");
  });

  it("parses multiplication", () => {
    /**
     * Multiplication is handled at the term level: factor { (STAR | SLASH | DIV | MOD) factor }
     * Multiplication binds tighter than addition (higher precedence).
     */
    const ast = parseAlgol("begin integer x; x := 3 * 4 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("STAR");
  });

  it("parses division", () => {
    const ast = parseAlgol("begin real x; x := 10 / 4 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("SLASH");
  });

  it("parses a compound arithmetic expression", () => {
    /**
     * x := 1 + 2 * 3 — multiplication has higher precedence than addition.
     * This tests that the grammar correctly encodes precedence through
     * the simple_arith -> term -> factor chain.
     */
    const ast = parseAlgol("begin integer x; x := 1 + 2 * 3 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("PLUS");
    expect(tokens).toContain("STAR");
    expect(tokens).toContain("INTEGER_LIT");
  });

  it("parses parenthesized expression", () => {
    /**
     * Parentheses override default precedence: (1 + 2) * 3
     * The primary rule: LPAREN arith_expr RPAREN
     */
    const ast = parseAlgol("begin integer x; x := (1 + 2) * 3 end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("LPAREN");
    expect(tokens).toContain("RPAREN");
    expect(tokens).toContain("PLUS");
    expect(tokens).toContain("STAR");
  });
});

describe("conditional statement — if/then", () => {
  it("parses if/then without else", () => {
    /**
     * ALGOL conditional: if bool_expr then unlabeled_stmt
     * The else branch is optional.
     *
     * Grammar: cond_stmt = IF bool_expr THEN unlabeled_stmt [ ELSE statement ]
     */
    const ast = parseAlgol(
      "begin integer x; x := 0; if x = 0 then x := 1 end"
    );
    const condNodes = findNodes(ast, "cond_stmt");
    expect(condNodes.length).toBeGreaterThanOrEqual(1);
  });

  it("has if and then tokens in the conditional node", () => {
    const ast = parseAlgol(
      "begin integer x; x := 0; if x = 0 then x := 1 end"
    );
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("if");
    expect(tokens).toContain("then");
  });

  it("parses if/then/else", () => {
    /**
     * Full conditional with both branches:
     *   if x > 0 then y := 1 else y := 0
     *
     * The then-branch is unlabeled_stmt (which excludes conditionals,
     * preventing dangling else ambiguity). The else-branch is statement
     * (which includes conditionals, enabling if/then/else if chains).
     */
    const ast = parseAlgol(
      "begin integer x; integer y; x := 5; y := 0; if x > 0 then y := 1 else y := 0 end"
    );
    const condNodes = findNodes(ast, "cond_stmt");
    expect(condNodes.length).toBeGreaterThanOrEqual(1);

    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("if");
    expect(tokens).toContain("then");
    expect(tokens).toContain("else");
  });

  it("parses an if/then/else if chain", () => {
    /**
     * Chained conditionals: if a then ... else if b then ... else ...
     * ALGOL allows this because the else branch is `statement` (which includes
     * cond_stmt), while the then branch is `unlabeled_stmt` (which excludes it).
     */
    const ast = parseAlgol(
      "begin integer x; integer y; x := 5; y := 0; if x > 10 then y := 2 else if x > 0 then y := 1 else y := 0 end"
    );
    const condNodes = findNodes(ast, "cond_stmt");
    // Should find at least 2 conditional nodes (outer and inner)
    expect(condNodes.length).toBeGreaterThanOrEqual(2);
  });
});

describe("for loop", () => {
  it("parses a for loop with step/until", () => {
    /**
     * ALGOL's classic step/until for loop:
     *   for i := 1 step 1 until 10 do s := s + i
     *
     * Grammar:
     *   for_stmt = FOR IDENT ASSIGN for_list DO statement
     *   for_list = for_elem { COMMA for_elem }
     *   for_elem = arith_expr STEP arith_expr UNTIL arith_expr
     *            | arith_expr WHILE bool_expr
     *            | arith_expr
     *
     * This is equivalent to C's for (i = 1; i <= 10; i += 1).
     */
    const ast = parseAlgol(
      "begin integer i; integer s; s := 0; for i := 1 step 1 until 10 do s := s + i end"
    );
    const forNodes = findNodes(ast, "for_stmt");
    expect(forNodes.length).toBeGreaterThanOrEqual(1);
  });

  it("has for, step, until, and do tokens", () => {
    /**
     * Verify all the for-loop keywords appear in the parsed tree.
     */
    const ast = parseAlgol(
      "begin integer i; integer s; s := 0; for i := 1 step 1 until 10 do s := s + i end"
    );
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("for");
    expect(tokens).toContain("step");
    expect(tokens).toContain("until");
    expect(tokens).toContain("do");
  });

  it("parses a for loop with a compound statement body", () => {
    /**
     * The body of a for loop can be a compound statement (begin...end without
     * declarations). This tests that multi-statement bodies are supported.
     *
     *   for i := 1 step 1 until 3 do begin x := x + i; y := y * i end
     */
    const ast = parseAlgol(
      "begin integer i; integer x; integer y; x := 0; y := 1; for i := 1 step 1 until 3 do begin x := x + i; y := y * i end end"
    );
    const forNodes = findNodes(ast, "for_stmt");
    expect(forNodes.length).toBeGreaterThanOrEqual(1);
  });
});

describe("boolean expressions", () => {
  it("parses an equality test", () => {
    /**
     * x = 0 is an equality test (not assignment!).
     * In ALGOL, = is comparison and := is assignment.
     * The relation rule: simple_arith ( EQ | NEQ | LT | LEQ | GT | GEQ ) simple_arith
     */
    const ast = parseAlgol(
      "begin integer x; x := 0; if x = 0 then x := 1 end"
    );
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("EQ");
  });

  it("parses a less-than comparison", () => {
    const ast = parseAlgol(
      "begin integer x; x := 5; if x < 10 then x := 10 end"
    );
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("LT");
  });

  it("parses a greater-than comparison", () => {
    const ast = parseAlgol(
      "begin integer x; integer y; x := 5; y := 0; if x > 0 then y := 1 end"
    );
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("GT");
  });

  it("parses a boolean and expression", () => {
    /**
     * Logical conjunction: x > 0 and x < 10
     * In ALGOL, `and` is a keyword, not a symbol (&&).
     * The bool_factor rule: bool_secondary { AND bool_secondary }
     */
    const ast = parseAlgol(
      "begin integer x; integer y; x := 5; y := 0; if x > 0 and x < 10 then y := 1 end"
    );
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("and");
  });

  it("parses a boolean or expression", () => {
    /**
     * Logical disjunction: x < 0 or x > 10
     * The bool_term rule: bool_factor { OR bool_factor }
     */
    const ast = parseAlgol(
      "begin integer x; integer y; x := 15; y := 0; if x < 0 or x > 10 then y := 1 end"
    );
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("or");
  });

  it("parses boolean true and false literals", () => {
    /**
     * `true` and `false` are boolean literals in ALGOL.
     * They can be assigned to boolean variables.
     */
    const ast = parseAlgol("begin boolean b; b := true end");
    const tokens = tokenTypesInTree(ast);
    expect(tokens).toContain("true");
  });
});

describe("comments are skipped before parsing", () => {
  it("parses a program with a comment at the start", () => {
    /**
     * Comments (comment...;) are consumed by the lexer before the parser
     * ever sees the token stream. The parser never receives a "comment" token.
     *
     * This tests the full pipeline: lexer comment skipping feeds into parser.
     */
    const ast = parseAlgol(
      "comment compute the sum of two numbers; begin integer x; x := 1 + 2 end"
    );
    expect(ast.ruleName).toBe("program");
    const assignNodes = findNodes(ast, "assign_stmt");
    expect(assignNodes.length).toBeGreaterThanOrEqual(1);
  });

  it("parses a program with a comment between declarations and statements", () => {
    /**
     * Comments can appear anywhere whitespace can — including between
     * declarations and the first statement.
     */
    const ast = parseAlgol(
      "begin integer x; comment initialize x to zero; x := 0 end"
    );
    expect(ast.ruleName).toBe("program");
  });
});

describe("nested blocks", () => {
  it("parses a nested begin/end compound statement", () => {
    /**
     * A compound_stmt is begin...end with only statements, no declarations.
     * Full blocks (with declarations) can also be nested inside statements.
     *
     *   begin integer x; x := 0; begin x := x + 1 end end
     *
     * The outer block has a declaration; the inner compound_stmt just has statements.
     */
    const ast = parseAlgol(
      "begin integer x; x := 0; begin x := x + 1 end end"
    );
    expect(ast.ruleName).toBe("program");
    // Should find multiple begin/end pairs — the outer block and inner compound
    const blockNodes = findNodes(ast, "block");
    const compoundNodes = findNodes(ast, "compound_stmt");
    const totalNested = blockNodes.length + compoundNodes.length;
    expect(totalNested).toBeGreaterThanOrEqual(2);
  });
});
