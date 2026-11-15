/**
 * Sample-to-Score transcription producing MUS04 PortableScore (MUS08, VER15).
 */

import {
  InstrumentDeclaration,
  MeterEvent,
  PORTABLE_FORMAT_VERSION,
  PortableScore,
  PortableScoreEvent,
  TempoEvent,
  TrackDeclaration,
} from "@coding-adventures/music-machine-web";
import { PCMBuffer } from "@coding-adventures/pcm-audio";

import { validatePcmSourceBounds } from "./dsp_utils.js";
import { rmsEnergy, rmsEnergyFramed } from "./features.js";
import { frequencyToNote } from "./frequency.js";
import { detectOnsets } from "./onset.js";
import { hpsPitch, yinPitch } from "./pitch.js";
import {
  InvalidInputError,
  MAX_BPM,
  MIN_BPM,
  validatePositiveFloat,
  validatePositiveInt,
  validateSamples,
} from "./types.js";

/**
 * Configuration options for score transcription and rhythmic grid quantization.
 */
export interface TranscriptionOptions {
  /** Metric tempo in beats per minute (default 120.0). */
  bpm?: number;
  /** Pulses (ticks) per quarter note (default 480). */
  ppq?: number;
  /** Rhythmic quantization step in beats (0.25 = 16th note). */
  quantizeGridBeats?: number;
  /** Score title string. */
  title?: string;
  /** Instrument preset profile identifier for the track. */
  instrumentId?: string;
  /** Track identifier string. */
  trackId?: string;
}

/**
 * Transcribe raw audio samples or `PCMBuffer` into a MUS04 `PortableScore`.
 *
 * Segments audio into discrete notes via multi-resolution onset detection,
 * estimates pitch using YIN and HPS, quantizes note timings to a musical
 * beat grid at the given tempo, and returns a verified `PortableScore`.
 *
 * @param samples ArrayLike of samples or PCMBuffer.
 * @param sampleRateHz Audio sampling frequency in Hertz (required if samples is not PCMBuffer).
 * @param options Transcription and quantization configuration options.
 * @returns Verified PortableScore instance.
 */
