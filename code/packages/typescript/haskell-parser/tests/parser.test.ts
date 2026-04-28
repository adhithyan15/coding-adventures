import { describe, expect, it } from "vitest";
import { createHaskellParser, parseHaskell } from "../src/index.js";

describe("Haskell parser", () => {
  it("uses file as the root rule", () => {
    const ast = parseHaskell("x");
    expect(ast.ruleName).toBe("file");
  });

  it("parses explicit-brace let expressions", () => {
    const parser = createHaskellParser("let { x = y } in x", "2010");
    const ast = parser.parse();
    expect(ast.ruleName).toBe("file");
  });
});
