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
} from "../../../../src/typescript/grammar-tools/index.js";

import { grammarTokenize, GrammarLexer, LexerContext } from "../src/grammar-lexer.js";
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

  it("should use string type for custom token names", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "IDENTIFIER", pattern: "[a-zA-Z]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("hello", grammar);
    // Custom token names are preserved as string types.
    expect(tokens[0].type).toBe("IDENTIFIER");
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

// ============================================================================
// Skip patterns
// ============================================================================

describe("grammarTokenize — skip patterns", () => {
  it("should skip whitespace via skip patterns", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NAME", pattern: "[a-z]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
      skipDefinitions: [
        { name: "WHITESPACE", pattern: "[ \\t]+", isRegex: true, lineNumber: 2 },
      ],
    };
    const tokens = grammarTokenize("hello world", grammar);
    const nameTokens = tokens.filter((t) => t.type === "NAME");
    expect(nameTokens).toHaveLength(2);
    expect(nameTokens[0].value).toBe("hello");
    expect(nameTokens[1].value).toBe("world");
  });
});

// ============================================================================
// Type aliases
// ============================================================================

describe("grammarTokenize — aliases", () => {
  it("should use alias as token type", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NUM", pattern: "[0-9]+", isRegex: true, lineNumber: 1, alias: "INT" },
      ],
      keywords: [],
    };
    const tokens = grammarTokenize("42", grammar);
    expect(tokens[0].type).toBe("INT");
  });
});

// ============================================================================
// Reserved keywords
// ============================================================================

describe("grammarTokenize — reserved keywords", () => {
  it("should throw on reserved keyword", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NAME", pattern: "[a-zA-Z_]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
      reservedKeywords: ["class", "import"],
    };
    expect(() => grammarTokenize("class", grammar)).toThrow(LexerError);
    expect(() => grammarTokenize("class", grammar)).toThrow(/Reserved keyword/);
  });

  it("should allow non-reserved identifiers", () => {
    const grammar: TokenGrammar = {
      definitions: [
        { name: "NAME", pattern: "[a-zA-Z_]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
      reservedKeywords: ["class"],
    };
    const tokens = grammarTokenize("hello", grammar);
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("hello");
  });
});

// ============================================================================
// Indentation mode
// ============================================================================

