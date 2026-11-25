import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

import {
  AudioSinkError,
  frameCount,
  MAX_SAMPLE_RATE_HZ,
  NoopAudioSink,
  PCM16_MAX,
  PCM16_MIN,
  PcmFormat,
  PcmPlaybackBuffer,
  validateFormat,
} from "../src/index.js";

test("PBT (SplitMix64): validateFormat agrees with PcmFormat constructor", () => {
  const prng = new SplitMix64(0x1122_3344_5566_7788n);
  for (let trial = 0; trial < 100; trial += 1) {
    const rate = 1 + prng.bounded(MAX_SAMPLE_RATE_HZ);
    assert.equal(validateFormat(rate, 1, 16), true);
    const fmt = new PcmFormat(rate, 1, 16);
    assert.equal(fmt.sampleWidthBytes(), 2);
  }
});

test("PBT (SplitMix64): PcmPlaybackBuffer preserves samples, frameCount, and duration invariant", () => {
  const prng = new SplitMix64(0xfeed_face_cafe_baben);
  const sampleSpan = PCM16_MAX - PCM16_MIN + 1;

  for (let trial = 0; trial < 60; trial += 1) {
    const rate = 8_000 + prng.bounded(88_001);
    const len = prng.bounded(256);
    const input: number[] = [];
    for (let i = 0; i < len; i += 1) {
      input.push(PCM16_MIN + prng.bounded(sampleSpan));
    }

    const format = new PcmFormat(rate, 1, 16);
    const buffer = new PcmPlaybackBuffer(input, format);
    const report = new NoopAudioSink("pbt-splitmix").playBlocking(buffer);

    assert.equal(buffer.sampleCount(), len);
    assert.equal(buffer.frameCount(), frameCount(len, 1));
    assert.equal(buffer.isEmpty(), len === 0);
    assert.deepEqual(Array.from(buffer.samples), input);
    assert.equal(report.framesPlayed, len);
    assert.equal(report.sampleRateHz, rate);
    assert.equal(report.channelCount, 1);
    assert.ok(Math.abs(report.durationSeconds * rate - len) < 1e-9);
  }
});

test("PBT (SplitMix64): out-of-range PCM16 samples fail closed with AudioSinkError", () => {
  const prng = new SplitMix64(0xdead_c0de_1337_0001n);
  const format = new PcmFormat(44_100, 1, 16);

  for (let trial = 0; trial < 60; trial += 1) {
    const delta = 1 + prng.bounded(100_000);
    const badSample = trial % 2 === 0 ? PCM16_MAX + delta : PCM16_MIN - delta;

    assert.throws(
      () => new PcmPlaybackBuffer([badSample], format),
      (err: unknown) => err instanceof AudioSinkError && err.kind === "invalid_samples",
    );
  }
});
