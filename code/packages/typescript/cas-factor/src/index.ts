export type IntegerLike = bigint | number | string;
export type Poly = readonly bigint[];
export type FactorList = Array<[bigint[], number]>;

export const FACTOR = "Factor";
export const IRREDUCIBLE = "Irreducible";

const MAX_COMBOS = 10_000;
const MAX_BZH_DEGREE = 20;
const MAX_BZH_PRIME = 200;
const SMALL_PRIMES = smallPrimes(MAX_BZH_PRIME);

export function normalize(poly: readonly IntegerLike[]): bigint[] {
  const out = poly.map(toBigInt);
  while (out.length > 0 && out[out.length - 1] === 0n) {
    out.pop();
  }
  return out;
}

export function degree(poly: readonly IntegerLike[]): number {
  return normalize(poly).length - 1;
}

export function content(poly: readonly IntegerLike[]): bigint {
  const normalized = normalize(poly);
  if (normalized.length === 0) return 0n;
  let g = 0n;
  for (const coeff of normalized) {
    g = gcd(g, abs(coeff));
  }
  return g;
}

export function primitivePart(poly: readonly IntegerLike[]): bigint[] {
  const c = content(poly);
  if (c <= 1n) return normalize(poly);
  return normalize(poly).map((coeff) => coeff / c);
}

export function evaluate(poly: readonly IntegerLike[], x: IntegerLike): bigint {
  const normalized = normalize(poly);
  const xv = toBigInt(x);
  let out = 0n;
  for (let i = normalized.length - 1; i >= 0; i -= 1) {
    out = out * xv + normalized[i];
  }
  return out;
}

export function divideLinear(poly: readonly IntegerLike[], root: IntegerLike): bigint[] {
  const normalized = normalize(poly);
  if (normalized.length === 0) return [];
  const r = toBigInt(root);
  const quotient = Array<bigint>(normalized.length - 1).fill(0n);
  let remainder = 0n;
  for (let i = normalized.length - 1; i >= 0; i -= 1) {
    remainder = remainder * r + normalized[i];
    if (i > 0) {
      quotient[i - 1] = remainder;
    }
  }
  return normalize(quotient);
}

export function divisors(value: IntegerLike): bigint[] {
  const n = abs(toBigInt(value));
  if (n === 0n) return [];
  const out: bigint[] = [];
  for (let i = 1n; i * i <= n; i += 1n) {
    if (n % i === 0n) {
      out.push(i);
      if (i !== n / i) out.push(n / i);
    }
  }
  return out.sort(compareBigint);
}

export function findIntegerRoots(poly: readonly IntegerLike[]): bigint[] {
  const normalized = normalize(poly);
  if (normalized.length === 0) return [];
  const constant = normalized[0];
  if (constant === 0n) {
    const rest = normalized.slice(1);
    return rest.length === 0 ? [0n] : [0n, ...findIntegerRoots(rest)];
  }

  const candidates = new Set<bigint>();
  for (const divisor of divisors(constant)) {
    candidates.add(divisor);
    candidates.add(-divisor);
  }
  return [...candidates].sort(compareBigint).filter((candidate) => evaluate(normalized, candidate) === 0n);
}

export function extractLinearFactors(poly: readonly IntegerLike[]): [Array<[bigint, number]>, bigint[]] {
  let residual = normalize(poly);
  const multiplicities = new Map<string, { root: bigint; count: number }>();

  while (true) {
    const roots = findIntegerRoots(residual);
    if (roots.length === 0) break;
    for (const root of roots) {
      residual = divideLinear(residual, root);
      const key = root.toString();
      const entry = multiplicities.get(key);
      if (entry) {
        entry.count += 1;
      } else {
        multiplicities.set(key, { root, count: 1 });
      }
    }
  }

  const factors = [...multiplicities.values()]
    .sort((a, b) => compareBigint(a.root, b.root))
    .map((entry): [bigint, number] => [entry.root, entry.count]);
  return [factors, residual];
}

