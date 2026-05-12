import {
  ADD,
  AND,
  DIV,
  EQUAL,
  GREATER,
  GREATER_EQUAL,
  LESS,
  LESS_EQUAL,
  MUL,
  NEG,
  POW,
  RULE,
  SQRT,
  SUB,
  app,
  equals,
  int,
  numberNode,
  rational,
  sym,
  type IRNode,
  type IRSymbol,
} from "@coding-adventures/symbolic-ir";

export const SOLVE = "Solve";
export const NSOLVE = "NSolve";
export const ROOTS = "Roots";
export const I_UNIT = "%i";
export const CBRT = "Cbrt";
const MAX_INEQUALITY_DEGREE = 4;
const REAL_ROOT_TOL = 1e-8;

export type IntegerLike = bigint | number | string;

export interface Complex {
  readonly re: number;
  readonly im: number;
}

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

export function solveLinearSystem(
  equations: readonly IRNode[],
  variables: readonly IRSymbol[],
): readonly IRNode[] | null {
  const dimension = variables.length;
  if (equations.length !== dimension || dimension === 0) return null;

  const variableColumns = new Map<string, number>();
  variables.forEach((variable, index) => variableColumns.set(variable.name, index));

  const matrix: Frac[][] = [];
  for (const equation of equations) {
    const row = equationToRow(equation, variableColumns, dimension);
    if (row === null) return null;
    matrix.push([...row]);
  }

  for (let col = 0; col < dimension; col += 1) {
    let pivotRow = col;
    for (let row = col + 1; row < dimension; row += 1) {
      if (fracAbsCompare(matrix[row][col], matrix[pivotRow][col]) > 0) pivotRow = row;
    }
    if (matrix[pivotRow][col].isZero()) return null;
    [matrix[col], matrix[pivotRow]] = [matrix[pivotRow], matrix[col]];

    const pivot = matrix[col][col];
    for (let row = col + 1; row < dimension; row += 1) {
      if (matrix[row][col].isZero()) continue;
      const factor = matrix[row][col].div(pivot);
      for (let j = col; j <= dimension; j += 1) {
        matrix[row][j] = matrix[row][j].sub(factor.mul(matrix[col][j]));
      }
    }
  }

  const solution = Array.from({ length: dimension }, () => Frac.zero());
  for (let row = dimension - 1; row >= 0; row -= 1) {
    if (matrix[row][row].isZero()) return null;
    let rhs = matrix[row][dimension];
    for (let col = row + 1; col < dimension; col += 1) {
      rhs = rhs.sub(matrix[row][col].mul(solution[col]));
    }
    solution[row] = rhs.div(matrix[row][row]);
  }

  return variables.map((variable, index) => app(RULE, [variable, solution[index].toIrNode()]));
}

export function trySolveInequality(ineq: IRNode, variable: IRSymbol): readonly IRNode[] | null {
  if (ineq.kind !== "apply" || ineq.args.length !== 2 || ineq.head.kind !== "symbol") return null;
  const head = ineq.head.name;
  if (![LESS.name, GREATER.name, LESS_EQUAL.name, GREATER_EQUAL.name].includes(head)) return null;

  const normalized = app(SUB, [ineq.args[0], ineq.args[1]]);
  const coeffs = extractPolynomial(normalized, variable.name, MAX_INEQUALITY_DEGREE);
  if (coeffs === null) return null;

  const wantPositive = head === GREATER.name || head === GREATER_EQUAL.name;
  const strict = head === LESS.name || head === GREATER.name;
  return solvePolynomialSign(coeffs, variable, wantPositive, strict);
}