describe("grammarTokenize — indentation mode", () => {
  it("should emit INDENT and DEDENT tokens", () => {
    const grammar: TokenGrammar = {
      mode: "indentation",
      definitions: [
        { name: "NAME", pattern: "[a-zA-Z_]+", isRegex: true, lineNumber: 1 },
        { name: "EQUALS", pattern: "=", isRegex: false, lineNumber: 2 },
        { name: "INT", pattern: "[0-9]+", isRegex: true, lineNumber: 3 },
        { name: "COLON", pattern: ":", isRegex: false, lineNumber: 4 },
      ],
      keywords: ["if"],
      skipDefinitions: [
        { name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10 },
      ],
    };
    const tokens = grammarTokenize("if x:\n    y = 1\n", grammar);
    const typeList = tokens.map((t) => t.type);
    expect(typeList).toContain("INDENT");
    expect(typeList).toContain("DEDENT");
  });

  it("should reject tab indentation", () => {
    const grammar: TokenGrammar = {
      mode: "indentation",
      definitions: [
        { name: "NAME", pattern: "[a-z]+", isRegex: true, lineNumber: 1 },
        { name: "COLON", pattern: ":", isRegex: false, lineNumber: 2 },
      ],
      keywords: [],
      skipDefinitions: [
        { name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10 },
      ],
    };
    expect(() => grammarTokenize("if:\n\ty\n", grammar)).toThrow(LexerError);
    expect(() => grammarTokenize("if:\n\ty\n", grammar)).toThrow(/Tab/);
  });

  it("should handle empty source in indentation mode", () => {
    const grammar: TokenGrammar = {
      mode: "indentation",
      definitions: [],
      keywords: [],
      skipDefinitions: [
        { name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10 },
      ],
    };
    const tokens = grammarTokenize("", grammar);
    expect(tokens[tokens.length - 1].type).toBe("EOF");
  });

  it("should suppress newlines inside brackets", () => {
    const grammar: TokenGrammar = {
      mode: "indentation",
      definitions: [
        { name: "NAME", pattern: "[a-z]+", isRegex: true, lineNumber: 1 },
        { name: "LPAREN", pattern: "(", isRegex: false, lineNumber: 2 },
        { name: "RPAREN", pattern: ")", isRegex: false, lineNumber: 3 },
        { name: "COMMA", pattern: ",", isRegex: false, lineNumber: 4 },
      ],
      keywords: [],
      skipDefinitions: [
        { name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10 },
      ],
    };
    const tokens = grammarTokenize("f(\n  a,\n  b\n)\n", grammar);
    const typeList = tokens.map((t) => t.type);
    // No NEWLINE inside brackets
    const firstLParen = typeList.indexOf("LPAREN");
    const firstRParen = typeList.indexOf("RPAREN");
    const betweenParens = typeList.slice(firstLParen + 1, firstRParen);
    expect(betweenParens).not.toContain("NEWLINE");
  });

  it("should skip blank lines without emitting tokens", () => {
    const grammar: TokenGrammar = {
      mode: "indentation",
      definitions: [
        { name: "NAME", pattern: "[a-z]+", isRegex: true, lineNumber: 1 },
      ],
      keywords: [],
      skipDefinitions: [
        { name: "WS", pattern: "[ \\t]+", isRegex: true, lineNumber: 10 },
      ],
    };
    const tokens = grammarTokenize("x\n\ny\n", grammar);
    const nameTokens = tokens.filter((t) => t.type === "NAME");
    expect(nameTokens).toHaveLength(2);
  });
});

// ============================================================================
// Helper — create a grammar with pattern groups for testing
// ============================================================================

/**
 * Create a grammar with pattern groups for testing.
 *
 * This simulates a simplified XML-like grammar:
 * - Default group: TEXT and OPEN_TAG
 * - tag group: TAG_NAME, EQUALS, VALUE, TAG_CLOSE
 *
 * The grammar uses skip patterns for whitespace and no keywords.
 */
function makeGroupGrammar(): TokenGrammar {
  const source = `\
escapes: none

skip:
  WS = /[ \\t\\r\\n]+/

TEXT      = /[^<]+/
OPEN_TAG  = "<"

group tag:
  TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
  EQUALS    = "="
  VALUE     = /"[^"]*"/
  TAG_CLOSE = ">"
`;
  return parseTokenGrammar(source);
}

// ============================================================================
// LexerContext — unit tests for the callback interface
// ============================================================================

describe("LexerContext", () => {
  it("pushGroup() records a push action", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    ctx.pushGroup("tag");
    expect(ctx._groupActions).toEqual([["push", "tag"]]);
  });

  it("pushGroup() with unknown name throws", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    expect(() => ctx.pushGroup("nonexistent")).toThrow("Unknown pattern group");
  });

  it("popGroup() records a pop action", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    ctx.popGroup();
    expect(ctx._groupActions).toEqual([["pop", ""]]);
  });

  it("activeGroup() returns the top of the lexer group stack", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    expect(ctx.activeGroup()).toBe("default");
  });

  it("groupStackDepth() returns the length of the group stack", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    expect(ctx.groupStackDepth()).toBe(1);
  });

  it("emit() appends a synthetic token to the emitted list", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    const synthetic: Token = { type: "SYNTHETIC", value: "!", line: 1, column: 1 };
    ctx.emit(synthetic);
    expect(ctx._emitted).toEqual([synthetic]);
  });

  it("suppress() sets the suppressed flag", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    expect(ctx._suppressed).toBe(false);
    ctx.suppress();
    expect(ctx._suppressed).toBe(true);
  });

  it("peek() reads characters from the source after the token", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("hello", grammar);
    // Suppose token ended at position 3 (consumed "hel")
    const ctx = new LexerContext(lexer, "hello", 3);
    expect(ctx.peek(1)).toBe("l");
    expect(ctx.peek(2)).toBe("o");
    expect(ctx.peek(3)).toBe(""); // past EOF
  });

  it("peekStr() reads a substring from the source after the token", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("hello world", grammar);
    const ctx = new LexerContext(lexer, "hello world", 5);
    expect(ctx.peekStr(6)).toBe(" world");
  });

  it("setSkipEnabled() records the new skip state", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    expect(ctx._skipEnabled).toBeNull(); // no change by default
    ctx.setSkipEnabled(false);
    expect(ctx._skipEnabled).toBe(false);
  });

  it("multiple pushGroup() calls are recorded in order", () => {
    const grammar = makeGroupGrammar();
    const lexer = new GrammarLexer("x", grammar);
    const ctx = new LexerContext(lexer, "x", 1);
    ctx.pushGroup("tag");
    ctx.pushGroup("tag");
    expect(ctx._groupActions).toEqual([["push", "tag"], ["push", "tag"]]);
  });
});

