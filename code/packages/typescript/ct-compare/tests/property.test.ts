/**
 * Property tests for the ct-compare lane (VER31 F5, VER05a engine).
 *
 * The Warrant corpus already checks this lane on fixed finite domains. These
 * properties add *generated* inputs of every length up to 512 bytes, checked
 * against an obviously correct oracle: the plain early-exit comparison a
 * constant-time function exists to replace. Each property runs with a fixed
 * seed, so a failure reproduces exactly; `@coding-adventures/pbt` shrinks the
 * counterexample before reporting it.
 *
 * | Property            | Oracle                                         |
 * |---------------------|------------------------------------------------|
 * | ctEq agrees         | same length and every byte equal               |
 * | ctEq reflexive      | a copy of `a` always equals `a`                |
 * | ctEq symmetric      | ctEq(a, b) === ctEq(b, a)                      |
 * | ctSelectBytes picks | true -> a fresh copy of a; false -> of b       |
 * | ctEqU64 agrees      | BigInt equality after reduction to u64         |
 */
import { describe, expect, it } from "vitest";
import { checkProperty, genBool, genU64, type SplitMix64 } from "@coding-adventures/pbt";
import { ctEq, ctEqU64, ctSelectBytes } from "../src/index";

const CASES = { numTests: 2048 };
// Generous per-test timeout: v8 coverage instrumentation slows generation.
const TIMEOUT_MS = 120_000;

function oracleEq(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
  return true;
}

function bytes(rng: SplitMix64, maxLen: number): Uint8Array {
  const out = new Uint8Array(rng.bounded(maxLen + 1));
  for (let i = 0; i < out.length; i += 1) out[i] = rng.bounded(256);
  return out;
}

/** Two byte strings that are often equal or differ in exactly one byte. */
function nearPair(rng: SplitMix64): [Uint8Array, Uint8Array] {
  const a = bytes(rng, 512);
  const mode = rng.bounded(4);
  if (mode === 0) return [a, bytes(rng, 512)];
  const b = a.slice();
  if (mode >= 2 && b.length > 0) {
    const at = rng.bounded(b.length);
    b[at] ^= 1 << rng.bounded(8);
  }
  return [a, b];
}

describe("ct-compare properties (VER05a)", () => {
  it("ctEq agrees with the early-exit oracle", () => {
    const verdict = checkProperty(0x43_54_45_51n, CASES, nearPair, ([a, b]) => ctEq(a, b) === oracleEq(a, b));
    expect(verdict).toEqual({ status: "pass" });
  }, TIMEOUT_MS);

  it("ctEq is reflexive on copies and symmetric", () => {
    const verdict = checkProperty(
      0x43_54_52_46n,
      CASES,
      nearPair,
      ([a, b]) => ctEq(a, a.slice()) && ctEq(a, b) === ctEq(b, a),
    );
    expect(verdict).toEqual({ status: "pass" });
  }, TIMEOUT_MS);

  it("ctSelectBytes returns a fresh copy of the chosen side", () => {
    const verdict = checkProperty(
      0x43_54_53_4cn,
      CASES,
      (rng) => {
        const a = bytes(rng, 512);
        const b = new Uint8Array(a.length).map(() => rng.bounded(256));
        return { a, b, choice: genBool(rng) };
      },
      ({ a, b, choice }) => {
        const out = ctSelectBytes(a, b, choice);
        const want = choice ? a : b;
        return out !== want && oracleEq(out, want);
      },
    );
    expect(verdict).toEqual({ status: "pass" });
  }, TIMEOUT_MS);

  it("ctEqU64 agrees with bigint equality", () => {
    const verdict = checkProperty(
      0x43_54_55_36n,
      CASES,
      (rng) => {
        const a = genU64(rng);
        return [a, genBool(rng) ? a : genU64(rng)] as const;
      },
      ([a, b]) => ctEqU64(a, b) === (a === b),
    );
    expect(verdict).toEqual({ status: "pass" });
  }, TIMEOUT_MS);
});
