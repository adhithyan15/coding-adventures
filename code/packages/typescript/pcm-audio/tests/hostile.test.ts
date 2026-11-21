import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

import {
  clampToPcm16,
  computeDataSize,
  computeWavFileSize,
  encodeSampleBuffer,
  fromWavBytes,
  MAX_CHANNELS,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  MAX_WAV_BYTES,
  PCM16_MAX,
  PCM16_MIN,
  PCMBuffer,
  PCMFormat,
  samplesToPcmBuffer,
  toWavBytes,
  validateWavHeader,
} from "../src/index.js";

test("CWE-20: clampToPcm16 rejects non-finite numbers", () => {
  assert.throws(() => clampToPcm16(Number.NaN), /finite/);
  assert.throws(() => clampToPcm16(Number.POSITIVE_INFINITY), /finite/);
  // @ts-expect-error test non-number
  assert.throws(() => clampToPcm16("32767"), /finite/);
});

test("CWE-20 / CWE-400: computeDataSize and computeWavFileSize bounds validation", () => {
  assert.throws(() => computeDataSize(-1, 1, 2), />= 0/);
  assert.throws(() => computeDataSize(1.5, 1, 2), /integer/);
  assert.throws(() => computeDataSize(MAX_SAMPLES + 1, 1, 2), /MAX_SAMPLES/);
  assert.throws(() => computeDataSize(100, 0, 2), /> 0/);
  assert.throws(() => computeDataSize(100, MAX_CHANNELS + 1, 2), /MAX_CHANNELS/);
  assert.throws(() => computeDataSize(100, 1, 0), /> 0/);
  assert.throws(() => computeDataSize(MAX_SAMPLES, 4, 2), /MAX_WAV_BYTES/);

  assert.throws(() => computeWavFileSize(-1), />= 0/);
  assert.throws(() => computeWavFileSize(1.5), /integer/);
  assert.throws(() => computeWavFileSize(MAX_WAV_BYTES), /MAX_WAV_BYTES/);
});

test("CWE-20 / CWE-400: PCMFormat rejects bad sample rate and bounds", () => {
  assert.throws(() => new PCMFormat(0), /> 0.0/);
  assert.throws(() => new PCMFormat(-100), /> 0.0/);
  assert.throws(() => new PCMFormat(Number.NaN), /finite/);
  assert.throws(() => new PCMFormat(MAX_SAMPLE_RATE_HZ + 1), /MAX_SAMPLE_RATE_HZ/);
});

test("CWE-20 / CWE-400: PCMBuffer rejects non-iterables, non-finite, and out of range samples", () => {
  // @ts-expect-error non-iterable
  assert.throws(() => new PCMBuffer(null, new PCMFormat(44_100)), /iterable/);
  assert.throws(() => new PCMBuffer([PCM16_MIN - 1], new PCMFormat(44_100)), /signed 16-bit/);
  assert.throws(() => new PCMBuffer([PCM16_MAX + 1], new PCMFormat(44_100)), /signed 16-bit/);
  assert.throws(() => new PCMBuffer([1.5], new PCMFormat(44_100)), /signed 16-bit/);
  // @ts-expect-error invalid format
  assert.throws(() => new PCMBuffer([0], {}), /PCMFormat/);
  assert.throws(() => new PCMBuffer([0], new PCMFormat(44_100), Number.NaN), /finite/);
  assert.throws(() => new PCMBuffer([0], new PCMFormat(44_100), 0, -1), />= 0/);
  assert.throws(() => new PCMBuffer([0], new PCMFormat(44_100), 0, 1.5), /integer/);

  const buf = new PCMBuffer([0, 10], new PCMFormat(44_100));
  assert.throws(() => buf.timeAt(-1), /index must be in/);
  assert.throws(() => buf.timeAt(2), /index must be in/);
  assert.throws(() => buf.timeAt(1.5), /integer/);
});

test("CWE-20: encodeSampleBuffer rejects invalid inputs", () => {
  // @ts-expect-error invalid sampleBuffer
  assert.throws(() => encodeSampleBuffer(null), /sampleBuffer must be an object/);
  // @ts-expect-error invalid sampleBuffer
  assert.throws(() => encodeSampleBuffer({ samples: null, sampleRateHz: 44100 }), /iterable/);
});

test("CWE-20: samplesToPcmBuffer rejects non-object options", () => {
  // @ts-expect-error invalid options
  assert.throws(() => samplesToPcmBuffer([0], null), /options must be an object/);
  // @ts-expect-error invalid options
  assert.throws(() => samplesToPcmBuffer([0], undefined), /options must be an object/);
});

