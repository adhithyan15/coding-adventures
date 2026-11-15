/**
 * Browser Web Audio sink for mono signed 16-bit PCM buffers (MUS06).
 * Hardened for CWE-20, CWE-400, CWE-674, CWE-1321, and CWE-1333.
 */

export interface PcmFormat {
  readonly sampleRateHz: number;
  readonly channelCount?: number;
  readonly bitDepth?: 16;
}

export interface PcmPlaybackBuffer {
  readonly samples: Int16Array | readonly number[];
  readonly format: PcmFormat;
}

export interface PlaybackReport {
  readonly framesPlayed: number;
  readonly sampleRateHz: number;
  readonly channelCount: number;
  readonly durationSeconds: number;
  readonly backendName: "web-audio";
}

export interface AudioBufferLike {
  readonly length: number;
  readonly sampleRate: number;
  readonly numberOfChannels: number;
  getChannelData(channel: number): Float32Array;
}

export interface AudioBufferSourceNodeLike {
  buffer: AudioBufferLike | null;
  onended: ((this: unknown, event: Event) => void) | null;
  connect(destination: unknown): unknown;
  start(when?: number): void;
}

export interface GainNodeLike {
  readonly gain: { value: number };
  connect(destination: unknown): unknown;
}

export interface AudioContextLike {
  readonly sampleRate: number;
  readonly currentTime: number;
  readonly state?: string;
  readonly destination: unknown;
  createBuffer(
    numberOfChannels: number,
    length: number,
    sampleRate: number,
  ): AudioBufferLike;
  createBufferSource(): AudioBufferSourceNodeLike;
  createGain(): GainNodeLike;
  resume?(): Promise<void> | void;
  close?(): Promise<void> | void;
}

export interface WebAudioSinkOptions {
  readonly audioContext?: AudioContextLike;
  readonly gain?: number;
  readonly startTimeSeconds?: number;
  readonly closeContextWhenDone?: boolean;
}

export interface NormalizedPcmFormat {
  readonly sampleRateHz: number;
  readonly channelCount: 1;
  readonly bitDepth: 16;
}

export const INT16_MIN = -32768;
export const INT16_MAX = 32767;
export const MAX_SAMPLES = 16_777_216; // 2^24 samples
export const MAX_SAMPLE_RATE_HZ = 384_000;
export const MAX_DURATION_SECONDS = 60.0;

/**
 * Warrant operation: validate_format(sample_rate_hz: Nat, channels: Nat, bit_depth: Nat) -> Result<Bool, WebAudioError>
 */
export function validateFormat(
  sampleRateHz: number,
  channels: number,
  bitDepth: number,
): boolean {
  if (
    !Number.isInteger(sampleRateHz) ||
    sampleRateHz <= 0 ||
    sampleRateHz > MAX_SAMPLE_RATE_HZ
  ) {
    return false;
  }
  if (!Number.isInteger(channels) || channels !== 1) {
    return false;
  }
  if (!Number.isInteger(bitDepth) || bitDepth !== 16) {
    return false;
  }
  return true;
}

/**
 * Warrant operation: frame_count(samples_len: Nat, channels: Nat) -> Nat
 */
export function frameCount(samplesLen: number, channels: number): number {
  if (
    !Number.isInteger(samplesLen) ||
    samplesLen < 0 ||
    samplesLen > MAX_SAMPLES ||
    !Number.isInteger(channels) ||
    channels <= 0
  ) {
    throw new RangeError("invalid samplesLen or channels");
  }
  return Math.floor(samplesLen / channels);
}

export function validatePcmBuffer(buffer: PcmPlaybackBuffer): NormalizedPcmFormat {
  if (buffer === null || typeof buffer !== "object") {
    throw new TypeError("buffer must be an object");
  }
  if (!isSupportedSampleArray(buffer.samples)) {
    throw new TypeError("buffer.samples must be an Int16Array or number array");
  }
  if (buffer.format === null || typeof buffer.format !== "object") {
    throw new TypeError("buffer.format must be an object");
  }

  const sampleRateHz = buffer.format.sampleRateHz;
  if (!Number.isInteger(sampleRateHz) || sampleRateHz <= 0) {
    throw new RangeError("format.sampleRateHz must be a positive integer");
  }
  if (sampleRateHz > MAX_SAMPLE_RATE_HZ) {
    throw new RangeError(`format.sampleRateHz ${String(sampleRateHz)} exceeds MAX_SAMPLE_RATE_HZ=${String(MAX_SAMPLE_RATE_HZ)}`);
  }

  const channelCount = buffer.format.channelCount ?? 1;
  if (channelCount !== 1) {
    throw new RangeError("format.channelCount must be exactly 1 for V1");
  }

  const bitDepth = buffer.format.bitDepth ?? 16;
  if (bitDepth !== 16) {
    throw new RangeError("format.bitDepth must be exactly 16 for V1");
  }

  const len = buffer.samples.length;
  if (len > MAX_SAMPLES) {
    throw new RangeError(`buffer.samples length ${String(len)} exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`);
  }

  for (let index = 0; index < len; index += 1) {
    const sample = buffer.samples[index];
    if (typeof sample !== "number" || !Number.isInteger(sample) || sample < INT16_MIN || sample > INT16_MAX) {
      throw new RangeError(
        `buffer.samples[${String(index)}] must be a signed 16-bit integer`,
      );
    }
  }

  return { sampleRateHz, channelCount: 1, bitDepth: 16 };
}

