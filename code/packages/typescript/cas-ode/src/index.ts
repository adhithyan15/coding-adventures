import {
  ADD,
  BESSEL_J,
  BESSEL_Y,
  CHEBYSHEV_T,
  CHEBYSHEV_U,
  COS,
  D,
  DIV,
  EQUAL,
  EXP,
  HERMITE_H,
  HERMITE_H2,
  INTEGRATE,
  LEGENDRE_P,
  LEGENDRE_Q,
  LOG,
  MUL,
  NEG,
  POW,
  SIN,
  SUB,
  app,
  equals,
  headName,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const ODE2 = sym("ODE2");
export const C = sym("%c");
export const C1 = sym("%c1");
export const C2 = sym("%c2");

export type EvalFn = (node: IRNode) => IRNode;

export interface SolveOdeOptions {
  readonly simplify?: EvalFn;
  readonly integrate?: (node: IRNode, variable: IRNode) => IRNode | null | undefined;
  readonly differentiate?: (node: IRNode, variable: IRNode) => IRNode | null | undefined;
}

type ClassifiedForcing =
  | readonly ["poly", readonly Frac[]]
  | readonly ["exp", Frac]
  | readonly ["sin", Frac]
  | readonly ["cos", Frac];

const ZERO = int(0);
const ONE = int(1);
const TWO = int(2);
const HALF = rational(1, 2);

class Frac {
  readonly numer: bigint;
  readonly denom: bigint;

  constructor(numerInput: bigint | number | string, denomInput: bigint | number | string = 1n) {
    let numer = toBigInt(numerInput);
    let denom = toBigInt(denomInput);
    if (denom === 0n) throw new RangeError("Frac denominator must not be zero");
    if (numer === 0n) {
      this.numer = 0n;
      this.denom = 1n;
      return;
    }
    if (denom < 0n) {
      numer = -numer;
      denom = -denom;
    }
    const g = gcd(abs(numer), denom);
    this.numer = numer / g;
    this.denom = denom / g;
  }

  static zero(): Frac {
    return new Frac(0n);
  }

  static one(): Frac {
    return new Frac(1n);
  }

  static fromNode(node: IRNode): Frac | null {
    if (node.kind === "integer") return new Frac(node.value);
    if (node.kind === "rational") return new Frac(node.numer, node.denom);
    if (node.kind === "apply" && headName(node.head) === NEG.name && node.args.length === 1) {
      const inner = Frac.fromNode(node.args[0]);
      return inner?.neg() ?? null;
    }
    return null;
  }

  isZero(): boolean {
    return this.numer === 0n;
  }

  isOne(): boolean {
    return this.numer === this.denom;
  }

  isNeg(): boolean {
    return this.numer < 0n;
  }

  neg(): Frac {
    return new Frac(-this.numer, this.denom);
  }

  add(rhs: Frac): Frac {
    return new Frac(this.numer * rhs.denom + rhs.numer * this.denom, this.denom * rhs.denom);
  }

  sub(rhs: Frac): Frac {
    return this.add(rhs.neg());
  }

  mul(rhs: Frac): Frac {
    return new Frac(this.numer * rhs.numer, this.denom * rhs.denom);
  }

  div(rhs: Frac): Frac {
    if (rhs.isZero()) throw new RangeError("Frac division by zero");
    return new Frac(this.numer * rhs.denom, this.denom * rhs.numer);
  }

  eq(rhs: Frac): boolean {
    return this.numer === rhs.numer && this.denom === rhs.denom;
  }

  gtZero(): boolean {
    return this.numer > 0n;
  }

  ltZero(): boolean {
    return this.numer < 0n;
  }

  toIr(): IRNode {
    return this.denom === 1n ? int(this.numer) : rational(this.numer, this.denom);
  }
}

export function solveOde(equation: IRNode, y: IRNode, x: IRNode, options: SolveOdeOptions = {}): IRNode | null {
  if (y.kind !== "symbol" || x.kind !== "symbol") return null;
  const ops = makeOps(options);
  const expr = normalizeEquation(equation);

  const nonhom = trySecondOrderNonhom(expr, y, x, ops);
  if (nonhom !== null) return nonhom;

  const second = collectSecondOrderCoeffs(expr, y, x);
  if (second !== null) return solveSecondOrderConstCoeff(second.a, second.b, second.c, y, x);

  const euler = tryEulerCauchy(expr, y, x);
  if (euler !== null) return euler;

  const named = tryVarCoeffNamedOde(expr, y, x);
  if (named !== null) return named;

  const bernoulli = tryBernoulli(expr, y, x, ops);
  if (bernoulli !== null) return bernoulli;

  const linear = collectLinearFirstOrder(expr, y, x);
  if (linear !== null) return solveLinearFirstOrder(linear.p, linear.q, y, x, ops);

  const exact = tryExact(expr, y, x, ops);
  if (exact !== null) return exact;

  const separable = trySeparable(expr, y, x, ops);
  if (separable !== null) return separable;

  const homogeneous = tryHomogeneousType(expr, y, x, ops);
  if (homogeneous !== null) return homogeneous;

  return null;
}

export function ode2(equation: IRNode, y: IRNode, x: IRNode, options: SolveOdeOptions = {}): IRNode {
  return solveOde(equation, y, x, options) ?? app(ODE2, [equation, y, x]);
}

export function ode2Handler(expr: IRNode, evalFn: EvalFn = (node) => node): IRNode {
  const args = applyArgs(expr, ODE2);
  if (args === undefined || args.length !== 3 || args[1].kind !== "symbol" || args[2].kind !== "symbol") return expr;
  const solved = solveOde(args[0], args[1], args[2], { simplify: evalFn });
  return solved === null ? expr : evalFn(solved);
}

export function buildOdeHandlerTable(): ReadonlyMap<string, (expr: IRNode, evalFn: EvalFn) => IRNode> {
  return new Map([["ODE2", ode2Handler]]);
}

function normalizeEquation(equation: IRNode): IRNode {
  const equal = applyArgs(equation, EQUAL);
  return equal !== undefined && equal.length === 2 ? sub(equal[0], equal[1]) : equation;
}

interface Ops {
  readonly simp: EvalFn;
  readonly integrate: (node: IRNode, variable: IRNode) => IRNode;
  readonly differentiate: (node: IRNode, variable: IRNode) => IRNode;
}

function makeOps(options: SolveOdeOptions): Ops {
  const simp = options.simplify ?? ((node: IRNode) => localSimplify(node));
  return {
    simp: (node) => simp(localSimplify(node)),
    integrate: (node, variable) => {
      const custom = options.integrate?.(node, variable);
      if (custom !== null && custom !== undefined) return simp(custom);
      return simp(integrateLocal(node, variable));
    },
    differentiate: (node, variable) => {
      const custom = options.differentiate?.(node, variable);
      if (custom !== null && custom !== undefined) return simp(custom);
      return simp(differentiateLocal(node, variable));
    },
  };
}

function solveLinearFirstOrder(p: IRNode, q: IRNode, y: IRNode, x: IRNode, ops: Ops): IRNode {
  const intP = ops.integrate(p, x);
  const mu = ops.simp(exp(intP));
  const intMuQ = ops.integrate(mul(mu, q), x);
  return app(EQUAL, [y, ops.simp(div(add(intMuQ, C), mu))]);
}

function trySeparable(expr: IRNode, y: IRNode, x: IRNode, ops: Ops): IRNode | null {
  const extracted = extractFirstOrderRhs(expr, y, x);
  if (extracted === null) return null;
  const rhs = ops.simp(extracted);
  if (isConstWrt(rhs, y)) return app(EQUAL, [y, ops.simp(add(ops.integrate(rhs, x), C))]);
  if (isConstWrt(rhs, x)) {
    const lhs = ops.integrate(div(ONE, rhs), y);
    return app(EQUAL, [lhs, add(x, C)]);
  }

  const product = splitSeparableProduct(rhs, y, x);
  if (product === null) return null;
  const lhs = ops.integrate(div(ONE, product.gy), y);
  const rhsInt = ops.integrate(product.fx, x);
  return app(EQUAL, [lhs, ops.simp(add(rhsInt, C))]);
}

function tryBernoulli(expr: IRNode, y: IRNode, x: IRNode, ops: Ops): IRNode | null {
  const yPrime = deriv(y, x);
  let yPrimeCoeff = Frac.zero();
  const pTerms: IRNode[] = [];
  const qTerms: IRNode[] = [];
  let power: bigint | null = null;

  for (const term of flattenAdd(expr)) {
    const coeffBase = extractRationalCoeff(term);
    if (equals(coeffBase.base, yPrime)) {
      yPrimeCoeff = yPrimeCoeff.add(coeffBase.coeff);
      continue;
    }

    const yPow = extractPowerOfYTerm(term, y);
    if (yPow !== null) {
      if (power === null) power = yPow.power;
      if (power !== yPow.power) return null;
      qTerms.push(neg(yPow.coeff));
      continue;
    }

    const yTerm = extractYFactor(term, y);
    if (yTerm !== null) {
      pTerms.push(yTerm);
      continue;
    }
    return null;
  }

  if (!yPrimeCoeff.eq(Frac.one()) || power === null) return null;
  const oneMinusN = new Frac(1n - power);
  const reducedP = ops.simp(mul(oneMinusN.toIr(), sum(pTerms)));
  const reducedQ = ops.simp(mul(oneMinusN.toIr(), sum(qTerms)));
  const linear = solveLinearFirstOrder(reducedP, reducedQ, y, x, ops);
  const rhs = linear.kind === "apply" && equals(linear.head, EQUAL) ? linear.args[1] : null;
  if (rhs === null) return null;
  return app(EQUAL, [y, ops.simp(pow(rhs, Frac.one().div(oneMinusN).toIr()))]);
}

function tryExact(expr: IRNode, y: IRNode, x: IRNode, ops: Ops): IRNode | null {
  const yPrime = deriv(y, x);
  const mParts: IRNode[] = [];
  const nParts: IRNode[] = [];

  for (const term of flattenAdd(expr)) {
    const withoutDerivative = removeFactor(term, yPrime);
    if (withoutDerivative !== null) {
      nParts.push(withoutDerivative);
    } else {
      mParts.push(term);
    }
  }

  if (nParts.length === 0) return null;
  const M = ops.simp(sum(mParts));
  const N = ops.simp(sum(nParts));
  const dMdy = ops.simp(ops.differentiate(M, y));
  const dNdx = ops.simp(ops.differentiate(N, x));
  if (!equals(dMdy, dNdx)) return null;

  const F = ops.integrate(M, x);
  if (isApplyOf(F, INTEGRATE)) return null;
  const dFdy = ops.differentiate(F, y);
  const gPrime = ops.simp(sub(N, dFdy));
  const g = ops.integrate(gPrime, y);
  if (isApplyOf(g, INTEGRATE)) return null;
  return app(EQUAL, [ops.simp(add(F, g)), C]);
}

export function substRatioIr(node: IRNode, y: IRNode, x: IRNode, v: IRNode): IRNode | null {
  if (equals(node, y)) return null;
  if (node.kind !== "apply") return node;

  if (headName(node.head) === DIV.name && node.args.length === 2 && equals(node.args[0], y) && equals(node.args[1], x)) {
    return v;
  }

  const args: IRNode[] = [];
  for (const arg of node.args) {
    const substituted = substRatioIr(arg, y, x, v);
    if (substituted === null) return null;
    args.push(substituted);
  }
  return app(node.head, args);
}

function tryHomogeneousType(expr: IRNode, y: IRNode, x: IRNode, ops: Ops): IRNode | null {
  const extracted = extractFirstOrderRhs(expr, y, x);
  if (extracted === null) return null;

  const rhs = ops.simp(extracted);
  if (isConstWrt(rhs, y)) return null;

  const v = sym("_hom_v");
  const fRaw = substRatioIr(rhs, y, x, v);
  if (fRaw === null) return null;

  const f = ops.simp(fRaw);
  if (!isConstWrt(f, x)) return null;
  if (equals(f, v)) return app(EQUAL, [y, ops.simp(mul(C, x))]);

  const denom = simplifyLinearInVariable(ops.simp(sub(f, v)), v);
  const integrand = ops.simp(div(ONE, denom));
  const hV = ops.integrate(integrand, v);
  const yOverX = div(y, x);
  const hYX = ops.simp(substIr(hV, v, yOverX));
  return app(EQUAL, [hYX, ops.simp(add(app(LOG, [x]), C))]);
}

function simplifyLinearInVariable(node: IRNode, variable: IRNode): IRNode {
  let coeff = Frac.zero();
  for (const term of flattenAdd(node)) {
    const termCoeff = extractLinearCoeff(term, variable);
    if (termCoeff === null) return node;
    coeff = coeff.add(termCoeff);
  }
  return mul(coeff.toIr(), variable);
}

function collectSecondOrderCoeffs(expr: IRNode, y: IRNode, x: IRNode): { a: Frac; b: Frac; c: Frac } | null {
  const yPrime = deriv(y, x);
  const yDouble = deriv(yPrime, x);
  let a = Frac.zero();
  let b = Frac.zero();
  let c = Frac.zero();
  let matched = 0;

  for (const term of flattenAdd(expr)) {
    const { coeff, base } = extractRationalCoeff(term);
    if (equals(base, yDouble)) {
      a = a.add(coeff);
      matched += 1;
    } else if (equals(base, yPrime)) {
      b = b.add(coeff);
      matched += 1;
    } else if (equals(base, y)) {
      c = c.add(coeff);
      matched += 1;
    } else {
      return null;
    }
  }

  return a.isZero() || matched < 2 ? null : { a, b, c };
}

function solveSecondOrderConstCoeff(a: Frac, b: Frac, c: Frac, y: IRNode, x: IRNode): IRNode {
  const roots = characteristicRoots(a, b, c);
  let solution: IRNode;
  if (roots.kind === "distinct") {
    solution = add(mul(C1, expR(roots.r1, x)), mul(C2, expR(roots.r2, x)));
  } else if (roots.kind === "repeated") {
    solution = mul(add(C1, mul(C2, x)), expR(roots.r, x));
  } else {
    const phase = mul(roots.beta, x);
    solution = mul(exp(mul(roots.alpha, x)), add(mul(C1, app(COS, [phase])), mul(C2, app(SIN, [phase]))));
  }
  return app(EQUAL, [y, localSimplify(solution)]);
}

function tryEulerCauchy(expr: IRNode, y: IRNode, x: IRNode): IRNode | null {
  const yPrime = deriv(y, x);
  const yDouble = deriv(yPrime, x);
  const xSq = pow(x, TWO);
  let a = Frac.zero();
  let b = Frac.zero();
  let c = Frac.zero();
  let matched = 0;

  for (const term of flattenAdd(expr)) {
    const product = flattenProduct(term);
    const factors = product.factors;
    if (factors.length === 1 && equals(factors[0], y)) {
      c = c.add(product.coeff);
      matched += 1;
      continue;
    }
    if (factors.length === 2 && hasFactors(factors, x, yPrime)) {
      b = b.add(product.coeff);
      matched += 1;
      continue;
    }
    if (factors.length === 2 && hasFactors(factors, xSq, yDouble)) {
      a = a.add(product.coeff);
      matched += 1;
      continue;
    }
    return null;
  }

  if (a.isZero() || matched < 2) return null;
  return solveEulerCauchy(a, b, c, y, x);
}

function solveEulerCauchy(a: Frac, b: Frac, c: Frac, y: IRNode, x: IRNode): IRNode {
  const roots = characteristicRoots(a, b.sub(a), c);
  const logX = app(LOG, [x]);
  let solution: IRNode;
  if (roots.kind === "distinct") {
    solution = add(mul(C1, pow(x, roots.r1)), mul(C2, pow(x, roots.r2)));
  } else if (roots.kind === "repeated") {
    solution = mul(add(C1, mul(C2, logX)), pow(x, roots.r));
  } else {
    const phase = mul(roots.beta, logX);
    solution = mul(pow(x, roots.alpha), add(mul(C1, app(COS, [phase])), mul(C2, app(SIN, [phase]))));
  }
  return app(EQUAL, [y, localSimplify(solution)]);
}

function trySecondOrderNonhom(expr: IRNode, y: IRNode, x: IRNode, ops: Ops): IRNode | null {
  const collected = collectSecondOrderNonhom(expr, y, x);
  if (collected === null) return null;
  const hom = solveSecondOrderConstCoeff(collected.a, collected.b, collected.c, y, x);
  const yh = hom.kind === "apply" && equals(hom.head, EQUAL) ? hom.args[1] : ZERO;
  const forcing = classifyForcing(collected.f, x);
  const particular = forcing === null
    ? null
    : computeParticular(collected.a, collected.b, collected.c, forcing, x);
  const yp = particular ?? variationOfParameters(collected.a, collected.b, collected.c, collected.f, x, ops);
  return app(EQUAL, [y, ops.simp(add(yh, yp))]);
}

function collectSecondOrderNonhom(expr: IRNode, y: IRNode, x: IRNode): { a: Frac; b: Frac; c: Frac; f: IRNode } | null {
  const yPrime = deriv(y, x);
  const yDouble = deriv(yPrime, x);
  let a = Frac.zero();
  let b = Frac.zero();
  let c = Frac.zero();
  const forcing: IRNode[] = [];

  for (const term of flattenAdd(expr)) {
    const { coeff, base } = extractRationalCoeff(term);
    if (equals(base, yDouble)) {
      a = a.add(coeff);
    } else if (equals(base, yPrime)) {
      b = b.add(coeff);
    } else if (equals(base, y)) {
      c = c.add(coeff);
    } else if (isConstWrt(term, y)) {
      forcing.push(neg(term));
    } else {
      return null;
    }
  }

  if (a.isZero() || forcing.length === 0) return null;
  return { a, b, c, f: sum(forcing) };
}

function classifyForcing(f: IRNode, x: IRNode): ClassifiedForcing | null {
  const poly = polynomialCoeffs(f, x, 2);
  if (poly !== null) return ["poly", poly];
  const expArg = unaryArg(f, EXP);
  if (expArg !== undefined) {
    const alpha = extractLinearCoeff(expArg, x);
    if (alpha !== null) return ["exp", alpha];
  }
  const sinArg = unaryArg(f, SIN);
  if (sinArg !== undefined) {
    const beta = extractLinearCoeff(sinArg, x);
    if (beta !== null && beta.gtZero()) return ["sin", beta];
  }
  const cosArg = unaryArg(f, COS);
  if (cosArg !== undefined) {
    const beta = extractLinearCoeff(cosArg, x);
    if (beta !== null && beta.gtZero()) return ["cos", beta];
  }
  return null;
}

function computeParticular(a: Frac, b: Frac, c: Frac, forcing: ClassifiedForcing, x: IRNode): IRNode | null {
  if (forcing[0] === "poly") return polynomialParticular(a, b, c, forcing[1], x);
  if (forcing[0] === "exp") {
    const alpha = forcing[1];
    const denom = charAt(a, b, c, alpha);
    return denom.isZero() ? null : div(exp(mul(alpha.toIr(), x)), denom.toIr());
  }
  if (forcing[0] === "sin" || forcing[0] === "cos") {
    const beta = forcing[1];
    const m = c.sub(a.mul(beta).mul(beta));
    const n = b.mul(beta);
    const det = m.mul(m).add(n.mul(n));
    if (det.isZero()) return null;
    const cosTarget = forcing[0] === "cos" ? Frac.one() : Frac.zero();
    const sinTarget = forcing[0] === "sin" ? Frac.one() : Frac.zero();
    const A = cosTarget.mul(m).sub(sinTarget.mul(n)).div(det);
    const B = sinTarget.mul(m).add(cosTarget.mul(n)).div(det);
    return add(mul(A.toIr(), app(COS, [mul(beta.toIr(), x)])), mul(B.toIr(), app(SIN, [mul(beta.toIr(), x)])));
  }
  return null;
}

function polynomialParticular(a: Frac, b: Frac, c: Frac, coeffs: readonly Frac[], x: IRNode): IRNode | null {
  const degree = coeffs.length - 1;
  const unknownCount = degree + 3;
  const equations = unknownCount;
  const matrix: Frac[][] = [];
  for (let row = 0; row < equations; row += 1) {
    const line = Array.from({ length: unknownCount + 1 }, () => Frac.zero());
    for (let k = 0; k < unknownCount; k += 1) {
      if (k === row) line[k] = line[k].add(c);
      if (k >= 1 && k - 1 === row) line[k] = line[k].add(b.mul(new Frac(k)));
      if (k >= 2 && k - 2 === row) line[k] = line[k].add(a.mul(new Frac(k)).mul(new Frac(k - 1)));
    }
    line[unknownCount] = coeffs[row] ?? Frac.zero();
    matrix.push(line);
  }
  const solution = solveLinearSystem(matrix, unknownCount);
  if (solution === null) return null;
  const terms: IRNode[] = [];
  solution.forEach((coeff, powerIndex) => {
    if (coeff.isZero()) return;
    const base = powerIndex === 0 ? ONE : powerIndex === 1 ? x : pow(x, int(powerIndex));
    terms.push(coeff.isOne() ? base : mul(coeff.toIr(), base));
  });
  return terms.length === 0 ? ZERO : sum(terms);
}

function variationOfParameters(a: Frac, b: Frac, c: Frac, f: IRNode, x: IRNode, ops: Ops): IRNode {
  const roots = characteristicRoots(a, b, c);
  const g = div(f, a.toIr());
  let y1: IRNode;
  let y2: IRNode;
  let wronskian: IRNode;

  if (roots.kind === "distinct") {
    y1 = expR(roots.r1, x);
    y2 = expR(roots.r2, x);
    wronskian = mul(sub(roots.r2, roots.r1), exp(mul(add(roots.r1, roots.r2), x)));
  } else if (roots.kind === "repeated") {
    y1 = expR(roots.r, x);
    y2 = mul(x, y1);
    wronskian = exp(mul(mul(TWO, roots.r), x));
  } else {
    const phase = mul(roots.beta, x);
    const envelope = exp(mul(roots.alpha, x));
    y1 = mul(envelope, app(COS, [phase]));
    y2 = mul(envelope, app(SIN, [phase]));
    wronskian = mul(roots.beta, exp(mul(mul(TWO, roots.alpha), x)));
  }

  const u1Prime = neg(div(mul(y2, g), wronskian));
  const u2Prime = div(mul(y1, g), wronskian);
  return ops.simp(add(mul(neg(y1), ops.integrate(div(mul(y2, g), wronskian), x)), mul(y2, ops.integrate(u2Prime, x))));
}

function characteristicRoots(a: Frac, b: Frac, c: Frac):
  | { readonly kind: "distinct"; readonly r1: IRNode; readonly r2: IRNode }
  | { readonly kind: "repeated"; readonly r: IRNode }
  | { readonly kind: "complex"; readonly alpha: IRNode; readonly beta: IRNode } {
  const disc = b.mul(b).sub(new Frac(4).mul(a).mul(c));
  if (disc.gtZero()) {
    const sqrtDisc = exactSqrt(disc);
    const twoA = new Frac(2).mul(a);
    if (sqrtDisc !== null) {
      return {
        kind: "distinct",
        r1: b.neg().add(sqrtDisc).div(twoA).toIr(),
        r2: b.neg().sub(sqrtDisc).div(twoA).toIr(),
      };
    }
    const sqrtIr = pow(disc.toIr(), HALF);
    return {
      kind: "distinct",
      r1: div(add(b.neg().toIr(), sqrtIr), twoA.toIr()),
      r2: div(sub(b.neg().toIr(), sqrtIr), twoA.toIr()),
    };
  }
  if (disc.isZero()) return { kind: "repeated", r: b.neg().div(new Frac(2).mul(a)).toIr() };
  const alpha = b.neg().div(new Frac(2).mul(a)).toIr();
  const betaSq = disc.neg().div(new Frac(4).mul(a).mul(a));
  const beta = exactSqrt(betaSq)?.toIr() ?? pow(betaSq.toIr(), HALF);
  return { kind: "complex", alpha, beta };
}

function collectLinearFirstOrder(expr: IRNode, y: IRNode, x: IRNode): { p: IRNode; q: IRNode } | null {
  const yPrime = deriv(y, x);
  let yPrimeCoeff = Frac.zero();
  const pTerms: IRNode[] = [];
  const qTerms: IRNode[] = [];

  for (const term of flattenAdd(expr)) {
    const coeffBase = extractRationalCoeff(term);
    if (equals(coeffBase.base, yPrime)) {
      yPrimeCoeff = yPrimeCoeff.add(coeffBase.coeff);
      continue;
    }
    const yFactor = extractYFactor(term, y);
    if (yFactor !== null) {
      pTerms.push(yFactor);
      continue;
    }
    if (isConstWrt(term, y)) {
      qTerms.push(neg(term));
      continue;
    }
    return null;
  }

  if (yPrimeCoeff.isZero()) return null;
  return {
    p: div(sum(pTerms), yPrimeCoeff.toIr()),
    q: div(sum(qTerms), yPrimeCoeff.toIr()),
  };
}

function extractFirstOrderRhs(expr: IRNode, y: IRNode, x: IRNode): IRNode | null {
  const yPrime = deriv(y, x);
  const rhsTerms: IRNode[] = [];
  let seen = false;
  for (const term of flattenAdd(expr)) {
    if (equals(term, yPrime)) {
      seen = true;
    } else {
      const coeffBase = extractRationalCoeff(term);
      if (equals(coeffBase.base, yPrime) && coeffBase.coeff.eq(Frac.one())) {
        seen = true;
      } else if (equals(coeffBase.base, yPrime)) {
        return null;
      } else {
        rhsTerms.push(neg(term));
      }
    }
  }
  return seen ? sum(rhsTerms) : null;
}

function splitSeparableProduct(rhs: IRNode, y: IRNode, x: IRNode): { fx: IRNode; gy: IRNode } | null {
  const product = flattenProduct(rhs);
  const fx: IRNode[] = [];
  const gy: IRNode[] = [];
  if (!product.coeff.isOne()) fx.push(product.coeff.toIr());
  for (const factor of product.factors) {
    if (isConstWrt(factor, y)) {
      fx.push(factor);
    } else if (isConstWrt(factor, x)) {
      gy.push(factor);
    } else {
      return null;
    }
  }
  if (fx.length === 0 || gy.length === 0) return null;
  return { fx: productOf(fx), gy: productOf(gy) };
}

function extractPowerOfYTerm(term: IRNode, y: IRNode): { coeff: IRNode; power: bigint } | null {
  const product = flattenProduct(term);
  const factors = [...product.factors];
  const index = factors.findIndex((factor) => {
    const args = applyArgs(factor, POW);
    return args !== undefined
      && args.length === 2
      && equals(args[0], y)
      && args[1].kind === "integer"
      && args[1].value !== 0n
      && args[1].value !== 1n;
  });
  if (index < 0) return null;
  const powNode = factors[index];
  const power = applyArgs(powNode, POW)?.[1];
  if (power?.kind !== "integer") return null;
  factors.splice(index, 1);
  const coeffParts = product.coeff.isOne() ? factors : [product.coeff.toIr(), ...factors];
  const coeff = productOf(coeffParts);
  return isConstWrt(coeff, y) ? { coeff, power: power.value } : null;
}

function extractYFactor(term: IRNode, y: IRNode): IRNode | null {
  const product = flattenProduct(term);
  const factors = [...product.factors];
  const index = factors.findIndex((factor) => equals(factor, y));
  if (index < 0) return null;
  factors.splice(index, 1);
  const coeffParts = product.coeff.isOne() ? factors : [product.coeff.toIr(), ...factors];
  const coeff = productOf(coeffParts);
  return isConstWrt(coeff, y) ? coeff : null;
}

function removeFactor(term: IRNode, factor: IRNode): IRNode | null {
  const product = flattenProduct(term);
  const factors = [...product.factors];
  const index = factors.findIndex((candidate) => equals(candidate, factor));
  if (index < 0) return null;
  factors.splice(index, 1);
  return productOf(product.coeff.isOne() ? factors : [product.coeff.toIr(), ...factors]);
}

function flattenAdd(node: IRNode): IRNode[] {
  if (node.kind === "apply" && headName(node.head) === ADD.name) return node.args.flatMap(flattenAdd);
  if (node.kind === "apply" && headName(node.head) === SUB.name && node.args.length === 2) {
    return [...flattenAdd(node.args[0]), ...flattenAdd(neg(node.args[1]))];
  }
  if (node.kind === "apply" && headName(node.head) === NEG.name && node.args.length === 1) {
    return flattenAdd(node.args[0]).map(neg);
  }
  return [node];
}

function flattenProduct(node: IRNode): { coeff: Frac; factors: IRNode[] } {
  if (node.kind === "apply" && headName(node.head) === MUL.name) {
    return node.args.reduce(
      (acc, arg) => {
        const rhs = flattenProduct(arg);
        return { coeff: acc.coeff.mul(rhs.coeff), factors: [...acc.factors, ...rhs.factors] };
      },
      { coeff: Frac.one(), factors: [] as IRNode[] },
    );
  }
  if (node.kind === "apply" && headName(node.head) === NEG.name && node.args.length === 1) {
    const inner = flattenProduct(node.args[0]);
    return { coeff: inner.coeff.neg(), factors: inner.factors };
  }
  const literal = Frac.fromNode(node);
  if (literal !== null) return { coeff: literal, factors: [] };
  return { coeff: Frac.one(), factors: [node] };
}

function extractRationalCoeff(term: IRNode): { coeff: Frac; base: IRNode } {
  const product = flattenProduct(term);
  return { coeff: product.coeff, base: productOf(product.factors) };
}

function polynomialCoeffs(node: IRNode, variable: IRNode, maxDegree: number): Frac[] | null {
  if (equals(node, variable)) return [Frac.zero(), Frac.one()];
  const lit = Frac.fromNode(node);
  if (lit !== null) return [lit];
  if (node.kind === "symbol") return null;
  if (node.kind !== "apply") return null;
  const name = headName(node.head);
  if (name === ADD.name) {
    let out = [Frac.zero()];
    for (const arg of node.args) {
      const rhs = polynomialCoeffs(arg, variable, maxDegree);
      if (rhs === null) return null;
      out = coeffsAdd(out, rhs);
    }
    return trimDegree(out, maxDegree);
  }
  if (name === SUB.name && node.args.length === 2) {
    const lhs = polynomialCoeffs(node.args[0], variable, maxDegree);
    const rhs = polynomialCoeffs(node.args[1], variable, maxDegree);
    return lhs === null || rhs === null ? null : trimDegree(coeffsSub(lhs, rhs), maxDegree);
  }
  if (name === NEG.name && node.args.length === 1) {
    const inner = polynomialCoeffs(node.args[0], variable, maxDegree);
    return inner?.map((c) => c.neg()) ?? null;
  }
  if (name === MUL.name) {
    let out = [Frac.one()];
    for (const arg of node.args) {
      const rhs = polynomialCoeffs(arg, variable, maxDegree);
      if (rhs === null) return null;
      out = coeffsMul(out, rhs);
      if (out.length > maxDegree + 1 && out.slice(maxDegree + 1).some((c) => !c.isZero())) return null;
    }
    return trimDegree(out, maxDegree);
  }
  if (name === POW.name && node.args.length === 2 && node.args[1].kind === "integer" && node.args[1].value >= 0n) {
    let out = [Frac.one()];
    const base = polynomialCoeffs(node.args[0], variable, maxDegree);
    if (base === null || node.args[1].value > BigInt(maxDegree)) return null;
    for (let i = 0n; i < node.args[1].value; i += 1n) out = coeffsMul(out, base);
    return trimDegree(out, maxDegree);
  }
  return null;
}

function differentiateLocal(node: IRNode, variable: IRNode): IRNode {
  if (equals(node, variable)) return ONE;
  if (node.kind !== "apply") return ZERO;
  const name = headName(node.head);
  if (name === ADD.name) return sum(node.args.map((arg) => differentiateLocal(arg, variable)));
  if (name === SUB.name && node.args.length === 2) return sub(differentiateLocal(node.args[0], variable), differentiateLocal(node.args[1], variable));
  if (name === NEG.name && node.args.length === 1) return neg(differentiateLocal(node.args[0], variable));
  if (name === MUL.name) {
    return sum(node.args.map((arg, index) => productOf(node.args.map((factor, factorIndex) => (
      index === factorIndex ? differentiateLocal(arg, variable) : factor
    )))));
  }
  if (name === DIV.name && node.args.length === 2) {
    const [u, v] = node.args;
    return div(sub(mul(differentiateLocal(u, variable), v), mul(u, differentiateLocal(v, variable))), pow(v, TWO));
  }
  if (name === POW.name && node.args.length === 2 && node.args[1].kind === "integer") {
    const n = node.args[1].value;
    if (n === 0n) return ZERO;
    return mul(int(n), mul(pow(node.args[0], int(n - 1n)), differentiateLocal(node.args[0], variable)));
  }
  if (name === EXP.name && node.args.length === 1) return mul(app(EXP, [node.args[0]]), differentiateLocal(node.args[0], variable));
  if (name === SIN.name && node.args.length === 1) return mul(app(COS, [node.args[0]]), differentiateLocal(node.args[0], variable));
  if (name === COS.name && node.args.length === 1) return neg(mul(app(SIN, [node.args[0]]), differentiateLocal(node.args[0], variable)));
  if (name === LOG.name && node.args.length === 1) return div(differentiateLocal(node.args[0], variable), node.args[0]);
  return app(D, [node, variable]);
}

function integrateLocal(node: IRNode, variable: IRNode): IRNode {
  const simplified = localSimplify(node);
  if (isConstWrt(simplified, variable)) return mul(simplified, variable);
  if (equals(simplified, variable)) return div(pow(variable, TWO), TWO);
  if (simplified.kind === "apply") {
    const name = headName(simplified.head);
    if (name === ADD.name) return sum(simplified.args.map((arg) => integrateLocal(arg, variable)));
    if (name === SUB.name && simplified.args.length === 2) return sub(integrateLocal(simplified.args[0], variable), integrateLocal(simplified.args[1], variable));
    if (name === NEG.name && simplified.args.length === 1) return neg(integrateLocal(simplified.args[0], variable));
    if (name === MUL.name) {
      const product = flattenProduct(simplified);
      const constants = product.factors.filter((factor) => isConstWrt(factor, variable));
      const rest = product.factors.filter((factor) => !isConstWrt(factor, variable));
      if (rest.length === 1 && (constants.length > 0 || !product.coeff.isOne())) {
        const constant = productOf(product.coeff.isOne() ? constants : [product.coeff.toIr(), ...constants]);
        return mul(constant, integrateLocal(rest[0], variable));
      }
    }
    if (name === POW.name && simplified.args.length === 2 && equals(simplified.args[0], variable) && simplified.args[1].kind === "integer") {
      const n = simplified.args[1].value;
      if (n === -1n) return app(LOG, [variable]);
      return div(pow(variable, int(n + 1n)), int(n + 1n));
    }
    if (name === DIV.name && simplified.args.length === 2 && equals(simplified.args[1], variable) && equals(simplified.args[0], ONE)) {
      return app(LOG, [variable]);
    }
    if (name === EXP.name && simplified.args.length === 1) {
      const coeff = extractLinearCoeff(simplified.args[0], variable);
      if (coeff !== null && !coeff.isZero()) return div(simplified, coeff.toIr());
    }
    if ((name === SIN.name || name === COS.name) && simplified.args.length === 1) {
      const coeff = extractLinearCoeff(simplified.args[0], variable);
      if (coeff !== null && !coeff.isZero()) {
        return name === SIN.name
          ? neg(div(app(COS, [simplified.args[0]]), coeff.toIr()))
          : div(app(SIN, [simplified.args[0]]), coeff.toIr());
      }
    }
  }
  return app(INTEGRATE, [simplified, variable]);
}

function localSimplify(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = localSimplify(node.head);
  const args = node.args.map(localSimplify);
  const name = headName(head);
  if (name === NEG.name && args.length === 1) return neg(args[0]);
  if (name === ADD.name) return sum(args);
  if (name === SUB.name && args.length === 2) return sub(args[0], args[1]);
  if (name === MUL.name) return productOf(args);
  if (name === DIV.name && args.length === 2) return div(args[0], args[1]);
  if (name === POW.name && args.length === 2) return pow(args[0], args[1]);
  if (name === EXP.name && args.length === 1 && isInteger(args[0], 0n)) return ONE;
  return app(head, args);
}

function extractLinearCoeff(node: IRNode, variable: IRNode): Frac | null {
  if (equals(node, variable)) return Frac.one();
  if (node.kind === "apply" && headName(node.head) === NEG.name && node.args.length === 1) {
    return extractLinearCoeff(node.args[0], variable)?.neg() ?? null;
  }
  const product = flattenProduct(node);
  if (product.factors.length === 1 && equals(product.factors[0], variable)) return product.coeff;
  return null;
}

function isConstWrt(node: IRNode, variable: IRNode): boolean {
  if (equals(node, variable)) return false;
  return node.kind !== "apply" || node.args.every((arg) => isConstWrt(arg, variable));
}

function substIr(node: IRNode, from: IRNode, to: IRNode): IRNode {
  if (equals(node, from)) return to;
  if (node.kind !== "apply") return node;
  return app(node.head, node.args.map((arg) => substIr(arg, from, to)));
}

function applyArgs(node: IRNode, head: IRNode): readonly IRNode[] | undefined {
  return node.kind === "apply" && equals(node.head, head) ? node.args : undefined;
}

function unaryArg(node: IRNode, head: IRNode): IRNode | undefined {
  const args = applyArgs(node, head);
  return args !== undefined && args.length === 1 ? args[0] : undefined;
}

function isApplyOf(node: IRNode, head: IRNode): boolean {
  return node.kind === "apply" && equals(node.head, head);
}

function hasFactors(factors: readonly IRNode[], a: IRNode, b: IRNode): boolean {
  return (equals(factors[0], a) && equals(factors[1], b)) || (equals(factors[0], b) && equals(factors[1], a));
}

function expR(r: IRNode, x: IRNode): IRNode {
  return exp(mul(r, x));
}

function deriv(expr: IRNode, variable: IRNode): IRNode {
  return app(D, [expr, variable]);
}

function add(...args: IRNode[]): IRNode {
  return sum(args);
}

function sum(args: readonly IRNode[]): IRNode {
  const flat = args.flatMap((arg) => (arg.kind === "apply" && headName(arg.head) === ADD.name ? [...arg.args] : [arg]));
  const kept: IRNode[] = [];
  let literal = Frac.zero();
  for (const arg of flat) {
    const f = Frac.fromNode(arg);
    if (f !== null) literal = literal.add(f);
    else if (!isInteger(arg, 0n)) {
      const negArg = unwrapNeg(arg);
      const cancelIndex = negArg === null
        ? kept.findIndex((candidate) => {
          const inner = unwrapNeg(candidate);
          return inner !== null && equals(inner, arg);
        })
        : kept.findIndex((candidate) => equals(candidate, negArg));
      if (cancelIndex >= 0) kept.splice(cancelIndex, 1);
      else kept.push(arg);
    }
  }
  if (!literal.isZero()) kept.unshift(literal.toIr());
  if (kept.length === 0) return ZERO;
  if (kept.length === 1) return kept[0];
  return app(ADD, kept);
}

function sub(lhs: IRNode, rhs: IRNode): IRNode {
  return equals(rhs, ZERO) ? lhs : sum([lhs, neg(rhs)]);
}

function mul(...args: IRNode[]): IRNode {
  return productOf(args);
}

function productOf(args: readonly IRNode[]): IRNode {
  const flat = args.flatMap((arg) => (arg.kind === "apply" && headName(arg.head) === MUL.name ? [...arg.args] : [arg]));
  const kept: IRNode[] = [];
  let literal = Frac.one();
  for (const arg of flat) {
    const f = Frac.fromNode(arg);
    if (f !== null) literal = literal.mul(f);
    else if (!isInteger(arg, 1n)) kept.push(arg);
  }
  for (let i = 0; i < kept.length; i += 1) {
    const divArgs = applyArgs(kept[i], DIV);
    const denom = divArgs?.length === 2 ? Frac.fromNode(divArgs[1]) : null;
    if (divArgs !== undefined && denom !== null && !literal.isOne()) {
      literal = literal.div(denom);
      kept[i] = divArgs[0];
    }
  }
  if (literal.isZero()) return ZERO;
  if (!literal.isOne()) kept.unshift(literal.toIr());
  if (kept.length === 0) return ONE;
  if (kept.length === 1) return kept[0];
  return app(MUL, kept);
}

function div(lhs: IRNode, rhs: IRNode): IRNode {
  if (equals(lhs, ZERO)) return ZERO;
  if (equals(rhs, ONE)) return lhs;
  const lf = Frac.fromNode(lhs);
  const rf = Frac.fromNode(rhs);
  if (lf !== null && rf !== null) return lf.div(rf).toIr();
  return app(DIV, [lhs, rhs]);
}

function pow(base: IRNode, exponent: IRNode): IRNode {
  if (isInteger(exponent, 0n)) return ONE;
  if (isInteger(exponent, 1n)) return base;
  if (base.kind === "integer" && exponent.kind === "integer" && exponent.value >= 0n) return int(base.value ** exponent.value);
  return app(POW, [base, exponent]);
}

function exp(arg: IRNode): IRNode {
  return isInteger(arg, 0n) ? ONE : app(EXP, [arg]);
}

function neg(node: IRNode): IRNode {
  const f = Frac.fromNode(node);
  if (f !== null) return f.neg().toIr();
  if (node.kind === "apply" && headName(node.head) === NEG.name && node.args.length === 1) return node.args[0];
  return app(NEG, [node]);
}

function unwrapNeg(node: IRNode): IRNode | null {
  return node.kind === "apply" && headName(node.head) === NEG.name && node.args.length === 1 ? node.args[0] : null;
}

function isInteger(node: IRNode, value: bigint): boolean {
  return node.kind === "integer" && node.value === value;
}

function coeffsAdd(lhs: readonly Frac[], rhs: readonly Frac[]): Frac[] {
  const n = Math.max(lhs.length, rhs.length);
  return Array.from({ length: n }, (_, index) => (lhs[index] ?? Frac.zero()).add(rhs[index] ?? Frac.zero()));
}

function coeffsSub(lhs: readonly Frac[], rhs: readonly Frac[]): Frac[] {
  const n = Math.max(lhs.length, rhs.length);
  return Array.from({ length: n }, (_, index) => (lhs[index] ?? Frac.zero()).sub(rhs[index] ?? Frac.zero()));
}

function coeffsMul(lhs: readonly Frac[], rhs: readonly Frac[]): Frac[] {
  const out = Array.from({ length: lhs.length + rhs.length - 1 }, () => Frac.zero());
  for (let i = 0; i < lhs.length; i += 1) {
    for (let j = 0; j < rhs.length; j += 1) out[i + j] = out[i + j].add(lhs[i].mul(rhs[j]));
  }
  return out;
}

function trimDegree(coeffs: readonly Frac[], maxDegree: number): Frac[] | null {
  if (coeffs.slice(maxDegree + 1).some((coeff) => !coeff.isZero())) return null;
  let end = Math.min(coeffs.length, maxDegree + 1);
  while (end > 1 && coeffs[end - 1].isZero()) end -= 1;
  return coeffs.slice(0, end);
}

function charAt(a: Frac, b: Frac, c: Frac, r: Frac): Frac {
  return a.mul(r).mul(r).add(b.mul(r)).add(c);
}

function exactSqrt(value: Frac): Frac | null {
  if (value.ltZero()) return null;
  const n = integerSqrt(value.numer);
  const d = integerSqrt(value.denom);
  return n === null || d === null ? null : new Frac(n, d);
}

function integerSqrt(value: bigint): bigint | null {
  if (value < 0n) return null;
  if (value < 2n) return value;
  let lo = 1n;
  let hi = value;
  while (lo <= hi) {
    const mid = (lo + hi) / 2n;
    const sq = mid * mid;
    if (sq === value) return mid;
    if (sq < value) lo = mid + 1n;
    else hi = mid - 1n;
  }
  return null;
}

function solveLinearSystem(matrix: Frac[][], unknownCount: number): Frac[] | null {
  let row = 0;
  const pivots: number[] = [];
  for (let col = 0; col < unknownCount && row < matrix.length; col += 1) {
    const pivot = matrix.findIndex((line, index) => index >= row && !line[col].isZero());
    if (pivot < 0) continue;
    [matrix[row], matrix[pivot]] = [matrix[pivot], matrix[row]];
    const scale = matrix[row][col];
    matrix[row] = matrix[row].map((entry) => entry.div(scale));
    for (let r = 0; r < matrix.length; r += 1) {
      if (r === row || matrix[r][col].isZero()) continue;
      const factor = matrix[r][col];
      matrix[r] = matrix[r].map((entry, index) => entry.sub(factor.mul(matrix[row][index])));
    }
    pivots[row] = col;
    row += 1;
  }
  if (matrix.some((line) => line.slice(0, unknownCount).every((entry) => entry.isZero()) && !line[unknownCount].isZero())) {
    return null;
  }
  const out = Array.from({ length: unknownCount }, () => Frac.zero());
  for (let r = 0; r < pivots.length; r += 1) out[pivots[r]] = matrix[r][unknownCount];
  return out;
}

function toBigInt(value: bigint | number | string): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) throw new RangeError("number inputs must be safe integers");
  return BigInt(value);
}

