import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  blackmanWindow,
  buildWindow,
  hammingWindow,
  hannWindow,
  rectangularWindow,
  WindowType,
} from "../src/index.js";

describe("Analysis Windows (dsp-stft)", () => {
  it("generates correct symmetric Hann window", () => {
    const w = hannWindow(5);
    assert.equal(w.length, 5);
    assert.equal(w[0], 0.0);
    assert.equal(w[4], 0.0);
    assert.equal(w[2], 1.0);
    assert.ok(Math.abs(w[1] - 0.5) < 1e-12);
    assert.ok(Math.abs(w[3] - 0.5) < 1e-12);
  });

  it("generates correct symmetric Hamming window", () => {
    const w = hammingWindow(5);
    assert.equal(w.length, 5);
    assert.ok(Math.abs(w[0] - 0.08) < 1e-12);
    assert.ok(Math.abs(w[4] - 0.08) < 1e-12);
    assert.ok(Math.abs(w[2] - 1.0) < 1e-12);
  });

  it("generates correct symmetric Blackman window", () => {
    const w = blackmanWindow(7);
    assert.equal(w.length, 7);
    // Symmetry
    for (let i = 0; i < 7; i++) {
      assert.ok(Math.abs(w[i] - w[6 - i]) < 1e-12);
    }
  });

  it("generates rectangular window with all 1.0s", () => {
    const w = rectangularWindow(16);
    assert.equal(w.length, 16);
    for (let i = 0; i < 16; i++) {
      assert.equal(w[i], 1.0);
    }
  });

  it("handles length 1 window cleanly", () => {
    for (const type of [
      WindowType.HANN,
      WindowType.HAMMING,
      WindowType.BLACKMAN,
      WindowType.RECTANGULAR,
    ]) {
      const w = buildWindow(type, 1);
      assert.equal(w.length, 1);
      assert.equal(w[0], 1.0);
    }
  });
});
