/**
 * Reusable PCM encoding stage and WAV container safety for virtual audio pipelines.
 * Complies with MUS01, OSC00, and VER15 security requirements (CWE-20, CWE-400).
 */

export const DEFAULT_SAMPLE_RATE_HZ = 44_100.0;
export const DEFAULT_BIT_DEPTH = 16;
export const DEFAULT_CHANNEL_COUNT = 1;
export const DEFAULT_FULL_SCALE_VOLTAGE = 1.0;
export const PCM16_MIN = -32_768;
export const PCM16_MAX = 32_767;

export const MAX_SAMPLES = 16_777_216; // 2^24 samples (~32 MiB for 16-bit)
export const MAX_SAMPLE_RATE_HZ = 192_000.0;
export const MAX_DURATION_SECONDS = 60.0;
export const MAX_CHANNELS = 8;
export const MAX_WAV_BYTES = 67_108_864; // 64 MiB
export const INTEGER_TOLERANCE = 1e-9;

export interface SampleBufferLike {
  readonly samples: Iterable<number>;
  readonly sampleRateHz: number;
  readonly startTimeSeconds?: number;
}

export interface PCMEncodingOptions {
  readonly sampleRateHz?: number;
  readonly startTimeSeconds?: number;
  readonly pcmFormat?: PCMFormat;
}

export interface WavHeaderInfo {
  readonly channels: number;
  readonly sampleRate: number;
  readonly bitDepth: number;
  readonly dataOffset: number;
  readonly dataLength: number;
}

/**
 * Warrant operation: clamp_to_pcm16(v: Int) -> Int
 */
export function clampToPcm16(v: number): number {
  if (typeof v !== "number" || !Number.isFinite(v)) {
    throw new TypeError(`value must be a finite number, got ${safeFormatValue(v)}`);
  }
  const intVal = Math.trunc(v);
  if (intVal > PCM16_MAX) {
    return PCM16_MAX;
  }
  if (intVal < PCM16_MIN) {
    return PCM16_MIN;
  }
  return intVal;
}

/**
 * Warrant operation: compute_data_size(sample_count: Int, channels: Int, bytes_per_sample: Int) -> Int
 */
export function computeDataSize(
  sampleCount: number,
  channels: number,
  bytesPerSample: number,
): number {
  const count = nonNegativeInt("sampleCount", sampleCount);
  const ch = positiveInt("channels", channels);
  const bps = positiveInt("bytesPerSample", bytesPerSample);

  if (count > MAX_SAMPLES) {
    throw new RangeError(`sampleCount ${String(count)} exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`);
  }
  if (ch > MAX_CHANNELS) {
    throw new RangeError(`channels ${String(ch)} exceeds MAX_CHANNELS=${String(MAX_CHANNELS)}`);
  }

  const payload = count * ch * bps;
  if (payload > MAX_WAV_BYTES) {
    throw new RangeError(`payload size ${String(payload)} exceeds MAX_WAV_BYTES=${String(MAX_WAV_BYTES)}`);
  }
  return payload;
}

/**
 * Warrant operation: compute_wav_file_size(data_size: Int) -> Int = 44 + data_size
 */
