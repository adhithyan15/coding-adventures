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

// Helpers — Track B1 (Apart simple-roots) + Track B3 (Phase 48 repeated
// linear factors) acceptance tests.
//
// Mirrors the Python test cases enumerated in
// ``code/specs/macsyma-finish-plan.md`` (Tracks B1 + B3) and verified
// against the Python reference output exactly.  Apart is registered by
// string-name "Apart" because no symbolic-ir constant exists for it.
const APART = sym("Apart");
const x = sym("x");
const k = sym("k");

function apart(inner: IRNode, variable: IRNode = x): IRNode {
  return app(APART, [inner, variable]);
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

  it("returns 1/(x^2 + 1) unchanged when the denominator is irreducible over Q", () => {
    const vm = new VM(new SymbolicBackend());
    const inner = app(DIV, [int(1), app(ADD, [app(POW, [x, int(2)]), int(1)])]);
    const result = vm.eval(apart(inner));
    expect(result).toEqual(app(DIV, [int(1), app(ADD, [int(1), app(POW, [x, int(2)])])]));
  });

  it("splits the polynomial part before returning an irreducible remainder", () => {
    const vm = new VM(new SymbolicBackend());
    const x2 = app(POW, [x, int(2)]);
    const inner = app(DIV, [app(ADD, [x2, int(2)]), app(ADD, [x2, int(1)])]);
    const result = vm.eval(apart(inner));
    expect(result).toEqual(
      app(ADD, [int(1), app(DIV, [int(1), app(ADD, [int(1), app(POW, [x, int(2)])])])]),
    );
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

describe("Apart — Track B3 (Phase 48 repeated linear factors)", () => {
  it("decomposes 1/(k^2 (k+1)^2) into -2/k + 1/k^2 + 2/(k+1) + 1/(k+1)^2 (acceptance)", () => {
    const vm = new VM(new SymbolicBackend());
    // 1 / (k^2 * (k+1)^2)
    const k2 = app(POW, [k, int(2)]);
    const kp1 = app(ADD, [k, int(1)]);
    const kp1Sq = app(POW, [kp1, int(2)]);
    const inner = app(DIV, [int(1), app(MUL, [k2, kp1Sq])]);
    const result = vm.eval(apart(inner, k));
    // Roots sorted ascending: -1 (mult 2) before 0 (mult 2).
    // For r = -1: φ(t) = 1 / (r+t)^2 |_{r=-1}; Q(x) = k^2, so
    //   Q(-1+t) = (-1+t)^2 = 1 - 2t + t^2  →  φ_0 = 1, φ_1 = 2.
    //   A_{-1, 2} = φ_0 = 1,  A_{-1, 1} = φ_1 = 2.
    // For r = 0: Q(x) = (k+1)^2; Q(t) = (1+t)^2 = 1 + 2t + t^2 →
    //   φ_0 = 1, φ_1 = -2.  A_{0, 2} = 1, A_{0, 1} = -2.
    // Emit in order: 2/(1+k), 1/(1+k)^2, -2/k, 1/k^2 (left-associated).
    expect(result).toEqual(
      app(ADD, [
        app(ADD, [
          app(ADD, [
            app(DIV, [int(2), app(ADD, [int(1), k])]),
            app(DIV, [int(1), app(POW, [app(ADD, [int(1), k]), int(2)])]),
          ]),
          app(DIV, [int(-2), k]),
        ]),
        app(DIV, [int(1), app(POW, [k, int(2)])]),
      ]),
    );
  });

  it("decomposes 1/(k-1)^3 to itself (single triple root, Q(x) = 1)", () => {
    const vm = new VM(new SymbolicBackend());
    const inner = app(DIV, [int(1), app(POW, [app(SUB, [k, int(1)]), int(3)])]);
    const result = vm.eval(apart(inner, k));
    // r = 1, m = 3, Q(x) = 1.  φ(t) = 1 / 1 = 1; φ_0 = 1, φ_1 = φ_2 = 0.
    // A_{1, 3} = 1, A_{1, 2} = 0, A_{1, 1} = 0.  Single term emitted.
    expect(result).toEqual(
      app(DIV, [int(1), app(POW, [app(ADD, [int(-1), k]), int(3)])]),
    );
  });

  it("decomposes 1/((k-1)(k-2)^2) (mixed simple + repeated)", () => {
    const vm = new VM(new SymbolicBackend());
    const f1 = app(SUB, [k, int(1)]);
    const f2Sq = app(POW, [app(SUB, [k, int(2)]), int(2)]);
    const inner = app(DIV, [int(1), app(MUL, [f1, f2Sq])]);
    const result = vm.eval(apart(inner, k));
    // Roots ascending: 1 (simple), 2 (mult 2).
    // For r = 1 (simple): emitted via fall-through to the same Taylor
    //   path; Q(x) = (k-2)^2, Q(1) = 1, so A = 1/Q(1) = 1.
    // For r = 2 (mult 2): Q(x) = (k-1); Q(2+t) = 1 + t →
    //   φ_0 = 1, φ_1 = -1.  A_{2, 2} = 1, A_{2, 1} = -1.
    // Emit: 1/(-1+k), -1/(-2+k) [as Neg(Div(1, …))], 1/(-2+k)^2.
    expect(result).toEqual(
      app(ADD, [
        app(ADD, [
          app(DIV, [int(1), app(ADD, [int(-1), k])]),
          app(NEG, [app(DIV, [int(1), app(ADD, [int(-2), k])])]),
        ]),
        app(DIV, [int(1), app(POW, [app(ADD, [int(-2), k]), int(2)])]),
      ]),
    );
  });

  it("decomposes 1/((x^2+1)(x-1)^2) into poles plus irreducible residual", () => {
    const vm = new VM(new SymbolicBackend());
    const quad = app(ADD, [app(POW, [x, int(2)]), int(1)]);
    const linSq = app(POW, [app(SUB, [x, int(1)]), int(2)]);
    const inner = app(DIV, [int(1), app(MUL, [quad, linSq])]);
    const result = vm.eval(apart(inner));
    expect(result).toEqual(
      app(ADD, [
        app(ADD, [
          app(DIV, [rational(-1, 2), app(ADD, [int(-1), x])]),
          app(DIV, [rational(1, 2), app(POW, [app(ADD, [int(-1), x]), int(2)])]),
        ]),
        app(DIV, [
          app(MUL, [rational(1, 2), x]),
          app(ADD, [int(1), app(POW, [x, int(2)])]),
        ]),
      ]),
    );
  });

  it("decomposes 1/(x-2)^2 to itself (single repeated root, Q(x) = 1)", () => {
    const vm = new VM(new SymbolicBackend());
    const inner = app(DIV, [int(1), app(POW, [app(SUB, [x, int(2)]), int(2)])]);
    const result = vm.eval(apart(inner));
    // r = 2, m = 2, Q(x) = 1.  φ_0 = 1, φ_1 = 0.
    // A_{2, 2} = 1, A_{2, 1} = 0.  Single term.
    expect(result).toEqual(
      app(DIV, [int(1), app(POW, [app(ADD, [int(-2), x]), int(2)])]),
    );
  });
});
