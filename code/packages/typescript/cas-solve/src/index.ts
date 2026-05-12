import { ADD, DIV, MUL, NEG, SQRT, SUB, app, equals, int, rational, sym, type IRNode } from "@coding-adventures/symbolic-ir";

export const SOLVE = "Solve";
export const NSOLVE = "NSolve";
export const ROOTS = "Roots";
export const I_UNIT = "%i";
export const CBRT = "Cbrt";

export type IntegerLike = bigint | number | string;

export type SolveResult =
  | { readonly kind: "solutions"; readonly roots: readonly IRNode[] }
  | { readonly kind: "all" };

export const ALL_SOLUTIONS: SolveResult = Object.freeze({ kind: "all" });

export class Frac {
  readonly numer: bigint;
  readonly denom: bigint;

  constructor(numerInput: IntegerLike, denomInput: IntegerLike = 1n) {
    let numer = toBigInt(numerInput);
    let denom = toBigInt(denomInput);
    if (denom === 0n) throw new RangeError("Frac: denominator must not be zero");
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

  static fromInt(value: IntegerLike): Frac {
    return new Frac(value, 1n);
  }

  static zero(): Frac {
    return new Frac(0n);
  }

  static one(): Frac {
    return new Frac(1n);
  }

  isZero(): boolean {
    return this.numer === 0n;
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
    if (rhs.numer === 0n) throw new RangeError("Frac: division by zero");
    return new Frac(this.numer * rhs.denom, this.denom * rhs.numer);
  }

  compare(rhs: Frac): number {
    const lhsWide = this.numer * rhs.denom;
    const rhsWide = rhs.numer * this.denom;
    return lhsWide < rhsWide ? -1 : lhsWide > rhsWide ? 1 : 0;
  }

  equals(rhs: Frac): boolean {
    return this.numer === rhs.numer && this.denom === rhs.denom;
  }

  toIrNode(): IRNode {
    return this.denom === 1n ? int(this.numer) : rational(this.numer, this.denom);
  }
}

export function solutions(roots: readonly IRNode[]): SolveResult {
  return Object.freeze({ kind: "solutions", roots: Object.freeze([...roots]) });
}

export function solveLinear(a: Frac, b: Frac): SolveResult {
  if (a.isZero()) return b.isZero() ? ALL_SOLUTIONS : solutions([]);
  return solutions([b.neg().div(a).toIrNode()]);
}

export function solveQuadratic(a: Frac, b: Frac, c: Frac): SolveResult {
  if (a.isZero()) return solveLinear(b, c);

  const discriminant = b.mul(b).sub(Frac.fromInt(4).mul(a).mul(c));
  const twoA = Frac.fromInt(2).mul(a);
  const negB = b.neg();
  const zero = Frac.zero();
  const cmp = discriminant.compare(zero);

  if (cmp > 0) {
    const sqrtDisc = sqrtOrRational(discriminant);
    if (sqrtDisc.kind === "rational") {
      const roots = [
        negB.add(sqrtDisc.value).div(twoA).toIrNode(),
        negB.sub(sqrtDisc.value).div(twoA).toIrNode(),
      ].sort(irNodeCompare);
      return solutions(roots);
    }
    return solutions([
      buildIrrationalRoot(negB, twoA, sqrtDisc.value, 1),
      buildIrrationalRoot(negB, twoA, sqrtDisc.value, -1),
    ]);
  }

  if (cmp === 0) {
    return solutions([negB.div(twoA).toIrNode()]);
  }

  const sqrtAbs = sqrtOrRational(discriminant.neg());
  return solutions([
    buildComplexRoot(negB, twoA, sqrtAbs, 1),
    buildComplexRoot(negB, twoA, sqrtAbs, -1),
  ]);
}

export function solveCubic(a: Frac, b: Frac, c: Frac, d: Frac): SolveResult {
  if (a.isZero()) return solveQuadratic(b, c, d);

  const rationalRoot = findRationalRoot(a, b, c, d);
  if (rationalRoot !== null) {
    const b2 = b.add(a.mul(rationalRoot));
    const c2 = c.add(b2.mul(rationalRoot));
    const remainder = d.add(c2.mul(rationalRoot));
    if (remainder.isZero()) {
      const remaining = solveQuadratic(a, b2, c2);
      if (remaining.kind === "all") return solutions([rationalRoot.toIrNode()]);
      return solutions(dedupRoots([rationalRoot.toIrNode(), ...remaining.roots]));
    }
  }

  const one = Frac.one();
  const two = Frac.fromInt(2);
  const three = Frac.fromInt(3);
  const four = Frac.fromInt(4);
  const twentySeven = Frac.fromInt(27);
  const aInv = one.div(a);
  const aInv2 = aInv.mul(aInv);
  const aInv3 = aInv2.mul(aInv);

  const p = c.mul(aInv).sub(b.mul(b).mul(aInv2).div(three));
  const q = d.mul(aInv)
    .sub(b.mul(c).mul(aInv2).div(three))
    .add(two.mul(b).mul(b).mul(b).mul(aInv3).div(twentySeven));
  const shift = b.neg().div(three.mul(a));
  const dCard = q.mul(q).div(four).add(p.mul(p).mul(p).div(twentySeven));

  const cmp = dCard.compare(Frac.zero());
  if (cmp > 0) return solutions(cardanoOneRealTwoComplex(q, shift, dCard));
  if (cmp === 0) return solutions(cardanoRepeated(p, q, shift));
  return solutions([]);
}

export function solveQuartic(a: Frac, b: Frac, c: Frac, d: Frac, e: Frac): SolveResult {
  if (a.isZero()) return solveCubic(b, c, d, e);

  const rationalRoot = findRationalRootQuartic(a, b, c, d, e);
  if (rationalRoot !== null) {
    const b2 = b.add(a.mul(rationalRoot));
    const c2 = c.add(b2.mul(rationalRoot));
    const d2 = d.add(c2.mul(rationalRoot));
    const remainder = e.add(d2.mul(rationalRoot));
    if (remainder.isZero()) {
      const remaining = solveCubic(a, b2, c2, d2);
      if (remaining.kind === "all") return solutions([rationalRoot.toIrNode()]);
      return solutions(dedupRoots([rationalRoot.toIrNode(), ...remaining.roots]));
    }
  }

  const two = Frac.fromInt(2);
  const four = Frac.fromInt(4);
  const eight = Frac.fromInt(8);
  const sixteen = Frac.fromInt(16);
  const twoFiftySix = Frac.fromInt(256);
  const a2 = a.mul(a);
  const a3 = a2.mul(a);
  const a4 = a3.mul(a);
  const b2 = b.mul(b);
  const b3 = b2.mul(b);
  const b4 = b2.mul(b2);

  const p = c.div(a).sub(Frac.fromInt(3).mul(b2).div(eight.mul(a2)));
  const q = b3.div(eight.mul(a3)).sub(b.mul(c).div(two.mul(a2))).add(d.div(a));
  const rCoef = Frac.fromInt(-3).mul(b4).div(twoFiftySix.mul(a4))
    .add(b2.mul(c).div(sixteen.mul(a3)))
    .sub(b.mul(d).div(four.mul(a2)))
    .add(e.div(a));
  const shift = b.neg().div(four.mul(a));

  if (q.isZero()) {
    const uRoots = solveQuadratic(Frac.one(), p, rCoef);
    if (uRoots.kind === "all") return solutions([]);
    return solutions(dedupRoots(uRoots.roots.flatMap((root) => {
      const t = app(SQRT, [root]);
      return [addShift(t, shift), addShift(app(NEG, [t]), shift)];
    })));
  }

  const ra = Frac.fromInt(8);
  const rb = Frac.fromInt(8).mul(p);
  const rc = two.mul(p).mul(p).sub(Frac.fromInt(8).mul(rCoef));
  const rd = q.mul(q).neg();
  const resolventRoots = solveCubic(ra, rb, rc, rd);
  if (resolventRoots.kind === "all" || resolventRoots.roots.length === 0) return solutions([]);

  let m: Frac | null = null;
  for (const root of resolventRoots.roots) {
    if (root.kind === "integer") {
      m = new Frac(root.value);
      break;
    }
    if (root.kind === "rational") {
      m = new Frac(root.numer, root.denom);
      break;
    }
  }
  if (m === null || m.isZero()) return solutions([]);

  const alpha = p.div(two).add(m.mul(m).div(two)).sub(q.div(two.mul(m)));
  const beta = p.div(two).add(m.mul(m).div(two)).add(q.div(two.mul(m)));
  const roots1 = solveQuadratic(Frac.one(), m, alpha);
  const roots2 = solveQuadratic(Frac.one(), m.neg(), beta);
  const shifted: IRNode[] = [];
  if (roots1.kind === "solutions") shifted.push(...roots1.roots.map((root) => addShift(root, shift)));
  if (roots2.kind === "solutions") shifted.push(...roots2.roots.map((root) => addShift(root, shift)));
  return solutions(dedupRoots(shifted));
}

type SqrtResult =
  | { readonly kind: "rational"; readonly value: Frac }
  | { readonly kind: "irrational"; readonly value: IRNode };

function sqrtOrRational(value: Frac): SqrtResult {
  if (value.numer >= 0n) {
    const rn = perfectSquareRoot(value.numer);
    const rd = perfectSquareRoot(value.denom);
    if (rn !== null && rd !== null) return { kind: "rational", value: new Frac(rn, rd) };
  }
  return { kind: "irrational", value: app(SQRT, [value.toIrNode()]) };
}

function buildIrrationalRoot(negB: Frac, twoA: Frac, sqrtNode: IRNode, sign: 1 | -1): IRNode {
  const head = sign > 0 ? ADD : SUB;
  return app(DIV, [app(head, [negB.toIrNode(), sqrtNode]), twoA.toIrNode()]);
}

function buildComplexRoot(negB: Frac, twoA: Frac, sqrtAbs: SqrtResult, sign: 1 | -1): IRNode {
  const realPart = negB.div(twoA).toIrNode();
  const imagCoef = sqrtAbs.kind === "rational"
    ? sqrtAbs.value.div(twoA).toIrNode()
    : app(DIV, [sqrtAbs.value, twoA.toIrNode()]);
  const imagPart = app(MUL, [imagCoef, sym(I_UNIT)]);
  return app(sign > 0 ? ADD : SUB, [realPart, imagPart]);
}

function cardanoOneRealTwoComplex(q: Frac, shift: Frac, dCard: Frac): readonly IRNode[] {
  const negQHalf = q.neg().div(Frac.fromInt(2));
  const sqrtD = tryExactSqrt(dCard);

  if (sqrtD !== null) {
    const aTerm = negQHalf.add(sqrtD);
    const bTerm = negQHalf.sub(sqrtD);
    const cbrtA = tryExactCbrt(aTerm);
    const cbrtB = tryExactCbrt(bTerm);
    if (cbrtA !== null && cbrtB !== null) {
      const t1 = cbrtA.add(cbrtB);
      const halfSum = cbrtA.add(cbrtB).neg().div(Frac.fromInt(2));
      const halfDiff = cbrtA.sub(cbrtB).div(Frac.fromInt(2));
      const roots: IRNode[] = [t1.add(shift).toIrNode()];
      if (halfDiff.isZero()) {
        const repeated = halfSum.add(shift).toIrNode();
        roots.push(repeated, repeated);
      } else {
        const realPart = halfSum.add(shift).toIrNode();
        const imagPart = imagTerm(halfDiff);
        roots.push(app(ADD, [realPart, imagPart]), app(SUB, [realPart, imagPart]));
      }
      return roots;
    }
  }

  const sqrtDNode = sqrtIr(dCard);
  const negQHalfNode = negQHalf.toIrNode();
  const cbrtHead = sym(CBRT);
  const cbrtA = negQHalf.isZero()
    ? app(cbrtHead, [sqrtDNode])
    : app(cbrtHead, [app(ADD, [negQHalfNode, sqrtDNode])]);
  const cbrtB = negQHalf.isZero()
    ? app(cbrtHead, [app(NEG, [sqrtDNode])])
    : app(cbrtHead, [app(SUB, [negQHalfNode, sqrtDNode])]);

  const t1 = app(ADD, [cbrtA, cbrtB]);
  const x1 = addShift(t1, shift);
  const minusT1Half = app(DIV, [app(NEG, [app(ADD, [cbrtA, cbrtB])]), int(2)]);
  const diff = app(SUB, [cbrtA, cbrtB]);
  const imagPart = app(MUL, [
    app(DIV, [app(MUL, [diff, app(SQRT, [int(3)])]), int(2)]),
    sym(I_UNIT),
  ]);
  const realPart = addShift(minusT1Half, shift);
  return [x1, app(ADD, [realPart, imagPart]), app(SUB, [realPart, imagPart])];
}

function cardanoRepeated(p: Frac, q: Frac, shift: Frac): readonly IRNode[] {
  if (p.isZero() && q.isZero()) return [shift.toIrNode()];

  const negQHalf = q.neg().div(Frac.fromInt(2));
  const cbrtValue = tryExactCbrt(negQHalf);
  if (cbrtValue !== null) {
    const t1 = Frac.fromInt(2).mul(cbrtValue);
    const t2 = cbrtValue.neg();
    return dedupRoots([t1.add(shift).toIrNode(), t2.add(shift).toIrNode()]);
  }

  const cbrtNode = app(sym(CBRT), [negQHalf.toIrNode()]);
  return [
    addShift(app(MUL, [int(2), cbrtNode]), shift),
    addShift(app(NEG, [cbrtNode]), shift),
  ];
}

function findRationalRoot(a: Frac, b: Frac, c: Frac, d: Frac): Frac | null {
  const scale = fractionDenominatorLcm(a, b, c, d);
  const scaledA = scaleFractionToInteger(a, scale);
  const scaledD = scaleFractionToInteger(d, scale);

  if (scaledD === 0n) return Frac.zero();

  const pDivs = divisors(abs(scaledD));
  const qDivs = divisors(abs(scaledA));
  for (const pValue of pDivs) {
    for (const qValue of qDivs) {
      for (const sign of [1n, -1n]) {
        const candidate = new Frac(sign * pValue, qValue);
        if (evalCubic(a, b, c, d, candidate).isZero()) return candidate;
      }
    }
  }
  return null;
}

function findRationalRootQuartic(a: Frac, b: Frac, c: Frac, d: Frac, e: Frac): Frac | null {
  const scale = fractionDenominatorLcm(a, b, c, d, e);
  const scaledA = scaleFractionToInteger(a, scale);
  const scaledE = scaleFractionToInteger(e, scale);

  if (scaledE === 0n) return Frac.zero();

  const pDivs = divisors(abs(scaledE));
  const qDivs = divisors(abs(scaledA));
  for (const pValue of pDivs) {
    for (const qValue of qDivs) {
      for (const sign of [1n, -1n]) {
        const candidate = new Frac(sign * pValue, qValue);
        if (evalQuartic(a, b, c, d, e, candidate).isZero()) return candidate;
      }
    }
  }
  return null;
}

function evalCubic(a: Frac, b: Frac, c: Frac, d: Frac, x: Frac): Frac {
  return a.mul(x).mul(x).mul(x).add(b.mul(x).mul(x)).add(c.mul(x)).add(d);
}

function evalQuartic(a: Frac, b: Frac, c: Frac, d: Frac, e: Frac, x: Frac): Frac {
  const x2 = x.mul(x);
  const x3 = x2.mul(x);
  const x4 = x3.mul(x);
  return a.mul(x4).add(b.mul(x3)).add(c.mul(x2)).add(d.mul(x)).add(e);
}

function fractionDenominatorLcm(...values: readonly Frac[]): bigint {
  let result = 1n;
  for (const value of values) {
    result = lcm(result, value.denom);
  }
  return result;
}

function scaleFractionToInteger(value: Frac, scale: bigint): bigint {
  return value.numer * (scale / value.denom);
}

function divisors(value: bigint): readonly bigint[] {
  if (value === 0n) return [0n];
  const result: bigint[] = [];
  for (let i = 1n; i * i <= value; i += 1n) {
    if (value % i === 0n) {
      result.push(i);
      const pair = value / i;
      if (pair !== i) result.push(pair);
    }
  }
  return result.sort((lhs, rhs) => lhs < rhs ? -1 : lhs > rhs ? 1 : 0);
}

function tryExactSqrt(value: Frac): Frac | null {
  if (value.numer < 0n) return null;
  const numerRoot = perfectSquareRoot(value.numer);
  const denomRoot = perfectSquareRoot(value.denom);
  return numerRoot !== null && denomRoot !== null ? new Frac(numerRoot, denomRoot) : null;
}

function tryExactCbrt(value: Frac): Frac | null {
  if (value.isZero()) return Frac.zero();
  const sign = value.numer < 0n ? -1n : 1n;
  const numerRoot = perfectCubeRoot(abs(value.numer));
  const denomRoot = perfectCubeRoot(value.denom);
  return numerRoot !== null && denomRoot !== null ? new Frac(sign * numerRoot, denomRoot) : null;
}

function sqrtIr(value: Frac): IRNode {
  const exact = tryExactSqrt(value);
  return exact !== null ? exact.toIrNode() : app(SQRT, [value.toIrNode()]);
}

function imagTerm(coef: Frac): IRNode {
  if (coef.equals(Frac.one())) return sym(I_UNIT);
  if (coef.equals(Frac.fromInt(-1))) return app(NEG, [sym(I_UNIT)]);
  return app(MUL, [coef.toIrNode(), sym(I_UNIT)]);
}

function addShift(node: IRNode, shift: Frac): IRNode {
  if (shift.isZero()) return node;
  if (shift.numer < 0n) return app(SUB, [node, shift.neg().toIrNode()]);
  return app(ADD, [node, shift.toIrNode()]);
}

function dedupRoots(roots: readonly IRNode[]): readonly IRNode[] {
  const seen: IRNode[] = [];
  for (const root of roots) {
    if (!seen.some((existing) => equals(existing, root))) seen.push(root);
  }
  return seen;
}

function perfectSquareRoot(value: bigint): bigint | null {
  if (value < 0n) return null;
  if (value < 2n) return value;
  let x0 = value;
  let x1 = (x0 + value / x0) / 2n;
  while (x1 < x0) {
    x0 = x1;
    x1 = (x0 + value / x0) / 2n;
  }
  return x0 * x0 === value ? x0 : null;
}

function perfectCubeRoot(value: bigint): bigint | null {
  if (value < 0n) return null;
  if (value < 2n) return value;

  let low = 0n;
  let high = 1n;
  while (high * high * high < value) high *= 2n;

  while (low <= high) {
    const mid = (low + high) / 2n;
    const cube = mid * mid * mid;
    if (cube === value) return mid;
    if (cube < value) {
      low = mid + 1n;
    } else {
      high = mid - 1n;
    }
  }
  return null;
}

function irNodeCompare(a: IRNode, b: IRNode): number {
  const ak = irNodeSortKey(a);
  const bk = irNodeSortKey(b);
  const lhs = ak[0] * bk[1];
  const rhs = bk[0] * ak[1];
  return lhs < rhs ? -1 : lhs > rhs ? 1 : 0;
}

function irNodeSortKey(node: IRNode): readonly [bigint, bigint] {
  if (node.kind === "integer") return [node.value, 1n];
  if (node.kind === "rational") return [node.numer, node.denom];
  return [BigInt(Number.MAX_SAFE_INTEGER), 1n];
}

function toBigInt(value: IntegerLike): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) {
    throw new RangeError("integer number inputs must be safe integers; pass bigint or string for larger values");
  }
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

function lcm(a: bigint, b: bigint): bigint {
  return a / gcd(abs(a), abs(b)) * b;
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
