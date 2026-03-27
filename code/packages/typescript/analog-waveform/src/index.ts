export interface AnalogWaveform {
  sampleAt(timeSeconds: number): number;
}

export class ConstantWaveform implements AnalogWaveform {
  readonly level: number;

  constructor(level: number) {
    this.level = level;
  }

  sampleAt(_timeSeconds: number): number {
    return this.level;
  }
}

export class SineWaveform implements AnalogWaveform {
  readonly amplitude: number;
  readonly frequencyHz: number;
  readonly phaseRadians: number;
  readonly offset: number;

  constructor(
    amplitude: number,
    frequencyHz: number,
    phaseRadians: number = 0,
    offset: number = 0
  ) {
    if (amplitude < 0) {
      throw new Error(`Amplitude must be >= 0, got ${amplitude}.`);
    }

    if (frequencyHz <= 0) {
      throw new Error(`Frequency must be > 0, got ${frequencyHz}.`);
    }

    this.amplitude = amplitude;
    this.frequencyHz = frequencyHz;
    this.phaseRadians = phaseRadians;
    this.offset = offset;
  }

  periodSeconds(): number {
    return 1 / this.frequencyHz;
  }

  angularFrequency(): number {
    return 2 * Math.PI * this.frequencyHz;
  }

  sampleAt(timeSeconds: number): number {
    return (
      this.offset +
      this.amplitude *
        Math.sin(2 * Math.PI * this.frequencyHz * timeSeconds + this.phaseRadians)
    );
  }
}
