import assert from "node:assert/strict";
import { describe, it } from "node:test";
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

class SimpleRng {
  private state: bigint;
  constructor(seed: bigint = 0x123456789abcdef0n) {
    this.state = seed;
  }
  nextU64(): bigint {
    this.state = (this.state + 0x9e3779b97f4a7c15n) & 0xffffffffffffffffn;
    let z = this.state;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & 0xffffffffffffffffn;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & 0xffffffffffffffffn;
    return (z ^ (z >> 31n)) & 0xffffffffffffffffn;
  }
  nextFloat(): number {
    return Number(this.nextU64() >> 11n) * (1.0 / 9007199254740992.0); // 1 << 53
  }
  nextF64(): number {
    return this.nextFloat() * 20.0 - 10.0;
  }
  array(len: number): Float64Array {
    const arr = new Float64Array(len);
    for (let i = 0; i < len; i++) {
      arr[i] = this.nextF64();
    }
    return arr;
  }
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
    const rng = new SimpleRng(0xcafe1234n);
    for (let trial = 0; trial < 25; trial++) {
      const n = 16;
      const sig = rng.array(n);
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
    const rng = new SimpleRng(0x5678abcdn);
    for (let trial = 0; trial < 25; trial++) {
      const n = 32;
      const sig = rng.array(n);
      for (const wName of ["haar", "db2", "db4"]) {
        const [ca, cd] = dwt(sig, wName, Periodic);
        const rec = idwt(ca, cd, wName, Periodic, n);
        assertClose(sig, rec, 1e-4);
      }
    }
  });

  it("verifies dwt1d multilevel roundtrips", () => {
    const rng = new SimpleRng(0x9abcdef0n);
    for (let trial = 0; trial < 15; trial++) {
      const n = 32;
      const sig = rng.array(n);
      const coeffs = dwt1d(sig, "db2", 3, Periodic);
      const rec = idwt1d(coeffs, "db2", 3, Periodic, n);
      assertClose(sig, rec, 1e-4);
    }
  });

  it("verifies wavedec / waverec roundtrips", () => {
    const rng = new SimpleRng(0xfeedfacen);
    for (let trial = 0; trial < 15; trial++) {
      const n = 32;
      const sig = rng.array(n);
      const decomp = wavedec(sig, "haar", Periodic, 3);
      const rec = waverec(decomp, "haar", Periodic);
      assertClose(sig, rec, 1e-4);
    }
  });

  it("verifies swt / iswt roundtrips", () => {
    const rng = new SimpleRng(0xdeadbeefn);
    for (let trial = 0; trial < 15; trial++) {
      const n = 16;
      const sig = rng.array(n);
      const coeffs = swt(sig, "haar", 2, Periodic);
      const rec = iswt(coeffs, "haar", Periodic);
      assertClose(sig, rec, 1e-4);
    }
  });

  it("verifies 2D DWT roundtrip", () => {
    const rng = new SimpleRng(0x1337c0den);
    for (let trial = 0; trial < 10; trial++) {
      const rows = 8;
      const cols = 8;
      const image = rng.array(rows * cols);
      const coeffs = dwt2d(image, rows, cols, "haar", 2, Periodic);
      const rec = idwt2d(coeffs, rows, cols, "haar", 2, Periodic);
      assertClose(image, rec, 1e-4);
    }
  });
});
