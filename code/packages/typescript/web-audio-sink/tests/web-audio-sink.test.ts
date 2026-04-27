import assert from "node:assert/strict";
import test from "node:test";

import {
  createAudioBufferFromPcm,
  pcmSamplesToFloat32,
  playPcmBuffer,
  validatePcmBuffer,
  type AudioBufferLike,
  type AudioBufferSourceNodeLike,
  type AudioContextLike,
  type GainNodeLike,
} from "../src/index.js";

class FakeAudioBuffer implements AudioBufferLike {
  readonly numberOfChannels: number;
  readonly length: number;
  readonly sampleRate: number;
  readonly data: Float32Array[];

  constructor(numberOfChannels: number, length: number, sampleRate: number) {
    this.numberOfChannels = numberOfChannels;
    this.length = length;
    this.sampleRate = sampleRate;
    this.data = Array.from(
      { length: numberOfChannels },
      () => new Float32Array(length),
    );
  }

  getChannelData(channel: number): Float32Array {
    return this.data[channel];
  }
}

class FakeGainNode implements GainNodeLike {
  readonly gain = { value: 1.0 };
  readonly connections: unknown[] = [];

  connect(destination: unknown): unknown {
    this.connections.push(destination);
    return destination;
  }
}

class FakeSourceNode implements AudioBufferSourceNodeLike {
  buffer: AudioBufferLike | null = null;
  onended: ((this: unknown, event: Event) => void) | null = null;
  readonly connections: unknown[] = [];
  readonly startTimes: number[] = [];

  connect(destination: unknown): unknown {
    this.connections.push(destination);
    return destination;
  }

  start(when?: number): void {
    this.startTimes.push(when ?? 0);
    this.onended?.(new Event("ended"));
  }
}

class FakeAudioContext implements AudioContextLike {
  readonly sampleRate = 48_000;
  readonly currentTime = 12.5;
  readonly destination = { kind: "speaker" };
  state = "running";
  resumeCount = 0;
  closeCount = 0;
  readonly buffers: FakeAudioBuffer[] = [];
  readonly sources: FakeSourceNode[] = [];
  readonly gains: FakeGainNode[] = [];

  createBuffer(
    numberOfChannels: number,
    length: number,
    sampleRate: number,
  ): FakeAudioBuffer {
    const buffer = new FakeAudioBuffer(numberOfChannels, length, sampleRate);
    this.buffers.push(buffer);
    return buffer;
  }

  createBufferSource(): FakeSourceNode {
    const source = new FakeSourceNode();
    this.sources.push(source);
    return source;
  }

  createGain(): FakeGainNode {
    const gain = new FakeGainNode();
    this.gains.push(gain);
    return gain;
  }

  async resume(): Promise<void> {
    this.resumeCount += 1;
    this.state = "running";
  }

  async close(): Promise<void> {
    this.closeCount += 1;
  }
}

test("validatePcmBuffer normalizes omitted mono 16-bit defaults", () => {
  assert.deepEqual(
    validatePcmBuffer({
      samples: [0, 1, -1],
      format: { sampleRateHz: 44_100 },
    }),
    { sampleRateHz: 44_100, channelCount: 1, bitDepth: 16 },
  );
});

test("validatePcmBuffer rejects unsupported formats and samples", () => {
  assert.throws(
    () =>
      validatePcmBuffer({
        samples: [0],
        format: { sampleRateHz: 0 },
      }),
    /sampleRateHz/,
  );

  assert.throws(
    () =>
      validatePcmBuffer({
        samples: [0],
        format: { sampleRateHz: 44_100, channelCount: 2 },
      }),
    /channelCount/,
  );

  assert.throws(
    () =>
      validatePcmBuffer({
        samples: [32_768],
        format: { sampleRateHz: 44_100 },
      }),
    /signed 16-bit/,
  );
});

test("pcmSamplesToFloat32 converts signed 16-bit PCM to floats", () => {
  const floats = pcmSamplesToFloat32(
    new Int16Array([-32768, -16384, 0, 16384, 32767]),
    0.5,
  );

  assert.deepEqual(Array.from(floats.slice(0, 3)), [-0.5, -0.25, 0]);
  assert.ok(Math.abs(floats[3] - 0.250007629627369) < 0.00000001);
  assert.equal(floats[4], 0.5);
});

test("pcmSamplesToFloat32 rejects invalid gain values", () => {
  assert.throws(() => pcmSamplesToFloat32([0], 1.1), /gain/);
  assert.throws(() => pcmSamplesToFloat32([0], Number.NaN), /gain/);
});

test("createAudioBufferFromPcm creates a mono AudioBuffer", () => {
  const audioContext = new FakeAudioContext();
  const audioBuffer = createAudioBufferFromPcm(
    audioContext,
    {
      samples: [0, 32767, -32768],
      format: { sampleRateHz: 22_050 },
    },
    { gain: 0.25 },
  );

  assert.equal(audioBuffer.numberOfChannels, 1);
  assert.equal(audioBuffer.length, 3);
  assert.equal(audioBuffer.sampleRate, 22_050);
  assert.deepEqual(Array.from(audioBuffer.getChannelData(0)), [0, 0.25, -0.25]);
});

test("playPcmBuffer schedules a source and resolves a report", async () => {
  const audioContext = new FakeAudioContext();

  const report = await playPcmBuffer(
    {
      samples: [0, 32767, 0, -32768],
      format: { sampleRateHz: 4 },
    },
    { audioContext, gain: 0.1, startTimeSeconds: 20.0 },
  );

  assert.deepEqual(report, {
    framesPlayed: 4,
    sampleRateHz: 4,
    channelCount: 1,
    durationSeconds: 1,
    backendName: "web-audio",
  });
  assert.equal(audioContext.sources.length, 1);
  assert.deepEqual(audioContext.sources[0].startTimes, [20.0]);
  assert.equal(audioContext.sources[0].buffer, audioContext.buffers[0]);
  assert.equal(audioContext.gains[0].gain.value, 0.1);
  assert.deepEqual(audioContext.gains[0].connections, [audioContext.destination]);
});

test("playPcmBuffer resumes suspended contexts and can close them", async () => {
  const audioContext = new FakeAudioContext();
  audioContext.state = "suspended";

  await playPcmBuffer(
    {
      samples: [0, 1],
      format: { sampleRateHz: 2 },
    },
    { audioContext, closeContextWhenDone: true },
  );

  assert.equal(audioContext.resumeCount, 1);
  assert.equal(audioContext.closeCount, 1);
});

test("playPcmBuffer returns immediately for empty buffers", async () => {
  await assert.rejects(
    playPcmBuffer({
      samples: [1],
      format: { sampleRateHz: 44_100 },
    }),
    /Web Audio API is not available/,
  );

  assert.deepEqual(
    await playPcmBuffer({
      samples: [],
      format: { sampleRateHz: 44_100 },
    }),
    {
      framesPlayed: 0,
      sampleRateHz: 44_100,
      channelCount: 1,
      durationSeconds: 0,
      backendName: "web-audio",
    },
  );
});
