/**
 * Warrant operation models (matching music-machine-web.warrant specifications).
 * Discharges CWE-20, CWE-190, CWE-208, and CWE-400.
 */

import {
  MAX_EVENTS,
  MAX_SAMPLE_RATE_HZ,
  MAX_SCORE_BYTES,
} from "./constants.js";
import { validateNonNegativeInt, validateSafeInteger } from "./validation.js";

/**
 * Warrant operation: verify score byte length budget.
 */
export function warrantParseScore(scoreBytes: number): number {
  validateNonNegativeInt("score_bytes", scoreBytes);
  if (scoreBytes > MAX_SCORE_BYTES) {
    throw new RangeError("score_bytes > MAX_SCORE_BYTES");
  }
  return scoreBytes;
}

/**
 * Warrant operation: verify portable score byte length budget.
 */
export function warrantParsePortableScore(scoreBytes: number): number {
  validateNonNegativeInt("score_bytes", scoreBytes);
  if (scoreBytes > MAX_SCORE_BYTES) {
    throw new RangeError("score_bytes > MAX_SCORE_BYTES");
  }
  return scoreBytes;
}

/**
 * Warrant operation: verify render sample rate and event count budget.
 */
export function warrantRenderScoreToPcm(
  eventCount: number,
  sampleRateHz: number,
): number {
  validateSafeInteger("sample_rate_hz", sampleRateHz);
  if (sampleRateHz <= 0 || sampleRateHz > MAX_SAMPLE_RATE_HZ) {
    throw new RangeError("invalid sample_rate_hz");
  }
  validateNonNegativeInt("event_count", eventCount);
  if (eventCount > MAX_EVENTS) {
    throw new RangeError("event_count > MAX_EVENTS");
  }
  return eventCount;
}

/**
 * Warrant operation: verify portable render sample rate and event count budget.
 */
export function warrantRenderPortableScoreToPcm(
  eventCount: number,
  sampleRateHz: number,
): number {
  validateSafeInteger("sample_rate_hz", sampleRateHz);
  if (sampleRateHz <= 0 || sampleRateHz > MAX_SAMPLE_RATE_HZ) {
    throw new RangeError("invalid sample_rate_hz");
  }
  validateNonNegativeInt("event_count", eventCount);
  if (eventCount > MAX_EVENTS) {
    throw new RangeError("event_count > MAX_EVENTS");
  }
  return eventCount;
}
