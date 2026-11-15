import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  cwt,
  dwt,
  dwt1d,
  dwt2d,
  EmptySignalError,
  FftError,
  forwardLevelLengths,
  idwt,
  idwt1d,
  idwt2d,
  InvalidCoefficientsError,
  InvalidParamError,
  iswt,
  MAX_SAMPLES,
  MAX_SCALES,
  MAX_WAVELET_LEVELS,
  parseWavelet,
  parseWaveletBoundary,
  SignalTooShortError,
  swt,
  wavedec,
  Wavelet,
  WaveletError,
  waverec,
} from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Hostile inputs & VER15 Security Hardening (dsp-wavelets)", () => {
  it("rejects empty signals fail-closed (CWE-20)", () => {
    assert.throws(() => dwt([]), EmptySignalError);
    assert.throws(() => idwt([], [1.0]), EmptySignalError);
    assert.throws(() => idwt([1.0], []), EmptySignalError);
    assert.throws(() => dwt1d([]), EmptySignalError);
    assert.throws(() => idwt1d([], "haar", 1, undefined, 4), EmptySignalError);
    assert.throws(() => dwt2d([], 1, 1), EmptySignalError);
    assert.throws(() => idwt2d([], 1, 1), EmptySignalError);
    assert.throws(() => wavedec([]), EmptySignalError);
    assert.throws(() => swt([]), EmptySignalError);
    assert.throws(() => iswt([]), EmptySignalError);
    assert.throws(() => iswt([[[], []]]), EmptySignalError);
    assert.throws(() => cwt([], [1.0]), EmptySignalError);
  });

  it("rejects invalid parameters fail-closed (CWE-190 / CWE-20)", () => {
    // Invalid levels
    assert.throws(() => dwt1d([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], "haar", 0), InvalidParamError);
    assert.throws(
      () => dwt1d([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], "haar", MAX_WAVELET_LEVELS + 1),
      InvalidParamError
    );
    assert.throws(() => wavedec([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], "haar", undefined, 0), InvalidParamError);
    assert.throws(
      () => wavedec([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], "haar", undefined, MAX_WAVELET_LEVELS + 1),
      InvalidParamError
    );
    assert.throws(() => swt([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], "haar", 0), InvalidParamError);
    assert.throws(
      () => swt([1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0], "haar", MAX_WAVELET_LEVELS + 1),
      InvalidParamError
    );

    const manyPairs: Array<[number[], number[]]> = [];
    for (let i = 0; i < MAX_WAVELET_LEVELS + 1; i++) {
      manyPairs.push([[1.0], [1.0]]);
    }
    assert.throws(() => iswt(manyPairs), InvalidParamError);

    const manyBands: number[][] = [];
    for (let i = 0; i < MAX_WAVELET_LEVELS + 2; i++) {
      manyBands.push([1.0]);
    }
    assert.throws(() => waverec(manyBands), InvalidParamError);

    // 2D invalid dimensions & levels
    assert.throws(() => dwt2d([1.0], 0, 1), InvalidParamError);
    assert.throws(() => dwt2d([1.0], 1, 0), InvalidParamError);
    assert.throws(() => dwt2d([1.0, 2.0], 2, 2), InvalidParamError); // size mismatch
    assert.throws(() => dwt2d([1.0], 1, 1, "haar", 0), InvalidParamError);
    assert.throws(() => dwt2d([1.0], 1, 1, "haar", MAX_WAVELET_LEVELS + 1), InvalidParamError);
    assert.throws(() => idwt2d([1.0], 0, 1), InvalidParamError);
    assert.throws(() => idwt2d([1.0], 1, 0), InvalidParamError);
    assert.throws(() => idwt2d([1.0, 2.0], 2, 2), InvalidCoefficientsError); // size mismatch
    assert.throws(() => idwt2d([1.0], 1, 1, "haar", 0), InvalidParamError);
    assert.throws(() => idwt2d([1.0], 1, 1, "haar", MAX_WAVELET_LEVELS + 1), InvalidParamError);

    // IDWT invalid params
    assert.throws(() => idwt([1.0, 2.0], [1.0]), InvalidCoefficientsError);
    assert.throws(() => idwt([1.0], [1.0], "haar", undefined, 0), InvalidParamError);
    assert.throws(() => idwt1d([1.0, 2.0], "haar", 1, undefined, 0), InvalidParamError);
    assert.throws(() => idwt1d([1.0, 2.0], "haar", 1, undefined, 5), InvalidCoefficientsError);

    // forwardLevelLengths validation
    assert.throws(() => forwardLevelLengths(0, 1), InvalidParamError);
    assert.throws(() => forwardLevelLengths(16, 0), InvalidParamError);
    assert.throws(() => forwardLevelLengths(16, MAX_WAVELET_LEVELS + 1), InvalidParamError);
    assert.throws(() => forwardLevelLengths(MAX_SAMPLES + 1, 1), InvalidParamError);

    // boundary invalid string
    assert.throws(() => parseWaveletBoundary("unknown_mode"), InvalidParamError);

    // Unvalidated boundary mode in dwt/idwt/dwt1d/idwt1d/dwt2d/idwt2d
    assert.throws(() => dwt([1, 2], "haar", "invalid_mode"), InvalidParamError);
    assert.throws(() => dwt([1, 2], "haar", 123 as any), InvalidParamError);
    assert.throws(() => idwt([1], [1], "haar", "invalid_mode"), InvalidParamError);
    assert.throws(() => idwt([1], [1], "haar", 123 as any), InvalidParamError);
    assert.throws(() => dwt1d([1, 2], "haar", 1, "invalid_mode"), InvalidParamError);
    assert.throws(() => dwt1d([1, 2], "haar", 1, 123 as any), InvalidParamError);
    assert.throws(() => idwt1d([1, 2], "haar", 1, "invalid_mode", 2), InvalidParamError);
    assert.throws(() => idwt1d([1, 2], "haar", 1, 123 as any, 2), InvalidParamError);
    assert.throws(() => dwt2d([1], 1, 1, "haar", 1, "invalid_mode"), InvalidParamError);
    assert.throws(() => dwt2d([1], 1, 1, "haar", 1, 123 as any), InvalidParamError);
    assert.throws(() => idwt2d([1], 1, 1, "haar", 1, "invalid_mode"), InvalidParamError);
    assert.throws(() => idwt2d([1], 1, 1, "haar", 1, 123 as any), InvalidParamError);

    // Tightened wavelet name validation
    assert.throws(() => parseWavelet("db2foo"), InvalidParamError);
    assert.throws(() => parseWavelet("sym4bar"), InvalidParamError);
    assert.throws(() => parseWavelet("coif1baz"), InvalidParamError);
    assert.throws(() => parseWavelet("db"), InvalidParamError);
    assert.throws(() => parseWavelet("sym"), InvalidParamError);
    assert.throws(() => parseWavelet("coif"), InvalidParamError);
  });

  it("rejects non-orthogonal wavelets in wavedec and swt", () => {
    const wBior = Wavelet.bior(2, 2);
    assert.throws(() => wavedec([1, 2, 3, 4], wBior), InvalidParamError);
    assert.throws(() => waverec([[1, 2], [1, 2]], wBior), InvalidParamError);
    assert.throws(() => swt([1, 2, 3, 4], wBior), InvalidParamError);
    assert.throws(() => iswt([[[1, 2], [1, 2]]], wBior), InvalidParamError);
    assert.throws(() => dwt2d([1, 2, 3, 4], 2, 2, wBior), InvalidParamError);
    assert.throws(() => idwt2d([1, 2, 3, 4], 2, 2, wBior), InvalidParamError);
  });

  it("rejects signal too short (SignalTooShortError)", () => {
    assert.throws(() => dwt1d([1.0, 2.0], "haar", 3), SignalTooShortError);
    assert.throws(() => wavedec([1.0, 2.0], "haar", undefined, 3), SignalTooShortError);
  });

  it("rejects non-finite floats fail-closed (CWE-20)", () => {
    for (const bad of [NaN, Infinity, -Infinity]) {
      assert.throws(() => dwt([1.0, bad]), InvalidParamError);
      assert.throws(() => idwt([1.0, bad], [1.0, 1.0]), InvalidCoefficientsError);
      assert.throws(() => idwt([1.0, 1.0], [1.0, bad]), InvalidCoefficientsError);
      assert.throws(() => dwt1d([1.0, bad, 2.0, 3.0]), InvalidParamError);
      assert.throws(() => idwt1d([1.0, bad], "haar", 1, undefined, 2), InvalidCoefficientsError);
      assert.throws(() => dwt2d([1.0, bad, 0.0, 0.0], 2, 2), InvalidParamError);
      assert.throws(() => idwt2d([1.0, bad, 0.0, 0.0], 2, 2), InvalidCoefficientsError);
      assert.throws(() => wavedec([1.0, bad, 2.0, 3.0]), InvalidParamError);
      assert.throws(() => waverec([[1.0, bad], [1.0, 1.0]]), InvalidCoefficientsError);
      assert.throws(() => swt([1.0, bad, 2.0, 3.0]), InvalidParamError);
      assert.throws(() => iswt([[[1.0, bad], [1.0, 1.0]]]), InvalidCoefficientsError);
      assert.throws(() => iswt([[[1.0, 1.0], [1.0, 1.0]], [[1.0, bad], [1.0, 1.0]]]), InvalidCoefficientsError);
      assert.throws(() => iswt([[[1.0, 1.0], [1.0, 1.0]], [[1.0, 1.0], [1.0, bad]]]), InvalidCoefficientsError);
      assert.throws(() => cwt([1.0, bad], [1.0]), InvalidParamError);
      assert.throws(() => cwt([1.0, 1.0], [bad]), InvalidParamError);
    }
  });

  it("enforces pre-flight bounds without allocating (CWE-400)", () => {
    const huge = { length: MAX_SAMPLES + 1, [0]: 1.0 };
    assert.throws(() => dwt(huge), InvalidParamError);
    assert.throws(() => idwt(huge, huge), InvalidParamError);
    assert.throws(() => idwt([1.0], [1.0], "haar", undefined, MAX_SAMPLES + 1), InvalidParamError);
    assert.throws(() => dwt1d(huge), InvalidParamError);
    assert.throws(() => dwt2d(huge, 1, MAX_SAMPLES + 1), InvalidParamError);
    assert.throws(() => idwt2d(huge, 1, MAX_SAMPLES + 1), InvalidParamError);
    assert.throws(() => wavedec(huge), InvalidParamError);
    assert.throws(() => swt(huge), InvalidParamError);
    assert.throws(() => iswt([[huge, huge]]), InvalidParamError);
    assert.throws(() => waverec([huge, [1.0]]), InvalidParamError);
    assert.throws(() => cwt(huge, [1.0]), InvalidParamError);

    const hugeScales = { length: MAX_SCALES + 1, [0]: 1.0 };
    assert.throws(() => cwt([1.0], hugeScales), InvalidParamError);

    // CWT product bound
    const fakeBigSig = { length: 1 << 20, [0]: 1.0 };
    const fakeBigScales = { length: 1 << 10, [0]: 1.0 };
    assert.throws(() => cwt(fakeBigSig, fakeBigScales), InvalidParamError);
  });

  it("verifies error hierarchy inheritance", () => {
    const base = new WaveletError("base");
    assert.ok(base instanceof Error);

    const empty = new EmptySignalError();
    assert.ok(empty instanceof WaveletError);

    const param = new InvalidParamError();
    assert.ok(param instanceof WaveletError);

    const short = new SignalTooShortError();
    assert.ok(short instanceof WaveletError);

    const coeff = new InvalidCoefficientsError();
    assert.ok(coeff instanceof WaveletError);

    const fftErr = new FftError();
    assert.ok(fftErr instanceof WaveletError);
  });

  it("verifies required_capabilities.json is pure empty []", () => {
    const capPath = path.resolve(__dirname, "../../required_capabilities.json");
    const content = JSON.parse(fs.readFileSync(capPath, "utf-8"));
    assert.deepEqual(content.capabilities, []);
  });

  it("verifies src has zero forbidden imports or calls (VER08a / CWE-1333)", () => {
    const srcDir = path.resolve(__dirname, "../../src");
    const files = fs.readdirSync(srcDir).filter((f) => f.endsWith(".ts"));
    for (const f of files) {
      const code = fs.readFileSync(path.join(srcDir, f), "utf-8");
      assert.ok(!code.includes('from "node:fs"'));
      assert.ok(!code.includes('from "node:net"'));
      assert.ok(!code.includes('from "node:http"'));
      assert.ok(!code.includes('from "node:child_process"'));
      assert.ok(!code.includes("eval("));
      assert.ok(!code.includes("Function("));
      assert.ok(!code.includes("RegExp"));
    }
  });
});
