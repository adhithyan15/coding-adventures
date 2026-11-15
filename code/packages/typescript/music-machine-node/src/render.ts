/**
 * Multi-track polyphonic PCM rendering, hi-hat choke mixing, and WAV generation (MUS01 / MUS03 / MUS04 / MUS07 / VER15).
 * Hardened for CWE-20, CWE-190, CWE-400, and CWE-674.
 */

import {
  InstrumentNoteRender,
  renderInstrumentNote,
} from "@coding-adventures/musical-instruments";
import { sampleCountForDuration } from "@coding-adventures/oscillator";
import {
  PCMBuffer,
  PCMFormat,
  toWavBytes,
} from "@coding-adventures/pcm-audio";
import {
  getDrumKit,
  PercussionHitEvent,
  PercussionHitRender,
  renderPercussionSequence,
} from "@coding-adventures/percussion-kit";

import {
  DEFAULT_MAX_SAMPLE_COUNT,
  MAX_DURATION_SECONDS,
  MAX_SAMPLE_COUNT,
} from "./constants.js";
import { TempoMap } from "./tempo.js";
import {
  InstrumentDeclaration,
  PortableScore,
  RenderedPortableScore,
  RenderedScore,
  ScoreEventKind,
  TextScore,
  TrackDeclaration,
} from "./types.js";
import {
  safeRoundSample,
  validateNonNegativeInt,
  validatePositiveInt,
  validateSafeInteger,
} from "./validation.js";

/**
 * Mix a source PCM sample slice into a target buffer at an offset using saturating 16-bit arithmetic.
 * Samples outside [-32768, 32767] saturate to boundary values and increment the clipped count.
 */
export function mixPcmSamples(
  target: number[],
  startSample: number,
  source: readonly number[],
): number {
  validateNonNegativeInt("startSample", startSample);
  const requiredCount = startSample + source.length;
  if (!Number.isSafeInteger(requiredCount)) {
    throw new RangeError("requiredCount exceeds safe integer");
  }

  while (target.length < requiredCount) {
    target.push(0);
  }

  let clippedCount = 0;
  for (let offset = 0; offset < source.length; offset += 1) {
    const srcVal = source[offset];
    if (typeof srcVal !== "number" || !Number.isFinite(srcVal)) {
      throw new RangeError(
        `source sample at offset ${String(offset)} must be finite, got ${String(srcVal)}`,
      );
    }
    const targetIndex = startSample + offset;
    const mixed = target[targetIndex] + srcVal;
    if (!Number.isFinite(mixed)) {
      throw new RangeError(
        `mixed sample at offset ${String(offset)} must be finite, got ${String(mixed)}`,
      );
    }
    if (mixed > 32_767) {
      target[targetIndex] = 32_767;
      clippedCount += 1;
    } else if (mixed < -32_768) {
      target[targetIndex] = -32_768;
      clippedCount += 1;
    } else {
      target[targetIndex] = mixed;
    }
  }
  return clippedCount;
}

/**
 * Render a parsed beginner TextScore into a RenderedScore.
 */
