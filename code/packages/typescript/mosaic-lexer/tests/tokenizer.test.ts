/**
 * Tests for the Mosaic Lexer (TypeScript).
 *
 * These tests verify that the grammar-driven lexer correctly tokenizes Mosaic
 * source text. The lexer must handle all token types defined in `mosaic.tokens`:
 * keywords, identifiers, string/number/dimension/color literals, delimiters,
 * and the `@` slot reference sigil.
 *
 * Token Type Conventions in This Codebase
 * ----------------------------------------
 *
 * Identifier tokens are named `NAME` (not `IDENT`). This follows the convention
 * established by the starlark-lexer and python-lexer packages. The generic
 * grammar engine only performs keyword reclassification for tokens named `NAME`:
 * when a `NAME` token's value matches an entry in the `keywords:` list, the
 * type is promoted from `NAME` to `KEYWORD`.
 *
 * String values do NOT include surrounding quotes — the lexer strips the quote
 * characters and returns the raw content.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Keywords** — component, slot, import, from, as, text, number, bool,
 *      image, color, node, list, true, false, when, each
 *   2. **Identifiers** — simple names, hyphenated names (corner-radius, a11y-label)
 *   3. **String literals** — double-quoted strings
 *   4. **Numeric literals** — integers, negatives, decimals
 *   5. **Dimension literals** — number + unit (16dp, 100%, 1.5sp)
 *   6. **Color hex literals** — #rgb, #rrggbb, #rrggbbaa
 *   7. **Delimiters** — { } < > : ; , . = @
 *   8. **Skip patterns** — line comments, block comments, whitespace
 *   9. **Complete component** — full tokenization of a realistic component
 *  10. **When and each blocks** — conditional and iteration keywords
 *  11. **Import declarations** — import...from...as syntax
 *  12. **Position tracking** — line and column numbers
 *  13. **DIMENSION before NUMBER ordering** — "16dp" must not split into NUMBER + NAME
 *  14. **List type syntax** — list<text>, list<Button>
 *  15. **Enum value syntax** — align.center, heading.small
 */

import { describe, it, expect } from "vitest";
import { tokenizeMosaic } from "../src/tokenizer.js";

/**
 * Helper: extract just the type and value from each token.
 * This makes test assertions more readable.
 */
function typesAndValues(source: string) {
  return tokenizeMosaic(source).map((t) => ({ type: t.type, value: t.value }));
}

// ============================================================================
// 1. Keywords
// ============================================================================

describe("keywords", () => {
  it("tokenizes 'component' as KEYWORD", () => {
    const tokens = typesAndValues("component");
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "component" });
  });

  it("tokenizes 'slot' as KEYWORD", () => {
    expect(typesAndValues("slot")[0]).toEqual({ type: "KEYWORD", value: "slot" });
  });

  it("tokenizes 'import' as KEYWORD", () => {
    expect(typesAndValues("import")[0]).toEqual({ type: "KEYWORD", value: "import" });
  });

  it("tokenizes 'from' as KEYWORD", () => {
    expect(typesAndValues("from")[0]).toEqual({ type: "KEYWORD", value: "from" });
  });

  it("tokenizes 'as' as KEYWORD", () => {
    expect(typesAndValues("as")[0]).toEqual({ type: "KEYWORD", value: "as" });
  });

  it("tokenizes all primitive type keywords", () => {
    const keywords = ["text", "number", "bool", "image", "color", "node", "list"];
    for (const kw of keywords) {
      expect(typesAndValues(kw)[0]).toEqual({ type: "KEYWORD", value: kw });
    }
  });

  it("tokenizes 'true' and 'false' as KEYWORD", () => {
    expect(typesAndValues("true")[0]).toEqual({ type: "KEYWORD", value: "true" });
    expect(typesAndValues("false")[0]).toEqual({ type: "KEYWORD", value: "false" });
  });

  it("tokenizes 'when' and 'each' as KEYWORD", () => {
    expect(typesAndValues("when")[0]).toEqual({ type: "KEYWORD", value: "when" });
    expect(typesAndValues("each")[0]).toEqual({ type: "KEYWORD", value: "each" });
  });

  it("does not treat keyword prefix as keyword — 'component2' is NAME", () => {
    // Identifiers that start with a keyword but have extra characters stay as NAME.
    expect(typesAndValues("component2")[0]).toEqual({ type: "NAME", value: "component2" });
  });
});

