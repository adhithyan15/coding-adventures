/**
 * Tests for @coding-adventures/rng
 *
 * All reference values are cross-checked against the Go reference
 * implementation. Every BigInt seed is passed with the `n` suffix.
 */

import { describe, it, expect } from "vitest";
import { VERSION, LCG, Xorshift64, PCG32 } from "../src/index.js";

// ── Package sanity ────────────────────────────────────────────────────────────

describe("package", () => {
  it("exports VERSION 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ── Reference values (seed = 1n) ──────────────────────────────────────────────

describe("LCG known values", () => {
  it("matches Go reference for seed=1", () => {
    const g = new LCG(1n);
    expect(g.nextU32()).toBe(1817669548);
    expect(g.nextU32()).toBe(2187888307);
    expect(g.nextU32()).toBe(2784682393);
  });
});

describe("Xorshift64 known values", () => {
  it("matches Go reference for seed=1", () => {
    const g = new Xorshift64(1n);
    expect(g.nextU32()).toBe(1082269761);
    expect(g.nextU32()).toBe(201397313);
    expect(g.nextU32()).toBe(1854285353);
  });
});

describe("PCG32 known values", () => {
  it("matches Go reference for seed=1", () => {
    const g = new PCG32(1n);
    expect(g.nextU32()).toBe(1412771199);
    expect(g.nextU32()).toBe(1791099446);
    expect(g.nextU32()).toBe(124312908);
  });
});

// ── Determinism ───────────────────────────────────────────────────────────────

function seq32(gen: LCG | Xorshift64 | PCG32, n = 10): number[] {
  return Array.from({ length: n }, () => gen.nextU32());
}

describe("determinism", () => {
  it("LCG: same seed → same sequence", () => {
    expect(seq32(new LCG(42n))).toEqual(seq32(new LCG(42n)));
  });

  it("Xorshift64: same seed → same sequence", () => {
    expect(seq32(new Xorshift64(42n))).toEqual(seq32(new Xorshift64(42n)));
  });

  it("PCG32: same seed → same sequence", () => {
    expect(seq32(new PCG32(42n))).toEqual(seq32(new PCG32(42n)));
  });
});

// ── Different seeds diverge ────────────────────────────────────────────────────

describe("different seeds diverge", () => {
  it("LCG", () => {
    expect(seq32(new LCG(1n), 5)).not.toEqual(seq32(new LCG(2n), 5));
  });

  it("Xorshift64", () => {
    expect(seq32(new Xorshift64(1n), 5)).not.toEqual(seq32(new Xorshift64(2n), 5));
  });

  it("PCG32", () => {
    expect(seq32(new PCG32(1n), 5)).not.toEqual(seq32(new PCG32(2n), 5));
  });
});

// ── Seed-0 Xorshift64 ─────────────────────────────────────────────────────────

describe("Xorshift64 seed=0", () => {
  it("is not stuck at zero", () => {
    const g = new Xorshift64(0n);
    for (let i = 0; i < 100; i++) {
      expect(g.nextU32()).not.toBe(0);
    }
  });

  it("seed 0n produces same as seed 1n (0 replaced by 1)", () => {
    const g0 = new Xorshift64(0n);
    const g1 = new Xorshift64(1n);
    expect(g0.nextU32()).toBe(g1.nextU32());
  });
});

// ── Float range ───────────────────────────────────────────────────────────────

describe("nextFloat range", () => {
  it("LCG: always in [0, 1)", () => {
    const g = new LCG(7n);
    for (let i = 0; i < 1000; i++) {
      const f = g.nextFloat();
      expect(f).toBeGreaterThanOrEqual(0.0);
      expect(f).toBeLessThan(1.0);
    }
  });

  it("Xorshift64: always in [0, 1)", () => {
    const g = new Xorshift64(7n);
    for (let i = 0; i < 1000; i++) {
      const f = g.nextFloat();
      expect(f).toBeGreaterThanOrEqual(0.0);
      expect(f).toBeLessThan(1.0);
    }
  });

  it("PCG32: always in [0, 1)", () => {
    const g = new PCG32(7n);
    for (let i = 0; i < 1000; i++) {
      const f = g.nextFloat();
      expect(f).toBeGreaterThanOrEqual(0.0);
      expect(f).toBeLessThan(1.0);
    }
  });
});

// ── Integer range bounds ───────────────────────────────────────────────────────

describe("nextIntInRange bounds", () => {
  it("LCG: die roll in [1,6]", () => {
    const g = new LCG(999n);
    for (let i = 0; i < 1000; i++) {
      const v = g.nextIntInRange(1, 6);
      expect(v).toBeGreaterThanOrEqual(1);
      expect(v).toBeLessThanOrEqual(6);
    }
  });

  it("Xorshift64: die roll in [1,6]", () => {
    const g = new Xorshift64(999n);
    for (let i = 0; i < 1000; i++) {
      const v = g.nextIntInRange(1, 6);
      expect(v).toBeGreaterThanOrEqual(1);
      expect(v).toBeLessThanOrEqual(6);
    }
  });

  it("PCG32: die roll in [1,6]", () => {
    const g = new PCG32(999n);
    for (let i = 0; i < 1000; i++) {
      const v = g.nextIntInRange(1, 6);
      expect(v).toBeGreaterThanOrEqual(1);
      expect(v).toBeLessThanOrEqual(6);
    }
  });

  it("LCG: single-value range always returns 42", () => {
    const g = new LCG(5n);
    for (let i = 0; i < 20; i++) expect(g.nextIntInRange(42, 42)).toBe(42);
  });

  it("Xorshift64: single-value range always returns 42", () => {
    const g = new Xorshift64(5n);
    for (let i = 0; i < 20; i++) expect(g.nextIntInRange(42, 42)).toBe(42);
  });

  it("PCG32: single-value range always returns 42", () => {
    const g = new PCG32(5n);
    for (let i = 0; i < 20; i++) expect(g.nextIntInRange(42, 42)).toBe(42);
  });

  it("LCG: negative range [-10,-1]", () => {
    const g = new LCG(11n);
    for (let i = 0; i < 500; i++) {
      const v = g.nextIntInRange(-10, -1);
      expect(v).toBeGreaterThanOrEqual(-10);
      expect(v).toBeLessThanOrEqual(-1);
    }
  });
});

// ── Distribution ──────────────────────────────────────────────────────────────
//
// 12 000 die rolls — each face must appear ~2000 ± 30% times.

function checkDistribution(counts: number[], label: string): void {
  for (let i = 0; i < counts.length; i++) {
    expect(counts[i], `${label}: face ${i + 1}`).toBeGreaterThanOrEqual(1400);
    expect(counts[i], `${label}: face ${i + 1}`).toBeLessThanOrEqual(2600);
  }
}

describe("distribution (12 000 die rolls, each face ±30%)", () => {
  it("LCG", () => {
    const g = new LCG(123n);
    const counts = [0, 0, 0, 0, 0, 0];
    for (let i = 0; i < 12_000; i++) counts[g.nextIntInRange(1, 6) - 1]++;
    checkDistribution(counts, "LCG");
  });

  it("Xorshift64", () => {
    const g = new Xorshift64(123n);
    const counts = [0, 0, 0, 0, 0, 0];
    for (let i = 0; i < 12_000; i++) counts[g.nextIntInRange(1, 6) - 1]++;
    checkDistribution(counts, "Xorshift64");
  });

  it("PCG32", () => {
    const g = new PCG32(123n);
    const counts = [0, 0, 0, 0, 0, 0];
    for (let i = 0; i < 12_000; i++) counts[g.nextIntInRange(1, 6) - 1]++;
    checkDistribution(counts, "PCG32");
  });
});

// ── nextU64 composition ────────────────────────────────────────────────────────
//
// nextU64 must equal (hi << 32n) | lo from two consecutive nextU32 calls.

describe("nextU64 composition", () => {
  it("LCG", () => {
    const gU64 = new LCG(55n);
    const gU32 = new LCG(55n);
    for (let i = 0; i < 50; i++) {
      const u64 = gU64.nextU64();
      const hi = BigInt(gU32.nextU32());
      const lo = BigInt(gU32.nextU32());
      expect(u64).toBe((hi << 32n) | lo);
    }
  });

  it("Xorshift64", () => {
    const gU64 = new Xorshift64(55n);
    const gU32 = new Xorshift64(55n);
    for (let i = 0; i < 50; i++) {
      const u64 = gU64.nextU64();
      const hi = BigInt(gU32.nextU32());
      const lo = BigInt(gU32.nextU32());
      expect(u64).toBe((hi << 32n) | lo);
    }
  });

  it("PCG32", () => {
    const gU64 = new PCG32(55n);
    const gU32 = new PCG32(55n);
    for (let i = 0; i < 50; i++) {
      const u64 = gU64.nextU64();
      const hi = BigInt(gU32.nextU32());
      const lo = BigInt(gU32.nextU32());
      expect(u64).toBe((hi << 32n) | lo);
    }
  });
});

// ── nextU32 output range ──────────────────────────────────────────────────────

describe("nextU32 output fits in 32 bits", () => {
  it("LCG", () => {
    const g = new LCG(77n);
    for (let i = 0; i < 200; i++) {
      const v = g.nextU32();
      expect(v >>> 0).toBe(v); // >>> 0 is identity on valid uint32
    }
  });

  it("Xorshift64", () => {
    const g = new Xorshift64(77n);
    for (let i = 0; i < 200; i++) {
      const v = g.nextU32();
      expect(v >>> 0).toBe(v);
    }
  });

  it("PCG32", () => {
    const g = new PCG32(77n);
    for (let i = 0; i < 200; i++) {
      const v = g.nextU32();
      expect(v >>> 0).toBe(v);
    }
  });
});
