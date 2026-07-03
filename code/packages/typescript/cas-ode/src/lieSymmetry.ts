// Lie point-symmetry handler for first-order ODEs — Track L2.
//
// TypeScript port of `cas_ode.lie_symmetry` (Python Track L1, commit
// d138e00f6).  See that module for the full literate explanation; this
// file is the structural twin.
//
// Three textbook point-symmetry groups are recognised numerically and
// reduced to a quadrature:
//
//   1. Translation in y     `(x, y) → (x, y + c)`   →  `y' = f(x)`     →
//        direct integration.
//   2. Translation in x     `(x, y) → (x + c, y)`   →  `y' = g(y)`     →
//        inverse quadrature  `x = ∫ 1/g(y) dy + C`.
//   3. Scaling              `(x, y) → (λx, λ^k y)`  for integer
//        k ∈ [-3, 3] \ {0}  →  similarity reduction  `v = y / x^k`,
//        giving a separable ODE in `(v, x)`.
//
// Detection is *numerical* — we substitute the candidate transformation
// into the IR-level `f(x, y)` and compare the result to the predicted
// transform at a handful of fixed sample points.  No symbolic
// linearised determining equation is computed.  All iteration is
// bounded: the scaling exponent search visits seven candidates, three
// scale factors and three sample points per candidate, for at most
// 63 numerical evaluations per ODE.
//
// The handler lives *after* every existing first-order family in the
// dispatcher (Bernoulli, linear, separable, homogeneous-type, exact)
// and *before* the unevaluated fall-through.  Linear, separable etc.
// always intercept the cases the dedicated handlers can solve; Lie
// fires only on what falls through — autonomous nonlinear `y' = g(y)`
// (logistic) is the canonical example.

