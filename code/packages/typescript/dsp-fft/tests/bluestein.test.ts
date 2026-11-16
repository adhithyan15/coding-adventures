import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  bluesteinScalar,
  Direction,
  fftScalar,
} from "../src/index.js";

describe("Bluestein arbitrary-length FFT", () => {
  it("handles N = 1 identity case", () => {
    const single = [3.5, -2.1];
    const outFwd = bluesteinScalar(single, Direction.FORWARD);
    assert.equal(outFwd.length, 2);
    assert.equal(outFwd[0], 3.5);
    assert.equal(outFwd[1], -2.1);

    const outInv = bluesteinScalar(single, Direction.INVERSE);
    assert.equal(outInv.length, 2);
    assert.equal(outInv[0], 3.5);
    assert.equal(outInv[1], -2.1);
  });

  it("matches radix-2 FFT on power-of-two N = 8", () => {
    const n = 8;
    const data: number[] = [];
    for (let i = 0; i < n; i++) {
      data.push(Math.sin(i));
      data.push(Math.cos(i));
    }

    const r2 = fftScalar(data);
    const blue = bluesteinScalar(data, Direction.FORWARD);

    for (let i = 0; i < 2 * n; i++) {
      assert.ok(
        Math.abs(r2[i] - blue[i]) < 1e-11,
        `mismatch at ${i}: radix2 ${r2[i]} vs bluestein ${blue[i]}`
      );
    }
  });

  it("performs forward-then-inverse roundtrip on prime length N = 7", () => {
    const n = 7;
    const signal: number[] = [];
    for (let i = 0; i < n; i++) {
      signal.push(i + 1);
      signal.push(-0.5 * i);
    }

    const spectrum = bluesteinScalar(signal, Direction.FORWARD);
    const recovered = bluesteinScalar(spectrum, Direction.INVERSE);

    for (let i = 0; i < 2 * n; i++) {
      assert.ok(
        Math.abs(recovered[i] - signal[i]) < 1e-11,
        `mismatch at ${i}: expected ${signal[i]}, got ${recovered[i]}`
      );
    }
  });

  it("performs forward-then-inverse roundtrip on arbitrary length N = 13", () => {
    const n = 13;
    const signal: number[] = [];
    for (let i = 0; i < n; i++) {
      signal.push(Math.cos((2 * Math.PI * i) / n));
      signal.push(Math.sin((4 * Math.PI * i) / n));
    }

    const spectrum = bluesteinScalar(signal, Direction.FORWARD);
    const recovered = bluesteinScalar(spectrum, Direction.INVERSE);

    for (let i = 0; i < 2 * n; i++) {
      assert.ok(Math.abs(recovered[i] - signal[i]) < 1e-11);
    }
  });
});
