/**
 * Coverage-guided fuzzing of the ct-compare lane (VER31 F5, VER06b engine).
 *
 * Every public function takes untrusted input, so the fuzz target exercises all
 * four with bytes the mutator chooses, and also feeds *malformed* inputs (plain
 * arrays holding values outside 0..255) to check that validation rejects them
 * with a RangeError instead of coercing. Coercion once made two different
 * inputs compare equal; this target would catch that regression.
 *
 * A "crash" is any disagreement with the oracle or any unexpected exception.
 */
import { describe, expect, it } from "vitest";
import { CoverageBitmap, fuzzTarget, type FuzzOutcome } from "@coding-adventures/fuzz";
import { ctEq, ctEqU64, ctSelectBytes } from "../src/index";

function oracleEq(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
  return true;
}

function crash(kind: string): FuzzOutcome {
  return { status: "crash", kind, frames: ["ct-compare fuzz target"] };
}

function target(input: Uint8Array, cov: CoverageBitmap): FuzzOutcome {
  if (input.length === 0) {
    cov.recordEdge(1);
    return ctEq(input, input) ? { status: "pass" } : crash("empty-unequal");
  }
  const split = input[0] % input.length;
  const a = input.subarray(1, 1 + split);
  const b = input.subarray(1 + split);
  cov.recordEdge(a.length === b.length ? 2 : 3);
  if (ctEq(a, b) !== oracleEq(a, b)) return crash("ctEq-disagrees");
  if (a.length === b.length) {
    cov.recordEdge(oracleEq(a, b) ? 4 : 5);
    const pick = (input[0] & 1) === 1;
    if (!oracleEq(ctSelectBytes(a, b, pick), pick ? a : b)) return crash("select-wrong");
  } else {
    try {
      ctSelectBytes(a, b, true);
      return crash("select-accepted-length-mismatch");
    } catch (error) {
      if (!(error instanceof RangeError)) return crash("select-wrong-error");
    }
  }
  if (input.length >= 16) {
    cov.recordEdge(6);
    const view = new DataView(input.buffer, input.byteOffset, input.byteLength);
    const x = view.getBigUint64(0);
    const y = view.getBigUint64(8);
    if (ctEqU64(x, y) !== (x === y) || !ctEqU64(x, x)) return crash("u64-disagrees");
  }
  // Malformed input: a plain array with an out-of-range element must be refused.
  const bad = Array.from(a, (v) => v * 2);
  if (bad.some((v) => v > 255)) {
    cov.recordEdge(7);
    try {
      ctEq(bad, bad);
      return crash("out-of-range-accepted");
    } catch (error) {
      if (!(error instanceof RangeError)) return crash("out-of-range-wrong-error");
    }
  }
  return { status: "pass" };
}

describe("ct-compare fuzzing (VER06b)", () => {
  it("finds no crash in 4096 coverage-guided iterations", () => {
    const corpus = [new Uint8Array([]), new Uint8Array([3, 1, 2, 3, 1, 2, 3]), new Uint8Array(33).fill(0xa5)];
    const report = fuzzTarget(corpus, { maxIterations: 4096, maxInputLen: 1024, seed: 0x43_54_46_5an }, target);
    expect(report.crashes).toEqual([]);
    expect(report.iterations).toBe(4096);
    expect(report.totalEdges).toBeGreaterThanOrEqual(5);
    // Generous: v8 coverage instrumentation slows the loop several-fold.
  }, 120_000);
});