function gcd(a: bigint, b: bigint): bigint {
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a === 0n ? 1n : a;
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}

// ============================================================================
// Phase 21 — Variable-coefficient named ODE recognition
//
// Recognises four classical second-order ODEs with variable polynomial
// coefficients by *numerical pattern matching*: the IR coefficient expressions
// P(x), Q(x), R(x) are evaluated at four canonical test points and compared
// against the expected analytic functions.  This is exact for polynomial
// coefficients (all that the ODE families use) and is robust enough for the
// handful of cases we care about.
//
// Reading order:
//   evalAtXy          — recursive numeric evaluator for IR trees
//   evalIrAtX         — thin wrapper: evaluate x-only expressions
//   coeffMatchesFunc  — check IR node ≈ expected function at test points
//   extractConstVal   — extract float if node is constant w.r.t. x
//   splitOutFactor    — extract coefficient K from K·target in Mul/Neg tree
//   collectVar2Coeffs — extract (P, Q, R) from variable-coeff 2nd-order ODE
//   legendreNFromLambda — find n with n(n+1) = λ
//   nuFromRMinusXSq   — extract ν from R(x) = x² − ν² (Bessel)
//   buildNamedSolution — build Equal(y, %c1·F(n,x) + %c2·G(n,x))
//   tryLegendreOde, tryBesselOde, tryHermiteOde, tryChebyshevOde
//   tryVarCoeffNamedOde — Phase 21 dispatcher (called from solveOde)
// ============================================================================

