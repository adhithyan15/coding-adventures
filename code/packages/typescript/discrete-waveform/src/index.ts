import type { AnalogWaveform } from "@coding-adventures/analog-waveform";

export class SampledWaveform {
  readonly samples: number[];
  readonly sampleRateHz: number;

  constructor(samples: number[], sampleRateHz: number) {
    if (samples.length === 0) {
      throw new Error("SampledWaveform requires at least one sample.");
    }

    if (sampleRateHz <= 0) {
      throw new Error(`Sample rate must be > 0, got ${sampleRateHz}.`);
    }

    this.samples = [...samples];
    this.sampleRateHz = sampleRateHz;
  }

  static fromAnalog(
    waveform: AnalogWaveform,
    sampleRateHz: number,
    sampleCount: number
  ): SampledWaveform {
    if (sampleCount <= 0) {
      throw new Error(`Sample count must be > 0, got ${sampleCount}.`);
    }

    const samples = Array.from({ length: sampleCount }, (_, index) =>
      waveform.sampleAt(index / sampleRateHz)
    );
    return new SampledWaveform(samples, sampleRateHz);
  }

  samplePeriodSeconds(): number {
    return 1 / this.sampleRateHz;
  }

  durationSeconds(): number {
    return this.samples.length / this.sampleRateHz;
  }

  sampleAtIndex(index: number): number {
    if (index < 0 || index >= this.samples.length) {
      throw new Error(`Sample index ${index} is out of range.`);
    }
    return this.samples[index]!;
  }

  heldValueAt(timeSeconds: number): number {
    if (timeSeconds <= 0) {
      return this.samples[0]!;
    }

    const rawIndex = Math.floor(timeSeconds * this.sampleRateHz);
    const clampedIndex = Math.max(0, Math.min(this.samples.length - 1, rawIndex));
    return this.samples[clampedIndex]!;
  }
}
