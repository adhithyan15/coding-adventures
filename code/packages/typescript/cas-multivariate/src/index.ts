import {
  ADD,
  LIST,
  MUL,
  NEG,
  POW,
  RULE,
  SUB,
  app,
  headName,
  int,
  rational,
  sym,
  type IRApply,
  type IRNode,
  type IRSymbol,
} from "@coding-adventures/symbolic-ir";

export type IntegerLike = bigint | number | string;
export type FractionLike = Fraction | IntegerLike;
export type Monomial = readonly number[];
export type MonomialOrder = "lex" | "grlex" | "grevlex";

export const GROEBNER = sym("Groebner");
export const POLY_REDUCE = sym("PolyReduce");
export const IDEAL_SOLVE = sym("IdealSolve");

const MAX_BASIS_SIZE = 50;
const MAX_DEGREE = 8;

export class Fraction {
  readonly numer: bigint;
  readonly denom: bigint;

  constructor(numerInput: IntegerLike, denomInput: IntegerLike = 1n) {
    let numer = toBigInt(numerInput);
    let denom = toBigInt(denomInput);
    if (denom === 0n) throw new RangeError("Fraction denominator cannot be zero");
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

  static from(value: FractionLike): Fraction {
    return value instanceof Fraction ? value : new Fraction(value);
  }

  static zero(): Fraction {
    return new Fraction(0n);
  }

  static one(): Fraction {
    return new Fraction(1n);
  }

  isZero(): boolean {
    return this.numer === 0n;
  }

  neg(): Fraction {
    return new Fraction(-this.numer, this.denom);
  }

  add(rhs: FractionLike): Fraction {
    const other = Fraction.from(rhs);
    return new Fraction(this.numer * other.denom + other.numer * this.denom, this.denom * other.denom);
  }

  sub(rhs: FractionLike): Fraction {
    return this.add(Fraction.from(rhs).neg());
  }

  mul(rhs: FractionLike): Fraction {
    const other = Fraction.from(rhs);
    return new Fraction(this.numer * other.numer, this.denom * other.denom);
  }

  div(rhs: FractionLike): Fraction {
    const other = Fraction.from(rhs);
    if (other.numer === 0n) throw new RangeError("Fraction division by zero");
    return new Fraction(this.numer * other.denom, this.denom * other.numer);
  }

  pow(exp: number): Fraction {
    if (!Number.isInteger(exp) || exp < 0) throw new RangeError("Fraction.pow expects a non-negative integer");
    return new Fraction(this.numer ** BigInt(exp), this.denom ** BigInt(exp));
  }

  compare(rhs: FractionLike): number {
    const other = Fraction.from(rhs);
    const lhsWide = this.numer * other.denom;
    const rhsWide = other.numer * this.denom;
    return lhsWide < rhsWide ? -1 : lhsWide > rhsWide ? 1 : 0;
  }

  equals(rhs: FractionLike): boolean {
    const other = Fraction.from(rhs);
    return this.numer === other.numer && this.denom === other.denom;
  }

  toIR(): IRNode {
    return this.denom === 1n ? int(this.numer) : rational(this.numer, this.denom);
  }

  toString(): string {
    return this.denom === 1n ? this.numer.toString() : `${this.numer}/${this.denom}`;
  }
}

export function frac(numer: IntegerLike, denom: IntegerLike = 1n): Fraction {
  return new Fraction(numer, denom);
}

export class GrobnerError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "GrobnerError";
  }
}

export class ConversionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ConversionError";
  }
}

export class MPoly {
  readonly nvars: number;
  private readonly coeffs: ReadonlyMap<string, Fraction>;

  constructor(terms: Iterable<readonly [Monomial, FractionLike]>, nvars: number) {
    if (!Number.isInteger(nvars) || nvars < 0) throw new RangeError("MPoly nvars must be a non-negative integer");
    this.nvars = nvars;
    const next = new Map<string, Fraction>();
    for (const [monomial, coeffLike] of terms) {
      assertMonomial(monomial, nvars);
      const coeff = Fraction.from(coeffLike);
      if (coeff.isZero()) continue;
      const key = monomialKey(monomial);
      const merged = (next.get(key) ?? Fraction.zero()).add(coeff);
      if (merged.isZero()) next.delete(key);
      else next.set(key, merged);
    }
    this.coeffs = next;
  }