// Four test x-values chosen to avoid singularities (|x| ≠ 1 for Legendre,
// x ≠ 0 for Bessel) while probing a representative range.
const VAR2_TEST_X = [0.3, 0.6, -0.25, 0.85] as const;

// Dummy y-symbol used when evaluating x-only coefficient expressions.
const DUMMY_Y_SYM = sym("__var2_dummy_y__");

/**
 * Recursively evaluate an IR node at concrete floating-point values of x and y.
 *
 * Supports Integer, Rational, Float, and the basic arithmetic/elementary
 * function heads (Add, Sub, Mul, Div, Neg, Pow, Exp, Log, Sin, Cos).
 * Throws a RangeError for unrecognised symbols or unsupported heads so that
 * the caller can catch and return null.
 */
function evalAtXy(node: IRNode, xSym: IRNode, ySym: IRNode, xVal: number, yVal: number): number {
  if (node.kind === "integer") return Number(node.value);
  if (node.kind === "rational") return Number(node.numer) / Number(node.denom);
  if (node.kind === "float") return node.value;
  if (node.kind === "symbol") {
    if (xSym.kind === "symbol" && node.name === xSym.name) return xVal;
    if (ySym.kind === "symbol" && node.name === ySym.name) return yVal;
    throw new RangeError(`Unknown symbol: ${node.name}`);
  }
  if (node.kind !== "apply") throw new RangeError("Unsupported node");
  const name = headName(node.head);
  const ev = (n: IRNode): number => evalAtXy(n, xSym, ySym, xVal, yVal);
  // n-ary Add and Mul (TypeScript IR uses multi-arg forms after simplification)
  if (name === ADD.name) return node.args.reduce((acc, arg) => acc + ev(arg), 0);
  if (name === MUL.name) return node.args.reduce((acc, arg) => acc * ev(arg), 1);
  if (name === SUB.name && node.args.length === 2) return ev(node.args[0]) - ev(node.args[1]);
  if (name === DIV.name && node.args.length === 2) {
    const dv = ev(node.args[1]);
    if (dv === 0) throw new RangeError("Division by zero");
    return ev(node.args[0]) / dv;
  }
  if (name === NEG.name && node.args.length === 1) return -ev(node.args[0]);
  if (name === POW.name && node.args.length === 2) return ev(node.args[0]) ** ev(node.args[1]);
  if (name === EXP.name && node.args.length === 1) return Math.exp(ev(node.args[0]));
  if (name === LOG.name && node.args.length === 1) return Math.log(Math.abs(ev(node.args[0])));
  if (name === SIN.name && node.args.length === 1) return Math.sin(ev(node.args[0]));
  if (name === COS.name && node.args.length === 1) return Math.cos(ev(node.args[0]));
  throw new RangeError(`Unsupported head: ${name}`);
}

