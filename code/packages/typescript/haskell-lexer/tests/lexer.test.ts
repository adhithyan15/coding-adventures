import { describe, expect, it } from "vitest";
import { createHaskellLexer, tokenizeHaskell } from "../src/index.js";

describe("Haskell lexer", () => {
  it("tokenizes with the default 2010 grammar", () => {
    const lexer = createHaskellLexer("x");
    const tokens = lexer.tokenize();
    expect(tokens[0].type).toBe("NAME");
  });

  it("emits virtual layout tokens", () => {
    const tokens = tokenizeHaskell("let\n  x = y\nin x");
    const types = tokens.map((token) => token.type);
    expect(types).toContain("VIRTUAL_LBRACE");
    expect(types).toContain("VIRTUAL_RBRACE");
  });

  it("supports historical grammar versions", () => {
    const tokens = tokenizeHaskell("x", "98");
    expect(tokens[0].type).toBe("NAME");
  });
});
