/**
 * Comprehensive tests for the recursive descent parser.
 *
 * =============================================================================
 * TESTING STRATEGY
 * =============================================================================
 *
 * These tests verify that the parser correctly transforms token streams into
 * Abstract Syntax Trees. We construct Token lists manually rather than running
 * the lexer, so these tests are completely self-contained — a failure here
 * means the parser is broken, not the lexer.
 *
 * The tests are organized from simple to complex:
 *
 *     1. Single atoms (numbers, strings, names)
 *     2. Binary operations (one operator)
 *     3. Operator precedence (multiple operators)
 *     4. Parenthesized expressions (precedence override)
 *     5. Assignment statements
 *     6. Multiple statements (programs)
 *     7. Edge cases and error handling
 *
 * Each test follows the pattern:
 *     1. Build a list of Token objects (the input)
 *     2. Create a Parser and call .parse()
 *     3. Assert the resulting AST matches the expected structure
 * =============================================================================
 */

import { describe, it, expect } from "vitest";
import type { Token } from "@coding-adventures/lexer";
import {
  Parser,
  ParseError,
} from "../src/parser.js";
import type {
  Program,
  NumberLiteral,
  StringLiteral,
  Name,
  BinaryOp,
  Assignment,
  Expression,
} from "../src/parser.js";

// =============================================================================
// HELPERS
// =============================================================================
//
// Building token lists by hand is verbose, so we provide a few helper
// functions to reduce the boilerplate. These make the tests much more
// readable — you can see the *meaning* of the token stream at a glance.
// =============================================================================

/**
 * Create a Token with sensible defaults for line and column.
 *
 * In most tests we don't care about exact line/column positions — we just
 * want to verify the AST structure. This helper lets us focus on token type
 * and value.
 *
 * @param type - The type of the token (e.g., "NUMBER").
 * @param value - The text value of the token (e.g., "42").
 * @param line - Line number (defaults to 1).
 * @param column - Column number (defaults to 1).
 * @returns A Token with the specified type, value, line, and column.
 */
function tok(type: string, value: string, line = 1, column = 1): Token {
  return { type, value, line, column };
}

/**
 * Create an EOF token — the standard end-of-input marker.
 *
 * Every token list should end with EOF. This helper makes that explicit
 * and reduces clutter in the test token lists.
 */
function eof(line = 1, column = 1): Token {
  return { type: "EOF", value: "", line, column };
}

/**
 * Create a NEWLINE token — the statement terminator.
 */
function nl(line = 1, column = 1): Token {
  return { type: "NEWLINE", value: "\n", line, column };
}

/**
 * Parse a token list and return the Program AST.
 *
 * This is the most common operation in our tests: create a parser,
 * parse the tokens, return the result.
 *
 * @param tokens - A list of Token objects (should end with EOF).
 * @returns The parsed Program AST.
 */
function parseTokens(tokens: Token[]): Program {
  const parser = new Parser(tokens);
  return parser.parse();
}

// =============================================================================
// TEST: SINGLE ATOMS (FACTORS)
// =============================================================================
//
// The simplest possible inputs — a single token that forms a complete
// expression. These test the parseFactor() method in isolation.
// =============================================================================