export function kroneckerFactor(poly: readonly IntegerLike[]): [bigint[], bigint[]] | null {
  const p = normalize(poly);
  const d = degree(p);
  if (d < 2) return null;

  for (let k = 1; k <= Math.floor(d / 2); k += 1) {
    const points = evalPoints(k + 1);
    const values = points.map((point) => evaluate(p, point));
    if (values.some((value) => value === 0n)) continue;

    const divisorSets = values.map(signedDivisors);
    if (divisorSets.some((set) => set.length === 0)) continue;

    let combos = 0;
    for (const ys of product(divisorSets)) {
      combos += 1;
      if (combos > MAX_COMBOS) break;
      const interpolated = lagrangeInterpolate(points, ys);
      if (interpolated === null) continue;

      const candidate: bigint[] = [];
      let integral = true;
      for (const coeff of interpolated) {
        if (coeff.denom !== 1n) {
          integral = false;
          break;
        }
        candidate.push(coeff.numer);
      }
      if (!integral) continue;

      const normalizedCandidate = normalizePositiveLeading(candidate);
      if (normalizedCandidate.length <= 1 || normalizedCandidate.length >= p.length) continue;

      const cofactor = dividesExactly(p, normalizedCandidate);
      if (cofactor !== null) {
        return [normalizedCandidate, normalizePositiveLeading(cofactor)];
      }
    }
  }

  return null;
}

export function bzhFactor(poly: readonly IntegerLike[]): bigint[][] | null {
  const f = normalize(poly);
  if (f.length === 0) return null;

  const d = f.length - 1;
  if (d < 2 || d > MAX_BZH_DEGREE) return null;
  if (f[f.length - 1] !== 1n) return null;

  let goodPrime: number | null = null;
  for (const p of SMALL_PRIMES) {
    if (isSquarefreeModP(f, p)) {
      goodPrime = p;
      break;
    }
  }
  if (goodPrime === null) return null;

  const p = goodPrime;
  const modFactors = berlekampFactorModP(pmodBigint(f, p), p);
  if (modFactors.length < 2) return null;

  const target = 2 * zassenhausBound(f) + 1;
  if (!Number.isFinite(target)) return null;

  const lifted = multiHenselLift(f, modFactors, p, target);
  if (lifted === null) return null;

  let modulus = BigInt(p);
  while (Number(modulus) <= target) {
    modulus *= BigInt(p);
  }

  const combined = combineBzhFactors(f, lifted, modulus);
  if (combined === null || combined.length < 2) return null;
  if (combined.length === 1 && normalizePositiveLeading(combined[0]).join(",") === f.join(",")) {
    return null;
  }
  return combined;
}

export function factorIntegerPolynomial(poly: readonly IntegerLike[]): [bigint, FactorList] {
  const normalized = normalize(poly);
  if (normalized.length === 0) return [0n, []];

  let c = content(normalized);
  const primitive = primitivePart(normalized);
  const [linearFactors, residual] = extractLinearFactors(primitive);
  const factors: FactorList = [];

  for (const [root, multiplicity] of linearFactors) {
    factors.push([[-root, 1n], multiplicity]);
  }

  if (residual.length > 0 && !(residual.length === 1 && abs(residual[0]) === 1n)) {
    factors.push(...factorResidual(residual));
  } else if (residual.length === 1 && residual[0] === -1n) {
    c = -c;
  }

  return [c, factors];
}

function factorResidual(poly: bigint[]): FactorList {
  const counts = new Map<string, { poly: bigint[]; count: number }>();
  const queue = [normalize(poly)];

  while (queue.length > 0) {
    let piece = normalize(queue.pop() ?? []);
    if (piece.length <= 1) continue;

    if (piece.length === 2) {
      piece = normalizePositiveLeading(piece);
      addFactor(counts, piece);
      continue;
    }

    const split = kroneckerFactor(piece);
    if (split === null) {
      if (piece.length >= 5 && piece[piece.length - 1] === 1n) {
        const bzh = bzhFactor(piece);
        if (bzh !== null && bzh.length >= 2) {
          queue.push(...bzh);
          continue;
        }
      }
      addFactor(counts, normalizePositiveLeading(piece));
    } else {
      queue.push(split[0], split[1]);
    }
  }

  return [...counts.values()].map((entry) => [entry.poly, entry.count]);
}

function addFactor(counts: Map<string, { poly: bigint[]; count: number }>, poly: bigint[]): void {
  const key = poly.join(",");
  const entry = counts.get(key);
  if (entry) {
    entry.count += 1;
  } else {
    counts.set(key, { poly, count: 1 });
  }
}

