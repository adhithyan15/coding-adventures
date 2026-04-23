export const DEFAULT_SAMPLE_RATE_HZ = 44_100.0;
export const DEFAULT_BIT_DEPTH = 16;
export const DEFAULT_CHANNEL_COUNT = 1;
export const DEFAULT_FULL_SCALE_VOLTAGE = 1.0;
export const PCM16_MIN = -32_768;
export const PCM16_MAX = 32_767;

export interface SampleBufferLike {
  samples: Iterable<number>;
  sampleRateHz: number;
  startTimeSeconds?: number;
}

export interface PCMEncodingOptions {
  sampleRateHz?: number;
  startTimeSeconds?: number;
  pcmFormat?: PCMFormat;
}

export class PCMFormat {
  readonly sampleRateHz: number;
  readonly channelCount: number;
  readonly bitDepth: number;
  readonly fullScaleVoltage: number;

  constructor(
    sampleRateHz: number = DEFAULT_SAMPLE_RATE_HZ,
    channelCount: number = DEFAULT_CHANNEL_COUNT,
    bitDepth: number = DEFAULT_BIT_DEPTH,
    fullScaleVoltage: number = DEFAULT_FULL_SCALE_VOLTAGE,
  ) {
    this.sampleRateHz = positiveFloat("sampleRateHz", sampleRateHz);
    this.channelCount = positiveInt("channelCount", channelCount);
    if (this.channelCount !== DEFAULT_CHANNEL_COUNT) {
      throw new RangeError("PCMFormat supports only mono audio in this V1 model");
    }
    this.bitDepth = positiveInt("bitDepth", bitDepth);
    if (this.bitDepth !== DEFAULT_BIT_DEPTH) {
      throw new RangeError("PCMFormat supports only 16-bit PCM in this V1 model");
    }
    this.fullScaleVoltage = positiveFloat("fullScaleVoltage", fullScaleVoltage);
  }

  get minimumInteger(): number {
    return PCM16_MIN;
  }

  get maximumInteger(): number {
    return PCM16_MAX;
  }

  get sampleWidthBytes(): number {
    return this.bitDepth / 8;
  }

  integerSampleRate(): number {
    return integerSampleRate(this.sampleRateHz);
  }
}

export class PCMBuffer {
  readonly samples: ReadonlyArray<number>;
  readonly pcmFormat: PCMFormat;
  readonly startTimeSeconds: number;
  readonly clippedSampleCount: number;

  constructor(
    samples: Iterable<number>,
    pcmFormat: PCMFormat,
    startTimeSeconds = 0.0,
    clippedSampleCount = 0,
  ) {
    this.samples = validateIntegerSamples(samples);
    if (!(pcmFormat instanceof PCMFormat)) {
      throw new TypeError("pcmFormat must be a PCMFormat");
    }
    this.pcmFormat = pcmFormat;
    this.startTimeSeconds = finiteFloat("startTimeSeconds", startTimeSeconds);
    this.clippedSampleCount = nonNegativeInt("clippedSampleCount", clippedSampleCount);
  }

  sampleCount(): number {
    return this.samples.length;
  }

  samplePeriodSeconds(): number {
    return 1.0 / this.pcmFormat.sampleRateHz;
  }

  durationSeconds(): number {
    return this.sampleCount() / this.pcmFormat.sampleRateHz;
  }

  timeAt(index: number): number {
    if (!Number.isInteger(index)) {
      throw new TypeError(`index must be an integer, got ${String(index)}`);
    }
    if (index < 0 || index >= this.sampleCount()) {
      throw new RangeError(
        `index must be in [0, ${this.sampleCount()}), got ${String(index)}`,
      );
    }
    return this.startTimeSeconds + index / this.pcmFormat.sampleRateHz;
  }

  toLittleEndianBytes(): Uint8Array {
    const bytes = new Uint8Array(this.sampleCount() * 2);
    const view = new DataView(bytes.buffer);
    for (let i = 0; i < this.sampleCount(); i += 1) {
      view.setInt16(i * 2, this.samples[i], true);
    }
    return bytes;
  }
}

