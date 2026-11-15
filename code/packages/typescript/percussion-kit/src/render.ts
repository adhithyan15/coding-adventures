/**
 * Single hit and sequence rendering pipelines with hi-hat choke dynamics (MUS07 §7, §9).
 * Strictly iterative, streaming timeline mixing with pre-flight CWE-400 resource checks.
 */

import { SampleBuffer, UniformSampler } from "@coding-adventures/oscillator";
import { encodeSampleBuffer, type PCMBuffer } from "@coding-adventures/pcm-audio";

import { applyHihatChoke, isChokedBy } from "./choke.js";
import {
  CHOKE_RELEASE_RAMP_SECONDS,
  DEFAULT_MAX_MODE_COUNT,
  DEFAULT_MAX_SAMPLE_COUNT,
  DEFAULT_SAMPLE_RATE_HZ,
  MAX_HITS,
  MAX_PERCUSSION_MODES,
  MAX_SAMPLE_COUNT,
  MAX_SEQUENCE_EVENTS,
} from "./constants.js";
import { DrumKitProfile, getPercussionVoice, standardKit } from "./kit.js";
import { PercussionVoiceProfile } from "./profile.js";
import { PercussionSignal } from "./signal.js";
import type { HitRenderOptions, SequenceRenderOptions } from "./types.js";
import {
  safeRoundSample,
  validateDuration,
  validateModeCountCeiling,
  validateNonEmptyString,
  validateNonNegative,
  validateSampleCountCeiling,
  validateSampleRate,
  validateSequenceEventCountCeiling,
  validateUnitFloat,
} from "./validation.js";

export class PercussionHitRender {
  readonly hitId: string;
  readonly voice: PercussionVoiceProfile;
  readonly signal: PercussionSignal;
  readonly floatingSamples: SampleBuffer;
  readonly pcmBuffer: PCMBuffer;

  constructor(
    hitId: string,
    voice: PercussionVoiceProfile,
    signal: PercussionSignal,
    floatingSamples: SampleBuffer,
    pcmBuffer: PCMBuffer,
  ) {
    this.hitId = hitId;
    this.voice = voice;
    this.signal = signal;
    this.floatingSamples = floatingSamples;
    this.pcmBuffer = pcmBuffer;
  }
}

export class PercussionHitEvent {
  readonly hitId: string;
  readonly startSeconds: number;
  readonly durationSeconds: number;
  readonly velocity: number;
  readonly trackId: string;
  readonly variation: string | null;

  constructor(
    hitId: string,
    startSeconds = 0.0,
    durationSeconds = 0.05,
    velocity = 1.0,
    trackId = "drums",
    variation: string | null = null,
  ) {
    this.hitId = validateNonEmptyString("hitId", hitId);
    this.startSeconds = validateNonNegative("startSeconds", startSeconds);
    this.durationSeconds = validateDuration("durationSeconds", durationSeconds);
    this.velocity = validateUnitFloat("velocity", velocity);
    this.trackId = validateNonEmptyString("trackId", trackId);
    this.variation = typeof variation === "string" ? variation : null;
  }

  static hit(hitId: string, startSeconds: number): PercussionHitEvent {
    return new PercussionHitEvent(hitId, startSeconds, 0.05, 1.0, "drums", null);
  }

  static withDuration(
    hitId: string,
    startSeconds: number,
    durationSeconds: number,
  ): PercussionHitEvent {
    return new PercussionHitEvent(
      hitId,
      startSeconds,
      durationSeconds,
      1.0,
      "drums",
      null,
    );
  }

  get startTimeSeconds(): number {
    return this.startSeconds;
  }
}

export class PercussionSequenceRender {
  readonly events: readonly PercussionHitEvent[];
  readonly floatingSamples: SampleBuffer;
  readonly pcmBuffer: PCMBuffer;

  constructor(
    events: readonly PercussionHitEvent[],
    floatingSamples: SampleBuffer,
    pcmBuffer: PCMBuffer,
  ) {
    this.events = Object.freeze([...events]);
    this.floatingSamples = floatingSamples;
    this.pcmBuffer = pcmBuffer;
  }

  get sampleRateHz(): number {
    return this.floatingSamples.sampleRateHz;
  }

