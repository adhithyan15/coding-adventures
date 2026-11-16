import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  Biquad,
  designBandPass,
  designLowPass,
  EmptyKernelError,
  EmptySignalError,
  FftError,
  filtfilt,
  fir,
  firFft,
  InvalidCoefficientError,
  lfilter,
  MAX_FILTER_TAPS,
  MAX_ORDER,
  MAX_SIGNAL_LEN,
  sosfilt,
} from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Hostile inputs & VER15 Security Hardening", () => {
  it("rejects empty signals and empty kernels fail-closed (CWE-20)", () => {
    const bq = Biquad.lowpass(0.2);
    assert.throws(() => bq.process([]), EmptySignalError);

    assert.throws(() => sosfilt([], []), EmptySignalError);
    assert.throws(() => sosfilt([[1, 0, 0, 1, 0, 0]], []), EmptySignalError);

    assert.throws(() => lfilter([], [1], [1]), EmptyKernelError);
    assert.throws(() => lfilter([1], [], [1]), EmptyKernelError);
    assert.throws(() => lfilter([1], [1], []), EmptySignalError);

    assert.throws(() => fir([], [1]), EmptySignalError);
    assert.throws(() => fir([1], []), EmptyKernelError);
    assert.throws(() => firFft([], [1]), EmptySignalError);
    assert.throws(() => firFft([1], []), EmptyKernelError);
  });

  it("rejects non-finite floats fail-closed (CWE-20)", () => {
    assert.throws(() => new Biquad(NaN, 0, 0, 1, 0, 0), InvalidCoefficientError);
    assert.throws(() => new Biquad(0, 0, 0, 0, 0, 0), InvalidCoefficientError);

    const bq = Biquad.lowpass(0.2);
    assert.throws(() => bq.processSample(Infinity), InvalidCoefficientError);
    assert.throws(() => bq.processSample(NaN), InvalidCoefficientError);
    assert.throws(() => bq.process([1.0, NaN]), InvalidCoefficientError);

    assert.throws(() => sosfilt([[1, 0, 0, 1, 0, 0]], [NaN]), InvalidCoefficientError);
    assert.throws(() => sosfilt([[1, 0, 0, 1, 0]], [1.0]), InvalidCoefficientError); // sec length != 6

    assert.throws(() => lfilter([NaN], [1], [1]), InvalidCoefficientError);
    assert.throws(() => lfilter([1], [Infinity], [1]), InvalidCoefficientError);
    assert.throws(() => lfilter([1], [1.0, NaN], [1]), InvalidCoefficientError);
    assert.throws(() => lfilter([1], [1], [-Infinity]), InvalidCoefficientError);

    assert.throws(() => fir([NaN], [1]), InvalidCoefficientError);
    assert.throws(() => fir([1], [Infinity]), InvalidCoefficientError);
    assert.throws(() => firFft([NaN], [1]), InvalidCoefficientError);
    assert.throws(() => firFft([1], [Infinity]), InvalidCoefficientError);
  });

  it("enforces pre-flight bounds checks without allocating (CWE-400)", () => {
    const fakeHugeSignal = {
      length: MAX_SIGNAL_LEN + 1,
      [0]: 1.0,
    };
    const bq = Biquad.lowpass(0.2);
    assert.throws(() => bq.process(fakeHugeSignal), InvalidCoefficientError);
    assert.throws(() => sosfilt([[1, 0, 0, 1, 0, 0]], fakeHugeSignal), InvalidCoefficientError);
    assert.throws(() => lfilter([1], [1], fakeHugeSignal), InvalidCoefficientError);
    assert.throws(() => fir(fakeHugeSignal, [1]), InvalidCoefficientError);
    assert.throws(() => firFft(fakeHugeSignal, [1]), InvalidCoefficientError);

    const fakeHugeTaps = {
      length: MAX_FILTER_TAPS + 1,
      [0]: 1.0,
    };
    assert.throws(() => lfilter(fakeHugeTaps, [1], [1]), InvalidCoefficientError);
    assert.throws(() => fir([1], fakeHugeTaps), InvalidCoefficientError);
    assert.throws(() => firFft([1], fakeHugeTaps), InvalidCoefficientError);
    assert.throws(() => designLowPass(0.2, MAX_FILTER_TAPS + 1), InvalidCoefficientError);
    assert.throws(() => designBandPass(0.1, 0.3, MAX_FILTER_TAPS + 1), InvalidCoefficientError);

    const fakeHugeSos = new Array(MAX_ORDER + 1).fill([1, 0, 0, 1, 0, 0]);
    assert.throws(() => sosfilt(fakeHugeSos, [1, 2]), InvalidCoefficientError);

    // outLen > MAX_SIGNAL_LEN
    assert.throws(
      () => fir({ length: MAX_SIGNAL_LEN, [0]: 1.0 }, [1.0, 2.0]),
      InvalidCoefficientError
    );
    // order > MAX_ORDER in lfilter
    assert.throws(
      () => lfilter({ length: MAX_ORDER + 2, [0]: 1.0 }, [1.0], [1.0]),
      InvalidCoefficientError
    );
  });

  it("handles error types and unknown window names", () => {
    const err = new FftError("fft fail");
    assert.equal(err.name, "FftError");
    assert.throws(
      () => designLowPass(0.1, 15, "unknown_window"),
      Error
    );
  });

  it("rejects out-of-range biquad helper parameters fail-closed", () => {
    assert.throws(() => Biquad.lowpass(-0.1), InvalidCoefficientError);
    assert.throws(() => Biquad.lowpass(0.6), InvalidCoefficientError);
    assert.throws(() => Biquad.lowpass(0.2, -1.0), InvalidCoefficientError);

    assert.throws(() => Biquad.highpass(-0.1), InvalidCoefficientError);
    assert.throws(() => Biquad.highpass(0.6), InvalidCoefficientError);
    assert.throws(() => Biquad.highpass(0.2, 0.0), InvalidCoefficientError);

    assert.throws(() => Biquad.bandpass(-0.1), InvalidCoefficientError);
    assert.throws(() => Biquad.bandpass(0.2, -1.0), InvalidCoefficientError);

    assert.throws(() => Biquad.notch(-0.1), InvalidCoefficientError);
    assert.throws(() => Biquad.notch(0.2, 0.0), InvalidCoefficientError);

    assert.throws(() => Biquad.allpass(-0.1), InvalidCoefficientError);
    assert.throws(() => Biquad.allpass(0.2, -0.5), InvalidCoefficientError);

    assert.throws(() => Biquad.peak(-0.1, 3.0), InvalidCoefficientError);
    assert.throws(() => Biquad.peak(0.2, NaN), InvalidCoefficientError);
    assert.throws(() => Biquad.peak(0.2, 3.0, 0.0), InvalidCoefficientError);
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

  it("enforces fail-closed bounds and finiteness on filtfilt (CWE-400/CWE-20)", () => {
    assert.throws(() => filtfilt([], [1], [1]), EmptyKernelError);
    assert.throws(() => filtfilt([1], [], [1]), EmptyKernelError);
    assert.throws(() => filtfilt([1], [1], []), EmptySignalError);
    assert.throws(() => filtfilt([1], [0], [1]), InvalidCoefficientError);
    assert.throws(() => filtfilt([NaN], [1], [1]), InvalidCoefficientError);
    assert.throws(() => filtfilt([1], [Infinity], [1]), InvalidCoefficientError);
    assert.throws(() => filtfilt([1], [1], [NaN]), InvalidCoefficientError);

    const fakeHugeSignal = { length: MAX_SIGNAL_LEN + 1, [0]: 1.0 };
    const fakeHugeTaps = { length: MAX_FILTER_TAPS + 1, [0]: 1.0 };
    assert.throws(() => filtfilt(fakeHugeTaps, [1], [1]), InvalidCoefficientError);
    assert.throws(() => filtfilt([1], fakeHugeTaps, [1]), InvalidCoefficientError);
    assert.throws(() => filtfilt([1], [1], fakeHugeSignal), InvalidCoefficientError);

    // order > MAX_ORDER in filtfilt
    const fakeHugeOrder = { length: MAX_ORDER + 2, [0]: 1.0 };
    assert.throws(() => filtfilt(fakeHugeOrder, [1], [1]), InvalidCoefficientError);

    // n + 2 * padLen > MAX_SIGNAL_LEN
    const nearLimitSignal = { length: MAX_SIGNAL_LEN - 1, [0]: 1.0 };
    assert.throws(() => filtfilt([1, 1], [1, 1], nearLimitSignal), InvalidCoefficientError);
  });
});