function pmodBigint(coeffs: readonly bigint[], p: number): number[] {
  const modulus = BigInt(p);
  const out = coeffs.map((coeff) => Number(((coeff % modulus) + modulus) % modulus));
  while (out.length > 0 && out[out.length - 1] === 0) out.pop();
  return out;
}

function pmodNumber(coeffs: readonly number[], p: number): number[] {
  const out = coeffs.map((coeff) => modNumber(coeff, p));
  while (out.length > 0 && out[out.length - 1] === 0) out.pop();
  return out;
}

function pdeg(poly: readonly number[]): number {
  return poly.length - 1;
}

function padd(a: readonly number[], b: readonly number[], p: number): number[] {
  const n = Math.max(a.length, b.length);
  const result = Array<number>(n).fill(0);
  for (let i = 0; i < a.length; i += 1) result[i] = modNumber(result[i] + a[i], p);
  for (let i = 0; i < b.length; i += 1) result[i] = modNumber(result[i] + b[i], p);
  while (result.length > 0 && result[result.length - 1] === 0) result.pop();
  return result;
}

function psub(a: readonly number[], b: readonly number[], p: number): number[] {
  const n = Math.max(a.length, b.length);
  const result = Array<number>(n).fill(0);
  for (let i = 0; i < a.length; i += 1) result[i] = modNumber(result[i] + a[i], p);
  for (let i = 0; i < b.length; i += 1) result[i] = modNumber(result[i] - b[i], p);
  while (result.length > 0 && result[result.length - 1] === 0) result.pop();
  return result;
}

function pmul(a: readonly number[], b: readonly number[], p: number): number[] {
  if (a.length === 0 || b.length === 0) return [];
  const result = Array<number>(a.length + b.length - 1).fill(0);
  for (let i = 0; i < a.length; i += 1) {
    for (let j = 0; j < b.length; j += 1) {
      result[i + j] = modNumber(result[i + j] + a[i] * b[j], p);
    }
  }
  while (result.length > 0 && result[result.length - 1] === 0) result.pop();
  return result;
}

function pscale(poly: readonly number[], scalar: number, p: number): number[] {
  const result = poly.map((coeff) => modNumber(coeff * scalar, p));
  while (result.length > 0 && result[result.length - 1] === 0) result.pop();
  return result;
}

function pmodPoly(aInput: readonly number[], b: readonly number[], p: number): number[] {
  const a = [...aInput];
  const db = pdeg(b);
  if (db < 0) throw new RangeError("division by zero polynomial");
  const leadInv = modInverse(b[b.length - 1], p);
  while (pdeg(a) >= db) {
    const shift = pdeg(a) - db;
    const factor = modNumber(a[a.length - 1] * leadInv, p);
    for (let k = 0; k < b.length; k += 1) {
      a[shift + k] = modNumber(a[shift + k] - factor * b[k], p);
    }
    while (a.length > 0 && a[a.length - 1] === 0) a.pop();
  }
  return a;
}

function pdivQuotient(aInput: readonly number[], b: readonly number[], p: number): number[] {
  const a = [...aInput];
  const db = pdeg(b);
  if (db < 0) throw new RangeError("division by zero polynomial");
  const leadInv = modInverse(b[b.length - 1], p);
  const quotient: number[] = [];
  while (pdeg(a) >= db) {
    const shift = pdeg(a) - db;
    const factor = modNumber(a[a.length - 1] * leadInv, p);
    while (quotient.length <= shift) quotient.push(0);
    quotient[shift] = modNumber(quotient[shift] + factor, p);
    for (let k = 0; k < b.length; k += 1) {
      a[shift + k] = modNumber(a[shift + k] - factor * b[k], p);
    }
    while (a.length > 0 && a[a.length - 1] === 0) a.pop();
  }
  while (quotient.length > 0 && quotient[quotient.length - 1] === 0) quotient.pop();
  return quotient;
}

function pgcd(aInput: readonly number[], bInput: readonly number[], p: number): number[] {
  let a = pmodNumber(aInput, p);
  let b = pmodNumber(bInput, p);
  while (b.length > 0) {
    [a, b] = [b, pmodPoly(a, b, p)];
  }
  if (a.length > 0 && a[a.length - 1] !== 1) {
    a = pscale(a, modInverse(a[a.length - 1], p), p);
  }
  return a;
}

