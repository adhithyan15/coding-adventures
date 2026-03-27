import type { AnalogWaveform } from "@coding-adventures/analog-waveform";

export class IdealResistor {
  readonly resistanceOhms: number;
  readonly tolerance: number;

  constructor(resistanceOhms: number, tolerance: number = 0) {
    if (resistanceOhms <= 0) {
      throw new Error(`Resistance must be > 0 ohms, got ${resistanceOhms}.`);
    }

    if (tolerance < 0) {
      throw new Error(`Tolerance must be >= 0, got ${tolerance}.`);
    }

    this.resistanceOhms = resistanceOhms;
    this.tolerance = tolerance;
  }

  conductance(): number {
    return 1 / this.resistanceOhms;
  }

  currentForVoltage(voltage: number): number {
    return voltage / this.resistanceOhms;
  }

  voltageForCurrent(current: number): number {
    return current * this.resistanceOhms;
  }

  powerForVoltage(voltage: number): number {
    return (voltage * voltage) / this.resistanceOhms;
  }

  powerForCurrent(current: number): number {
    return current * current * this.resistanceOhms;
  }

  minResistance(): number {
    return this.resistanceOhms * (1 - this.tolerance);
  }

  maxResistance(): number {
    return this.resistanceOhms * (1 + this.tolerance);
  }
}

export interface TimePoint {
  timeSeconds: number;
  value: number;
}

export interface ResistorWaveResponse {
  voltage: TimePoint[];
  current: TimePoint[];
  instantaneousPower: TimePoint[];
}

export function analyzeDcResistor(
  sourceVoltage: number,
  resistor: IdealResistor
): {
  voltage: number;
  current: number;
  power: number;
} {
  return {
    voltage: sourceVoltage,
    current: resistor.currentForVoltage(sourceVoltage),
    power: resistor.powerForVoltage(sourceVoltage),
  };
}

export function analyzeVoltageDivider(
  inputVoltage: number,
  top: IdealResistor,
  bottom: IdealResistor
): number {
  return inputVoltage * (bottom.resistanceOhms / (top.resistanceOhms + bottom.resistanceOhms));
}

export function sampleVoltageDrivenResistor(
  waveform: AnalogWaveform,
  resistor: IdealResistor,
  durationSeconds: number,
  sampleCount: number
): ResistorWaveResponse {
  if (durationSeconds <= 0) {
    throw new Error(`Duration must be > 0, got ${durationSeconds}.`);
  }

  if (sampleCount <= 1) {
    throw new Error(`Sample count must be > 1, got ${sampleCount}.`);
  }

  const times = Array.from({ length: sampleCount }, (_, index) =>
    (durationSeconds * index) / (sampleCount - 1)
  );

  const voltage = times.map((timeSeconds) => ({
    timeSeconds,
    value: waveform.sampleAt(timeSeconds),
  }));

  const current = voltage.map(({ timeSeconds, value }) => ({
    timeSeconds,
    value: resistor.currentForVoltage(value),
  }));

  const instantaneousPower = voltage.map(({ timeSeconds, value }) => ({
    timeSeconds,
    value: resistor.powerForVoltage(value),
  }));

  return { voltage, current, instantaneousPower };
}
