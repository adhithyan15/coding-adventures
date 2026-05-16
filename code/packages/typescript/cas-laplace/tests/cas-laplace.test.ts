import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  COSH,
  DIV,
  EXP,
  MUL,
  NEG,
  POW,
  SIN,
  SUB,
  app,
  equals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  DIRAC_DELTA,
  ILT,
  LAPLACE,
  UNIT_STEP,
  buildLaplaceHandlerTable,
  diracDeltaHandler,
  iltHandler,
  inverseLaplace,
  laplaceHandler,
  laplaceTransform,
  unitStepHandler,
} from "../src/index";

const t = sym("t");
const s = sym("s");

// ─── helpers ─────────────────────────────────────────────────────────────────

function isHead(node: IRNode, head: IRNode): boolean {
  return node.kind === "apply" && equals(node.head, head);
}

/** Build `Mul(a, b)`. */
function mul(a: IRNode, b: IRNode): IRNode {
  return app(MUL, [a, b]);
}

/** Build `Pow(base, exp)`. */
function pow(base: IRNode, exponent: IRNode): IRNode {
  return app(POW, [base, exponent]);
}

/** Build `Add(a, b)`. */
function add(a: IRNode, b: IRNode): IRNode {
  return app(ADD, [a, b]);
}

/** Build `Sub(a, b)`. */
function sub(a: IRNode, b: IRNode): IRNode {
  return app(SUB, [a, b]);
}

/** Build `Div(a, b)`. */
function div(a: IRNode, b: IRNode): IRNode {
  return app(DIV, [a, b]);
}

// ─── Forward transform: existing cases ───────────────────────────────────────

describe("laplaceTransform — constants and powers", () => {
  it("handles 1", () => {
    expect(laplaceTransform(int(1), t, s)).toEqual(div(int(1), s));
  });

  it("handles t", () => {
    expect(laplaceTransform(t, t, s)).toEqual(div(int(1), pow(s, int(2))));
  });

  it("handles t^3", () => {
    expect(laplaceTransform(pow(t, int(3)), t, s)).toEqual(
      div(int(6), pow(s, int(4))),
    );
  });
});

describe("laplaceTransform — exp, trig, hyperbolic, shifted products", () => {
  it("handles exp(3t)", () => {
    expect(laplaceTransform(app(EXP, [mul(int(3), t)]), t, s)).toEqual(
      div(int(1), sub(s, int(3))),
    );
  });

  it("handles sin(2t)", () => {
    expect(laplaceTransform(app(SIN, [mul(int(2), t)]), t, s)).toEqual(
      div(int(2), add(pow(s, int(2)), pow(int(2), int(2)))),
    );
  });

  it("handles exp(t)*sin(2t)", () => {
    const expSin = mul(app(EXP, [t]), app(SIN, [mul(int(2), t)]));
    expect(isHead(laplaceTransform(expSin, t, s), DIV)).toBe(true);
  });

  it("handles cosh(t)", () => {
    expect(isHead(laplaceTransform(app(COSH, [t]), t, s), DIV)).toBe(true);
  });
});

describe("laplaceTransform — linearity and fallthrough", () => {
  it("applies linearity over Add", () => {
    const sum = add(app(SIN, [t]), app(COS, [t]));
    expect(isHead(laplaceTransform(sum, t, s), ADD)).toBe(true);
  });

  it("applies linearity over scalar multiple", () => {
    expect(isHead(laplaceTransform(mul(int(5), app(SIN, [t])), t, s), MUL)).toBe(true);
  });

  it("falls through for unknown heads", () => {
    const unknown = app(sym("Mystery"), [t]);
    expect(laplaceTransform(unknown, t, s)).toEqual(app(LAPLACE, [unknown, t, s]));
  });
});

describe("laplaceTransform — special heads", () => {
  it("handles DiracDelta(t)", () => {
    expect(laplaceTransform(app(DIRAC_DELTA, [t]), t, s)).toEqual(int(1));
  });

  it("handles UnitStep(t)", () => {
    expect(laplaceTransform(app(UNIT_STEP, [t]), t, s)).toEqual(div(int(1), s));
  });
});

// ─── Forward transform: t^n·trig (n ≥ 2) ────────────────────────────────────

