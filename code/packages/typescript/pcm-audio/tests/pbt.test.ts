import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

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
  const prng = new SplitMix64(0x1337_cafe_0001n);
  for (let trial = 0; trial < 200; trial += 1) {
    const v = prng.bounded(200_001) - 100_000;
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
  const prng = new SplitMix64(0x2468_ace0_1357n);
  for (let trial = 0; trial < 50; trial += 1) {
    const samples = prng.bounded(50_001);
    const dataSize = computeDataSize(samples, 1, 2);
    assert.equal(dataSize, samples * 2);
    const fileSize = computeWavFileSize(dataSize);
    assert.equal(fileSize, 44 + dataSize);
  }
});

test("PBT: floatToPcm16 boundedness and clipping correctness", () => {
  const prng = new SplitMix64(0xdead_beef_9999n);
  for (let trial = 0; trial < 200; trial += 1) {
    const u = Number(prng.nextU64() >> 11n) * (1.0 / Number(1n << 53n));
    const sample = (u * 4.0) - 2.0; // [-2.0, 2.0)
    const [pcm, clipped] = floatToPcm16(sample);

    assert.ok(pcm >= PCM16_MIN && pcm <= PCM16_MAX);
    if (sample < -1.0 || sample > 1.0) {
      assert.equal(clipped, true);
    } else {
      assert.equal(clipped, false);
    }
  }
});

test("PBT (SplitMix64): WAV roundtrip idempotence across generated sample lists", () => {
  const prng = new SplitMix64(0xfeed_face_4242n);
  const sampleSpan = PCM16_MAX - PCM16_MIN + 1;

  for (let trial = 0; trial < 25; trial += 1) {
    const len = prng.bounded(256);
    const pattern: number[] = [];
    for (let i = 0; i < len; i += 1) {
      pattern.push(PCM16_MIN + prng.bounded(sampleSpan));
    }
    const orig = new PCMBuffer(pattern, new PCMFormat(48_000));
    const wavBytes = toWavBytes(orig);
    const recovered = fromWavBytes(wavBytes);

    assert.deepEqual(Array.from(recovered.samples), pattern);
    assert.equal(recovered.pcmFormat.sampleRateHz, 48_000);
  }
});