describe("TestNumberLiteral", () => {
  it("should parse a single number to NumberLiteral(42)", () => {
    /** A bare number like `42` should parse to NumberLiteral(42). */
    const tokens = [tok("NUMBER", "42"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual({
      kind: "NumberLiteral",
      value: 42,
    } satisfies NumberLiteral);
  });

  it("should parse zero as a valid number literal", () => {
    /** Zero is a valid number literal. */
    const tokens = [tok("NUMBER", "0"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements[0]).toEqual({
      kind: "NumberLiteral",
      value: 0,
    } satisfies NumberLiteral);
  });

  it("should parse large numbers correctly", () => {
    /** Large numbers should parse correctly. */
    const tokens = [tok("NUMBER", "999999"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements[0]).toEqual({
      kind: "NumberLiteral",
      value: 999999,
    } satisfies NumberLiteral);
  });
});

describe("TestStringLiteral", () => {
  it("should parse a simple string to StringLiteral", () => {
    /** A string like `"hello"` should parse to StringLiteral("hello"). */
    const tokens = [tok("STRING", "hello"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual({
      kind: "StringLiteral",
      value: "hello",
    } satisfies StringLiteral);
  });

  it("should parse an empty string", () => {
    /** An empty string `""` should parse to StringLiteral(""). */
    const tokens = [tok("STRING", ""), eof()];
    const program = parseTokens(tokens);

    expect(program.statements[0]).toEqual({
      kind: "StringLiteral",
      value: "",
    } satisfies StringLiteral);
  });

  it("should parse strings with spaces", () => {
    /** Strings can contain spaces. */
    const tokens = [tok("STRING", "hello world"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements[0]).toEqual({
      kind: "StringLiteral",
      value: "hello world",
    } satisfies StringLiteral);
  });
});

describe("TestName", () => {
  it("should parse a simple name to Name('x')", () => {
    /** A bare name like `x` should parse to Name("x"). */
    const tokens = [tok("NAME", "x"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual({
      kind: "Name",
      name: "x",
    } satisfies Name);
  });

  it("should parse longer names", () => {
    /** Multi-character names work too. */
    const tokens = [tok("NAME", "total"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements[0]).toEqual({
      kind: "Name",
      name: "total",
    } satisfies Name);
  });

  it("should parse names with underscores", () => {
    /** Names with underscores should work. */
    const tokens = [tok("NAME", "my_var"), eof()];
    const program = parseTokens(tokens);

    expect(program.statements[0]).toEqual({
      kind: "Name",
      name: "my_var",
    } satisfies Name);
  });
});

// =============================================================================
// TEST: BINARY OPERATIONS
// =============================================================================
//
// Two operands connected by an operator. These test parseExpression()
// and parseTerm() with a single operator.
// =============================================================================

describe("TestBinaryOp", () => {
  it("should parse addition: 1 + 2", () => {
    /** `1 + 2` should parse to BinaryOp(1, "+", 2). */
    const tokens = [
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 1 },
      op: "+",
      right: { kind: "NumberLiteral", value: 2 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse subtraction: 3 - 1", () => {
    /** `3 - 1` should parse to BinaryOp(3, "-", 1). */
    const tokens = [
      tok("NUMBER", "3"),
      tok("MINUS", "-"),
      tok("NUMBER", "1"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 3 },
      op: "-",
      right: { kind: "NumberLiteral", value: 1 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse multiplication: 4 * 5", () => {
    /** `4 * 5` should parse to BinaryOp(4, "*", 5). */
    const tokens = [
      tok("NUMBER", "4"),
      tok("STAR", "*"),
      tok("NUMBER", "5"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 4 },
      op: "*",
      right: { kind: "NumberLiteral", value: 5 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse division: 10 / 2", () => {
    /** `10 / 2` should parse to BinaryOp(10, "/", 2). */
    const tokens = [
      tok("NUMBER", "10"),
      tok("SLASH", "/"),
      tok("NUMBER", "2"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 10 },
      op: "/",
      right: { kind: "NumberLiteral", value: 2 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse names in expressions: x + 1", () => {
    /** `x + 1` should parse with Name("x") on the left. */
    const tokens = [
      tok("NAME", "x"),
      tok("PLUS", "+"),
      tok("NUMBER", "1"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "Name", name: "x" },
      op: "+",
      right: { kind: "NumberLiteral", value: 1 },
    };
    expect(program.statements[0]).toEqual(expected);
  });
});

// =============================================================================
// TEST: OPERATOR PRECEDENCE
// =============================================================================
//
// These tests verify that * and / bind tighter than + and -, ensuring the
// tree structure correctly encodes precedence.
// =============================================================================

describe("TestOperatorPrecedence", () => {
  it("should parse multiplication before addition: 1 + 2 * 3", () => {
    /**
     * `1 + 2 * 3` should parse as `1 + (2 * 3)`.
     *
     * The multiplication becomes a subtree of the addition node,
     * meaning it's evaluated first — exactly what we want.
     */
    const tokens = [
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("STAR", "*"),
      tok("NUMBER", "3"),
      eof(),
    ];
    const program = parseTokens(tokens);

    // Expected: BinaryOp(1, "+", BinaryOp(2, "*", 3))
    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 1 },
      op: "+",
      right: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 2 },
        op: "*",
        right: { kind: "NumberLiteral", value: 3 },
      },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse division before subtraction: 10 - 6 / 2", () => {
    /** `10 - 6 / 2` should parse as `10 - (6 / 2)`. */
    const tokens = [
      tok("NUMBER", "10"),
      tok("MINUS", "-"),
      tok("NUMBER", "6"),
      tok("SLASH", "/"),
      tok("NUMBER", "2"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 10 },
      op: "-",
      right: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 6 },
        op: "/",
        right: { kind: "NumberLiteral", value: 2 },
      },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse left-associative addition: 1 + 2 + 3", () => {
    /** `1 + 2 + 3` should parse as `(1 + 2) + 3` (left-associative). */
    const tokens = [
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("PLUS", "+"),
      tok("NUMBER", "3"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 1 },
        op: "+",
        right: { kind: "NumberLiteral", value: 2 },
      },
      op: "+",
      right: { kind: "NumberLiteral", value: 3 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse left-associative multiplication: 2 * 3 * 4", () => {
    /** `2 * 3 * 4` should parse as `(2 * 3) * 4` (left-associative). */
    const tokens = [
      tok("NUMBER", "2"),
      tok("STAR", "*"),
      tok("NUMBER", "3"),
      tok("STAR", "*"),
      tok("NUMBER", "4"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 2 },
        op: "*",
        right: { kind: "NumberLiteral", value: 3 },
      },
      op: "*",
      right: { kind: "NumberLiteral", value: 4 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should handle complex precedence: 1 + 2 * 3 + 4", () => {
    /** `1 + 2 * 3 + 4` should parse as `(1 + (2 * 3)) + 4`. */
    const tokens = [
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("STAR", "*"),
      tok("NUMBER", "3"),
      tok("PLUS", "+"),
      tok("NUMBER", "4"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 1 },
        op: "+",
        right: {
          kind: "BinaryOp",
          left: { kind: "NumberLiteral", value: 2 },
          op: "*",
          right: { kind: "NumberLiteral", value: 3 },
        },
      },
      op: "+",
      right: { kind: "NumberLiteral", value: 4 },
    };
    expect(program.statements[0]).toEqual(expected);
  });
});

// =============================================================================
// TEST: PARENTHESIZED EXPRESSIONS
// =============================================================================
//
// Parentheses let the programmer override the default precedence.
// The parser handles this by recursively calling parseExpression()
// when it sees a `(`, then expecting a `)` afterward.
// =============================================================================

describe("TestParentheses", () => {
  it("should parse simple parentheses: (1 + 2)", () => {
    /**
     * `(1 + 2)` should parse the same as `1 + 2`.
     *
     * Parentheses around a single expression don't change the tree,
     * but they should still parse correctly.
     */
    const tokens = [
      tok("LPAREN", "("),
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("RPAREN", ")"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 1 },
      op: "+",
      right: { kind: "NumberLiteral", value: 2 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should override precedence with parens: (1 + 2) * 3", () => {
    /**
     * `(1 + 2) * 3` should parse as `(1 + 2) * 3`, NOT `1 + (2 * 3)`.
     *
     * Without parentheses, multiplication would bind tighter. The parens
     * force addition to happen first by making it a deeper subtree.
     */
    const tokens = [
      tok("LPAREN", "("),
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("RPAREN", ")"),
      tok("STAR", "*"),
      tok("NUMBER", "3"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 1 },
        op: "+",
        right: { kind: "NumberLiteral", value: 2 },
      },
      op: "*",
      right: { kind: "NumberLiteral", value: 3 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should handle nested parentheses: ((1 + 2))", () => {
    /** `((1 + 2))` should handle nested parens correctly. */
    const tokens = [
      tok("LPAREN", "("),
      tok("LPAREN", "("),
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("RPAREN", ")"),
      tok("RPAREN", ")"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 1 },
      op: "+",
      right: { kind: "NumberLiteral", value: 2 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should handle parentheses with names: (x + 1) * y", () => {
    /** `(x + 1) * y` mixes names and parentheses. */
    const tokens = [
      tok("LPAREN", "("),
      tok("NAME", "x"),
      tok("PLUS", "+"),
      tok("NUMBER", "1"),
      tok("RPAREN", ")"),
      tok("STAR", "*"),
      tok("NAME", "y"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: BinaryOp = {
      kind: "BinaryOp",
      left: {
        kind: "BinaryOp",
        left: { kind: "Name", name: "x" },
        op: "+",
        right: { kind: "NumberLiteral", value: 1 },
      },
      op: "*",
      right: { kind: "Name", name: "y" },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should unwrap parenthesized single number: (42)", () => {
    /** `(42)` wrapping a single number should unwrap to NumberLiteral. */
    const tokens = [
      tok("LPAREN", "("),
      tok("NUMBER", "42"),
      tok("RPAREN", ")"),
      eof(),
    ];
    const program = parseTokens(tokens);

    expect(program.statements[0]).toEqual({
      kind: "NumberLiteral",
      value: 42,
    } satisfies NumberLiteral);
  });
});

// =============================================================================
// TEST: ASSIGNMENT STATEMENTS
// =============================================================================
//
// Assignments bind a name to a value. They use the pattern:
//   NAME EQUALS expression NEWLINE
// =============================================================================

describe("TestAssignment", () => {
  it("should parse simple assignment: x = 42", () => {
    /** `x = 42\n` should parse to Assignment(Name("x"), NumberLiteral(42)). */
    const tokens = [
      tok("NAME", "x"),
      tok("EQUALS", "="),
      tok("NUMBER", "42"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: Assignment = {
      kind: "Assignment",
      target: { kind: "Name", name: "x" },
      value: { kind: "NumberLiteral", value: 42 },
    };
    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse assignment with expression: x = 1 + 2", () => {
    /** `x = 1 + 2\n` should parse the right side as a BinaryOp. */
    const tokens = [
      tok("NAME", "x"),
      tok("EQUALS", "="),
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: Assignment = {
      kind: "Assignment",
      target: { kind: "Name", name: "x" },
      value: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 1 },
        op: "+",
        right: { kind: "NumberLiteral", value: 2 },
      },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse assignment with complex expression: result = 1 + 2 * 3", () => {
    /** `result = 1 + 2 * 3\n` — precedence applies in the value. */
    const tokens = [
      tok("NAME", "result"),
      tok("EQUALS", "="),
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("STAR", "*"),
      tok("NUMBER", "3"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: Assignment = {
      kind: "Assignment",
      target: { kind: "Name", name: "result" },
      value: {
        kind: "BinaryOp",
        left: { kind: "NumberLiteral", value: 1 },
        op: "+",
        right: {
          kind: "BinaryOp",
          left: { kind: "NumberLiteral", value: 2 },
          op: "*",
          right: { kind: "NumberLiteral", value: 3 },
        },
      },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse assignment with string: name = 'hello'", () => {
    /** `name = "hello"\n` should assign a string literal. */
    const tokens = [
      tok("NAME", "name"),
      tok("EQUALS", "="),
      tok("STRING", "hello"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: Assignment = {
      kind: "Assignment",
      target: { kind: "Name", name: "name" },
      value: { kind: "StringLiteral", value: "hello" },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse assignment without trailing newline", () => {
    /** `x = 42` at EOF (no trailing newline) should still parse. */
    const tokens = [
      tok("NAME", "x"),
      tok("EQUALS", "="),
      tok("NUMBER", "42"),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: Assignment = {
      kind: "Assignment",
      target: { kind: "Name", name: "x" },
      value: { kind: "NumberLiteral", value: 42 },
    };
    expect(program.statements[0]).toEqual(expected);
  });

  it("should parse assignment with parenthesized value: x = (1 + 2) * 3", () => {
    /** `x = (1 + 2) * 3\n` — parentheses in the value expression. */
    const tokens = [
      tok("NAME", "x"),
      tok("EQUALS", "="),
      tok("LPAREN", "("),
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      tok("RPAREN", ")"),
      tok("STAR", "*"),
      tok("NUMBER", "3"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    const expected: Assignment = {
      kind: "Assignment",
      target: { kind: "Name", name: "x" },
      value: {
        kind: "BinaryOp",
        left: {
          kind: "BinaryOp",
          left: { kind: "NumberLiteral", value: 1 },
          op: "+",
          right: { kind: "NumberLiteral", value: 2 },
        },
        op: "*",
        right: { kind: "NumberLiteral", value: 3 },
      },
    };
    expect(program.statements[0]).toEqual(expected);
  });
});

// =============================================================================
// TEST: MULTIPLE STATEMENTS (PROGRAMS)
// =============================================================================
//
// Real programs have multiple statements. These tests verify that the
// parser correctly handles newline-separated statement sequences.
// =============================================================================

describe("TestMultipleStatements", () => {
  it("should parse two assignments", () => {
    /** `x = 1\ny = 2\n` should produce two assignment nodes. */
    const tokens = [
      tok("NAME", "x", 1),
      tok("EQUALS", "=", 1),
      tok("NUMBER", "1", 1),
      nl(1),
      tok("NAME", "y", 2),
      tok("EQUALS", "=", 2),
      tok("NUMBER", "2", 2),
      nl(2),
      eof(3),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(2);
    expect(program.statements[0]).toEqual({
      kind: "Assignment",
      target: { kind: "Name", name: "x" },
      value: { kind: "NumberLiteral", value: 1 },
    } satisfies Assignment);
    expect(program.statements[1]).toEqual({
      kind: "Assignment",
      target: { kind: "Name", name: "y" },
      value: { kind: "NumberLiteral", value: 2 },
    } satisfies Assignment);
  });

  it("should parse assignment then expression", () => {
    /** `x = 1\nx + 2\n` — assignment followed by expression statement. */
    const tokens = [
      tok("NAME", "x", 1),
      tok("EQUALS", "=", 1),
      tok("NUMBER", "1", 1),
      nl(1),
      tok("NAME", "x", 2),
      tok("PLUS", "+", 2),
      tok("NUMBER", "2", 2),
      nl(2),
      eof(3),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(2);
    expect(program.statements[0]).toEqual({
      kind: "Assignment",
      target: { kind: "Name", name: "x" },
      value: { kind: "NumberLiteral", value: 1 },
    } satisfies Assignment);
    expect(program.statements[1]).toEqual({
      kind: "BinaryOp",
      left: { kind: "Name", name: "x" },
      op: "+",
      right: { kind: "NumberLiteral", value: 2 },
    } satisfies BinaryOp);
  });

  it("should parse three statements", () => {
    /** Three statements in sequence. */
    const tokens = [
      // a = 10
      tok("NAME", "a", 1),
      tok("EQUALS", "=", 1),
      tok("NUMBER", "10", 1),
      nl(1),
      // b = 20
      tok("NAME", "b", 2),
      tok("EQUALS", "=", 2),
      tok("NUMBER", "20", 2),
      nl(2),
      // a + b
      tok("NAME", "a", 3),
      tok("PLUS", "+", 3),
      tok("NAME", "b", 3),
      nl(3),
      eof(4),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(3);
  });

  it("should skip blank lines between statements", () => {
    /** Blank lines (extra newlines) between statements should be skipped. */
    const tokens = [
      tok("NUMBER", "1"),
      nl(),
      nl(), // blank line
      nl(), // another blank line
      tok("NUMBER", "2"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(2);
    expect(program.statements[0]).toEqual({
      kind: "NumberLiteral",
      value: 1,
    } satisfies NumberLiteral);
    expect(program.statements[1]).toEqual({
      kind: "NumberLiteral",
      value: 2,
    } satisfies NumberLiteral);
  });

  it("should skip leading newlines", () => {
    /** Leading blank lines before any statements should be skipped. */
    const tokens = [
      nl(),
      nl(),
      tok("NUMBER", "42"),
      eof(),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual({
      kind: "NumberLiteral",
      value: 42,
    } satisfies NumberLiteral);
  });
});

// =============================================================================
// TEST: EMPTY PROGRAM
// =============================================================================

describe("TestEmptyProgram", () => {
  it("should parse an empty program", () => {
    /** An empty program (just EOF) should produce an empty statement list. */
    const tokens = [eof()];
    const program = parseTokens(tokens);

    expect(program).toEqual({ kind: "Program", statements: [] } satisfies Program);
  });

  it("should parse a program with only newlines", () => {
    /** A program with only newlines should produce an empty statement list. */
    const tokens = [nl(), nl(), nl(), eof()];
    const program = parseTokens(tokens);

    expect(program).toEqual({ kind: "Program", statements: [] } satisfies Program);
  });
});

// =============================================================================
// TEST: ERROR HANDLING
// =============================================================================
//
// The parser should raise clear, informative errors when it encounters
// invalid syntax. Good error messages are critical for a good developer
// experience.
// =============================================================================

describe("TestErrors", () => {
  it("should error on unexpected token at start", () => {
    /** A stray operator at the start should raise ParseError. */
    const tokens = [
      tok("PLUS", "+"),
      tok("NUMBER", "1"),
      eof(),
    ];
    expect(() => parseTokens(tokens)).toThrow(ParseError);
    expect(() => parseTokens(tokens)).toThrow(/Unexpected token/);
  });

  it("should error on missing closing paren", () => {
    /** `(1 + 2` without closing paren should raise ParseError. */
    const tokens = [
      tok("LPAREN", "("),
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      eof(),
    ];
    expect(() => parseTokens(tokens)).toThrow(ParseError);
    expect(() => parseTokens(tokens)).toThrow(/Expected RPAREN, got EOF/);
  });

  it("should error on unexpected equals", () => {
    /** `= 42` (equals without a name on the left) should be an error. */
    const tokens = [
      tok("EQUALS", "="),
      tok("NUMBER", "42"),
      eof(),
    ];
    expect(() => parseTokens(tokens)).toThrow(ParseError);
    expect(() => parseTokens(tokens)).toThrow(/Unexpected token/);
  });

  it("should include token info in error", () => {
    /** ParseError should include line and column information. */
    const badToken = tok("RPAREN", ")", 3, 7);
    const tokens = [badToken, eof()];

    try {
      parseTokens(tokens);
      expect.unreachable("Should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(ParseError);
      const error = e as ParseError;
      expect(error.token).toEqual(badToken);
      expect(String(error)).toContain("line 3");
      expect(String(error)).toContain("column 7");
    }
  });

  it("should error on missing operand after plus", () => {
    /** `1 +` with no right operand should raise ParseError. */
    const tokens = [
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      eof(),
    ];
    expect(() => parseTokens(tokens)).toThrow(ParseError);
    expect(() => parseTokens(tokens)).toThrow(/Unexpected token/);
  });

  it("should error on missing operand after star", () => {
    /** `2 *` with no right operand should raise ParseError. */
    const tokens = [
      tok("NUMBER", "2"),
      tok("STAR", "*"),
      eof(),
    ];
    expect(() => parseTokens(tokens)).toThrow(ParseError);
    expect(() => parseTokens(tokens)).toThrow(/Unexpected token/);
  });

  it("should error on double operator", () => {
    /** `1 + + 2` — double operator should be an error. */
    const tokens = [
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      eof(),
    ];
    expect(() => parseTokens(tokens)).toThrow(ParseError);
    expect(() => parseTokens(tokens)).toThrow(/Unexpected token/);
  });

  it("should error on unexpected rparen", () => {
    /** A stray `)` without matching `(` should be caught. */
    const tokens = [
      tok("NUMBER", "1"),
      tok("RPAREN", ")"),
      eof(),
    ];
    // The parser will parse NumberLiteral(1) as an expression statement,
    // then try to consume NEWLINE but find RPAREN instead.
    expect(() => parseTokens(tokens)).toThrow(ParseError);
    expect(() => parseTokens(tokens)).toThrow(/Expected NEWLINE, got RPAREN/);
  });
});

// =============================================================================
// TEST: EXPRESSION STATEMENTS
// =============================================================================
//
// When an expression appears on its own line (not as part of an assignment),
// it's an expression statement. The parser should handle these correctly,
// distinguishing them from assignments.
// =============================================================================

describe("TestExpressionStatements", () => {
  it("should parse number expression statement: 42", () => {
    /** `42\n` — a number on its own line. */
    const tokens = [
      tok("NUMBER", "42"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual({
      kind: "NumberLiteral",
      value: 42,
    } satisfies NumberLiteral);
  });

  it("should parse binary expression statement: 1 + 2", () => {
    /** `1 + 2\n` — an expression on its own line. */
    const tokens = [
      tok("NUMBER", "1"),
      tok("PLUS", "+"),
      tok("NUMBER", "2"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual({
      kind: "BinaryOp",
      left: { kind: "NumberLiteral", value: 1 },
      op: "+",
      right: { kind: "NumberLiteral", value: 2 },
    } satisfies BinaryOp);
  });

  it("should parse name expression statement: x", () => {
    /** `x\n` — a name on its own line (not an assignment). */
    const tokens = [
      tok("NAME", "x"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(1);
    expect(program.statements[0]).toEqual({
      kind: "Name",
      name: "x",
    } satisfies Name);
  });

  it("should not confuse name expression with assignment: x + 1", () => {
    /** `x + 1\n` — name followed by operator, not assignment. */
    const tokens = [
      tok("NAME", "x"),
      tok("PLUS", "+"),
      tok("NUMBER", "1"),
      nl(),
      eof(),
    ];
    const program = parseTokens(tokens);

    expect((program.statements[0] as BinaryOp).kind).toBe("BinaryOp");
  });
});

// =============================================================================
// TEST: END-TO-END (THE TARGET EXPRESSION)
// =============================================================================
//
// This is the "money test" — the expression from the spec that demonstrates
// the parser working end-to-end: `x = 1 + 2`
// =============================================================================

describe("TestEndToEnd", () => {
  it("should parse the target expression: x = 1 + 2", () => {
    /**
     * The target expression `x = 1 + 2` should produce the correct AST.
     *
     * This is the canonical example from the spec. It exercises:
     * - Assignment parsing
     * - Expression parsing with a binary operator
     * - Number literals
     * - Variable names
     */
    const tokens = [
      tok("NAME", "x", 1, 1),
      tok("EQUALS", "=", 1, 3),
      tok("NUMBER", "1", 1, 5),
      tok("PLUS", "+", 1, 7),
      tok("NUMBER", "2", 1, 9),
      nl(1, 10),
      eof(2, 1),
    ];
    const program = parseTokens(tokens);

    const expected: Program = {
      kind: "Program",
      statements: [
        {
          kind: "Assignment",
          target: { kind: "Name", name: "x" },
          value: {
            kind: "BinaryOp",
            left: { kind: "NumberLiteral", value: 1 },
            op: "+",
            right: { kind: "NumberLiteral", value: 2 },
          },
        },
      ],
    };
    expect(program).toEqual(expected);
  });

  it("should parse a full multi-line program", () => {
    /**
     * A multi-line program with assignments and expressions.
     *
     * Simulates:
     *     x = 10
     *     y = 20
     *     x + y * 2
     */
    const tokens = [
      // x = 10
      tok("NAME", "x", 1, 1),
      tok("EQUALS", "=", 1, 3),
      tok("NUMBER", "10", 1, 5),
      nl(1),
      // y = 20
      tok("NAME", "y", 2, 1),
      tok("EQUALS", "=", 2, 3),
      tok("NUMBER", "20", 2, 5),
      nl(2),
      // x + y * 2
      tok("NAME", "x", 3, 1),
      tok("PLUS", "+", 3, 3),
      tok("NAME", "y", 3, 5),
      tok("STAR", "*", 3, 7),
      tok("NUMBER", "2", 3, 9),
      nl(3),
      eof(4),
    ];
    const program = parseTokens(tokens);

    expect(program.statements).toHaveLength(3);
    // x = 10
    expect(program.statements[0]).toEqual({
      kind: "Assignment",
      target: { kind: "Name", name: "x" },
      value: { kind: "NumberLiteral", value: 10 },
    } satisfies Assignment);
    // y = 20
    expect(program.statements[1]).toEqual({
      kind: "Assignment",
      target: { kind: "Name", name: "y" },
      value: { kind: "NumberLiteral", value: 20 },
    } satisfies Assignment);
    // x + y * 2  ->  x + (y * 2)
    expect(program.statements[2]).toEqual({
      kind: "BinaryOp",
      left: { kind: "Name", name: "x" },
      op: "+",
      right: {
        kind: "BinaryOp",
        left: { kind: "Name", name: "y" },
        op: "*",
        right: { kind: "NumberLiteral", value: 2 },
      },
    } satisfies BinaryOp);
  });
});

// =============================================================================
// TEST: PARSER CLASS API
// =============================================================================

describe("TestParserAPI", () => {
  it("should always return a Program from parse()", () => {
    /** parse() should always return a Program node. */
    const tokens = [eof()];
    const parser = new Parser(tokens);
    const result = parser.parse();

    expect(result.kind).toBe("Program");
  });

  it("should make ParseError a proper Error subclass", () => {
    /** ParseError should be a proper Error subclass. */
    const token = tok("PLUS", "+");
    const error = new ParseError("test message", token);

    expect(error).toBeInstanceOf(Error);
    expect(error.token).toEqual(token);
  });

  it("should include location in ParseError string", () => {
    /** ParseError string representation includes location. */
    const token = tok("PLUS", "+", 5, 10);
    const error = new ParseError("bad syntax", token);

    expect(String(error)).toContain("bad syntax");
    expect(String(error)).toContain("line 5");
    expect(String(error)).toContain("column 10");
  });
});
