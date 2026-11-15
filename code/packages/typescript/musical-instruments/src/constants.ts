/**
 * Mathematical constants and VER15 security bounds for musical instruments (MUS05).
 */

export const TWO_PI = 2.0 * Math.PI;

export const MAX_PARTIALS = 64;
export const MAX_NOTE_DURATION_SECONDS = 60.0;
export const MAX_SAMPLE_RATE_HZ = 192_000.0;
export const MAX_SAMPLE_COUNT = 16_777_216; // 2^24 samples (~128 MiB for 64-bit floats)

export const DEFAULT_MAX_PARTIAL_COUNT = MAX_PARTIALS;
export const DEFAULT_MAX_SAMPLE_COUNT = 10_000_000;
export const DEFAULT_SAMPLE_RATE_HZ = 44_100.0;

export const MIN_OCTAVE = -10;
export const MAX_OCTAVE = 20;

export const SYNTHESIS_KINDS = ["sine", "additive", "silence"] as const;
