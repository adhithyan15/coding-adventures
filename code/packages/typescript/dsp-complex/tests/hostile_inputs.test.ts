import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  Complex,
  ComplexArray,
  InvalidInputError,
  LengthMismatchError,
  MAX_SAMPLES,
  OddInterleavedLengthError,
} from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Hostile inputs & VER15 Security Hardening", () => {
  it("rejects odd interleaved lengths (CWE-20)", () => {
    assert.throws(
      () => new ComplexArray([1, 2, 3]),
      (err: unknown) => err instanceof OddInterleavedLengthError && err.length === 3
    );
  });

  it("rejects length mismatch between real and imag (CWE-20)", () => {
    assert.throws(
      () => ComplexArray.fromRealImag([1, 2], [1]),
      (err: unknown) =>
        err instanceof LengthMismatchError &&
        err.realLen === 2 &&
        err.imagLen === 1
    );
  });

  it("rejects non-finite floats in ComplexArray inputs (CWE-20)", () => {
    assert.throws(() => new ComplexArray([NaN, 0]), InvalidInputError);
    assert.throws(() => new ComplexArray([0, Infinity]), InvalidInputError);
    assert.throws(() => ComplexArray.fromReal([1, -Infinity]), InvalidInputError);
    assert.throws(() => ComplexArray.fromRealImag([NaN], [1]), InvalidInputError);
    assert.throws(() => ComplexArray.fromRealImag([1], [NaN]), InvalidInputError);
  });

  it("enforces pre-flight bounds checks without allocating (CWE-400)", () => {
    // Array with length > MAX_SAMPLES (fake array with custom length property to avoid actual allocation)
    const fakeHugeArray = {
      length: MAX_SAMPLES + 1,
      [0]: 1.0,
    };
    assert.throws(() => ComplexArray.fromReal(fakeHugeArray), InvalidInputError);
    assert.throws(
      () => ComplexArray.fromRealImag(fakeHugeArray, fakeHugeArray),
      InvalidInputError
    );
    assert.throws(
      () => new ComplexArray({ length: 2 * MAX_SAMPLES + 2 }),
      InvalidInputError
    );
    assert.throws(() => ComplexArray.zeros(MAX_SAMPLES + 1), InvalidInputError);
    assert.throws(() => ComplexArray.zeros(-1), InvalidInputError);
    assert.throws(() => ComplexArray.zeros(1.5), InvalidInputError);
  });

  it("rejects out of bounds and non-integer indexing", () => {
    const arr = ComplexArray.fromReal([1, 2, 3]);
    assert.throws(() => arr.get(3), RangeError);
    assert.throws(() => arr.get(-4), RangeError);
    assert.throws(() => arr.get(1.5), RangeError);

    assert.throws(() => arr.set(3, { re: 0, im: 0 }), RangeError);
    assert.throws(() => arr.set(0, { re: NaN, im: 0 }), InvalidInputError);
  });

  it("rejects element-wise arithmetic length mismatch and zero division", () => {
    const a = ComplexArray.fromReal([1, 2]);
    const b = ComplexArray.fromReal([1]);
    assert.throws(() => a.add(b), LengthMismatchError);
    assert.throws(() => a.sub(b), LengthMismatchError);
    assert.throws(() => a.mul(b), LengthMismatchError);
    assert.throws(() => a.div(b), LengthMismatchError);

    const zero = ComplexArray.zeros(2);
    assert.throws(() => a.div(zero), InvalidInputError);
    assert.throws(() => a.scale(NaN), InvalidInputError);
  });

  it("verifies required_capabilities.json is pure empty []", () => {
    const capPath = path.resolve(__dirname, "../../required_capabilities.json");
    const content = JSON.parse(fs.readFileSync(capPath, "utf-8"));
    assert.deepEqual(content.capabilities, []);
  });

  it("verifies src has zero forbidden imports or calls (VER08a)", () => {
    const srcDir = path.resolve(__dirname, "../../src");
    const files = fs.readdirSync(srcDir).filter((f) => f.endsWith(".ts"));
    for (const f of files) {
      const code = fs.readFileSync(path.join(srcDir, f), "utf-8");
      // zero node:fs, node:net, node:http, node:child_process, process, eval
      assert.ok(!code.includes('from "node:fs"'));
      assert.ok(!code.includes('from "node:net"'));
      assert.ok(!code.includes('from "node:child_process"'));
      assert.ok(!code.includes("eval("));
      assert.ok(!code.includes("Function("));
      // zero regular expressions (CWE-1333)
      assert.ok(!code.includes("RegExp"));
    }
  });

  it("checks equals with mismatched lengths returns false", () => {
    const a = ComplexArray.fromReal([1, 2]);
    const b = ComplexArray.fromReal([1, 2, 3]);
    assert.equal(a.equals(b), false);
    const c = ComplexArray.fromReal([1, 99]);
    assert.equal(a.equals(c), false);
  });

  it("rejects non-finite or negative eps in equals (CWE-20)", () => {
    const c1 = new Complex(1, 2);
    const c2 = new Complex(1, 2);
    assert.throws(() => c1.equals(c2, -1e-5), InvalidInputError);
    assert.throws(() => c1.equals(c2, NaN), InvalidInputError);
    assert.throws(() => c1.equals(c2, Infinity), InvalidInputError);

    const a1 = ComplexArray.zeros(2);
    const a2 = ComplexArray.zeros(2);
    assert.throws(() => a1.equals(a2, -0.01), InvalidInputError);
    assert.throws(() => a1.equals(a2, NaN), InvalidInputError);
    assert.throws(() => a1.equals(a2, Infinity), InvalidInputError);
  });

  it("VER15: rejects > MAX_SAFE_INTEGER (1e100), Symbol, Object.create(null), DataView, and invalid lengths cleanly", () => {
    const c1 = new Complex(1, 2);
    const c2 = new Complex(1, 2);
    const a1 = ComplexArray.zeros(2);
    const a2 = ComplexArray.zeros(2);

    // Number.isSafeInteger checks
    assert.throws(() => ComplexArray.zeros(1e100), InvalidInputError);
    assert.throws(() => ComplexArray.zeros(Number.MAX_SAFE_INTEGER + 1), InvalidInputError);
    assert.throws(() => a1.get(1e100), RangeError);
    assert.throws(() => a1.get(1.5), RangeError);
    assert.throws(() => a1.set(1e100, c1), RangeError);
    assert.throws(() => new ComplexArray({ length: -2 }), InvalidInputError);
    assert.throws(() => ComplexArray.fromReal({ length: 1.5 }), InvalidInputError);
    assert.throws(() => ComplexArray.fromReal({ length: Number.NaN }), InvalidInputError);

    // Symbol & Object.create(null) & throwing toString must not crash String() or Number()
    const sym = Symbol("hostile");
    const nullProto = Object.create(null);
    const throwingObj = {
      toString() {
        throw new Error("boom");
      },
    };
    for (const hostile of [sym, nullProto, throwingObj]) {
      assert.throws(() => c1.equals(c2, hostile as number), InvalidInputError);
      assert.throws(() => a1.equals(a2, hostile as number), InvalidInputError);
      assert.throws(() => new Complex(hostile as number, 0), InvalidInputError);
      assert.throws(() => c1.add(hostile as Complex), InvalidInputError);
      assert.throws(() => c1.div(hostile as Complex), InvalidInputError);
      assert.throws(() => new ComplexArray([hostile as number, 0]), InvalidInputError);
      assert.throws(() => ComplexArray.fromReal([hostile as number]), InvalidInputError);
      assert.throws(() => ComplexArray.fromRealImag([hostile as number], [0]), InvalidInputError);
      assert.throws(() => new ComplexArray({ length: hostile as number }), InvalidInputError);
      assert.throws(() => a1.get(hostile as number), RangeError);
      assert.throws(() => a1.set(0, hostile as Complex), InvalidInputError);
    }

    // Null / invalid ComplexArray operands
    assert.throws(() => c1.add(null as unknown as Complex), InvalidInputError);
    assert.throws(() => a1.add(null as unknown as ComplexArray), InvalidInputError);
    assert.equal(c1.equals(null as unknown as Complex), false);
    assert.equal(a1.equals(null as unknown as ComplexArray), false);

    // ArrayBuffer.isView: DataView has no .length; BigInt64Array elements are bigints; oversized typed arrays hit pre-flight cap
    const dv = new DataView(new ArrayBuffer(16));
    assert.throws(() => new ComplexArray(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(() => ComplexArray.fromReal(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(
      () => ComplexArray.fromRealImag(dv as unknown as Float64Array, dv as unknown as Float64Array),
      InvalidInputError
    );
    const bi = new BigInt64Array([1n, 2n]);
    assert.throws(() => ComplexArray.fromReal(bi as unknown as Float64Array), InvalidInputError);
    const oversized = new Uint8Array(MAX_SAMPLES + 1);
    assert.throws(() => ComplexArray.fromReal(oversized as unknown as Float64Array), InvalidInputError);

    // CWE-1321: prototype pollution resistance
    const protoPayload = JSON.parse('{"re":1,"im":2,"__proto__":{"pollutedComplexArr":true}}');
    a1.set(0, protoPayload);
    assert.equal(a1.get(0).re, 1);
    assert.equal(({} as { pollutedComplexArr?: boolean }).pollutedComplexArr, undefined);
  });

  it("VER15: Smith complex division avoids overflow to NaN and ComplexArray arithmetic enforces finiteness", () => {
    // 1e200 / 1e200 must equal 1 + 0i without squaring overflow
    const bigScalar = new Complex(1e200, 0);
    assert.ok(bigScalar.div(bigScalar).equals(Complex.one(), 1e-12));
    const bigImag = new Complex(0, 1e200);
    assert.ok(bigImag.div(bigImag).equals(Complex.one(), 1e-12));
    const bigDiag = new Complex(1e200, -1e200);
    assert.ok(bigDiag.div(bigDiag).equals(Complex.one(), 1e-12));

    const bigArr = ComplexArray.fromReal([1e200]);
    const divRes = bigArr.div(bigArr);
    assert.equal(divRes.get(0).re, 1);
    assert.equal(divRes.get(0).im, 0);

    const bigImagArr = ComplexArray.fromRealImag([0], [1e200]);
    const divImagRes = bigImagArr.div(bigImagArr);
    assert.equal(divImagRes.get(0).re, 1);
    assert.equal(divImagRes.get(0).im, 0);

    // Overflow in scale, add, sub, mul, div must fail closed with InvalidInputError (never NaN/Inf in _data)
    const huge = ComplexArray.fromReal([1e308]);
    const tiny = ComplexArray.fromReal([1e-10]);
    assert.throws(() => huge.scale(10), InvalidInputError);
    assert.throws(() => huge.add(huge), InvalidInputError);
    assert.throws(() => huge.sub(huge.scale(-1)), InvalidInputError);
    assert.throws(() => huge.mul(huge), InvalidInputError);
    assert.throws(() => huge.div(tiny), InvalidInputError);
    assert.throws(() => new Complex(1e308, 0).div(new Complex(1e-10, 0)), InvalidInputError);
  });
});
