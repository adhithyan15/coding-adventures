/**
 * Continuous signal generator for individual percussion hits (MUS07 §4, §9).
 * Zero allocations in valueAt().
 */

import { sin } from "trig/dist/trig.js";
import {
  type ContinuousSignal,
  SampleBuffer,
  UniformSampler,
} from "@coding-adventures/oscillator";

import {
  DEFAULT_SAMPLE_RATE_HZ,
  MAX_DURATION_SECONDS,
  MAX_SAMPLE_COUNT,
} from "./constants.js";
import { PercussionVoiceProfile } from "./profile.js";
import {
  safeRoundSample,
  validateDuration,
  validateNonNegative,
  validateSampleCountCeiling,
  validateSampleRate,
  validateUnitFloat,
} from "./validation.js";

const TWO_PI = 2.0 * Math.PI;

export class PercussionSignal implements ContinuousSignal {
  readonly voice: PercussionVoiceProfile;
  readonly hitDurationSeconds: number;
  readonly amplitude: number;
  readonly sampleRateHz: number;

  constructor(
    voice: PercussionVoiceProfile,
    hitDurationSeconds: number,
    amplitude = 1.0,
    sampleRateHz = DEFAULT_SAMPLE_RATE_HZ,
  ) {
    if (!(voice instanceof PercussionVoiceProfile)) {
      throw new TypeError("voice must be a PercussionVoiceProfile");
    }
    this.voice = voice;
    this.hitDurationSeconds = validateDuration(
      "hitDurationSeconds",
      hitDurationSeconds,
    );
    this.amplitude = validateUnitFloat("amplitude", amplitude);
    this.sampleRateHz = validateSampleRate("sampleRateHz", sampleRateHz);
  }

  durationSeconds(): number {
    return this.hitDurationSeconds;
  }

  valueAt(timeSeconds: number): number {
    const time = validateNonNegative("timeSeconds", timeSeconds);

    const envelope = this.voice.envelope.valueAt(time, this.hitDurationSeconds);
    if (envelope <= 0.0) {
      return 0.0;
    }

    const sampleIndex = safeRoundSample(time, this.sampleRateHz);

    let raw = 0.0;
    if (this.voice.clickAmplitude > 0.0 && time < 0.002) {
      raw += this.voice.clickAmplitude * (1.0 - time / 0.002);
    }

    for (const mode of this.voice.modes) {
      raw +=
        mode.amplitude *
        Math.exp(-time / mode.decaySeconds) *
        sin(TWO_PI * (mode.frequencyHz * time + mode.phaseCycles));
    }

    if (this.voice.noiseProfile !== null) {
      raw +=
        this.voice.noiseProfile.amplitude *
        Math.exp(-time / this.voice.noiseProfile.decaySeconds) *
        this.voice.noiseProfile.sampleNoise(sampleIndex);
    }

    const norm = this.voice.amplitudeNormalizer();
    const sampleVal = (this.amplitude * this.voice.gain * envelope * raw) / norm;

    if (!Number.isFinite(sampleVal)) {
      return 0.0;
    }
    return sampleVal;
  }

  sample(durationSeconds: number, maxSampleCount = MAX_SAMPLE_COUNT): SampleBuffer {
    const dur = validateDuration("durationSeconds", durationSeconds, MAX_DURATION_SECONDS * 2.0);
    const maxSamples = validateSampleCountCeiling("maxSampleCount", maxSampleCount);

    const neededCount = Math.round(dur * this.sampleRateHz);
    if (neededCount > maxSamples) {
      throw new RangeError(
        `Needed sample count ${String(neededCount)} exceeds maxSampleCount=${String(maxSamples)}`,
      );
    }

    const sampler = new UniformSampler(this.sampleRateHz);
    return sampler.sample(this, dur);
  }
}
