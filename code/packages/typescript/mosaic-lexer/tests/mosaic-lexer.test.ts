import { describe, it, expect } from "vitest";
import { tokenizeMosaic, TOKEN_GRAMMAR } from "../src/index.js";

describe("mosaic-lexer exports", () => {
  it("exports tokenizeMosaic function", () => {
    expect(typeof tokenizeMosaic).toBe("function");
  });

  it("exports TOKEN_GRAMMAR constant", () => {
    expect(TOKEN_GRAMMAR).toBeDefined();
    expect(TOKEN_GRAMMAR.keywords).toBeDefined();
  });

  it("TOKEN_GRAMMAR contains all 16 keywords", () => {
    const keywords = TOKEN_GRAMMAR.keywords;
    const expected = [
      "component", "slot", "import", "from", "as",
      "text", "number", "bool", "image", "color", "node", "list",
      "true", "false", "when", "each",
    ];
    for (const kw of expected) {
      expect(keywords).toContain(kw);
    }
  });

  it("tokenizeMosaic returns a non-empty array for any input", () => {
    const tokens = tokenizeMosaic("slot x: text;");
    expect(tokens.length).toBeGreaterThan(0);
  });
});
