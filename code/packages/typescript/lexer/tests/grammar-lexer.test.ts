/**
 * Tests for the Grammar-Driven Lexer
 * ====================================
 *
 * These tests verify that `grammarTokenize` correctly tokenizes source code
 * using token definitions from a `.tokens` file. The critical property we
 * are testing is **interchangeability**: for all well-formed inputs, the
 * `grammarTokenize` must produce *identical* token output to the hand-written
 * `tokenize`.
 *
 * We test in three layers:
 *
 * 1. **Standalone tests** — verify grammarTokenize behavior on its own
 * 2. **Comparison tests** — verify grammarTokenize matches hand-written tokenize
 * 3. **Custom grammar tests** — verify grammarTokenize works with minimal/custom
 *    grammars (not loaded from a file)
 *
 * The python.tokens grammar file is loaded from the `code/grammars/` directory,
 * which is the canonical source of token definitions for this project.
 */

import { describe, it, expect, beforeAll } from "vitest";
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

import {
  parseTokenGrammar,
  type TokenGrammar,
  type TokenDefinition,
} from "@coding-adventures/grammar-tools";

import { grammarTokenize } from "../src/grammar-lexer.js";
import { tokenize, LexerError } from "../src/tokenizer.js";
import type { Token } from "../src/token.js";
import type { LexerConfig } from "../src/tokenizer.js";

// ---------------------------------------------------------------------------
// Path resolution — locate the grammar files
// ---------------------------------------------------------------------------

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * The grammars directory is four levels up from the test file, then into grammars/:
 *   tests/ -> lexer/ -> typescript/ -> packages/ -> code/ -> grammars/
 * That's four ".." to reach code/, then "grammars".
 */
const GRAMMARS_DIR = join(__dirname, "..", "..", "..", "..", "grammars");

// ---------------------------------------------------------------------------
// Fixtures — load grammars once for all tests
// ---------------------------------------------------------------------------

let pythonGrammar: TokenGrammar;
let rubyGrammar: TokenGrammar;
let pythonConfig: LexerConfig;

beforeAll(() => {
  const pythonTokensPath = join(GRAMMARS_DIR, "python.tokens");
  const rubyTokensPath = join(GRAMMARS_DIR, "ruby.tokens");

  pythonGrammar = parseTokenGrammar(readFileSync(pythonTokensPath, "utf-8"));
  rubyGrammar = parseTokenGrammar(readFileSync(rubyTokensPath, "utf-8"));

  // Create a LexerConfig with the same keywords as python.tokens,
  // so the hand-written tokenize and grammarTokenize produce identical output.
  pythonConfig = { keywords: [...pythonGrammar.keywords] };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function types(tokens: Token[]): string[] {
  return tokens.map((t) => t.type);
}

function values(tokens: Token[]): string[] {
  return tokens.map((t) => t.value);
}

// ============================================================================
// Standalone tests — grammarTokenize behavior
// ============================================================================

describe("grammarTokenize — basics", () => {
  it("should tokenize a simple assignment: x = 1 + 2", () => {
    const tokens = grammarTokenize("x = 1 + 2", pythonGrammar);
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF",
    ]);
    expect(values(tokens)).toEqual(["x", "=", "1", "+", "2", ""]);
  });

  it("should tokenize arithmetic: 1 + 2 * 3", () => {
    const tokens = grammarTokenize("1 + 2 * 3", pythonGrammar);
    expect(types(tokens)).toEqual([
      "NUMBER", "PLUS", "NUMBER", "STAR", "NUMBER", "EOF",
    ]);
    expect(values(tokens)).toEqual(["1", "+", "2", "*", "3", ""]);
  });

  it("should tokenize string literal", () => {
    const tokens = grammarTokenize('"Hello, World!"', pythonGrammar);
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("Hello, World!");
    expect(tokens[1].type).toBe("EOF");
  });

  it("should tokenize empty string", () => {
    const tokens = grammarTokenize('""', pythonGrammar);
    expect(tokens[0].type).toBe("STRING");
    expect(tokens[0].value).toBe("");
  });

  it("should handle escape sequence \\n", () => {
    const tokens = grammarTokenize('"hello\\nworld"', pythonGrammar);
    expect(tokens[0].value).toBe("hello\nworld");
  });

  it("should handle escape sequence \\t", () => {
    const tokens = grammarTokenize('"col1\\tcol2"', pythonGrammar);
    expect(tokens[0].value).toBe("col1\tcol2");
  });

  it("should handle escape sequence \\\\", () => {
    const tokens = grammarTokenize('"path\\\\to\\\\file"', pythonGrammar);
    expect(tokens[0].value).toBe("path\\to\\file");
  });

  it("should handle escape sequence \\\"", () => {
    const tokens = grammarTokenize('"He said \\"hi\\""', pythonGrammar);
    expect(tokens[0].value).toBe('He said "hi"');
  });

  it("should handle unknown escape sequences", () => {
    const tokens = grammarTokenize('"hello\\xworld"', pythonGrammar);
    expect(tokens[0].value).toBe("helloxworld");
  });

  it("should tokenize multiline input", () => {
    const tokens = grammarTokenize("x = 1\ny = 2", pythonGrammar);
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NUMBER", "NEWLINE",
      "NAME", "EQUALS", "NUMBER", "EOF",
    ]);
  });

  it("should handle blank lines (consecutive newlines)", () => {
    const tokens = grammarTokenize("x\n\ny", pythonGrammar);
    expect(types(tokens)).toEqual([
      "NAME", "NEWLINE", "NEWLINE", "NAME", "EOF",
    ]);
  });

  it("should produce only EOF for empty input", () => {
    const tokens = grammarTokenize("", pythonGrammar);
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("should produce only EOF for whitespace-only input", () => {
    const tokens = grammarTokenize("   \t  ", pythonGrammar);
    expect(tokens).toHaveLength(1);
    expect(tokens[0].type).toBe("EOF");
  });

  it("should distinguish = from ==", () => {
    const tokens = grammarTokenize("a = b == c", pythonGrammar);
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NAME", "EQUALS_EQUALS", "NAME", "EOF",
    ]);
  });

  it("should tokenize function call style", () => {
    const tokens = grammarTokenize("print(x, y)", pythonGrammar);
    expect(types(tokens)).toEqual([
      "NAME", "LPAREN", "NAME", "COMMA", "NAME", "RPAREN", "EOF",
    ]);
  });

  it("should tokenize tokens without spaces", () => {
    const tokens = grammarTokenize("x=1+2", pythonGrammar);
    expect(types(tokens)).toEqual([
      "NAME", "EQUALS", "NUMBER", "PLUS", "NUMBER", "EOF",
    ]);
  });

  it("should track position: x = 1", () => {
    const tokens = grammarTokenize("x = 1", pythonGrammar);
    expect(tokens[0].line).toBe(1);
    expect(tokens[0].column).toBe(1); // x
    expect(tokens[1].column).toBe(3); // =
    expect(tokens[2].column).toBe(5); // 1
  });

  it("should track position across line boundaries", () => {
    const tokens = grammarTokenize("abc\nde = 1", pythonGrammar);
    expect(tokens[0]).toEqual({ type: "NAME", value: "abc", line: 1, column: 1 });
    const deToken = tokens.find((t) => t.value === "de");
    expect(deToken?.line).toBe(2);
    expect(deToken?.column).toBe(1);
  });

  it("should place EOF at correct position", () => {
    const tokens = grammarTokenize("ab", pythonGrammar);
    const eof = tokens[tokens.length - 1];
    expect(eof.type).toBe("EOF");
    expect(eof.line).toBe(1);
    expect(eof.column).toBe(3);
  });
});