describe("laplaceTransform — t^n·trig for n = 2, 3", () => {
  // ── n = 2 ──────────────────────────────────────────────────────────────
  //
  // L{t²·sin(ωt)} = 2ω(3s²−ω²) / (s²+ω²)³
  // L{t²·cos(ωt)} = 2s(s²−3ω²) / (s²+ω²)³

  it("L{t²·sin(2t)} = 4(3s²−4)/(s²+4)³", () => {
    // f = Mul(Pow(t,2), Sin(Mul(2,t)))
    const f = mul(pow(t, int(2)), app(SIN, [mul(int(2), t)]));
    const result = laplaceTransform(f, t, s);
    // Expected: Div(Mul(Mul(2,2), Sub(Mul(3,s²), Pow(2,2))), Pow(Add(s²,Pow(2,2)),3))
    const s2 = pow(s, int(2));
    const w2 = pow(int(2), int(2));
    const s2pw2 = add(s2, w2);
    const expectedNum = mul(mul(int(2), int(2)), sub(mul(int(3), s2), w2));
    const expected = div(expectedNum, pow(s2pw2, int(3)));
    expect(result).toEqual(expected);
  });

  it("L{t²·sin(2t)} matches reversed factor order too", () => {
    // Mul(Sin(2t), Pow(t,2)) — commuted operands
    const f = mul(app(SIN, [mul(int(2), t)]), pow(t, int(2)));
    const result = laplaceTransform(f, t, s);
    expect(isHead(result, DIV)).toBe(true);
  });

  it("L{t²·cos(2t)} = 2s(s²−12)/(s²+4)³", () => {
    const f = mul(pow(t, int(2)), app(COS, [mul(int(2), t)]));
    const result = laplaceTransform(f, t, s);
    const s2 = pow(s, int(2));
    const w2 = pow(int(2), int(2)); // 4
    const s2pw2 = add(s2, w2);
    const expectedNum = mul(mul(int(2), s), sub(s2, mul(int(3), w2)));
    const expected = div(expectedNum, pow(s2pw2, int(3)));
    expect(result).toEqual(expected);
  });

  // ── n = 3 ──────────────────────────────────────────────────────────────
  //
  // L{t³·sin(ωt)} = 24ωs(s²−ω²) / (s²+ω²)⁴
  // L{t³·cos(ωt)} = 6(s⁴−6s²ω²+ω⁴) / (s²+ω²)⁴

  it("L{t³·sin(t)} = 24s(s²−1)/(s²+1)⁴", () => {
    const f = mul(pow(t, int(3)), app(SIN, [t]));
    const result = laplaceTransform(f, t, s);
    const s2 = pow(s, int(2));
    const w2 = pow(int(1), int(2)); // 1
    const s2pw2 = add(s2, w2);
    // Num: Mul(Mul(24, 1), Mul(s, Sub(s², w²)))
    const expectedNum = mul(mul(int(24), int(1)), mul(s, sub(s2, w2)));
    const expected = div(expectedNum, pow(s2pw2, int(4)));
    expect(result).toEqual(expected);
  });

  it("L{t³·cos(t)} = 6(s⁴−6s²+1)/(s²+1)⁴", () => {
    const f = mul(pow(t, int(3)), app(COS, [t]));
    const result = laplaceTransform(f, t, s);
    const s2 = pow(s, int(2));
    const w2 = pow(int(1), int(2));
    const s2pw2 = add(s2, w2);
    const s4 = pow(s, int(4));
    const w4 = pow(int(1), int(4));
    const inner = add(sub(s4, mul(int(6), mul(s2, w2))), w4);
    const expected = div(mul(int(6), inner), pow(s2pw2, int(4)));
    expect(result).toEqual(expected);
  });

  it("L{t⁴·sin(t)} falls through to unevaluated Laplace", () => {
    const f = mul(pow(t, int(4)), app(SIN, [t]));
    const result = laplaceTransform(f, t, s);
    // n=4 is unsupported; the linearity rule extracts no coefficient,
    // so the whole expression goes to the table which returns undefined,
    // and the top-level wraps it in Laplace(...).
    expect(isHead(result, LAPLACE)).toBe(true);
  });
});

// ─── Inverse transform: existing direct table ────────────────────────────────