/**
 * Evaluate an x-only IR expression at xv.  Returns null on any error
 * (unknown symbol, division by zero, unsupported head).
 */
function evalIrAtX(node: IRNode, x: IRNode, xv: number): number | null {
  try {
    return evalAtXy(node, x, DUMMY_Y_SYM, xv, 0);
  } catch {
    return null;
  }
}

/**
 * Return true iff `node` numerically agrees with `expected(xv)` at every
 * canonical test point.  Falls through to false on evaluation failures.
 */
function coeffMatchesFunc(
  node: IRNode,
  x: IRNode,
  expected: (xv: number) => number,
  tol = 1e-9,
): boolean {
  for (const xv of VAR2_TEST_X) {
    const actual = evalIrAtX(node, x, xv);
    if (actual === null) return false;
    let want: number;
    try {
      want = expected(xv);
    } catch {
      return false;
    }
    if (Math.abs(actual - want) > tol) return false;
  }
  return true;
}

/**
 * Return the float value of `node` if it is constant w.r.t. `x`.
 * Returns null if `node` contains `x` or if evaluation fails.
 */
function extractConstVal(node: IRNode, x: IRNode): number | null {
  if (!isConstWrt(node, x)) return null;
  return evalIrAtX(node, x, 0);
}

/**
 * Return the coefficient K such that term = K * target, or null.
 *
 * Handles nested Mul trees, Neg wrappers, and the degenerate case
 * term === target (coefficient = 1).  Works on both binary and n-ary Mul.
 *
 * Examples:
 *   splitOutFactor(Mul(Sub(1,Pow(x,2)), ypp), ypp)  → Sub(1,Pow(x,2))
 *   splitOutFactor(Neg(Mul(2, Mul(x, yp))), yp)     → Neg(Mul(2, x))
 *   splitOutFactor(ypp, yp)                          → null  (ypp ≠ yp)
 */