// ============================================================================
// Keyword tests
// ============================================================================

describe("grammarTokenize — keywords", () => {
  it("should classify 'if' as KEYWORD", () => {
    const tokens = grammarTokenize("if x == 1", pythonGrammar);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("if");
  });

  it("should classify 'def' as KEYWORD", () => {
    const tokens = grammarTokenize("def foo", pythonGrammar);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("def");
  });

  it("should keep non-keywords as NAME", () => {
    const tokens = grammarTokenize("iffy", pythonGrammar);
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("iffy");
  });

  it("should recognize all Python keywords", () => {
    for (const keyword of pythonGrammar.keywords) {
      const tokens = grammarTokenize(keyword, pythonGrammar);
      expect(tokens[0].type).toBe("KEYWORD");
      expect(tokens[0].value).toBe(keyword);
    }
  });
});

// ============================================================================
// Error tests
// ============================================================================

describe("grammarTokenize — errors", () => {
  it("should throw on unexpected character @", () => {
    expect(() => grammarTokenize("@", pythonGrammar)).toThrow(LexerError);
    expect(() => grammarTokenize("@", pythonGrammar)).toThrow("Unexpected character");
  });

  it("should throw on unexpected character #", () => {
    expect(() => grammarTokenize("#", pythonGrammar)).toThrow(LexerError);
  });

  it("should report correct error position", () => {
    try {
      grammarTokenize("x = @", pythonGrammar);
      expect.unreachable("Should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(LexerError);
      expect((e as LexerError).line).toBe(1);
      expect((e as LexerError).column).toBe(5);
    }
  });

  it("should report correct error position on second line", () => {
    try {
      grammarTokenize("x = 1\n@", pythonGrammar);
      expect.unreachable("Should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(LexerError);
      expect((e as LexerError).line).toBe(2);
      expect((e as LexerError).column).toBe(1);
    }
  });
});

// ============================================================================
// Comparison tests — grammarTokenize vs. hand-written tokenize
// ============================================================================

describe("grammarTokenize vs. tokenize — interchangeability", () => {
  /**
   * A collection of source strings to test with both lexers.
   * Each entry is a source string that both lexers should handle identically.
   */
  const COMPARISON_INPUTS: string[] = [
    // Simple expressions
    "x = 1 + 2",
    "1 + 2 * 3",
    "a + b - c",
    "x == 1",
    "a = b == c",
    // Operators without spaces
    "x=1+2",
    "+-*/",
    // Strings
    '"Hello, World!"',
    '""',
    '"abc 123"',
    // Parentheses and delimiters
    "print(x, y)",
    "(1 + 2)",
    "key: value",
    // Multi-line
    "x = 1\ny = 2",
    "a = 1\nb = 2\nc = a + b",
    "x\n\ny",
    // Whitespace variations
    "  x   =   1  ",
    "\tx",
    "x\r= 1",
    // Edge cases
    "",
    "   \t  ",
    "\n\n",
    "x",
    "_",
    "_foo",
    "var1",
    "hello_world_123",
    "0",
    "42",
    "1000",
    // Mixed
    'x = "hello"',
    '"a" "b"',
  ];

  it.each(COMPARISON_INPUTS)(
    "should produce identical tokens for: %s",
    (source) => {
      const grammarTokens = grammarTokenize(source, pythonGrammar);
      const handTokens = tokenize(source, pythonConfig);
      expect(grammarTokens).toEqual(handTokens);
    },
  );

  it("should match for keyword expression: if x == 1", () => {
    const grammarTokens = grammarTokenize("if x == 1", pythonGrammar);
    const handTokens = tokenize("if x == 1", pythonConfig);
    expect(grammarTokens).toEqual(handTokens);
  });

  it("should match for string escapes", () => {
    const testCases = [
      '"hello\\nworld"',
      '"col1\\tcol2"',
      '"path\\\\to\\\\file"',
      '"He said \\"hi\\""',
      '"hello\\xworld"',
    ];
    for (const source of testCases) {
      const grammarTokens = grammarTokenize(source, pythonGrammar);
      const handTokens = tokenize(source, pythonConfig);
      expect(grammarTokens).toEqual(handTokens);
    }
  });

  it("should match for return keyword in expression", () => {
    const grammarTokens = grammarTokenize("return x + 1", pythonGrammar);
    const handTokens = tokenize("return x + 1", pythonConfig);
    expect(grammarTokens).toEqual(handTokens);
  });
});

// ============================================================================
// Ruby grammar tests
// ============================================================================

describe("grammarTokenize — Ruby grammar", () => {
  it("should recognize Ruby keywords", () => {
    const tokens = grammarTokenize("if x end", rubyGrammar);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("if");
    expect(tokens[2].type).toBe("KEYWORD");
    expect(tokens[2].value).toBe("end");
  });

  it("should recognize elsif as a keyword (Ruby-specific)", () => {
    const tokens = grammarTokenize("elsif", rubyGrammar);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("elsif");
  });

  it("should recognize puts as a keyword", () => {
    const tokens = grammarTokenize("puts", rubyGrammar);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("puts");
  });

  it("should tokenize Ruby assignment", () => {
    const tokens = grammarTokenize("x = 42", rubyGrammar);
    expect(types(tokens)).toEqual(["NAME", "EQUALS", "NUMBER", "EOF"]);
  });
});

// ============================================================================
// Custom grammar tests — build a TokenGrammar programmatically
// ============================================================================

describe("grammarTokenize — custom grammars", () => {
  it("should work with a minimal numbers-only grammar", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NUMBER", pattern: "[0-9]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("42", grammar);
    expect(tokens[0]).toEqual({ type: "NUMBER", value: "42", line: 1, column: 1 });
    expect(tokens[1].type).toBe("EOF");
  });

  it("should work with names and a literal = operator", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1 },
        { name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 2 },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("x = y", grammar);
    expect(types(tokens)).toEqual(["NAME", "EQUALS", "NAME", "EOF"]);
  });

  it("should support custom keyword lists", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1 },
      ],
      keywords: ["let", "var"],
    };
    const tokens = grammarTokenize("let x", grammar);
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("let");
    expect(tokens[1].type).toBe("NAME");
    expect(tokens[1].value).toBe("x");
  });

  it("should fall back to NAME for unknown token names", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "IDENTIFIER", pattern: "[a-zA-Z]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("hello", grammar);
    // "IDENTIFIER" is not in the known types, so it falls back to NAME.
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("hello");
  });

  it("should escape regex-special characters in literal patterns", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "PLUS", pattern: "+", isRegex: false, lineNumber: 1 },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("+", grammar);
    expect(tokens[0]).toEqual({ type: "PLUS", value: "+", line: 1, column: 1 });
  });

  it("should use first-match-wins ordering", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "EQUALS_EQUALS", pattern: "==", isRegex: false, lineNumber: 1 },
        { name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 2 },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("==", grammar);
    expect(tokens[0].type).toBe("EQUALS_EQUALS");
    expect(tokens[0].value).toBe("==");
    expect(tokens[1].type).toBe("EOF");
  });

  it("should handle newlines regardless of grammar", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NUMBER", pattern: "[0-9]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("1\n2", grammar);
    expect(types(tokens)).toEqual(["NUMBER", "NEWLINE", "NUMBER", "EOF"]);
  });

  it("should raise error for unrecognized characters", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NUMBER", pattern: "[0-9]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
    };
    expect(() => grammarTokenize("abc", grammar)).toThrow(LexerError);
    expect(() => grammarTokenize("abc", grammar)).toThrow("Unexpected character");
  });
});
