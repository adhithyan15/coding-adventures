/**
 * Types and interfaces for virtual continuous oscillators and samplers (OSC00).
 */

export interface ContinuousSignal {
  /**
   * Return the instantaneous signal value at time_seconds.
   * Pure function of time: deterministic, no mutation, no side effects.
   */
  valueAt(timeSeconds: number): number;
}

export type WaveformType =
  | "sine"
  | "square"
  | "triangle"
  | "sawtooth"
  | "pulse"
  | "noise";

export enum Waveform {
  Sine = "sine",
  Square = "square",
  Triangle = "triangle",
  Sawtooth = "sawtooth",
  Pulse = "pulse",
  Noise = "noise",
}

export type NoiseType = "splitmix64" | "lcg";

export interface OscillatorConfig {
  readonly waveform?: WaveformType | Waveform;
  readonly frequencyHz?: number;
  readonly sampleRateHz?: number;
  readonly amplitude?: number;
  readonly offset?: number;
  readonly dutyCycle?: number;
  readonly phaseCycles?: number;
  readonly seed?: number | bigint;
  readonly noiseType?: NoiseType;
}
