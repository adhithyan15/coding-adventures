import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { ComplexArray } from "@coding-adventures/dsp-complex";
import {
  buildChirp,
  buildWindow,
  fft,
  fftScalar,
  ifft,
  ifftScalar,
  InvalidInputError,
  irfft,
  irfftScalar,
  MAX_FFT_LEN,
  MAX_SAMPLES,
  requirePow2Length,
  rfft,
  rfftScalar,
  WindowType,
} from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Hostile inputs & VER15 Security Hardening", () => {
  it("rejects non-finite floats fail-closed (CWE-20)", () => {
    assert.throws(() => fftScalar([NaN, 0]), InvalidInputError);
    assert.throws(() => fftScalar([0, Infinity]), InvalidInputError);
    assert.throws(() => ifftScalar([0, -Infinity]), InvalidInputError);
    assert.throws(() => rfftScalar([1, NaN, 2]), InvalidInputError);
    assert.throws(() => fft([NaN, 1, 2, 3]), InvalidInputError);
    assert.throws(
      () => fft([NaN, 1, 2, 3], { complex: true }),
      InvalidInputError
    );
  });

  it("rejects empty signals (CWE-20)", () => {
    assert.throws(() => fftScalar([]), InvalidInputError);
    assert.throws(() => ifftScalar([]), InvalidInputError);
    assert.throws(() => rfftScalar([]), InvalidInputError);
    assert.throws(() => fft([]), InvalidInputError);
    assert.throws(() => fft([], { complex: true }), InvalidInputError);
    assert.throws(() => fft(new ComplexArray()), InvalidInputError);
    assert.throws(() => ifft(new ComplexArray()), InvalidInputError);
  });

  it("enforces pre-flight bounds checks without allocating (CWE-400)", () => {
    const fakeHugeSignal = {
      length: MAX_FFT_LEN + 1,
      [0]: 1.0,
    };
    assert.throws(() => rfft(fakeHugeSignal), InvalidInputError);
    assert.throws(() => fft(fakeHugeSignal), InvalidInputError);
    assert.throws(
      () => fft(fakeHugeSignal, { complex: true }),
      InvalidInputError
    );

    const fakeHugeComplex = {
      length: 2 * MAX_FFT_LEN + 2,
      [0]: 1.0,
    };
    assert.throws(
      () => fft(fakeHugeComplex, { complex: true }),
      InvalidInputError
    );
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

  it("exercises remaining hostile error branches across radix2, rfft, and windows", () => {
    // validateBuffer with odd length
    assert.throws(() => fftScalar([1, 2, 3]), InvalidInputError);
    // validateBuffer with huge length
    const hugeBuf = { length: 2 * MAX_FFT_LEN + 2, [0]: 1.0 };
    assert.throws(() => fftScalar(hugeBuf), InvalidInputError);
    const hugeSamples = { length: MAX_SAMPLES + 2, [0]: 1.0 };
    assert.throws(() => fftScalar(hugeSamples), InvalidInputError);

    // irfftScalar bounds and non-finite checks
    assert.throws(() => irfftScalar([1, 0, 2, 0], MAX_FFT_LEN + 1), InvalidInputError);
    assert.throws(() => irfftScalar({ length: MAX_SAMPLES + 2, [0]: 1.0 }, 2), InvalidInputError);
    assert.throws(() => irfftScalar([1, NaN, 2, 0], 2), InvalidInputError);

    // rfftScalar bounds
    assert.throws(() => rfftScalar(hugeSamples), InvalidInputError);

    // WindowType enum directly
    const wFromEnum = buildWindow(WindowType.HANN, 4);
    assert.equal(wFromEnum.length, 4);
    assert.throws(() => buildWindow(WindowType.HANN, MAX_FFT_LEN + 1), InvalidInputError);

    // fft and ifft bounds with ComplexArray
    const fakeHugeComplexArray = Object.create(ComplexArray.prototype);
    Object.defineProperty(fakeHugeComplexArray, "length", { value: MAX_FFT_LEN + 1 });
    Object.defineProperty(fakeHugeComplexArray, "asInterleaved", { value: () => new Float64Array(0) });
    assert.throws(() => fft(fakeHugeComplexArray), InvalidInputError);
    assert.throws(() => ifft(fakeHugeComplexArray), InvalidInputError);
  });

  it("tests irfft with plain array and ComplexArray inputs", () => {
    const half = rfft([1, 2, 3, 4]);
    const recovered1 = irfft(half, 4);
    const recovered2 = irfft(half.asInterleaved(), 4);
    assert.deepEqual(Array.from(recovered1), Array.from(recovered2));
  });

  it("enforces fail-closed bounds and type checks on buildWindow, buildChirp, and requirePow2Length (CWE-20/CWE-400)", () => {
    // buildWindow Kaiser beta validation before n === 1 early return
    assert.throws(() => buildWindow("kaiser", 1, { beta: NaN }), InvalidInputError);
    assert.throws(() => buildWindow("kaiser", 1, { beta: -1.0 }), InvalidInputError);
    assert.throws(() => buildWindow("kaiser", 5, { beta: NaN }), InvalidInputError);
    assert.throws(() => buildWindow("kaiser", 5, { beta: -0.5 }), InvalidInputError);

    // buildChirp validation
    assert.throws(() => buildChirp(-1, 1), InvalidInputError);
    assert.throws(() => buildChirp(1.5, 1), InvalidInputError);
    assert.throws(() => buildChirp(MAX_FFT_LEN + 1, 1), InvalidInputError);
    assert.throws(() => buildChirp(4, NaN), InvalidInputError);
    assert.throws(() => buildChirp(4, Infinity), InvalidInputError);

    // requirePow2Length validation
    assert.throws(() => requirePow2Length(NaN), InvalidInputError);
    assert.throws(() => requirePow2Length(Infinity), InvalidInputError);
    assert.throws(() => requirePow2Length(1.5), InvalidInputError);
    assert.throws(() => requirePow2Length(0), InvalidInputError);
    assert.throws(() => requirePow2Length(-4), InvalidInputError);
  });
});
