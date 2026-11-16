/**
 * Continuous instrument voice signal generator (MUS05 §10).
 * Strictly iterative, zero allocations in valueAt().
 */

import type { ContinuousSignal } from "@coding-adventures/oscillator";

import {
  DEFAULT_SAMPLE_RATE_HZ,
  MAX_NOTE_DURATION_SECONDS,
  MAX_PARTIALS,
  TWO_PI,
} from "./constants.js";
import { sin } from "trig/dist/trig.js";
import { AdsrEnvelope } from "./envelope.js";
import { HarmonicPartial } from "./partial.js";
import { InstrumentProfile } from "./profile.js";
import { SplitMix64 } from "./splitmix64.js";
import {
  validateDuration,
  validateFinite,
  validateNonNegative,
  validatePositive,
  validateSampleRate,
  validateUnit,
} from "./validation.js";
import { VariationProfile } from "./variation.js";

export class InstrumentSignal implements ContinuousSignal {
  readonly fundamentalHz: number;
  readonly instrument: InstrumentProfile;
  readonly noteDurationSeconds: number;
  readonly amplitude: number;
  readonly sampleRateHz: number;
  readonly variationProfile: VariationProfile;
  readonly effectiveFundamentalHz: number;
  readonly effectiveAmplitude: number;
  readonly effectiveEnvelope: AdsrEnvelope;
  readonly effectiveHarmonicProfile: readonly HarmonicPartial[];
  private readonly normalizer: number;

  constructor(
    fundamentalHz: number,
    instrument: InstrumentProfile,
    noteDurationSeconds: number,
    amplitude = 1.0,
    sampleRateHz = DEFAULT_SAMPLE_RATE_HZ,
    variationProfile?: VariationProfile,
  ) {
    const fundHz = validatePositive("fundamentalHz", fundamentalHz);
    if (!(instrument instanceof InstrumentProfile)) {
      throw new TypeError("instrument must be an InstrumentProfile");
    }
    const dur = validateDuration("noteDurationSeconds", noteDurationSeconds);
    const amp = validateUnit("amplitude", amplitude);
    const sr = validateSampleRate("sampleRateHz", sampleRateHz);

    if (instrument.harmonicProfile.length > MAX_PARTIALS) {
      throw new RangeError(
        `instrument.harmonicProfile length ${String(instrument.harmonicProfile.length)} exceeds MAX_PARTIALS=${String(MAX_PARTIALS)}`,
      );
    }

    const varProfile =
      variationProfile instanceof VariationProfile
        ? variationProfile
        : instrument.variationProfile;

    let effFund = fundHz;
    let effAmp = amp;
    let effEnv = instrument.envelopeProfile;
    let effPartials = instrument.harmonicProfile;

    if (varProfile.seed !== null) {
      const rng = new SplitMix64(varProfile.seed);

      // 1. Pitch jitter in cents: f' = f * 2^(cents / 1200)
      const pitchDraw = rng.nextFloatSigned();
      const deltaCents = pitchDraw * varProfile.pitchCentsJitter;
      const ratio = 2.0 ** (deltaCents / 1200.0);
      effFund = fundHz * ratio;
      if (!Number.isFinite(effFund) || effFund <= 0.0) {
        throw new RangeError(`Jittered fundamental frequency must be positive finite, got ${String(effFund)}`);
      }

      // 2. Amplitude jitter
      const ampDraw = rng.nextFloatSigned();
      const ampFactor = 1.0 + ampDraw * varProfile.amplitudeJitter;
      effAmp = Math.max(0.0, Math.min(1.0, amp * ampFactor));

      // 3. Attack timing jitter (ms to seconds)
      const atkDraw = rng.nextFloatSigned();
      const atkDelta = atkDraw * (varProfile.attackJitterMs / 1000.0);
      const baseEnv = instrument.envelopeProfile;
      const effAtk = Math.max(
        0.0,
        Math.min(MAX_NOTE_DURATION_SECONDS, baseEnv.attackSeconds + atkDelta),
      );
      effEnv = new AdsrEnvelope(
        effAtk,
        baseEnv.decaySeconds,
        baseEnv.sustainLevel,
        baseEnv.releaseSeconds,
      );

      // 4. Harmonic overtone jitter
      if (varProfile.harmonicJitter > 0.0) {
        const varied: HarmonicPartial[] = [];
        for (const p of instrument.harmonicProfile) {
          const hDraw = rng.nextFloatSigned();
          const pFactor = 1.0 + hDraw * varProfile.harmonicJitter;
          const pAmp = Math.max(0.0, p.amplitude * pFactor);
          varied.push(new HarmonicPartial(p.frequencyMultiplier, pAmp, p.phaseCycles));
        }
        effPartials = Object.freeze(varied);
      }
    }

    let totalAmp = 0.0;
    for (const p of effPartials) {
      totalAmp += p.amplitude;
    }
    validateFinite("amplitude_normalizer", totalAmp);
    const norm = totalAmp > 0.0 ? totalAmp : 1.0;

    this.fundamentalHz = fundHz;
    this.instrument = instrument;
    this.noteDurationSeconds = dur;
    this.amplitude = amp;
    this.sampleRateHz = sr;
    this.variationProfile = varProfile;
    this.effectiveFundamentalHz = effFund;
    this.effectiveAmplitude = effAmp;
    this.effectiveEnvelope = effEnv;
    this.effectiveHarmonicProfile = effPartials;
    this.normalizer = norm;
  }

  valueAt(timeSeconds: number): number {
    const time = validateNonNegative("timeSeconds", timeSeconds);

    if (this.instrument.synthesisKind === "silence") {
      return 0.0;
    }

    const nyquistHz = this.sampleRateHz / 2.0;
    let raw = 0.0;
    const effFund = this.effectiveFundamentalHz;

    for (const partial of this.effectiveHarmonicProfile) {
      const partialHz = effFund * partial.frequencyMultiplier;
      if (partialHz >= nyquistHz) {
        continue;
      }
      raw += partial.amplitude * sin(TWO_PI * (partialHz * time + partial.phaseCycles));
    }

    const envelope = this.effectiveEnvelope.valueAt(time, this.noteDurationSeconds);
    const sample = (this.effectiveAmplitude * this.instrument.gain * envelope * raw) / this.normalizer;

    if (!Number.isFinite(sample)) {
      return 0.0;
    }
    return sample;
  }
}
