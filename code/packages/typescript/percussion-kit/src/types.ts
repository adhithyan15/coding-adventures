/**
 * Types and interfaces for unpitched percussion synthesis and drum kit events (MUS07).
 */

import type { DrumKitProfile } from "./kit.js";

export type NoiseAlgorithm = "pseudo_noise" | "splitmix64";

export interface HitRenderOptions {
  readonly kit?: DrumKitProfile;
  readonly sampleRateHz?: number;
  readonly amplitude?: number;
  readonly maxSampleCount?: number;
  readonly maxModeCount?: number;
}

export interface SequenceRenderOptions {
  readonly kit?: DrumKitProfile;
  readonly sampleRateHz?: number;
  readonly chokeRampSeconds?: number;
  readonly maxSampleCount?: number;
  readonly maxSequenceEvents?: number;
  readonly maxModeCount?: number;
}
