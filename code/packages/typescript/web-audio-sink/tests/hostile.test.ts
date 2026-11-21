import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

import {
  createAudioBufferFromPcm,
  frameCount,
  INT16_MAX,
  INT16_MIN,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  pcmSamplesToFloat32,
  playPcmBuffer,
  validateFormat,
  validatePcmBuffer,
  type AudioContextLike,
} from "../src/index.js";

test("web-audio-sink.warrant operations", () => {
  // mono_44100: validate_format(44100, 1, 16) == Ok(true)
  assert.equal(validateFormat(44_100, 1, 16), true);
  // mono_frames: frame_count(44100, 1) == 44100
  assert.equal(frameCount(44_100, 1), 44_100);

  // Negative / hostile checks
  assert.equal(validateFormat(0, 1, 16), false);
  assert.equal(validateFormat(-1, 1, 16), false);
  assert.equal(validateFormat(MAX_SAMPLE_RATE_HZ + 1, 1, 16), false);
  assert.equal(validateFormat(44_100, 2, 16), false);
  assert.equal(validateFormat(44_100, 1, 24), false);
  assert.equal(validateFormat(44_100.5, 1, 16), false);

  assert.throws(() => frameCount(-1, 1), /invalid samplesLen/);
  assert.throws(() => frameCount(1.5, 1), /invalid samplesLen/);
  assert.throws(() => frameCount(MAX_SAMPLES + 1, 1), /invalid samplesLen/);
  assert.throws(() => frameCount(100, 0), /invalid samplesLen/);
  assert.throws(() => frameCount(100, -1), /invalid samplesLen/);
});

test("CWE-20: validatePcmBuffer rejects null, non-objects, and malformed buffers", () => {
  // @ts-expect-error test null
  assert.throws(() => validatePcmBuffer(null), /buffer must be an object/);
  // @ts-expect-error test string
  assert.throws(() => validatePcmBuffer("buffer"), /buffer must be an object/);
  // @ts-expect-error test bad samples
  assert.throws(() => validatePcmBuffer({ samples: null, format: { sampleRateHz: 44100 } }), /Int16Array or number array/);
  // @ts-expect-error test bad format
  assert.throws(() => validatePcmBuffer({ samples: [0], format: null }), /format must be an object/);

  assert.throws(
    () => validatePcmBuffer({ samples: [0], format: { sampleRateHz: MAX_SAMPLE_RATE_HZ + 1 } }),
    /MAX_SAMPLE_RATE_HZ/,
  );
  assert.throws(
    () => validatePcmBuffer({ samples: [0], format: { sampleRateHz: -44100 } }),
    /positive integer/,
  );
  assert.throws(
    () => validatePcmBuffer({ samples: [0], format: { sampleRateHz: 44100.5 } }),
    /positive integer/,
  );
  assert.throws(() => validatePcmBuffer({ samples: [0], format: { sampleRateHz: 44100, channelCount: 0 } }), /channelCount/);
  // @ts-expect-error bad bitDepth
  assert.throws(() => validatePcmBuffer({ samples: [0], format: { sampleRateHz: 44100, bitDepth: 32 } }), /bitDepth/);

  // Non-integer and out of bounds samples
  assert.throws(() => validatePcmBuffer({ samples: [1.5], format: { sampleRateHz: 44100 } }), /signed 16-bit/);
  // @ts-expect-error bad sample type
  assert.throws(() => validatePcmBuffer({ samples: ["0"], format: { sampleRateHz: 44100 } }), /signed 16-bit/);
  assert.throws(() => validatePcmBuffer({ samples: [INT16_MIN - 1], format: { sampleRateHz: 44100 } }), /signed 16-bit/);
  assert.throws(() => validatePcmBuffer({ samples: [INT16_MAX + 1], format: { sampleRateHz: 44100 } }), /signed 16-bit/);
});