function pgcdExtended(
  a: readonly number[],
  b: readonly number[],
  p: number,
): [number[], number[], number[]] {
  let oldR = [...a];
  let r = [...b];
  let oldS = [1];
  let s: number[] = [];
  let oldT: number[] = [];
  let t = [1];

  while (r.length > 0) {
    const q = pdivQuotient(oldR, r, p);
    [oldR, r] = [r, psub(oldR, pmul(q, r, p), p)];
    [oldS, s] = [s, psub(oldS, pmul(q, s, p), p)];
    [oldT, t] = [t, psub(oldT, pmul(q, t, p), p)];
  }

  if (oldR.length > 0 && oldR[oldR.length - 1] !== 1) {
    const inv = modInverse(oldR[oldR.length - 1], p);
    oldR = pscale(oldR, inv, p);
    oldS = pscale(oldS, inv, p);
    oldT = pscale(oldT, inv, p);
  }
  return [oldR, oldS, oldT];
}

function pderiv(poly: readonly number[], p: number): number[] {
  if (poly.length <= 1) return [];
  const result = Array.from({ length: poly.length - 1 }, (_, i) => modNumber((i + 1) * poly[i + 1], p));
  while (result.length > 0 && result[result.length - 1] === 0) result.pop();
  return result;
}

function isSquarefreeModP(poly: readonly bigint[], p: number): boolean {
  const f = pmodBigint(poly, p);
  if (f.length === 0) return false;
  const df = pderiv(f, p);
  if (df.length === 0) return false;
  return pdeg(pgcd(f, df, p)) === 0;
}

function polyPowmod(exp: number, modPoly: readonly number[], p: number): number[] {
  let result = [1];
  let current = pmodPoly([0, 1], modPoly, p);
  while (exp > 0) {
    if ((exp & 1) === 1) result = pmodPoly(pmul(result, current, p), modPoly, p);
    current = pmodPoly(pmul(current, current, p), modPoly, p);
    exp = Math.floor(exp / 2);
  }
  return result;
}

function nullSpaceModP(matrix: readonly number[][], n: number, p: number): number[][] {
  const a = matrix.map((row) => [...row]);
  const pivotCols: number[] = [];
  let row = 0;

  for (let col = 0; col < n; col += 1) {
    let pivot = -1;
    for (let r = row; r < n; r += 1) {
      if (a[r][col] !== 0) {
        pivot = r;
        break;
      }
    }
    if (pivot === -1) continue;

    [a[row], a[pivot]] = [a[pivot], a[row]];
    const inv = modInverse(a[row][col], p);
    a[row] = a[row].map((value) => modNumber(value * inv, p));
    for (let r = 0; r < n; r += 1) {
      if (r === row || a[r][col] === 0) continue;
      const factor = a[r][col];
      a[r] = a[r].map((value, j) => modNumber(value - factor * a[row][j], p));
    }
    pivotCols.push(col);
    row += 1;
  }

  const pivotSet = new Set(pivotCols);
  const pivotRow = new Map<number, number>();
  pivotCols.forEach((col, index) => pivotRow.set(col, index));
  const basis: number[][] = [];

  for (let freeCol = 0; freeCol < n; freeCol += 1) {
    if (pivotSet.has(freeCol)) continue;
    const vector = Array<number>(n).fill(0);
    vector[freeCol] = 1;
    for (const pivotCol of pivotCols) {
      const r = pivotRow.get(pivotCol) ?? 0;
      vector[pivotCol] = modNumber(-a[r][freeCol], p);
    }
    basis.push(vector);
  }

  return basis.length > 0 ? basis : [[1, ...Array<number>(Math.max(0, n - 1)).fill(0)]];
}