export function transcribePcmToPortableScore(
  samples: ArrayLike<number> | Iterable<number> | PCMBuffer,
  sampleRateHz?: number,
  options?: TranscriptionOptions,
): PortableScore {
  const opts = options ?? {};
  const validPpq = validatePositiveInt(opts.ppq ?? 480, "ppq", 9600);
  const validQuantize = validatePositiveFloat(
    opts.quantizeGridBeats ?? 0.25,
    "quantize_grid_beats",
    16.0,
  );
  const validBpm = validatePositiveFloat(opts.bpm ?? 120.0, "bpm", MAX_BPM);
  if (validBpm < MIN_BPM) {
    throw new InvalidInputError(`bpm ${validBpm} below minimum ${MIN_BPM}`);
  }

  const title = typeof opts.title === "string" ? opts.title : "Transcribed Score";
  const instrumentId =
    typeof opts.instrumentId === "string" ? opts.instrumentId : "sine";
  const trackId = typeof opts.trackId === "string" ? opts.trackId : "melody";

  let floatSamples: Float64Array;
  let validSr: number;

  if (samples instanceof PCMBuffer) {
    const sr = samples.pcmFormat.sampleRateHz;
    validSr = validatePcmSourceBounds(samples.samples.length, sr);
    const raw = samples.samples;
    floatSamples = new Float64Array(raw.length);
    for (let i = 0; i < raw.length; i++) {
      floatSamples[i] = raw[i] / 32767.0;
    }
  } else {
    if (sampleRateHz === undefined || sampleRateHz === null) {
      throw new InvalidInputError(
        "sample_rate_hz must be specified when samples is not a PCMBuffer",
      );
    }
    const rawLen = "length" in samples ? (samples.length as number) : 0;
    validSr = validatePcmSourceBounds(rawLen, sampleRateHz);
    const s = validateSamples(samples, "samples");

    let isIntPcm = false;
    const checkLen = Math.min(100, s.length);
    for (let i = 0; i < checkLen; i++) {
      if (Math.abs(s[i]) > 1.5) {
        isIntPcm = true;
        break;
      }
    }

    if (isIntPcm) {
      floatSamples = new Float64Array(s.length);
      for (let i = 0; i < s.length; i++) {
        floatSamples[i] = s[i] / 32767.0;
      }
    } else {
      floatSamples = s;
    }
  }

  const n = floatSamples.length;
  const overallRms = rmsEnergy(floatSamples);
  const events: PortableScoreEvent[] = [];

  if (overallRms >= 1e-4) {
    let onsets = detectOnsets(floatSamples, validSr, 512, 1024, 1.5, 0.08);
    if (onsets.length === 0) {
      onsets = [0.0];
    }

    const totalDurSec = n / validSr;
    const tps = (validBpm * validPpq) / 60.0;
    const gridStepTicks = Math.max(1, Math.round(validQuantize * validPpq));

    for (let idx = 0; idx < onsets.length; idx++) {
      const tStart = onsets[idx];
      const tNext = idx + 1 < onsets.length ? onsets[idx + 1] : totalDurSec;

      const sStart = Math.floor(tStart * validSr);
      const sEnd = Math.min(n, Math.floor(tNext * validSr));

      if (sEnd - sStart < 128) {
        continue;
      }

      const sliceSamples = floatSamples.subarray(sStart, sEnd);
      const sliceDur = sliceSamples.length / validSr;

      // Estimate active duration
      const hopLen = Math.max(64, Math.floor(sliceSamples.length / 16));
      const frameLen = Math.min(256, sliceSamples.length);
      const rmsFrames =
        sliceSamples.length >= frameLen
          ? rmsEnergyFramed(sliceSamples, frameLen, hopLen)
          : [overallRms];

      let durSec: number;
      if (rmsFrames.length > 0) {
        let maxFrameRms = 0.0;
        for (let i = 0; i < rmsFrames.length; i++) {
          if (rmsFrames[i] > maxFrameRms) {
            maxFrameRms = rmsFrames[i];
          }
        }
        let activeCount = 0;
        for (let i = 0; i < rmsFrames.length; i++) {
          if (rmsFrames[i] >= 0.2 * maxFrameRms) {
            activeCount++;
          }
        }
        durSec = Math.max(
          0.05,
          Math.min(sliceDur, (activeCount * hopLen) / validSr),
        );
      } else {
        durSec = sliceDur;
      }

      // Extract pitch from stable portion
      const skipTransient = Math.min(
        Math.floor(sliceSamples.length / 4),
        Math.floor(0.015 * validSr),
      );
      const stable = sliceSamples.subarray(skipTransient);
      let pitch = yinPitch(stable, validSr, 40.0, 2000.0, 0.15);
      if (pitch <= 0.0) {
        pitch = hpsPitch(stable, validSr, 5, 40.0, 2000.0, undefined);
      }

      if (pitch > 0.0) {
        const noteMatch = frequencyToNote(pitch, 440.0);
        const rawStartTick = tStart * tps;
        const startTickUnits = Math.max(
          0,
          Math.round(rawStartTick / gridStepTicks),
        );
        const startTick = startTickUnits * gridStepTicks;

        const rawDurTicks = durSec * tps;
        const durTickUnits = Math.max(
          1,
          Math.round(rawDurTicks / gridStepTicks),
        );
        const durTicks = Math.max(
          gridStepTicks,
          durTickUnits * gridStepTicks,
        );

        const stableRms = rmsEnergy(stable);
        const vel = Math.min(1.0, Math.max(0.1, stableRms * 2.5));
        const roundedVel = Math.round(vel * 100.0) / 100.0;

        const event = new PortableScoreEvent(
          trackId,
          startTick,
          durTicks,
          "note",
          [noteMatch.note],
          roundedVel,
          events.length,
        );
        events.push(event);
      }
    }
  }

  const kindStr =
    instrumentId === "sine" || instrumentId === "silence"
      ? instrumentId
      : "preset";

  const instDecl = new InstrumentDeclaration(
    instrumentId,
    instrumentId,
    1.0,
    kindStr,
  );
  const trackDecl = new TrackDeclaration(trackId, instrumentId);

  return new PortableScore({
    formatVersion: PORTABLE_FORMAT_VERSION,
    title,
    ppq: validPpq,
    sampleRateHz: Math.round(validSr),
    tempoEvents: [new TempoEvent(0, validBpm)],
    meterEvents: [new MeterEvent(0, "4/4")],
    instruments: [instDecl],
    tracks: [trackDecl],
    events,
  });
}

/**
 * Convenience wrapper for transcribing a `PCMBuffer` directly.
 */
export function transcribePcmBuffer(
  pcm: PCMBuffer,
  options?: TranscriptionOptions,
): PortableScore {
  return transcribePcmToPortableScore(pcm, undefined, options);
}

/**
 * Convenience wrapper for transcribing a slice of samples.
 */
export function transcribeSamples(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  options?: TranscriptionOptions,
): PortableScore {
  return transcribePcmToPortableScore(samples, sampleRateHz, options);
}

// Snake-case aliases
export const transcribe_pcm_to_portable_score = transcribePcmToPortableScore;
export const transcribe_pcm_buffer = transcribePcmBuffer;
export const transcribe_samples = transcribeSamples;
