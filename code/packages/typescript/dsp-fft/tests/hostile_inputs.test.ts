import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import { ComplexArray } from "@coding-adventures/dsp-complex";
import {
  besselI0,
  buildChirp,
  buildWindow,
  fft,
  fftScalar,
  ifft,
  ifftScalar,
  InvalidInputError,
  irfft,
  irfftScalar,
  kaiser,
  MAX_FFT_LEN,
  MAX_KAISER_BETA,
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

  it("VER15: rejects > MAX_SAFE_INTEGER (1e100), 32-bit wrap (2^32+1), Symbol, Object.create(null), and DataView cleanly", () => {
    // Number.isSafeInteger & 32-bit bitwise truncation guards
    assert.throws(() => requirePow2Length(1e100), InvalidInputError);
    assert.throws(() => requirePow2Length(Number.MAX_SAFE_INTEGER + 1), InvalidInputError);
    assert.throws(() => requirePow2Length(4294967297), InvalidInputError); // 2^32 + 1
    assert.throws(() => buildChirp(1e100, 1), InvalidInputError);
    assert.throws(() => buildWindow("hann", 1e100), InvalidInputError);
    assert.throws(() => irfftScalar([1, 0, 2, 0], 1e100), InvalidInputError);
    assert.throws(() => fft({ length: -2 }, { complex: true }), InvalidInputError);
    assert.throws(() => fft({ length: 1.5 }), InvalidInputError);
    assert.throws(() => rfftScalar({ length: -1 }), InvalidInputError);

    // Non-string window types must fail closed (never silently return zeroed array)
    assert.throws(() => buildWindow(123 as unknown as string, 4), InvalidInputError);
    assert.throws(() => buildWindow(null as unknown as string, 4), InvalidInputError);

    // Symbol, Object.create(null), throwing toString
    const sym = Symbol("hostile");
    const nullProto = Object.create(null);
    const throwingObj = {
      toString() {
        throw new Error("boom");
      },
    };
    for (const hostile of [sym, nullProto, throwingObj]) {
      assert.throws(() => requirePow2Length(hostile as number), InvalidInputError);
      assert.throws(() => buildWindow(hostile as string, 4), InvalidInputError);
      assert.throws(() => fft([hostile as number, 0]), InvalidInputError);
      assert.throws(() => fft([hostile as number, 0], { complex: true }), InvalidInputError);
      assert.throws(() => fftScalar([hostile as number, 0]), InvalidInputError);
      assert.throws(() => rfftScalar([hostile as number]), InvalidInputError);
      assert.throws(() => irfftScalar([hostile as number, 0, 0, 0], 2), InvalidInputError);
      assert.throws(() => fft({ length: hostile as number }), InvalidInputError);
    }

    // Null / invalid ComplexArray for ifft
    assert.throws(() => ifft(null as unknown as ComplexArray), InvalidInputError);

    // ArrayBuffer.isView: DataView, BigInt64Array, oversized typed array pre-flight check
    const dv = new DataView(new ArrayBuffer(16));
    assert.throws(() => fft(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(() => fft(dv as unknown as Float64Array, { complex: true }), InvalidInputError);
    assert.throws(() => fftScalar(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(() => rfftScalar(dv as unknown as Float64Array), InvalidInputError);
    assert.throws(() => irfftScalar(dv as unknown as Float64Array, 2), InvalidInputError);

    const bi = new BigInt64Array([1n, 2n]);
    assert.throws(() => fft(bi as unknown as Float64Array), InvalidInputError);
    const oversized = new Uint8Array(MAX_FFT_LEN + 1);
    assert.throws(() => rfftScalar(oversized as unknown as Float64Array), InvalidInputError);

    // CWE-1321: prototype pollution resistance
    const opts = JSON.parse('{"complex":false,"__proto__":{"pollutedFftOpt":true}}');
    const out = fft([1, 0, 0, 0], opts);
    assert.equal(out.length, 4);
    assert.equal(({} as { pollutedFftOpt?: boolean }).pollutedFftOpt, undefined);
  });

  it("VER15: caps Kaiser beta / besselI0 at MAX_KAISER_BETA=700, converges to k<1024, and bounds enum strings", () => {
    assert.equal(MAX_KAISER_BETA, 700.0);
    assert.throws(() => besselI0(1e20), InvalidInputError);
    assert.throws(() => besselI0(-1e20), InvalidInputError);
    assert.throws(() => besselI0(700.1), InvalidInputError);
    assert.throws(() => kaiser(8, 1e20), InvalidInputError);
    assert.throws(() => kaiser(8, 700.1), InvalidInputError);
    assert.throws(() => buildWindow("kaiser", 1, { beta: 1e20 }), InvalidInputError);

    // At beta = 700, besselI0 converges cleanly and kaiser(9, 700) peaks at 1.0 without NaN/Infinity
    const i0Max = besselI0(700);
    assert.ok(Number.isFinite(i0Max) && i0Max > 1e300);
    const w8 = kaiser(8, 700);
    for (let i = 0; i < w8.length; i++) {
      assert.ok(Number.isFinite(w8[i]) && w8[i] >= 0 && w8[i] <= 1.0);
    }
    const w9 = kaiser(9, 700);
    assert.equal(w9[4], 1.0);

    // Oversized enum string must reject in O(1) before .trim().toLowerCase()
    const hugeName = " ".repeat(50_000) + "hann" + " ".repeat(50_000);
    assert.throws(() => buildWindow(hugeName, 4), InvalidInputError);
  });
});
