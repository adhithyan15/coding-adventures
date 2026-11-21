/**
 * Sampled waveform utilities for DAC and numerical simulation (PHY04, VER15).
 *
 * Hardened against:
 * - CWE-20 (Improper Input Validation): Rejects non-array/sparse sample buffers,
 *   non-finite samples, invalid sample rates, non-integer indices, and non-finite times.
 * - CWE-190 / CWE-400 (Integer Overflow / Resource Exhaustion): Enforces
 *   MAX_SAMPLES before allocation and avoids time * rate overflow in heldValueAt.
 * - CWE-1321 (Prototype Pollution / Tampering): Freezes copied samples and instances,
 *   validates integer index bounds before array indexing, and rejects invalid objects.
 */

import type { AnalogWaveform } from "@coding-adventures/analog-waveform";

export const MAX_SAMPLES = 1_048_576;

function requireFiniteNumber(label: string, value: unknown): number {
  if (typeof value !== "number") {
    throw new TypeError(`${label} must be a finite number`);
  }
  if (!Number.isFinite(value)) {
    throw new RangeError(`${label} must be finite`);
  }
  return value;
}

function validateSampleRate(sampleRateHz: unknown): number {
  const rate = requireFiniteNumber("Sample rate", sampleRateHz);
  if (rate <= 0) {
    throw new RangeError(`Sample rate must be > 0, got ${rate}.`);
  }
  if (!Number.isFinite(1 / rate)) {
    throw new RangeError(`Sample rate ${rate} produces non-finite sample period.`);
  }
  return rate;
}

export class SampledWaveform {
  readonly samples: readonly number[];
  readonly sampleRateHz: number;

  constructor(samples: readonly number[], sampleRateHz: number) {
    if (!Array.isArray(samples)) {
      throw new TypeError("samples must be an array of finite numbers.");
    }
    if (samples.length === 0) {
      throw new RangeError("SampledWaveform requires at least one sample.");
    }
    if (samples.length > MAX_SAMPLES) {
      throw new RangeError(
        `Sample count ${samples.length} exceeds MAX_SAMPLES=${MAX_SAMPLES}.`
      );
    }

    const rate = validateSampleRate(sampleRateHz);
    const copied: number[] = [];
    for (let i = 0; i < samples.length; i += 1) {
      if (!Object.hasOwn(samples, i)) {
        throw new TypeError(`Sample at index ${i} is missing.`);
      }
      const val = requireFiniteNumber(`Sample[${i}]`, samples[i]);
      copied.push(val === 0 ? 0 : val);
    }

    this.samples = Object.freeze(copied);
    this.sampleRateHz = rate;
    Object.freeze(this);
  }

  static fromAnalog(
    waveform: AnalogWaveform,
    sampleRateHz: number,
    sampleCount: number
  ): SampledWaveform {
    if (
      waveform === null ||
      typeof waveform !== "object" ||
      typeof waveform.sampleAt !== "function"
    ) {
      throw new TypeError("waveform must implement sampleAt(timeSeconds).");
    }

    const rate = validateSampleRate(sampleRateHz);
    if (typeof sampleCount !== "number" || !Number.isSafeInteger(sampleCount)) {
      throw new TypeError("Sample count must be a safe integer.");
    }
    if (sampleCount <= 0) {
      throw new RangeError(`Sample count must be > 0, got ${sampleCount}.`);
    }
    if (sampleCount > MAX_SAMPLES) {
      throw new RangeError(
        `Sample count ${sampleCount} exceeds MAX_SAMPLES=${MAX_SAMPLES}.`
      );
    }

    const samples: number[] = [];
    for (let index = 0; index < sampleCount; index += 1) {
      samples.push(waveform.sampleAt(index / rate));
    }
    return new SampledWaveform(samples, rate);
  }

  private validateState(): void {
    if (
      !Array.isArray(this.samples) ||
      this.samples.length === 0 ||
      this.samples.length > MAX_SAMPLES ||
      typeof this.sampleRateHz !== "number" ||
      !Number.isFinite(this.sampleRateHz) ||
      this.sampleRateHz <= 0 ||
      !Number.isFinite(1 / this.sampleRateHz)
    ) {
      throw new RangeError("SampledWaveform state is invalid.");
    }
  }

  private readSample(index: number): number {
    if (!Object.hasOwn(this.samples, index)) {
      throw new RangeError(`Sample at index ${index} is missing.`);
    }
    const val = requireFiniteNumber(`Sample[${index}]`, this.samples[index]);
    return val === 0 ? 0 : val;
  }

  samplePeriodSeconds(): number {
    this.validateState();
    return 1 / this.sampleRateHz;
  }

  durationSeconds(): number {
    this.validateState();
    return this.samples.length / this.sampleRateHz;
  }

  sampleAtIndex(index: number): number {
    this.validateState();
    if (typeof index !== "number" || !Number.isSafeInteger(index)) {
      throw new TypeError("Sample index must be a safe integer.");
    }
    if (index < 0 || index >= this.samples.length) {
      throw new RangeError(`Sample index ${index} is out of range.`);
    }
    return this.readSample(index);
  }

  heldValueAt(timeSeconds: number): number {
    this.validateState();
    const time = requireFiniteNumber("Time", timeSeconds);
    if (time <= 0) {
      return this.readSample(0);
    }
    const duration = this.samples.length / this.sampleRateHz;
    if (time >= duration) {
      return this.readSample(this.samples.length - 1);
    }

    const rawIndex = Math.floor(time * this.sampleRateHz);
    const clampedIndex = Math.max(0, Math.min(this.samples.length - 1, rawIndex));
    return this.readSample(clampedIndex);
  }
}
