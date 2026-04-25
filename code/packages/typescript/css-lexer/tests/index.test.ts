import { describe, expect, it } from "vitest";
import { CssLexerError, createCssLexer, tokenizeCss } from "../src/index.js";

const nonEof = (source: string) => tokenizeCss(source).filter((token) => token.type !== "EOF");
const types = (source: string) => nonEof(source).map((token) => token.type);
const pairs = (source: string) => nonEof(source).map((token) => [token.type, token.value]);

describe("tokenizeCss", () => {
  it("tokenizes basic identifiers, numbers, strings, hashes, and at-keywords", () => {
    expect(pairs("color 42 3.14 .5 -42")).toEqual([
      ["IDENT", "color"],
      ["NUMBER", "42"],
      ["NUMBER", "3.14"],
      ["NUMBER", ".5"],
      ["NUMBER", "-42"],
    ]);
    expect(pairs('"hello\\nworld"')).toEqual([["STRING", "hello\\nworld"]]);
    expect(pairs("'world'")).toEqual([["STRING", "world"]]);
    expect(pairs("#fff #header @media @-webkit-keyframes")).toEqual([
      ["HASH", "#fff"],
      ["HASH", "#header"],
      ["AT_KEYWORD", "@media"],
      ["AT_KEYWORD", "@-webkit-keyframes"],
    ]);
  });

  it("keeps compound CSS values as single tokens", () => {
    expect(pairs("10px 2em 1.5rem -20px 50% -10% 10 px")).toEqual([
      ["DIMENSION", "10px"],
      ["DIMENSION", "2em"],
      ["DIMENSION", "1.5rem"],
      ["DIMENSION", "-20px"],
      ["PERCENTAGE", "50%"],
      ["PERCENTAGE", "-10%"],
      ["NUMBER", "10"],
      ["IDENT", "px"],
    ]);
    expect(types("1e10")[0]).toBe("DIMENSION");
  });

  it("handles functions, urls, custom properties, and unicode ranges", () => {
    expect(pairs("rgb( calc( linear-gradient( url(image.jpg) --main-color U+0025-00FF U+4??")).toEqual([
      ["FUNCTION", "rgb("],
      ["FUNCTION", "calc("],
      ["FUNCTION", "linear-gradient("],
      ["URL_TOKEN", "url(image.jpg)"],
      ["CUSTOM_PROPERTY", "--main-color"],
      ["UNICODE_RANGE", "U+0025-00FF"],
      ["UNICODE_RANGE", "U+4??"],
    ]);
  });

  it("matches multi-character operators before single-character delimiters", () => {
    expect(types(":: ~= |= ^= $= *= { } ( ) [ ] ; : , . + > ~ * | ! / = & -")).toEqual([
      "COLON_COLON",
      "TILDE_EQUALS",
      "PIPE_EQUALS",
      "CARET_EQUALS",
      "DOLLAR_EQUALS",
      "STAR_EQUALS",
      "LBRACE",
      "RBRACE",
      "LPAREN",
      "RPAREN",
      "LBRACKET",
      "RBRACKET",
      "SEMICOLON",
      "COLON",
      "COMMA",
      "DOT",
      "PLUS",
      "GREATER",
      "TILDE",
      "STAR",
      "PIPE",
      "BANG",
      "SLASH",
      "EQUALS",
      "AMPERSAND",
      "MINUS",
    ]);
  });

  it("skips whitespace and comments", () => {
    expect(types("h1 /* selector */ {\n  color: red;\n}")).toEqual([
      "IDENT",
      "LBRACE",
      "IDENT",
      "COLON",
      "IDENT",
      "SEMICOLON",
      "RBRACE",
    ]);
    expect(tokenizeCss("/* only */").map((token) => token.type)).toEqual(["EOF"]);
  });

  it("tokenizes realistic CSS snippets and tracks positions", () => {
    const tokens = tokenizeCss("h1 {\n  color: #333;\n  width: calc(100% - 20px);\n}");
    expect(tokens[0]).toMatchObject({ type: "IDENT", value: "h1", line: 1, column: 1 });
    expect(types("color: rgb(255, 0, 0);")).toEqual([
      "IDENT",
      "COLON",
      "FUNCTION",
      "NUMBER",
      "COMMA",
      "NUMBER",
      "COMMA",
      "NUMBER",
      "RPAREN",
      "SEMICOLON",
    ]);
    const color = tokens.find((token) => token.value === "color");
    expect(color).toMatchObject({ line: 2, column: 3 });
  });

  it("emits error tokens where the grammar recovers", () => {
    expect(types('"unclosed string')).toEqual(["BAD_STRING"]);
    expect(pairs("url(unclosed")).toEqual([
      ["FUNCTION", "url("],
      ["IDENT", "unclosed"],
    ]);
  });

  it("supports the lexer factory and throws on truly unknown characters", () => {
    const lexer = createCssLexer("a { }");
    expect(lexer.tokenize().at(-1)?.type).toBe("EOF");
    expect(() => tokenizeCss("`")).toThrow(CssLexerError);
  });
});
