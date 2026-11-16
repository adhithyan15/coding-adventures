/**
 * Operations defined in percussion-kit.warrant specification.
 */

import { MAX_PERCUSSION_MODES, MAX_SAMPLE_COUNT, MAX_SAMPLE_RATE_HZ } from "./constants.js";

export function calcHitSampleCount(durMs: number, rate: number): number | null {
  if (
    typeof durMs !== "number" ||
    !Number.isSafeInteger(durMs) ||
    durMs < 0 ||
    typeof rate !== "number" ||
    !Number.isSafeInteger(rate) ||
    rate <= 0 ||
    rate > MAX_SAMPLE_RATE_HZ
  ) {
    return null;
  }
  const count = Math.floor((durMs * rate) / 1000);
  if (!Number.isSafeInteger(count) || count > MAX_SAMPLE_COUNT) {
    return null;
  }
  return count;
}

export function isModeCountSafe(modes: number): boolean {
  return (
    typeof modes === "number" &&
    Number.isSafeInteger(modes) &&
    modes >= 0 &&
    modes <= MAX_PERCUSSION_MODES
  );
}
