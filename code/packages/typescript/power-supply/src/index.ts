import { ConstantWaveform, SineWaveform } from "@coding-adventures/analog-waveform";

export class IdealDcSupply {
  readonly voltage: number;

  constructor(voltage: number) {
    this.voltage = voltage;
  }

  asWaveform(): ConstantWaveform {
    return new ConstantWaveform(this.voltage);
  }

  powerForCurrent(currentAmps: number): number {
    return this.voltage * currentAmps;
  }
}

export class IdealSineSupply {
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
    this.amplitude = amplitude;
    this.frequencyHz = frequencyHz;
    this.phaseRadians = phaseRadians;
    this.offset = offset;
  }

  asWaveform(): SineWaveform {
    return new SineWaveform(
      this.amplitude,
      this.frequencyHz,
      this.phaseRadians,
      this.offset
    );
  }
}