  get durationSeconds(): number {
    return this.pcmBuffer.durationSeconds();
  }

  sampleCount(): number {
    return this.floatingSamples.sampleCount();
  }
}

export function renderPercussionHit(
  hitId: string,
  durationSeconds: number,
  options: HitRenderOptions = {},
): PercussionHitRender {
  const sampleRate = validateSampleRate(
    "sampleRateHz",
    options.sampleRateHz ?? DEFAULT_SAMPLE_RATE_HZ,
  );
  const dur = validateDuration("durationSeconds", durationSeconds);
  const amplitude = validateUnitFloat("amplitude", options.amplitude ?? 1.0);

  const maxModes = validateModeCountCeiling(
    "maxModeCount",
    options.maxModeCount ?? DEFAULT_MAX_MODE_COUNT,
    MAX_PERCUSSION_MODES,
  );

  const maxSamples = validateSampleCountCeiling(
    "maxSampleCount",
    options.maxSampleCount ?? DEFAULT_MAX_SAMPLE_COUNT,
  );

  const activeKit = options.kit ?? standardKit();
  const voice = getPercussionVoice(hitId, activeKit);

  if (voice.modes.length > maxModes) {
    throw new RangeError(
      `Voice has ${String(voice.modes.length)} modes, exceeding maxModeCount=${String(maxModes)}`,
    );
  }

  const renderedDur = voice.envelope.renderedDurationSeconds(dur);
  const neededCount = Math.round(renderedDur * sampleRate);
  if (neededCount > maxSamples) {
    throw new RangeError(
      `Hit requires ${String(neededCount)} samples, exceeding maxSampleCount=${String(maxSamples)}`,
    );
  }

  const signal = new PercussionSignal(voice, dur, amplitude, sampleRate);
  const sampler = new UniformSampler(sampleRate);
  const samples = sampler.sample(signal, renderedDur);
  const pcmBuffer = encodeSampleBuffer(samples);

  return new PercussionHitRender(
    hitId,
    voice,
    signal,
    samples,
    pcmBuffer,
  );
}

export function renderPercussionHitPcm(
  hitId: string,
  durationSeconds: number,
  options: HitRenderOptions = {},
): PCMBuffer {
  const rendered = renderPercussionHit(hitId, durationSeconds, options);
  return rendered.pcmBuffer;
}

