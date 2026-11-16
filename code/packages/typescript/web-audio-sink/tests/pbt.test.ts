import assert from "node:assert/strict";
import test from "node:test";

import {
  frameCount,
  INT16_MAX,
  INT16_MIN,
  MAX_SAMPLE_RATE_HZ,
  pcmSamplesToFloat32,
  playPcmBuffer,
  validateFormat,
} from "../src/index.js";

test("PBT: validateFormat conformance across sample rate range", () => {
  for (let rate = 1; rate <= MAX_SAMPLE_RATE_HZ; rate += 19_199) {
    assert.equal(validateFormat(rate, 1, 16), true);
  }
  assert.equal(validateFormat(0, 1, 16), false);
  assert.equal(validateFormat(MAX_SAMPLE_RATE_HZ + 1, 1, 16), false);
});

test("PBT: frameCount mono identity", () => {
  for (let len = 0; len <= 100_000; len += 2500) {
    assert.equal(frameCount(len, 1), len);
  }
});

test("PBT: pcmSamplesToFloat32 boundedness and gain scaling", () => {
  const step = 257;
  const samples: number[] = [];
  for (let s = INT16_MIN; s <= INT16_MAX; s += step) {
    samples.push(s);
  }

  const floats1 = pcmSamplesToFloat32(samples, 1.0);
  const floatsHalf = pcmSamplesToFloat32(samples, 0.5);

  assert.equal(floats1.length, samples.length);
  for (let i = 0; i < samples.length; i += 1) {
    assert.ok(floats1[i] >= -1.0 && floats1[i] <= 1.0);
    assert.ok(floatsHalf[i] >= -0.5 && floatsHalf[i] <= 0.5);
    assert.ok(Math.abs(floatsHalf[i] - floats1[i] * 0.5) < 1e-6);
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
