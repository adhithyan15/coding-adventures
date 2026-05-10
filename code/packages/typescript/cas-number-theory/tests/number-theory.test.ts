import { describe, expect, it } from "vitest";
import { MUL, POW, app, int, sym } from "@coding-adventures/symbolic-ir";
import {
  crt,
  extendedGcd,
  factorInteger,
  factorizeIr,
  gcd,
  isPrime,
  lcm,
  modInverse,
  modPow,
  nextPrime,
  nthPrime,
  primesUpTo,
  totient,
} from "../src/index";

describe("arithmetic", () => {
  it("computes gcd and lcm", () => {
    expect(gcd(12, 8)).toBe(4n);
    expect(gcd(-12, 8)).toBe(4n);
    expect(gcd(0, 5)).toBe(5n);
    expect(gcd(0, 0)).toBe(0n);
    expect(lcm(4, 6)).toBe(12n);
    expect(lcm(-4, 6)).toBe(12n);
    expect(lcm(0, 5)).toBe(0n);
  });

  it("computes extended gcd and modular inverses", () => {
    for (const [a, b] of [[3n, 5n], [12n, 8n], [100n, 37n], [0n, 5n]]) {
      const result = extendedGcd(a, b);
      expect(a * result.s + b * result.t).toBe(result.gcd);
      expect(result.gcd).toBe(gcd(a, b));
    }
    expect(modInverse(3, 7)).toBe(5n);
    expect(modInverse(2, 4)).toBeNull();
  });

  it("computes totient and modular powers", () => {
    expect(totient(1)).toBe(1n);
    expect(totient(7)).toBe(6n);
    expect(totient(12)).toBe(4n);
    expect(totient(36)).toBe(12n);
    expect(totient(0)).toBe(0n);
    expect(modPow(2, 10, 1000)).toBe(24n);
    expect(modPow(3, 0, 7)).toBe(1n);
    expect(modPow(100, 100, 1)).toBe(0n);
  });
});

describe("primality", () => {
  it("tests primes", () => {
    expect(isPrime(0)).toBe(false);
    expect(isPrime(1)).toBe(false);
    expect(isPrime(2)).toBe(true);
    expect(isPrime(3)).toBe(true);
    expect(isPrime(4)).toBe(false);
    expect(isPrime(97)).toBe(true);
    expect(isPrime(561)).toBe(false);
    expect(isPrime(-7)).toBe(false);
  });

  it("enumerates and selects primes", () => {
    expect(primesUpTo(20)).toEqual([2n, 3n, 5n, 7n, 11n, 13n, 17n, 19n]);
    expect(primesUpTo(1)).toEqual([]);
    expect(primesUpTo(100)).toHaveLength(25);
    expect(nextPrime(0)).toBe(2n);
    expect(nextPrime(13)).toBe(17n);
    expect(nthPrime(1)).toBe(2n);
    expect(nthPrime(10)).toBe(29n);
  });
});

describe("factorization", () => {
  it("factors integers", () => {
    expect(factorInteger(7)).toEqual([[7n, 1]]);
    expect(factorInteger(12)).toEqual([[2n, 2], [3n, 1]]);
    expect(factorInteger(360)).toEqual([[2n, 3], [3n, 2], [5n, 1]]);
    expect(factorInteger(0)).toEqual([]);
    expect(factorInteger(-12)).toEqual([[2n, 2], [3n, 1]]);
  });

  it("reconstructs products", () => {
    for (const n of [2n, 6n, 12n, 100n, 360n, 1023n, 9973n]) {
      const product = factorInteger(n).reduce((acc, [prime, exponent]) => acc * prime ** BigInt(exponent), 1n);
      expect(product).toBe(n);
    }
  });

  it("factorizes IR integers", () => {
    expect(factorizeIr(int(7))).toEqual(int(7));
    expect(factorizeIr(int(0))).toEqual(int(0));
    expect(factorizeIr(sym("x"))).toEqual(sym("x"));

    const factored = factorizeIr(int(12));
    expect(factored.kind).toBe("apply");
    if (factored.kind === "apply") {
      expect(factored.head).toEqual(MUL);
      expect(factored.args).toContainEqual(app(POW, [int(2), int(2)]));
      expect(factored.args).toContainEqual(int(3));
    }

    const negative = factorizeIr(int(-6));
    expect(negative.kind).toBe("apply");
    if (negative.kind === "apply") {
      expect(negative.args).toContainEqual(int(-1));
      expect(negative.args).toContainEqual(int(2));
      expect(negative.args).toContainEqual(int(3));
    }
  });
});

describe("crt", () => {
  it("solves congruence systems", () => {
    expect(crt([2, 3, 2], [3, 5, 7])).toBe(23n);
    expect(crt([5], [7])).toBe(5n);
    expect(crt([1, 0], [2, 3])).toBe(3n);
  });

  it("rejects invalid or inconsistent systems", () => {
    expect(crt([], [])).toBeNull();
    expect(crt([1], [0])).toBeNull();
    expect(crt([1], [-3])).toBeNull();
    expect(crt([0, 1], [4, 2])).toBeNull();
  });

  it("returns the unique representative modulo the lcm", () => {
    const result = crt([2, 3, 2], [3, 5, 7]);
    expect(result).not.toBeNull();
    expect(result! >= 0n && result! < 105n).toBe(true);
    expect(result! % 3n).toBe(2n);
    expect(result! % 5n).toBe(3n);
    expect(result! % 7n).toBe(2n);
  });
});
