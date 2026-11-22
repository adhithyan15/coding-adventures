import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  checkProperty,
  CoverageBitmap,
  fuzzTarget,
  SplitMix64,
  type FuzzOutcome,
} from "@coding-adventures/test-bootstrap";
import {
  dct,
  dct2d,
  DctError,
  DctNorm,
  DctType,
  dst,
  DstType,
  idct,
  idct2d,
  idst,
} from "../src/index.js";

function sampleFloat(rng: SplitMix64, scale = 5): number {
  const u = rng.bounded(2_000_001) - 1_000_000;
  return (u / 1_000_000) * scale;
}

function sampleSignal(rng: SplitMix64, len: number): Float64Array {
  const arr = new Float64Array(len);
  for (let i = 0; i < len; i++) {
    arr[i] = sampleFloat(rng);
  }
  return arr;
}

function energy(arr: Float64Array): number {
  let sum = 0;
  for (let i = 0; i < arr.length; i++) {
    sum += arr[i] * arr[i];
  }
  return sum;
}

describe("Property-Based Tests (SplitMix64) & Fuzzing for dsp-dct", () => {
  it("PBT: DCT-I..IV forward/inverse round-trip across power-of-2 and arbitrary lengths", () => {
    const lengths = [2, 3, 4, 5, 7, 8, 12, 16, 25, 32];
    const pairs: Array<[DctType, DctType]> = [
      [DctType.I, DctType.I],
      [DctType.II, DctType.III],
      [DctType.III, DctType.II],
      [DctType.IV, DctType.IV],
    ];
    const norms = [DctNorm.ORTHO, DctNorm.NONE];

    const verdict = checkProperty(
      0xdc70_1234n,
      { numTests: 80 },
      (rng) => ({
        signal: sampleSignal(rng, lengths[rng.bounded(lengths.length)]),
        pair: pairs[rng.bounded(pairs.length)],
        norm: norms[rng.bounded(norms.length)],
      }),
      ({ signal, pair, norm }) => {
        const coeffs = dct(signal, pair[0], norm);
        const recovered = idct(coeffs, pair[1], norm);
        if (recovered.length !== signal.length) return false;
        for (let i = 0; i < signal.length; i++) {
          if (Math.abs(recovered[i] - signal[i]) > 1e-8) return false;
        }
        return true;
      },
    );
    assert.equal(verdict.status, "pass");
  });

  it("PBT: Parseval's energy preservation for orthonormal DCT-I/II/IV and DST-I/II/IV + DST-I..IV round-trip", () => {
    const lengths = [2, 3, 4, 7, 8, 13, 16, 24];
    const rng = new SplitMix64(0xdc70_5678n);

    for (const dctType of [DctType.I, DctType.II, DctType.IV]) {
      for (let trial = 0; trial < 10; trial++) {
        const len = lengths[rng.bounded(lengths.length)];
        const sig = sampleSignal(rng, len);
        const coeffs = dct(sig, dctType, DctNorm.ORTHO);
        const eIn = energy(sig);
        const eOut = energy(coeffs);
        assert.ok(
          Math.abs(eIn - eOut) <= 1e-8 * Math.max(1, eIn),
          `DCT-${dctType} Parseval mismatch at N=${len}: ${eIn} vs ${eOut}`,
        );
      }
    }

    for (const dstType of [DstType.I, DstType.II, DstType.III, DstType.IV]) {
      for (let trial = 0; trial < 10; trial++) {
        const len = lengths[rng.bounded(lengths.length)];
        const sig = sampleSignal(rng, len);
        const coeffs = dst(sig, dstType, DctNorm.ORTHO);
        const recovered = idst(
          coeffs,
          dstType === DstType.II
            ? DstType.III
            : dstType === DstType.III
              ? DstType.II
              : dstType,
          DctNorm.ORTHO,
        );
        if (dstType !== DstType.III) {
          const eIn = energy(sig);
          const eOut = energy(coeffs);
          assert.ok(
            Math.abs(eIn - eOut) <= 1e-8 * Math.max(1, eIn),
            `DST-${dstType} Parseval mismatch at N=${len}: ${eIn} vs ${eOut}`,
          );
        }
        for (let i = 0; i < len; i++) {
          assert.ok(Math.abs(recovered[i] - sig[i]) <= 1e-8);
        }
      }
    }
  });

  it("PBT: DCT linearity: dct(a*x + b*y) == a*dct(x) + b*dct(y)", () => {
    const rng = new SplitMix64(0xdc70_9999n);
    for (const len of [4, 7, 16]) {
      for (let trial = 0; trial < 8; trial++) {
        const x = sampleSignal(rng, len);
        const y = sampleSignal(rng, len);
        const a = sampleFloat(rng, 3);
        const b = sampleFloat(rng, 3);
        const combined = new Float64Array(len);
        for (let i = 0; i < len; i++) {
          combined[i] = a * x[i] + b * y[i];
        }
        const lhs = dct(combined, DctType.II, DctNorm.ORTHO);
        const dx = dct(x, DctType.II, DctNorm.ORTHO);
        const dy = dct(y, DctType.II, DctNorm.ORTHO);
        for (let i = 0; i < len; i++) {
          assert.ok(Math.abs(lhs[i] - (a * dx[i] + b * dy[i])) <= 1e-8);
        }
      }
    }
  });

  it("PBT: 2-D DCT/IDCT round-trip and orthonormal Parseval invariant", () => {
    const dims = [
      [2, 2],
      [3, 5],
      [4, 8],
      [6, 7],
    ];
    const rng = new SplitMix64(0xdc70_2d2dn);
    for (const [h, w] of dims) {
      for (let trial = 0; trial < 6; trial++) {
        const img = sampleSignal(rng, h * w);
        const coeffs = dct2d(img, h, w, DctType.II, DctNorm.ORTHO);
        const recovered = idct2d(coeffs, h, w, DctType.III, DctNorm.ORTHO);
        const eIn = energy(img);
        const eOut = energy(coeffs);
        assert.ok(Math.abs(eIn - eOut) <= 1e-8 * Math.max(1, eIn));
        for (let i = 0; i < img.length; i++) {
          assert.ok(Math.abs(recovered[i] - img[i]) <= 1e-8);
        }
      }
    }
  });

  it("PBT: CWE-20/190/400/1321 fuzzing of DCT/DST/2D APIs fails closed cleanly", () => {
    const report = fuzzTarget(
      [new Uint8Array([1, 2, 3, 4]), new Uint8Array([255, 254, 128, 0])],
      { maxIterations: 150, seed: 0xdc70_f002n, maxInputLen: 32 },
      (input: Uint8Array, cov: CoverageBitmap): FuzzOutcome => {
        const b0 = input[0] ?? 0;
        const b1 = input[1] ?? 1;
        const nProbe =
          b0 === 255
            ? 1e100
            : b0 === 254
              ? Number.NaN
              : b0 === 253
                ? -1
                : (b0 % 8) + 1;

        try {
          cov.recordEdge(1);
          const arr = Array.from({ length: Math.max(1, b1 % 10) }, (_, i) => ((input[i % input.length] ?? i) - 128) / 32);
          const c = dct(arr, (b0 % 4) + 1, b1 % 2 === 0 ? DctNorm.ORTHO : DctNorm.NONE);
          assert.equal(c.length, arr.length);
          dct2d(arr, 1, nProbe);
          cov.recordEdge(2);
        } catch (err) {
          if (err instanceof DctError) {
            cov.recordEdge(3);
            return { status: "pass" };
          }
          return {
            status: "crash",
            kind: err instanceof Error ? err.name : "UnknownError",
            frames: ["fuzzDct"],
          };
        }

        const hostileValues: unknown[] = [
          Symbol("hostile"),
          Object.create(null),
          new DataView(new ArrayBuffer(8)),
          JSON.parse('{"__proto__":{"pollutedDct":true}}'),
        ];
        for (const hostile of hostileValues) {
          try {
            dct([1, 2], hostile as DctType);
            return { status: "crash", kind: "AcceptedHostileType", frames: ["dct"] };
          } catch (err) {
            if (!(err instanceof DctError)) {
              return { status: "crash", kind: "UnexpectedError", frames: ["dct"] };
            }
          }
        }
        assert.equal(({} as { pollutedDct?: boolean }).pollutedDct, undefined);
        cov.recordEdge(4);
        return { status: "pass" };
      },
    );

    assert.equal(report.clean, true);
    assert.equal(report.crashes.length, 0);
  });
});