// ============================================================================
// Pattern Group Tokenization — integration tests
// ============================================================================

describe("GrammarLexer — pattern group tokenization", () => {
  it("without a callback, only default group patterns are used", () => {
    const grammar = makeGroupGrammar();
    const tokens = new GrammarLexer("hello", grammar).tokenize();
    // TEXT pattern matches in default group
    expect(tokens[0].type).toBe("TEXT");
    expect(tokens[0].value).toBe("hello");
  });

  it("callback can push/pop groups to switch pattern sets", () => {
    // Simulates: <div> where < triggers push("tag"), > triggers pop().
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer("<div>hello", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        ctx.pushGroup("tag");
      } else if (token.type === "TAG_CLOSE") {
        ctx.popGroup();
      }
    });
    const tokens = lexer.tokenize();

    const pairs = tokens
      .filter((t) => t.type !== "EOF")
      .map((t) => [t.type, t.value]);
    expect(pairs).toEqual([
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "div"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "hello"],
    ]);
  });

  it("callback handles tag with attributes", () => {
    // Simulates: <div class="main">
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer('<div class="main">', grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        ctx.pushGroup("tag");
      } else if (token.type === "TAG_CLOSE") {
        ctx.popGroup();
      }
    });
    const tokens = lexer.tokenize();

    const pairs = tokens
      .filter((t) => t.type !== "EOF")
      .map((t) => [t.type, t.value]);
    expect(pairs).toEqual([
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "div"],
      ["TAG_NAME", "class"],
      ["EQUALS", "="],
      ["VALUE", '"main"'],
      ["TAG_CLOSE", ">"],
    ]);
  });

  it("group stack handles nested structures", () => {
    // Simulates: <a>text<b>inner</b></a> with push/pop on < and >.
    const source = `\
escapes: none

skip:
  WS = /[ \\t\\r\\n]+/

TEXT             = /[^<]+/
CLOSE_TAG_START  = "</"
OPEN_TAG         = "<"

group tag:
  TAG_NAME  = /[a-zA-Z_][a-zA-Z0-9_]*/
  TAG_CLOSE = ">"
  SLASH     = "/"
`;
    const grammar = parseTokenGrammar(source);

    const lexer = new GrammarLexer("<a>text<b>inner</b></a>", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG" || token.type === "CLOSE_TAG_START") {
        ctx.pushGroup("tag");
      } else if (token.type === "TAG_CLOSE") {
        ctx.popGroup();
      }
    });
    const tokens = lexer.tokenize();

    const pairs = tokens
      .filter((t) => t.type !== "EOF")
      .map((t) => [t.type, t.value]);
    expect(pairs).toEqual([
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "a"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "text"],
      ["OPEN_TAG", "<"],
      ["TAG_NAME", "b"],
      ["TAG_CLOSE", ">"],
      ["TEXT", "inner"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "b"],
      ["TAG_CLOSE", ">"],
      ["CLOSE_TAG_START", "</"],
      ["TAG_NAME", "a"],
      ["TAG_CLOSE", ">"],
    ]);
  });

  it("callback can suppress tokens (remove from output)", () => {
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer("<hello", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        ctx.suppress();
      }
    });
    const tokens = lexer.tokenize();

    const nonEof = tokens.filter((t) => t.type !== "EOF").map((t) => t.type);
    // OPEN_TAG was suppressed, only TEXT remains
    expect(nonEof).toEqual(["TEXT"]);
  });

  it("callback can emit synthetic tokens after the current one", () => {
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer("<hello", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        ctx.emit({
          type: "MARKER",
          value: "[start]",
          line: token.line,
          column: token.column,
        });
      }
    });
    const tokens = lexer.tokenize();

    const pairs = tokens
      .filter((t) => t.type !== "EOF")
      .map((t) => [t.type, t.value]);
    expect(pairs).toEqual([
      ["OPEN_TAG", "<"],
      ["MARKER", "[start]"],
      ["TEXT", "hello"],
    ]);
  });

  it("suppress + emit = token replacement", () => {
    // The current token is swallowed, but emitted tokens still output.
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer("<hello", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        ctx.suppress();
        ctx.emit({
          type: "REPLACED",
          value: "<",
          line: token.line,
          column: token.column,
        });
      }
    });
    const tokens = lexer.tokenize();

    const pairs = tokens
      .filter((t) => t.type !== "EOF")
      .map((t) => [t.type, t.value]);
    expect(pairs).toEqual([
      ["REPLACED", "<"],
      ["TEXT", "hello"],
    ]);
  });

  it("popping when only default remains is a no-op (no crash)", () => {
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer("hello", grammar);
    lexer.setOnToken((_token, ctx) => {
      ctx.popGroup(); // Should be safe even at the bottom
    });
    const tokens = lexer.tokenize();

    // Should still produce TEXT token without crashing
    expect(tokens[0].type).toBe("TEXT");
  });

  it("callback can disable skip patterns for significant whitespace", () => {
    // Grammar with a group that captures whitespace as a token
    const source = `\
escapes: none

skip:
  WS = /[ \\t]+/

TEXT      = /[^<]+/
START     = "<!"

group raw:
  RAW_TEXT = /[^>]+/
  END      = ">"
`;
    const grammar = parseTokenGrammar(source);

    const lexer = new GrammarLexer("<! hello world >after", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "START") {
        ctx.pushGroup("raw");
        ctx.setSkipEnabled(false);
      } else if (token.type === "END") {
        ctx.popGroup();
        ctx.setSkipEnabled(true);
      }
    });
    const tokens = lexer.tokenize();

    const pairs = tokens
      .filter((t) => t.type !== "EOF")
      .map((t) => [t.type, t.value]);
    expect(pairs).toEqual([
      ["START", "<!"],
      ["RAW_TEXT", " hello world "],
      ["END", ">"],
      ["TEXT", "after"],
    ]);
  });

  it("a grammar with no groups behaves identically (backward compat)", () => {
    const source = `\
NAME   = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER = /[0-9]+/
PLUS   = "+"
`;
    const grammar = parseTokenGrammar(source);
    const tokens = new GrammarLexer("x + 1", grammar).tokenize();

    const pairs = tokens
      .filter((t) => t.type !== "NEWLINE" && t.type !== "EOF")
      .map((t) => [t.type, t.value]);
    expect(pairs).toEqual([
      ["NAME", "x"],
      ["PLUS", "+"],
      ["NUMBER", "1"],
    ]);
  });

  it("passing null to setOnToken clears the callback", () => {
    const grammar = makeGroupGrammar();
    const called: string[] = [];

    const lexer = new GrammarLexer("hello", grammar);
    lexer.setOnToken((token, _ctx) => {
      called.push(token.type);
    });
    lexer.setOnToken(null);
    lexer.tokenize();

    expect(called).toEqual([]);
  });

  it("group stack resets between tokenize() calls", () => {
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer("<div", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        ctx.pushGroup("tag");
      }
    });

    // First call: pushes "tag" group
    const tokens1 = lexer.tokenize();
    expect(tokens1.some((t) => t.type === "TAG_NAME")).toBe(true);

    // Second call with a new instance should start fresh from "default"
    const lexer2 = new GrammarLexer("<div", grammar);
    lexer2.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        ctx.pushGroup("tag");
      }
    });
    const tokens2 = lexer2.tokenize();
    expect(tokens2.some((t) => t.type === "TAG_NAME")).toBe(true);
  });

  it("multiple push/pop in one callback are applied in order", () => {
    const grammar = makeGroupGrammar();

    const lexer = new GrammarLexer("<div", grammar);
    lexer.setOnToken((token, ctx) => {
      if (token.type === "OPEN_TAG") {
        // Push tag twice (stacking)
        ctx.pushGroup("tag");
        ctx.pushGroup("tag");
      }
    });
    // Should not crash with double push
    const tokens = lexer.tokenize();
    expect(tokens.some((t) => t.type === "TAG_NAME")).toBe(true);
  });

  it("grammarTokenize wrapper still works with group grammars", () => {
    // The convenience function should work transparently
    const grammar = makeGroupGrammar();
    const tokens = grammarTokenize("hello", grammar);
    expect(tokens[0].type).toBe("TEXT");
    expect(tokens[0].value).toBe("hello");
  });
});