test("CWE-20: toWavBytes rejects invalid pcmBuffer", () => {
  // @ts-expect-error invalid buffer
  assert.throws(() => toWavBytes(null), /instance of PCMBuffer/);
  // @ts-expect-error invalid buffer
  assert.throws(() => toWavBytes({}), /instance of PCMBuffer/);
});

test("CWE-20 / CWE-400: validateWavHeader hostile input rejection", () => {
  // @ts-expect-error non-Uint8Array
  assert.throws(() => validateWavHeader(null), /Uint8Array/);
  // @ts-expect-error non-Uint8Array
  assert.throws(() => validateWavHeader([0, 1, 2]), /Uint8Array/);

  // Too short
  assert.throws(() => validateWavHeader(new Uint8Array(43)), /smaller than minimum/);

  // Missing RIFF magic
  const badRiff = new Uint8Array(44);
  assert.throws(() => validateWavHeader(badRiff), /RIFF/);

  // Set RIFF
  badRiff[0] = 0x52;
  badRiff[1] = 0x49;
  badRiff[2] = 0x46;
  badRiff[3] = 0x46;
  const view = new DataView(badRiff.buffer);
  view.setUint32(4, 36, true);

  // Missing WAVE
  assert.throws(() => validateWavHeader(badRiff), /WAVE/);

  badRiff[8] = 0x57;
  badRiff[9] = 0x41;
  badRiff[10] = 0x56;
  badRiff[11] = 0x45;

  // Small RIFF payload
  view.setUint32(4, 20, true);
  assert.throws(() => validateWavHeader(badRiff), /payload size too small/);
  view.setUint32(4, 36, true);

  // Missing fmt chunk
  assert.throws(() => validateWavHeader(badRiff), /missing fmt chunk/);
});

test("validateWavHeader: handles chunk boundary overflow and fmt checks", () => {
  const buf = new Uint8Array(60);
  const view = new DataView(buf.buffer);
  // RIFF
  buf[0] = 0x52; buf[1] = 0x49; buf[2] = 0x46; buf[3] = 0x46;
  view.setUint32(4, 52, true);
  // WAVE
  buf[8] = 0x57; buf[9] = 0x41; buf[10] = 0x56; buf[11] = 0x45;
  // chunk with declared size that exceeds buffer
  buf[12] = 0x66; buf[13] = 0x6d; buf[14] = 0x74; buf[15] = 0x20; // "fmt "
  view.setUint32(16, 1000, true); // size 1000 > 60
  assert.throws(() => validateWavHeader(buf), /exceeds buffer boundary/);

  // fmt chunk size too small (< 16)
  view.setUint32(16, 8, true);
  assert.throws(() => validateWavHeader(buf), /fmt chunk size too small/);

  // fmt chunk size 16, bad audio format
  view.setUint32(16, 16, true);
  view.setUint16(20, 2, true); // format 2 (unsupported)
  assert.throws(() => validateWavHeader(buf), /unsupported WAV audio format/);

  // bad channel count: 0
  view.setUint16(20, 1, true); // format 1 (PCM)
  view.setUint16(22, 0, true); // 0 channels
  assert.throws(() => validateWavHeader(buf), /unsupported channel count/);

  // bad sample rate: 0
  view.setUint16(22, 1, true); // 1 channel
  view.setUint32(24, 0, true); // sampleRate = 0
  assert.throws(() => validateWavHeader(buf), /sample rate 0/);

  // sample rate > MAX_SAMPLE_RATE_HZ
  view.setUint32(24, MAX_SAMPLE_RATE_HZ + 1, true);
  assert.throws(() => validateWavHeader(buf), /outside/);
  view.setUint32(24, 44100, true);

  // bad bit depth for PCM: 24
  view.setUint16(34, 24, true);
  assert.throws(() => validateWavHeader(buf), /unsupported bit depth/);

  // bit depth 16, but inconsistent blockAlign
  view.setUint16(34, 16, true);
  view.setUint16(32, 4, true); // expected 2
  assert.throws(() => validateWavHeader(buf), /inconsistent block align/);
  view.setUint16(32, 2, true);

  // inconsistent byteRate
  view.setUint32(28, 100, true); // expected 44100 * 2 = 88200
  assert.throws(() => validateWavHeader(buf), /inconsistent byte rate/);
  view.setUint32(28, 88200, true);

  // data chunk before fmt chunk (test by putting data chunk first)
  const dataFirst = new Uint8Array(60);
  const dfView = new DataView(dataFirst.buffer);
  dataFirst[0] = 0x52; dataFirst[1] = 0x49; dataFirst[2] = 0x46; dataFirst[3] = 0x46;
  dfView.setUint32(4, 52, true);
  dataFirst[8] = 0x57; dataFirst[9] = 0x41; dataFirst[10] = 0x56; dataFirst[11] = 0x45;
  dataFirst[12] = 0x64; dataFirst[13] = 0x61; dataFirst[14] = 0x74; dataFirst[15] = 0x61; // "data"
  dfView.setUint32(16, 4, true);
  assert.throws(() => validateWavHeader(dataFirst), /data chunk before fmt chunk/);
});