function splitOutFactor(term: IRNode, target: IRNode): IRNode | null {
  if (equals(term, target)) return ONE;
  if (term.kind !== "apply") return null;
  const name = headName(term.head);
  // Neg wrapper: negate the coefficient
  if (name === NEG.name && term.args.length === 1) {
    const inner = splitOutFactor(term.args[0], target);
    return inner !== null ? neg(inner) : null;
  }
  if (name === MUL.name) {
    const args = term.args;
    // Direct match: one of the factors IS the target
    for (let i = 0; i < args.length; i++) {
      if (equals(args[i], target)) {
        const rest = args.filter((_, j) => j !== i) as IRNode[];
        if (rest.length === 0) return ONE;
        if (rest.length === 1) return rest[0];
        return app(MUL, rest);
      }
    }
    // For binary Mul, recurse into sub-trees to handle nested Mul chains
    if (args.length === 2) {
      const coeffB = splitOutFactor(args[1], target);
      if (coeffB !== null) return mul(args[0], coeffB);
      const coeffA = splitOutFactor(args[0], target);
      if (coeffA !== null) return mul(coeffA, args[1]);
    }
  }
  return null;
}

/**
 * Extract (P, Q, R) from a variable-coefficient 2nd-order ODE:
 *   P(x)·y'' + Q(x)·y' + R(x)·y = 0
 *
 * Unlike collectSecondOrderCoeffs, P/Q/R may be arbitrary IR expressions
 * in x (not just rationals).  Returns null if any term does not fit the
 * pattern or if no y'' term is present.
 */
