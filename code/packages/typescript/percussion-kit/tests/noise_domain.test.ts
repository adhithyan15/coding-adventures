// Pseudo-noise must stay finite for every sample index and seed.
//
// trig.sin returns NaN beyond 2^29 radians (PHY00a 3.4.1).  The noise phase
// (i + 1) * (seed * 0.1618 + 0.618) grows without bound, so it is reduced to
// [0, 1) cycles first (PHY00a 7).  These are the seeds and counts at which the
// unreduced phase used to fail.
import assert from "node:assert/strict";
import test from "node:test";

import { pseudoNoise } from "../src/noise.js";

const CASES: Array<[number, number]> = [
  [0, 1_000_000_000],
  [50_000, 12_345],
  [5_700_000, 89],
  [1_000_000_000_000, 89],
  [7, 4_294_967_295],
];
// The same values are asserted in the Python and Rust lanes.
const EXPECTED: number[] = [0.06019736691951039, -0.578681446131668, -0.08452169376437269, -0.22354608627210837, -0.9668349004277843];

test("pseudo-noise is finite and in range at the old failing seeds", () => {
  for (const [index, seed] of CASES) {
    const v = pseudoNoise(index, seed);
    assert.ok(Number.isFinite(v) && v >= -1 && v <= 1, `${index} ${seed} ${v}`);
  }
});

test("pseudo-noise over a long run never fails", () => {
  for (let i = 0; i < 200_000; i += 997) {
    assert.ok(Number.isFinite(pseudoNoise(i, 12_345)));
  }
});

test("cross-lane values", () => {
  assert.equal(EXPECTED.length, CASES.length);
  CASES.forEach(([index, seed], k) => assert.equal(pseudoNoise(index, seed), EXPECTED[k]));
});
