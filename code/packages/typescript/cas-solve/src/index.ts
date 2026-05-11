import { ADD, DIV, MUL, SQRT, SUB, app, int, rational, sym, type IRNode } from "@coding-adventures/symbolic-ir";

export const SOLVE = "Solve";
export const NSOLVE = "NSolve";
export const ROOTS = "Roots";
export const I_UNIT = "%i";

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

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
