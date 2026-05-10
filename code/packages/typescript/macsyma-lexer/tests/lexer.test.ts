import { describe, expect, it } from "vitest";
import { tokenizeMacsyma } from "../src/index.js";

function typesOf(source: string): string[] {
  return tokenizeMacsyma(source).filter((t) => t.type !== "EOF").map((t) => t.type);
}

function valuesOf(source: string): string[] {
  return tokenizeMacsyma(source).filter((t) => t.type !== "EOF").map((t) => t.value);
}

describe("macsyma lexer", () => {
  it("tokenizes numbers, names, percent constants, and history references", () => {
    expect(typesOf("42 3.14 1.5e10 x %pi % %i1 %o2")).toEqual([
      "NUMBER", "NUMBER", "NUMBER", "NAME", "NAME", "NAME", "NAME", "NAME",
    ]);
    expect(valuesOf("%pi % %i1 %o2")).toEqual(["%pi", "%", "%i1", "%o2"]);
  });

  it("uses the compiled grammar's longest-match operator order", () => {
    expect(typesOf("f(x) := x ** 2 <= y >= z -> q")).toEqual([
      "NAME", "LPAREN", "NAME", "RPAREN", "COLONEQ", "NAME", "STAREQ",
      "NUMBER", "LEQ", "NAME", "GEQ", "NAME", "ARROW", "NAME",
    ]);
  });

  it("promotes MACSYMA keywords to KEYWORD", () => {
    const tokens = tokenizeMacsyma("x and y or not false");
    expect(tokens.filter((t) => t.type !== "EOF").map((t) => [t.type, t.value])).toEqual([
      ["NAME", "x"],
      ["KEYWORD", "and"],
      ["NAME", "y"],
      ["KEYWORD", "or"],
      ["KEYWORD", "not"],
      ["KEYWORD", "false"],
    ]);
  });

  it("skips whitespace and comments from the compiled skip grammar", () => {
    expect(typesOf("/* ignored */\n x + y")).toEqual(["NAME", "PLUS", "NAME"]);
  });

  it("preserves distinct statement terminators", () => {
    expect(typesOf("x; y$")).toEqual(["NAME", "SEMI", "NAME", "DOLLAR"]);
  });
});
