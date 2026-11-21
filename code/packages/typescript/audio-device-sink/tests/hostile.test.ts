import assert from "node:assert/strict";
import test from "node:test";

import {
  AudioSinkError,
  frameCount,
  MAX_BLOCKING_DURATION_SECONDS,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  NoopAudioSink,
  PCM16_MAX,
  PCM16_MIN,
  PcmFormat,
  PcmPlaybackBuffer,
  validateFormat,
} from "../src/index.js";

test("audio-device-sink.warrant operations and boundary checks", () => {
  assert.equal(validateFormat(44_100, 1, 16), true);
  assert.equal(validateFormat(1, 1, 16), true);
  assert.equal(validateFormat(MAX_SAMPLE_RATE_HZ, 1, 16), true);
  assert.equal(validateFormat(0, 1, 16), false);
  assert.equal(validateFormat(-1, 1, 16), false);
  assert.equal(validateFormat(MAX_SAMPLE_RATE_HZ + 1, 1, 16), false);
  assert.equal(validateFormat(44_100, 2, 16), false);
  assert.equal(validateFormat(44_100, 1, 24), false);
  assert.equal(validateFormat(44_100.5, 1, 16), false);
  assert.equal(validateFormat(Number.MAX_SAFE_INTEGER + 1, 1, 16), false);

  assert.equal(frameCount(44_100, 1), 44_100);
  assert.equal(frameCount(0, 1), 0);
  assert.throws(() => frameCount(-1, 1), AudioSinkError);
  assert.throws(() => frameCount(1.5, 1), AudioSinkError);
  assert.throws(() => frameCount(MAX_SAMPLES + 1, 1), AudioSinkError);
  assert.throws(() => frameCount(100, 0), AudioSinkError);
  assert.throws(() => frameCount(100, 2), AudioSinkError);
});

test("CWE-20 / CWE-190: rejects unsafe integers, forged formats, and invalid buffers", () => {
  assert.throws(() => new PcmFormat(Number.MAX_SAFE_INTEGER + 1, 1, 16), AudioSinkError);
  assert.throws(() => new PcmFormat(44_100, Number.MAX_SAFE_INTEGER + 1, 16), AudioSinkError);
  assert.throws(() => new PcmFormat(44_100, 1, Number.MAX_SAFE_INTEGER + 1), AudioSinkError);

  const validFormat = new PcmFormat(44_100, 1, 16);
  // @ts-expect-error testing non-PcmFormat object
  assert.throws(() => new PcmPlaybackBuffer([0], { sampleRateHz: 44_100, channelCount: 1, bitDepth: 16 }), AudioSinkError);
  // @ts-expect-error testing string iterable rejection
  assert.throws(() => new PcmPlaybackBuffer("0123", validFormat), AudioSinkError);
  // @ts-expect-error testing null samples rejection
  assert.throws(() => new PcmPlaybackBuffer(null, validFormat), AudioSinkError);

  assert.throws(() => new PcmPlaybackBuffer([PCM16_MIN - 1], validFormat), AudioSinkError);
  assert.throws(() => new PcmPlaybackBuffer([PCM16_MAX + 1], validFormat), AudioSinkError);
  assert.throws(() => new PcmPlaybackBuffer([Number.MAX_SAFE_INTEGER + 1], validFormat), AudioSinkError);

  // Symbol and Object.create(null) must throw AudioSinkError cleanly rather than TypeError during string formatting
  assert.throws(
    // @ts-expect-error testing Symbol sample
    () => new PcmPlaybackBuffer([Symbol("x")], validFormat),
    (err: unknown) => err instanceof AudioSinkError && err.kind === "invalid_samples",
  );
  assert.throws(
    () => new PcmPlaybackBuffer([Object.create(null)], validFormat),
    (err: unknown) => err instanceof AudioSinkError && err.kind === "invalid_samples",
  );
  assert.doesNotThrow(() => new AudioSinkError("backend_failure", Object.create(null)));
  assert.throws(
    // @ts-expect-error testing Symbol kind
    () => new AudioSinkError(Symbol("x"), "msg"),
    (err: unknown) => err instanceof Error && !(err instanceof TypeError),
  );
  assert.throws(
    () => new AudioSinkError(Object.create(null), "msg"),
    (err: unknown) => err instanceof Error && !(err instanceof TypeError),
  );
  assert.throws(
    () => frameCount(10, Object.create(null)),
    (err: unknown) => err instanceof AudioSinkError && err.kind === "invalid_format",
  );

  const sink = new NoopAudioSink("hostile-check");
  // @ts-expect-error testing non-buffer input
  assert.throws(() => sink.playBlocking(null), AudioSinkError);
  // @ts-expect-error testing forged buffer object
  assert.throws(() => sink.playBlocking({ samples: new Int16Array(1), format: validFormat }), AudioSinkError);
});

test("CWE-400: pre-flight bounds on Int16Array, Array, and error/backendName lengths", () => {
  const lowRateFormat = new PcmFormat(1, 1, 16); // max = 600 samples
  const hugeTyped = new Int16Array(MAX_BLOCKING_DURATION_SECONDS + 1);
  assert.throws(() => new PcmPlaybackBuffer(hugeTyped, lowRateFormat), /limited/);
  const hugeInt32 = new Int32Array(MAX_BLOCKING_DURATION_SECONDS + 1);
  assert.throws(() => new PcmPlaybackBuffer(hugeInt32, lowRateFormat), /limited/);

  const longMsg = "x".repeat(5000);
  const err = new AudioSinkError("backend_failure", longMsg);
  assert.ok(err.message.length < 1100);

  // @ts-expect-error testing non-string error message bounding
  const nonStrErr = new AudioSinkError("backend_failure", { toString: () => longMsg });
  assert.ok(nonStrErr.message.length < 1100);

  const sink = new NoopAudioSink("a".repeat(1000));
  assert.equal(sink.backendName.length, 256);
});

test("CWE-1321: rejects prototype keys in error kinds and backendName", () => {
  for (const key of ["__proto__", "constructor", "prototype", "toString", "valueOf"]) {
    assert.throws(() => new NoopAudioSink(key), Error);
    // @ts-expect-error testing prototype key as error kind
    assert.throws(() => new AudioSinkError(key, "msg"), Error);
  }
});
