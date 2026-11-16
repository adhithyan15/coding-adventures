/**
 * Percussion voice profile definition and amplitude normalization (MUS07 §5.2).
 */

import { MAX_PERCUSSION_MODES } from "./constants.js";
import { PercussionEnvelope } from "./envelope.js";
import { PercussionMode } from "./mode.js";
import { PercussionNoiseProfile } from "./noise.js";
import {
  validateNonEmptyString,
  validateNonNegative,
  validateUnitFloat,
} from "./validation.js";

export class PercussionVoiceProfile {
  readonly id: string;
  readonly displayName: string;
  readonly family: string;
  readonly gain: number;
  readonly modes: readonly PercussionMode[];
  readonly noiseProfile: PercussionNoiseProfile | null;
  readonly envelope: PercussionEnvelope;
  readonly clickAmplitude: number;
  readonly chokeGroup: string | null;

  constructor(
    id: string,
    displayName: string,
    family: string,
    gain: number,
    modes: readonly PercussionMode[],
    noiseProfile: PercussionNoiseProfile | null,
    envelope: PercussionEnvelope,
    clickAmplitude = 0.0,
    chokeGroup: string | null = null,
  ) {
    this.id = validateNonEmptyString("id", id);
    this.displayName = validateNonEmptyString("displayName", displayName);
    this.family = validateNonEmptyString("family", family);
    this.gain = validateUnitFloat("gain", gain);

    if (!Array.isArray(modes)) {
      throw new TypeError("modes must be an array of PercussionMode");
    }
    if (modes.length > MAX_PERCUSSION_MODES) {
      throw new RangeError(
        `Mode count ${String(modes.length)} exceeds MAX_PERCUSSION_MODES=${String(MAX_PERCUSSION_MODES)}`,
      );
    }
    for (let i = 0; i < modes.length; i += 1) {
      if (!(modes[i] instanceof PercussionMode)) {
        throw new TypeError(`modes[${String(i)}] must be a PercussionMode`);
      }
    }
    this.modes = Object.freeze([...modes]);

    if (noiseProfile !== null && !(noiseProfile instanceof PercussionNoiseProfile)) {
      throw new TypeError("noiseProfile must be a PercussionNoiseProfile or null");
    }
    this.noiseProfile = noiseProfile;

    if (!(envelope instanceof PercussionEnvelope)) {
      throw new TypeError("envelope must be a PercussionEnvelope");
    }
    this.envelope = envelope;
    this.clickAmplitude = validateNonNegative("clickAmplitude", clickAmplitude);

    let choke: string | null = null;
    if (chokeGroup !== null && chokeGroup !== undefined) {
      const trimmed = chokeGroup.trim();
      choke = trimmed.length > 0 ? trimmed : null;
    }
    this.chokeGroup = choke;
  }

  amplitudeNormalizer(): number {
    let total = this.clickAmplitude;
    for (const mode of this.modes) {
      total += mode.amplitude;
    }
    if (this.noiseProfile !== null) {
      total += this.noiseProfile.amplitude;
    }
    return total > 0.0 ? total : 1.0;
  }
}
