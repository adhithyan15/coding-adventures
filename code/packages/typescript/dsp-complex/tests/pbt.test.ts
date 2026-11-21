import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  checkProperty,
  CoverageBitmap,
  fuzzTarget,
  SplitMix64,
  type FuzzOutcome,
} from "@coding-adventures/test-bootstrap";
import { Complex, ComplexArray, InvalidInputError } from "../src/index.js";

function sampleFloat(rng: SplitMix64, scale = 10): number {
  const u = rng.bounded(2_000_001) - 1_000_000;
  return (u / 1_000_000) * scale;
}

function sampleNonZeroComplex(rng: SplitMix64): Complex {
  const c = new Complex(sampleFloat(rng), sampleFloat(rng));
  return c.absSq() < 1e-6 ? new Complex(1.5, -0.5) : c;
}

describe("Property-Based Tests (SplitMix64) & Fuzzing for dsp-complex", () => {
  it("PBT: Complex field axioms (commutativity, associativity, distributivity, inverse)", () => {
    const verdict = checkProperty(
      0xc0a1_1e71n,
      { numTests: 200 },
      (rng) => ({
        a: new Complex(sampleFloat(rng), sampleFloat(rng)),
        b: sampleNonZeroComplex(rng),
        c: new Complex(sampleFloat(rng), sampleFloat(rng)),
      }),
      ({ a, b, c }) => {
        const eps = 1e-9;
        // Commutativity
        if (!a.add(b).equals(b.add(a), eps)) return false;
        if (!a.mul(b).equals(b.mul(a), eps)) return false;
        // Associativity
        if (!a.add(b).add(c).equals(a.add(b.add(c)), eps)) return false;
        if (!a.mul(b).mul(c).equals(a.mul(b.mul(c)), 1e-7)) return false;
        // Distributivity
        if (!a.mul(b.add(c)).equals(a.mul(b).add(a.mul(c)), 1e-7)) return false;
        // Additive and multiplicative identities & inverses
        if (!a.add(Complex.zero()).equals(a, eps)) return false;
        if (!a.mul(Complex.one()).equals(a, eps)) return false;
        if (!a.sub(a).equals(Complex.zero(), eps)) return false;
        if (!a.mul(b).div(b).equals(a, 1e-7)) return false;
        return true;
      },
    );
    assert.equal(verdict.status, "pass");
  });

  it("PBT: Complex conjugation, norm multiplicativity, and exponential homomorphism", () => {
    const verdict = checkProperty(
      0xc0a1_2026n,
      { numTests: 200 },
      (rng) => ({
        a: new Complex(sampleFloat(rng, 2), sampleFloat(rng, Math.PI)),
        b: new Complex(sampleFloat(rng, 2), sampleFloat(rng, Math.PI)),
      }),
      ({ a, b }) => {
        const eps = 1e-9;
        // Double conjugation involution
        if (!a.conj().conj().equals(a, eps)) return false;
        // (a * b)* == a* * b*
        if (!a.mul(b).conj().equals(a.conj().mul(b.conj()), eps)) return false;
        // |a * b| == |a| * |b|
        if (Math.abs(a.mul(b).abs() - a.abs() * b.abs()) > 1e-8) return false;
        // exp(a + b) == exp(a) * exp(b)
        if (!a.add(b).exp().equals(a.exp().mul(b.exp()), 1e-7)) return false;
        return true;
      },
    );
    assert.equal(verdict.status, "pass");
  });

  it("PBT: ComplexArray element-wise axioms & scalar parity", () => {
    const rng = new SplitMix64(0xbeef_1234n);
    for (let trial = 0; trial < 50; trial++) {
      const len = 1 + rng.bounded(32);
      const reA = new Float64Array(len);
      const imA = new Float64Array(len);
      const reB = new Float64Array(len);
      const imB = new Float64Array(len);
      for (let i = 0; i < len; i++) {
        reA[i] = sampleFloat(rng, 5);
        imA[i] = sampleFloat(rng, 5);
        const br = sampleFloat(rng, 5);
        const bi = sampleFloat(rng, 5);
        if (br * br + bi * bi < 1e-4) {
          reB[i] = 1.0;
          imB[i] = 0.5;
        } else {
          reB[i] = br;
          imB[i] = bi;
        }
      }
      const arrA = ComplexArray.fromRealImag(reA, imA);
      const arrB = ComplexArray.fromRealImag(reB, imB);

      assert.ok(arrA.conjugate().conjugate().equals(arrA, 1e-12));
      assert.ok(arrA.add(arrB).equals(arrB.add(arrA), 1e-12));
      assert.ok(arrA.mul(arrB).div(arrB).equals(arrA, 1e-8));

      const k = sampleFloat(rng, 3);
      const scaled = arrA.scale(k);
      for (let i = 0; i < len; i++) {
        assert.ok(scaled.get(i).equals(arrA.get(i).scale(k), 1e-11));
      }
    }
  });

  it("PBT: CWE-20/190/400/1321 fuzzing of Complex & ComplexArray fails closed cleanly", () => {
    const report = fuzzTarget(
      [new Uint8Array([0, 1, 2, 3]), new Uint8Array([255, 254, 128, 7])],
      { maxIterations: 150, seed: 0xd5c0_0001n, maxInputLen: 32 },
      (input: Uint8Array, cov: CoverageBitmap): FuzzOutcome => {
        const b0 = input[0] ?? 0;
        const b1 = input[1] ?? 0;
        const probe =
          b0 === 255
            ? 1e100
            : b0 === 254
              ? Number.NaN
              : b0 === 253
                ? Number.POSITIVE_INFINITY
                : b0 === 252
                  ? -2
                  : b0 - 128;

        try {
          cov.recordEdge(1);
          const c = new Complex(probe, b1 - 64);
          assert.ok(c.equals(c.clone()));
          const len = Math.min(8, Math.max(0, b1 % 9));
          const arr = ComplexArray.zeros(len);
          assert.equal(arr.length, len);
          cov.recordEdge(2);
        } catch (err) {
          if (err instanceof InvalidInputError || err instanceof RangeError) {
            cov.recordEdge(3);
            return { status: "pass" };
          }
          return {
            status: "crash",
            kind: err instanceof Error ? err.name : "UnknownError",
            frames: ["fuzzComplex"],
          };
        }

        const hostileValues: unknown[] = [
          Symbol("bad"),
          Object.create(null),
          new DataView(new ArrayBuffer(8)),
          JSON.parse('{"__proto__":{"pollutedComplex":true}}'),
        ];
        for (const hostile of hostileValues) {
          try {
            new Complex(1, 2).equals(new Complex(1, 2), hostile as number);
            return { status: "crash", kind: "AcceptedHostileEps", frames: ["Complex.equals"] };
          } catch (err) {
            if (!(err instanceof InvalidInputError)) {
              return { status: "crash", kind: "UnexpectedError", frames: ["Complex.equals"] };
            }
          }
        }
        assert.equal(({} as { pollutedComplex?: boolean }).pollutedComplex, undefined);
        cov.recordEdge(4);
        return { status: "pass" };
      },
    );

    assert.equal(report.clean, true);
    assert.equal(report.crashes.length, 0);
  });
});