  static zero(nvars: number): MPoly {
    return new MPoly([], nvars);
  }

  static constant(value: FractionLike, nvars: number): MPoly {
    const coeff = Fraction.from(value);
    return coeff.isZero() ? MPoly.zero(nvars) : new MPoly([[(new Array(nvars).fill(0)), coeff]], nvars);
  }

  static monomial(exp: Monomial, value: FractionLike, nvars: number): MPoly {
    return new MPoly([[exp, value]], nvars);
  }

  isZero(): boolean {
    return this.coeffs.size === 0;
  }

  entries(): Array<readonly [Monomial, Fraction]> {
    return [...this.coeffs.entries()].map(([key, coeff]) => [parseMonomial(key), coeff] as const);
  }

  coefficient(monomial: Monomial): Fraction {
    assertMonomial(monomial, this.nvars);
    return this.coeffs.get(monomialKey(monomial)) ?? Fraction.zero();
  }

  lm(order: MonomialOrder = "grlex"): Monomial {
    if (this.isZero()) throw new Error("Leading monomial of the zero polynomial is undefined");
    let best: Monomial | undefined;
    for (const key of this.coeffs.keys()) {
      const monomial = parseMonomial(key);
      if (best === undefined || cmpMonomials(monomial, best, order) > 0) best = monomial;
    }
    return best ?? [];
  }

  lc(order: MonomialOrder = "grlex"): Fraction {
    return this.coefficient(this.lm(order));
  }

  lt(order: MonomialOrder = "grlex"): MPoly {
    const monomial = this.lm(order);
    return MPoly.monomial(monomial, this.coefficient(monomial), this.nvars);
  }

  totalDegree(): number {
    if (this.isZero()) return 0;
    return Math.max(...this.entries().map(([m]) => totalDegree(m)));
  }

  add(other: MPoly): MPoly {
    assertSameRing(this, other, "addition");
    return new MPoly([...this.entries(), ...other.entries()], this.nvars);
  }

  neg(): MPoly {
    return new MPoly(this.entries().map(([m, c]) => [m, c.neg()] as const), this.nvars);
  }

  sub(other: MPoly): MPoly {
    return this.add(other.neg());
  }

  mul(other: MPoly): MPoly {
    assertSameRing(this, other, "multiplication");
    const terms: Array<readonly [Monomial, Fraction]> = [];
    for (const [ma, ca] of this.entries()) {
      for (const [mb, cb] of other.entries()) {
        terms.push([ma.map((a, i) => a + mb[i]), ca.mul(cb)]);
      }
    }
    return new MPoly(terms, this.nvars);
  }

  scale(value: FractionLike): MPoly {
    const coeff = Fraction.from(value);
    if (coeff.isZero()) return MPoly.zero(this.nvars);
    return new MPoly(this.entries().map(([m, c]) => [m, c.mul(coeff)] as const), this.nvars);
  }

  mulMonomial(exp: Monomial, value: FractionLike = 1n): MPoly {
    assertMonomial(exp, this.nvars);
    const coeff = Fraction.from(value);
    if (coeff.isZero()) return MPoly.zero(this.nvars);
    return new MPoly(this.entries().map(([m, c]) => [m.map((e, i) => e + exp[i]), c.mul(coeff)] as const), this.nvars);
  }

  equals(other: MPoly): boolean {
    if (this.nvars !== other.nvars || this.coeffs.size !== other.coeffs.size) return false;
    for (const [key, coeff] of this.coeffs) {
      if (!coeff.equals(other.coeffs.get(key) ?? Fraction.zero())) return false;
    }
    return true;
  }