import {
  ADD,
  D,
  DIV,
  EQUAL,
  EXP,
  INTEGRATE,
  LOG,
  MUL,
  NEG,
  POW,
  SIN,
  COS,
  SUB,
  app,
  equals,
  headName,
  int,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

// -----------------------------------------------------------------------------
// Cross-module API surface (re-exported from ./index so we can reuse the
// dispatcher's local helpers verbatim).
// -----------------------------------------------------------------------------

export interface LieOps {
  readonly simp: (node: IRNode) => IRNode;
  readonly integrate: (node: IRNode, variable: IRNode) => IRNode;
  // Predicates we share with the main dispatcher.
  readonly isConstWrt: (node: IRNode, variable: IRNode) => boolean;
  readonly substIr: (node: IRNode, from: IRNode, to: IRNode) => IRNode;
  readonly flattenAdd: (node: IRNode) => IRNode[];
  readonly sub: (lhs: IRNode, rhs: IRNode) => IRNode;
  readonly add: (...args: IRNode[]) => IRNode;
  readonly mul: (...args: IRNode[]) => IRNode;
  readonly div: (lhs: IRNode, rhs: IRNode) => IRNode;
  readonly neg: (node: IRNode) => IRNode;
  readonly pow: (base: IRNode, exponent: IRNode) => IRNode;
  readonly C: IRNode;
}

const ZERO = int(0);
const ONE = int(1);

// -----------------------------------------------------------------------------
// Section 1 — Normalise the ODE to ``y' = f(x, y)`` form.
//
// The dispatcher hands us an expression in zero form `y' - f(x, y) = 0`.
// We pull the bare `D(y, x)` summand out, treat the rest (negated) as f.
// A coefficient on `y'` other than +1 is rejected — the linear family
// already handled those cases.
// -----------------------------------------------------------------------------

function extractF(
  expr: IRNode,
  y: IRNode,
  x: IRNode,
  ops: LieOps,
): IRNode | null {
  const yPrime = app(D, [y, x]);
  let found = false;
  const rest: IRNode[] = [];
  for (const term of ops.flattenAdd(expr)) {
    const { neg: isNeg, core } = unwrapNeg(term);
    if (equals(core, yPrime)) {
      if (isNeg) return null; // bare `-y'` top-level — non-standard.
      found = true;
      continue;
    }
    rest.push(term);
  }
  if (!found) return null;
  if (rest.length === 0) return ZERO;
  let acc = rest[0];
  for (let i = 1; i < rest.length; i += 1) acc = ops.add(acc, rest[i]);
  return ops.simp(ops.neg(acc));
}

function unwrapNeg(node: IRNode): { neg: boolean; core: IRNode } {
  if (node.kind === "apply" && headName(node.head) === NEG.name && node.args.length === 1) {
    return { neg: true, core: node.args[0] };
  }
  if (node.kind === "integer" && node.value < 0n) {
    return { neg: true, core: int(-node.value) };
  }
  return { neg: false, core: node };
}

// -----------------------------------------------------------------------------
// Section 2 — Numerical evaluation.
//
// We use a small IR-walking evaluator over the operators that appear in
// textbook first-order ODEs.  Any unsupported head causes a `null` return
// which we treat as "give up".  Identical to `_eval_at_xy` in cas_ode.ode.
// -----------------------------------------------------------------------------

function evalAtXY(
  node: IRNode,
  x: IRNode,
  y: IRNode,
  xv: number,
  yv: number,
): number | null {
  if (equals(node, x)) return xv;
  if (equals(node, y)) return yv;
  switch (node.kind) {
    case "integer":
      return Number(node.value);
    case "rational":
      return Number(node.numer) / Number(node.denom);
    case "float":
      return node.value;
    case "symbol":
      return null;
    case "apply": {
      const name = headName(node.head);
      const args = node.args;
      if (name === ADD.name) {
        let acc = 0;
        for (const a of args) {
          const v = evalAtXY(a, x, y, xv, yv);
          if (v === null || !Number.isFinite(v)) return null;
          acc += v;
        }
        return acc;
      }
      if (name === MUL.name) {
        let acc = 1;
        for (const a of args) {
          const v = evalAtXY(a, x, y, xv, yv);
          if (v === null || !Number.isFinite(v)) return null;
          acc *= v;
        }
        return acc;
      }
      if (name === SUB.name && args.length === 2) {
        const a = evalAtXY(args[0], x, y, xv, yv);
        const b = evalAtXY(args[1], x, y, xv, yv);
        if (a === null || b === null) return null;
        return a - b;
      }
      if (name === DIV.name && args.length === 2) {
        const a = evalAtXY(args[0], x, y, xv, yv);
        const b = evalAtXY(args[1], x, y, xv, yv);
        if (a === null || b === null || b === 0) return null;
        return a / b;
      }
      if (name === NEG.name && args.length === 1) {
        const v = evalAtXY(args[0], x, y, xv, yv);
        return v === null ? null : -v;
      }
      if (name === POW.name && args.length === 2) {
        const a = evalAtXY(args[0], x, y, xv, yv);
        const b = evalAtXY(args[1], x, y, xv, yv);
        if (a === null || b === null) return null;
        const r = Math.pow(a, b);
        return Number.isFinite(r) ? r : null;
      }
      if (name === EXP.name && args.length === 1) {
        const v = evalAtXY(args[0], x, y, xv, yv);
        if (v === null) return null;
        const r = Math.exp(v);
        return Number.isFinite(r) ? r : null;
      }
      if (name === LOG.name && args.length === 1) {
        const v = evalAtXY(args[0], x, y, xv, yv);
        if (v === null || v === 0) return null;
        return Math.log(Math.abs(v));
      }
      if (name === SIN.name && args.length === 1) {
        const v = evalAtXY(args[0], x, y, xv, yv);
        return v === null ? null : Math.sin(v);
      }
      if (name === COS.name && args.length === 1) {
        const v = evalAtXY(args[0], x, y, xv, yv);
        return v === null ? null : Math.cos(v);
      }
      return null;
    }
    default:
      return null;
  }
}

// -----------------------------------------------------------------------------
// Section 3 — Autonomy checks.
//
// `f` is x-autonomous if `f(x, y)` doesn't change as x varies (for several
// fixed y).  Same idea for y-autonomous.  The three triples below give
// enough independent variations to make a coincidence vanishingly unlikely
// while keeping the runtime trivial.
// -----------------------------------------------------------------------------

const AUTONOMY_TEST_PTS: readonly (readonly [number, number, number])[] = [
  [0.7, 1.1, 2.3],
  [1.3, 0.4, 1.9],
  [2.1, 0.9, 3.0],
];

const AUTONOMY_TOL = 1e-9;

function isXAutonomous(f: IRNode, x: IRNode, y: IRNode): boolean {
  for (const [yv, x1, x2] of AUTONOMY_TEST_PTS) {
    const v1 = evalAtXY(f, x, y, x1, yv);
    const v2 = evalAtXY(f, x, y, x2, yv);
    if (v1 === null || v2 === null) return false;
    if (Math.abs(v1 - v2) > AUTONOMY_TOL) return false;
  }
  return true;
}

function isYAutonomous(f: IRNode, x: IRNode, y: IRNode): boolean {
  for (const [xv, y1, y2] of AUTONOMY_TEST_PTS) {
    const v1 = evalAtXY(f, x, y, xv, y1);
    const v2 = evalAtXY(f, x, y, xv, y2);
    if (v1 === null || v2 === null) return false;
    if (Math.abs(v1 - v2) > AUTONOMY_TOL) return false;
  }
  return true;
}

// -----------------------------------------------------------------------------
// Section 4 — Scaling symmetry.
//
// Test invariance under `(x, y) → (λx, λ^k y)`.  Under this group,
// `y' → λ^(k-1) y'`, so the ODE `y' = f` is invariant iff
//
//     f(λx, λ^k y) = λ^(k-1) · f(x, y)
//
// at all sample points.  k ∈ {1, 2, 3, -1, -2, -3} — the canonical
// bounded search space.  k = 0 is omitted (handled by the translation-
// in-x branch).
// -----------------------------------------------------------------------------

const SCALING_LAMBDAS: readonly number[] = [2.0, 3.0, 0.5];
const SCALING_POINTS: readonly (readonly [number, number])[] = [
  [1.0, 1.0],
  [2.0, 3.0],
  [1.0, 2.0],
];
const SCALING_K_RANGE: readonly number[] = [1, 2, 3, -1, -2, -3];
const SCALING_TOL = 1e-7;

function detectScalingK(f: IRNode, x: IRNode, y: IRNode): number | null {
  for (const k of SCALING_K_RANGE) {
    let allOk = true;
    outer: for (const lam of SCALING_LAMBDAS) {
      for (const [xv, yv] of SCALING_POINTS) {
        const yScaled = Math.pow(lam, k) * yv;
        const lhs = evalAtXY(f, x, y, lam * xv, yScaled);
        const base = evalAtXY(f, x, y, xv, yv);
        if (lhs === null || base === null) {
          allOk = false;
          break outer;
        }
        const expected = Math.pow(lam, k - 1) * base;
        const scale = Math.max(1.0, Math.abs(expected));
        if (Math.abs(lhs - expected) > SCALING_TOL * scale) {
          allOk = false;
          break outer;
        }
      }
    }
    if (allOk) return k;
  }
  return null;
}

// -----------------------------------------------------------------------------
// Section 5 — Reductions.
// -----------------------------------------------------------------------------

function reduceTranslationY(
  f: IRNode,
  y: IRNode,
  x: IRNode,
  ops: LieOps,
): IRNode | null {
  const intF = ops.integrate(f, x);
  if (containsUnevaluatedIntegrate(intF, x)) return null;
  return app(EQUAL, [y, ops.simp(ops.add(intF, ops.C))]);
}

function reduceTranslationX(
  f: IRNode,
  y: IRNode,
  x: IRNode,
  ops: LieOps,
): IRNode | null {
  // Guard the f = 0 case (caught by separable upstream).
  if (f.kind === "integer" && f.value === 0n) return null;
  const inv = ops.simp(ops.div(ONE, f));
  const intInv = ops.integrate(inv, y);
  if (containsUnevaluatedIntegrate(intInv, y)) return null;
  return app(EQUAL, [x, ops.simp(ops.add(intInv, ops.C))]);
}

const CERT_SAMPLE_X: readonly number[] = [1.5, 2.5, 0.4];
const CERT_SAMPLE_V: readonly number[] = [0.7, 1.3, 2.1];
const CERT_TOL = 1e-6;

function verifyScalingCertificate(
  fSubst: IRNode,
  gRaw: IRNode,
  k: number,
  x: IRNode,
  v: IRNode,
): boolean {
  for (const xv of CERT_SAMPLE_X) {
    for (const vv of CERT_SAMPLE_V) {
      const lhs = evalAtXY(fSubst, x, v, xv, vv);
      const g = evalAtXY(gRaw, x, v, xv, vv); // G is x-free, xv ignored
      if (lhs === null || g === null) return false;
      const expected = Math.pow(xv, k - 1) * g;
      const scale = Math.max(1.0, Math.abs(expected));
      if (Math.abs(lhs - expected) > CERT_TOL * scale) return false;
    }
  }
  return true;
}

function reduceScaling(
  f: IRNode,
  k: number,
  y: IRNode,
  x: IRNode,
  ops: LieOps,
): IRNode | null {
  if (k === 0) return null;

  // Build `x^k` (degenerate small powers handled by the local `pow`).
  let xToK: IRNode;
  if (k === 1) xToK = x;
  else if (k > 1) xToK = ops.pow(x, int(BigInt(k)));
  else xToK = ops.div(ONE, ops.pow(x, int(BigInt(-k))));

  const v = sym("_lie_v");

  // Step 1: f_subst = f(x, v · x^k).
  const fSubst = ops.simp(ops.substIr(f, y, ops.mul(v, xToK)));

  // Step 2: extract G(v) = f_subst|_{x=1}.  At the certificate point
  // x = 1, x^(k-1) = 1, so f_subst reduces to G(v) directly.  We verify
  // numerically that G(v) is indeed x-independent and that
  // f_subst(x, v) = x^(k-1) · G(v) at other sample x's.
  const gRaw = ops.simp(ops.substIr(fSubst, x, ONE));
  if (!ops.isConstWrt(gRaw, x)) return null;
  if (!verifyScalingCertificate(fSubst, gRaw, k, x, v)) return null;

  // Step 3: separable denominator G(v) − k·v.  Degenerate case
  // G(v) = k·v ⇒ v = const ⇒ y = C · x^k.
  const denom = ops.simp(ops.sub(gRaw, ops.mul(int(BigInt(k)), v)));
  if (denom.kind === "integer" && denom.value === 0n) {
    return app(EQUAL, [y, ops.simp(ops.mul(ops.C, xToK))]);
  }

  // Build the integrand `1/denom`.  When `denom` is `Pow(v, n)` with
  // positive integer `n`, prefer `Pow(v, -n)` so the local integrator's
  // power rule applies; otherwise leave it as `Div(1, denom)`.
  let integrand: IRNode;
  if (
    denom.kind === "apply" &&
    headName(denom.head) === POW.name &&
    denom.args.length === 2 &&
    equals(denom.args[0], v) &&
    denom.args[1].kind === "integer" &&
    denom.args[1].value > 0n
  ) {
    integrand = ops.pow(v, int(-denom.args[1].value));
  } else {
    integrand = ops.simp(ops.div(ONE, denom));
  }
  const hV = ops.integrate(integrand, v);
  if (containsUnevaluatedIntegrate(hV, v)) return null;

  // Step 4: back-substitute v → y/x^k, RHS log(x) + C.
  const hYxk = ops.simp(ops.substIr(hV, v, ops.div(y, xToK)));
  let logX = ops.integrate(ops.div(ONE, x), x);
  if (containsUnevaluatedIntegrate(logX, x)) logX = app(LOG, [x]);
  const rhs = ops.simp(ops.add(logX, ops.C));
  return app(EQUAL, [hYxk, rhs]);
}

function containsUnevaluatedIntegrate(node: IRNode, variable: IRNode): boolean {
  if (node.kind !== "apply") return false;
  if (headName(node.head) === INTEGRATE.name && node.args.length === 2 && equals(node.args[1], variable)) {
    return true;
  }
  return node.args.some((arg) => containsUnevaluatedIntegrate(arg, variable));
}

// -----------------------------------------------------------------------------
// Section 6 — Public entry point.
//
// Dispatch order matches the Python original:
//   1. Translation in y  (cheapest, explicit `y = ∫ f dx + C`)
//   2. Translation in x  (autonomous, implicit `x = ∫ 1/g dy + C`)
//   3. Scaling           (similarity reduction `v = y/x^k`)
// -----------------------------------------------------------------------------

export function tryLieSymmetry(
  expr: IRNode,
  y: IRNode,
  x: IRNode,
  ops: LieOps,
): IRNode | null {
  if (y.kind !== "symbol" || x.kind !== "symbol") return null;
  const f = extractF(expr, y, x, ops);
  if (f === null) return null;

  if (isYAutonomous(f, x, y)) {
    const sol = reduceTranslationY(f, y, x, ops);
    if (sol !== null) return sol;
  }

  if (isXAutonomous(f, x, y)) {
    const sol = reduceTranslationX(f, y, x, ops);
    if (sol !== null) return sol;
  }

  const k = detectScalingK(f, x, y);
  if (k !== null) {
    const sol = reduceScaling(f, k, y, x, ops);
    if (sol !== null) return sol;
  }

  return null;
}