test("CWE-20 / CWE-400: pcmSamplesToFloat32 rejects bad sample arrays and non-finite gains", () => {
  // @ts-expect-error non-array
  assert.throws(() => pcmSamplesToFloat32(null), /Int16Array or number array/);
  assert.throws(() => pcmSamplesToFloat32([0], -0.1), /gain/);
  assert.throws(() => pcmSamplesToFloat32([0], 1.5), /gain/);
  assert.throws(() => pcmSamplesToFloat32([0], Number.NaN), /gain/);
  assert.throws(() => pcmSamplesToFloat32([0], Number.POSITIVE_INFINITY), /gain/);

  // Non-integer sample in array
  assert.throws(() => pcmSamplesToFloat32([1.5]), /signed 16-bit/);
  assert.throws(() => pcmSamplesToFloat32([INT16_MAX + 10]), /signed 16-bit/);
});

test("CWE-20: createAudioBufferFromPcm rejects null or invalid audioContext", () => {
  // @ts-expect-error null audio context
  assert.throws(() => createAudioBufferFromPcm(null, { samples: [0], format: { sampleRateHz: 44100 } }), /AudioContext/);
  // @ts-expect-error non-object audio context
  assert.throws(() => createAudioBufferFromPcm("bad", { samples: [0], format: { sampleRateHz: 44100 } }), /AudioContext/);
});

test("playPcmBuffer rejects when source.start throws", async () => {
  const failingContext: AudioContextLike = {
    sampleRate: 44_100,
    currentTime: 0,
    destination: {},
    createBuffer(channels, length, sampleRate) {
      return {
        numberOfChannels: channels,
        length,
        sampleRate,
        getChannelData: () => new Float32Array(length),
      };
    },
    createBufferSource() {
      return {
        buffer: null,
        onended: null,
        connect() {},
        start() {
          throw new Error("simulated audio device failure");
        },
      };
    },
    createGain() {
      return {
        gain: { value: 1.0 },
        connect() {},
      };
    },
  };

  await assert.rejects(
    playPcmBuffer(
      { samples: [100], format: { sampleRateHz: 44_100 } },
      { audioContext: failingContext },
    ),
    /simulated audio device failure/,
  );
});

test("playPcmBuffer closeContextWhenDone without close method", async () => {
  const noCloseContext: AudioContextLike = {
    sampleRate: 44_100,
    currentTime: 0,
    destination: {},
    createBuffer(channels, length, sampleRate) {
      return {
        numberOfChannels: channels,
        length,
        sampleRate,
        getChannelData: () => new Float32Array(length),
      };
    },
    createBufferSource() {
      let onendedCb: ((this: unknown, event: Event) => void) | null = null;
      return {
        buffer: null,
        get onended() {
          return onendedCb;
        },
        set onended(val) {
          onendedCb = val;
        },
        connect() {},
        start() {
          onendedCb?.(new Event("ended"));
        },
      };
    },
    createGain() {
      return {
        gain: { value: 1.0 },
        connect() {},
      };
    },
  };

  const report = await playPcmBuffer(
    { samples: [100], format: { sampleRateHz: 44_100 } },
    { audioContext: noCloseContext, closeContextWhenDone: true },
  );
  assert.equal(report.framesPlayed, 1);
});

test("playPcmBuffer validates startTimeSeconds against hostile inputs", async () => {
  const mockContext: AudioContextLike = {
    sampleRate: 44_100,
    currentTime: 0,
    destination: {},
    createBuffer(channels, length, sampleRate) {
      return {
        numberOfChannels: channels,
        length,
        sampleRate,
        getChannelData: () => new Float32Array(length),
      };
    },
    createBufferSource() {
      return {
        buffer: null,
        onended: null,
        connect() {},
        start() {},
      };
    },
    createGain() {
      return {
        gain: { value: 1.0 },
        connect() {},
      };
    },
  };

  const buffer = { samples: [100], format: { sampleRateHz: 44_100 } };

  await assert.rejects(
    playPcmBuffer(buffer, { audioContext: mockContext, startTimeSeconds: Number.NaN }),
    RangeError,
  );
  await assert.rejects(
    playPcmBuffer(buffer, { audioContext: mockContext, startTimeSeconds: Number.POSITIVE_INFINITY }),
    RangeError,
  );
  await assert.rejects(
    playPcmBuffer(buffer, { audioContext: mockContext, startTimeSeconds: Number.NEGATIVE_INFINITY }),
    RangeError,
  );
  await assert.rejects(
    playPcmBuffer(buffer, { audioContext: mockContext, startTimeSeconds: -0.5 }),
    RangeError,
  );
  await assert.rejects(
    // @ts-expect-error non-number startTimeSeconds
    playPcmBuffer(buffer, { audioContext: mockContext, startTimeSeconds: "invalid" }),
    TypeError,
  );
});

