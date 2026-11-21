/**
 * MUS02: Audio Device Sink (TypeScript implementation).
 *
 * Backend-neutral audio device sink contract, PCM validation, and noop playback reporting.
 *
 * Hardened against:
 * - CWE-1321 (Prototype Pollution): Closed set validation for error kinds, prototype property rejection.
 * - CWE-400 (Uncontrolled Resource Consumption): Pre-flight and streaming length guards against oversized arrays and unbounded iterables.
 * - CWE-20 / CWE-190 (Improper Input Validation / Integer Overflow): Strict Number.isSafeInteger, Number.isFinite, and boundary checks.
 */

export const VERSION = "0.1.0";
export const MAX_BLOCKING_DURATION_SECONDS = 10 * 60;
export const MAX_SAMPLE_RATE_HZ = 384_000;
export const MAX_SAMPLES = MAX_SAMPLE_RATE_HZ * MAX_BLOCKING_DURATION_SECONDS;
export const SUPPORTED_BIT_DEPTH = 16;
export const SUPPORTED_CHANNEL_COUNT = 1;
export const PCM16_MIN = -32_768;
export const PCM16_MAX = 32_767;

const MAX_ERROR_MESSAGE_LENGTH = 1024;
const MAX_BACKEND_NAME_LENGTH = 256;

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
    if (typeof kind !== "string" || !VALID_ERROR_KINDS.has(kind) || BANNED_PROPERTIES.has(kind)) {
      throw new Error(`Invalid audio sink error kind: ${safeFormatValue(kind)}`);
    }
    const safeMessage = safeFormatValue(message);
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

/**
 * Warrant operation: validate_format(sample_rate_hz: Nat, channels: Nat, bit_depth: Nat) -> Result<Bool, AudioError>
 */
export function validateFormat(
  sampleRateHz: number,
  channelCount: number,
  bitDepth: number,
): boolean {
  if (
    typeof sampleRateHz !== "number" ||
    !Number.isSafeInteger(sampleRateHz) ||
    sampleRateHz <= 0 ||
    sampleRateHz > MAX_SAMPLE_RATE_HZ
  ) {
    return false;
  }
  if (
    typeof channelCount !== "number" ||
    !Number.isSafeInteger(channelCount) ||
    channelCount !== SUPPORTED_CHANNEL_COUNT
  ) {
    return false;
  }
  if (
    typeof bitDepth !== "number" ||
    !Number.isSafeInteger(bitDepth) ||
    bitDepth !== SUPPORTED_BIT_DEPTH
  ) {
    return false;
  }
  return true;
}

/**
 * Warrant operation: frame_count(samples_len: Nat, channels: Nat) -> Nat
 */
