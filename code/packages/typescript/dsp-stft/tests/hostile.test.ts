import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  buildWindow,
  EmptySignalError,
  FftError,
  hzToMel,
  InvalidParamError,
  InvalidSpectrogramError,
  istft,
  logSpectrogram,
  MAX_FFT_LEN,
  MAX_FRAMES,
  MAX_MELS,
  MAX_SAMPLES,
  melFilterbank,
  melSpectrogram,
  melToHz,
  mfcc,
  parseWindowType,
  powerSpectrogram,
  safeFormatValue,
  SignalTooShortError,
  stft,
  StftError,
  WindowType,
} from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Hostile inputs & VER15 Security Hardening (dsp-stft)", () => {
  it("rejects empty signals and empty spectrograms fail-closed (CWE-20)", () => {
    assert.throws(() => stft([], 256, 128), EmptySignalError);
    assert.throws(() => istft([], 256, 128), InvalidSpectrogramError);
    assert.throws(() => powerSpectrogram([], 256, 128), EmptySignalError);
    assert.throws(() => logSpectrogram([], 256, 128), EmptySignalError);
    assert.throws(() => melSpectrogram([], 256, 128, 40, 16000), EmptySignalError);
    assert.throws(() => mfcc([], 256, 128, 40, 13, 16000), EmptySignalError);
  });

  it("rejects non-finite floats fail-closed (CWE-20)", () => {
    assert.throws(() => stft([NaN, 1.0, 2.0, 3.0], 4, 2), InvalidParamError);
    assert.throws(() => stft([1.0, Infinity, 2.0, 3.0], 4, 2), InvalidParamError);
    assert.throws(() => stft([1.0, -Infinity, 2.0, 3.0], 4, 2), InvalidParamError);

    // Spectrogram with NaN
    assert.throws(() => istft([NaN, 0, 0, 0, 0, 0], 4, 2), InvalidSpectrogramError);
  });

  it("enforces pre-flight defensive bounds without allocating (CWE-400)", () => {
    const fakeHugeSignal = { length: MAX_SAMPLES + 1, [0]: 1.0 };
    assert.throws(() => stft(fakeHugeSignal, 256, 128), InvalidParamError);
    assert.throws(() => stft([1, 2], MAX_FFT_LEN + 1, 128), InvalidParamError);

    // Signal too short for nFft
    assert.throws(() => stft([1, 2, 3], 8, 4), SignalTooShortError);

    // Spectrogram bounds
    const fakeHugeSpec = { length: MAX_SAMPLES + 1, [0]: 1.0 };
    assert.throws(() => istft(fakeHugeSpec, 256, 128), InvalidParamError);
    assert.throws(() => istft([0, 0], 256, 128, WindowType.HANN, MAX_SAMPLES + 1), InvalidParamError);

    // Mel bounds
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, MAX_MELS + 1, 16000), InvalidParamError);
    assert.equal(melFilterbank(MAX_MELS + 1, 256, 16000).length, 0);
  });

  it("rejects invalid parameters fail-closed", () => {
    assert.throws(() => stft([1, 2, 3, 4], 0, 2), InvalidParamError);
    assert.throws(() => stft([1, 2, 3, 4], 4, 0), InvalidParamError);
    assert.throws(() => stft([1, 2, 3, 4], 4, -1), InvalidParamError);
    assert.throws(() => stft([1, 2, 3, 4], 4, 1.5), InvalidParamError);

    assert.throws(() => istft([1, 0, 0, 0], 0, 2), InvalidParamError);
    assert.throws(() => istft([1, 0, 0, 0], 4, 0), InvalidParamError);
    assert.throws(() => istft([1, 0, 0, 0], 4, 2, WindowType.HANN, 0), InvalidParamError);
    assert.throws(() => istft([1, 0, 0, 0], 4, 2, WindowType.HANN, -5), InvalidParamError);

    // Malformed spectrogram length (not divisible by frameFloats)
    assert.throws(() => istft([1, 0, 0], 4, 2), InvalidSpectrogramError);

    // Windows validations
    assert.throws(() => buildWindow(WindowType.HANN, 0), InvalidParamError);
    assert.throws(() => buildWindow(WindowType.HANN, -5), InvalidParamError);
    assert.throws(() => buildWindow(WindowType.HANN, MAX_FFT_LEN + 1), InvalidParamError);
    assert.throws(() => parseWindowType("unknown_window"), InvalidParamError);
    assert.throws(() => parseWindowType(123 as any), InvalidParamError);

    // Log spectrogram eps
    assert.throws(() => logSpectrogram([1, 2, 3, 4], 4, 2, WindowType.HANN, 0.0), InvalidParamError);
    assert.throws(() => logSpectrogram([1, 2, 3, 4], 4, 2, WindowType.HANN, -1.0), InvalidParamError);
    assert.throws(() => logSpectrogram([1, 2, 3, 4], 4, 2, WindowType.HANN, NaN), InvalidParamError);

    // Mel & MFCC params
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, 0, 16000), InvalidParamError);
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, 40, 0), InvalidParamError);
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, 40, -1000), InvalidParamError);
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, 40, NaN), InvalidParamError);

    assert.throws(() => mfcc([1, 2, 3, 4], 4, 2, 40, 0, 16000), InvalidParamError);
    assert.throws(() => mfcc([1, 2, 3, 4], 4, 2, 40, 41, 16000), InvalidParamError); // n_mfcc > n_mels
  });

  it("melFilterbank gracefully returns empty on invalid inputs", () => {
    assert.equal(melFilterbank(0, 256, 16000).length, 0);
    assert.equal(melFilterbank(40, 0, 16000).length, 0);
    assert.equal(melFilterbank(40, 256, 0).length, 0);
    assert.equal(melFilterbank(40, 256, -1000).length, 0);
    assert.equal(melFilterbank(40, 256, NaN).length, 0);
  });

  it("verifies error inheritance structure", () => {
    const base = new StftError("base");
    assert.ok(base instanceof Error);

    const empty = new EmptySignalError();
    assert.ok(empty instanceof StftError);

    const param = new InvalidParamError();
    assert.ok(param instanceof StftError);

    const short = new SignalTooShortError();
    assert.ok(short instanceof StftError);

    const spec = new InvalidSpectrogramError();
    assert.ok(spec instanceof StftError);

    const fftErr = new FftError();
    assert.ok(fftErr instanceof StftError);
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

  it("rejects Symbol, Object.create(null), DataView, BigInt64Array, null inputs, and oversized enum strings (VER15)", () => {
    const sym = Symbol("bad");
    const nullProto = Object.create(null);

    assert.equal(safeFormatValue(sym), "Symbol(bad)");
    assert.equal(safeFormatValue(nullProto), "[object Object]");
    assert.equal(safeFormatValue(null), "null");

    assert.throws(() => parseWindowType(sym as any), InvalidParamError);
    assert.throws(() => parseWindowType(nullProto as any), InvalidParamError);
    assert.throws(() => parseWindowType(" ".repeat(65) + "hann"), InvalidParamError);

    assert.throws(() => buildWindow(WindowType.HANN, sym as any), InvalidParamError);
    assert.throws(() => buildWindow(WindowType.HANN, nullProto as any), InvalidParamError);
    assert.throws(() => buildWindow(WindowType.HANN, Number.MAX_SAFE_INTEGER + 1), InvalidParamError);

    assert.throws(() => stft([1, 2, 3, 4], sym as any, 2), InvalidParamError);
    assert.throws(() => stft([1, 2, 3, 4], 4, nullProto as any), InvalidParamError);
    assert.throws(() => stft([1, 2, 3, 4], 4, 2, " ".repeat(65) + "hann"), InvalidParamError);

    assert.throws(() => istft([1, 0, 0, 0, 0, 0], sym as any, 2), InvalidParamError);
    assert.throws(() => istft([1, 0, 0, 0, 0, 0], 4, nullProto as any), InvalidParamError);
    assert.throws(() => istft([1, 0, 0, 0, 0, 0], 4, 2, WindowType.HANN, sym as any), InvalidParamError);

    assert.throws(() => logSpectrogram([1, 2, 3, 4], 4, 2, WindowType.HANN, sym as any), InvalidParamError);
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, sym as any, 16000), InvalidParamError);
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, 4, nullProto as any), InvalidParamError);
    assert.throws(() => mfcc([1, 2, 3, 4], 4, 2, 4, sym as any, 16000), InvalidParamError);

    const dv = new DataView(new ArrayBuffer(16));
    const bi = new BigInt64Array(4);
    const bui = new BigUint64Array(4);

    assert.throws(() => stft(dv as any, 4, 2), InvalidParamError);
    assert.throws(() => stft(bi as any, 4, 2), InvalidParamError);
    assert.throws(() => stft(bui as any, 4, 2), InvalidParamError);
    assert.throws(() => stft(null as any, 4, 2), InvalidParamError);
    assert.throws(() => stft({} as any, 4, 2), InvalidParamError);
    assert.throws(() => stft([sym as any, 1, 2, 3], 4, 2), InvalidParamError);
    assert.throws(() => stft([nullProto as any, 1, 2, 3], 4, 2), InvalidParamError);

    assert.throws(() => istft(dv as any, 4, 2), InvalidParamError);
    assert.throws(() => istft(bi as any, 4, 2), InvalidParamError);
    assert.throws(() => istft(null as any, 4, 2), InvalidParamError);
    assert.throws(() => istft([sym as any, 0, 0, 0, 0, 0], 4, 2), InvalidSpectrogramError);

    // Findings 6, 7, 10 regressions
    assert.throws(() => hzToMel(sym as any), InvalidParamError);
    assert.throws(() => hzToMel(-1000), InvalidParamError);
    assert.throws(() => hzToMel(NaN), InvalidParamError);
    assert.throws(() => melToHz(sym as any), InvalidParamError);
    assert.throws(() => melToHz(-1), InvalidParamError);
    assert.throws(() => melToHz(1e6), InvalidParamError);
    assert.equal(melFilterbank(4, 512, 1e-310).length, 0);
    assert.throws(() => melSpectrogram([1, 2, 3, 4], 4, 2, 4, 1e-310), InvalidParamError);
    assert.throws(() => powerSpectrogram([1e200, 1e200, 1e200, 1e200], 4, 2), InvalidParamError);
    assert.throws(() => melSpectrogram([1e200, 1e200, 1e200, 1e200], 4, 2, 4, 16000), InvalidParamError);
    assert.throws(() => mfcc([1e200, 1e200, 1e200, 1e200], 4, 2, 4, 2, 16000), (err) => err instanceof StftError && err instanceof InvalidParamError);
    const nullCtorDv = Object.assign(new DataView(new ArrayBuffer(8)), { constructor: null });
    assert.throws(() => stft(nullCtorDv as any, 4, 2), InvalidParamError);
  });
});
