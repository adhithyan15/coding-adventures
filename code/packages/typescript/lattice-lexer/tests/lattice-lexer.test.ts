/**
 * Tests for @coding-adventures/lattice-lexer
 *
 * These tests verify that the Lattice lexer correctly tokenizes all token
 * types defined in lattice.tokens, including CSS tokens and the 5 Lattice
 * extensions (VARIABLE, EQUALS_EQUALS, NOT_EQUALS, GREATER_EQUALS, LESS_EQUALS).
 *
 * The tests follow the structure of the grammar file, verifying each
 * category of tokens in turn.
 */

import { describe, it, expect } from "vitest";
import { tokenizeLatticeLexer, createLatticeLexer } from "../src/index.js";
import type { Token } from "../src/index.js";

// Helper: get token types without EOF
function types(tokens: Token[]): string[] {
  return tokens.slice(0, -1).map((t) => t.type);
}

// Helper: get token values without EOF
function values(tokens: Token[]): string[] {
  return tokens.slice(0, -1).map((t) => t.value);
}

describe("lattice-lexer", () => {
  // =========================================================================
  // Basic API
  // =========================================================================

  describe("API", () => {
    it("tokenizeLatticeLexer returns Token array ending with EOF", () => {
      const tokens = tokenizeLatticeLexer("$x: 1;");
      expect(tokens.length).toBeGreaterThan(0);
      expect(tokens[tokens.length - 1].type).toBe("EOF");
      expect(tokens[tokens.length - 1].value).toBe("");
    });

    it("createLatticeLexer returns GrammarLexer with tokenize()", () => {
      const lexer = createLatticeLexer("$x: 1;");
      expect(typeof lexer.tokenize).toBe("function");
      const tokens = lexer.tokenize();
      expect(tokens[tokens.length - 1].type).toBe("EOF");
    });

    it("createLatticeLexer and tokenizeLatticeLexer produce same results", () => {
      const src = "$color: #4a90d9;";
      const t1 = tokenizeLatticeLexer(src);
      const t2 = createLatticeLexer(src).tokenize();
      expect(t1).toEqual(t2);
    });

    it("empty string produces only EOF", () => {
      const tokens = tokenizeLatticeLexer("");
      expect(tokens).toHaveLength(1);
      expect(tokens[0].type).toBe("EOF");
    });
  });

  // =========================================================================
  // Lattice Extensions: VARIABLE
  // =========================================================================

  describe("VARIABLE tokens", () => {
    it("tokenizes simple variable", () => {
      const tokens = tokenizeLatticeLexer("$color");
      expect(types(tokens)).toEqual(["VARIABLE"]);
      expect(values(tokens)).toEqual(["$color"]);
    });

    it("tokenizes variable with hyphens", () => {
      const tokens = tokenizeLatticeLexer("$font-size");
      expect(types(tokens)).toEqual(["VARIABLE"]);
      expect(values(tokens)).toEqual(["$font-size"]);
    });

    it("tokenizes variable with underscores", () => {
      const tokens = tokenizeLatticeLexer("$base_color");
      expect(types(tokens)).toEqual(["VARIABLE"]);
      expect(values(tokens)).toEqual(["$base_color"]);
    });

    it("tokenizes variable with uppercase letters", () => {
      const tokens = tokenizeLatticeLexer("$MyVar");
      expect(types(tokens)).toEqual(["VARIABLE"]);
      expect(values(tokens)).toEqual(["$MyVar"]);
    });

    it("tokenizes variable in declaration", () => {
      const tokens = tokenizeLatticeLexer("$color: red;");
      expect(types(tokens)).toEqual(["VARIABLE", "COLON", "IDENT", "SEMICOLON"]);
      expect(values(tokens)).toEqual(["$color", ":", "red", ";"]);
    });

    it("tokenizes variable in property value", () => {
      const tokens = tokenizeLatticeLexer("color: $primary;");
      expect(types(tokens)).toEqual(["IDENT", "COLON", "VARIABLE", "SEMICOLON"]);
    });
  });

  // =========================================================================
  // Lattice Extensions: Comparison Operators
  // =========================================================================

  describe("comparison operators", () => {
    it("tokenizes == as EQUALS_EQUALS", () => {
      const tokens = tokenizeLatticeLexer("==");
      expect(types(tokens)).toEqual(["EQUALS_EQUALS"]);
      expect(values(tokens)).toEqual(["=="]);
    });

    it("tokenizes != as NOT_EQUALS", () => {
      const tokens = tokenizeLatticeLexer("!=");
      expect(types(tokens)).toEqual(["NOT_EQUALS"]);
      expect(values(tokens)).toEqual(["!="]);
    });

    it("tokenizes >= as GREATER_EQUALS", () => {
      const tokens = tokenizeLatticeLexer(">=");
      expect(types(tokens)).toEqual(["GREATER_EQUALS"]);
      expect(values(tokens)).toEqual([">="]);
    });

    it("tokenizes <= as LESS_EQUALS", () => {
      const tokens = tokenizeLatticeLexer("<=");
      expect(types(tokens)).toEqual(["LESS_EQUALS"]);
      expect(values(tokens)).toEqual(["<="]);
    });

    it("does not confuse == with single =", () => {
      const tokens = tokenizeLatticeLexer("=");
      expect(types(tokens)).toEqual(["EQUALS"]);
    });

    it("tokenizes != before ! and = separately", () => {
      // != must match as a unit, not as BANG then EQUALS
      const tokens = tokenizeLatticeLexer("a != b");
      expect(types(tokens)).toContain("NOT_EQUALS");
    });

    it("tokenizes comparison in @if expression", () => {
      const tokens = tokenizeLatticeLexer("$theme == dark");
      expect(types(tokens)).toEqual(["VARIABLE", "EQUALS_EQUALS", "IDENT"]);
    });
  });

  // =========================================================================
  // CSS Tokens: Numbers
  // =========================================================================

  describe("numeric tokens", () => {
    it("tokenizes integer as NUMBER", () => {
      const tokens = tokenizeLatticeLexer("42");
      expect(types(tokens)).toEqual(["NUMBER"]);
      expect(values(tokens)).toEqual(["42"]);
    });

    it("tokenizes decimal as NUMBER", () => {
      const tokens = tokenizeLatticeLexer("3.14");
      expect(types(tokens)).toEqual(["NUMBER"]);
      expect(values(tokens)).toEqual(["3.14"]);
    });

    it("tokenizes negative number", () => {
      const tokens = tokenizeLatticeLexer("-42");
      expect(types(tokens)).toEqual(["NUMBER"]);
      expect(values(tokens)).toEqual(["-42"]);
    });

    it("tokenizes dimension (number + unit) as DIMENSION", () => {
      const tokens = tokenizeLatticeLexer("16px");
      expect(types(tokens)).toEqual(["DIMENSION"]);
      expect(values(tokens)).toEqual(["16px"]);
    });

    it("tokenizes various CSS dimensions", () => {
      const cases = ["2em", "1.5rem", "100vh", "300ms", "2s"];
      for (const dim of cases) {
        const tokens = tokenizeLatticeLexer(dim);
        expect(types(tokens)).toEqual(["DIMENSION"]);
      }
    });

    it("tokenizes percentage as PERCENTAGE", () => {
      const tokens = tokenizeLatticeLexer("50%");
      expect(types(tokens)).toEqual(["PERCENTAGE"]);
      expect(values(tokens)).toEqual(["50%"]);
    });

    it("DIMENSION matches before PERCENTAGE and NUMBER (order matters)", () => {
      // "16px" should be DIMENSION, not NUMBER followed by IDENT
      const tokens = tokenizeLatticeLexer("16px");
      expect(tokens.length).toBe(2); // DIMENSION + EOF
      expect(tokens[0].type).toBe("DIMENSION");
    });
  });

  // =========================================================================
  // CSS Tokens: Strings
  // =========================================================================

  describe("string tokens", () => {
    it("tokenizes double-quoted string as STRING", () => {
      const tokens = tokenizeLatticeLexer('"hello"');
      expect(types(tokens)).toEqual(["STRING"]);
      // The lexer strips quotes (escapeMode: none strips outer quotes)
      expect(values(tokens)).toEqual(["hello"]);
    });

    it("tokenizes single-quoted string as STRING", () => {
      const tokens = tokenizeLatticeLexer("'world'");
      expect(types(tokens)).toEqual(["STRING"]);
      expect(values(tokens)).toEqual(["world"]);
    });

    it("tokenizes @use string", () => {
      const tokens = tokenizeLatticeLexer('@use "colors";');
      const tokenTypes = types(tokens);
      expect(tokenTypes).toContain("STRING");
    });
  });

  // =========================================================================
  // CSS Tokens: Identifiers
  // =========================================================================

  describe("identifier tokens", () => {
    it("tokenizes bare identifier as IDENT", () => {
      const tokens = tokenizeLatticeLexer("red");
      expect(types(tokens)).toEqual(["IDENT"]);
      expect(values(tokens)).toEqual(["red"]);
    });

    it("tokenizes CSS property name as IDENT", () => {
      const tokens = tokenizeLatticeLexer("color");
      expect(types(tokens)).toEqual(["IDENT"]);
    });

    it("tokenizes hyphenated identifier as IDENT", () => {
      const tokens = tokenizeLatticeLexer("font-size");
      expect(types(tokens)).toEqual(["IDENT"]);
      expect(values(tokens)).toEqual(["font-size"]);
    });

    it("tokenizes custom property as CUSTOM_PROPERTY", () => {
      const tokens = tokenizeLatticeLexer("--primary-color");
      expect(types(tokens)).toEqual(["CUSTOM_PROPERTY"]);
    });

    it("tokenizes hash as HASH", () => {
      const tokens = tokenizeLatticeLexer("#4a90d9");
      expect(types(tokens)).toEqual(["HASH"]);
      expect(values(tokens)).toEqual(["#4a90d9"]);
    });

    it("tokenizes short hex color as HASH", () => {
      const tokens = tokenizeLatticeLexer("#fff");
      expect(types(tokens)).toEqual(["HASH"]);
    });
  });

  // =========================================================================
  // CSS Tokens: At-Keywords
  // =========================================================================

  describe("at-keyword tokens", () => {
    const atKeywords = [
      "@media",
      "@import",
      "@keyframes",
      "@mixin",
      "@include",
      "@if",
      "@else",
      "@for",
      "@each",
      "@function",
      "@return",
      "@use",
    ];

    for (const kw of atKeywords) {
      it(`tokenizes ${kw} as AT_KEYWORD`, () => {
        const tokens = tokenizeLatticeLexer(kw);
        expect(types(tokens)).toEqual(["AT_KEYWORD"]);
        expect(values(tokens)).toEqual([kw]);
      });
    }
  });

  // =========================================================================
  // CSS Tokens: Functions
  // =========================================================================

  describe("function tokens", () => {
    it("tokenizes function call start as FUNCTION", () => {
      // "rgb(" is a single FUNCTION token — the lexer includes the "(" in the token value
      const tokens = tokenizeLatticeLexer("rgb(");
      expect(types(tokens)).toEqual(["FUNCTION"]);
      expect(tokens[0].value).toBe("rgb(");
    });

    it("FUNCTION token includes the opening paren", () => {
      const tokens = tokenizeLatticeLexer("rgb(255, 0, 0)");
      expect(tokens[0].type).toBe("FUNCTION");
      expect(tokens[0].value).toBe("rgb(");
    });

    it("tokenizes url() as URL_TOKEN", () => {
      const tokens = tokenizeLatticeLexer("url(image.png)");
      expect(types(tokens)).toEqual(["URL_TOKEN"]);
      expect(values(tokens)).toEqual(["url(image.png)"]);
    });
  });

  // =========================================================================
  // CSS Tokens: Delimiters
  // =========================================================================

  describe("delimiter tokens", () => {
    const delimiters: Array<[string, string]> = [
      ["{", "LBRACE"],
      ["}", "RBRACE"],
      ["(", "LPAREN"],
      [")", "RPAREN"],
      ["[", "LBRACKET"],
      ["]", "RBRACKET"],
      [";", "SEMICOLON"],
      [":", "COLON"],
      [",", "COMMA"],
      [".", "DOT"],
      ["+", "PLUS"],
      ["*", "STAR"],
      ["|", "PIPE"],
      ["!", "BANG"],
      ["/", "SLASH"],
      ["=", "EQUALS"],
      ["&", "AMPERSAND"],
      ["-", "MINUS"],
    ];

    for (const [char, tokenType] of delimiters) {
      it(`tokenizes '${char}' as ${tokenType}`, () => {
        const tokens = tokenizeLatticeLexer(char);
        expect(types(tokens)).toEqual([tokenType]);
        expect(values(tokens)).toEqual([char]);
      });
    }

    it("tokenizes :: as COLON_COLON", () => {
      const tokens = tokenizeLatticeLexer("::");
      expect(types(tokens)).toEqual(["COLON_COLON"]);
    });

    it("tokenizes > as GREATER (not GREATER_EQUALS)", () => {
      const tokens = tokenizeLatticeLexer(">");
      expect(types(tokens)).toEqual(["GREATER"]);
    });

    it("tokenizes ~ as TILDE", () => {
      const tokens = tokenizeLatticeLexer("~");
      expect(types(tokens)).toEqual(["TILDE"]);
    });
  });

  // =========================================================================
  // CSS Tokens: Attribute Matchers
  // =========================================================================

  describe("attribute matcher tokens", () => {
    const matchers: Array<[string, string]> = [
      ["~=", "TILDE_EQUALS"],
      ["|=", "PIPE_EQUALS"],
      ["^=", "CARET_EQUALS"],
      ["$=", "DOLLAR_EQUALS"],
      ["*=", "STAR_EQUALS"],
    ];

    for (const [op, tokenType] of matchers) {
      it(`tokenizes '${op}' as ${tokenType}`, () => {
        const tokens = tokenizeLatticeLexer(op);
        expect(types(tokens)).toEqual([tokenType]);
      });
    }
  });

  // =========================================================================
  // Comment Skipping
  // =========================================================================

  describe("comment skipping", () => {
    it("skips single-line // comments", () => {
      const tokens = tokenizeLatticeLexer("// this is a comment\n$x: 1;");
      expect(types(tokens)).toEqual(["VARIABLE", "COLON", "NUMBER", "SEMICOLON"]);
    });

    it("skips block /* */ comments", () => {
      const tokens = tokenizeLatticeLexer("/* comment */ $x: 1;");
      expect(types(tokens)).toEqual(["VARIABLE", "COLON", "NUMBER", "SEMICOLON"]);
    });

    it("skips comments between tokens", () => {
      const tokens = tokenizeLatticeLexer("$x /* mid */ : 1;");
      expect(types(tokens)).toEqual(["VARIABLE", "COLON", "NUMBER", "SEMICOLON"]);
    });

    it("skips whitespace (spaces, tabs, newlines)", () => {
      const tokens = tokenizeLatticeLexer("  $x  :  1  ;  ");
      expect(types(tokens)).toEqual(["VARIABLE", "COLON", "NUMBER", "SEMICOLON"]);
    });
  });

  // =========================================================================
  // Position Tracking
  // =========================================================================

  describe("position tracking", () => {
    it("tracks line and column for first token", () => {
      const tokens = tokenizeLatticeLexer("$color");
      expect(tokens[0].line).toBe(1);
      expect(tokens[0].column).toBe(1);
    });

    it("increments column for tokens on same line", () => {
      const tokens = tokenizeLatticeLexer("$x: 1;");
      expect(tokens[0].line).toBe(1);
      expect(tokens[0].column).toBe(1);
      // The colon comes after "$x" (2 chars)
      expect(tokens[1].line).toBe(1);
      expect(tokens[1].column).toBeGreaterThan(1);
    });
  });

  // =========================================================================
  // Full Lattice Snippets
  // =========================================================================

  describe("full snippet tokenization", () => {
    it("tokenizes a variable declaration", () => {
      const tokens = tokenizeLatticeLexer("$primary: #4a90d9;");
      expect(types(tokens)).toEqual(["VARIABLE", "COLON", "HASH", "SEMICOLON"]);
    });

    it("tokenizes a CSS rule", () => {
      const tokens = tokenizeLatticeLexer("h1 { color: red; }");
      expect(types(tokens)).toContain("IDENT");
      expect(types(tokens)).toContain("LBRACE");
      expect(types(tokens)).toContain("COLON");
      expect(types(tokens)).toContain("SEMICOLON");
      expect(types(tokens)).toContain("RBRACE");
    });

    it("tokenizes @mixin definition start", () => {
      const tokens = tokenizeLatticeLexer("@mixin button($bg)");
      // AT_KEYWORD(@mixin) FUNCTION(button() VARIABLE($bg) RPAREN
      expect(tokens[0].type).toBe("AT_KEYWORD");
      expect(tokens[0].value).toBe("@mixin");
      expect(tokens[1].type).toBe("FUNCTION");
      expect(tokens[1].value).toBe("button(");
    });

    it("tokenizes @if expression with comparison", () => {
      const tokens = tokenizeLatticeLexer("@if $theme == dark");
      expect(types(tokens)).toEqual([
        "AT_KEYWORD",
        "VARIABLE",
        "EQUALS_EQUALS",
        "IDENT",
      ]);
    });

    it("tokenizes @for directive", () => {
      const tokens = tokenizeLatticeLexer("@for $i from 1 through 12");
      expect(types(tokens)).toEqual([
        "AT_KEYWORD",
        "VARIABLE",
        "IDENT",
        "NUMBER",
        "IDENT",
        "NUMBER",
      ]);
    });

    it("tokenizes complex selector", () => {
      const tokens = tokenizeLatticeLexer(".btn:hover");
      // DOT IDENT COLON IDENT
      expect(types(tokens)).toEqual(["DOT", "IDENT", "COLON", "IDENT"]);
    });

    it("tokenizes pseudo-element selector", () => {
      const tokens = tokenizeLatticeLexer("p::before");
      // IDENT COLON_COLON IDENT
      expect(types(tokens)).toEqual(["IDENT", "COLON_COLON", "IDENT"]);
    });

    it("tokenizes !important", () => {
      const tokens = tokenizeLatticeLexer("!important");
      expect(types(tokens)).toEqual(["BANG", "IDENT"]);
      expect(values(tokens)).toEqual(["!", "important"]);
    });

    it("tokenizes multi-value declaration", () => {
      const tokens = tokenizeLatticeLexer("margin: 10px 20px;");
      expect(types(tokens)).toEqual([
        "IDENT", "COLON", "DIMENSION", "DIMENSION", "SEMICOLON",
      ]);
    });

    it("tokenizes @use directive", () => {
      const tokens = tokenizeLatticeLexer('@use "colors";');
      expect(tokens[0].type).toBe("AT_KEYWORD");
      expect(tokens[0].value).toBe("@use");
      expect(tokens[1].type).toBe("STRING");
      expect(tokens[2].type).toBe("SEMICOLON");
    });

    it("handles Lattice operators in context", () => {
      const src = "@if $size >= 16px";
      const tokens = tokenizeLatticeLexer(src);
      expect(types(tokens)).toEqual([
        "AT_KEYWORD", "VARIABLE", "GREATER_EQUALS", "DIMENSION",
      ]);
    });

    it("tokenizes nested block structure", () => {
      const src = ".nav { .item { color: red; } }";
      const tokens = tokenizeLatticeLexer(src);
      const tokenTypes = types(tokens);
      // Should have 2 LBRACE and 2 RBRACE for nested structure
      const braces = tokenTypes.filter((t) => t === "LBRACE" || t === "RBRACE");
      expect(braces).toHaveLength(4);
    });

    it("tokenizes calc function", () => {
      const tokens = tokenizeLatticeLexer("calc(100% - 20px)");
      expect(tokens[0].type).toBe("FUNCTION");
      expect(tokens[0].value).toBe("calc(");
    });

    it("tokenizes @media query", () => {
      const tokens = tokenizeLatticeLexer("@media (max-width: 768px)");
      expect(tokens[0].type).toBe("AT_KEYWORD");
      expect(tokens[0].value).toBe("@media");
    });

    it("tokenizes @function definition", () => {
      const tokens = tokenizeLatticeLexer("@function spacing($n)");
      expect(tokens[0].type).toBe("AT_KEYWORD");
      expect(tokens[0].value).toBe("@function");
      expect(tokens[1].type).toBe("FUNCTION");
    });

    it("tokenizes @return directive", () => {
      const tokens = tokenizeLatticeLexer("@return $n * 8px;");
      expect(tokens[0].type).toBe("AT_KEYWORD");
      expect(tokens[0].value).toBe("@return");
      expect(types(tokens)).toContain("STAR");
    });

    it("tokenizes @each directive", () => {
      const tokens = tokenizeLatticeLexer("@each $color in red, blue");
      expect(tokens[0].type).toBe("AT_KEYWORD");
      expect(tokens[0].value).toBe("@each");
    });

    it("handles line comment at end of file", () => {
      const tokens = tokenizeLatticeLexer("$x: 1; // end");
      const last = tokens[tokens.length - 1];
      expect(last.type).toBe("EOF");
    });

    it("tokenizes @include with arguments", () => {
      const tokens = tokenizeLatticeLexer("@include button(red, white);");
      expect(tokens[0].type).toBe("AT_KEYWORD");
      expect(tokens[0].value).toBe("@include");
      expect(tokens[1].type).toBe("FUNCTION");
      expect(tokens[1].value).toBe("button(");
    });
  });
});