export function computeWavFileSize(dataSize: number): number {
  const ds = nonNegativeInt("dataSize", dataSize);
  const total = 44 + ds;
  if (total > MAX_WAV_BYTES) {
    throw new RangeError(`WAV file size ${String(total)} exceeds MAX_WAV_BYTES=${String(MAX_WAV_BYTES)}`);
  }
  return total;
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
    this.sampleRateHz = validateSampleRate("sampleRateHz", sampleRateHz);
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
  readonly samplesArray: ReadonlyArray<number>;
  readonly pcmFormat: PCMFormat;
  readonly startTimeSeconds: number;
  readonly clippedSampleCount: number;

  constructor(
    samples: Iterable<number>,
    pcmFormat: PCMFormat,
    startTimeSeconds = 0.0,
    clippedSampleCount = 0,
  ) {
    this.samplesArray = validateIntegerSamples(samples);
    if (!(pcmFormat instanceof PCMFormat)) {
      throw new TypeError("pcmFormat must be a PCMFormat");
    }
    this.pcmFormat = pcmFormat;
    this.startTimeSeconds = finiteFloat("startTimeSeconds", startTimeSeconds);
    this.clippedSampleCount = nonNegativeInt("clippedSampleCount", clippedSampleCount);
  }

  get samples(): ReadonlyArray<number> {
    return this.samplesArray;
  }

  sampleCount(): number {
    return this.samplesArray.length;
  }

  samplePeriodSeconds(): number {
    return 1.0 / this.pcmFormat.sampleRateHz;
  }

  durationSeconds(): number {
    return this.sampleCount() / this.pcmFormat.sampleRateHz;
  }

  timeAt(index: number): number {
    if (typeof index !== "number" || !Number.isSafeInteger(index)) {
      throw new TypeError(`index must be an integer, got ${safeFormatValue(index)}`);
    }
    if (index < 0 || index >= this.sampleCount()) {
      throw new RangeError(
        `index must be in [0, ${String(this.sampleCount())}), got ${String(index)}`,
      );
    }
    return this.startTimeSeconds + index / this.pcmFormat.sampleRateHz;
  }

  toLittleEndianBytes(): Uint8Array {
    const count = this.sampleCount();
    const byteLength = count * 2;
    if (byteLength > MAX_WAV_BYTES) {
      throw new RangeError(`byte length ${String(byteLength)} exceeds MAX_WAV_BYTES=${String(MAX_WAV_BYTES)}`);
    }
    const bytesArray = new Uint8Array(byteLength);
    const view = new DataView(bytesArray.buffer);
    for (let i = 0; i < count; i += 1) {
      view.setInt16(i * 2, this.samplesArray[i], true);
    }
    return bytesArray;
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

export const safeRoundSample = floatToPcm16;

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

  if (
    (Array.isArray(sampleBuffer.samples) || ArrayBuffer.isView(sampleBuffer.samples)) &&
    "length" in sampleBuffer.samples &&
    typeof sampleBuffer.samples.length === "number" &&
    sampleBuffer.samples.length > MAX_SAMPLES
  ) {
    throw new RangeError(`sample count exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`);
  }

  const pcmSamplesArray: number[] = [];
  let clippedSampleCount = 0;

  for (const sample of sampleBuffer.samples) {
    if (pcmSamplesArray.length >= MAX_SAMPLES) {
      throw new RangeError(`sample count exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`);
    }
    const [encoded, clipped] = floatToPcm16(sample);
    pcmSamplesArray.push(encoded);
    if (clipped) {
      clippedSampleCount += 1;
    }
  }

  return new PCMBuffer(
    pcmSamplesArray,
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

/**
 * Validate a WAV byte buffer against container structure and bounds (CWE-20, CWE-400).
 */
export function validateWavHeader(wavBytes: Uint8Array): WavHeaderInfo {
  if (!(wavBytes instanceof Uint8Array)) {
    throw new TypeError(`wavBytes must be a Uint8Array, got ${typeof wavBytes}`);
  }

  const totalLen = wavBytes.length;
  if (totalLen > MAX_WAV_BYTES) {
    throw new RangeError(
      `WAV byte length ${String(totalLen)} exceeds MAX_WAV_BYTES=${String(MAX_WAV_BYTES)}`,
    );
  }
  if (totalLen < 44) {
    throw new RangeError(
      `WAV byte length ${String(totalLen)} is smaller than minimum header size 44`,
    );
  }

  // Check 'RIFF' magic (0x52, 0x49, 0x46, 0x46)
  if (
    wavBytes[0] !== 0x52 ||
    wavBytes[1] !== 0x49 ||
    wavBytes[2] !== 0x46 ||
    wavBytes[3] !== 0x46
  ) {
    throw new RangeError("missing 'RIFF' container magic");
  }

  // Check 'WAVE' format marker (0x57, 0x41, 0x56, 0x45)
  if (
    wavBytes[8] !== 0x57 ||
    wavBytes[9] !== 0x41 ||
    wavBytes[10] !== 0x56 ||
    wavBytes[11] !== 0x45
  ) {
    throw new RangeError("missing 'WAVE' format marker");
  }

  const view = new DataView(wavBytes.buffer, wavBytes.byteOffset, wavBytes.byteLength);
  const riffPayload = view.getUint32(4, true);
  if (riffPayload < 36) {
    throw new RangeError(`RIFF payload size too small: ${String(riffPayload)}`);
  }

  let offset = 12;
  let fmtParsed = false;
  let channels = 0;
  let sampleRate = 0;
  let bitsPerSample = 0;
  let blockAlign = 0;
  let dataOffset = 0;
  let dataLength = 0;

  while (offset + 8 <= totalLen) {
    const c0 = wavBytes[offset];
    const c1 = wavBytes[offset + 1];
    const c2 = wavBytes[offset + 2];
    const c3 = wavBytes[offset + 3];
    const chunkSize = view.getUint32(offset + 4, true);
    const dataStart = offset + 8;
    const dataEnd = dataStart + chunkSize;

    if (dataEnd > totalLen) {
      throw new RangeError(
        `chunk declared size ${String(chunkSize)} exceeds buffer boundary`,
      );
    }

    // "fmt " chunk (0x66, 0x6d, 0x74, 0x20)
    if (c0 === 0x66 && c1 === 0x6d && c2 === 0x74 && c3 === 0x20) {
      if (chunkSize < 16) {
        throw new RangeError("fmt chunk size too small");
      }
      const audioFormat = view.getUint16(dataStart, true);
      if (audioFormat !== 1 && audioFormat !== 3) {
        throw new RangeError(`unsupported WAV audio format ${String(audioFormat)}`);
      }
      channels = view.getUint16(dataStart + 2, true);
      if (channels === 0 || channels > MAX_CHANNELS) {
        throw new RangeError(`unsupported channel count ${String(channels)}`);
      }
      sampleRate = view.getUint32(dataStart + 4, true);
      if (sampleRate === 0 || sampleRate > MAX_SAMPLE_RATE_HZ) {
        throw new RangeError(
          `sample rate ${String(sampleRate)} outside (0, ${String(MAX_SAMPLE_RATE_HZ)}]`,
        );
      }
      const byteRate = view.getUint32(dataStart + 8, true);
      blockAlign = view.getUint16(dataStart + 12, true);
      bitsPerSample = view.getUint16(dataStart + 14, true);

      if (audioFormat === 1 && bitsPerSample !== 16) {
        throw new RangeError(`unsupported bit depth ${String(bitsPerSample)} for PCM`);
      }
      if (audioFormat === 3 && bitsPerSample !== 32) {
        throw new RangeError(
          `unsupported bit depth ${String(bitsPerSample)} for IEEE float`,
        );
      }

      const expectedBlockAlign = channels * (bitsPerSample / 8);
      if (blockAlign !== expectedBlockAlign) {
        throw new RangeError(
          `inconsistent block align ${String(blockAlign)} !== ${String(expectedBlockAlign)}`,
        );
      }
      const expectedByteRate = sampleRate * blockAlign;
      if (byteRate !== expectedByteRate) {
        throw new RangeError(
          `inconsistent byte rate ${String(byteRate)} !== ${String(expectedByteRate)}`,
        );
      }
      fmtParsed = true;
    } else if (c0 === 0x64 && c1 === 0x61 && c2 === 0x74 && c3 === 0x61) {
      // "data" chunk (0x64, 0x61, 0x74, 0x61)
      if (!fmtParsed) {
        throw new RangeError("data chunk before fmt chunk");
      }
      if (blockAlign > 0 && chunkSize % blockAlign !== 0) {
        throw new RangeError(
          `data chunk size ${String(chunkSize)} is not a multiple of block align ${String(blockAlign)}`,
        );
      }
      dataOffset = dataStart;
      dataLength = chunkSize;
      break;
    }

    const pad = chunkSize % 2;
    offset = dataEnd + pad;
  }

  if (!fmtParsed) {
    throw new RangeError("missing fmt chunk in WAV");
  }
  if (dataOffset === 0) {
    throw new RangeError("missing data chunk in WAV");
  }

  return {
    channels,
    sampleRate,
    bitDepth: bitsPerSample,
    dataOffset,
    dataLength,
  };
}

/**
 * Decode RIFF/WAVE bytes into a mono 16-bit PCMBuffer.
 */
export function fromWavBytes(wavBytes: Uint8Array): PCMBuffer {
  const header = validateWavHeader(wavBytes);
  if (header.channels !== 1) {
    throw new RangeError(`only mono WAV files are supported, got ${String(header.channels)} channels`);
  }

  const bytesPerSample = header.bitDepth / 8;
  const sampleCount = header.dataLength / bytesPerSample;
  if (sampleCount > MAX_SAMPLES) {
    throw new RangeError(
      `sample count ${String(sampleCount)} exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`,
    );
  }

  const samplesArray: number[] = [];
  const view = new DataView(wavBytes.buffer, wavBytes.byteOffset, wavBytes.byteLength);
  let clippedCount = 0;

  if (header.bitDepth === 16) {
    for (let i = 0; i < header.dataLength; i += 2) {
      const val = view.getInt16(header.dataOffset + i, true);
      samplesArray.push(val);
    }
  } else if (header.bitDepth === 32) {
    for (let i = 0; i < header.dataLength; i += 4) {
      const fval = view.getFloat32(header.dataOffset + i, true);
      const [val, clipped] = floatToPcm16(fval);
      samplesArray.push(val);
      if (clipped) {
        clippedCount += 1;
      }
    }
  }

  const fmt = new PCMFormat(header.sampleRate, 1, 16);
  return new PCMBuffer(samplesArray, fmt, 0.0, clippedCount);
}

/**
 * Encode a PCMBuffer into a standard 44-byte RIFF/WAVE container file.
 */
export function toWavBytes(pcmBuffer: PCMBuffer): Uint8Array {
  if (!(pcmBuffer instanceof PCMBuffer)) {
    throw new TypeError("pcmBuffer must be an instance of PCMBuffer");
  }

  const count = pcmBuffer.sampleCount();
  const dataSize = computeDataSize(count, 1, 2);
  const totalFileSize = computeWavFileSize(dataSize);

  const wavBytesArray = new Uint8Array(totalFileSize);
  const view = new DataView(wavBytesArray.buffer);

  // 'RIFF'
  wavBytesArray[0] = 0x52;
  wavBytesArray[1] = 0x49;
  wavBytesArray[2] = 0x46;
  wavBytesArray[3] = 0x46;
  view.setUint32(4, totalFileSize - 8, true);

  // 'WAVE'
  wavBytesArray[8] = 0x57;
  wavBytesArray[9] = 0x41;
  wavBytesArray[10] = 0x56;
  wavBytesArray[11] = 0x45;

  // 'fmt '
  wavBytesArray[12] = 0x66;
  wavBytesArray[13] = 0x6d;
  wavBytesArray[14] = 0x74;
  wavBytesArray[15] = 0x20;
  view.setUint32(16, 16, true); // Subchunk1Size = 16
  view.setUint16(20, 1, true); // AudioFormat = 1 (PCM)
  view.setUint16(22, 1, true); // NumChannels = 1
  view.setUint32(24, pcmBuffer.pcmFormat.integerSampleRate(), true); // SampleRate
  view.setUint32(28, pcmBuffer.pcmFormat.integerSampleRate() * 2, true); // ByteRate
  view.setUint16(32, 2, true); // BlockAlign = 2
  view.setUint16(34, 16, true); // BitsPerSample = 16

  // 'data'
  wavBytesArray[36] = 0x64;
  wavBytesArray[37] = 0x61;
  wavBytesArray[38] = 0x74;
  wavBytesArray[39] = 0x61;
  view.setUint32(40, dataSize, true);

  // Write PCM16 samples
  for (let i = 0; i < count; i += 1) {
    view.setInt16(44 + i * 2, pcmBuffer.samplesArray[i], true);
  }

  return wavBytesArray;
}

function positiveFloat(name: string, value: number): number {
  const converted = finiteFloat(name, value);
  if (converted <= 0.0) {
    throw new RangeError(`${name} must be > 0.0, got ${String(converted)}`);
  }
  return converted;
}

function validateSampleRate(name: string, value: number): number {
  const converted = positiveFloat(name, value);
  if (converted > MAX_SAMPLE_RATE_HZ) {
    throw new RangeError(
      `${name} ${String(converted)} exceeds MAX_SAMPLE_RATE_HZ=${String(MAX_SAMPLE_RATE_HZ)}`,
    );
  }
  return converted;
}

function finiteFloat(name: string, value: number): number {
  if (typeof value !== "number" || Number.isNaN(value) || !Number.isFinite(value)) {
    throw new TypeError(`${name} must be a finite real number, got ${safeFormatValue(value)}`);
  }
  return value;
}

function nonNegativeInt(name: string, value: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    throw new TypeError(`${name} must be an integer >= 0, got ${safeFormatValue(value)}`);
  }
  if (value < 0) {
    throw new RangeError(`${name} must be >= 0, got ${String(value)}`);
  }
  return value;
}

function positiveInt(name: string, value: number): number {
  const converted = nonNegativeInt(name, value);
  if (converted === 0) {
    throw new RangeError(`${name} must be > 0, got ${String(converted)}`);
  }
  return converted;
}

function integerSampleRate(sampleRateHz: number): number {
  const rounded = Math.round(sampleRateHz);
  if (Math.abs(sampleRateHz - rounded) > INTEGER_TOLERANCE) {
    throw new RangeError(
      `sampleRateHz must be integer-valued for WAV output, got ${String(sampleRateHz)}`,
    );
  }
  return rounded;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function validateIntegerSamples(samples: Iterable<number>): number[] {
  if (!isIterable(samples)) {
    throw new TypeError("samples must be an iterable of integers");
  }
  if (
    (Array.isArray(samples) || ArrayBuffer.isView(samples)) &&
    "length" in samples &&
    typeof samples.length === "number" &&
    samples.length > MAX_SAMPLES
  ) {
    throw new RangeError(`sample count exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`);
  }
  const checkedArray: number[] = [];
  for (const sample of samples) {
    if (checkedArray.length >= MAX_SAMPLES) {
      throw new RangeError(`sample count exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`);
    }
    if (typeof sample !== "number" || !Number.isSafeInteger(sample) || sample < PCM16_MIN || sample > PCM16_MAX) {
      throw new RangeError(
        `PCM samples must be signed 16-bit integers in [${String(PCM16_MIN)}, ${String(PCM16_MAX)}], got ${safeFormatValue(
          sample,
        )}`,
      );
    }
    checkedArray.push(sample);
  }
  return checkedArray;
}

function safeFormatValue(value: unknown): string {
  if (typeof value === "symbol") {
    return value.toString().slice(0, 1024);
  }
  if (typeof value === "string") {
    return value.slice(0, 1024);
  }
  try {
    return String(value).slice(0, 1024);
  } catch {
    return Object.prototype.toString.call(value).slice(0, 1024);
  }
}

function isIterable(value: unknown): value is Iterable<number> {
  return value != null && typeof value === "object" && Symbol.iterator in Object(value);
}
