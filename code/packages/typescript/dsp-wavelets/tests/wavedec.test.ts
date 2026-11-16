import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  InvalidCoefficientsError,
  Periodic,
  wavedec,
  waverec,
} from "../src/index.js";

describe("Multi-level 1-D wavedec and waverec", () => {
  it("wavedec decomposition structure", () => {
    const sig = new Float64Array(16);
    for (let i = 0; i < 16; i++) {
      sig[i] = i;
    }
    // 2 levels: returns [cA_2, cD_2, cD_1]
    const decomp = wavedec(sig, "haar", Periodic, 2);
    assert.equal(decomp.length, 3);
    assert.equal(decomp[0].length, 4); // cA_2
    assert.equal(decomp[1].length, 4); // cD_2
    assert.equal(decomp[2].length, 8); // cD_1
  });

  it("wavedec default level calculation", () => {
    const sig = new Float64Array(32);
    for (let i = 0; i < 32; i++) {
      sig[i] = i;
    }
    const decomp = wavedec(sig, "haar", Periodic);
    // Default level should be log2(32) = 5 -> [cA_5, cD_5, cD_4, cD_3, cD_2, cD_1]
    assert.equal(decomp.length, 6);
    assert.equal(decomp[0].length, 1);
  });

  it("waverec Haar roundtrip", () => {
    const sig = [1.0, 3.0, 5.0, 7.0, 2.0, 4.0, 6.0, 8.0];
    const decomp = wavedec(sig, "haar", Periodic, 2);
    const rec = waverec(decomp, "haar", Periodic);
    assert.equal(rec.length, sig.length);
    for (let i = 0; i < sig.length; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-5);
    }
  });

  it("waverec sym4 roundtrip", () => {
    const sig = new Float64Array(16);
    for (let i = 0; i < 16; i++) {
      sig[i] = Math.sin(i * 0.4);
    }
    const decomp = wavedec(sig, "sym4", Periodic, 2);
    const rec = waverec(decomp, "sym4", Periodic);
    assert.equal(rec.length, sig.length);
    for (let i = 0; i < sig.length; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-4);
    }
  });

  it("waverec rejects invalid coeffs structure", () => {
    assert.throws(() => waverec([[1.0, 2.0]]), InvalidCoefficientsError);
  });
});
