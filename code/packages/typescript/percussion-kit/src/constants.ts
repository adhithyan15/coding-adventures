/**
 * Constants and VER15 security bounds for unpitched percussion and drum kit events (MUS07).
 */

export const MAX_PERCUSSION_MODES = 32;
export const MAX_HIT_DURATION_SECONDS = 30.0;
export const MAX_DURATION_SECONDS = 60.0;
export const MAX_SAMPLE_RATE_HZ = 192_000.0;
export const MAX_SAMPLE_COUNT = 16_777_216; // 2^24 samples (~128 MiB for 64-bit floats)
export const MAX_HITS = 65_536;
export const MAX_SEQUENCE_EVENTS = 16_384;

export const DEFAULT_SAMPLE_RATE_HZ = 44_100.0;
export const DEFAULT_MAX_SAMPLE_COUNT = 10_000_000;
export const DEFAULT_MAX_MODE_COUNT = 32;

export const CHOKE_RELEASE_RAMP_SECONDS = 0.015; // 15 ms linear attenuation ramp
export const INTEGER_TOLERANCE = 1e-9;

export const ALL_STANDARD_HIT_IDS: readonly string[] = Object.freeze([
  "kick",
  "snare",
  "closed_hihat",
  "open_hihat",
  "pedal_hihat",
  "low_tom",
  "mid_tom",
  "high_tom",
  "crash",
  "ride",
  "clap",
  "cowbell",
  "tambourine",
  "shaker",
  "woodblock",
]);