export function nsolvePoly(
  coeffs: readonly (number | Complex)[],
  maxIter = 200,
  tol = 1e-12,
): readonly Complex[] {
  const degree = coeffs.length - 1;
  if (degree <= 0) return [];

  const lead = toComplex(coeffs[0]);
  if (complexAbs(lead) === 0) throw new RangeError("nsolvePoly: leading coefficient must not be zero");
  const poly = coeffs.map((coef) => complexDiv(toComplex(coef), lead));

  if (degree === 1) {
    return [complexNeg(poly[1])];
  }

  const radius = initialRadius(poly);
  let roots = Array.from({ length: degree }, (_, k) => {
    const theta = (2 * Math.PI * k) / degree + 0.1;
    return complex(radius * Math.cos(theta), radius * Math.sin(theta));
  });

  for (let iter = 0; iter < maxIter; iter += 1) {
    let maxDelta = 0;
    const next = [...roots];
    for (let i = 0; i < degree; i += 1) {
      const z = roots[i];
      let denom = complex(1, 0);
      for (let j = 0; j < degree; j += 1) {
        if (i === j) continue;
        const diff = complexSub(z, roots[j]);
        denom = complexMul(denom, complexAbs(diff) < 1e-300 ? complex(1e-300, 0) : diff);
      }
      const delta = complexDiv(evalPoly(poly, z), denom);
      next[i] = complexSub(z, delta);
      maxDelta = Math.max(maxDelta, complexAbs(delta));
    }
    roots = next;
    if (maxDelta < tol) break;
  }
  return roots;
}

export function rootsToIr(roots: readonly Complex[]): readonly IRNode[] {
  return roots.map((root) => {
    if (Math.abs(root.im) < 1e-10) return numberNode(root.re);
    return app(ADD, [numberNode(root.re), app(MUL, [numberNode(root.im), sym(I_UNIT)])]);
  });
}