  monomialsDescending(order: MonomialOrder = "grlex"): Monomial[] {
    return this.entries().map(([m]) => m).sort((a, b) => cmpMonomials(b, a, order));
  }

  isUnivariate(): number | null {
    const active = new Set<number>();
    for (const [m] of this.entries()) {
      m.forEach((exp, index) => {
        if (exp !== 0) active.add(index);
      });
    }
    if (active.size === 0) return 0;
    if (active.size === 1) return [...active][0];
    return null;
  }

  toUnivariateCoeffs(varIdx: number): Fraction[] {
    if (!Number.isInteger(varIdx) || varIdx < 0 || varIdx >= this.nvars) {
      throw new RangeError("variable index out of range");
    }
    const maxDegree = Math.max(0, ...this.entries().map(([m]) => m[varIdx]));
    const out = Array.from({ length: maxDegree + 1 }, () => Fraction.zero());
    for (const [m, coeff] of this.entries()) out[m[varIdx]] = coeff;
    return out;
  }

  leadingMonomialDivides(monomial: Monomial, order: MonomialOrder = "grlex"): boolean {
    return divides(this.lm(order), monomial);
  }

  diff(varIdx: number): MPoly {
    const terms: Array<readonly [Monomial, Fraction]> = [];
    for (const [m, coeff] of this.entries()) {
      const exp = m[varIdx];
      if (exp === 0) continue;
      terms.push([m.map((e, i) => e - (i === varIdx ? 1 : 0)), coeff.mul(exp)]);
    }
    return new MPoly(terms, this.nvars);
  }

  evalAt(varIdx: number, value: FractionLike): MPoly {
    const v = Fraction.from(value);
    const terms: Array<readonly [Monomial, Fraction]> = [];
    for (const [m, coeff] of this.entries()) {
      terms.push([m.map((e, i) => (i === varIdx ? 0 : e)), coeff.mul(v.pow(m[varIdx]))]);
    }
    return new MPoly(terms, this.nvars);
  }
}

export function makeVar(varIdx: number, nvars: number): MPoly {
  return MPoly.monomial(Array.from({ length: nvars }, (_, i) => (i === varIdx ? 1 : 0)), 1n, nvars);
}

export function cmpMonomials(a: Monomial, b: Monomial, order: MonomialOrder = "grlex"): number {
  const ka = monomialOrderKey(a, order);
  const kb = monomialOrderKey(b, order);
  for (let i = 0; i < ka.length; i += 1) {
    if (ka[i] > kb[i]) return 1;
    if (ka[i] < kb[i]) return -1;
  }
  return 0;
}

export function lcmMonomial(a: Monomial, b: Monomial): Monomial {
  assertSameMonomialLength(a, b);
  return a.map((ai, i) => Math.max(ai, b[i]));
}

export function divides(a: Monomial, b: Monomial): boolean {
  assertSameMonomialLength(a, b);
  return a.every((ai, i) => ai <= b[i]);
}

export function divMonomial(b: Monomial, a: Monomial): Monomial {
  assertSameMonomialLength(a, b);
  return b.map((bi, i) => bi - a[i]);
}

export function totalDegree(monomial: Monomial): number {
  return monomial.reduce((sum, exp) => sum + exp, 0);
}

export function divReductionStep(f: MPoly, g: MPoly, order: MonomialOrder = "grlex"): readonly [MPoly, MPoly] | null {
  if (f.isZero()) return null;
  const lmF = f.lm(order);
  const lmG = g.lm(order);
  if (!divides(lmG, lmF)) return null;
  const expDiff = divMonomial(lmF, lmG);
  const coeff = f.lc(order).div(g.lc(order));
  const term = MPoly.monomial(expDiff, coeff, f.nvars);
  return [term, f.sub(g.mulMonomial(expDiff, coeff))] as const;
}

