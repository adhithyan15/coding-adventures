/**
 * Tests for the Starlark Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes Starlark
 * source code when loaded with the `starlark.tokens` grammar file.
 *
 * Starlark is a restricted dialect of Python used in build systems (Bazel, Buck).
 * It shares Python's lexical structure but has additional constraints:
 *   - Reserved keywords (class, import, while, etc.) cause errors
 *   - Significant indentation produces INDENT/DEDENT tokens
 *   - No while loops, no classes, no try/except
 */

import { describe, it, expect } from "vitest";
import { tokenizeStarlark } from "../src/tokenizer.js";

/**
 * Helper: extract just the token types from a source string.
 * This makes assertions concise — we can compare arrays of type strings
 * instead of inspecting full Token objects.
 */
function tokenTypes(source: string): string[] {
  return tokenizeStarlark(source).map((t) => t.type);
}

/**
 * Helper: extract just the token values from a source string.
 * Useful for verifying that the lexer captures the correct text for each token.
 */
function tokenValues(source: string): string[] {
  return tokenizeStarlark(source).map((t) => t.value);
}

describe("simple expressions", () => {
  it("tokenizes x = 1 + 2", () => {
    /**
     * The simplest Starlark assignment: a name, equals sign, and an
     * arithmetic expression. Note that Starlark uses INT (not NUMBER)
     * for integer literals, matching the starlark.tokens grammar.
     */
    const types = tokenTypes("x = 1 + 2");
    expect(types).toEqual([
      "NAME", "EQUALS", "INT", "PLUS", "INT", "NEWLINE", "EOF",
    ]);
  });

  it("captures correct values for x = 1 + 2", () => {
    /**
     * Verify the lexer captures the correct text for each token.
     * The NEWLINE token's value is a literal newline character.
     * The EOF token's value is an empty string.
     */
    const tokens = tokenizeStarlark("x = 1 + 2");
    const nonSynthetic = tokens.filter((t) => t.type !== "NEWLINE" && t.type !== "EOF");
    expect(nonSynthetic.map((t) => t.value)).toEqual(["x", "=", "1", "+", "2"]);
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("tokenizes all arithmetic operators", () => {
    /**
     * Starlark supports the standard arithmetic operators: +, -, *, /, %.
     * These are single-character tokens, each with their own type.
     */
    const types = tokenTypes("a + b - c * d / e % f");
    expect(types).toEqual([
      "NAME", "PLUS", "NAME", "MINUS", "NAME",
      "STAR", "NAME", "SLASH", "NAME", "PERCENT", "NAME",
      "NEWLINE", "EOF",
    ]);
  });
});

describe("Starlark keywords", () => {
  it("recognizes def as a keyword", () => {
    /**
     * 'def' defines a function in Starlark. The lexer matches it as a NAME
     * first, then reclassifies it to KEYWORD because 'def' appears in the
     * keywords: section of starlark.tokens.
     */
    const tokens = tokenizeStarlark("def");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("def");
  });

  it("recognizes return as a keyword", () => {
    const tokens = tokenizeStarlark("return");
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("return");
  });

  it("recognizes if, else, elif as keywords", () => {
    /**
     * Conditional keywords. Starlark supports if/elif/else but NOT
     * while or try/except (those are reserved and cause errors).
     */
    const tokens = tokenizeStarlark("if else elif");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["if", "else", "elif"]);
  });

  it("recognizes for, in, pass as keywords", () => {
    /**
     * Loop and placeholder keywords. 'for' and 'in' are used together
     * for iteration. 'pass' is a no-op placeholder for empty blocks.
     */
    const tokens = tokenizeStarlark("for in pass");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["for", "in", "pass"]);
  });

  it("recognizes True, False, None as keywords", () => {
    /**
     * Boolean and null literals in Starlark are capitalized (like Python).
     * They are classified as KEYWORD tokens, not separate literal types.
     */
    const tokens = tokenizeStarlark("True False None");
    const keywords = tokens.filter((t) => t.type === "KEYWORD");
    expect(keywords.map((k) => k.value)).toEqual(["True", "False", "None"]);
  });
});

describe("reserved keywords cause errors", () => {
  it("rejects 'class' as a reserved keyword", () => {
    /**
     * Starlark explicitly reserves Python keywords it does not support.
     * Using 'class' should produce a clear error instead of a confusing
     * parse failure. The lexer checks the reserved: section and throws.
     */
    expect(() => tokenizeStarlark("class Foo:")).toThrow();
  });

  it("rejects 'import' as a reserved keyword", () => {
    /**
     * Starlark uses 'load()' instead of 'import'. The 'import' keyword
     * is reserved to prevent confusion with Python import statements.
     */
    expect(() => tokenizeStarlark("import os")).toThrow();
  });
});