function berlekampFactorModP(f: readonly number[], p: number): number[][] {
  const n = pdeg(f);
  if (n <= 0) return f.length > 0 ? [[...f]] : [];
  if (n === 1) return [[...f]];

  const xpModF = polyPowmod(p, f, p);
  const qMatrix: number[][] = [];
  let current = [1];
  for (let j = 0; j < n; j += 1) {
    qMatrix.push([...current, ...Array<number>(n - current.length).fill(0)]);
    current = pmodPoly(pmul(current, xpModF, p), f, p);
  }

  const matrix = Array.from({ length: n }, () => Array<number>(n).fill(0));
  for (let i = 0; i < n; i += 1) {
    for (let j = 0; j < n; j += 1) {
      matrix[i][j] = modNumber(qMatrix[j][i] - (i === j ? 1 : 0), p);
    }
  }

  const basis = nullSpaceModP(matrix, n, p);
  const targetFactorCount = basis.length;
  if (targetFactorCount === 1) return [[...f]];

  let factors: number[][] = [[...f]];
  for (const vector of basis.slice(1)) {
    if (factors.length === targetFactorCount) break;
    const nextFactors: number[][] = [];
    for (const factor of factors) {
      if (pdeg(factor) <= 0) {
        nextFactors.push(factor);
        continue;
      }

      let splitFound = false;
      for (let s = 0; s < p; s += 1) {
        const shifted = [...vector];
        shifted[0] = modNumber((shifted[0] ?? 0) - s, p);
        while (shifted.length > 0 && shifted[shifted.length - 1] === 0) shifted.pop();
        const h = pgcd(factor, shifted.length > 0 ? shifted : [0], p);
        if (pdeg(h) > 0 && pdeg(h) < pdeg(factor)) {
          nextFactors.push(h, pdivQuotient(factor, h, p));
          splitFound = true;
          break;
        }
      }

      if (!splitFound) nextFactors.push(factor);
    }
    factors = nextFactors;
  }

  return factors
    .filter((factor) => factor.length > 0)
    .map((factor) => (factor[factor.length - 1] === 1 ? factor : pscale(factor, modInverse(factor[factor.length - 1], p), p)));
}

function zassenhausBound(poly: readonly bigint[]): number {
  const d = poly.length - 1;
  if (d < 0) return 0;
  let sumSquares = 0;
  for (const coeff of poly) {
    const n = Number(coeff);
    if (!Number.isFinite(n)) return Number.POSITIVE_INFINITY;
    sumSquares += n * n;
  }
  return 2 ** d * Math.sqrt(d + 1) * Math.sqrt(sumSquares);
}

function izMul(a: readonly bigint[], b: readonly bigint[]): bigint[] {
  if (a.length === 0 || b.length === 0) return [];
  const result = Array<bigint>(a.length + b.length - 1).fill(0n);
  for (let i = 0; i < a.length; i += 1) {
    for (let j = 0; j < b.length; j += 1) {
      result[i + j] += a[i] * b[j];
    }
  }
  return normalize(result);
}

function izSub(a: readonly bigint[], b: readonly bigint[]): bigint[] {
  const n = Math.max(a.length, b.length);
  const result = Array<bigint>(n).fill(0n);
  for (let i = 0; i < a.length; i += 1) result[i] += a[i];
  for (let i = 0; i < b.length; i += 1) result[i] -= b[i];
  return normalize(result);
}

function centerModBigint(coeffs: readonly bigint[], modulus: bigint): bigint[] {
  const half = modulus / 2n;
  const result = coeffs.map((coeff) => {
    let r = ((coeff % modulus) + modulus) % modulus;
    if (r > half) r -= modulus;
    return r;
  });
  return normalize(result);
}

function toZCentered(poly: readonly number[], p: number): bigint[] {
  const half = Math.floor(p / 2);
  const result = poly.map((coeff) => BigInt(coeff <= half ? coeff : coeff - p));
  return normalize(result);
}

function diophantineModP(
  a: readonly number[],
  b: readonly number[],
  c: readonly number[],
  p: number,
): [number[], number[]] {
  const [, s, t] = pgcdExtended(a, b, p);
  const sc = pmul(s, c, p);
  const u = pmodPoly(sc, b, p);
  const q = pdivQuotient(sc, b, p);
  const v = pmodPoly(padd(pmul(t, c, p), pmul(q, a, p), p), a, p);
  return [u, v];
}