// ============================================================================
// 2. Identifiers (NAME tokens)
// ============================================================================

describe("identifiers", () => {
  it("tokenizes simple identifier as NAME", () => {
    expect(typesAndValues("Button")[0]).toEqual({ type: "NAME", value: "Button" });
  });

  it("tokenizes underscore-prefixed identifier as NAME", () => {
    expect(typesAndValues("_private")[0]).toEqual({ type: "NAME", value: "_private" });
  });

  it("tokenizes hyphenated identifier as single NAME token", () => {
    // CSS-like property names are valid in Mosaic: corner-radius, a11y-label, etc.
    // The NAME pattern allows hyphens: [a-zA-Z_][a-zA-Z0-9_-]*
    expect(typesAndValues("corner-radius")[0]).toEqual({ type: "NAME", value: "corner-radius" });
  });

  it("tokenizes 'a11y-label' (alphanumeric + hyphen) as NAME", () => {
    expect(typesAndValues("a11y-label")[0]).toEqual({ type: "NAME", value: "a11y-label" });
  });

  it("tokenizes 'display-name' as single NAME token", () => {
    expect(typesAndValues("display-name")[0]).toEqual({ type: "NAME", value: "display-name" });
  });
});

// ============================================================================
// 3. String Literals
// ============================================================================
// Note: the lexer strips surrounding quotes from STRING values.
// The token value for '"hello"' is 'hello', not '"hello"'.

describe("string literals", () => {
  it("tokenizes simple double-quoted string (value has no quotes)", () => {
    expect(typesAndValues('"hello"')[0]).toEqual({ type: "STRING", value: "hello" });
  });

  it("tokenizes empty string", () => {
    expect(typesAndValues('""')[0]).toEqual({ type: "STRING", value: "" });
  });

  it("tokenizes string with spaces and punctuation", () => {
    expect(typesAndValues('"hello, world!"')[0]).toEqual({
      type: "STRING",
      value: "hello, world!",
    });
  });

  it("tokenizes file path string", () => {
    expect(typesAndValues('"./button.mosaic"')[0]).toEqual({
      type: "STRING",
      value: "./button.mosaic",
    });
  });

  it("type is STRING", () => {
    expect(typesAndValues('"any string"')[0].type).toBe("STRING");
  });
});

// ============================================================================
// 4. Numeric Literals
// ============================================================================

describe("number literals", () => {
  it("tokenizes integer", () => {
    expect(typesAndValues("42")[0]).toEqual({ type: "NUMBER", value: "42" });
  });

  it("tokenizes negative integer", () => {
    expect(typesAndValues("-5")[0]).toEqual({ type: "NUMBER", value: "-5" });
  });

  it("tokenizes decimal", () => {
    expect(typesAndValues("3.14")[0]).toEqual({ type: "NUMBER", value: "3.14" });
  });

  it("tokenizes zero", () => {
    expect(typesAndValues("0")[0]).toEqual({ type: "NUMBER", value: "0" });
  });
});

// ============================================================================
// 5. Dimension Literals
// ============================================================================

describe("dimension literals", () => {
  it("tokenizes dp dimension", () => {
    expect(typesAndValues("16dp")[0]).toEqual({ type: "DIMENSION", value: "16dp" });
  });

  it("tokenizes sp dimension", () => {
    expect(typesAndValues("14sp")[0]).toEqual({ type: "DIMENSION", value: "14sp" });
  });

  it("tokenizes percent dimension", () => {
    expect(typesAndValues("100%")[0]).toEqual({ type: "DIMENSION", value: "100%" });
  });

  it("tokenizes fractional dp dimension", () => {
    expect(typesAndValues("1.5dp")[0]).toEqual({ type: "DIMENSION", value: "1.5dp" });
  });

  it("does NOT split '16dp' into NUMBER + NAME — DIMENSION wins", () => {
    // This is the critical ordering test: DIMENSION must come before NUMBER
    // in the grammar definitions. Without this ordering, "16dp" would produce
    // two tokens: NUMBER("16") and NAME("dp").
    const tokens = typesAndValues("16dp");
    const nonEof = tokens.filter((t) => t.type !== "EOF");
    expect(nonEof).toHaveLength(1);
    expect(nonEof[0]).toEqual({ type: "DIMENSION", value: "16dp" });
  });
});

// ============================================================================
// 6. Color Hex Literals
// ============================================================================

