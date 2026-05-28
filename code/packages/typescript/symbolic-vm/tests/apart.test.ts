import { describe, expect, it } from "vitest";
import {
  ADD,
  DIV,
  IRNode,
  MUL,
  NEG,
  POW,
  SUB,
  app,
  int,
  rational,
  sym,
} from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM } from "../src/index.js";

// Helpers — Track B1 (Apart simple-roots port) acceptance tests.
//
// Mirrors the six Python test cases enumerated in macsyma-finish-plan.md
// (Track B1).  Apart is registered by string-name "Apart" because no
// symbolic-ir constant exists for it.
const APART = sym("Apart");
const x = sym("x");

function apart(inner: IRNode): IRNode {
  return app(APART, [inner, x]);
}

describe("Apart — Track B1 (Phase 1 simple roots)", () => {
  it("decomposes 1/(x^2 - 1) into 1/(2(x-1)) - 1/(2(x+1)) (acceptance)", () => {
    const vm = new VM(new SymbolicBackend());
    // 1 / (x^2 - 1)
    const inner = app(DIV, [int(1), app(SUB, [app(POW, [x, int(2)]), int(1)])]);
    const result = vm.eval(apart(inner));
    // Roots sort ascending: -1 first, then 1.  Matches Python's output shape
    // exactly: Add(Div(-1/2, (1+x)), Div(1/2, (-1+x))).
    expect(result).toEqual(
      app(ADD, [
        app(DIV, [rational(-1, 2), app(ADD, [int(1), x])]),
        app(DIV, [rational(1, 2), app(ADD, [int(-1), x])]),
      ]),
    );
  });

  it("decomposes 1/((x-1)(x-2)(x-3)) — three distinct simple roots", () => {
    const vm = new VM(new SymbolicBackend());
    const factor1 = app(SUB, [x, int(1)]);
    const factor2 = app(SUB, [x, int(2)]);
    const factor3 = app(SUB, [x, int(3)]);
    const inner = app(DIV, [int(1), app(MUL, [app(MUL, [factor1, factor2]), factor3])]);
    const result = vm.eval(apart(inner));
    // Residues: A_1 = 1/2, A_2 = -1, A_3 = 1/2 (verified against Python).
    // Roots sort 1 < 2 < 3.  ``A = -1`` renders as ``Neg(Div(1, ...))``.
    expect(result).toEqual(
      app(ADD, [
        app(ADD, [
          app(DIV, [rational(1, 2), app(ADD, [int(-1), x])]),
          app(NEG, [app(DIV, [int(1), app(ADD, [int(-2), x])])]),
        ]),
        app(DIV, [rational(1, 2), app(ADD, [int(-3), x])]),
      ]),
    );
  });

  it("handles improper fraction x^3/(x^2-1) via polynomial division", () => {
    const vm = new VM(new SymbolicBackend());
    const inner = app(DIV, [
      app(POW, [x, int(3)]),
      app(SUB, [app(POW, [x, int(2)]), int(1)]),
    ]);
    const result = vm.eval(apart(inner));
    // x^3 / (x^2 - 1) = x + x/(x^2-1) = x + 1/(2(x-1)) + 1/(2(x+1)).
    // ``from_polynomial([0, 1], x)`` for the quotient ``x`` collapses to bare x.
    expect(result).toEqual(
      app(ADD, [
        x,
        app(ADD, [
          app(DIV, [rational(1, 2), app(ADD, [int(1), x])]),
          app(DIV, [rational(1, 2), app(ADD, [int(-1), x])]),
        ]),
      ]),
    );
  });

  it("leaves 1/(x^2 + 1) unevaluated (no rational roots, Phase 48 out of scope)", () => {
    const vm = new VM(new SymbolicBackend());
    const inner = app(DIV, [int(1), app(ADD, [app(POW, [x, int(2)]), int(1)])]);
    const result = vm.eval(apart(inner));
    // After VM args-eval, ``inner`` itself is already normalised; the
    // handler returns ``expr`` (the Apart application) when rational
    // roots are absent.  Result remains wrapped in Apart.
    expect(result).toEqual(app(APART, [inner, x]));
  });

  it("leaves 1/(x-1)^2 unevaluated (repeated root — Phase 48 out of scope)", () => {
    const vm = new VM(new SymbolicBackend());
    const inner = app(DIV, [int(1), app(POW, [app(SUB, [x, int(1)]), int(2)])]);
    const result = vm.eval(apart(inner));
    expect(result).toEqual(app(APART, [inner, x]));
  });

  it("passes a bare polynomial Apart(x+1, x) through as x+1", () => {
    const vm = new VM(new SymbolicBackend());
    const inner = app(ADD, [x, int(1)]);
    const result = vm.eval(apart(inner));
    // ``to_rational(x+1, x)`` → num = [1, 1], den = [1]; the early-return
    // path emits ``from_polynomial(num, x)`` = Add(1, x).
    expect(result).toEqual(app(ADD, [int(1), x]));
  });
});