export function reducePoly(f: MPoly, basis: readonly MPoly[], order: MonomialOrder = "grlex"): MPoly {
  let p = f;
  let remainder = MPoly.zero(f.nvars);

  while (!p.isZero()) {
    const lmP = p.lm(order);
    let reduced = false;
    for (const g of basis) {
      if (g.isZero()) continue;
      const lmG = g.lm(order);
      if (divides(lmG, lmP)) {
        const expDiff = divMonomial(lmP, lmG);
        const coeff = p.lc(order).div(g.lc(order));
        p = p.sub(g.mulMonomial(expDiff, coeff));
        reduced = true;
        break;
      }
    }
    if (!reduced) {
      const lt = p.lt(order);
      remainder = remainder.add(lt);
      p = p.sub(lt);
    }
  }

  return remainder;
}

export function sPoly(f: MPoly, g: MPoly, order: MonomialOrder = "grlex"): MPoly {
  if (f.isZero() || g.isZero()) throw new Error("S-polynomial undefined for zero polynomials");
  const lmF = f.lm(order);
  const lmG = g.lm(order);
  const lcm = lcmMonomial(lmF, lmG);
  return f.mulMonomial(divMonomial(lcm, lmF), Fraction.one().div(f.lc(order)))
    .sub(g.mulMonomial(divMonomial(lcm, lmG), Fraction.one().div(g.lc(order))));
}

export function buchberger(input: readonly MPoly[], order: MonomialOrder = "grlex"): MPoly[] {
  const basis = input.filter((p) => !p.isZero());
  if (basis.length === 0) return [];
  for (const p of basis) {
    if (p.totalDegree() > MAX_DEGREE) {
      throw new GrobnerError(`Input polynomial has total degree ${p.totalDegree()} which exceeds ${MAX_DEGREE}`);
    }
  }

  const pairs = new Set<string>();
  for (let i = 0; i < basis.length; i += 1) {
    for (let j = i + 1; j < basis.length; j += 1) pairs.add(pairKey(i, j));
  }

  while (pairs.size > 0) {
    const key = pairs.values().next().value as string;
    pairs.delete(key);
    const [i, j] = key.split(",").map(Number);
    const remainder = reducePoly(sPoly(basis[i], basis[j], order), basis, order);
    if (remainder.isZero()) continue;
    if (basis.length >= MAX_BASIS_SIZE) {
      throw new GrobnerError(`Groebner basis grew beyond ${MAX_BASIS_SIZE} elements`);
    }
    if (remainder.totalDegree() > MAX_DEGREE) {
      throw new GrobnerError(`New basis element has total degree ${remainder.totalDegree()} which exceeds ${MAX_DEGREE}`);
    }
    const newIndex = basis.length;
    for (let k = 0; k < newIndex; k += 1) pairs.add(pairKey(k, newIndex));
    basis.push(remainder);
  }

  return interReduce(basis, order);
}

export function rationalRoots(coeffsInput: readonly FractionLike[]): Fraction[] {
  const coeffs = coeffsInput.map(Fraction.from);
  let lcmDenom = 1n;
  for (const coeff of coeffs) lcmDenom = (lcmDenom * coeff.denom) / gcd(lcmDenom, coeff.denom);
  const intCoeffs = coeffs.map((coeff) => coeff.numer * (lcmDenom / coeff.denom));
  while (intCoeffs.length > 1 && intCoeffs[intCoeffs.length - 1] === 0n) intCoeffs.pop();
  if (intCoeffs.length <= 1) return [];

  const constant = intCoeffs[0];
  const leading = intCoeffs[intCoeffs.length - 1];
  if (constant === 0n) {
    const rest = rationalRoots(intCoeffs.slice(1).map((c) => new Fraction(c)));
    return uniqueFractions([Fraction.zero(), ...rest]);
  }

  const roots: Fraction[] = [];
  const seen = new Set<string>();
  for (const p of divisorsBigInt(constant)) {
    for (const q of divisorsBigInt(leading)) {
      for (const sign of [1n, -1n]) {
        const candidate = new Fraction(sign * p, q);
        const key = candidate.toString();
        if (seen.has(key)) continue;
        seen.add(key);
        let value = Fraction.zero();
        for (let k = 0; k < intCoeffs.length; k += 1) {
          value = value.add(new Fraction(intCoeffs[k]).mul(candidate.pow(k)));
        }
        if (value.isZero()) roots.push(candidate);
      }
    }
  }
  return roots;
}

