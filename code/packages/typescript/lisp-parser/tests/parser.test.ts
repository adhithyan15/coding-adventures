import { describe, expect, it } from "vitest";
import { createLispParser, parseLisp } from "../src/index.js";

describe("Lisp parser", () => {
  it("parses atoms and lists as a program", () => {
    const ast = parseLisp("(define x 42)");
    expect(ast.ruleName).toBe("program");
    expect(ast.children.length).toBeGreaterThan(0);
  });

  it("parses quoted forms", () => {
    const ast = parseLisp("'(a b c)");
    expect(ast.ruleName).toBe("program");
  });

  it("parses dotted pairs", () => {
    const parser = createLispParser("(a . b)");
    const ast = parser.parse();
    expect(ast.ruleName).toBe("program");
  });

  it("raises parser errors for malformed lists", () => {
    expect(() => parseLisp("(a b")).toThrow();
  });
});