export function frameCount(samplesLen: number, channelCount: number): number {
  if (
    typeof samplesLen !== "number" ||
    !Number.isSafeInteger(samplesLen) ||
    samplesLen < 0 ||
    samplesLen > MAX_SAMPLES
  ) {
    throw AudioSinkError.invalidSamples("samplesLen must be a safe non-negative integer within MAX_SAMPLES");
  }
  if (
    typeof channelCount !== "number" ||
    !Number.isSafeInteger(channelCount) ||
    channelCount !== SUPPORTED_CHANNEL_COUNT
  ) {
    throw AudioSinkError.invalidFormat(`only mono PCM is supported in V1, got ${safeFormatValue(channelCount)} channels`);
  }
  return Math.floor(samplesLen / channelCount);
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
      !Number.isSafeInteger(this.sampleRateHz) ||
      this.sampleRateHz <= 0
    ) {
      throw AudioSinkError.invalidFormat("sample_rate_hz must be a positive safe finite integer");
    }
    if (this.sampleRateHz > MAX_SAMPLE_RATE_HZ) {
      throw AudioSinkError.invalidFormat(`sample_rate_hz must be <= ${MAX_SAMPLE_RATE_HZ}`);
    }
    if (
      typeof this.channelCount !== "number" ||
      !Number.isSafeInteger(this.channelCount) ||
      this.channelCount !== SUPPORTED_CHANNEL_COUNT
    ) {
      throw AudioSinkError.invalidFormat(`only mono PCM is supported in V1, got ${safeFormatValue(this.channelCount)} channels`);
    }
    if (
      typeof this.bitDepth !== "number" ||
      !Number.isSafeInteger(this.bitDepth) ||
      this.bitDepth !== SUPPORTED_BIT_DEPTH
    ) {
      throw AudioSinkError.invalidFormat(`only signed 16-bit PCM is supported in V1, got ${safeFormatValue(this.bitDepth)} bits`);
    }
  }

  sampleWidthBytes(): number {
    this.validate();
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

    if (
      samples === null ||
      samples === undefined ||
      typeof samples === "string" ||
      typeof (samples as Iterable<number>)[Symbol.iterator] !== "function"
    ) {
      throw AudioSinkError.invalidSamples("samples must be an iterable");
    }

    const maxSamples = format.sampleRateHz * MAX_BLOCKING_DURATION_SECONDS;
    if (!Number.isSafeInteger(maxSamples) || maxSamples <= 0 || maxSamples > MAX_SAMPLES) {
      throw AudioSinkError.invalidFormat("format sample capacity exceeds safe bounds");
    }

    if (
      (Array.isArray(samples) || ArrayBuffer.isView(samples)) &&
      "length" in samples &&
      typeof samples.length === "number" &&
      samples.length > maxSamples
    ) {
      throw AudioSinkError.invalidSamples(
        `blocking playback is limited to ${MAX_BLOCKING_DURATION_SECONDS} seconds`,
      );
    }

    const collected: number[] = [];
    let count = 0;

    for (const rawSample of samples) {
      if (count >= maxSamples) {
        throw AudioSinkError.invalidSamples(
          `blocking playback is limited to ${MAX_BLOCKING_DURATION_SECONDS} seconds`,
        );
      }
      collected.push(normalizeSample(rawSample, count));
      count += 1;
    }

    this.samples = Int16Array.from(collected);
    this.format = format;
  }

  sampleCount(): number {
    return this.samples.length;
  }

  frameCount(): number {
    this.format.validate();
    return Math.floor(this.sampleCount() / this.format.channelCount);
  }

  isEmpty(): boolean {
    return this.sampleCount() === 0;
  }

  durationSeconds(): number {
    const duration = this.frameCount() / this.format.sampleRateHz;
    if (!Number.isFinite(duration) || duration < 0) {
      throw AudioSinkError.invalidFormat("computed duration is not finite");
    }
    return duration;
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
    this.backendName = backendName.slice(0, MAX_BACKEND_NAME_LENGTH);
  }

  playBlocking(buffer: PcmPlaybackBuffer): PlaybackReport {
    if (!buffer || !(buffer instanceof PcmPlaybackBuffer)) {
      throw AudioSinkError.invalidSamples("buffer must be an instance of PcmPlaybackBuffer");
    }
    buffer.format.validate();
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
  if (typeof sample !== "number" || !Number.isSafeInteger(sample)) {
    throw AudioSinkError.invalidSamples(`samples[${index}] must be a safe finite integer, got ${safeFormatValue(sample)}`);
  }
  if (sample < PCM16_MIN || sample > PCM16_MAX) {
    throw AudioSinkError.invalidSamples(`samples[${index}] must fit signed 16-bit PCM, got ${sample}`);
  }
  return sample;
}

function safeFormatValue(value: unknown): string {
  if (typeof value === "symbol") {
    return value.toString().slice(0, MAX_ERROR_MESSAGE_LENGTH);
  }
  if (typeof value === "string") {
    return value.slice(0, MAX_ERROR_MESSAGE_LENGTH);
  }
  try {
    return String(value).slice(0, MAX_ERROR_MESSAGE_LENGTH);
  } catch {
    return Object.prototype.toString.call(value).slice(0, MAX_ERROR_MESSAGE_LENGTH);
  }
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
