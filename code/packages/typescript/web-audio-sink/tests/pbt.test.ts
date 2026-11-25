import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

import {
  frameCount,
  INT16_MAX,
  INT16_MIN,
  MAX_SAMPLE_RATE_HZ,
  pcmSamplesToFloat32,
  playPcmBuffer,
  validateFormat,
} from "../src/index.js";

test("PBT (SplitMix64): validateFormat conformance across sample rate range", () => {
  const prng = new SplitMix64(0x51a0_0001_2026n);
  for (let trial = 0; trial < 100; trial += 1) {
    const rate = 1 + prng.bounded(MAX_SAMPLE_RATE_HZ);
    assert.equal(validateFormat(rate, 1, 16), true);
  }
  assert.equal(validateFormat(0, 1, 16), false);
  assert.equal(validateFormat(MAX_SAMPLE_RATE_HZ + 1, 1, 16), false);
});

test("PBT (SplitMix64): frameCount mono identity", () => {
  const prng = new SplitMix64(0x9876_5432_1000n);
  for (let trial = 0; trial < 100; trial += 1) {
    const len = prng.bounded(100_001);
    assert.equal(frameCount(len, 1), len);
  }
});

test("PBT (SplitMix64): pcmSamplesToFloat32 boundedness and gain scaling", () => {
  const prng = new SplitMix64(0xdead_beef_cafe_0002n);
  const sampleSpan = INT16_MAX - INT16_MIN + 1;

  for (let trial = 0; trial < 40; trial += 1) {
    const len = prng.bounded(128);
    const samples: number[] = [];
    for (let i = 0; i < len; i += 1) {
      samples.push(INT16_MIN + prng.bounded(sampleSpan));
    }

    const gain = Number(prng.nextU64() >> 11n) * (1.0 / Number(1n << 53n));
    const floatsFull = pcmSamplesToFloat32(samples, 1.0);
    const floatsScaled = pcmSamplesToFloat32(samples, gain);

    assert.equal(floatsFull.length, samples.length);
    for (let i = 0; i < samples.length; i += 1) {
      assert.ok(floatsFull[i] >= -1.0 && floatsFull[i] <= 1.0);
      assert.ok(floatsScaled[i] >= -gain - 1e-6 && floatsScaled[i] <= gain + 1e-6);
      assert.ok(Math.abs(floatsScaled[i] - floatsFull[i] * gain) < 1e-5);
    }
  }
});

test("PBT: empty buffer playback invariance", async () => {
  for (const rate of [8000, 22050, 44100, 48000, 96000]) {
    const report = await playPcmBuffer({
      samples: [],
      format: { sampleRateHz: rate },
    });
    assert.equal(report.framesPlayed, 0);
    assert.equal(report.durationSeconds, 0);
    assert.equal(report.sampleRateHz, rate);
    assert.equal(report.channelCount, 1);
  }
});