export function renderScoreToPcm(
  score: TextScore,
  maxSampleCount: number = DEFAULT_MAX_SAMPLE_COUNT,
): RenderedScore {
  if (!(score instanceof TextScore)) {
    throw new TypeError("score must be a TextScore instance");
  }
  const sampleLimit =
    maxSampleCount === 0 ? DEFAULT_MAX_SAMPLE_COUNT : maxSampleCount;
  validatePositiveInt("maxSampleCount", sampleLimit);
  if (sampleLimit > MAX_SAMPLE_COUNT) {
    throw new RangeError(
      `maxSampleCount ${String(sampleLimit)} exceeds MAX_SAMPLE_COUNT=${String(MAX_SAMPLE_COUNT)}`,
    );
  }

  const totalDur = score.totalDurationSeconds();
  if (totalDur > MAX_DURATION_SECONDS) {
    throw new RangeError(
      `total score duration ${String(totalDur)}s exceeds MAX_DURATION_SECONDS=${String(MAX_DURATION_SECONDS)}`,
    );
  }

  const rate = score.sampleRateHz;
  const preSamples = Math.round(totalDur * rate);
  if (preSamples > sampleLimit) {
    throw new RangeError(
      `score render would require ${String(preSamples)} samples, exceeding maxSampleCount=${String(sampleLimit)}`,
    );
  }

  const pcmSamples: number[] = [];
  const renderedNotes: InstrumentNoteRender[] = [];
  const renderedHits: (PercussionHitRender | PercussionHitEvent)[] = [];
  let clippedSampleCount = 0;

  // Check if score is entirely drum hits and rests to apply hi-hat choke sequence
  const hasHits = score.events.some((e) => e.kind === "hit");
  const allHitsOrRests = score.events.every(
    (e) => e.kind === "hit" || e.kind === "rest",
  );

  if (hasHits && allHitsOrRests) {
    const kitEvents: PercussionHitEvent[] = [];
    let curT = 0.0;
    for (const ev of score.events) {
      if (ev.kind === "hit" && ev.hitId) {
        kitEvents.push(
          new PercussionHitEvent(
            ev.hitId,
            curT,
            ev.durationSeconds,
            score.amplitude,
            "drums",
            null,
          ),
        );
      }
      curT += ev.durationSeconds;
    }

    const kitName = score.drumkit ?? "standard_kit";
    const kitProf = getDrumKit(kitName);
    const seqRender = renderPercussionSequence(kitEvents, {
      kit: kitProf,
      sampleRateHz: rate,
      maxSampleCount: sampleLimit,
    });

    const totalSamples = [...seqRender.pcmBuffer.samples];
    const minRequired = sampleCountForDuration(totalDur, rate);
    while (totalSamples.length < minRequired) {
      totalSamples.push(0);
    }

    clippedSampleCount += seqRender.pcmBuffer.clippedSampleCount;
    const format = new PCMFormat(rate, 1, 16);
    const pcmBuffer = new PCMBuffer(
      totalSamples,
      format,
      0.0,
      clippedSampleCount,
    );

    for (let i = 0; i < seqRender.events.length; i += 1) {
      renderedHits.push(seqRender.events[i]);
    }

    return new RenderedScore(
      score,
      pcmBuffer,
      [],
      renderedHits,
      clippedSampleCount,
    );
  }

  let cursorSampleIndex = 0;
  let elapsedSeconds = 0.0;

  for (const ev of score.events) {
    switch (ev.kind) {
      case "rest": {
        const silenceCount = sampleCountForDuration(ev.durationSeconds, rate);
        cursorSampleIndex += silenceCount;
        if (cursorSampleIndex > sampleLimit) {
          throw new RangeError(
            `cursorSampleIndex ${String(cursorSampleIndex)} exceeds maxSampleCount=${String(sampleLimit)}`,
          );
        }
        while (pcmSamples.length < cursorSampleIndex) {
          pcmSamples.push(0);
        }
        break;
      }
      case "hit": {
        const hitId = ev.hitId ?? "kick";
        const kitEvent = new PercussionHitEvent(
          hitId,
          0.0,
          ev.durationSeconds,
          score.amplitude,
          "drums",
          null,
        );
        const kitName = score.drumkit ?? "standard_kit";
        const kitProf = getDrumKit(kitName);
        const seqRender = renderPercussionSequence([kitEvent], {
          kit: kitProf,
          sampleRateHz: rate,
          maxSampleCount: sampleLimit,
        });

        clippedSampleCount += seqRender.pcmBuffer.clippedSampleCount;
        clippedSampleCount += mixPcmSamples(
          pcmSamples,
          cursorSampleIndex,
          seqRender.pcmBuffer.samples,
        );
        const hitSamples = sampleCountForDuration(ev.durationSeconds, rate);
        cursorSampleIndex += hitSamples;
        if (cursorSampleIndex > sampleLimit) {
          throw new RangeError(
            `cursorSampleIndex ${String(cursorSampleIndex)} exceeds maxSampleCount=${String(sampleLimit)}`,
          );
        }
        for (let i = 0; i < seqRender.events.length; i += 1) {
          renderedHits.push(seqRender.events[i]);
        }
        break;
      }
      case "note": {
        const noteStr = ev.note ?? "A4";
        const noteRender = renderInstrumentNote(noteStr, ev.durationSeconds, {
          sampleRateHz: rate,
          amplitude: score.amplitude,
          instrumentId: score.instrumentId,
          startTimeSeconds: elapsedSeconds,
          maxSampleCount: sampleLimit,
        });

        clippedSampleCount += noteRender.pcmBuffer.clippedSampleCount;
        clippedSampleCount += mixPcmSamples(
          pcmSamples,
          cursorSampleIndex,
          noteRender.pcmBuffer.samples,
        );
        const noteSamples = sampleCountForDuration(ev.durationSeconds, rate);
        cursorSampleIndex += noteSamples;
        if (cursorSampleIndex > sampleLimit) {
          throw new RangeError(
            `cursorSampleIndex ${String(cursorSampleIndex)} exceeds maxSampleCount=${String(sampleLimit)}`,
          );
        }
        renderedNotes.push(noteRender);
        break;
      }
    }
    elapsedSeconds += ev.durationSeconds;
  }

  const format = new PCMFormat(rate, 1, 16);
  const pcmBuffer = new PCMBuffer(
    pcmSamples,
    format,
    0.0,
    clippedSampleCount,
  );

  return new RenderedScore(
    score,
    pcmBuffer,
    renderedNotes,
    renderedHits,
    clippedSampleCount,
  );
}