export function nsolveFractionPoly(coeffs: readonly Frac[]): readonly IRNode[] {
  return rootsToIr(nsolvePoly(coeffs.map((coef) => Number(coef.numer) / Number(coef.denom))));
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

function complex(re: number, im: number): Complex {
  return Object.freeze({ re, im });
}

function toComplex(value: number | Complex): Complex {
  return typeof value === "number" ? complex(value, 0) : value;
}

function complexAdd(lhs: Complex, rhs: Complex): Complex {
  return complex(lhs.re + rhs.re, lhs.im + rhs.im);
}

function complexSub(lhs: Complex, rhs: Complex): Complex {
  return complex(lhs.re - rhs.re, lhs.im - rhs.im);
}

function complexNeg(value: Complex): Complex {
  return complex(-value.re, -value.im);
}

function complexMul(lhs: Complex, rhs: Complex): Complex {
  return complex(lhs.re * rhs.re - lhs.im * rhs.im, lhs.re * rhs.im + lhs.im * rhs.re);
}

function complexDiv(lhs: Complex, rhs: Complex): Complex {
  const denom = rhs.re * rhs.re + rhs.im * rhs.im;
  if (denom === 0) throw new RangeError("complex division by zero");
  return complex((lhs.re * rhs.re + lhs.im * rhs.im) / denom, (lhs.im * rhs.re - lhs.re * rhs.im) / denom);
}

function complexAbs(value: Complex): number {
  return Math.hypot(value.re, value.im);
}

function evalPoly(poly: readonly Complex[], z: Complex): Complex {
  return poly.reduce((acc, coef) => complexAdd(complexMul(acc, z), coef), complex(0, 0));
}

function initialRadius(poly: readonly Complex[]): number {
  const degree = poly.length - 1;
  if (degree <= 0) return 1;
  const cauchy = 1 + Math.max(...poly.slice(1).map(complexAbs));
  const constant = complexAbs(poly[degree]);
  const lagrange = constant > 1e-300 ? constant ** (1 / degree) : 1;
  return Math.max(Math.min(cauchy, 10), lagrange, 0.5);
}

function equationToRow(
  equation: IRNode,
  variableColumns: ReadonlyMap<string, number>,
  dimension: number,
): readonly Frac[] | null {
  const expr = isApplyHead(equation, EQUAL.name) && equation.args.length === 2
    ? app(SUB, [equation.args[0], equation.args[1]])
    : equation;
  const linear = linearEval(expr, variableColumns, dimension);
  if (linear === null) return null;
  return [...linear.coeffs, linear.constant.neg()];
}

function nodeToFrac(node: IRNode): Frac | null {
  if (node.kind === "integer") return Frac.fromInt(node.value);
  if (node.kind === "rational") return new Frac(node.numer, node.denom);
  return null;
}

interface LinearForm {
  readonly coeffs: readonly Frac[];
  readonly constant: Frac;
}

function linearEval(
  node: IRNode,
  variableColumns: ReadonlyMap<string, number>,
  dimension: number,
): LinearForm | null {
  const constant = nodeToFrac(node);
  if (constant !== null) return { coeffs: zeroVector(dimension), constant };

  if (node.kind === "symbol") {
    const column = variableColumns.get(node.name);
    if (column === undefined) return null;
    const coeffs = zeroVector(dimension);
    coeffs[column] = Frac.one();
    return { coeffs, constant: Frac.zero() };
  }

  if (node.kind !== "apply" || node.head.kind !== "symbol") return null;
  const head = node.head.name;

  if (head === ADD.name) {
    let coeffs = zeroVector(dimension);
    let value = Frac.zero();
    for (const arg of node.args) {
      const form = linearEval(arg, variableColumns, dimension);
      if (form === null) return null;
      coeffs = addVectors(coeffs, form.coeffs);
      value = value.add(form.constant);
    }
    return { coeffs, constant: value };
  }

  if (head === SUB.name && node.args.length === 2) {
    const lhs = linearEval(node.args[0], variableColumns, dimension);
    const rhs = linearEval(node.args[1], variableColumns, dimension);
    if (lhs === null || rhs === null) return null;
    return { coeffs: subVectors(lhs.coeffs, rhs.coeffs), constant: lhs.constant.sub(rhs.constant) };
  }

  if (head === NEG.name && node.args.length === 1) {
    const form = linearEval(node.args[0], variableColumns, dimension);
    if (form === null) return null;
    return {
      coeffs: form.coeffs.map((coef) => coef.neg()),
      constant: form.constant.neg(),
    };
  }

  if (head === MUL.name) {
    let scalar = Frac.one();
    let linearPart: LinearForm | null = null;
    for (const arg of node.args) {
      const scalarFactor = nodeToFrac(arg);
      if (scalarFactor !== null) {
        scalar = scalar.mul(scalarFactor);
        continue;
      }

      const form = linearEval(arg, variableColumns, dimension);
      if (form === null) return null;
      if (isZeroVector(form.coeffs)) {
        scalar = scalar.mul(form.constant);
      } else {
        if (linearPart !== null) return null;
        linearPart = form;
      }
    }
    if (linearPart === null) return { coeffs: zeroVector(dimension), constant: scalar };
    return {
      coeffs: linearPart.coeffs.map((coef) => coef.mul(scalar)),
      constant: linearPart.constant.mul(scalar),
    };
  }

  if (head === POW.name && node.args.length === 2 && node.args[1].kind === "integer") {
    if (node.args[1].value === 0n) return { coeffs: zeroVector(dimension), constant: Frac.one() };
    if (node.args[1].value === 1n) return linearEval(node.args[0], variableColumns, dimension);
    return null;
  }

  return null;
}

function zeroVector(length: number): Frac[] {
  return Array.from({ length }, () => Frac.zero());
}

function addVectors(lhs: readonly Frac[], rhs: readonly Frac[]): Frac[] {
  return lhs.map((coef, index) => coef.add(rhs[index]));
}

function subVectors(lhs: readonly Frac[], rhs: readonly Frac[]): Frac[] {
  return lhs.map((coef, index) => coef.sub(rhs[index]));
}

function isZeroVector(values: readonly Frac[]): boolean {
  return values.every((value) => value.isZero());
}

function solvePolynomialSign(
  coeffsInput: readonly Frac[],
  variable: IRSymbol,
  wantPositive: boolean,
  strict: boolean,
): readonly IRNode[] {
  const coeffs = trimPolynomial(coeffsInput);
  const degree = coeffs.length - 1;
  if (degree === 0) return signMatches(coeffs[0], wantPositive, strict) ? [allRealsSentinel()] : [];

  const roots = realBoundaryRoots(coeffs);
  if (roots.length === 0) {
    return signMatches(evaluatePolynomial(coeffs, 0), wantPositive, strict) ? [allRealsSentinel()] : [];
  }

  const intervals: IRNode[] = [];
  const samples = intervalSamples(roots.map((root) => root.value));
  for (let index = 0; index < samples.length; index += 1) {
    if (!signMatches(evaluatePolynomial(coeffs, samples[index]), wantPositive, true)) continue;
    const lower = index === 0 ? null : roots[index - 1].node;
    const upper = index === roots.length ? null : roots[index].node;
    intervals.push(makeInterval(variable, lower, upper, strict, strict));
  }

  if (!strict && intervals.length === roots.length + 1) return [allRealsSentinel()];
  return intervals;
}

interface BoundaryRoot {
  readonly value: number;
  readonly node: IRNode;
}

function realBoundaryRoots(coeffsAscending: readonly Frac[]): readonly BoundaryRoot[] {
  const exact = exactPolynomialRoots(coeffsAscending);
  const exactRoots = exact
    .map((node) => ({ value: numericValue(node), node }))
    .filter((root): root is BoundaryRoot => root.value !== null);

  const numericRoots = nsolvePoly([...coeffsAscending].reverse().map(fracToNumber));
  const roots: BoundaryRoot[] = [];
  for (const root of numericRoots) {
    if (Math.abs(root.im) > REAL_ROOT_TOL) continue;
    if (roots.some((candidate) => Math.abs(candidate.value - root.re) < 1e-7)) continue;
    const exactNode = exactRoots.find((candidate) => Math.abs(candidate.value - root.re) < 1e-7)?.node;
    roots.push({ value: root.re, node: exactNode ?? numberNode(root.re) });
  }

  if (roots.length === 0 && coeffsAscending.length === 2) {
    const root = coeffsAscending[0].neg().div(coeffsAscending[1]);
    roots.push({ value: fracToNumber(root), node: root.toIrNode() });
  }

  return roots.sort((lhs, rhs) => lhs.value - rhs.value);
}

function exactPolynomialRoots(coeffsAscending: readonly Frac[]): readonly IRNode[] {
  const coeffs = trimPolynomial(coeffsAscending);
  switch (coeffs.length - 1) {
    case 1: {
      const result = solveLinear(coeffs[1], coeffs[0]);
      return result.kind === "solutions" ? result.roots : [];
    }
    case 2: {
      const result = solveQuadratic(coeffs[2], coeffs[1], coeffs[0]);
      return result.kind === "solutions" ? result.roots : [];
    }
    case 3: {
      const result = solveCubic(coeffs[3], coeffs[2], coeffs[1], coeffs[0]);
      return result.kind === "solutions" ? result.roots : [];
    }
    case 4: {
      const result = solveQuartic(coeffs[4], coeffs[3], coeffs[2], coeffs[1], coeffs[0]);
      return result.kind === "solutions" ? result.roots : [];
    }
    default:
      return [];
  }
}

function intervalSamples(roots: readonly number[]): readonly number[] {
  if (roots.length === 0) return [0];
  const samples: number[] = [roots[0] - Math.max(1, Math.abs(roots[0]) * 0.5)];
  for (let i = 0; i < roots.length - 1; i += 1) {
    samples.push((roots[i] + roots[i + 1]) / 2);
  }
  samples.push(roots[roots.length - 1] + Math.max(1, Math.abs(roots[roots.length - 1]) * 0.5));
  return samples;
}

function makeInterval(
  variable: IRSymbol,
  lower: IRNode | null,
  upper: IRNode | null,
  lowerStrict: boolean,
  upperStrict: boolean,
): IRNode {
  if (lower === null && upper === null) return allRealsSentinel();
  if (lower === null) return app(upperStrict ? LESS : LESS_EQUAL, [variable, upper as IRNode]);
  if (upper === null) return app(lowerStrict ? GREATER : GREATER_EQUAL, [variable, lower]);
  return app(AND, [
    app(lowerStrict ? GREATER : GREATER_EQUAL, [variable, lower]),
    app(upperStrict ? LESS : LESS_EQUAL, [variable, upper]),
  ]);
}

function allRealsSentinel(): IRNode {
  return app(GREATER_EQUAL, [int(0), int(0)]);
}

function signMatches(value: Frac | number, wantPositive: boolean, strict: boolean): boolean {
  const cmp = typeof value === "number"
    ? (Math.abs(value) < 1e-9 ? 0 : value < 0 ? -1 : 1)
    : value.compare(Frac.zero());
  if (wantPositive) return strict ? cmp > 0 : cmp >= 0;
  return strict ? cmp < 0 : cmp <= 0;
}

function evaluatePolynomial(coeffs: readonly Frac[], x: number): number {
  let result = 0;
  for (let i = coeffs.length - 1; i >= 0; i -= 1) {
    result = result * x + fracToNumber(coeffs[i]);
  }
  return result;
}

function extractPolynomial(node: IRNode, variable: string, maxDegree: number): readonly Frac[] | null {
  const constant = nodeToFrac(node);
  if (constant !== null) return [constant];

  if (node.kind === "symbol") {
    if (node.name !== variable) return null;
    return [Frac.zero(), Frac.one()];
  }

  if (node.kind !== "apply" || node.head.kind !== "symbol") return null;
  const head = node.head.name;

  if (head === ADD.name) {
    let result: readonly Frac[] = [Frac.zero()];
    for (const arg of node.args) {
      const poly = extractPolynomial(arg, variable, maxDegree);
      if (poly === null) return null;
      result = addPolynomials(result, poly);
      if (result.length - 1 > maxDegree) return null;
    }
    return trimPolynomial(result);
  }

  if (head === SUB.name && node.args.length === 2) {
    const lhs = extractPolynomial(node.args[0], variable, maxDegree);
    const rhs = extractPolynomial(node.args[1], variable, maxDegree);
    if (lhs === null || rhs === null) return null;
    return trimPolynomial(addPolynomials(lhs, scalePolynomial(rhs, Frac.fromInt(-1))));
  }

  if (head === NEG.name && node.args.length === 1) {
    const poly = extractPolynomial(node.args[0], variable, maxDegree);
    return poly === null ? null : scalePolynomial(poly, Frac.fromInt(-1));
  }

  if (head === MUL.name) {
    let result: readonly Frac[] = [Frac.one()];
    for (const arg of node.args) {
      const poly = extractPolynomial(arg, variable, maxDegree);
      if (poly === null) return null;
      result = multiplyPolynomials(result, poly, maxDegree);
      if (result.length - 1 > maxDegree) return null;
    }
    return trimPolynomial(result);
  }

  if (head === POW.name && node.args.length === 2 && node.args[1].kind === "integer") {
    const exponent = node.args[1].value;
    if (exponent < 0n || exponent > BigInt(maxDegree)) return null;
    const base = extractPolynomial(node.args[0], variable, maxDegree);
    if (base === null) return null;
    let result: readonly Frac[] = [Frac.one()];
    for (let i = 0n; i < exponent; i += 1n) {
      result = multiplyPolynomials(result, base, maxDegree);
      if (result.length - 1 > maxDegree) return null;
    }
    return trimPolynomial(result);
  }

  return null;
}

function addPolynomials(lhs: readonly Frac[], rhs: readonly Frac[]): readonly Frac[] {
  const length = Math.max(lhs.length, rhs.length);
  return trimPolynomial(Array.from({ length }, (_, index) =>
    (lhs[index] ?? Frac.zero()).add(rhs[index] ?? Frac.zero())));
}

function scalePolynomial(poly: readonly Frac[], scalar: Frac): readonly Frac[] {
  return trimPolynomial(poly.map((coef) => coef.mul(scalar)));
}

function multiplyPolynomials(lhs: readonly Frac[], rhs: readonly Frac[], maxDegree: number): readonly Frac[] {
  const result = Array.from({ length: Math.min(lhs.length + rhs.length - 1, maxDegree + 2) }, () => Frac.zero());
  for (let i = 0; i < lhs.length; i += 1) {
    for (let j = 0; j < rhs.length; j += 1) {
      if (i + j > maxDegree) return Array.from({ length: maxDegree + 2 }, () => Frac.one());
      result[i + j] = result[i + j].add(lhs[i].mul(rhs[j]));
    }
  }
  return trimPolynomial(result);
}

function trimPolynomial(poly: readonly Frac[]): readonly Frac[] {
  let end = poly.length - 1;
  while (end > 0 && poly[end].isZero()) end -= 1;
  return poly.slice(0, end + 1);
}

function numericValue(node: IRNode): number | null {
  if (node.kind === "integer") return Number(node.value);
  if (node.kind === "rational") return Number(node.numer) / Number(node.denom);
  if (node.kind === "float") return node.value;
  if (isApplyHead(node, NEG.name) && node.args.length === 1) {
    const value = numericValue(node.args[0]);
    return value === null ? null : -value;
  }
  return null;
}

function fracToNumber(value: Frac): number {
  return Number(value.numer) / Number(value.denom);
}

function fracAbsCompare(lhs: Frac, rhs: Frac): number {
  const l = abs(lhs.numer) * rhs.denom;
  const r = abs(rhs.numer) * lhs.denom;
  return l < r ? -1 : l > r ? 1 : 0;
}

function isApplyHead(node: IRNode, head: string): node is Extract<IRNode, { readonly kind: "apply" }> {
  return node.kind === "apply" && node.head.kind === "symbol" && node.head.name === head;
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