function collectVar2Coeffs(
  expr: IRNode,
  y: IRNode,
  x: IRNode,
): { p: IRNode; q: IRNode; r: IRNode } | null {
  const yPrime = deriv(y, x);
  const yDouble = deriv(yPrime, x);
  const pParts: IRNode[] = [];
  const qParts: IRNode[] = [];
  const rParts: IRNode[] = [];

  for (const term of flattenAdd(expr)) {
    const cp = splitOutFactor(term, yDouble);
    if (cp !== null) { pParts.push(cp); continue; }
    const cq = splitOutFactor(term, yPrime);
    if (cq !== null) { qParts.push(cq); continue; }
    const cr = splitOutFactor(term, y);
    if (cr !== null) { rParts.push(cr); continue; }
    return null;  // unrecognised term
  }

  if (pParts.length === 0) return null;  // no y'' term found
  return {
    p: pParts.length === 1 ? pParts[0] : sum(pParts),
    q: qParts.length === 0 ? ZERO : qParts.length === 1 ? qParts[0] : sum(qParts),
    r: rParts.length === 0 ? ZERO : rParts.length === 1 ? rParts[0] : sum(rParts),
  };
}

/**
 * Return the non-negative integer n such that n(n+1) = λ, or null.
 *
 * Uses the quadratic formula: n = (−1 + √(1+4λ)) / 2.
 *
 * Examples:
 *   legendreNFromLambda(0)   → 0   (0·1 = 0)
 *   legendreNFromLambda(6)   → 2   (2·3 = 6)
 *   legendreNFromLambda(5)   → null
 */
