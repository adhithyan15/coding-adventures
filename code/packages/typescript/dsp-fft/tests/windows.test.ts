import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  besselI0,
  blackman,
  buildWindow,
  hamming,
  hann,
  InvalidInputError,
  kaiser,
  rectangular,
  WindowType,
} from "../src/index.js";

describe("Window Functions", () => {
  it("computes modified Bessel function of first kind I0(x)", () => {
    // I0(0) = 1
    assert.equal(besselI0(0), 1.0);
    // I0(1) ≈ 1.2660658777520083
    assert.ok(Math.abs(besselI0(1) - 1.2660658777520083) < 1e-12);
    // I0(5) ≈ 27.23987182360445
    assert.ok(Math.abs(besselI0(5) - 27.23987182360445) < 1e-10);
    assert.throws(() => besselI0(NaN), InvalidInputError);
  });

  it("handles N = 1 window of any type", () => {
    assert.deepEqual(Array.from(rectangular(1)), [1.0]);
    assert.deepEqual(Array.from(hann(1)), [1.0]);
    assert.deepEqual(Array.from(hamming(1)), [1.0]);
    assert.deepEqual(Array.from(blackman(1)), [1.0]);
    assert.deepEqual(Array.from(kaiser(1)), [1.0]);
  });

  it("produces symmetric Hann window with zero endpoints", () => {
    const w = hann(5);
    assert.equal(w.length, 5);
    assert.equal(w[0], 0.0);
    assert.equal(w[4], 0.0);
    assert.equal(w[2], 1.0); // Center peak
    assert.ok(Math.abs(w[1] - w[3]) < 1e-12); // Symmetry
  });

  it("produces symmetric Hamming window with 0.08 endpoints", () => {
    const w = hamming(5);
    assert.equal(w.length, 5);
    assert.ok(Math.abs(w[0] - 0.08) < 1e-12);
    assert.ok(Math.abs(w[4] - 0.08) < 1e-12);
    assert.equal(w[2], 1.0);
    assert.ok(Math.abs(w[1] - w[3]) < 1e-12);
  });

  it("produces symmetric Blackman window", () => {
    const w = blackman(5);
    assert.equal(w.length, 5);
    assert.ok(Math.abs(w[0]) < 1e-12);
    assert.ok(Math.abs(w[4]) < 1e-12);
    assert.ok(Math.abs(w[2] - 1.0) < 1e-12);
    assert.ok(Math.abs(w[1] - w[3]) < 1e-12);
  });

  it("produces symmetric Kaiser window with center peak = 1.0", () => {
    const w = kaiser(9, 6.0);
    assert.equal(w.length, 9);
    assert.ok(Math.abs(w[4] - 1.0) < 1e-12);
    for (let i = 0; i < 4; i++) {
      assert.ok(Math.abs(w[i] - w[8 - i]) < 1e-12);
      assert.ok(w[i] < w[i + 1]);
    }
  });

  it("supports buildWindow with string names and options", () => {
    const w1 = buildWindow("hanning", 8);
    const w2 = hann(8);
    assert.deepEqual(Array.from(w1), Array.from(w2));

    const wRec = buildWindow(WindowType.RECTANGULAR, 4);
    assert.deepEqual(Array.from(wRec), [1, 1, 1, 1]);

    const periodic = hann(8, false);
    assert.equal(periodic.length, 8);
  });

  it("rejects invalid window inputs fail-closed", () => {
    assert.throws(() => buildWindow("nonexistent", 8), InvalidInputError);
    assert.throws(() => hann(0), InvalidInputError);
    assert.throws(() => hann(-5), InvalidInputError);
    assert.throws(() => hann(1.5), InvalidInputError);
    assert.throws(() => kaiser(8, -1), InvalidInputError);
    assert.throws(() => kaiser(8, NaN), InvalidInputError);
  });
});
