/**
 * Security regression tests: hostile headers must not drive work or memory.
 *
 * Each case is a few bytes of input whose header claims an enormous count or
 * length.  An implementation that sized an allocation or loop from the claim
 * would exhaust memory or run for minutes; these tests assert bounded time
 * and sane results, plus round trips over every input of length <= 2 and a
 * seeded random sample.
 */

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
import { compress, decompress } from "../src/lz78.js";

const LIMIT_MS = 10_000;

function quick<T>(fn: () => T): T {
  const start = Date.now();
  const out = fn();
  assert.ok(Date.now() - start < LIMIT_MS, "hostile input took too long");
  return out;
}

const bytes = (...b: number[]) => new Uint8Array(b);
const text = (u: Uint8Array) => new TextDecoder().decode(u);
const u32 = (n: number) => [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255];

// Small deterministic PRNG (mulberry32).
function rng(seed: number) {
  let a = seed;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

describe("lz78 hostile headers", () => {
  const hdr = (len: number, count: number) => [...u32(len), ...u32(count)];

  it("max header only", () => {
    assert.equal(quick(() => decompress(new Uint8Array(8).fill(255))).length, 0);
  });

  it("max length, zero tokens", () => {
    assert.equal(quick(() => decompress(bytes(...hdr(0xffffffff, 0)))).length, 0);
  });

  it("count larger than remaining bytes yields the present tokens", () => {
    const d = bytes(...hdr(2, 0xffffffff), 0, 0, 104, 0, 0, 0, 105, 0);
    assert.equal(text(quick(() => decompress(d))), "hi");
  });

  it("length larger than the tokens supply", () => {
    const d = bytes(...hdr(0xffffffff, 1), 0, 0, 104, 0);
    assert.equal(text(quick(() => decompress(d))), "h");
  });

  it("dangling dict index does not throw", () => {
    const d = bytes(...hdr(10, 2), 0, 0, 97, 0, 0x27, 0x0f, 98, 0);
    assert.equal(text(quick(() => decompress(d))), "a");
  });

  it("chain tokens stop at the declared length", () => {
    const n = 20000;
    const d = new Uint8Array(8 + n * 4);
    d.set(hdr(1000, n));
    for (let i = 0; i < n; i++) d.set([i >> 8, i & 255, 97, 0], 8 + i * 4);
    assert.equal(quick(() => decompress(d)).length, 1000);
  });
});

describe("lz78 round trips", () => {
  it("round-trips every input of length <= 2", () => {
    const inputs: Uint8Array[] = [bytes()];
    for (let a = 0; a < 256; a++) {
      inputs.push(bytes(a));
      for (let b = 0; b < 256; b++) inputs.push(bytes(a, b));
    }
    for (const d of inputs) {
      assert.deepEqual(decompress(compress(d)), d);
    }
  });

  it("round-trips a seeded random sample", () => {
    const next = rng(0x5eed);
    const alphabets = [2, 4, 16, 256];
    for (let i = 0; i < 300; i++) {
      const len = 3 + Math.floor(next() * 597);
      const d = new Uint8Array(len);
      for (let j = 0; j < len; j++) d[j] = Math.floor(next() * alphabets[i % 4]);
      assert.deepEqual(decompress(compress(d)), d);
    }
  });
});
