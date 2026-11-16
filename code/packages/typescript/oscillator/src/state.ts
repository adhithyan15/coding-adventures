/**
 * Stateful phase accumulator oscillator and block synthesis (OSC00, CWE-20, CWE-400).
 */

import { LcgPrng, SplitMix64 } from "./prng.js";
import { SampleBuffer } from "./sampler.js";
import type { OscillatorConfig, WaveformType } from "./types.js";
import { Waveform } from "./types.js";
import {
  fractionalPart,
  sampleCountForDuration,
  validateDutyCycle,
  validateFinite,
  validateFrequency,
  validateNonNegative,
  validateSampleCount,
  validateSampleRate,
} from "./validation.js";
import {
  evaluateSawtoothRaw,
  evaluateSineRaw,
  evaluateSquareRaw,
  evaluateTriangleRaw,
} from "./waveforms.js";

export class OscillatorState {
  readonly waveform: WaveformType;
  readonly frequencyHz: number;
  readonly sampleRateHz: number;
  readonly amplitude: number;
  readonly offset: number;
  readonly dutyCycle: number;
  private currentPhase: number;
  private readonly splitMixPrng: SplitMix64 | null;
  private readonly lcgPrng: LcgPrng | null;

  constructor(config: OscillatorConfig = {}) {
    const rawWaveform = config.waveform ?? Waveform.Sine;
    const waveStr = typeof rawWaveform === "string" ? rawWaveform.toLowerCase() : "";

    switch (waveStr) {
      case "sine":
      case "square":
      case "triangle":
      case "sawtooth":
      case "pulse":
      case "noise":
        this.waveform = waveStr as WaveformType;
        break;
      default:
        throw new TypeError(`invalid waveform '${String(rawWaveform)}'`);
    }

    this.frequencyHz = validateFrequency("frequencyHz", config.frequencyHz ?? 440.0);
    this.sampleRateHz = validateSampleRate("sampleRateHz", config.sampleRateHz ?? 44_100.0);
    this.amplitude = validateNonNegative("amplitude", config.amplitude ?? 1.0);
    this.offset = validateFinite("offset", config.offset ?? 0.0);
    this.dutyCycle = validateDutyCycle("dutyCycle", config.dutyCycle ?? 0.5);

    const initialPhase = validateFinite("phaseCycles", config.phaseCycles ?? 0.0);
    this.currentPhase = fractionalPart(initialPhase);

    const noiseType = config.noiseType ?? "splitmix64";
    const seed = config.seed ?? 0n;

    if (noiseType !== "splitmix64" && noiseType !== "lcg") {
      throw new RangeError(
        `noiseType must be 'splitmix64' or 'lcg', got '${String(noiseType)}'`,
      );
    }

    if (noiseType === "lcg") {
      const numSeed = typeof seed === "bigint" ? Number(BigInt.asUintN(32, seed)) : Number(seed);
      this.lcgPrng = new LcgPrng(numSeed);
      this.splitMixPrng = null;
    } else {
      this.splitMixPrng = new SplitMix64(seed);
      this.lcgPrng = null;
    }
  }

  get phase(): number {
    return this.currentPhase;
  }

  reset(phaseCycles = 0.0): void {
    const p = validateFinite("phaseCycles", phaseCycles);
    this.currentPhase = fractionalPart(p);
  }

  /**
   * Advance the phase accumulator by one sample step and return the instantaneous value.
   */
  step(): number {
    const pos = this.currentPhase;
    const delta = this.frequencyHz / this.sampleRateHz;
    this.currentPhase = fractionalPart(this.currentPhase + delta);

    let raw: number;
    switch (this.waveform) {
      case "sine":
        raw = evaluateSineRaw(pos);
        break;
      case "square":
      case "pulse":
        raw = evaluateSquareRaw(pos, this.dutyCycle);
        break;
      case "sawtooth":
        raw = evaluateSawtoothRaw(pos);
        break;
      case "triangle":
        raw = evaluateTriangleRaw(pos);
        break;
      case "noise":
        raw = this.splitMixPrng !== null ? this.splitMixPrng.nextFloat() : this.lcgPrng!.nextFloat();
        break;
    }

    return this.offset + this.amplitude * raw;
  }

  /**
   * Render an array of consecutive samples, advancing the internal phase accumulator.
   */
  renderSamples(count: number): Float64Array {
    const validCount = validateSampleCount("count", count);
    const samplesArray = new Float64Array(validCount);
    for (let i = 0; i < validCount; i += 1) {
      samplesArray[i] = this.step();
    }
    return samplesArray;
  }
}

/**
 * Render samples from a given oscillator configuration without retaining state afterwards.
 */
export function renderSamples(config: OscillatorConfig, count: number): Float64Array {
  const osc = new OscillatorState(config);
  return osc.renderSamples(count);
}

/**
 * Synthesize a fixed duration of audio into a SampleBuffer.
 */
export function synthesize(
  config: OscillatorConfig,
  durationSeconds: number,
  startTimeSeconds = 0.0,
): SampleBuffer {
  const sampleRateHz = config.sampleRateHz ?? 44_100.0;
  const count = sampleCountForDuration(durationSeconds, sampleRateHz);
  const osc = new OscillatorState(config);
  const renderedSamplesArray = osc.renderSamples(count);
  return new SampleBuffer(renderedSamplesArray, sampleRateHz, startTimeSeconds);
}
