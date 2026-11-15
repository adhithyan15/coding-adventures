/**
 * Instrument profile and timbre recipe model (MUS05 §4 & §8).
 */

import { MAX_PARTIALS, SYNTHESIS_KINDS } from "./constants.js";
import { AdsrEnvelope } from "./envelope.js";
import { HarmonicPartial } from "./partial.js";
import type { SynthesisKind } from "./types.js";
import {
  validatePositiveInt,
  validateUnit,
} from "./validation.js";
import { VariationProfile } from "./variation.js";

export class InstrumentProfile {
  readonly id: string;
  readonly displayName: string;
  readonly synthesisKind: SynthesisKind;
  readonly gain: number;
  readonly harmonicProfile: readonly HarmonicPartial[];
  readonly envelopeProfile: AdsrEnvelope;
  readonly variationProfile: VariationProfile;
  readonly gmProgram: number | null;
  readonly family: string;

  constructor(
    id: string,
    displayName: string,
    synthesisKind: SynthesisKind,
    gain: number,
    harmonicProfile: readonly HarmonicPartial[],
    envelopeProfile: AdsrEnvelope,
    variationProfile: VariationProfile = new VariationProfile(),
    gmProgram: number | null = null,
    family = "custom",
  ) {
    if (typeof id !== "string" || id.trim().length === 0) {
      throw new TypeError("id must be a non-empty string");
    }
    if (typeof displayName !== "string" || displayName.trim().length === 0) {
      throw new TypeError("displayName must be a non-empty string");
    }
    if (!SYNTHESIS_KINDS.includes(synthesisKind)) {
      throw new RangeError(`Unsupported synthesisKind: ${String(synthesisKind)}`);
    }
    const validGain = validateUnit("gain", gain);

    if (!Array.isArray(harmonicProfile)) {
      throw new TypeError("harmonicProfile must be an array of HarmonicPartial");
    }
    if (harmonicProfile.length > MAX_PARTIALS) {
      throw new RangeError(
        `harmonicProfile partial count ${String(harmonicProfile.length)} exceeds MAX_PARTIALS=${String(MAX_PARTIALS)}`,
      );
    }
    for (let i = 0; i < harmonicProfile.length; i += 1) {
      if (!(harmonicProfile[i] instanceof HarmonicPartial)) {
        throw new TypeError(`harmonicProfile[${String(i)}] must be a HarmonicPartial`);
      }
    }
    if (synthesisKind !== "silence" && harmonicProfile.length === 0) {
      throw new RangeError("Non-silent instruments must contain at least one harmonic partial");
    }
    if (!(envelopeProfile instanceof AdsrEnvelope)) {
      throw new TypeError("envelopeProfile must be an AdsrEnvelope");
    }
    if (!(variationProfile instanceof VariationProfile)) {
      throw new TypeError("variationProfile must be a VariationProfile");
    }

    let validGmProgram: number | null = null;
    if (gmProgram !== null && gmProgram !== undefined) {
      const prog = validatePositiveInt("gmProgram", gmProgram);
      if (prog < 1 || prog > 128) {
        throw new RangeError(`gmProgram must be in [1, 128], got ${String(prog)}`);
      }
      validGmProgram = prog;
    }

    this.id = id;
    this.displayName = displayName;
    this.synthesisKind = synthesisKind;
    this.gain = validGain;
    this.harmonicProfile = Object.freeze([...harmonicProfile]);
    this.envelopeProfile = envelopeProfile;
    this.variationProfile = variationProfile;
    this.gmProgram = validGmProgram;
    this.family = typeof family === "string" ? family : "custom";
  }

  amplitudeNormalizer(): number {
    let sum = 0.0;
    for (const partial of this.harmonicProfile) {
      sum += partial.amplitude;
    }
    return sum > 0.0 ? sum : 1.0;
  }
}
