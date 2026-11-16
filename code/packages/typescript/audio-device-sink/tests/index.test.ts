import { describe, it, expect, assertProperty, Gen } from "../../test-bootstrap/dist/index.js";

import {
  AudioSinkError,
  MAX_BLOCKING_DURATION_SECONDS,
  MAX_SAMPLE_RATE_HZ,
  NoopAudioSink,
  PcmFormat,
  PcmPlaybackBuffer,
  VERSION,
} from "../dist/index.js";

describe("audio-device-sink core functionality", () => {
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

describe("CWE-1321: Prototype Pollution & Object Key Hardening", () => {
  it("rejects prototype property names as backendName in NoopAudioSink", () => {
    for (const key of ["__proto__", "constructor", "prototype", "toString"]) {
      expect(() => new NoopAudioSink(key)).toThrow();
    }
  });

  it("rejects invalid or prototype-polluting error kinds in AudioSinkError", () => {
    // @ts-expect-error Testing invalid error kinds
    expect(() => new AudioSinkError("__proto__", "msg")).toThrow();
    // @ts-expect-error Testing invalid error kinds
    expect(() => new AudioSinkError("constructor", "msg")).toThrow();
    // @ts-expect-error Testing invalid error kinds
    expect(() => new AudioSinkError("prototype", "msg")).toThrow();
  });

  it("verifies global Object.prototype remains unpolluted", () => {
    const testObj: Record<string, unknown> = {};
    expect(testObj["backendName"]).toBeUndefined();
    expect(testObj["invalid_format"]).toBeUndefined();
  });
});

describe("CWE-400: Resource Exhaustion & Unbounded Iterables", () => {
  it("prevents infinite generator from causing memory exhaustion", () => {
    function* infiniteSamples(): Generator<number> {
      while (true) {
        yield 0;
      }
    }
    const format = new PcmFormat(10, 1, 16); // max = 10 * 600 = 6000 samples
    expect(() => new PcmPlaybackBuffer(infiniteSamples(), format)).toThrow(AudioSinkError);
  });
});

describe("CWE-20: Improper Input Validation & Type Guards", () => {
  it("rejects non-finite numbers in PcmFormat", () => {
    for (const badRate of [NaN, Infinity, -Infinity, 44100.5]) {
      expect(() => new PcmFormat(badRate, 1, 16)).toThrow(AudioSinkError);
    }
  });

  it("rejects non-finite samples in buffer", () => {
    const format = new PcmFormat(44100, 1, 16);
    for (const badSample of [NaN, Infinity, -Infinity]) {
      expect(() => new PcmPlaybackBuffer([badSample], format)).toThrow(AudioSinkError);
    }
  });

  it("rejects non-string backendName in NoopAudioSink", () => {
    // @ts-expect-error Runtime validation
    expect(() => new NoopAudioSink(1234)).toThrow(TypeError);
    // @ts-expect-error Runtime validation
    expect(() => new NoopAudioSink(null)).toThrow(TypeError);
  });
});

describe("PBT & Invariant Verification (VER05a)", () => {
  it("proves frameCount and duration invariants for arbitrary valid buffers", () => {
    const sampleGen = Gen.integer(-32768, 32767);
    const listGen = Gen.list(0, 100, sampleGen);

    assertProperty(listGen, (samples) => {
      const format = new PcmFormat(48000, 1, 16);
      const numSamples = samples.map((n) => Number(n));
      const buffer = new PcmPlaybackBuffer(numSamples, format);
      const sink = new NoopAudioSink("pbt-sink");
      const report = sink.playBlocking(buffer);

      expect(report.framesPlayed).toBe(numSamples.length);
      expect(report.sampleRateHz).toBe(48000);
      expect(report.channelCount).toBe(1);
      expect(Number.isFinite(report.durationSeconds)).toBe(true);
      return true;
    }, { numRuns: 50 });
  });

  it("fuzzes sample normalization fail-closed bounds", () => {
    const hostileSampleGen = Gen.oneOf(
      Gen.integer(-100000, -32769),
      Gen.integer(32768, 100000)
    );

    assertProperty(hostileSampleGen, (outOfBounds) => {
      const format = new PcmFormat(44100, 1, 16);
      try {
        new PcmPlaybackBuffer([Number(outOfBounds)], format);
        return false; // Should not reach here
      } catch (err) {
        return err instanceof AudioSinkError && err.kind === "invalid_samples";
      }
    }, { numRuns: 50 });
  });
});