// ============================================================================
// Case-insensitive keyword tests
// ============================================================================

/**
 * Build a minimal grammar with `# @case_insensitive true`.
 *
 * Tokens defined:
 *   - NAME  — [a-zA-Z_][a-zA-Z0-9_]*  (regex)
 *   - COMMA — ","                       (literal)
 * Skip: whitespace [ \t\r]+
 * Keywords: select, from
 */
function makeCaseInsensitiveGrammar(): TokenGrammar {
  // .tokens format: NAME = /pattern/ at top level (no "tokens:" section header).
  // Indented lines under skip: and keywords: belong to those sections.
  const source = [
    "# @case_insensitive true",
    "NAME = /[a-zA-Z_][a-zA-Z0-9_]*/",
    "skip:",
    "  WS = /[ \\t\\r\\n]+/",
    "keywords:",
    "  select",
    "  from",
  ].join("\n");
  return parseTokenGrammar(source);
}

describe("GrammarLexer — case-insensitive keywords", () => {
  it("lowercase 'select' is emitted as KEYWORD with value 'SELECT'", () => {
    const grammar = makeCaseInsensitiveGrammar();
    const tokens = new GrammarLexer("select", grammar).tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("SELECT");
  });

  it("uppercase 'SELECT' is emitted as KEYWORD with value 'SELECT'", () => {
    const grammar = makeCaseInsensitiveGrammar();
    const tokens = new GrammarLexer("SELECT", grammar).tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("SELECT");
  });

  it("mixed-case 'Select' is emitted as KEYWORD with value 'SELECT'", () => {
    const grammar = makeCaseInsensitiveGrammar();
    const tokens = new GrammarLexer("Select", grammar).tokenize();
    expect(tokens[0].type).toBe("KEYWORD");
    expect(tokens[0].value).toBe("SELECT");
  });

  it("non-keyword identifier retains its original casing in case-insensitive grammar", () => {
    // 'myTable' is not in the keywords list, so it stays as NAME with original case
    const grammar = makeCaseInsensitiveGrammar();
    const tokens = new GrammarLexer("myTable", grammar).tokenize();
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("myTable");
  });

  it("case-sensitive grammar (default) does not promote mixed-case identifier to KEYWORD", () => {
    // Build the same grammar without the @case_insensitive directive
    const source = [
      "NAME = /[a-zA-Z_][a-zA-Z0-9_]*/",
      "skip:",
      "  WS = /[ \\t\\r]+/",
      "keywords:",
      "  select",
    ].join("\n");
    const grammar = parseTokenGrammar(source);

    // 'SELECT' (all caps) should NOT match the lowercase keyword entry
    const tokens = new GrammarLexer("SELECT", grammar).tokenize();
    expect(tokens[0].type).toBe("NAME");
    expect(tokens[0].value).toBe("SELECT");

    // 'select' (exact match) should still be a KEYWORD
    const tokens2 = new GrammarLexer("select", grammar).tokenize();
    expect(tokens2[0].type).toBe("KEYWORD");
    expect(tokens2[0].value).toBe("select");
  });
});

