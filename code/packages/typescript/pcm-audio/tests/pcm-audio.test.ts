import assert from "node:assert/strict";
import test from "node:test";

import {
  clampToPcm16,
  computeDataSize,
  computeWavFileSize,
  DEFAULT_BIT_DEPTH,
  DEFAULT_CHANNEL_COUNT,
  DEFAULT_FULL_SCALE_VOLTAGE,
  DEFAULT_SAMPLE_RATE_HZ,
  encodeSampleBuffer,
  floatToPcm16,
  fromWavBytes,
  MAX_CHANNELS,
  MAX_DURATION_SECONDS,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  MAX_WAV_BYTES,
  PCM16_MAX,
  PCM16_MIN,
  PCMBuffer,
  PCMFormat,
  SampleBufferLike,
  samplesToPcmBuffer,
  toWavBytes,
  validateWavHeader,
} from "../src/index.js";

test("versioned defaults and security bounds are visible", () => {
  assert.equal(DEFAULT_SAMPLE_RATE_HZ, 44_100.0);
  assert.equal(DEFAULT_BIT_DEPTH, 16);
  assert.equal(DEFAULT_CHANNEL_COUNT, 1);
  assert.equal(DEFAULT_FULL_SCALE_VOLTAGE, 1.0);
  assert.equal(PCM16_MIN, -32_768);
  assert.equal(PCM16_MAX, 32_767);
  assert.equal(MAX_SAMPLES, 16_777_216);
  assert.equal(MAX_SAMPLE_RATE_HZ, 192_000.0);
  assert.equal(MAX_DURATION_SECONDS, 60.0);
  assert.equal(MAX_CHANNELS, 8);
  assert.equal(MAX_WAV_BYTES, 67_108_864);
});

test("pcm-audio.warrant operation conformance", () => {
  // clamp_zero: clamp_to_pcm16(0) == 0
  assert.equal(clampToPcm16(0), 0);
  // clamp_max: clamp_to_pcm16(32767) == 32767
  assert.equal(clampToPcm16(32767), 32767);
  // clamp_overflow: clamp_to_pcm16(50000) == 32767
  assert.equal(clampToPcm16(50000), 32767);
  // clamp_min: clamp_to_pcm16(-32768) == -32768
  assert.equal(clampToPcm16(-32768), -32768);
  // clamp_underflow: clamp_to_pcm16(-60000) == -32768
  assert.equal(clampToPcm16(-60000), -32768);

  // data_size_mono_16: compute_data_size(44100, 1, 2) == 88200
  assert.equal(computeDataSize(44100, 1, 2), 88200);
  // wav_file_size_sample: compute_wav_file_size(88200) == 88244
  assert.equal(computeWavFileSize(88200), 88244);
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

test("WAV encode and decode roundtrip", () => {
  const original = new PCMBuffer([0, 1000, -1000, 32767, -32768], new PCMFormat(44_100.0));
  const wavBytes = toWavBytes(original);

  assert.equal(wavBytes.length, 44 + 5 * 2);

  const header = validateWavHeader(wavBytes);
  assert.equal(header.channels, 1);
  assert.equal(header.sampleRate, 44_100);
  assert.equal(header.bitDepth, 16);
  assert.equal(header.dataLength, 10);
  assert.equal(header.dataOffset, 44);

  const decoded = fromWavBytes(wavBytes);
  assert.deepEqual(Array.from(decoded.samples), [0, 1000, -1000, 32767, -32768]);
  assert.equal(decoded.pcmFormat.sampleRateHz, 44_100);
  assert.equal(decoded.clippedSampleCount, 0);
});

test("32-bit float WAV decode with quantization to PCM16", () => {
  const floatSamples = [0.0, 0.5, -0.5, 1.0, 1.5]; // 1.5 clips
  const dataSize = floatSamples.length * 4;
  const fileSize = 44 + dataSize;
  const wavBytes = new Uint8Array(fileSize);
  const view = new DataView(wavBytes.buffer);

  // RIFF
  wavBytes[0] = 0x52;
  wavBytes[1] = 0x49;
  wavBytes[2] = 0x46;
  wavBytes[3] = 0x46;
  view.setUint32(4, fileSize - 8, true);

  // WAVE
  wavBytes[8] = 0x57;
  wavBytes[9] = 0x41;
  wavBytes[10] = 0x56;
  wavBytes[11] = 0x45;

  // fmt
  wavBytes[12] = 0x66;
  wavBytes[13] = 0x6d;
  wavBytes[14] = 0x74;
  wavBytes[15] = 0x20;
  view.setUint32(16, 16, true);
  view.setUint16(20, 3, true); // IEEE float
  view.setUint16(22, 1, true); // mono
  view.setUint32(24, 48_000, true);
  view.setUint32(28, 48_000 * 4, true);
  view.setUint16(32, 4, true);
  view.setUint16(34, 32, true);

  // data
  wavBytes[36] = 0x64;
  wavBytes[37] = 0x61;
  wavBytes[38] = 0x74;
  wavBytes[39] = 0x61;
  view.setUint32(40, dataSize, true);

  for (let i = 0; i < floatSamples.length; i += 1) {
    view.setFloat32(44 + i * 4, floatSamples[i], true);
  }

  const decoded = fromWavBytes(wavBytes);
  assert.equal(decoded.sampleCount(), 5);
  assert.equal(decoded.pcmFormat.sampleRateHz, 48_000);
  assert.equal(decoded.clippedSampleCount, 1);
  assert.equal(decoded.samples[0], 0);
  assert.equal(decoded.samples[3], 32767);
  assert.equal(decoded.samples[4], 32767); // clamped
});