describe("inverseLaplace — direct table entries", () => {
  it("1/s → UnitStep(t)", () => {
    expect(inverseLaplace(div(int(1), s), s, t)).toEqual(app(UNIT_STEP, [t]));
  });

  it("1/(s−3) → exp(3t)", () => {
    expect(
      inverseLaplace(div(int(1), sub(s, int(3))), s, t),
    ).toEqual(app(EXP, [mul(int(3), t)]));
  });

  it("2/(s²+4) → sin(2t)", () => {
    expect(
      inverseLaplace(div(int(2), add(pow(s, int(2)), int(4))), s, t),
    ).toEqual(app(SIN, [mul(int(2), t)]));
  });

  it("s/(s²−1) → cosh(1·t)", () => {
    expect(
      inverseLaplace(div(s, sub(pow(s, int(2)), int(1))), s, t),
    ).toEqual(app(COSH, [mul(int(1), t)]));
  });

  it("returns unevaluated ILT for unknown forms", () => {
    const unknown = app(sym("Unknown"), [s]);
    expect(inverseLaplace(unknown, s, t)).toEqual(app(ILT, [unknown, s, t]));
  });
});

// ─── Inverse transform: complex conjugate poles (irreducible quadratic) ───────

describe("inverseLaplace — irreducible quadratic (complex poles)", () => {
  // 1/(s²+2s+2) = 1/((s+1)²+1²) → exp(−t)·sin(t)
  // The denominator has no rational roots (discriminant = 4−8 = −4).
  it("1/(s²+2s+2) → exp(−t)·sin(t)", () => {
    // Build 1 / (s² + 2s + 2)
    const denom = add(add(pow(s, int(2)), mul(int(2), s)), int(2));
    const F = div(int(1), denom);
    const result = inverseLaplace(F, s, t);
    // Expected: Mul(Exp(Neg(t)), Sin(t))
    const expected = mul(app(EXP, [app(NEG, [t])]), app(SIN, [t]));
    expect(result).toEqual(expected);
  });

  // s/(s²+2s+2) → exp(−t)·cos(t) − exp(−t)·sin(t)
  it("s/(s²+2s+2) → exp(−t)·cos(t) + (−1)·exp(−t)·sin(t)", () => {
    const denom = add(add(pow(s, int(2)), mul(int(2), s)), int(2));
    const F = div(s, denom);
    const result = inverseLaplace(F, s, t);
    // First term A=1: exp(-t)*cos(t)
    // baa = B - A*alpha = 0 - 1*1 = -1 → coeff = -1/1 = -1
    // Second term: Mul(-1, Mul(Exp(Neg(t)), Sin(t)))
    const expNegT = app(EXP, [app(NEG, [t])]);
    const t1 = mul(expNegT, app(COS, [t]));
    const t2 = mul(int(-1), mul(expNegT, app(SIN, [t])));
    const expected = add(t1, t2);
    expect(result).toEqual(expected);
  });

  // 1/(s²+1) goes through direct table, not PF engine
  // (matchSSqPlusParamSq matches it as omega=1)
  it("1/(s²+1) → sin(1·t) [via direct table]", () => {
    const F = div(int(1), add(pow(s, int(2)), int(1)));
    const result = inverseLaplace(F, s, t);
    expect(result).toEqual(app(SIN, [mul(int(1), t)]));
  });

  // 1/(s*(s²+1)) — mixed: simple pole at 0 + irreducible quadratic
  it("1/(s·(s²+1)) → UnitStep(t) + (−1)·cos(t)", () => {
    // Build 1 / (s * (s² + 1))
    const F = div(int(1), mul(s, add(pow(s, int(2)), int(1))));
    const result = inverseLaplace(F, s, t);
    // UnitStep(t) from simple pole at 0,
    // Mul(-1, Cos(t)) from the irreducible quadratic with A=-1, B=0
    const expected = add(app(UNIT_STEP, [t]), mul(int(-1), app(COS, [t])));
    expect(result).toEqual(expected);
  });
});

// ─── Inverse transform: repeated poles ───────────────────────────────────────