function linearHenselLift(
  f: readonly bigint[],
  gInit: readonly number[],
  hInit: readonly number[],
  p: number,
  targetMod: bigint,
): [bigint[], bigint[]] | null {
  const gMod = pmodNumber(gInit, p);
  const hMod = pmodNumber(hInit, p);
  if (pdeg(pgcd(gMod, hMod, p)) !== 0) return null;

  let g = toZCentered(gMod, p);
  let h = toZCentered(hMod, p);
  let pk = BigInt(p);
  let modulus = BigInt(p);

  while (modulus < targetMod) {
    const diff = izSub(f, izMul(g, h));
    if (diff.length === 0) break;
    if (diff.some((coeff) => coeff % pk !== 0n)) return null;
    const error = normalize(diff.map((coeff) => coeff / pk));
    const errorMod = pmodBigint(error, p);
    const [uMod, vMod] = diophantineModP(gMod, hMod, errorMod, p);
    const u = toZCentered(uMod, p);
    const v = toZCentered(vMod, p);

    const nextG = [...g];
    for (let i = 0; i < v.length; i += 1) {
      while (nextG.length <= i) nextG.push(0n);
      nextG[i] += pk * v[i];
    }
    const nextH = [...h];
    for (let i = 0; i < u.length; i += 1) {
      while (nextH.length <= i) nextH.push(0n);
      nextH[i] += pk * u[i];
    }

    g = normalize(nextG);
    h = normalize(nextH);
    pk *= BigInt(p);
    modulus *= BigInt(p);
  }

  return [centerModBigint(g, targetMod), centerModBigint(h, targetMod)];
}

function multiHenselLift(
  f: readonly bigint[],
  factorsModP: readonly number[][],
  p: number,
  target: number,
): bigint[][] | null {
  if (factorsModP.length === 0) return [];
  if (factorsModP.length === 1) return [[...f]];

  let modulus = BigInt(p);
  while (Number(modulus) <= target) {
    modulus *= BigInt(p);
  }

  if (factorsModP.length === 2) {
    const lifted = linearHenselLift(f, factorsModP[0], factorsModP[1], p, modulus);
    return lifted === null ? null : [lifted[0], lifted[1]];
  }

  const mid = Math.floor(factorsModP.length / 2);
  const leftFactors = factorsModP.slice(0, mid);
  const rightFactors = factorsModP.slice(mid);
  let leftProduct = leftFactors.reduce((acc, factor) => pmul(acc, factor, p), [1]);
  let rightProduct = rightFactors.reduce((acc, factor) => pmul(acc, factor, p), [1]);

  if (leftProduct.length > 0 && leftProduct[leftProduct.length - 1] !== 1) {
    leftProduct = pscale(leftProduct, modInverse(leftProduct[leftProduct.length - 1], p), p);
  }
  if (rightProduct.length > 0 && rightProduct[rightProduct.length - 1] !== 1) {
    rightProduct = pscale(rightProduct, modInverse(rightProduct[rightProduct.length - 1], p), p);
  }

  const pair = linearHenselLift(f, leftProduct, rightProduct, p, modulus);
  if (pair === null) return null;

  const leftLifted = multiHenselLift(pair[0], leftFactors, p, target);
  const rightLifted = multiHenselLift(pair[1], rightFactors, p, target);
  if (leftLifted === null || rightLifted === null) return null;
  return [...leftLifted, ...rightLifted];
}

function exactPolynomialDivides(poly: readonly bigint[], divisor: readonly bigint[]): bigint[] | null {
  const f = normalize(poly);
  const g = normalize(divisor);
  if (g.length === 0 || g.length > f.length) return null;
  if (g.length === 1) {
    if (g[0] === 0n || f.some((coeff) => coeff % g[0] !== 0n)) return null;
    return normalize(f.map((coeff) => coeff / g[0]));
  }

  const remainder = [...f];
  const quotient = Array<bigint>(f.length - g.length + 1).fill(0n);
  while (remainder.length >= g.length) {
    const shift = remainder.length - g.length;
    const lead = remainder[remainder.length - 1];
    const divisorLead = g[g.length - 1];
    if (lead % divisorLead !== 0n) return null;
    const q = lead / divisorLead;
    quotient[shift] = q;
    for (let i = 0; i < g.length; i += 1) {
      remainder[shift + i] -= q * g[i];
    }
    while (remainder.length > 0 && remainder[remainder.length - 1] === 0n) remainder.pop();
  }
  return remainder.length === 0 ? normalize(quotient) : null;
}

