/**
 * Note rendering pipeline: continuous synthesis to discrete float & PCM samples (MUS05 §10).
 */

import {
  SampleBuffer,
  sampleCountForDuration,
  UniformSampler,
} from "@coding-adventures/oscillator";
import { encodeSampleBuffer, type PCMBuffer } from "@coding-adventures/pcm-audio";

import {
  DEFAULT_MAX_PARTIAL_COUNT,
  DEFAULT_MAX_SAMPLE_COUNT,
  DEFAULT_SAMPLE_RATE_HZ,
  MAX_PARTIALS,
  MAX_SAMPLE_COUNT,
} from "./constants.js";
import { Note, resolveNote } from "./note.js";
import { getInstrument, sine } from "./presets.js";
import { InstrumentProfile } from "./profile.js";
import { InstrumentSignal } from "./signal.js";
import type { NoteInput, RenderOptions } from "./types.js";
import {
  validateDuration,
  validateFinite,
  validatePartialCountCeiling,
  validateSampleCountCeiling,
  validateSampleRate,
  validateUnit,
} from "./validation.js";

export class InstrumentNoteRender {
  readonly note: Note;
  readonly fundamentalHz: number;
  readonly instrument: InstrumentProfile;
  readonly signal: InstrumentSignal;
  readonly floatingSamples: SampleBuffer;
  readonly pcmBuffer: PCMBuffer;

  constructor(
    note: Note,
    fundamentalHz: number,
    instrument: InstrumentProfile,
    signal: InstrumentSignal,
    floatingSamples: SampleBuffer,
    pcmBuffer: PCMBuffer,
  ) {
    this.note = note;
    this.fundamentalHz = fundamentalHz;
    this.instrument = instrument;
    this.signal = signal;
    this.floatingSamples = floatingSamples;
    this.pcmBuffer = pcmBuffer;
  }
}

export function renderInstrumentNote(
  note: NoteInput,
  durationSeconds: number,
  options: RenderOptions = {},
): InstrumentNoteRender {
  const parsedNote = resolveNote(note);
  const noteDuration = validateDuration("durationSeconds", durationSeconds);
  const sampleRate = validateSampleRate(
    "sampleRateHz",
    options.sampleRateHz ?? DEFAULT_SAMPLE_RATE_HZ,
  );
  const amplitude = validateUnit("amplitude", options.amplitude ?? 1.0);
  const startTime = validateFinite(
    "startTimeSeconds",
    options.startTimeSeconds ?? 0.0,
  );

  const maxPartials = validatePartialCountCeiling(
    "maxPartialCount",
    options.maxPartialCount ?? DEFAULT_MAX_PARTIAL_COUNT,
    MAX_PARTIALS,
  );
  const maxSamples = validateSampleCountCeiling(
    "maxSampleCount",
    options.maxSampleCount ?? DEFAULT_MAX_SAMPLE_COUNT,
    MAX_SAMPLE_COUNT,
  );

  let profile: InstrumentProfile;
  if (options.instrument instanceof InstrumentProfile) {
    profile = options.instrument;
  } else if (typeof options.instrumentId === "string") {
    profile = getInstrument(options.instrumentId);
  } else {
    profile = sine();
  }

  if (profile.harmonicProfile.length > maxPartials) {
    throw new RangeError(
      `Instrument has ${String(profile.harmonicProfile.length)} partials, exceeding maxPartialCount=${String(maxPartials)}`,
    );
  }

  const fundamentalHz = parsedNote.frequency();
  const nyquistHz = sampleRate / 2.0;
  if (fundamentalHz >= nyquistHz) {
    throw new RangeError(
      `fundamentalHz ${String(fundamentalHz)} must be below Nyquist frequency ${String(nyquistHz)}`,
    );
  }

  const renderedDuration = profile.envelopeProfile.renderedDurationSeconds(noteDuration);
  const expectedSampleCount = sampleCountForDuration(renderedDuration, sampleRate);
  if (expectedSampleCount > maxSamples) {
    throw new RangeError(
      `Render would create ${String(expectedSampleCount)} samples, exceeding maxSampleCount=${String(maxSamples)}`,
    );
  }

  const signal = new InstrumentSignal(
    fundamentalHz,
    profile,
    noteDuration,
    amplitude,
    sampleRate,
    options.variationProfile,
  );

  const sampler = new UniformSampler(sampleRate);
  const floatingSamples = sampler.sample(signal, renderedDuration, startTime);
  const pcmBuffer = encodeSampleBuffer(floatingSamples);

  return new InstrumentNoteRender(
    parsedNote,
    fundamentalHz,
    profile,
    signal,
    floatingSamples,
    pcmBuffer,
  );
}

export function renderInstrumentNotePcm(
  note: NoteInput,
  durationSeconds: number,
  options: RenderOptions = {},
): PCMBuffer {
  const rendered = renderInstrumentNote(note, durationSeconds, options);
  return rendered.pcmBuffer;
}
