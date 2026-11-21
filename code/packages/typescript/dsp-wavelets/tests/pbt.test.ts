import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";
import {
  dwt,
  dwt1d,
  dwt2d,
  idwt,
  idwt1d,
  idwt2d,
  iswt,
  Periodic,
  swt,
  wavedec,
  waverec,
} from "../src/index.js";

function randomArray(rng: SplitMix64, len: number): Float64Array {
  const arr = new Float64Array(len);
  for (let i = 0; i < len; i++) {
    const unit = Number(rng.nextU64() >> 11n) * (1.0 / 9007199254740992.0);
    arr[i] = unit * 20.0 - 10.0;
  }
  return arr;
}

function assertClose(a: ArrayLike<number>, b: ArrayLike<number>, tol = 1e-3): void {
  assert.equal(a.length, b.length, `length mismatch: ${a.length} vs ${b.length}`);
  for (let i = 0; i < a.length; i++) {
    const x = Number(a[i]);
    const y = Number(b[i]);
    const scale = Math.max(Math.abs(x), Math.abs(y), 1.0);
    assert.ok(
      Math.abs(x - y) <= scale * tol,
      `mismatch at ${i}: ${x} vs ${y} (tol ${tol})`
    );
  }
}

describe("Property-based testing for dsp-wavelets", () => {
  it("verifies Haar energy conservation", () => {
    const rng = new SplitMix64(0xcafe1234n);
    for (let trial = 0; trial < 25; trial++) {
      const n = 16;
      const sig = randomArray(rng, n);
      const [ca, cd] = dwt(sig, "haar", Periodic);

      let energySig = 0;
      for (let i = 0; i < n; i++) {
        energySig += sig[i] * sig[i];
      }

      let energyCoeffs = 0;
      for (let i = 0; i < ca.length; i++) {
        energyCoeffs += ca[i] * ca[i];
      }
      for (let i = 0; i < cd.length; i++) {
        energyCoeffs += cd[i] * cd[i];
      }

      assert.ok(Math.abs(energySig - energyCoeffs) / energySig < 1e-4);
    }
  });

  it("verifies dwt / idwt roundtrips across wavelets", () => {
    const rng = new SplitMix64(0x5678abcdn);
    for (let trial = 0; trial < 25; trial++) {
      const n = 32;
      const sig = randomArray(rng, n);
      for (const wName of ["haar", "db2", "db4"]) {
        const [ca, cd] = dwt(sig, wName, Periodic);
        const rec = idwt(ca, cd, wName, Periodic, n);
        assertClose(sig, rec, 1e-4);
      }
    }
  });

  it("verifies dwt1d multilevel roundtrips", () => {
    const rng = new SplitMix64(0x9abcdef0n);
    for (let trial = 0; trial < 15; trial++) {
      const n = 32;
      const sig = randomArray(rng, n);
      const coeffs = dwt1d(sig, "db2", 3, Periodic);
      const rec = idwt1d(coeffs, "db2", 3, Periodic, n);
      assertClose(sig, rec, 1e-4);
    }
  });

  it("verifies wavedec / waverec roundtrips", () => {
    const rng = new SplitMix64(0xfeedfacen);
    for (let trial = 0; trial < 15; trial++) {
      const n = 32;
      const sig = randomArray(rng, n);
      const decomp = wavedec(sig, "haar", Periodic, 3);
      const rec = waverec(decomp, "haar", Periodic);
      assertClose(sig, rec, 1e-4);
    }
  });

  it("verifies swt / iswt roundtrips", () => {
    const rng = new SplitMix64(0xdeadbeefn);
    for (let trial = 0; trial < 15; trial++) {
      const n = 16;
      const sig = randomArray(rng, n);
      const coeffs = swt(sig, "haar", 2, Periodic);
      const rec = iswt(coeffs, "haar", Periodic);
      assertClose(sig, rec, 1e-4);
    }
  });

  it("verifies 2D DWT roundtrip", () => {
    const rng = new SplitMix64(0x1337c0den);
    for (let trial = 0; trial < 10; trial++) {
      const rows = 8;
      const cols = 8;
      const image = randomArray(rng, rows * cols);
      const coeffs = dwt2d(image, rows, cols, "haar", 2, Periodic);
      const rec = idwt2d(coeffs, rows, cols, "haar", 2, Periodic);
      assertClose(image, rec, 1e-4);
    }
  });
});