export function renderPercussionSequence(
  events: readonly PercussionHitEvent[],
  options: SequenceRenderOptions = {},
): PercussionSequenceRender {
  const sampleRate = validateSampleRate(
    "sampleRateHz",
    options.sampleRateHz ?? DEFAULT_SAMPLE_RATE_HZ,
  );
  const rampDur = validateDuration(
    "chokeRampSeconds",
    options.chokeRampSeconds ?? CHOKE_RELEASE_RAMP_SECONDS,
  );

  const maxEvents = validateSequenceEventCountCeiling(
    "maxSequenceEvents",
    options.maxSequenceEvents ?? MAX_SEQUENCE_EVENTS,
    MAX_SEQUENCE_EVENTS,
  );

  const maxSamples = validateSampleCountCeiling(
    "maxSampleCount",
    options.maxSampleCount ?? MAX_SAMPLE_COUNT,
  );

  const maxModes = validateModeCountCeiling(
    "maxModeCount",
    options.maxModeCount ?? MAX_PERCUSSION_MODES,
    MAX_PERCUSSION_MODES,
  );

  if (events.length > maxEvents) {
    throw new RangeError(
      `Sequence contains ${String(events.length)} events, exceeding maxSequenceEvents=${String(maxEvents)}`,
    );
  }

  if (events.length === 0) {
    const emptyBuf = new SampleBuffer(new Float64Array(0), sampleRate, 0.0);
    const emptyPcm = encodeSampleBuffer(emptyBuf);
    return new PercussionSequenceRender([], emptyBuf, emptyPcm);
  }

  const activeKit = options.kit ?? standardKit();
  const numEvents = events.length;

  const eventVoices: PercussionVoiceProfile[] = [];
  const eventStartIndices: number[] = [];
  const eventRenderedDurs: number[] = [];
  let maxEndSample = 0;

  for (let i = 0; i < numEvents; i += 1) {
    const ev = events[i];
    if (!(ev instanceof PercussionHitEvent)) {
      throw new TypeError(`events[${String(i)}] must be a PercussionHitEvent`);
    }

    const voice = getPercussionVoice(ev.hitId, activeKit);
    if (voice.modes.length > maxModes) {
      throw new RangeError(
        `Voice "${voice.id}" has ${String(voice.modes.length)} modes, exceeding maxModeCount=${String(maxModes)}`,
      );
    }
    eventVoices.push(voice);

    const renderedDur = voice.envelope.renderedDurationSeconds(ev.durationSeconds);
    eventRenderedDurs.push(renderedDur);

    const hitSamples = Math.round(renderedDur * sampleRate);
    if (hitSamples > maxSamples) {
      throw new RangeError(
        `Event ${String(i)} requires ${String(hitSamples)} samples, exceeding maxSampleCount=${String(maxSamples)}`,
      );
    }

    const startIdx = safeRoundSample(ev.startSeconds, sampleRate);
    eventStartIndices.push(startIdx);

    const hitEnd = startIdx + hitSamples;
    if (hitEnd > maxSamples) {
      throw new RangeError(
        `Sequence requires ${String(hitEnd)} samples, exceeding maxSampleCount=${String(maxSamples)}`,
      );
    }
    if (hitEnd > maxEndSample) {
      maxEndSample = hitEnd;
    }
  }

  // Determine earliest choke times for hi-hats
  const earliestChokeTimesArray: (number | null)[] = new Array(numEvents).fill(null);
  for (let i = 0; i < numEvents; i += 1) {
    const vI = eventVoices[i];
    if (!vI.chokeGroup) {
      continue;
    }
    const evI = events[i];
    let chokeT: number | null = null;
    for (let j = 0; j < numEvents; j += 1) {
      if (i === j) {
        continue;
      }
      const evJ = events[j];
      if (evJ.startSeconds < evI.startSeconds) {
        continue;
      }
      if (
        isChokedBy(vI, evI.hitId, eventVoices[j], evJ.hitId) &&
        (chokeT === null || evJ.startSeconds < chokeT)
      ) {
        chokeT = evJ.startSeconds;
      }
    }
    earliestChokeTimesArray[i] = chokeT;
  }

  // Allocate timeline buffer
  const timelineArray = new Float64Array(maxEndSample);

  // Streaming synthesis: render and mix one hit at a time
  const sampler = new UniformSampler(sampleRate);
  for (let i = 0; i < numEvents; i += 1) {
    const ev = events[i];
    const voice = eventVoices[i];
    const renderedDur = eventRenderedDurs[i];
    const st = eventStartIndices[i];

    const sig = new PercussionSignal(voice, ev.durationSeconds, ev.velocity, sampleRate);
    const sbuf = sampler.sample(sig, renderedDur);
    let hitSamples = sbuf.samples;

    const chokeT = earliestChokeTimesArray[i];
    if (chokeT !== null) {
      hitSamples = applyHihatChoke(hitSamples, ev.startSeconds, chokeT, sampleRate, rampDur);
    }

    const hitLen = hitSamples.length;
    for (let k = 0; k < hitLen; k += 1) {
      timelineArray[st + k] += hitSamples[k];
    }
  }

  // Clamp mixed samples to [-1.0, 1.0]
  for (let i = 0; i < timelineArray.length; i += 1) {
    const s = timelineArray[i];
    if (s > 1.0) {
      timelineArray[i] = 1.0;
    } else if (s < -1.0) {
      timelineArray[i] = -1.0;
    }
  }

  const sampleBuf = new SampleBuffer(timelineArray, sampleRate, 0.0);
  const pcmBuf = encodeSampleBuffer(sampleBuf);

  return new PercussionSequenceRender(events, sampleBuf, pcmBuf);
}

export function renderPercussionSequencePcm(
  events: readonly PercussionHitEvent[],
  options: SequenceRenderOptions = {},
): PCMBuffer {
  const rendered = renderPercussionSequence(events, options);
  return rendered.pcmBuffer;
}