export function solveUnivariate(coeffsInput: readonly FractionLike[]): Fraction[] | null {
  let coeffs = coeffsInput.map(Fraction.from);
  while (coeffs.length > 1 && coeffs[coeffs.length - 1].isZero()) coeffs = coeffs.slice(0, -1);
  const degree = coeffs.length - 1;
  if (degree <= 0) return [];
  if (degree === 1) {
    const [b, a] = coeffs;
    return a.isZero() ? [] : [b.neg().div(a)];
  }
  if (degree === 2) {
    const [c, b, a] = coeffs;
    const disc = b.mul(b).sub(new Fraction(4n).mul(a).mul(c));
    if (disc.compare(0n) < 0) return [];
    if (disc.isZero()) return [b.neg().div(new Fraction(2n).mul(a))];
    const sqrtNumer = perfectSquareRoot(disc.numer);
    const sqrtDenom = perfectSquareRoot(disc.denom);
    if (sqrtNumer === null || sqrtDenom === null) return [];
    const sqrtDisc = new Fraction(sqrtNumer, sqrtDenom);
    const denom = new Fraction(2n).mul(a);
    const r1 = b.neg().add(sqrtDisc).div(denom);
    const r2 = b.neg().sub(sqrtDisc).div(denom);
    return r1.equals(r2) ? [r1] : [r1, r2];
  }
  if (degree > 4) return null;

  const rational = rationalRoots(coeffs);
  if (rational.length === 0) return [];
  let allRoots = [...rational];
  let remaining = coeffs;
  for (const root of rational) {
    remaining = divideByLinearRoot(remaining, root);
    if (remaining.length === 0) break;
  }
  const more = solveUnivariate(remaining);
  if (more !== null) allRoots = allRoots.concat(more);
  return uniqueFractions(allRoots);
}

export function idealSolve(polys: readonly MPoly[], order: MonomialOrder = "lex"): Fraction[][] | null {
  if (polys.length === 0) return null;
  const nvars = polys[0].nvars;
  let basis: MPoly[];
  try {
    basis = buchberger(polys, order);
  } catch (error) {
    if (error instanceof GrobnerError) return null;
    throw error;
  }
  if (basis.length === 0) return null;

  const lastVar = nvars - 1;
  const univariate = basis.find((g) => g.isUnivariate() === lastVar);
  if (univariate === undefined) return null;
  const roots = solveUnivariate(univariate.toUnivariateCoeffs(lastVar));
  if (roots === null || roots.length === 0) return null;

  const solutions: Fraction[][] = [];
  for (const root of roots) {
    const reducedBasis = basis.map((g) => g.evalAt(lastVar, root)).filter((p) => !p.isZero());
    if (nvars === 1) {
      solutions.push([root]);
    } else if (nvars === 2) {
      const linear = findLinearInVar(reducedBasis, 0);
      if (linear !== null) {
        const value = evalLinearRoot(linear, 0);
        if (value !== null) solutions.push([value, root]);
      } else {
        const subSolutions = solveFromBasis(reducedBasis, nvars - 1);
        if (subSolutions !== null) {
          for (const sub of subSolutions) solutions.push([...sub, root]);
        }
      }
    } else {
      const projected = projectOutLast(reducedBasis, nvars);
      if (projected !== null) {
        const subSolutions = idealSolve(projected, order);
        if (subSolutions !== null) {
          for (const sub of subSolutions) solutions.push([...sub, root]);
        }
      }
    }
  }

  return solutions.length === 0 ? null : solutions;
}