/**
 * Render a parsed portable multi-track PortableScore into a RenderedPortableScore.
 */
export function renderPortableScoreToPcm(
  score: PortableScore,
  maxSampleCount: number = DEFAULT_MAX_SAMPLE_COUNT,
): RenderedPortableScore {
  if (!(score instanceof PortableScore)) {
    throw new TypeError("score must be a PortableScore instance");
  }
  const sampleLimit =
    maxSampleCount === 0 ? DEFAULT_MAX_SAMPLE_COUNT : maxSampleCount;
  validatePositiveInt("maxSampleCount", sampleLimit);
  if (sampleLimit > MAX_SAMPLE_COUNT) {
    throw new RangeError(
      `maxSampleCount ${String(sampleLimit)} exceeds MAX_SAMPLE_COUNT=${String(MAX_SAMPLE_COUNT)}`,
    );
  }

  const instrumentById = new Map<string, InstrumentDeclaration>();
  for (const inst of score.instruments) {
    instrumentById.set(inst.id, inst);
  }
  const trackById = new Map<string, TrackDeclaration>();
  for (const tr of score.tracks) {
    trackById.set(tr.id, tr);
  }

  const tempoMap = new TempoMap(score.tempoEvents, score.ppq);
  const rate = score.sampleRateHz;

  let lastTick = 0;
  for (const e of score.events) {
    const endTick = e.startTick + e.durationTick;
    if (endTick > lastTick) {
      lastTick = endTick;
    }
  }

  const totalDur = tempoMap.secondsAtTick(lastTick);
  if (totalDur > MAX_DURATION_SECONDS) {
    throw new RangeError(
      `score duration ${String(totalDur)}s exceeds MAX_DURATION_SECONDS=${String(MAX_DURATION_SECONDS)}`,
    );
  }

  const preSamples = Math.round(totalDur * rate);
  if (preSamples > sampleLimit) {
    throw new RangeError(
      `score render would require ${String(preSamples)} samples, exceeding maxSampleCount=${String(sampleLimit)}`,
    );
  }

  // Validate track/instrument compatibility
  for (const ev of score.events) {
    const tr = trackById.get(ev.trackId);
    if (!tr) {
      throw new RangeError(`event references unknown track '${ev.trackId}'`);
    }
    const inst = instrumentById.get(tr.instrumentId);
    if (!inst) {
      throw new RangeError(
        `track '${tr.id}' references unknown instrument '${tr.instrumentId}'`,
      );
    }
    if (ev.kind === "note" && inst.kind === "drumkit") {
      throw new RangeError(
        `track '${tr.id}' uses drumkit instrument '${inst.id}' and cannot play note events`,
      );
    }
    if (ev.kind === "hit" && inst.kind !== "drumkit") {
      throw new RangeError(
        `track '${tr.id}' uses pitched instrument '${inst.id}' and cannot play hit events`,
      );
    }
  }

  const pcmSamples: number[] = [];
  const renderedNotes: InstrumentNoteRender[] = [];
  const renderedHits: (PercussionHitRender | PercussionHitEvent)[] = [];
  let clippedSampleCount = 0;

  // 1. Render Pitched Notes
  for (const ev of score.events) {
    if (ev.kind !== "note") {
      continue;
    }
    const endTick = ev.startTick + ev.durationTick;
    const startSec = tempoMap.secondsAtTick(ev.startTick);
    const endSec = tempoMap.secondsAtTick(endTick);
    const durSec = endSec - startSec;
    const startSample = safeRoundSample(startSec, rate);

    const tr = trackById.get(ev.trackId)!;
    const inst = instrumentById.get(tr.instrumentId)!;
    const divisor = ev.notes.length > 0 ? ev.notes.length : 1.0;
    const noteAmplitude = (ev.velocity * inst.gain) / divisor;

    for (const note of ev.notes) {
      const renderedNote = renderInstrumentNote(note, durSec, {
        sampleRateHz: rate,
        amplitude: noteAmplitude,
        startTimeSeconds: 0.0,
        instrument: inst.customProfile ?? undefined,
        instrumentId: inst.customProfile ? undefined : inst.profileId,
        maxSampleCount: sampleLimit,
      });

      clippedSampleCount += renderedNote.pcmBuffer.clippedSampleCount;
      const requiredCount = startSample + renderedNote.pcmBuffer.sampleCount();
      if (requiredCount > sampleLimit) {
        throw new RangeError(
          `render would create ${String(requiredCount)} samples, exceeding maxSampleCount=${String(sampleLimit)}`,
        );
      }

      clippedSampleCount += mixPcmSamples(
        pcmSamples,
        startSample,
        renderedNote.pcmBuffer.samples,
      );
      if (pcmSamples.length > sampleLimit) {
        throw new RangeError(
          `render would create ${String(pcmSamples.length)} samples, exceeding maxSampleCount=${String(sampleLimit)}`,
        );
      }
      renderedNotes.push(renderedNote);
    }
  }

  // 2. Render Drum Hits with Hi-Hat Choking (MUS07 §7 & §9)
  const drumKitsPresent: string[] = [];
  for (const inst of score.instruments) {
    if (inst.kind === "drumkit") {
      const kitName = inst.kit ?? "standard_kit";
      if (!drumKitsPresent.includes(kitName)) {
        drumKitsPresent.push(kitName);
      }
    }
  }

  for (const kitName of drumKitsPresent) {
    const kitEvents: PercussionHitEvent[] = [];
    for (const ev of score.events) {
      if (ev.kind !== "hit") {
        continue;
      }
      const tr = trackById.get(ev.trackId)!;
      const inst = instrumentById.get(tr.instrumentId)!;
      const instKit = inst.kit ?? "standard_kit";
      if (instKit !== kitName) {
        continue;
      }

      const endTick = ev.startTick + ev.durationTick;
      const startSec = tempoMap.secondsAtTick(ev.startTick);
      const endSec = tempoMap.secondsAtTick(endTick);
      const durSec = endSec - startSec;
      const hitId = ev.hitId ?? "kick";

      kitEvents.push(
        new PercussionHitEvent(
          hitId,
          startSec,
          durSec,
          ev.velocity * inst.gain,
          ev.trackId,
          null,
        ),
      );
    }

    if (kitEvents.length > 0) {
      const kitProf = getDrumKit(kitName);
      const seqRender = renderPercussionSequence(kitEvents, {
        kit: kitProf,
        sampleRateHz: rate,
        maxSampleCount: sampleLimit,
      });

      clippedSampleCount += seqRender.pcmBuffer.clippedSampleCount;
      clippedSampleCount += mixPcmSamples(
        pcmSamples,
        0,
        seqRender.pcmBuffer.samples,
      );
      for (let i = 0; i < seqRender.events.length; i += 1) {
        renderedHits.push(seqRender.events[i]);
      }
    }
  }

  // 3. Handle Rest Bounds
  for (const ev of score.events) {
    if (ev.kind === "rest") {
      const endTick = ev.startTick + ev.durationTick;
      const endSec = tempoMap.secondsAtTick(endTick);
      const restEnd = safeRoundSample(endSec, rate);
      if (restEnd > sampleLimit) {
        throw new RangeError(
          `render would create ${String(restEnd)} samples, exceeding maxSampleCount=${String(sampleLimit)}`,
        );
      }
      while (pcmSamples.length < restEnd) {
        pcmSamples.push(0);
      }
    }
  }

  if (pcmSamples.length > sampleLimit) {
    throw new RangeError(
      `render would create ${String(pcmSamples.length)} samples, exceeding maxSampleCount=${String(sampleLimit)}`,
    );
  }

  const format = new PCMFormat(rate, 1, 16);
  const pcmBuffer = new PCMBuffer(
    pcmSamples,
    format,
    0.0,
    clippedSampleCount,
  );

  return new RenderedPortableScore(
    score,
    pcmBuffer,
    renderedNotes,
    renderedHits,
    clippedSampleCount,
  );
}

/**
 * Render a beginner TextScore and return standard RIFF/WAVE file bytes.
 */
export function renderScoreToWav(
  score: TextScore,
  maxSampleCount: number = DEFAULT_MAX_SAMPLE_COUNT,
): Uint8Array {
  const rendered = renderScoreToPcm(score, maxSampleCount);
  return toWavBytes(rendered.pcmBuffer);
}

/**
 * Render a portable multi-track PortableScore and return standard RIFF/WAVE file bytes.
 */
export function renderPortableScoreToWav(
  score: PortableScore,
  maxSampleCount: number = DEFAULT_MAX_SAMPLE_COUNT,
): Uint8Array {
  const rendered = renderPortableScoreToPcm(score, maxSampleCount);
  return toWavBytes(rendered.pcmBuffer);
}