describe("color hex literals", () => {
  it("tokenizes 3-digit hex color", () => {
    expect(typesAndValues("#fff")[0]).toEqual({ type: "COLOR_HEX", value: "#fff" });
  });

  it("tokenizes 6-digit hex color", () => {
    expect(typesAndValues("#2563eb")[0]).toEqual({ type: "COLOR_HEX", value: "#2563eb" });
  });

  it("tokenizes 8-digit hex color (with alpha)", () => {
    expect(typesAndValues("#2563ebff")[0]).toEqual({ type: "COLOR_HEX", value: "#2563ebff" });
  });

  it("tokenizes uppercase hex color", () => {
    expect(typesAndValues("#FFFFFF")[0]).toEqual({ type: "COLOR_HEX", value: "#FFFFFF" });
  });
});

// ============================================================================
// 7. Delimiters
// ============================================================================

describe("delimiters", () => {
  const cases: Array<[string, string]> = [
    ["{", "LBRACE"],
    ["}", "RBRACE"],
    ["<", "LANGLE"],
    [">", "RANGLE"],
    [":", "COLON"],
    [";", "SEMICOLON"],
    [",", "COMMA"],
    [".", "DOT"],
    ["=", "EQUALS"],
    ["@", "AT"],
  ];

  for (const [src, type] of cases) {
    it(`tokenizes '${src}' as ${type}`, () => {
      expect(typesAndValues(src)[0]).toEqual({ type, value: src });
    });
  }
});

// ============================================================================
// 8. Skip Patterns
// ============================================================================

describe("skip patterns", () => {
  it("skips line comments — no comment token in output", () => {
    const tokens = typesAndValues("// this is a comment\nslot");
    expect(tokens.filter((t) => t.type !== "EOF")).toHaveLength(1);
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "slot" });
  });

  it("skips block comments", () => {
    const tokens = typesAndValues("/* block */ slot");
    expect(tokens.filter((t) => t.type !== "EOF")).toHaveLength(1);
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "slot" });
  });

  it("skips all whitespace between tokens", () => {
    const tokens = typesAndValues("  slot  \t  label  ");
    const nonEof = tokens.filter((t) => t.type !== "EOF");
    expect(nonEof).toHaveLength(2);
    expect(nonEof[0]).toEqual({ type: "KEYWORD", value: "slot" });
    expect(nonEof[1]).toEqual({ type: "NAME", value: "label" });
  });

  it("skips newlines between tokens", () => {
    const tokens = typesAndValues("component\nButton");
    const nonEof = tokens.filter((t) => t.type !== "EOF");
    expect(nonEof).toHaveLength(2);
    expect(nonEof[0]).toEqual({ type: "KEYWORD", value: "component" });
    expect(nonEof[1]).toEqual({ type: "NAME", value: "Button" });
  });
});

// ============================================================================
// 9. Complete Component
// ============================================================================

describe("complete component", () => {
  const source = `
    component ProfileCard {
      slot avatar-url: image;
      slot display-name: text;
      slot count: number = 0;

      Column {
        Text { content: @display-name; }
      }
    }
  `;

  it("tokenizes without throwing", () => {
    expect(() => tokenizeMosaic(source)).not.toThrow();
  });

  it("starts with component keyword", () => {
    const tokens = tokenizeMosaic(source);
    expect(tokens[0]).toMatchObject({ type: "KEYWORD", value: "component" });
  });

  it("includes all expected token types", () => {
    const tokens = tokenizeMosaic(source);
    const types = new Set(tokens.map((t) => t.type));
    expect(types).toContain("KEYWORD");
    expect(types).toContain("NAME");
    expect(types).toContain("COLON");
    expect(types).toContain("SEMICOLON");
    expect(types).toContain("LBRACE");
    expect(types).toContain("RBRACE");
    expect(types).toContain("AT");
    expect(types).toContain("NUMBER");
    expect(types).toContain("EQUALS");
  });

  it("tokenizes hyphenated slot names as single NAME token", () => {
    const tokens = tokenizeMosaic(source);
    const avatarUrl = tokens.find((t) => t.value === "avatar-url");
    expect(avatarUrl).toBeDefined();
    expect(avatarUrl!.type).toBe("NAME");
  });

  it("tokenizes slot reference @display-name as AT + NAME", () => {
    const tokens = tokenizeMosaic(source);
    const atIdx = tokens.findIndex((t) => t.type === "AT");
    expect(atIdx).toBeGreaterThan(-1);
    expect(tokens[atIdx + 1]).toMatchObject({ type: "NAME", value: "display-name" });
  });

  it("ends with EOF", () => {
    const tokens = tokenizeMosaic(source);
    expect(tokens[tokens.length - 1]).toMatchObject({ type: "EOF" });
  });
});

