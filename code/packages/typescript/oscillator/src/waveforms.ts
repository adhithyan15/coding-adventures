/**
 * Pure continuous waveform oscillators (OSC00).
 */

import { TWO_PI } from "./constants.js";
import type { ContinuousSignal } from "./types.js";
import {
  fractionalPart,
  validateDutyCycle,
  validateFinite,
  validateFrequency,
  validateNonNegative,
} from "./validation.js";

export function evaluateSineRaw(phaseCycles: number): number {
  return Math.sin(TWO_PI * phaseCycles);
}

export function evaluateSquareRaw(pos: number, dutyCycle = 0.5): number {
  return pos < dutyCycle ? 1.0 : -1.0;
}

export function evaluateSawtoothRaw(pos: number): number {
  return 2.0 * pos - 1.0;
}

export function evaluateTriangleRaw(pos: number): number {
  if (pos < 0.25) {
    return 4.0 * pos;
  }
  if (pos < 0.75) {
    return 2.0 - 4.0 * pos;
  }
  return 4.0 * pos - 4.0;
}

export class SineOscillator implements ContinuousSignal {
  readonly frequencyHz: number;
  readonly amplitude: number;
  readonly phaseCycles: number;
  readonly offset: number;

  constructor(
    frequencyHz: number,
    amplitude = 1.0,
    phaseCycles = 0.0,
    offset = 0.0,
  ) {
    this.frequencyHz = validateFrequency("frequencyHz", frequencyHz);
    this.amplitude = validateNonNegative("amplitude", amplitude);
    this.phaseCycles = validateFinite("phaseCycles", phaseCycles);
    this.offset = validateFinite("offset", offset);
  }

  valueAt(timeSeconds: number): number {
    const t = validateFinite("timeSeconds", timeSeconds);
    const phase = this.frequencyHz * t + this.phaseCycles;
    return this.offset + this.amplitude * evaluateSineRaw(phase);
  }
}

export class SquareOscillator implements ContinuousSignal {
  readonly frequencyHz: number;
  readonly low: number;
  readonly high: number;
  readonly dutyCycle: number;
  readonly phaseCycles: number;

  constructor(
    frequencyHz: number,
    low = -1.0,
    high = 1.0,
    dutyCycle = 0.5,
    phaseCycles = 0.0,
  ) {
    this.frequencyHz = validateFrequency("frequencyHz", frequencyHz);
    this.low = validateFinite("low", low);
    this.high = validateFinite("high", high);
    this.dutyCycle = validateDutyCycle("dutyCycle", dutyCycle);
    this.phaseCycles = validateFinite("phaseCycles", phaseCycles);
  }

  valueAt(timeSeconds: number): number {
    const t = validateFinite("timeSeconds", timeSeconds);
    const pos = fractionalPart(this.frequencyHz * t + this.phaseCycles);
    return pos < this.dutyCycle ? this.high : this.low;
  }
}

export class PulseOscillator implements ContinuousSignal {
  readonly frequencyHz: number;
  readonly dutyCycle: number;
  readonly amplitude: number;
  readonly phaseCycles: number;
  readonly offset: number;

  constructor(
    frequencyHz: number,
    dutyCycle = 0.5,
    amplitude = 1.0,
    phaseCycles = 0.0,
    offset = 0.0,
  ) {
    this.frequencyHz = validateFrequency("frequencyHz", frequencyHz);
    this.dutyCycle = validateDutyCycle("dutyCycle", dutyCycle);
    this.amplitude = validateNonNegative("amplitude", amplitude);
    this.phaseCycles = validateFinite("phaseCycles", phaseCycles);
    this.offset = validateFinite("offset", offset);
  }

  valueAt(timeSeconds: number): number {
    const t = validateFinite("timeSeconds", timeSeconds);
    const pos = fractionalPart(this.frequencyHz * t + this.phaseCycles);
    return this.offset + this.amplitude * evaluateSquareRaw(pos, this.dutyCycle);
  }
}

export class SawtoothOscillator implements ContinuousSignal {
  readonly frequencyHz: number;
  readonly amplitude: number;
  readonly phaseCycles: number;
  readonly offset: number;

  constructor(
    frequencyHz: number,
    amplitude = 1.0,
    phaseCycles = 0.0,
    offset = 0.0,
  ) {
    this.frequencyHz = validateFrequency("frequencyHz", frequencyHz);
    this.amplitude = validateNonNegative("amplitude", amplitude);
    this.phaseCycles = validateFinite("phaseCycles", phaseCycles);
    this.offset = validateFinite("offset", offset);
  }

  valueAt(timeSeconds: number): number {
    const t = validateFinite("timeSeconds", timeSeconds);
    const pos = fractionalPart(this.frequencyHz * t + this.phaseCycles);
    return this.offset + this.amplitude * evaluateSawtoothRaw(pos);
  }
}

export class TriangleOscillator implements ContinuousSignal {
  readonly frequencyHz: number;
  readonly amplitude: number;
  readonly phaseCycles: number;
  readonly offset: number;

  constructor(
    frequencyHz: number,
    amplitude = 1.0,
    phaseCycles = 0.0,
    offset = 0.0,
  ) {
    this.frequencyHz = validateFrequency("frequencyHz", frequencyHz);
    this.amplitude = validateNonNegative("amplitude", amplitude);
    this.phaseCycles = validateFinite("phaseCycles", phaseCycles);
    this.offset = validateFinite("offset", offset);
  }

  valueAt(timeSeconds: number): number {
    const t = validateFinite("timeSeconds", timeSeconds);
    const pos = fractionalPart(this.frequencyHz * t + this.phaseCycles);
    return this.offset + this.amplitude * evaluateTriangleRaw(pos);
  }
}