describe("INDENT and DEDENT from indented blocks", () => {
  it("emits INDENT and DEDENT for a function body", () => {
    /**
     * Starlark uses significant indentation like Python. When the lexer
     * encounters a line with more leading whitespace than the current
     * indentation level, it emits an INDENT token. When indentation
     * decreases, it emits one or more DEDENT tokens to return to the
     * previous level.
     *
     * For a simple function:
     *   def f():
     *       return 1
     *
     * The token stream includes:
     *   KEYWORD("def") NAME("f") LPAREN RPAREN COLON NEWLINE
     *   INDENT KEYWORD("return") INT("1") NEWLINE DEDENT EOF
     */
    const types = tokenTypes("def f():\n    return 1");
    expect(types).toContain("INDENT");
    expect(types).toContain("DEDENT");
  });

  it("emits matching INDENT/DEDENT pairs for nested blocks", () => {
    /**
     * Nested indentation produces nested INDENT/DEDENT pairs. Each INDENT
     * must eventually be matched by a DEDENT when the block ends.
     */
    const source = "if True:\n    if True:\n        pass";
    const types = tokenTypes(source);

    const indentCount = types.filter((t) => t === "INDENT").length;
    const dedentCount = types.filter((t) => t === "DEDENT").length;
    expect(indentCount).toBe(2);
    expect(dedentCount).toBe(2);
  });
});

describe("multi-character operators", () => {
  it("tokenizes == and != comparison operators", () => {
    /**
     * Two-character comparison operators. The grammar file lists these
     * before the single-character = so that "==" is matched as one token,
     * not two EQUALS tokens.
     */
    const types = tokenTypes("x == 1");
    expect(types).toContain("EQUALS_EQUALS");

    const types2 = tokenTypes("x != 1");
    expect(types2).toContain("NOT_EQUALS");
  });

  it("tokenizes ** exponentiation and // floor division", () => {
    /**
     * ** is exponentiation (2 ** 10 = 1024).
     * // is floor division (7 // 2 = 3).
     * Both must be recognized as two-character tokens, not two single-char tokens.
     */
    const types = tokenTypes("2 ** 10");
    expect(types).toContain("DOUBLE_STAR");

    const types2 = tokenTypes("7 // 2");
    expect(types2).toContain("FLOOR_DIV");
  });

  it("tokenizes augmented assignment operators", () => {
    /**
     * Augmented assignments combine an operator with assignment:
     *   x += 1   is shorthand for   x = x + 1
     *
     * Starlark supports +=, -=, *=, /=, //=, %=, and more.
     */
    const types = tokenTypes("x += 1");
    expect(types).toContain("PLUS_EQUALS");

    const types2 = tokenTypes("x -= 1");
    expect(types2).toContain("MINUS_EQUALS");

    const types3 = tokenTypes("x //= 2");
    expect(types3).toContain("FLOOR_DIV_EQUALS");
  });
});

describe("string literals", () => {
  it("tokenizes a double-quoted string", () => {
    /**
     * Starlark strings use Python syntax. Double-quoted strings are the
     * most common form. The lexer strips the quotes and returns just the
     * content as the token value.
     */
    const tokens = tokenizeStarlark('"hello"');
    expect(tokens[0].type).toBe("STRING");
  });

  it("tokenizes a single-quoted string", () => {
    const tokens = tokenizeStarlark("'hello'");
    expect(tokens[0].type).toBe("STRING");
  });
});

describe("comment skipping", () => {
  it("skips comments and does not include them in token stream", () => {
    /**
     * Comments in Starlark start with # and extend to end of line.
     * The lexer's skip section matches comments via a regex pattern
     * and discards the match without producing a token.
     */
    const types = tokenTypes("x = 1 # this is a comment");
    expect(types).not.toContain("COMMENT");
    expect(types).toEqual([
      "NAME", "EQUALS", "INT", "NEWLINE", "EOF",
    ]);
  });
});

describe("position tracking", () => {
  it("tracks line and column for each token", () => {
    /**
     * Every token includes position information: the line number and
     * column number where it starts. This is essential for error messages
     * that point to the exact location of a problem.
     *
     * Both line and column are 1-indexed (first line is line 1, first
     * column is column 1), following the Token interface convention.
     */
    const tokens = tokenizeStarlark("x = 1\ny = 2");

    // First token: 'x' is at line 1, column 1
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1);

    // After the NEWLINE, 'y' should be on line 2
    const yToken = tokens.find((t) => t.value === "y");
    expect(yToken).toBeDefined();
    expect(yToken!.line).toBe(2);
  });
});

describe("delimiters and punctuation", () => {
  it("tokenizes parentheses, brackets, braces, commas, colons", () => {
    /**
     * Starlark uses all standard Python delimiters:
     *   ()  — function calls and grouping
     *   []  — list literals and indexing
     *   {}  — dict literals
     *   ,   — separating arguments and elements
     *   :   — after if/for/def headers, in dict literals, in slices
     */
    const types = tokenTypes("f(a, b)");
    expect(types).toEqual([
      "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN", "NEWLINE", "EOF",
    ]);
  });
});

describe("numeric literals", () => {
  it("tokenizes integer and float literals", () => {
    /**
     * Starlark distinguishes INT and FLOAT tokens. The grammar file
     * puts FLOAT patterns before INT patterns so that "3.14" matches
     * as FLOAT(3.14), not INT(3) DOT INT(14).
     */
    const intTokens = tokenizeStarlark("42");
    expect(intTokens[0].type).toBe("INT");

    const floatTokens = tokenizeStarlark("3.14");
    expect(floatTokens[0].type).toBe("FLOAT");
  });

  it("tokenizes hex literals", () => {
    /**
     * Hex integers use the 0x prefix. They are aliased to INT via the
     * -> INT syntax in the grammar file, so they appear as INT tokens.
     */
    const tokens = tokenizeStarlark("0xFF");
    expect(tokens[0].type).toBe("INT");
  });
});