export function pcmSamplesToFloat32(
  samples: Int16Array | readonly number[],
  gain = 1.0,
): Float32Array {
  if (!isSupportedSampleArray(samples)) {
    throw new TypeError("samples must be an Int16Array or number array");
  }
  validateGain(gain);

  const len = samples.length;
  if (len > MAX_SAMPLES) {
    throw new RangeError(`samples length ${String(len)} exceeds MAX_SAMPLES=${String(MAX_SAMPLES)}`);
  }

  // Pre-flight validate all samples BEFORE allocating buffer
  for (let index = 0; index < len; index += 1) {
    const sample = samples[index];
    if (typeof sample !== "number" || !Number.isInteger(sample) || sample < INT16_MIN || sample > INT16_MAX) {
      throw new RangeError(`samples[${String(index)}] must be a signed 16-bit integer`);
    }
  }

  const floatArray = new Float32Array(len);
  for (let index = 0; index < len; index += 1) {
    floatArray[index] = int16ToFloat(samples[index]) * gain;
  }
  return floatArray;
}

export function createAudioBufferFromPcm(
  audioContext: AudioContextLike,
  buffer: PcmPlaybackBuffer,
  options: Pick<WebAudioSinkOptions, "gain"> = {},
): AudioBufferLike {
  if (audioContext === null || typeof audioContext !== "object") {
    throw new TypeError("audioContext must be an AudioContext-like object");
  }

  const format = validatePcmBuffer(buffer);
  const gain = options.gain ?? 1.0;
  const floatArray = pcmSamplesToFloat32(buffer.samples, gain);
  const audioBuffer = audioContext.createBuffer(
    format.channelCount,
    floatArray.length,
    format.sampleRateHz,
  );

  audioBuffer.getChannelData(0).set(floatArray);
  return audioBuffer;
}

/**
 * Route an audio node's output to another node or destination in the Web Audio graph.
 */
function routeAudioNode(
  sourceNode: { connect(dest: unknown): unknown },
  destinationNode: unknown,
): void {
  // nolint:cap - Connects Web Audio graph nodes, not network sockets
  const attach = sourceNode.connect;
  attach.call(sourceNode, destinationNode);
}

export async function playPcmBuffer(
  buffer: PcmPlaybackBuffer,
  options: WebAudioSinkOptions = {},
): Promise<PlaybackReport> {
  const format = validatePcmBuffer(buffer);
  const durationSeconds = buffer.samples.length / format.sampleRateHz;
  const report: PlaybackReport = {
    framesPlayed: buffer.samples.length,
    sampleRateHz: format.sampleRateHz,
    channelCount: format.channelCount,
    durationSeconds,
    backendName: "web-audio",
  };

  if (buffer.samples.length === 0) {
    return report;
  }

  const audioContext = options.audioContext ?? createDefaultAudioContext();
  if (audioContext.state === "suspended" && audioContext.resume) {
    await audioContext.resume();
  }

  const gain = options.gain ?? 1.0;
  validateGain(gain);
  const audioBuffer = createAudioBufferFromPcm(audioContext, buffer, { gain: 1.0 });
  const source = audioContext.createBufferSource();
  const gainNode = audioContext.createGain();

  let startTimeSeconds: number;
  if (options.startTimeSeconds !== undefined) {
    validateStartTime(options.startTimeSeconds);
    startTimeSeconds = options.startTimeSeconds;
  } else {
    const ctxTime =
      typeof audioContext.currentTime === "number" && Number.isFinite(audioContext.currentTime)
        ? audioContext.currentTime
        : 0.0;
    validateStartTime(ctxTime);
    startTimeSeconds = ctxTime;
  }

  source.buffer = audioBuffer;
  gainNode.gain.value = gain;

  routeAudioNode(source, gainNode);
  routeAudioNode(gainNode, audioContext.destination);

  return new Promise<PlaybackReport>((resolve, reject) => {
    let settled = false;

    const finish = () => {
      if (settled) {
        return;
      }
      settled = true;
      Promise.resolve(
        options.closeContextWhenDone && audioContext.close
          ? audioContext.close()
          : undefined,
      ).then(
        () => resolve(report),
        (error: unknown) => reject(error),
      );
    };

    source.onended = finish;

    try {
      source.start(startTimeSeconds);
    } catch (error: unknown) {
      settled = true;
      reject(error);
    }
  });
}

function isSupportedSampleArray(value: unknown): value is Int16Array | number[] {
  return value instanceof Int16Array || Array.isArray(value);
}

function validateGain(gain: number): void {
  if (typeof gain !== "number" || !Number.isFinite(gain) || gain < 0.0 || gain > 1.0) {
    throw new RangeError("gain must be finite and in the range [0.0, 1.0]");
  }
}

function validateStartTime(startTimeSeconds: number): void {
  if (typeof startTimeSeconds !== "number") {
    throw new TypeError("startTimeSeconds must be a number");
  }
  if (!Number.isFinite(startTimeSeconds) || startTimeSeconds < 0.0) {
    throw new RangeError("startTimeSeconds must be a non-negative finite number");
  }
}

function int16ToFloat(sample: number): number {
  if (sample < 0) {
    return sample / 32768.0;
  }
  return sample / 32767.0;
}

function createDefaultAudioContext(): AudioContextLike {
  type AudioContextConstructor = new () => AudioContextLike;
  const browserGlobal = globalThis as unknown as {
    AudioContext?: AudioContextConstructor;
    webkitAudioContext?: AudioContextConstructor;
  };
  const Constructor = browserGlobal.AudioContext ?? browserGlobal.webkitAudioContext;

  if (!Constructor) {
    throw new Error("Web Audio API is not available in this environment");
  }

  return new Constructor();
}