export function irToMPoly(node: IRNode, varList: readonly string[]): MPoly {
  const nvars = varList.length;
  if (node.kind === "integer") return MPoly.constant(node.value, nvars);
  if (node.kind === "rational") return MPoly.constant(new Fraction(node.numer, node.denom), nvars);
  if (node.kind === "symbol") {
    const index = varList.indexOf(node.name);
    if (index < 0) throw new ConversionError(`Unrecognized symbol in polynomial context: ${node.name}`);
    return makeVar(index, nvars);
  }
  if (node.kind !== "apply") throw new ConversionError(`Cannot convert ${node.kind} to polynomial`);

  const name = headName(node.head);
  if (name === ADD.name) {
    return node.args.reduce((acc, arg) => acc.add(irToMPoly(arg, varList)), MPoly.zero(nvars));
  }
  if (name === SUB.name) {
    if (node.args.length !== 2) throw new ConversionError("Sub expects 2 arguments");
    return irToMPoly(node.args[0], varList).sub(irToMPoly(node.args[1], varList));
  }
  if (name === MUL.name) {
    return node.args.reduce((acc, arg) => acc.mul(irToMPoly(arg, varList)), MPoly.constant(1n, nvars));
  }
  if (name === NEG.name) {
    if (node.args.length !== 1) throw new ConversionError("Neg expects 1 argument");
    return irToMPoly(node.args[0], varList).neg();
  }
  if (name === POW.name) {
    if (node.args.length !== 2) throw new ConversionError("Pow expects 2 arguments");
    const [base, exponent] = node.args;
    if (exponent.kind !== "integer") throw new ConversionError("Pow exponent must be an integer");
    if (exponent.value < 0n || exponent.value > 20n) throw new ConversionError("Pow exponent out of polynomial conversion range");
    let result = MPoly.constant(1n, nvars);
    const basePoly = irToMPoly(base, varList);
    for (let i = 0n; i < exponent.value; i += 1n) result = result.mul(basePoly);
    return result;
  }

  throw new ConversionError(`Cannot convert head ${name || "?"} to polynomial`);
}

export function mpolyToIR(poly: MPoly, varSymbols: readonly IRSymbol[]): IRNode {
  if (poly.isZero()) return int(0);
  const terms: IRNode[] = [];
  for (const monomial of poly.monomialsDescending("grlex")) {
    const coeff = poly.coefficient(monomial);
    const parts: IRNode[] = [];
    monomial.forEach((exp, i) => {
      if (exp === 1) parts.push(varSymbols[i]);
      else if (exp > 1) parts.push(app(POW, [varSymbols[i], int(exp)]));
    });
    if (parts.length === 0) {
      terms.push(coeff.toIR());
    } else if (coeff.equals(1n)) {
      terms.push(parts.length === 1 ? parts[0] : app(MUL, parts));
    } else if (coeff.equals(-1n)) {
      const term = parts.length === 1 ? parts[0] : app(MUL, parts);
      terms.push(app(NEG, [term]));
    } else {
      terms.push(app(MUL, [coeff.toIR(), ...parts]));
    }
  }
  return terms.length === 1 ? terms[0] : app(ADD, terms);
}

export function extractVarList(node: IRNode): string[] | null {
  if (node.kind !== "apply" || headName(node.head) !== LIST.name) return null;
  const names: string[] = [];
  for (const arg of node.args) {
    if (arg.kind !== "symbol") return null;
    names.push(arg.name);
  }
  return names;
}

export function extractPolyList(node: IRNode, varList: readonly string[]): MPoly[] | null {
  if (node.kind !== "apply" || headName(node.head) !== LIST.name) return null;
  const polys: MPoly[] = [];
  for (const arg of node.args) {
    try {
      polys.push(irToMPoly(arg, varList));
    } catch (error) {
      if (error instanceof ConversionError) return null;
      throw error;
    }
  }
  return polys;
}

export function groebnerHandler(expr: IRApply): IRNode {
  if (expr.args.length !== 2) return expr;
  const [polyListNode, varListNode] = expr.args;
  const varList = extractVarList(varListNode);
  if (varList === null || varList.length === 0) return expr;
  const polys = extractPolyList(polyListNode, varList);
  if (polys === null) return expr;
  try {
    return app(LIST, buchberger(polys, "grlex").map((g) => mpolyToIR(g, varList.map(sym))));
  } catch (error) {
    if (error instanceof GrobnerError) return expr;
    throw error;
  }
}