test("fromWavBytes rejects non-mono WAVs", () => {
  const stereoWav = new Uint8Array(44);
  const view = new DataView(stereoWav.buffer);
  stereoWav[0] = 0x52;
  stereoWav[1] = 0x49;
  stereoWav[2] = 0x46;
  stereoWav[3] = 0x46;
  view.setUint32(4, 36, true);
  stereoWav[8] = 0x57;
  stereoWav[9] = 0x41;
  stereoWav[10] = 0x56;
  stereoWav[11] = 0x45;
  stereoWav[12] = 0x66;
  stereoWav[13] = 0x6d;
  stereoWav[14] = 0x74;
  stereoWav[15] = 0x20;
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true); // PCM
  view.setUint16(22, 2, true); // 2 channels (stereo!)
  view.setUint32(24, 44_100, true);
  view.setUint32(28, 44_100 * 4, true);
  view.setUint16(32, 4, true);
  view.setUint16(34, 16, true);
  stereoWav[36] = 0x64;
  stereoWav[37] = 0x61;
  stereoWav[38] = 0x74;
  stereoWav[39] = 0x61;
  view.setUint32(40, 0, true);

  assert.throws(() => fromWavBytes(stereoWav), /only mono WAV files/);
});

test("CWE-20 / CWE-400: SplitMix64 fuzzed malformed WAV bytes fail closed without panics", () => {
  const prng = new SplitMix64(0xbad_f00d_1111n);
  for (let trial = 0; trial < 100; trial += 1) {
    const len = prng.bounded(128);
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i += 1) {
      bytes[i] = prng.bounded(256);
    }
    try {
      const header = validateWavHeader(bytes);
      assert.ok(header.channels >= 1 && header.channels <= MAX_CHANNELS);
    } catch (err) {
      assert.ok(err instanceof Error);
    }
  }
});

test("CWE-190 / CWE-20 / CWE-400: unsafe integer, Symbol, null-prototype, and pre-flight TypedArray bounds", () => {
  const fmt = new PCMFormat(44_100);

  // Unsafe integers (> MAX_SAFE_INTEGER) must fail closed
  assert.throws(() => new PCMBuffer([0], fmt, 0, 1e100), TypeError);
  assert.throws(() => computeDataSize(1e100, 1, 2), TypeError);
  assert.throws(() => computeWavFileSize(1e100), TypeError);
  assert.throws(() => new PCMBuffer([0, 1], fmt).timeAt(1e100), TypeError);

  // Symbol and Object.create(null) must produce clean domain errors, not string-conversion TypeErrors
  // @ts-expect-error testing Symbol sample
  assert.throws(() => new PCMBuffer([Symbol("x")], fmt), RangeError);
  assert.throws(() => new PCMBuffer([Object.create(null)], fmt), RangeError);
  assert.throws(
    () => clampToPcm16(Object.create(null)),
    (err: unknown) => err instanceof TypeError && err.message.includes("finite number"),
  );
  assert.throws(
    // @ts-expect-error testing Symbol value
    () => clampToPcm16(Symbol("x")),
    (err: unknown) => err instanceof TypeError && err.message.includes("finite number"),
  );
  assert.throws(
    () => new PCMBuffer([0], fmt).timeAt(Object.create(null)),
    (err: unknown) => err instanceof TypeError && err.message.includes("integer"),
  );

  // O(1) pre-flight TypedArray / Array length checks exceed MAX_SAMPLES without looping
  assert.throws(() => new PCMBuffer(new Uint8Array(MAX_SAMPLES + 1), fmt), /MAX_SAMPLES/);
  assert.throws(
    () => encodeSampleBuffer({ samples: new Uint8Array(MAX_SAMPLES + 1), sampleRateHz: 44_100 }),
    /MAX_SAMPLES/,
  );
});