function legendreNFromLambda(lam: number): number | null {
  const disc = 1 + 4 * lam;
  if (disc < -1e-12) return null;
  const sqrtDisc = Math.sqrt(Math.max(disc, 0));
  const nFloat = (-1 + sqrtDisc) / 2;
  const n = Math.round(nFloat);
  if (n < 0) return null;
  if (Math.abs(nFloat - n) > 1e-7) return null;
  if (Math.abs(n * (n + 1) - lam) > 1e-7) return null;
  return n;
}

/**
 * Extract ν as a rational [p, q] from R(x) = x² − ν².
 *
 * Strategy: R(x) − x² must be constant (= −ν²).  Evaluate R at two points to
 * verify the quadratic shape, then determine ν = p/q (denominator ≤ 20) by
 * trial.  Returns [p, q] in lowest terms, or null.
 *
 * Examples:
 *   R(x) = x² − 4     → ν = 2,   returns [2, 1]
 *   R(x) = x² − 1/4   → ν = 1/2, returns [1, 2]
 *   R(x) = x² − 9/4   → ν = 3/2, returns [3, 2]
 */
function nuFromRMinusXSq(rNode: IRNode, x: IRNode): [number, number] | null {
  const r1 = evalIrAtX(rNode, x, 1.0);
  const r2 = evalIrAtX(rNode, x, 2.0);
  if (r1 === null || r2 === null) return null;
  // R(2) − R(1) should equal 4 − 1 = 3 for R(x) = x² + const
  if (Math.abs((r2 - r1) - 3) > 1e-8) return null;
  // ν² = x² − R(x) evaluated at x=1: ν² = 1 − R(1)
  const nuSq = 1 - r1;
  if (nuSq < -1e-12) return null;
  const nuSqPos = Math.max(nuSq, 0);
  // Trial-search for rational ν = p/q with denominator ≤ 20
  for (let q = 1; q <= 20; q++) {
    const pSq = nuSqPos * q * q;
    const p = Math.round(Math.sqrt(pSq));
    if (p >= 0 && Math.abs(p * p - pSq) < 1e-6) {
      const g = Number(gcd(BigInt(p), BigInt(q)));
      return [p / g, q / g];
    }
  }
  return null;
}

