import { describe, expect, it } from "vitest";

import { parseNib } from "@coding-adventures/nib-parser";

import { checkNib, NibType } from "../src/index.js";

function findAnnotatedTypes(node: unknown, result: string[] = []): string[] {
  if (!node || typeof node !== "object") {
    return result;
  }

  const candidate = node as { _nibType?: NibType; children?: unknown[] };
  if (candidate._nibType) {
    result.push(candidate._nibType);
  }

  for (const child of candidate.children ?? []) {
    findAnnotatedTypes(child, result);
  }

  return result;
}

describe("nib-type-checker", () => {
  it("accepts a small well-typed Nib program", () => {
    const ast = parseNib("fn add(a: u4, b: u4) -> u4 { return a +% b; }");
    const result = checkNib(ast);

    expect(result.ok).toBe(true);
    expect(findAnnotatedTypes(result.typedAst)).toContain("u4");
  });

  it("infers types through additive expressions now that shift_expr sits between add_expr and mul_expr", () => {
    // Regression for #11257, which inserted a new `shift_expr` precedence
    // level into the grammar (add_expr → shift_expr → mul_expr → ...) to
    // support `<<`/`>>`. Before `shift_expr` was recognised as an expression
    // rule, an enclosing add_expr filtered its shift_expr operand out and
    // inferred no type, so *any* additive expression — even `a + b` —
    // annotated nothing.
    const ast = parseNib("fn add(a: u4, b: u4) -> u4 { return a + b; }");
    const result = checkNib(ast);

    expect(result.ok).toBe(true);
    expect(findAnnotatedTypes(result.typedAst)).toContain("u4");
  });

  it("infers types through multiplicative (mul_expr) operators", () => {
    // Regression for the precedence cascade add_expr → mul_expr → bitwise_expr:
    // before `mul_expr` was recognised as an expression rule, an enclosing
    // add_expr filtered its mul_expr operands out and inferred no type, so a
    // body like `a * b` annotated nothing.
    const ast = parseNib("fn mul(a: u4, b: u4) -> u4 { return a * b; }");
    const result = checkNib(ast);

    expect(result.ok).toBe(true);
    expect(findAnnotatedTypes(result.typedAst)).toContain("u4");
  });

  it("rejects mismatched operands of a multiplicative operator", () => {
    const ast = parseNib("fn main() { let x: u4 = 1; let y: bool = true; let z: u4 = x * y; }");
    const result = checkNib(ast);

    expect(result.ok).toBe(false);
    expect(result.errors.some((error) => error.message.includes("same type"))).toBe(true);
  });

  it("rejects mismatched let bindings", () => {
    const ast = parseNib("fn main() { let flag: bool = 1; }");
    const result = checkNib(ast);

    expect(result.ok).toBe(false);
    expect(result.errors[0]?.message).toContain("bool");
  });

  it("rejects non-bool if conditions", () => {
    const ast = parseNib("fn main() { if 1 { } }");
    const result = checkNib(ast);

    expect(result.ok).toBe(false);
    expect(result.errors.some((error) => error.message.includes("condition"))).toBe(true);
  });

  it("rejects bad function call argument types", () => {
    const ast = parseNib("fn f(x: u4) { } fn main() { f(true); }");
    const result = checkNib(ast);

    expect(result.ok).toBe(false);
    expect(result.errors.some((error) => error.message.includes("Argument 1"))).toBe(true);
  });
});
