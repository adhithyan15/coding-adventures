import assert from "node:assert/strict";
import test from "node:test";

import {
  DEFAULT_BIT_DEPTH,
  DEFAULT_CHANNEL_COUNT,
  DEFAULT_FULL_SCALE_VOLTAGE,
  DEFAULT_SAMPLE_RATE_HZ,
  PCM16_MAX,
  PCM16_MIN,
  PCMBuffer,
  PCMFormat,
  encodeSampleBuffer,
  floatToPcm16,
  SampleBufferLike,
  samplesToPcmBuffer,
} from "../src/index.js";

test("versioned defaults are visible", () => {
  assert.equal(DEFAULT_SAMPLE_RATE_HZ, 44_100.0);
  assert.equal(DEFAULT_BIT_DEPTH, 16);
  assert.equal(DEFAULT_CHANNEL_COUNT, 1);
  assert.equal(DEFAULT_FULL_SCALE_VOLTAGE, 1.0);
  assert.equal(PCM16_MIN, -32_768);
  assert.equal(PCM16_MAX, 32_767);
});

test("PCMFormat validates V1 shape", () => {
  const format = new PCMFormat(8.0, 1, 16, 2.0);
  assert.equal(format.sampleRateHz, 8.0);
  assert.equal(format.minimumInteger, PCM16_MIN);
  assert.equal(format.maximumInteger, PCM16_MAX);
  assert.equal(format.sampleWidthBytes, 2);
  assert.equal(format.integerSampleRate(), 8);
});

test("PCMFormat rejects invalid shapes", () => {
  assert.throws(() => new PCMFormat(0.0), /sampleRateHz/i);
  assert.throws(() => new PCMFormat(8.0, 2), /mono/i);
  assert.throws(() => new PCMFormat(8.0, 1, 24), /16-bit/i);
  assert.throws(() => new PCMFormat(8.0, 1, 16, 0.0), /fullScaleVoltage/i);
});

test("PCMFormat rejects non-integer sample rates through integerSampleRate", () => {
  assert.throws(() => new PCMFormat(44_100.5).integerSampleRate(), /integer-valued/i);
});

test("floatToPcm16 clips and reports clipping", () => {
  assert.deepEqual(floatToPcm16(0.0), [0, false]);
  assert.deepEqual(floatToPcm16(1.0), [32_767, false]);
  assert.deepEqual(floatToPcm16(-1.0), [-32_768, false]);
  assert.deepEqual(floatToPcm16(2.0), [32_767, true]);
  assert.deepEqual(floatToPcm16(-2.0), [-32_768, true]);
});

test("floatToPcm16 rejects non-finite or non-number", () => {
  assert.throws(() => floatToPcm16(Number.NaN), /finite real/);
  assert.throws(() => floatToPcm16(Number.POSITIVE_INFINITY), /finite real/);
  // @ts-expect-error deliberate negative test
  assert.throws(() => floatToPcm16(undefined), /finite real/);
});

test("encodeSampleBuffer tracks clipping and timing", () => {
  const sampleBuffer: SampleBufferLike = {
    samples: [0.0, 1.0, -1.0, 2.0],
    sampleRateHz: 4.0,
    startTimeSeconds: 10.0,
  };
  const pcm = encodeSampleBuffer(sampleBuffer);
  assert.deepEqual(Array.from(pcm.samples), [0, 32767, -32768, 32767]);
  assert.equal(pcm.clippedSampleCount, 1);
  assert.equal(pcm.sampleCount(), 4);
  assert.equal(pcm.samplePeriodSeconds(), 0.25);
  assert.equal(pcm.durationSeconds(), 1.0);
  assert.equal(pcm.timeAt(2), 10.5);
});

test("samplesToPcmBuffer supports raw float sequences", () => {
  const pcm = samplesToPcmBuffer([0.0, 0.5, -0.5], { sampleRateHz: 3.0 });
  assert.deepEqual(Array.from(pcm.samples), [0, 16384, -16384]);
  assert.equal(pcm.sampleCount(), 3);
  assert.equal(pcm.durationSeconds(), 1.0);
});

test("PCMBuffer packs little-endian bytes", () => {
  const pcm = new PCMBuffer([0, 32767, -32768], new PCMFormat(3.0));
  assert.deepEqual(Array.from(pcm.toLittleEndianBytes()), [0, 0, 255, 127, 0, 128]);
});

test("PCMBuffer rejects invalid integer samples", () => {
  assert.throws(
    () => new PCMBuffer([32768], new PCMFormat(8.0)),
    /signed 16-bit/i,
  );
});

test("PCMBuffer rejects invalid metadata", () => {
  assert.throws(() => new PCMBuffer([0], {} as PCMFormat), /PCMFormat/i);
  assert.throws(() => new PCMBuffer([0], new PCMFormat(8.0), Number.NaN), /finite real/);
  assert.throws(
    () => new PCMBuffer([0], new PCMFormat(8.0), 0.0, -1),
    /clippedSampleCount/i,
  );
});

test("PCMBuffer validates index bounds for time lookup", () => {
  const pcm = new PCMBuffer([0], new PCMFormat(8.0));
  assert.throws(() => pcm.timeAt(1), /in \[0, 1\)/);
  assert.throws(() => pcm.timeAt(0.5), /integer/);
});
