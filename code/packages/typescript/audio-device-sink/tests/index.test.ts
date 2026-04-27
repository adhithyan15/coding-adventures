import { describe, expect, it } from "vitest";

import {
  AudioSinkError,
  MAX_BLOCKING_DURATION_SECONDS,
  MAX_SAMPLE_RATE_HZ,
  NoopAudioSink,
  PcmFormat,
  PcmPlaybackBuffer,
  VERSION,
} from "../src/index.js";

describe("audio-device-sink", () => {
  it("records valid PCM metadata", () => {
    const format = new PcmFormat(44_100, 1, 16);

    expect(VERSION).toBe("0.1.0");
    expect(format.sampleRateHz).toBe(44_100);
    expect(format.channelCount).toBe(1);
    expect(format.bitDepth).toBe(16);
    expect(format.sampleWidthBytes()).toBe(2);
  });

  it("rejects invalid formats", () => {
    expect(() => new PcmFormat(0, 1, 16)).toThrow(/sample_rate_hz/);
    expect(() => new PcmFormat(MAX_SAMPLE_RATE_HZ + 1, 1, 16)).toThrow(/384000/);
    expect(() => new PcmFormat(44_100, 2, 16)).toThrow(/mono/);
    expect(() => new PcmFormat(44_100, 1, 24)).toThrow(/16-bit/);
  });

  it("reports buffer counts and duration", () => {
    const format = new PcmFormat(4, 1, 16);
    const buffer = new PcmPlaybackBuffer([0, 1000, 0, -1000], format);

    expect(buffer.sampleCount()).toBe(4);
    expect(buffer.frameCount()).toBe(4);
    expect(buffer.durationSeconds()).toBe(1);
    expect(buffer.isEmpty()).toBe(false);
  });

  it("rejects invalid samples and too-long buffers", () => {
    const format = new PcmFormat(1, 1, 16);

    expect(() => new PcmPlaybackBuffer([1.5], format)).toThrow(/integer/);
    expect(() => new PcmPlaybackBuffer([32_768], format)).toThrow(/signed 16-bit/);
    expect(() => new PcmPlaybackBuffer(new Array(MAX_BLOCKING_DURATION_SECONDS + 1).fill(0), format)).toThrow(
      /limited/,
    );
  });

  it("returns a playback report from the no-op sink", () => {
    const format = new PcmFormat(44_100, 1, 16);
    const buffer = new PcmPlaybackBuffer([0, 1000], format);
    const report = new NoopAudioSink("test-noop").playBlocking(buffer);

    expect(report.framesPlayed).toBe(2);
    expect(report.sampleRateHz).toBe(44_100);
    expect(report.channelCount).toBe(1);
    expect(report.backendName).toBe("test-noop");
  });

  it("accepts empty buffers and formats error categories", () => {
    const format = new PcmFormat(44_100, 1, 16);
    const buffer = new PcmPlaybackBuffer([], format);

    expect(buffer.isEmpty()).toBe(true);
    expect(new NoopAudioSink("empty").playBlocking(buffer).durationSeconds).toBe(0);
    expect(AudioSinkError.backendUnavailable("default device missing").message).toBe(
      "audio backend unavailable: default device missing",
    );
    expect(AudioSinkError.unsupportedPlatform("Core Audio requires macOS").message).toBe(
      "unsupported platform: Core Audio requires macOS",
    );
    expect(AudioSinkError.backendFailure("write failed").message).toBe("audio backend failure: write failed");
  });
});