describe("inverseLaplace — repeated rational poles", () => {
  // 1/(s−2)² → t·exp(2t)
  it("1/(s−2)² → t·exp(2t)", () => {
    // Build 1 / (s-2)²  = 1 / Pow(Sub(s,2), 2)
    const F = div(int(1), pow(sub(s, int(2)), int(2)));
    const result = inverseLaplace(F, s, t);
    // iltRepeatedPole(1, 2, 2, t) = t * exp(2t)
    const expected = mul(t, app(EXP, [mul(int(2), t)]));
    expect(result).toEqual(expected);
  });

  // 1/s² → t  (repeated pole at 0; the direct table handles it before PF)
  it("1/s² → t [via direct table]", () => {
    const F = div(int(1), pow(s, int(2)));
    const result = inverseLaplace(F, s, t);
    expect(result).toEqual(t);
  });

  // s/(s−1)² — s in numerator, repeated pole at 1
  // PF: s = A*(s-1) + B, but via PF engine...
  // num=(0,1), den=(1,-2,1)=(s-1)^2, roots=[1,1]
  // residues via power series: computeRepeatedResidues((0,1),(1,-2,1),1,2)
  //   N_t = shift (0,1) by 1: (1,1) = 1 + t
  //   D_t = shift (1,-2,1) by 1: t² = (0,0,1)
  //   Qother = (1,) [from D_t[2:]]
  //   ps coeffs of (1,1)/(1,): g0=1, g1=1 → [1,1] = [A2, A1]
  // k=0: A=1, poleOrder=2 → iltRepeatedPole(1,1,2,t) = t*exp(t)
  // k=1: A=1, poleOrder=1 → iltSimplePole(1,1,t) = exp(t)
  // result = Add(t*exp(t), exp(t))
  it("s/(s−1)² → t·exp(t) + exp(t)", () => {
    const F = div(s, pow(sub(s, int(1)), int(2)));
    const result = inverseLaplace(F, s, t);
    // iltSimplePole and iltRepeatedPole both use exp(t) (not exp(1*t)) when a=1
    const expT = app(EXP, [t]);
    const expected = add(mul(t, expT), expT);
    expect(result).toEqual(expected);
  });
});

// ─── Inverse transform: improper fractions ───────────────────────────────────

describe("inverseLaplace — improper fractions", () => {
  // s²/(s²+1): polynomial part = 1 (quotient) with remainder = -1/(s²+1)
  // L⁻¹{1} = DiracDelta(t), L⁻¹{-1/(s²+1)} = -sin(t)
  it("s²/(s²+1) → DiracDelta(t) + (−1)·sin(t)", () => {
    const F = div(pow(s, int(2)), add(pow(s, int(2)), int(1)));
    const result = inverseLaplace(F, s, t);
    // poly part: (1,), so DiracDelta(t)
    // remainder: (-1,0) / (1,0,1) → but via irredQuad:
    // linNum = (-1,), quadDen = (1,0,1), A=0, B=-1
    // baa = -1 - 0 = -1, coeff2 = -1/1 = -1
    // term2: Mul(-1, Sin(t))
    const expected = add(app(DIRAC_DELTA, [t]), mul(int(-1), app(SIN, [t])));
    expect(result).toEqual(expected);
  });
});

// ─── Handlers ─────────────────────────────────────────────────────────────────

describe("handlers", () => {
  const id = (node: IRNode): IRNode => node;

  it("dispatches laplaceHandler", () => {
    expect(laplaceHandler(app(LAPLACE, [int(1), t, s]), id)).toEqual(div(int(1), s));
  });

  it("dispatches iltHandler", () => {
    expect(
      iltHandler(app(ILT, [div(int(1), s), s, t]), id),
    ).toEqual(app(UNIT_STEP, [t]));
  });

  it("evaluates DiracDelta and UnitStep special functions", () => {
    expect(diracDeltaHandler(app(DIRAC_DELTA, [int(0)]))).toEqual(int(1));
    expect(unitStepHandler(app(UNIT_STEP, [int(-1)]))).toEqual(int(0));
    expect(unitStepHandler(app(UNIT_STEP, [int(0)]))).toEqual(rational(1, 2));
    expect(unitStepHandler(app(UNIT_STEP, [int(2)]))).toEqual(int(1));
  });

  it("exposes the four handler table keys", () => {
    expect([...buildLaplaceHandlerTable().keys()]).toEqual([
      "Laplace",
      "ILT",
      "DiracDelta",
      "UnitStep",
    ]);
  });
});
