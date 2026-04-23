export interface PcmFormat {
  sampleRateHz: number;
  channelCount?: number;
  bitDepth?: 16;
}

export interface PcmPlaybackBuffer {
  samples: Int16Array | readonly number[];
  format: PcmFormat;
}

export interface PlaybackReport {
  framesPlayed: number;
  sampleRateHz: number;
  channelCount: number;
  durationSeconds: number;
  backendName: "web-audio";
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
  gain: { value: number };
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
  audioContext?: AudioContextLike;
  gain?: number;
  startTimeSeconds?: number;
  closeContextWhenDone?: boolean;
}

export interface NormalizedPcmFormat {
  sampleRateHz: number;
  channelCount: 1;
  bitDepth: 16;
}

const INT16_MIN = -32768;
const INT16_MAX = 32767;

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

  const channelCount = buffer.format.channelCount ?? 1;
  if (channelCount !== 1) {
    throw new RangeError("format.channelCount must be exactly 1 for V1");
  }

  const bitDepth = buffer.format.bitDepth ?? 16;
  if (bitDepth !== 16) {
    throw new RangeError("format.bitDepth must be exactly 16 for V1");
  }

  for (let index = 0; index < buffer.samples.length; index += 1) {
    const sample = buffer.samples[index];
    if (!Number.isInteger(sample) || sample < INT16_MIN || sample > INT16_MAX) {
      throw new RangeError(
        `buffer.samples[${index}] must be a signed 16-bit integer`,
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

  const floats = new Float32Array(samples.length);
  for (let index = 0; index < samples.length; index += 1) {
    const sample = samples[index];
    if (!Number.isInteger(sample) || sample < INT16_MIN || sample > INT16_MAX) {
      throw new RangeError(`samples[${index}] must be a signed 16-bit integer`);
    }
    floats[index] = int16ToFloat(sample) * gain;
  }
  return floats;
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
  const floats = pcmSamplesToFloat32(buffer.samples, gain);
  const audioBuffer = audioContext.createBuffer(
    format.channelCount,
    floats.length,
    format.sampleRateHz,
  );

  audioBuffer.getChannelData(0).set(floats);
  return audioBuffer;
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
  const audioBuffer = createAudioBufferFromPcm(audioContext, buffer);
  const source = audioContext.createBufferSource();
  const gainNode = audioContext.createGain();
  const startTimeSeconds = options.startTimeSeconds ?? audioContext.currentTime;

  source.buffer = audioBuffer;
  gainNode.gain.value = gain;
  source.connect(gainNode);
  gainNode.connect(audioContext.destination);

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
  if (!Number.isFinite(gain) || gain < 0.0 || gain > 1.0) {
    throw new RangeError("gain must be finite and in the range [0.0, 1.0]");
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
