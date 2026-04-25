import { describe, expect, it } from "vitest";
import { createLispLexer, tokenizeLisp } from "../src/index.js";

describe("Lisp lexer", () => {
  it("tokenizes a basic definition", () => {
    const tokens = tokenizeLisp("(define x 42)");
    expect(tokens.map((token) => token.type)).toEqual([
      "LPAREN",
      "SYMBOL",
      "SYMBOL",
      "NUMBER",
      "RPAREN",
      "EOF",
    ]);
    expect(tokens.map((token) => token.value)).toEqual([
      "(",
      "define",
      "x",
      "42",
      ")",
      "",
    ]);
  });

  it("keeps operator names as symbols", () => {
    const tokens = tokenizeLisp("(+ -42 (* x 2))");
    expect(tokens[1]).toMatchObject({ type: "SYMBOL", value: "+" });
    expect(tokens[2]).toMatchObject({ type: "NUMBER", value: "-42" });
    expect(tokens[4]).toMatchObject({ type: "SYMBOL", value: "*" });
  });

  it("skips comments and tokenizes dotted pairs and quotes", () => {
    const tokens = tokenizeLisp("; ignore me\n'(a . b)");
    const types = tokens.map((token) => token.type);
    expect(types).toEqual([
      "QUOTE",
      "LPAREN",
      "SYMBOL",
      "DOT",
      "SYMBOL",
      "RPAREN",
      "EOF",
    ]);
  });

  it("creates a configurable lexer instance", () => {
    const lexer = createLispLexer('"hello\\nworld"');
    const tokens = lexer.tokenize();
    expect(tokens[0]).toMatchObject({
      type: "STRING",
      value: "hello\\nworld",
    });
  });
});
