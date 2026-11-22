import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { ComplexArray } from "@coding-adventures/dsp-complex";
import {
  checkProperty,
  CoverageBitmap,
  fuzzTarget,
  SplitMix64,
  type FuzzOutcome,
} from "@coding-adventures/test-bootstrap";
import {
  buildWindow,
  fft,
  ifft,
  InvalidInputError,
  irfft,
  rfft,
  WindowType,
} from "../src/index.js";

function sampleFloat(rng: SplitMix64, scale = 5): number {
  const u = rng.bounded(2_000_001) - 1_000_000;
  return (u / 1_000_000) * scale;
}

function sampleComplexArray(rng: SplitMix64, len: number): ComplexArray {
  const re = new Float64Array(len);
  const im = new Float64Array(len);
  for (let i = 0; i < len; i++) {
    re[i] = sampleFloat(rng);
    im[i] = sampleFloat(rng);
  }
  return ComplexArray.fromRealImag(re, im);
}

describe("Property-Based Tests (SplitMix64) & Fuzzing for dsp-fft", () => {
  it("PBT: FFT/IFFT round-trip across Radix-2 and Bluestein lengths", () => {
    const lengths = [1, 2, 3, 4, 5, 7, 8, 11, 16, 23, 32, 45, 64];
    const verdict = checkProperty(
      0xff70_1234n,
      { numTests: 65 },
      (rng) => {
        const len = lengths[rng.bounded(lengths.length)];
        return sampleComplexArray(rng, len);
      },
      (signal) => {
        const spectrum = fft(signal);
        const recovered = ifft(spectrum);
        return recovered.equals(signal, 1e-9);
      },
    );
    assert.equal(verdict.status, "pass");
  });

  it("PBT: Parseval's theorem holds for Radix-2 and Bluestein FFTs", () => {
    const lengths = [1, 2, 3, 4, 6, 8, 13, 16, 29, 32, 50];
    const verdict = checkProperty(
      0xff70_5678n,
      { numTests: 55 },
      (rng) => {
        const len = lengths[rng.bounded(lengths.length)];
        return sampleComplexArray(rng, len);
      },
      (signal) => {
        const n = signal.length;
        const spectrum = fft(signal);
        let timeEnergy = 0;
        let freqEnergy = 0;
        for (let i = 0; i < n; i++) {
          timeEnergy += signal.get(i).absSq();
          freqEnergy += spectrum.get(i).absSq();
        }
        return Math.abs(timeEnergy - freqEnergy / n) <= 1e-8 * Math.max(1, timeEnergy);
      },
    );
    assert.equal(verdict.status, "pass");
  });

  it("PBT: FFT linearity: fft(a*x + b*y) == a*fft(x) + b*fft(y)", () => {
    const rng = new SplitMix64(0xff70_9999n);
    for (const len of [4, 7, 16, 19]) {
      for (let trial = 0; trial < 8; trial++) {
        const x = sampleComplexArray(rng, len);
        const y = sampleComplexArray(rng, len);
        const a = sampleFloat(rng, 3);
        const b = sampleFloat(rng, 3);
        const lhs = fft(x.scale(a).add(y.scale(b)));
        const rhs = fft(x).scale(a).add(fft(y).scale(b));
        assert.ok(lhs.equals(rhs, 1e-8));
      }
    }
  });

  it("PBT: rfft / irfft real round-trip and Parseval symmetry", () => {
    const lengths = [1, 2, 3, 4, 5, 8, 12, 17, 32, 41];
    const verdict = checkProperty(
      0xff70_abcden,
      { numTests: 50 },
      (rng) => {
        const len = lengths[rng.bounded(lengths.length)];
        const arr = new Float64Array(len);
        for (let i = 0; i < len; i++) {
          arr[i] = sampleFloat(rng);
        }
        return arr;
      },
      (signal) => {
        const half = rfft(signal);
        const recovered = irfft(half, signal.length);
        if (recovered.length !== signal.length) return false;
        for (let i = 0; i < signal.length; i++) {
          if (Math.abs(recovered[i] - signal[i]) > 1e-9) return false;
        }
        return true;
      },
    );
    assert.equal(verdict.status, "pass");
  });

  it("PBT: Window symmetry and bounded [0, 1] range", () => {
    const rng = new SplitMix64(0xff70_2468n);
    const types = [
      WindowType.RECTANGULAR,
      WindowType.HANN,
      WindowType.HAMMING,
      WindowType.BLACKMAN,
      WindowType.KAISER,
    ];
    for (const wType of types) {
      for (let trial = 0; trial < 10; trial++) {
        const n = 2 + rng.bounded(63);
        const w = buildWindow(wType, n, { beta: 1 + rng.bounded(8), symmetric: true });
        assert.equal(w.length, n);
        for (let i = 0; i < n; i++) {
          assert.ok(w[i] >= -1e-12 && w[i] <= 1.0 + 1e-12);
          assert.ok(Math.abs(w[i] - w[n - 1 - i]) <= 1e-12);
        }
      }
    }
  });

  it("PBT: CWE-20/190/400/1321 fuzzing of FFT APIs fails closed cleanly", () => {
    const report = fuzzTarget(
      [new Uint8Array([1, 2, 3, 4]), new Uint8Array([255, 254, 128, 0])],
      { maxIterations: 150, seed: 0xff70_f002n, maxInputLen: 32 },
      (input: Uint8Array, cov: CoverageBitmap): FuzzOutcome => {
        const b0 = input[0] ?? 0;
        const b1 = input[1] ?? 1;
        const probe =
          b0 === 255
            ? 1e100
            : b0 === 254
              ? Number.NaN
              : b0 === 253
                ? Number.POSITIVE_INFINITY
                : b0 === 252
                  ? -4
                  : (b0 % 16) + 1;

        try {
          cov.recordEdge(1);
          const arr = Array.from({ length: Math.max(1, b1 % 12) }, (_, i) => ((input[i % input.length] ?? i) - 128) / 32);
          const spec = fft(arr);
          const rec = ifft(spec);
          assert.equal(rec.length, arr.length);
          buildWindow("hann", probe);
          cov.recordEdge(2);
        } catch (err) {
          if (err instanceof InvalidInputError) {
            cov.recordEdge(3);
            return { status: "pass" };
          }
          return {
            status: "crash",
            kind: err instanceof Error ? err.name : "UnknownError",
            frames: ["fuzzFft"],
          };
        }

        const hostileValues: unknown[] = [
          Symbol("hostile"),
          Object.create(null),
          new DataView(new ArrayBuffer(8)),
          JSON.parse('{"__proto__":{"pollutedFft":true}}'),
        ];
        for (const hostile of hostileValues) {
          try {
            fft(hostile as number[]);
            return { status: "crash", kind: "AcceptedHostileFft", frames: ["fft"] };
          } catch (err) {
            if (!(err instanceof InvalidInputError)) {
              return { status: "crash", kind: "UnexpectedError", frames: ["fft"] };
            }
          }
        }
        assert.equal(({} as { pollutedFft?: boolean }).pollutedFft, undefined);
        cov.recordEdge(4);
        return { status: "pass" };
      },
    );

    assert.equal(report.clean, true);
    assert.equal(report.crashes.length, 0);
  });
});