function combineBzhFactors(
  f: readonly bigint[],
  lifted: readonly bigint[][],
  modulus: bigint,
): bigint[][] | null {
  let remainingF = [...f];
  let remainingLifted = lifted.map((factor) => [...factor]);
  const factors: bigint[][] = [];

  while (remainingLifted.length > 1) {
    let found = false;
    const maxSize = Math.floor(remainingLifted.length / 2);
    for (let size = 1; size <= maxSize; size += 1) {
      for (const subset of combinations(remainingLifted.length, size)) {
        let productPoly = [1n];
        for (const index of subset) {
          productPoly = izMul(productPoly, remainingLifted[index]);
        }
        const primitive = normalizePositiveLeading(primitivePart(centerModBigint(productPoly, modulus)));
        if (primitive.length === 0) continue;
        const quotient = exactPolynomialDivides(remainingF, primitive);
        if (quotient !== null) {
          factors.push(primitive);
          remainingF = normalizePositiveLeading(primitivePart(quotient));
          const selected = new Set(subset);
          remainingLifted = remainingLifted.filter((_, index) => !selected.has(index));
          found = true;
          break;
        }
      }
      if (found) break;
    }
    if (!found) break;
  }

  remainingF = normalize(remainingF);
  if (remainingF.length > 1 && !(remainingF.length === 1 && abs(remainingF[0]) === 1n)) {
    factors.push(normalizePositiveLeading(remainingF));
  }

  return factors.length > 0 ? factors : null;
}

function* combinations(n: number, size: number, start = 0, prefix: number[] = []): Generator<number[]> {
  if (prefix.length === size) {
    yield [...prefix];
    return;
  }
  for (let i = start; i <= n - (size - prefix.length); i += 1) {
    prefix.push(i);
    yield* combinations(n, size, i + 1, prefix);
    prefix.pop();
  }
}

function smallPrimes(limit: number): number[] {
  const sieve = Array<boolean>(limit + 1).fill(true);
  sieve[0] = false;
  sieve[1] = false;
  const primes: number[] = [];
  for (let i = 2; i <= limit; i += 1) {
    if (!sieve[i]) continue;
    primes.push(i);
    for (let j = i * i; j <= limit; j += i) sieve[j] = false;
  }
  return primes;
}

function modNumber(value: number, p: number): number {
  return ((value % p) + p) % p;
}

function modInverse(value: number, p: number): number {
  let t = 0;
  let nextT = 1;
  let r = p;
  let nextR = modNumber(value, p);
  while (nextR !== 0) {
    const q = Math.floor(r / nextR);
    [t, nextT] = [nextT, t - q * nextT];
    [r, nextR] = [nextR, r - q * nextR];
  }
  if (r > 1) throw new RangeError("value is not invertible modulo p");
  return modNumber(t, p);
}

function dividesExactly(poly: bigint[], candidate: bigint[]): bigint[] | null {
  const [quotient, remainder] = polyDivmodFrac(poly.map(Rational.fromInt), candidate.map(Rational.fromInt));
  if (remainder.length > 0) return null;

  const out: bigint[] = [];
  for (const coeff of quotient) {
    if (coeff.denom !== 1n) return null;
    out.push(coeff.numer);
  }
  return normalize(out);
}

function polyDivmodFrac(aInput: Rational[], bInput: Rational[]): [Rational[], Rational[]] {
  const a = trimRational(aInput);
  const b = trimRational(bInput);
  if (b.length === 0) throw new RangeError("division by zero polynomial");

  const db = b.length - 1;
  const quotient = Array.from(
    { length: Math.max(0, a.length - b.length + 1) },
    () => Rational.ZERO,
  );
  while (a.length > db) {
    const c = a[a.length - 1].div(b[b.length - 1]);
    const shift = a.length - b.length;
    quotient[shift] = c;
    for (let k = 0; k < b.length; k += 1) {
      a[shift + k] = a[shift + k].sub(c.mul(b[k]));
    }
    trimRationalInPlace(a);
  }

  trimRationalInPlace(quotient);
  return [quotient, a];
}

