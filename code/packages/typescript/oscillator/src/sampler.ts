/**
 * Uniform sampler and sample buffer storage (OSC00 §10-§12).
 */

import { MAX_SAMPLES } from "./constants.js";
import type { ContinuousSignal } from "./types.js";
import {
  sampleCountForDuration,
  timeAtSample,
  validateFinite,
  validateIntegerIndex,
  validateSampleCount,
  validateSampleRate,
} from "./validation.js";

export class SampleBuffer {
  readonly samplesArray: Float64Array;
  readonly sampleRateHz: number;
  readonly startTimeSeconds: number;

  constructor(
    samples: Float64Array | readonly number[],
    sampleRateHz: number,
    startTimeSeconds = 0.0,
  ) {
    if (!samples || typeof samples.length !== "number") {
      throw new TypeError("samples must be a Float64Array or array of numbers");
    }
    const len = validateSampleCount("samples.length", samples.length);

    // Verify all samples are finite BEFORE allocating or copying
    for (let i = 0; i < len; i += 1) {
      const s = samples[i];
      if (typeof s !== "number" || !Number.isFinite(s)) {
        throw new TypeError(`samples[${String(i)}] must be a finite number, got ${String(s)}`);
      }
    }

    const validatedRate = validateSampleRate("sampleRateHz", sampleRateHz);
    const validatedStart = validateFinite("startTimeSeconds", startTimeSeconds);

    if (samples instanceof Float64Array) {
      this.samplesArray = new Float64Array(samples);
    } else {
      const bufferArray = new Float64Array(len);
      for (let i = 0; i < len; i += 1) {
        bufferArray[i] = samples[i];
      }
      this.samplesArray = bufferArray;
    }

    this.sampleRateHz = validatedRate;
    this.startTimeSeconds = validatedStart;
  }

  get samples(): Float64Array {
    return this.samplesArray;
  }

  sampleCount(): number {
    return this.samplesArray.length;
  }

  samplePeriodSeconds(): number {
    return 1.0 / this.sampleRateHz;
  }

  durationSeconds(): number {
    return this.samplesArray.length / this.sampleRateHz;
  }

  timeAt(index: number): number {
    const validIdx = validateIntegerIndex("index", index, this.sampleCount());
    return timeAtSample(validIdx, this.sampleRateHz, this.startTimeSeconds);
  }
}

export class UniformSampler {
  readonly sampleRateHz: number;

  constructor(sampleRateHz: number) {
    this.sampleRateHz = validateSampleRate("sampleRateHz", sampleRateHz);
  }

  sample(
    signal: ContinuousSignal,
    durationSeconds: number,
    startTimeSeconds = 0.0,
  ): SampleBuffer {
    const count = sampleCountForDuration(durationSeconds, this.sampleRateHz);
    return this.sampleCount(signal, count, startTimeSeconds);
  }

  sampleCount(
    signal: ContinuousSignal,
    count: number,
    startTimeSeconds = 0.0,
  ): SampleBuffer {
    const validCount = validateSampleCount("count", count);
    const validStart = validateFinite("startTimeSeconds", startTimeSeconds);

    if (!signal || typeof signal.valueAt !== "function") {
      throw new TypeError("signal must implement ContinuousSignal (valueAt method)");
    }

    const outputSamplesArray = new Float64Array(validCount);
    for (let i = 0; i < validCount; i += 1) {
      const t = timeAtSample(i, this.sampleRateHz, validStart);
      const val = signal.valueAt(t);
      if (!Number.isFinite(val)) {
        throw new RangeError(`signal.valueAt(${String(t)}) returned non-finite value ${String(val)}`);
      }
      outputSamplesArray[i] = val;
    }

    return new SampleBuffer(outputSamplesArray, this.sampleRateHz, validStart);
  }

  *samples(
    signal: ContinuousSignal,
    count: number,
    startTimeSeconds = 0.0,
  ): IterableIterator<number> {
    const validCount = validateSampleCount("count", count);
    const validStart = validateFinite("startTimeSeconds", startTimeSeconds);

    if (!signal || typeof signal.valueAt !== "function") {
      throw new TypeError("signal must implement ContinuousSignal (valueAt method)");
    }

    for (let i = 0; i < validCount; i += 1) {
      const t = timeAtSample(i, this.sampleRateHz, validStart);
      const val = signal.valueAt(t);
      if (!Number.isFinite(val)) {
        throw new RangeError(`signal.valueAt(${String(t)}) returned non-finite value ${String(val)}`);
      }
      yield val;
    }
  }

  nyquistFrequency(): number {
    return this.sampleRateHz / 2.0;
  }
}