export function polyReduceHandler(expr: IRApply): IRNode {
  if (expr.args.length !== 3) return expr;
  const [fNode, polyListNode, varListNode] = expr.args;
  const varList = extractVarList(varListNode);
  if (varList === null || varList.length === 0) return expr;
  let fPoly: MPoly;
  try {
    fPoly = irToMPoly(fNode, varList);
  } catch (error) {
    if (error instanceof ConversionError) return expr;
    throw error;
  }
  const polys = extractPolyList(polyListNode, varList);
  if (polys === null) return expr;
  return mpolyToIR(reducePoly(fPoly, polys, "grlex"), varList.map(sym));
}

export function idealSolveHandler(expr: IRApply): IRNode {
  if (expr.args.length !== 2) return expr;
  const [polyListNode, varListNode] = expr.args;
  const varList = extractVarList(varListNode);
  if (varList === null || varList.length === 0) return expr;
  const polys = extractPolyList(polyListNode, varList);
  if (polys === null) return expr;
  const solutions = idealSolve(polys, "lex");
  if (solutions === null) return expr;
  const vars = varList.map(sym);
  const solutionNodes = solutions
    .filter((solution) => solution.length === vars.length)
    .map((solution) => app(LIST, solution.map((value, i) => app(RULE, [vars[i], value.toIR()]))));
  return solutionNodes.length === 0 ? expr : app(LIST, solutionNodes);
}

export function buildMultivariateHandlerTable(): ReadonlyMap<string, (expr: IRApply) => IRNode> {
  return new Map([
    [GROEBNER.name, groebnerHandler],
    [POLY_REDUCE.name, polyReduceHandler],
    [IDEAL_SOLVE.name, idealSolveHandler],
  ]);
}

function makeMonic(poly: MPoly, order: MonomialOrder): MPoly {
  return poly.isZero() ? poly : poly.scale(Fraction.one().div(poly.lc(order)));
}

function interReduce(basis: readonly MPoly[], order: MonomialOrder): MPoly[] {
  const minimal: MPoly[] = [];
  for (let i = 0; i < basis.length; i += 1) {
    const g = basis[i];
    if (g.isZero()) continue;
    const lmG = g.lm(order);
    let dominated = false;
    for (let j = 0; j < basis.length; j += 1) {
      if (i === j || basis[j].isZero()) continue;
      const lmH = basis[j].lm(order);
      if (divides(lmH, lmG) && (!monomialEquals(lmH, lmG) || j < i)) {
        dominated = true;
        break;
      }
    }
    if (!dominated) minimal.push(makeMonic(g, order));
  }

  const reduced: MPoly[] = [];
  for (let i = 0; i < minimal.length; i += 1) {
    const others = minimal.filter((_, j) => i !== j);
    const r = reducePoly(minimal[i], others, order);
    if (!r.isZero()) reduced.push(makeMonic(r, order));
  }
  return reduced;
}

function findLinearInVar(basis: readonly MPoly[], varIdx: number): MPoly | null {
  for (const poly of basis) {
    if (poly.isUnivariate() !== varIdx) continue;
    const coeffs = poly.toUnivariateCoeffs(varIdx);
    if (coeffs.length === 2 && !coeffs[1].isZero()) return poly;
  }
  return null;
}

function evalLinearRoot(poly: MPoly, varIdx: number): Fraction | null {
  const coeffs = poly.toUnivariateCoeffs(varIdx);
  if (coeffs.length !== 2 || coeffs[1].isZero()) return null;
  return coeffs[0].neg().div(coeffs[1]);
}

function solveFromBasis(basis: readonly MPoly[], nvars: number): Fraction[][] | null {
  if (nvars !== 1) return null;
  for (const poly of basis) {
    if (poly.isUnivariate() === 0) {
      const roots = solveUnivariate(poly.toUnivariateCoeffs(0));
      if (roots !== null && roots.length > 0) return roots.map((root) => [root]);
    }
  }
  return null;
}