function lagrangeInterpolate(xs: bigint[], ys: bigint[]): Rational[] | null {
  const n = xs.length;
  const result = Array.from({ length: n }, () => Rational.ZERO);

  for (let i = 0; i < n; i += 1) {
    let denom = Rational.ONE;
    for (let j = 0; j < n; j += 1) {
      if (i === j) continue;
      const diff = xs[i] - xs[j];
      if (diff === 0n) return null;
      denom = denom.mul(Rational.fromInt(diff));
    }

    const weight = Rational.fromInt(ys[i]).div(denom);
    let basis = [Rational.ONE];
    for (let j = 0; j < n; j += 1) {
      if (i === j) continue;
      const next = Array.from({ length: basis.length + 1 }, () => Rational.ZERO);
      for (let k = 0; k < basis.length; k += 1) {
        next[k + 1] = next[k + 1].add(basis[k]);
        next[k] = next[k].sub(basis[k].mul(Rational.fromInt(xs[j])));
      }
      basis = next;
    }

    for (let k = 0; k < basis.length; k += 1) {
      result[k] = result[k].add(weight.mul(basis[k]));
    }
  }

  return result;
}

function evalPoints(count: number): bigint[] {
  const points: bigint[] = [];
  let i = 0n;
  while (points.length < count) {
    if (i === 0n) {
      points.push(0n);
    } else {
      points.push(i);
      if (points.length < count) points.push(-i);
    }
    i += 1n;
  }
  return points;
}

function signedDivisors(value: bigint): bigint[] {
  if (value === 0n) return [];
  const out: bigint[] = [];
  for (const divisor of divisors(value)) {
    out.push(divisor, -divisor);
  }
  return out;
}

function* product(sets: bigint[][], index = 0, prefix: bigint[] = []): Generator<bigint[]> {
  if (index === sets.length) {
    yield [...prefix];
    return;
  }
  for (const value of sets[index]) {
    prefix.push(value);
    yield* product(sets, index + 1, prefix);
    prefix.pop();
  }
}

function normalizePositiveLeading(poly: readonly IntegerLike[]): bigint[] {
  const normalized = normalize(poly);
  if (normalized.length > 0 && normalized[normalized.length - 1] < 0n) {
    return normalized.map((coeff) => -coeff);
  }
  return normalized;
}

function trimRational(values: Rational[]): Rational[] {
  const out = [...values];
  trimRationalInPlace(out);
  return out;
}

function trimRationalInPlace(values: Rational[]): void {
  while (values.length > 0 && values[values.length - 1].isZero()) {
    values.pop();
  }
}

function toBigInt(value: IntegerLike): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) {
    throw new RangeError("number inputs must be safe integers; pass bigint or string for larger values");
  }
  return BigInt(value);
}

function gcd(a: bigint, b: bigint): bigint {
  a = abs(a);
  b = abs(b);
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a;
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}

function compareBigint(a: bigint, b: bigint): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

class Rational {
  static readonly ZERO = new Rational(0n, 1n);
  static readonly ONE = new Rational(1n, 1n);

  readonly numer: bigint;
  readonly denom: bigint;

  constructor(numer: bigint, denom: bigint) {
    if (denom === 0n) throw new RangeError("Rational denominator cannot be zero");
    if (numer === 0n) {
      this.numer = 0n;
      this.denom = 1n;
      return;
    }
    if (denom < 0n) {
      numer = -numer;
      denom = -denom;
    }
    const g = gcd(numer, denom);
    this.numer = numer / g;
    this.denom = denom / g;
  }

  static fromInt(value: bigint): Rational {
    return new Rational(value, 1n);
  }

  add(other: Rational): Rational {
    return new Rational(this.numer * other.denom + other.numer * this.denom, this.denom * other.denom);
  }

  sub(other: Rational): Rational {
    return new Rational(this.numer * other.denom - other.numer * this.denom, this.denom * other.denom);
  }

  mul(other: Rational): Rational {
    return new Rational(this.numer * other.numer, this.denom * other.denom);
  }

  div(other: Rational): Rational {
    if (other.isZero()) throw new RangeError("Rational division by zero");
    return new Rational(this.numer * other.denom, this.denom * other.numer);
  }

  isZero(): boolean {
    return this.numer === 0n;
  }
}
