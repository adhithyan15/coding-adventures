export type IntegerLike = bigint | number | string;
export type Poly = readonly bigint[];
export type FactorList = Array<[bigint[], number]>;

export const FACTOR = "Factor";
export const IRREDUCIBLE = "Irreducible";

const MAX_COMBOS = 10_000;

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
