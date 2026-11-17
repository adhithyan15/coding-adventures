/**
 * Types and interfaces for naive musical instruments and timbre models (MUS05).
 */

import type { Note } from "./note.js";
import type { InstrumentProfile } from "./profile.js";
import type { VariationProfile } from "./variation.js";

export type SynthesisKind = "sine" | "additive" | "silence";

export type NoteInput = Note | string;

export interface VariationProfileOptions {
  readonly pitchCentsJitter?: number;
  readonly amplitudeJitter?: number;
  readonly attackJitterMs?: number;
  readonly seed?: number | bigint | null;
  readonly pitchJitterCents?: number;
  readonly timingJitterSeconds?: number;
  readonly harmonicJitter?: number;
}

export interface RenderOptions {
  /** Explicit instrument profile. If omitted, instrumentId is consulted. */
  readonly instrument?: InstrumentProfile;
  /** Built-in instrument preset identifier (default "sine"). */
  readonly instrumentId?: string;
  /** Audio sampling rate in Hertz (default 44100.0). */
  readonly sampleRateHz?: number;
  /** Amplitude / velocity in [0.0, 1.0] (default 1.0). */
  readonly amplitude?: number;
  /** Start timestamp offset in seconds (default 0.0). */
  readonly startTimeSeconds?: number;
  /** Variation profile overriding the instrument's variation. */
  readonly variationProfile?: VariationProfile;
  /** Ceiling on total rendered samples including release tail (CWE-400 guard). */
  readonly maxSampleCount?: number;
  /** Ceiling on number of partials permitted (CWE-400 guard). */
  readonly maxPartialCount?: number;
}