/**
 * Build Equal(y, %c1·head1(param, x) + %c2·head2(param, x)).
 *
 * Used by all four named-ODE solvers to assemble the general solution.
 */
function buildNamedSolution(h1: IRNode, h2: IRNode, paramIr: IRNode, y: IRNode, x: IRNode): IRNode {
  const sol1 = mul(C1, app(h1, [paramIr, x]));
  const sol2 = mul(C2, app(h2, [paramIr, x]));
  return app(EQUAL, [y, add(sol1, sol2)]);
}

// ---------------------------------------------------------------------------
// The four named-ODE recognisers
// ---------------------------------------------------------------------------

/**
 * Recognise the Legendre ODE: (1−x²)·y'' − 2x·y' + n(n+1)·y = 0.
 *
 * Checks:
 *   1. P(x) ≈ 1 − x²  at four test points.
 *   2. Q(x) ≈ −2x     at four test points.
 *   3. R is constant = n(n+1) for some non-negative integer n.
 *
 * Returns: Equal(y, %c1·LegendreP(n,x) + %c2·LegendreQ(n,x))
 */
function tryLegendreOde(expr: IRNode, y: IRNode, x: IRNode): IRNode | null {
  const coeffs = collectVar2Coeffs(expr, y, x);
  if (coeffs === null) return null;
  const { p, q, r } = coeffs;
  if (!coeffMatchesFunc(p, x, (xv) => 1 - xv * xv)) return null;
  if (!coeffMatchesFunc(q, x, (xv) => -2 * xv)) return null;
  const lam = extractConstVal(r, x);
  if (lam === null) return null;
  const n = legendreNFromLambda(lam);
  if (n === null) return null;
  return buildNamedSolution(LEGENDRE_P, LEGENDRE_Q, int(n), y, x);
}

/**
 * Recognise the Bessel ODE: x²·y'' + x·y' + (x²−ν²)·y = 0.
 *
 * Checks:
 *   1. P(x) ≈ x²   at four test points.
 *   2. Q(x) ≈ x    at four test points.
 *   3. R(x) = x² − ν² for some non-negative rational ν (denominator ≤ 20).
 *
 * Returns: Equal(y, %c1·BesselJ(ν,x) + %c2·BesselY(ν,x))
 */
function tryBesselOde(expr: IRNode, y: IRNode, x: IRNode): IRNode | null {
  const coeffs = collectVar2Coeffs(expr, y, x);
  if (coeffs === null) return null;
  const { p, q, r } = coeffs;
  if (!coeffMatchesFunc(p, x, (xv) => xv * xv)) return null;
  if (!coeffMatchesFunc(q, x, (xv) => xv)) return null;
  const nuPq = nuFromRMinusXSq(r, x);
  if (nuPq === null) return null;
  const [np, nq] = nuPq;
  const nuIr: IRNode = nq === 1 ? int(np) : rational(np, nq);
  return buildNamedSolution(BESSEL_J, BESSEL_Y, nuIr, y, x);
}

/**
 * Recognise the Hermite ODE: y'' − 2x·y' + 2n·y = 0.
 *
 * Checks:
 *   1. P ≡ 1 (constant).
 *   2. Q(x) ≈ −2x   at four test points.
 *   3. R is constant = 2n for some non-negative integer n.
 *
 * Returns: Equal(y, %c1·HermiteH(n,x) + %c2·HermiteH2(n,x))
 */
function tryHermiteOde(expr: IRNode, y: IRNode, x: IRNode): IRNode | null {
  const coeffs = collectVar2Coeffs(expr, y, x);
  if (coeffs === null) return null;
  const { p, q, r } = coeffs;
  const pVal = extractConstVal(p, x);
  if (pVal === null || Math.abs(pVal - 1) > 1e-9) return null;
  if (!coeffMatchesFunc(q, x, (xv) => -2 * xv)) return null;
  const rVal = extractConstVal(r, x);
  if (rVal === null || rVal < -1e-12) return null;
  const nFloat = rVal / 2;
  const n = Math.round(nFloat);
  if (n < 0 || Math.abs(nFloat - n) > 1e-9) return null;
  return buildNamedSolution(HERMITE_H, HERMITE_H2, int(n), y, x);
}

/**
 * Recognise the Chebyshev ODE: (1−x²)·y'' − x·y' + n²·y = 0.
 *
 * Checks:
 *   1. P(x) ≈ 1 − x²  at four test points.
 *   2. Q(x) ≈ −x       at four test points.
 *   3. R is constant = n² for some non-negative integer n.
 *
 * Checked before Legendre because both have P ≈ 1−x²; Chebyshev is
 * distinguished by Q ≈ −x while Legendre has Q ≈ −2x.
 *
 * Returns: Equal(y, %c1·ChebyshevT(n,x) + %c2·ChebyshevU(n,x))
 */
function tryChebyshevOde(expr: IRNode, y: IRNode, x: IRNode): IRNode | null {
  const coeffs = collectVar2Coeffs(expr, y, x);
  if (coeffs === null) return null;
  const { p, q, r } = coeffs;
  if (!coeffMatchesFunc(p, x, (xv) => 1 - xv * xv)) return null;
  if (!coeffMatchesFunc(q, x, (xv) => -xv)) return null;
  const rVal = extractConstVal(r, x);
  if (rVal === null || rVal < -1e-12) return null;
  const nFloat = Math.sqrt(Math.max(rVal, 0));
  const n = Math.round(nFloat);
  if (n < 0 || Math.abs(nFloat - n) > 1e-7 || Math.abs(n * n - rVal) > 1e-7) return null;
  return buildNamedSolution(CHEBYSHEV_T, CHEBYSHEV_U, int(n), y, x);
}

/**
 * Phase 21 dispatcher — try all four named variable-coefficient ODE families.
 *
 * Priority order:
 *   1. Chebyshev — before Legendre (both have P ≈ 1−x²; Q distinguishes them)
 *   2. Legendre  — (1−x²)y'' − 2xy' + n(n+1)y = 0
 *   3. Bessel    — x²y'' + xy' + (x²−ν²)y = 0
 *   4. Hermite   — y'' − 2xy' + 2ny = 0
 *
 * Called from solveOde after tryEulerCauchy.
 */
function tryVarCoeffNamedOde(expr: IRNode, y: IRNode, x: IRNode): IRNode | null {
  return tryChebyshevOde(expr, y, x)
    ?? tryLegendreOde(expr, y, x)
    ?? tryBesselOde(expr, y, x)
    ?? tryHermiteOde(expr, y, x);
}
