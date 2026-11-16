/**
 * MUS02: Audio Device Sink (TypeScript implementation).
 *
 * Backend-neutral audio device sink contract, PCM validation, and noop playback reporting.
 *
 * Hardened against:
 * - CWE-1321 (Prototype Pollution): Closed set validation for error kinds, prototype property rejection.
 * - CWE-400 (Uncontrolled Resource Consumption): Streaming length guard against unbounded iterables.
 * - CWE-20 (Improper Input Validation): Strict Number.isFinite, Number.isInteger, and boundary checks.
 */

export const VERSION = "0.1.0";
export const MAX_BLOCKING_DURATION_SECONDS = 10 * 60;
export const MAX_SAMPLE_RATE_HZ = 384_000;
export const SUPPORTED_BIT_DEPTH = 16;
export const SUPPORTED_CHANNEL_COUNT = 1;
export const PCM16_MIN = -32_768;
export const PCM16_MAX = 32_767;

const BANNED_PROPERTIES = new Set(["__proto__", "constructor", "prototype", "toString", "valueOf"]);

const VALID_ERROR_KINDS: ReadonlySet<string> = new Set([
  "invalid_format",
  "invalid_samples",
  "unsupported_platform",
  "backend_unavailable",
  "backend_failure",
]);

export type AudioSinkErrorKind =
  | "invalid_format"
  | "invalid_samples"
  | "unsupported_platform"
  | "backend_unavailable"
  | "backend_failure";

export class AudioSinkError extends Error {
  constructor(readonly kind: AudioSinkErrorKind, message: string) {
    if (!VALID_ERROR_KINDS.has(kind) || BANNED_PROPERTIES.has(kind)) {
      throw new Error(`Invalid audio sink error kind: ${String(kind)}`);
    }
    const safeMessage = typeof message === "string" ? message.slice(0, 1024) : String(message);
    super(formatError(kind, safeMessage));
    this.name = "AudioSinkError";
  }

  static invalidFormat(message: string): AudioSinkError {
    return new AudioSinkError("invalid_format", message);
  }

  static invalidSamples(message: string): AudioSinkError {
    return new AudioSinkError("invalid_samples", message);
  }

  static unsupportedPlatform(message: string): AudioSinkError {
    return new AudioSinkError("unsupported_platform", message);
  }

  static backendUnavailable(message: string): AudioSinkError {
    return new AudioSinkError("backend_unavailable", message);
  }

  static backendFailure(message: string): AudioSinkError {
    return new AudioSinkError("backend_failure", message);
  }
}

export class PcmFormat {
  constructor(
    readonly sampleRateHz: number,
    readonly channelCount: number,
    readonly bitDepth: number,
  ) {
    this.validate();
  }

  validate(): void {
    if (
      typeof this.sampleRateHz !== "number" ||
      !Number.isFinite(this.sampleRateHz) ||
      !Number.isInteger(this.sampleRateHz) ||
      this.sampleRateHz <= 0
    ) {
      throw AudioSinkError.invalidFormat("sample_rate_hz must be a positive finite integer");
    }
    if (this.sampleRateHz > MAX_SAMPLE_RATE_HZ) {
      throw AudioSinkError.invalidFormat(`sample_rate_hz must be <= ${MAX_SAMPLE_RATE_HZ}`);
    }
    if (
      typeof this.channelCount !== "number" ||
      !Number.isFinite(this.channelCount) ||
      !Number.isInteger(this.channelCount) ||
      this.channelCount !== SUPPORTED_CHANNEL_COUNT
    ) {
      throw AudioSinkError.invalidFormat(`only mono PCM is supported in V1, got ${this.channelCount} channels`);
    }
    if (
      typeof this.bitDepth !== "number" ||
      !Number.isFinite(this.bitDepth) ||
      !Number.isInteger(this.bitDepth) ||
      this.bitDepth !== SUPPORTED_BIT_DEPTH
    ) {
      throw AudioSinkError.invalidFormat(`only signed 16-bit PCM is supported in V1, got ${this.bitDepth} bits`);
    }
  }

  sampleWidthBytes(): number {
    return this.bitDepth / 8;
  }
}

export class PcmPlaybackBuffer {
  readonly samples: Int16Array;
  readonly format: PcmFormat;

  constructor(samples: Iterable<number>, format: PcmFormat) {
    if (!format || !(format instanceof PcmFormat)) {
      throw AudioSinkError.invalidFormat("Invalid PcmFormat instance provided");
    }
    format.validate();

    if (!samples || typeof (samples as any)[Symbol.iterator] !== "function") {
      throw AudioSinkError.invalidSamples("samples must be an iterable");
    }

    const maxSamples = format.sampleRateHz * MAX_BLOCKING_DURATION_SECONDS;
    const collected: number[] = [];
    let count = 0;

    for (const rawSample of samples) {
      if (count >= maxSamples) {
        throw AudioSinkError.invalidSamples(
          `blocking playback is limited to ${MAX_BLOCKING_DURATION_SECONDS} seconds`,
        );
      }
      collected.push(normalizeSample(rawSample, count));
      count++;
    }

    this.samples = Int16Array.from(collected);
    this.format = format;
  }

  sampleCount(): number {
    return this.samples.length;
  }

  frameCount(): number {
    return Math.floor(this.sampleCount() / this.format.channelCount);
  }

  isEmpty(): boolean {
    return this.sampleCount() === 0;
  }

  durationSeconds(): number {
    return this.frameCount() / this.format.sampleRateHz;
  }
}

export interface PlaybackReport {
  framesPlayed: number;
  sampleRateHz: number;
  channelCount: number;
  durationSeconds: number;
  backendName: string;
}

export interface AudioSink {
  playBlocking(buffer: PcmPlaybackBuffer): PlaybackReport;
}

export class NoopAudioSink implements AudioSink {
  readonly backendName: string;

  constructor(backendName = "noop") {
    if (typeof backendName !== "string") {
      throw new TypeError("backendName must be a string");
    }
    if (BANNED_PROPERTIES.has(backendName)) {
      throw new Error(`Forbidden backendName: ${backendName}`);
    }
    this.backendName = backendName.slice(0, 256);
  }

  playBlocking(buffer: PcmPlaybackBuffer): PlaybackReport {
    if (!buffer || !(buffer instanceof PcmPlaybackBuffer)) {
      throw AudioSinkError.invalidSamples("buffer must be an instance of PcmPlaybackBuffer");
    }
    return {
      framesPlayed: buffer.frameCount(),
      sampleRateHz: buffer.format.sampleRateHz,
      channelCount: buffer.format.channelCount,
      durationSeconds: buffer.durationSeconds(),
      backendName: this.backendName,
    };
  }
}

function normalizeSample(sample: number, index: number): number {
  if (typeof sample !== "number" || !Number.isFinite(sample) || !Number.isInteger(sample)) {
    throw AudioSinkError.invalidSamples(`samples[${index}] must be a finite integer, got ${sample}`);
  }
  if (sample < PCM16_MIN || sample > PCM16_MAX) {
    throw AudioSinkError.invalidSamples(`samples[${index}] must fit signed 16-bit PCM, got ${sample}`);
  }
  return sample;
}

function formatError(kind: AudioSinkErrorKind, message: string): string {
  switch (kind) {
    case "invalid_format":
      return `invalid PCM format: ${message}`;
    case "invalid_samples":
      return `invalid PCM samples: ${message}`;
    case "unsupported_platform":
      return `unsupported platform: ${message}`;
    case "backend_unavailable":
      return `audio backend unavailable: ${message}`;
    case "backend_failure":
      return `audio backend failure: ${message}`;
  }
}
