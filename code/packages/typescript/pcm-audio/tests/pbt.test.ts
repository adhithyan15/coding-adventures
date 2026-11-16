import assert from "node:assert/strict";
import test from "node:test";

import {
  clampToPcm16,
  computeDataSize,
  computeWavFileSize,
  floatToPcm16,
  fromWavBytes,
  PCM16_MAX,
  PCM16_MIN,
  PCMBuffer,
  PCMFormat,
  toWavBytes,
} from "../src/index.js";

test("PBT: clampToPcm16 bounded in [PCM16_MIN, PCM16_MAX]", () => {
  // Check wide range of inputs
  for (let v = -70_000; v <= 70_000; v += 137) {
    const clamped = clampToPcm16(v);
    assert.ok(clamped >= PCM16_MIN && clamped <= PCM16_MAX);
    if (v >= PCM16_MIN && v <= PCM16_MAX) {
      assert.equal(clamped, v);
    } else if (v > PCM16_MAX) {
      assert.equal(clamped, PCM16_MAX);
    } else {
      assert.equal(clamped, PCM16_MIN);
    }
  }
});

test("PBT: computeWavFileSize and computeDataSize mathematical identities", () => {
  for (let samples = 0; samples <= 10_000; samples += 250) {
    const dataSize = computeDataSize(samples, 1, 2);
    assert.equal(dataSize, samples * 2);
    const fileSize = computeWavFileSize(dataSize);
    assert.equal(fileSize, 44 + dataSize);
  }
});

test("PBT: floatToPcm16 boundedness and clipping correctness", () => {
  for (let i = -200; i <= 200; i += 1) {
    const sample = i * 0.01;
    const [pcm, clipped] = floatToPcm16(sample);

    assert.ok(pcm >= PCM16_MIN && pcm <= PCM16_MAX);
    if (sample < -1.0 || sample > 1.0) {
      assert.equal(clipped, true);
    } else {
      assert.equal(clipped, false);
    }
  }
});

test("PBT: WAV roundtrip idempotence across arbitrary sample lists", () => {
  const testPatterns = [
    [0],
    [-32768, 32767],
    [0, 100, -100, 200, -200, 300, -300],
    Array.from({ length: 128 }, (_, i) => Math.trunc(Math.sin(i * 0.1) * 30000)),
  ];

  for (const pattern of testPatterns) {
    const orig = new PCMBuffer(pattern, new PCMFormat(48_000));
    const wavBytes = toWavBytes(orig);
    const recovered = fromWavBytes(wavBytes);

    assert.deepEqual(Array.from(recovered.samples), pattern);
    assert.equal(recovered.pcmFormat.sampleRateHz, 48_000);
  }
});