test("playPcmBuffer applies gain exactly once without double-attenuation", async () => {
  let createdChannelData: Float32Array | null = null;
  let gainNodeValue = 1.0;

  const mockContext: AudioContextLike = {
    sampleRate: 44_100,
    currentTime: 1.0,
    destination: {},
    createBuffer(channels, length, sampleRate) {
      const channelData = new Float32Array(length);
      createdChannelData = channelData;
      return {
        numberOfChannels: channels,
        length,
        sampleRate,
        getChannelData: () => channelData,
        copyToChannel: (src: Float32Array) => {
          channelData.set(src);
        },
      };
    },
    createBufferSource() {
      let onendedCb: ((this: unknown, event: Event) => void) | null = null;
      return {
        buffer: null,
        get onended() {
          return onendedCb;
        },
        set onended(cb) {
          onendedCb = cb;
        },
        connect() {},
        start() {
          onendedCb?.(new Event("ended"));
        },
      };
    },
    createGain() {
      const gainObj = {
        get value() {
          return gainNodeValue;
        },
        set value(v: number) {
          gainNodeValue = v;
        },
      };
      return {
        gain: gainObj,
        connect() {},
      };
    },
  };

  // 32767 is max positive int16, normalized to ~1.0 in float
  const pcmBuffer = { samples: [32767], format: { sampleRateHz: 44_100 } };
  const requestedGain = 0.5;

  await playPcmBuffer(pcmBuffer, { audioContext: mockContext, gain: requestedGain });

  // GainNode must have the requested gain
  assert.equal(gainNodeValue, 0.5);
  // AudioBuffer must have unscaled unity gain (~1.0), NOT 0.5 (which would square the gain)
  assert.notEqual(createdChannelData, null);
  const sampleFloat = (createdChannelData as unknown as Float32Array)[0];
  assert.ok(Math.abs(sampleFloat - 1.0) < 1e-4);
});

test("CWE-20 / CWE-400: SplitMix64 hostile out-of-range PCM samples fail closed", () => {
  const prng = new SplitMix64(0xface_b00c_7777n);
  for (let trial = 0; trial < 50; trial += 1) {
    const delta = 1 + prng.bounded(100_000);
    const badSample = trial % 2 === 0 ? INT16_MAX + delta : INT16_MIN - delta;
    assert.throws(() => pcmSamplesToFloat32([badSample]), RangeError);
  }
});

test("CWE-190 / CWE-20: unsafe integers, non-mono frameCount, Symbol, and null-prototype inputs fail closed", () => {
  assert.equal(validateFormat(1e100, 1, 16), false);
  assert.equal(validateFormat(44_100, 1e100, 16), false);
  assert.equal(validateFormat(44_100, 1, 1e100), false);

  assert.throws(() => frameCount(100, 1e100), RangeError);
  assert.throws(() => frameCount(1e100, 1), RangeError);
  assert.throws(() => frameCount(100, 2), RangeError);

  assert.throws(
    () => validatePcmBuffer({ samples: [0], format: { sampleRateHz: 1e100 } }),
    RangeError,
  );
  assert.throws(
    () => validatePcmBuffer({ samples: [1e100], format: { sampleRateHz: 44_100 } }),
    RangeError,
  );
  assert.throws(
    // @ts-expect-error testing Symbol sample
    () => validatePcmBuffer({ samples: [Symbol("x")], format: { sampleRateHz: 44_100 } }),
    RangeError,
  );
  assert.throws(
    () => validatePcmBuffer({ samples: [Object.create(null)], format: { sampleRateHz: 44_100 } }),
    RangeError,
  );
  assert.throws(
    // @ts-expect-error testing Symbol sample
    () => pcmSamplesToFloat32([Symbol("x")]),
    RangeError,
  );
  assert.throws(
    () => pcmSamplesToFloat32([Object.create(null)]),
    RangeError,
  );
});