// ============================================================================
// Rich source info tests
// ============================================================================

function makeRichSourceGrammar(): TokenGrammar {
  return {
    definitions: [
      { name: "NAME", pattern: "[a-zA-Z_][a-zA-Z0-9_]*", isRegex: true, lineNumber: 1 },
      { name: "EQ", pattern: "=", isRegex: false, lineNumber: 2 },
    ],
    keywords: [],
    skipDefinitions: [
      { name: "WHITESPACE", pattern: "[ \\t\\r\\n]+", isRegex: true, lineNumber: 3 },
      { name: "LINE_COMMENT", pattern: "\\/\\/[^\\n]*", isRegex: true, lineNumber: 4 },
    ],
    version: 0,
    caseInsensitive: false,
  };
}

describe("GrammarLexer — rich source preservation", () => {
  it("keeps the default path lean when preserveSourceInfo is disabled", () => {
    const grammar = makeRichSourceGrammar();
    const tokens = grammarTokenize("foo=bar", grammar);
    expect(tokens[0].startOffset).toBeUndefined();
    expect(tokens[0].leadingTrivia).toBeUndefined();
    expect(tokens[0].tokenIndex).toBeUndefined();
  });

  it("attaches leading trivia, offsets, and token indices when enabled", () => {
    const grammar = makeRichSourceGrammar();
    const tokens = grammarTokenize(
      "  // lead\nfoo=bar",
      grammar,
      { preserveSourceInfo: true },
    );

    expect(tokens.map((token) => token.type)).toEqual(["NAME", "EQ", "NAME", "EOF"]);

    expect(tokens[0].tokenIndex).toBe(0);
    expect(tokens[1].tokenIndex).toBe(1);
    expect(tokens[2].tokenIndex).toBe(2);
    expect(tokens[3].tokenIndex).toBe(3);

    expect(tokens[0].startOffset).toBe(10);
    expect(tokens[0].endOffset).toBe(13);
    expect(tokens[0].line).toBe(2);
    expect(tokens[0].column).toBe(1);
    expect(tokens[0].endLine).toBe(2);
    expect(tokens[0].endColumn).toBe(4);

    expect(tokens[0].leadingTrivia).toEqual([
      {
        type: "WHITESPACE",
        value: "  ",
        line: 1,
        column: 1,
        endLine: 1,
        endColumn: 3,
        startOffset: 0,
        endOffset: 2,
      },
      {
        type: "LINE_COMMENT",
        value: "// lead",
        line: 1,
        column: 3,
        endLine: 1,
        endColumn: 10,
        startOffset: 2,
        endOffset: 9,
      },
      {
        type: "WHITESPACE",
        value: "\n",
        line: 1,
        column: 10,
        endLine: 2,
        endColumn: 1,
        startOffset: 9,
        endOffset: 10,
      },
    ]);
  });

  it("attaches trailing trivia to EOF when enabled", () => {
    const grammar = makeRichSourceGrammar();
    const tokens = grammarTokenize(
      "foo // tail",
      grammar,
      { preserveSourceInfo: true },
    );
    const eof = tokens[tokens.length - 1];
    expect(eof.type).toBe("EOF");
    expect(eof.leadingTrivia).toEqual([
      {
        type: "WHITESPACE",
        value: " ",
        line: 1,
        column: 4,
        endLine: 1,
        endColumn: 5,
        startOffset: 3,
        endOffset: 4,
      },
      {
        type: "LINE_COMMENT",
        value: "// tail",
        line: 1,
        column: 5,
        endLine: 1,
        endColumn: 12,
        startOffset: 4,
        endOffset: 11,
      },
    ]);
  });
});
