/**
 * Hi-hat choke group dynamics and linear release ramp attenuation (MUS07 §7).
 */

import { CHOKE_RELEASE_RAMP_SECONDS, MAX_SAMPLE_COUNT } from "./constants.js";
import type { PercussionVoiceProfile } from "./profile.js";
import {
  safeRoundSample,
  validateDuration,
  validateNonNegative,
  validateSampleRate,
} from "./validation.js";

/**
 * Apply a linear release ramp attenuation to a hi-hat sample buffer (MUS07 §7).
 */
export function applyHihatChoke(
  samples: Float64Array | readonly number[],
  hitStartSeconds: number,
  chokeStartSeconds: number,
  sampleRateHz: number,
  chokeRampSeconds = CHOKE_RELEASE_RAMP_SECONDS,
): Float64Array {
  const totalLen = samples.length;
  if (totalLen > MAX_SAMPLE_COUNT) {
    throw new RangeError(
      `Sample buffer length ${String(totalLen)} exceeds MAX_SAMPLE_COUNT=${String(MAX_SAMPLE_COUNT)}`,
    );
  }

  const hitStart = validateNonNegative("hitStartSeconds", hitStartSeconds);
  const chokeStart = validateNonNegative("chokeStartSeconds", chokeStartSeconds);
  const rate = validateSampleRate("sampleRateHz", sampleRateHz);
  const rampDur = validateDuration("chokeRampSeconds", chokeRampSeconds);

  if (totalLen === 0) {
    return new Float64Array(0);
  }

  const outputArray = new Float64Array(samples);

  if (chokeStart <= hitStart) {
    const rampSamples = safeRoundSample(rampDur, rate);
    if (rampSamples === 0) {
      outputArray.fill(0.0);
      return outputArray;
    }
    const limit = Math.min(totalLen, rampSamples);
    for (let i = 0; i < limit; i += 1) {
      outputArray[i] *= Math.max(0.0, 1.0 - i / rampSamples);
    }
    for (let i = rampSamples; i < totalLen; i += 1) {
      outputArray[i] = 0.0;
    }
    return outputArray;
  }

  const timeDelta = chokeStart - hitStart;
  const chokeIdx = safeRoundSample(timeDelta, rate);
  if (chokeIdx >= totalLen) {
    return outputArray;
  }

  const rampSamples = safeRoundSample(rampDur, rate);
  if (rampSamples === 0) {
    for (let i = chokeIdx; i < totalLen; i += 1) {
      outputArray[i] = 0.0;
    }
    return outputArray;
  }

  const rampEnd = Math.min(totalLen, chokeIdx + rampSamples);
  for (let i = chokeIdx; i < rampEnd; i += 1) {
    const fraction = (i - chokeIdx) / rampSamples;
    outputArray[i] *= Math.max(0.0, Math.min(1.0, 1.0 - fraction));
  }

  for (let i = rampEnd; i < totalLen; i += 1) {
    outputArray[i] = 0.0;
  }

  return outputArray;
}

/**
 * Determine whether a later percussion hit chokes an active earlier hit (MUS07 §7).
 */
export function isChokedBy(
  earlierVoice: PercussionVoiceProfile,
  earlierHitId: string,
  laterVoice: PercussionVoiceProfile,
  laterHitId: string,
): boolean {
  if (!earlierVoice.chokeGroup || !laterVoice.chokeGroup) {
    return false;
  }
  if (earlierVoice.chokeGroup !== laterVoice.chokeGroup) {
    return false;
  }

  if (earlierVoice.chokeGroup === "hihat") {
    const isEarlierOpen =
      earlierHitId === "open_hihat" ||
      earlierVoice.id === "open_hihat_naive" ||
      earlierVoice.id.toLowerCase().includes("open");

    const isLaterChoker =
      laterHitId === "closed_hihat" ||
      laterHitId === "pedal_hihat" ||
      laterHitId === "open_hihat" ||
      laterVoice.id === "closed_hihat_naive" ||
      laterVoice.id === "pedal_hihat_naive" ||
      laterVoice.id === "open_hihat_naive" ||
      laterVoice.id.toLowerCase().includes("closed") ||
      laterVoice.id.toLowerCase().includes("pedal") ||
      laterVoice.id.toLowerCase().includes("open");

    return isEarlierOpen && isLaterChoker;
  }

  return true;
}
