import { MUL, POW, app, int, sym, type IRNode } from "@coding-adventures/symbolic-ir";

export type IntegerLike = bigint | number | string;

export interface Bezout {
  readonly gcd: bigint;
  readonly s: bigint;
  readonly t: bigint;
}

export type PrimeFactor = readonly [prime: bigint, exponent: number];

export function gcd(aInput: IntegerLike, bInput: IntegerLike): bigint {
  let a = abs(toBigInt(aInput));
  let b = abs(toBigInt(bInput));
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a;
}

export function lcm(aInput: IntegerLike, bInput: IntegerLike): bigint {
  const a = toBigInt(aInput);
  const b = toBigInt(bInput);
  if (a === 0n || b === 0n) return 0n;
  return (abs(a) / gcd(a, b)) * abs(b);
}

export function extendedGcd(aInput: IntegerLike, bInput: IntegerLike): Bezout {
  const a = toBigInt(aInput);
  const b = toBigInt(bInput);
  if (b === 0n) return { gcd: a, s: 1n, t: 0n };
  const next = extendedGcd(b, a % b);
  return {
    gcd: next.gcd,
    s: next.t,
    t: next.s - (a / b) * next.t,
  };
}

export function totient(nInput: IntegerLike): bigint {
  let n = toBigInt(nInput);
  if (n <= 0n) return 0n;
  let result = n;
  let p = 2n;
  while (p * p <= n) {
    if (n % p === 0n) {
      while (n % p === 0n) n /= p;
      result -= result / p;
    }
    p += 1n;
  }
  if (n > 1n) result -= result / n;
  return result;
}

export function modInverse(aInput: IntegerLike, mInput: IntegerLike): bigint | null {
  const a = toBigInt(aInput);
  const m = toBigInt(mInput);
  const result = extendedGcd(a, m);
  if (result.gcd !== 1n) return null;
  return mod(result.s, m);
}

export function modPow(baseInput: IntegerLike, expInput: IntegerLike, modulusInput: IntegerLike): bigint {
  let base = toBigInt(baseInput);
  let exp = toBigInt(expInput);
  const modulus = toBigInt(modulusInput);
  if (exp < 0n) throw new RangeError("modPow exponent must be non-negative");
  if (modulus === 1n) return 0n;
  let result = 1n;
  base = mod(base, modulus);
  while (exp > 0n) {
    if (exp % 2n === 1n) result = mod(result * base, modulus);
    exp /= 2n;
    base = mod(base * base, modulus);
  }
  return result;
}

export function isPrime(nInput: IntegerLike): boolean {
  const n = toBigInt(nInput);
  if (n < 2n) return false;
  if (n < 4n) return true;
  if (n % 2n === 0n || n % 3n === 0n) return false;
  let k = 5n;
  while (k * k <= n) {
    if (n % k === 0n || n % (k + 2n) === 0n) return false;
    k += 6n;
  }
  return true;
}

export function primesUpTo(limitInput: IntegerLike): bigint[] {
  const limit = toBigInt(limitInput);
  if (limit < 2n) return [];
  if (limit > BigInt(Number.MAX_SAFE_INTEGER - 1)) {
    throw new RangeError("primesUpTo limit is too large for an in-memory JS sieve");
  }
  const size = Number(limit) + 1;
  const sieve = Array<boolean>(size).fill(true);
  sieve[0] = false;
  sieve[1] = false;
  for (let p = 2; p * p < size; p += 1) {
    if (!sieve[p]) continue;
    for (let multiple = p * p; multiple < size; multiple += p) {
      sieve[multiple] = false;
    }
  }
  const primes: bigint[] = [];
  sieve.forEach((prime, index) => {
    if (prime) primes.push(BigInt(index));
  });
  return primes;
}

export function nextPrime(nInput: IntegerLike): bigint {
  const n = toBigInt(nInput);
  let candidate = n < 2n ? 2n : n + 1n;
  while (!isPrime(candidate)) candidate += 1n;
  return candidate;
}

export function nthPrime(kInput: number): bigint {
  if (!Number.isInteger(kInput) || kInput < 1) {
    throw new RangeError("nthPrime: k must be a positive integer");
  }
  let count = 0;
  let candidate = 2n;
  while (true) {
    if (isPrime(candidate)) {
      count += 1;
      if (count === kInput) return candidate;
    }
    candidate += 1n;
  }
}

export function factorInteger(nInput: IntegerLike): PrimeFactor[] {
  let n = abs(toBigInt(nInput));
  if (n <= 1n) return [];
  const factors: Array<[bigint, number]> = [];

  if (n % 2n === 0n) {
    let exponent = 0;
    while (n % 2n === 0n) {
      n /= 2n;
      exponent += 1;
    }
    factors.push([2n, exponent]);
  }

  let divisor = 3n;
  while (divisor * divisor <= n) {
    if (n % divisor === 0n) {
      let exponent = 0;
      while (n % divisor === 0n) {
        n /= divisor;
        exponent += 1;
      }
      factors.push([divisor, exponent]);
    }
    divisor += 2n;
  }

  if (n > 1n) factors.push([n, 1]);
  return factors;
}

export function factorizeIr(expr: IRNode): IRNode {
  if (expr.kind !== "integer") return expr;
  const n = expr.value;
  if (n === 0n || n === 1n || n === -1n) return expr;

  const sign = n < 0n ? -1n : 1n;
  const factors = factorInteger(n);
  if (factors.length === 0) return expr;
  if (factors.length === 1 && factors[0][1] === 1 && sign === 1n) return expr;

  const terms: IRNode[] = [];
  if (sign === -1n) terms.push(int(-1n));
  for (const [prime, exponent] of factors) {
    terms.push(exponent === 1 ? int(prime) : app(POW, [int(prime), int(exponent)]));
  }
  return terms.length === 1 ? terms[0] : app(MUL, terms);
}

export function crt(remaindersInput: readonly IntegerLike[], moduliInput: readonly IntegerLike[]): bigint | null {
  if (remaindersInput.length === 0 || remaindersInput.length !== moduliInput.length) return null;
  const remainders = remaindersInput.map(toBigInt);
  const moduli = moduliInput.map(toBigInt);
  if (moduli.some((m) => m <= 0n)) return null;

  let x = mod(remainders[0], moduli[0]);
  let m = moduli[0];

  for (let i = 1; i < remainders.length; i += 1) {
    const r = remainders[i];
    const n = moduli[i];
    const { gcd: g, s } = extendedGcd(m, n);
    const diff = r - x;
    if (diff % g !== 0n) return null;

    const nOverG = n / g;
    const t = mod(mod(diff / g, nOverG) * mod(s, nOverG), nOverG);
    const nextModulus = lcm(m, n);
    x = mod(x + m * t, nextModulus);
    m = nextModulus;
  }

  return x;
}

function mod(value: bigint, modulus: bigint): bigint {
  if (modulus === 0n) throw new RangeError("modulus cannot be zero");
  const result = value % modulus;
  return result < 0n ? result + abs(modulus) : result;
}

function toBigInt(value: IntegerLike): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) {
    throw new RangeError("number inputs must be safe integers; pass bigint or string for larger values");
  }
  return BigInt(value);
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
