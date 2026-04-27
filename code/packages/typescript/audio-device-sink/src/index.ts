export const VERSION = "0.1.0";
export const MAX_BLOCKING_DURATION_SECONDS = 10 * 60;
export const MAX_SAMPLE_RATE_HZ = 384_000;
export const SUPPORTED_BIT_DEPTH = 16;
export const SUPPORTED_CHANNEL_COUNT = 1;
export const PCM16_MIN = -32_768;
export const PCM16_MAX = 32_767;

export type AudioSinkErrorKind =
  | "invalid_format"
  | "invalid_samples"
  | "unsupported_platform"
  | "backend_unavailable"
  | "backend_failure";

export class AudioSinkError extends Error {
  constructor(readonly kind: AudioSinkErrorKind, message: string) {
    super(formatError(kind, message));
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
    if (!Number.isInteger(this.sampleRateHz) || this.sampleRateHz <= 0) {
      throw AudioSinkError.invalidFormat("sample_rate_hz must be greater than zero");
    }
    if (this.sampleRateHz > MAX_SAMPLE_RATE_HZ) {
      throw AudioSinkError.invalidFormat(`sample_rate_hz must be <= ${MAX_SAMPLE_RATE_HZ}`);
    }
    if (this.channelCount !== SUPPORTED_CHANNEL_COUNT) {
      throw AudioSinkError.invalidFormat(`only mono PCM is supported in V1, got ${this.channelCount} channels`);
    }
    if (this.bitDepth !== SUPPORTED_BIT_DEPTH) {
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
    format.validate();
    const normalized = Array.from(samples, normalizeSample);
    this.samples = Int16Array.from(normalized);
    this.format = format;
    this.validateSize();
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

  private validateSize(): void {
    const maxSamples = this.format.sampleRateHz * MAX_BLOCKING_DURATION_SECONDS;
    if (this.sampleCount() > maxSamples) {
      throw AudioSinkError.invalidSamples(
        `blocking playback is limited to ${MAX_BLOCKING_DURATION_SECONDS} seconds`,
      );
    }
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
  constructor(readonly backendName = "noop") {}

  playBlocking(buffer: PcmPlaybackBuffer): PlaybackReport {
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
  if (!Number.isInteger(sample)) {
    throw AudioSinkError.invalidSamples(`samples[${index}] must be an integer`);
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