export function floatToPcm16(sample: number): [number, boolean] {
  const value = finiteFloat("sample", sample);
  const clipped = value < -1.0 || value > 1.0;
  const bounded = clamp(value, -1.0, 1.0);
  if (bounded >= 0.0) {
    return [Math.round(bounded * PCM16_MAX), clipped];
  }
  return [Math.round(bounded * Math.abs(PCM16_MIN)), clipped];
}

export function encodeSampleBuffer(
  sampleBuffer: SampleBufferLike,
  pcmFormat?: PCMFormat,
): PCMBuffer {
  if (
    sampleBuffer === null ||
    typeof sampleBuffer !== "object" ||
    !isIterable(sampleBuffer.samples)
  ) {
    throw new TypeError("sampleBuffer must be an object with iterable samples");
  }
  const activeFormat =
    pcmFormat === undefined
      ? new PCMFormat(sampleBuffer.sampleRateHz)
      : pcmFormat;

  const pcmSamples: number[] = [];
  let clippedSampleCount = 0;

  for (const sample of sampleBuffer.samples) {
    const [encoded, clipped] = floatToPcm16(sample);
    pcmSamples.push(encoded);
    if (clipped) {
      clippedSampleCount += 1;
    }
  }

  return new PCMBuffer(
    pcmSamples,
    activeFormat,
    finiteFloat("sampleBuffer.startTimeSeconds", sampleBuffer.startTimeSeconds ?? 0.0),
    clippedSampleCount,
  );
}

export function samplesToPcmBuffer(
  samples: Iterable<number>,
  options: PCMEncodingOptions,
): PCMBuffer {
  if (options === undefined || options === null || typeof options !== "object") {
    throw new TypeError("options must be an object");
  }

  const pcmFormat = options.pcmFormat ?? new PCMFormat(options.sampleRateHz);

  const sampleBuffer: SampleBufferLike = {
    samples,
    sampleRateHz: options.sampleRateHz ?? DEFAULT_SAMPLE_RATE_HZ,
    startTimeSeconds: options.startTimeSeconds ?? 0.0,
  };
  return encodeSampleBuffer(sampleBuffer, pcmFormat);
}

function positiveFloat(name: string, value: number): number {
  const converted = finiteFloat(name, value);
  if (converted <= 0.0) {
    throw new RangeError(`${name} must be > 0.0, got ${converted}`);
  }
  return converted;
}

function finiteFloat(name: string, value: number): number {
  if (typeof value !== "number" || Number.isNaN(value) || !Number.isFinite(value)) {
    throw new TypeError(`${name} must be a finite real number, got ${String(value)}`);
  }
  return value;
}

function nonNegativeInt(name: string, value: number): number {
  if (!Number.isInteger(value)) {
    throw new TypeError(`${name} must be an integer >= 0, got ${String(value)}`);
  }
  if (value < 0) {
    throw new RangeError(`${name} must be >= 0, got ${value}`);
  }
  return value;
}

function positiveInt(name: string, value: number): number {
  const converted = nonNegativeInt(name, value);
  if (converted === 0) {
    throw new RangeError(`${name} must be > 0, got ${converted}`);
  }
  return converted;
}

function integerSampleRate(sampleRateHz: number): number {
  const rounded = Math.round(sampleRateHz);
  if (Math.abs(sampleRateHz - rounded) > 1e-9) {
    throw new RangeError(
      "sampleRateHz must be integer-valued for WAV output, got " + sampleRateHz,
    );
  }
  return rounded;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function validateIntegerSamples(samples: Iterable<number>): number[] {
  const checked: number[] = [];
  for (const sample of samples) {
    if (!Number.isInteger(sample) || sample < PCM16_MIN || sample > PCM16_MAX) {
      throw new RangeError(
        `PCM samples must be signed 16-bit integers in [${PCM16_MIN}, ${PCM16_MAX}], got ${String(
          sample,
        )}`,
      );
    }
    checked.push(sample);
  }
  return checked;
}

function isIterable(value: unknown): value is Iterable<number> {
  return value != null && typeof value === "object" && Symbol.iterator in Object(value);
}