function projectOutLast(basis: readonly MPoly[], nvars: number): MPoly[] | null {
  const out: MPoly[] = [];
  for (const poly of basis) {
    if (poly.entries().some(([m]) => m[nvars - 1] !== 0)) return null;
    out.push(new MPoly(poly.entries().map(([m, c]) => [m.slice(0, nvars - 1), c] as const), nvars - 1));
  }
  return out.length === 0 ? null : out;
}

function divideByLinearRoot(coeffs: readonly Fraction[], root: Fraction): Fraction[] {
  if (coeffs.length <= 1) return [];
  const quotient = Array.from({ length: coeffs.length - 1 }, () => Fraction.zero());
  quotient[quotient.length - 1] = coeffs[coeffs.length - 1];
  for (let k = quotient.length - 2; k >= 0; k -= 1) {
    quotient[k] = coeffs[k + 1].add(root.mul(quotient[k + 1]));
  }
  while (quotient.length > 1 && quotient[quotient.length - 1].isZero()) quotient.pop();
  return quotient;
}

function uniqueFractions(values: readonly Fraction[]): Fraction[] {
  const seen = new Set<string>();
  const out: Fraction[] = [];
  for (const value of values) {
    const key = value.toString();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(value);
  }
  return out;
}

function monomialOrderKey(monomial: Monomial, order: MonomialOrder): number[] {
  if (order === "lex") return [...monomial];
  if (order === "grlex") return [totalDegree(monomial), ...monomial];
  if (order === "grevlex") return [totalDegree(monomial), ...[...monomial].reverse().map((exp) => -exp)];
  throw new Error(`Unknown monomial order: ${order}`);
}

function monomialKey(monomial: Monomial): string {
  return monomial.join(",");
}

function parseMonomial(key: string): Monomial {
  return key === "" ? [] : key.split(",").map(Number);
}

function monomialEquals(a: Monomial, b: Monomial): boolean {
  return a.length === b.length && a.every((value, i) => value === b[i]);
}

function pairKey(i: number, j: number): string {
  return `${i},${j}`;
}

function assertSameRing(a: MPoly, b: MPoly, op: string): void {
  if (a.nvars !== b.nvars) throw new Error(`Variable count mismatch in MPoly ${op}`);
}

function assertMonomial(monomial: Monomial, nvars: number): void {
  if (monomial.length !== nvars) throw new Error(`Monomial has ${monomial.length} exponents, expected ${nvars}`);
  for (const exp of monomial) {
    if (!Number.isInteger(exp) || exp < 0) throw new RangeError("Monomial exponents must be non-negative integers");
  }
}

function assertSameMonomialLength(a: Monomial, b: Monomial): void {
  if (a.length !== b.length) throw new Error("Monomial length mismatch");
}

function divisorsBigInt(value: bigint): bigint[] {
  const n = abs(value);
  if (n === 0n) return [];
  const out: bigint[] = [];
  const limit = bigintSqrt(n);
  for (let d = 1n; d <= limit; d += 1n) {
    if (n % d === 0n) {
      out.push(d);
      if (d !== n / d) out.push(n / d);
    }
  }
  return out.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

function perfectSquareRoot(value: bigint): bigint | null {
  if (value < 0n) return null;
  const root = bigintSqrt(value);
  return root * root === value ? root : null;
}

function bigintSqrt(value: bigint): bigint {
  if (value < 2n) return value;
  let x0 = value;
  let x1 = (x0 + value / x0) / 2n;
  while (x1 < x0) {
    x0 = x1;
    x1 = (x0 + value / x0) / 2n;
  }
  return x0;
}

function toBigInt(value: IntegerLike): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) throw new RangeError("integer number inputs must be safe integers");
  return BigInt(value);
}

function gcd(aInput: bigint, bInput: bigint): bigint {
  let a = aInput;
  let b = bInput;
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