// ============================================================================
// 10. When and Each Blocks
// ============================================================================

describe("when and each blocks", () => {
  it("tokenizes when block header", () => {
    const tokens = typesAndValues("when @visible {");
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "when" });
    expect(tokens[1]).toEqual({ type: "AT", value: "@" });
    expect(tokens[2]).toEqual({ type: "NAME", value: "visible" });
    expect(tokens[3]).toEqual({ type: "LBRACE", value: "{" });
  });

  it("tokenizes each block header", () => {
    const tokens = typesAndValues("each @items as item {");
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "each" });
    expect(tokens[1]).toEqual({ type: "AT", value: "@" });
    expect(tokens[2]).toEqual({ type: "NAME", value: "items" });
    expect(tokens[3]).toEqual({ type: "KEYWORD", value: "as" });
    expect(tokens[4]).toEqual({ type: "NAME", value: "item" });
    expect(tokens[5]).toEqual({ type: "LBRACE", value: "{" });
  });
});

// ============================================================================
// 11. Import Declarations
// ============================================================================

describe("import declarations", () => {
  it("tokenizes simple import", () => {
    const tokens = typesAndValues('import Button from "./button.mosaic";');
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "import" });
    expect(tokens[1]).toEqual({ type: "NAME", value: "Button" });
    expect(tokens[2]).toEqual({ type: "KEYWORD", value: "from" });
    // String value has quotes stripped.
    expect(tokens[3]).toEqual({ type: "STRING", value: "./button.mosaic" });
    expect(tokens[4]).toEqual({ type: "SEMICOLON", value: ";" });
  });

  it("tokenizes aliased import", () => {
    const tokens = typesAndValues('import Card as InfoCard from "./info.mosaic";');
    expect(tokens[2]).toEqual({ type: "KEYWORD", value: "as" });
    expect(tokens[3]).toEqual({ type: "NAME", value: "InfoCard" });
  });
});

// ============================================================================
// 12. Position Tracking
// ============================================================================

describe("position tracking", () => {
  it("tracks line number for first token", () => {
    const tokens = tokenizeMosaic("component Button {");
    expect(tokens[0].line).toBe(1);
  });

  it("tracks column for first token", () => {
    const tokens = tokenizeMosaic("component Button {");
    expect(tokens[0].column).toBe(1);
  });

  it("increments line number across newlines", () => {
    const tokens = tokenizeMosaic("component\nButton");
    const buttonToken = tokens.find((t) => t.value === "Button");
    expect(buttonToken?.line).toBe(2);
  });
});

// ============================================================================
// 13. List Type Syntax
// ============================================================================

describe("list type syntax", () => {
  it("tokenizes 'list<text>' correctly", () => {
    const tokens = typesAndValues("list<text>");
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "list" });
    expect(tokens[1]).toEqual({ type: "LANGLE", value: "<" });
    expect(tokens[2]).toEqual({ type: "KEYWORD", value: "text" });
    expect(tokens[3]).toEqual({ type: "RANGLE", value: ">" });
  });

  it("tokenizes 'list<Button>' with component type", () => {
    const tokens = typesAndValues("list<Button>");
    expect(tokens[0]).toEqual({ type: "KEYWORD", value: "list" });
    expect(tokens[2]).toEqual({ type: "NAME", value: "Button" });
  });
});

// ============================================================================
// 14. Enum Value Syntax
// ============================================================================

describe("enum value syntax (NAME.NAME)", () => {
  it("tokenizes 'align.center' as NAME DOT NAME", () => {
    const tokens = typesAndValues("align.center");
    expect(tokens[0]).toEqual({ type: "NAME", value: "align" });
    expect(tokens[1]).toEqual({ type: "DOT", value: "." });
    expect(tokens[2]).toEqual({ type: "NAME", value: "center" });
  });

  it("tokenizes 'heading.small' as NAME DOT NAME", () => {
    const tokens = typesAndValues("heading.small");
    expect(tokens[0]).toEqual({ type: "NAME", value: "heading" });
    expect(tokens[1]).toEqual({ type: "DOT", value: "." });
    expect(tokens[2]).toEqual({ type: "NAME", value: "small" });
  });
});
